use crate::kugou::models::{DeviceIdentity, KugouSession, Song};
use crate::kugou::signer::{self, ANDROID_SALT};
use base64::{engine::general_purpose::STANDARD as BASE64_STD, Engine as _};
use chrono::Utc;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use reqwest::Client;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::Mutex;
use std::time::Duration;
use uuid::Uuid;

const CSCC_HOST: &str = "http://d.kugou.com";
const CSCC_VERSION: &str = "10597";
const QRYDID_SALT: &str = "pbKC7zn{4U*ydo2M1Rir";

#[derive(Debug, Clone)]
struct CsccSession {
    cache_key: String,
    cookie: String,
    client_time: i64,
    server_str: String,
    field2: String,
    expire_time_ms: i64,
}

/// 严格对齐 MakcRe/KuGouMusicApi module/user_listen_report.js 的概念版 CSCC 播放上报器
pub struct KugouCsccReporter {
    client: Client,
    cached_session: Mutex<Option<CsccSession>>,
}

impl KugouCsccReporter {
    pub fn new() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            cached_session: Mutex::new(None),
        }
    }

    fn normalize_uuid(raw_guid: &str) -> String {
        let cleaned: String = raw_guid
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        if cleaned.len() == 32 {
            cleaned
        } else {
            signer::md5_hex(raw_guid)
        }
    }

    fn sanitize_field(raw: &str, fallback: &str) -> String {
        let cleaned: String = raw
            .trim()
            .chars()
            .filter(|c| !matches!(c, '&' | '\t' | '\r' | '\n') && !c.is_control())
            .collect();
        if cleaned.is_empty() {
            fallback.to_string()
        } else {
            cleaned
        }
    }

    pub async fn resolve_mix_song_id(&self, song: &Song) -> i64 {
        if song.album_audio_id > 0 {
            return song.album_audio_id;
        }
        if song.audio_id > 0 {
            return song.audio_id;
        }
        if let Ok(parsed) = song.mix_song_id.parse::<i64>() {
            if parsed > 0 {
                return parsed;
            }
        }
        if let Ok(parsed) = song.id.parse::<i64>() {
            if parsed > 0 {
                return parsed;
            }
        }
        if !song.hash.trim().is_empty() {
            let url = format!(
                "https://m.kugou.com/app/i/getSongInfo.php?cmd=playInfo&hash={}",
                song.hash.trim()
            );
            if let Ok(resp) = self.client.get(&url).send().await {
                if let Ok(text) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if let Some(id) = json
                            .get("audio_id")
                            .and_then(|v| v.as_i64())
                            .or_else(|| json.get("songid").and_then(|v| v.as_i64()))
                        {
                            if id > 0 {
                                return id;
                            }
                        }
                    }
                }
            }
        }
        0
    }

    pub async fn report_event(
        &self,
        device: &DeviceIdentity,
        session: &KugouSession,
        song: &Song,
        is_start: bool,
        duration_ms: i64,
        state: &str,
    ) -> Result<bool, String> {
        if !is_start && duration_ms < 10_000 {
            return Ok(false);
        }
        let user_id = session.user_id.trim();
        if user_id.is_empty() || user_id == "0" || session.token.trim().is_empty() {
            return Ok(false);
        }

        let mix_song_id = self.resolve_mix_song_id(song).await;
        if mix_song_id <= 0 {
            return Ok(false);
        }

        let clean_uuid = Self::normalize_uuid(&device.guid);
        let safe_dev = Self::sanitize_field(&device.dev, "KuGouMusicApi");
        let safe_state = Self::sanitize_field(state, "完整播放");
        let now_ms = Utc::now().timestamp_millis();
        let now_sec = now_ms / 1000;

        let fields = if is_start {
            format!(
                "type_id=20431&action=play&fo3=3878,4320,4326,5747,5749,7229,7289,7863&spt=0&sty=手动&mixsongid={}&source=46&ivar1=1&fo=/专辑播放页&mid={}&uuid={}&ss1=1&ss2=1&sys=14&mod={}&channelid=18&ip=0.0.0.0&net=1&ver={}&gitversion=7aa8a76&time={}&userid={}&ss3={}",
                mix_song_id,
                device.mid,
                clean_uuid,
                safe_dev,
                CSCC_VERSION,
                now_ms,
                user_id,
                signer::md5_hex(&Uuid::new_v4().to_string())
            )
        } else {
            format!(
                "type_id=4&action=play&fo3=3878,4320,4326,5747,5749,7229,7289,7863&duration={}&svar3=0&svar2=0&fo=我的音乐/主态/自建歌单/RU&sty=手动&state={}&mixsongid={}&source=46&ivar3=0&type=1&fs=1.0&ivar1=1&mid={}&uuid={}&ss1=1&ss2=1&sys=14&mod={}&channelid=18&ip=0.0.0.0&net=1&ver={}&gitversion=7aa8a76&time={}&userid={}&ss3={}",
                duration_ms.max(0),
                safe_state,
                mix_song_id,
                device.mid,
                clean_uuid,
                safe_dev,
                CSCC_VERSION,
                now_ms,
                user_id,
                signer::md5_hex(&Uuid::new_v4().to_string())
            )
        };

        let cscc_session = self
            .get_or_create_session(device, &clean_uuid, &safe_dev, session)
            .await?;

        // 对齐 user_listen_report.js：不自动重试 /v2/post 事件，避免超时已入账时产生重复上报
        match self
            .post_single_packet(
                &cscc_session,
                &clean_uuid,
                &safe_dev,
                &device.mid,
                user_id,
                now_sec,
                10048,
                &fields,
            )
            .await
        {
            Ok(true) => Ok(true),
            Ok(false) => {
                self.invalidate_session();
                Ok(false)
            }
            Err(e) => {
                self.invalidate_session();
                Err(e)
            }
        }
    }

    fn invalidate_session(&self) {
        if let Ok(mut guard) = self.cached_session.lock() {
            *guard = None;
        }
    }

    async fn get_or_create_session(
        &self,
        device: &DeviceIdentity,
        clean_uuid: &str,
        safe_dev: &str,
        user_session: &KugouSession,
    ) -> Result<CsccSession, String> {
        let now = Utc::now().timestamp_millis();
        let cache_key = signer::md5_hex(&format!(
            "{}|{}|{}|{}|{}",
            clean_uuid, device.mid, user_session.user_id, user_session.token, safe_dev
        ));

        if let Ok(guard) = self.cached_session.lock() {
            if let Some(ref s) = *guard {
                if s.cache_key == cache_key && s.expire_time_ms > now {
                    return Ok(s.clone());
                }
            }
        }

        // 1. POST /v3/qrydid
        let now_sec = (now / 1000).to_string();
        let did_data = format!(
            "{{\"machine\":\"{}\",\"mid\":\"{}\",\"uuid\":\"{}\",\"wh\":[1920,1080]}}",
            safe_dev, device.mid, clean_uuid
        );
        let did_sign = signer::md5_hex(&format!(
            "_t{}appidand02{}{}",
            now_sec, QRYDID_SALT, did_data
        ));
        let qry_url = format!(
            "{}/v3/qrydid?appid=and02&_t={}&sign={}",
            CSCC_HOST, now_sec, did_sign
        );

        let qry_resp = self
            .client
            .post(&qry_url)
            .header("content-type", "application/octet-stream")
            .body(did_data)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let qry_text = qry_resp.text().await.map_err(|e| e.to_string())?;
        let qry_json: Value = serde_json::from_str(&qry_text).map_err(|e| e.to_string())?;
        let device_id = qry_json
            .get("data")
            .and_then(|d| d.get("deviceid"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| "CSCC 未返回有效 deviceid".to_string())?;

        // 2. GET /v2/gen
        let field2 = signer::random_hex(32);
        let s_plain = format!(
            "{}\t{}\t{}\t{}",
            clean_uuid,
            field2,
            Utc::now().timestamp_millis(),
            device_id
        );
        let s_enc_hex = signer::rsa_pkcs1_hex(&s_plain)?;
        let s_enc_bytes = hex::decode(s_enc_hex).map_err(|e| e.to_string())?;
        let s_b64 = BASE64_STD.encode(&s_enc_bytes);

        let gen_time_sec = Utc::now().timestamp().to_string();
        let gen_sign = signer::md5_hex(&format!(
            "_t{}appid3116s{}{}",
            gen_time_sec, s_b64, ANDROID_SALT
        ));

        let gen_resp = self
            .client
            .get(&format!("{}/v2/gen", CSCC_HOST))
            .query(&[
                ("s", s_b64.as_str()),
                ("appid", "3116"),
                ("_t", gen_time_sec.as_str()),
                ("sign", gen_sign.as_str()),
            ])
            .header("KG-Rec", "1")
            .header(
                "User-Agent",
                format!("Android14-1070-{}-18-0-CsccGen-wifi", CSCC_VERSION),
            )
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let gen_text = gen_resp.text().await.map_err(|e| e.to_string())?;
        let gen_json: Value = serde_json::from_str(&gen_text).map_err(|e| e.to_string())?;
        let enc_data_b64 = gen_json
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "CSCC gen 响应数据无效".to_string())?;
        let enc_data_bytes = BASE64_STD.decode(enc_data_b64).map_err(|e| e.to_string())?;
        let iv_str: String = clean_uuid.chars().take(16).collect();
        let dec_bytes = signer::aes_cbc_decrypt_bytes(
            &enc_data_bytes,
            field2.as_bytes(),
            iv_str.as_bytes(),
        )?;
        let dec_str = String::from_utf8(dec_bytes).map_err(|e| e.to_string())?;
        let dec_json: Value = serde_json::from_str(&dec_str).map_err(|e| e.to_string())?;

        let cookie = dec_json
            .get("cookie")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let server_str = dec_json
            .get("serverstr")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if cookie.is_empty() || server_str.is_empty() {
            return Err("CSCC 会话响应缺少 cookie 或 serverstr".to_string());
        }

        let new_session = CsccSession {
            cache_key,
            cookie,
            client_time: dec_json
                .get("clienttime")
                .and_then(|v| v.as_i64())
                .unwrap_or(now / 1000),
            server_str,
            field2,
            expire_time_ms: now + 28 * 60 * 1000,
        };

        if let Ok(mut guard) = self.cached_session.lock() {
            *guard = Some(new_session.clone());
        }
        Ok(new_session)
    }

    #[allow(clippy::too_many_arguments)]
    async fn post_single_packet(
        &self,
        cscc_session: &CsccSession,
        clean_uuid: &str,
        safe_dev: &str,
        mid: &str,
        user_id: &str,
        now_sec: i64,
        business_id: i32,
        fields: &str,
    ) -> Result<bool, String> {
        let event_bytes = fields.as_bytes();
        let deflated_event = zlib_compress(event_bytes)?;

        let line1 = format!(
            "4\t1\t{}\t0\t3116\t{}\t18\t14\t{}\t{}\t{}\t0\t000000000000000000000000000000000000",
            clean_uuid, CSCC_VERSION, safe_dev, clean_uuid, mid
        );
        let line2 = format!(
            "{}\t{}\t0\t{}\t1\t{}\t0\t0",
            event_bytes.len(),
            business_id,
            now_sec,
            user_id
        );
        let mut plain = format!("{}\r\n{}\r\n", line1, line2).into_bytes();
        plain.extend_from_slice(&deflated_event);

        let deflated_plain = zlib_compress(&plain)?;
        let aes_key_str = signer::md5_hex(&format!(
            "{}{}{}{}",
            clean_uuid, cscc_session.client_time, cscc_session.field2, cscc_session.server_str
        ));
        let iv_str: String = clean_uuid.chars().take(16).collect();
        let encrypted = signer::aes_cbc_encrypt_bytes(
            &deflated_plain,
            aes_key_str.as_bytes(),
            iv_str.as_bytes(),
        )?;

        let mut query_params = BTreeMap::new();
        query_params.insert("appid".to_string(), "3116".to_string());
        query_params.insert("cookie".to_string(), cscc_session.cookie.clone());
        query_params.insert("length".to_string(), plain.len().to_string());
        query_params.insert("_t".to_string(), now_sec.to_string());

        let sorted_str: String = query_params
            .iter()
            .map(|(k, v)| format!("{}{}", k, v))
            .collect::<Vec<_>>()
            .join("");

        let mut sign_input = Vec::new();
        sign_input.extend_from_slice(sorted_str.as_bytes());
        sign_input.extend_from_slice(ANDROID_SALT.as_bytes());
        sign_input.extend_from_slice(&encrypted);
        let sign = signer::md5_bytes_hex(&sign_input);

        let mut query_vec: Vec<(String, String)> = query_params.into_iter().collect();
        query_vec.push(("sign".to_string(), sign));

        let resp = self
            .client
            .post(&format!("{}/v2/post", CSCC_HOST))
            .query(&query_vec)
            .header(
                "User-Agent",
                format!("Android14-1070-{}-18-0-CsccPost-wifi", CSCC_VERSION),
            )
            .header("KG-Rec", "1")
            .header("Content-Type", "application/octet-stream")
            .body(encrypted)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status_ok = resp.status().is_success();
        let body_str = resp.text().await.unwrap_or_default();
        if let Ok(obj) = serde_json::from_str::<Value>(&body_str) {
            let st = obj.get("status").and_then(|v| v.as_i64()).unwrap_or(1);
            let ec = obj
                .get("errcode")
                .or_else(|| obj.get("error_code"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            return Ok(status_ok && st != 0 && ec == 0);
        }
        Ok(status_ok)
    }
}

fn zlib_compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}
