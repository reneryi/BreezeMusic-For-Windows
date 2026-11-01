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

const CSCC_HOST: &str = "https://d.kugou.com";
const CSCC_VERSION: &str = "10597";
const QRYDID_SALT: &str = "pbKC7zn{4U*ydo2M1Rir";

#[derive(Debug, Clone)]
struct CsccSession {
    cookie: String,
    client_time: i64,
    server_str: String,
    field2: String,
    expire_time_ms: i64,
}

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
        let mix_song_id = self.resolve_mix_song_id(song).await;
        if mix_song_id <= 0 {
            return Ok(false);
        }

        let user_id = &session.user_id;
        let duration_sec = (duration_ms / 1000).max(10);
        let now_ms = Utc::now().timestamp_millis();
        let now_sec = now_ms / 1000;

        if is_start {
            let fields = format!(
                "type_id=20431&action=play&fo3=3878,4320,4326,5747,5749,7229,7289,7863&spt=0&sty=手动&mixsongid={}&source=46&ivar1=1&fo=/专辑播放页&mid={}&uuid={}&ss1=1&ss2=1&sys=14&mod={}&channelid=18&ip=0.0.0.0&net=1&ver={}&gitversion=7aa8a76&time={}&userid={}&ss3={}",
                mix_song_id,
                device.mid,
                device.guid,
                device.dev,
                CSCC_VERSION,
                now_ms,
                user_id,
                signer::md5_hex(&Uuid::new_v4().to_string())
            );
            return self
                .post_packet_with_retry(device, user_id, now_sec, 10048, &fields)
                .await;
        }

        // 1. 核心播放结算事件 (10048 / type_id=4)
        let fields_10048 = format!(
            "type_id=4&action=play&fo3=3878,4320,4326,5747,5749,7229,7289,7863&duration={}&svar3=0&svar2=0&fo=我的音乐/主态/自建歌单/RU&sty=手动&state=完整播放&mixsongid={}&source=46&ivar3=0&type=1&fs=1.0&ivar1=1&mid={}&uuid={}&ss1=1&ss2=1&sys=14&mod={}&channelid=18&ip=0.0.0.0&net=1&ver={}&gitversion=7aa8a76&time={}&userid={}&ss3={}",
            duration_ms,
            mix_song_id,
            device.mid,
            device.guid,
            device.dev,
            CSCC_VERSION,
            now_ms,
            user_id,
            signer::md5_hex(&Uuid::new_v4().to_string())
        );
        let settled = self
            .post_packet_with_retry(device, user_id, now_sec, 10048, &fields_10048)
            .await
            .unwrap_or(false);

        // 2. 单曲听歌明细 (30025) & 听歌时长心跳 (12904)
        let detail_state = if state == "完整播放" {
            "播放完成"
        } else {
            "切换歌曲"
        };
        let fields_30025 = format!(
            "type_id=30025&time={}&mid={}&uuid={}&Check=0&net=1&ver={}&channelid=18&mod={}&sys=14&userid={}&gitversion=8767fd9&ip=0.0.0.0&action=play&ivar1=1&ivar3=0&svar1=音频&svar2=0&svar3=0&svar6=正式播放&svar8=-1&svar9=否&svar10=未知&svar12=否&svar16=默认音质&svar20=音乐&svar21=否&svar22=无&svar24=2&svar28=-999&svar34=手动切歌&svar38=原始&svar39=关&svar40=0&svar41=主包&svar42=1&svar43=0&svar44=1&svar47=无效果&svar60=普通歌曲&svar61=关闭&svar66=0&svar81=否&svar86=0&svar93=0&svar102={}&svar103={}&svar106=1&svar128=否&svar133=已登录&svar138=长按&bvar1={}&bvar2=1&bvar5={}&bvar9={}&duration={}&mxid={}&source=2&sty=手动&state={}&play_Check=1&fo=搜索/搜索结果页&fo3=3149,3366,4455,6377&fs=1.0&ss1=1&ss2=1&ss3={}",
            now_ms,
            device.mid,
            device.guid,
            CSCC_VERSION,
            device.dev,
            user_id,
            now_ms - duration_ms,
            now_ms,
            duration_sec,
            duration_sec,
            duration_sec,
            duration_ms,
            mix_song_id,
            detail_state,
            signer::md5_hex(&Uuid::new_v4().to_string())
        );
        let _ = self
            .post_packet_with_retry(device, user_id, now_sec + 1, 30025, &fields_30025)
            .await;

        let fields_12904 = format!(
            "type_id=12904&time={}&mid={}&uuid={}&Check=0&net=1&ver={}&channelid=18&mod={}&sys=14&userid={}&gitversion=8767fd9&ip=0.0.0.0&ivar1={}&ivar2=1&ivar4=0&ivar5={}&ivar6={}&ivar7={}&ivar9=1&svar1=音频&svar3=音乐&svar4=概念版&svar5=手动播放&svar7=否&svar8=前台&svar9=主应用&svar10=扬声器&svar14=否&svar19=未知&mxid={}&source=2&fo=搜索/搜索结果页&fo3=3149,3366,4455,6377&ss1=1&ss2=1&ss3={}",
            now_ms,
            device.mid,
            device.guid,
            CSCC_VERSION,
            device.dev,
            user_id,
            duration_sec,
            now_sec - duration_sec,
            now_sec,
            duration_sec,
            mix_song_id,
            signer::md5_hex(&Uuid::new_v4().to_string())
        );
        let _ = self
            .post_packet_with_retry(device, user_id, now_sec + 2, 12904, &fields_12904)
            .await;

        Ok(settled)
    }

    async fn get_or_create_session(&self, device: &DeviceIdentity) -> Result<CsccSession, String> {
        let now = Utc::now().timestamp_millis();
        if let Ok(guard) = self.cached_session.lock() {
            if let Some(ref s) = *guard {
                if s.expire_time_ms > now {
                    return Ok(s.clone());
                }
            }
        }

        // 1. POST /v3/qrydid
        let now_sec = (now / 1000).to_string();
        let did_data = format!(
            "{{\"machine\":\"{}\",\"mid\":\"{}\",\"uuid\":\"{}\",\"wh\":[1080,2400]}}",
            device.dev, device.mid, device.guid
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
            .header("content-type", "application/json; charset=utf-8")
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
        let s_plain = format!("{}\t{}\t{}\t{}", device.guid, field2, now, device_id);
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
        let iv_str: String = device.guid.chars().take(16).collect();
        let dec_bytes = signer::aes_cbc_decrypt_bytes(
            &enc_data_bytes,
            field2.as_bytes(),
            iv_str.as_bytes(),
        )?;
        let dec_str = String::from_utf8(dec_bytes).map_err(|e| e.to_string())?;
        let dec_json: Value = serde_json::from_str(&dec_str).map_err(|e| e.to_string())?;

        let new_session = CsccSession {
            cookie: dec_json
                .get("cookie")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            client_time: dec_json
                .get("clienttime")
                .and_then(|v| v.as_i64())
                .unwrap_or(now / 1000),
            server_str: dec_json
                .get("serverstr")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            field2,
            expire_time_ms: now + 25 * 60 * 1000,
        };

        if let Ok(mut guard) = self.cached_session.lock() {
            *guard = Some(new_session.clone());
        }
        Ok(new_session)
    }

    async fn post_packet_with_retry(
        &self,
        device: &DeviceIdentity,
        user_id: &str,
        now_sec: i64,
        business_id: i32,
        fields: &str,
    ) -> Result<bool, String> {
        if let Ok(s1) = self.get_or_create_session(device).await {
            if let Ok(true) = self
                .post_single_packet(&s1, device, user_id, now_sec, business_id, fields)
                .await
            {
                return Ok(true);
            }
        }
        if let Ok(mut guard) = self.cached_session.lock() {
            *guard = None;
        }
        let s2 = self.get_or_create_session(device).await?;
        self.post_single_packet(&s2, device, user_id, now_sec + 1, business_id, fields)
            .await
    }

    async fn post_single_packet(
        &self,
        cscc_session: &CsccSession,
        device: &DeviceIdentity,
        user_id: &str,
        now_sec: i64,
        business_id: i32,
        fields: &str,
    ) -> Result<bool, String> {
        let event_bytes = fields.as_bytes();
        let deflated_event = zlib_compress(event_bytes)?;

        let line1 = format!(
            "4\t1\t{}\t0\t3116\t{}\t18\t14\t{}\t{}\t{}\t0\t000000000000000000000000000000000000",
            device.guid, CSCC_VERSION, device.dev, device.guid, device.mid
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
            device.guid, cscc_session.client_time, cscc_session.field2, cscc_session.server_str
        ));
        let iv_str: String = device.guid.chars().take(16).collect();
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
            let ec = obj.get("errcode").and_then(|v| v.as_i64()).unwrap_or(0);
            return Ok(st == 1 && ec == 0);
        }
        Ok(status_ok)
    }
}

fn zlib_compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}
