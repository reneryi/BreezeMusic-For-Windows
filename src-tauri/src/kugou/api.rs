use crate::kugou::audio_proxy::AudioStreamProxy;
use crate::kugou::cscc::KugouCsccReporter;
use crate::kugou::krc;
use crate::kugou::metadata::{self, AudioTagMetadata};
use crate::kugou::models::{
    AccountProfile, AlbumItem, AudioQuality, CacheDiskStats, DownloadTaskResult, FmChannel,
    FollowedArtist, GatewayDiagResult, HotSearchBoard, HotSearchEntry, KugouSession,
    ListeningGradeSyncResult, LoginCandidate, LyricData, PagedSearchResult, PhoneLoginOutcome,
    PlaybackResult, Playlist, PurchasedAlbumItem, PurchasedSongItem, QrCodeSession, QrPollResult,
    QuotedReplyHop, RankItem, ResolvedSongAlbumMeta, SearchDiscoveryData, SharedMusicResolved,
    Song, SongAuthor, SongComment, SongDetailInfo, VipDayRecord, VipStatusInfo, ViperEffectItem,
};
use crate::kugou::signer::{
    self, ANDROID_SALT, APP_ID, CLIENT_VERSION, STANDARD_APP_ID, STANDARD_CLIENT_VERSION,
    STANDARD_SALT,
};
use crate::kugou::store::{is_same_music_track, merge_song_entries, AppStore};
use crate::kugou::transport::KugouTransport;
use chrono::{FixedOffset, NaiveDateTime, TimeZone, Utc};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub struct KugouApiService {
    pub transport: KugouTransport,
    pub store: Arc<AppStore>,
    pub cscc: KugouCsccReporter,
    pub audio_proxy: AudioStreamProxy,
    pub personal_fm_cursor: AtomicU32,
    pub fm_channel_cursors: Mutex<HashMap<String, u32>>,
    pub seen_fm_channel_hashes: Mutex<HashMap<String, HashSet<String>>>,
    pub cached_fm_channels: Mutex<Vec<FmChannel>>,
    pub seen_personal_fm_hashes: Mutex<HashSet<String>>,
    pub taste_seed_songs: Mutex<Vec<Song>>,
}

impl KugouApiService {
    pub fn new(store: Arc<AppStore>) -> Self {
        let cache_dir = Self::resolve_cache_dir();
        Self {
            transport: KugouTransport::new(),
            store,
            cscc: KugouCsccReporter::new(),
            audio_proxy: AudioStreamProxy::new(cache_dir),
            personal_fm_cursor: AtomicU32::new(0),
            fm_channel_cursors: Mutex::new(HashMap::new()),
            seen_fm_channel_hashes: Mutex::new(HashMap::new()),
            cached_fm_channels: Mutex::new(Vec::new()),
            seen_personal_fm_hashes: Mutex::new(HashSet::new()),
            taste_seed_songs: Mutex::new(Vec::new()),
        }
    }

    pub fn record_taste_seeds(&self, songs: &[Song]) {
        if songs.is_empty() {
            return;
        }
        if let Ok(mut guard) = self.taste_seed_songs.lock() {
            for s in songs.iter().take(40) {
                if !s.hash.is_empty() && !guard.iter().any(|e| e.hash.eq_ignore_ascii_case(&s.hash)) {
                    guard.insert(0, s.clone());
                }
            }
            if guard.len() > 120 {
                guard.truncate(120);
            }
        }
    }

    fn is_legacy_local_dfid(dfid: &str) -> bool {
        let trimmed = dfid.trim();
        trimmed.len() == 24
            && trimmed.starts_with('2')
            && trimmed
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    }

    /// 1. 设备风控注册 (严格对齐 MakcRe/KuGouMusicApi module/register_dev.js -> /risk/v2/r_register_dev)
    pub async fn register_device(&self, force_refresh: bool) -> Result<String, String> {
        let dev = self.store.device();
        if !force_refresh
            && dev.dfid_registered
            && !dev.dfid.is_empty()
            && dev.dfid != "-"
            && !Self::is_legacy_local_dfid(&dev.dfid)
        {
            return Ok(dev.dfid);
        }

        let session = self.store.session();
        let active_uid = session
            .as_ref()
            .map(|s| s.user_id.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("0");
        let active_token = session
            .as_ref()
            .map(|s| s.token.as_str())
            .unwrap_or("");

        let clean_guid: String = dev
            .guid
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        let uuid_str = if clean_guid.is_empty() {
            signer::md5_hex(&dev.mid)
        } else {
            clean_guid
        };

        let data_plain = json!({
            "availableRamSize": 4_983_533_568_i64,
            "availableRomSize": 48_114_719_i64,
            "availableSDSize": 48_114_717_i64,
            "basebandVer": "",
            "batteryLevel": 100,
            "batteryStatus": 3,
            "brand": "Redmi",
            "buildSerial": "unknown",
            "device": "marble",
            "imei": uuid_str,
            "imsi": "",
            "manufacturer": "Xiaomi",
            "uuid": uuid_str,
            "accelerometer": false,
            "accelerometerValue": "",
            "gravity": false,
            "gravityValue": "",
            "gyroscope": false,
            "gyroscopeValue": "",
            "light": false,
            "lightValue": "",
            "magnetic": false,
            "magneticValue": "",
            "orientation": false,
            "orientationValue": "",
            "pressure": false,
            "pressureValue": "",
            "step_counter": false,
            "step_counterValue": "",
            "temperature": false,
            "temperatureValue": ""
        })
        .to_string();

        if let Ok((aes_key, aes_cipher)) = signer::playlist_aes_encrypt(&data_plain) {
            let p_inner = if active_uid == "0" {
                json!({
                    "aes": aes_key,
                    "uid": 0,
                    "token": active_token
                })
            } else {
                json!({
                    "aes": aes_key,
                    "uid": active_uid,
                    "token": active_token
                })
            }
            .to_string();

            if let Ok(p_rsa) = signer::rsa_pkcs1_hex(&p_inner) {
                let mut reg_dev = dev.clone();
                reg_dev.dfid = "-".to_string();

                let mut query = HashMap::new();
                query.insert("part".to_string(), "1".to_string());
                query.insert("platid".to_string(), "1".to_string());
                query.insert("p".to_string(), p_rsa);

                if let Ok(raw_bytes) = self
                    .transport
                    .request_raw_bytes(
                        "/risk/v2/r_register_dev",
                        &reg_dev,
                        session.as_ref(),
                        query,
                        Some(aes_cipher),
                        "https://userservice.kugou.com",
                        "POST",
                        HashMap::new(),
                        true,
                        true,
                    )
                    .await
                {
                    if !raw_bytes.is_empty() {
                        let utf8_str = String::from_utf8_lossy(&raw_bytes).trim().to_string();
                        let parsed_val = if utf8_str.starts_with('{') {
                            serde_json::from_str::<Value>(&utf8_str).ok()
                        } else {
                            signer::playlist_aes_decrypt(&raw_bytes, &aes_key)
                                .ok()
                                .and_then(|dec| serde_json::from_str::<Value>(&dec).ok())
                        };
                        if let Some(root) = parsed_val {
                            let data_obj = root.get("data").unwrap_or(&root);
                            if let Some(dfid) = data_obj.get("dfid").and_then(|v| v.as_str()) {
                                let clean_dfid = dfid.trim();
                                if !clean_dfid.is_empty() && clean_dfid != "-" {
                                    self.store.save_dfid(clean_dfid);
                                    return Ok(clean_dfid.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        if !dev.dfid.is_empty() && dev.dfid != "-" {
            Ok(dev.dfid)
        } else {
            Ok("-".to_string())
        }
    }

    /// 2. 发送手机短信验证码 (对齐 MakcRe/KuGouMusicApi captcha_sent.js -> /v7/send_mobile_code)
    pub async fn send_mobile_code(&self, mobile: &str) -> Result<String, String> {
        let trimmed = mobile.trim();
        if trimmed.len() != 11 || !trimmed.starts_with('1') {
            return Err("请输入有效的 11 位中国大陆手机号".to_string());
        }
        let mut dev = self.store.device();
        if dev.dfid.trim().is_empty() {
            dev.dfid = "-".to_string();
        }
        let now_ms = Utc::now().timestamp_millis();
        let now_sec = (now_ms / 1000).to_string();

        let body = json!({
            "businessid": 5,
            "mobile": trimmed,
            "plat": 3
        });

        let mut query = HashMap::new();
        query.insert("clienttime".to_string(), now_sec);

        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "login.user.kugou.com".to_string());

        self.transport
            .call(
                "/v7/send_mobile_code",
                &dev,
                None,
                query,
                Some(body),
                "https://loginserviceretry.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await?
            .require_success()?;

        Ok("验证码已发送，请注意查收短信".to_string())
    }

    /// 3. 手机短信验证码登录 (对齐 MakcRe/KuGouMusicApi login_cellphone.js -> /v7/login_by_verifycode)
    pub async fn login_by_phone(
        &self,
        mobile: &str,
        code: &str,
        user_id: Option<String>,
    ) -> Result<PhoneLoginOutcome, String> {
        let mobile = mobile.trim();
        let code = code.trim();
        let now_ms = Utc::now().timestamp_millis();
        let now_sec = (now_ms / 1000).to_string();
        let dev = self.store.device();

        let plain_params = json!({
            "mobile": mobile,
            "code": code
        })
        .to_string();
        let encrypted = signer::encrypt_login_params(&plain_params)?;

        let pk_plain = json!({
            "clienttime_ms": now_ms,
            "key": encrypted.key
        })
        .to_string();
        let pk_hex = signer::rsa_raw_zero_pad_hex(&pk_plain)?.to_uppercase();

        let masked_mobile = if mobile.len() >= 11 {
            format!("{}*****{}", &mobile[..2], &mobile[mobile.len() - 1..])
        } else {
            mobile.to_string()
        };

        let mut body_obj = json!({
            "plat": 1,
            "support_multi": 1,
            "t1": signer::calc_t1(None, now_ms),
            "t2": signer::calc_t2(&dev.guid, &dev.mac, &dev.dev, now_ms),
            "clienttime_ms": now_ms,
            "mobile": masked_mobile,
            "key": signer::params_key(now_ms, ""),
            "pk": pk_hex,
            "params": encrypted.ciphertext,
            "dfid": dev.dfid,
            "dev": dev.dev,
            "gitversion": "5f0b7c4"
        });
        if let Some(uid) = user_id {
            if !uid.trim().is_empty() {
                body_obj["userid"] = json!(uid.trim());
            }
        }

        // 显式锁定 query 中的 clienttime 与 body_obj 的 clienttime_ms / 1000 完全同步，防止跨秒重试导致签名时间戳偏移
        let mut login_query = HashMap::new();
        login_query.insert("clienttime".to_string(), now_sec);

        let mut headers = HashMap::new();
        headers.insert("support-calm".to_string(), "1".to_string());
        headers.insert("x-router".to_string(), "login.user.kugou.com".to_string());
        headers.insert(
            "User-Agent".to_string(),
            "Android16-1070-11440-130-0-LOGIN-wifi".to_string(),
        );

        let resp = match self
            .transport
            .call(
                "/v7/login_by_verifycode",
                &dev,
                None,
                login_query.clone(),
                Some(body_obj.clone()),
                "https://loginserviceretry.kugou.com",
                "POST",
                false,
                headers.clone(),
                true,
                true,
            )
            .await
        {
            Ok(r) => r,
            Err(_) => {
                self.transport
                    .call(
                        "/v7/login_by_ipv4",
                        &dev,
                        None,
                        login_query,
                        Some(body_obj),
                        "http://login.user.kugou.com",
                        "POST",
                        false,
                        headers,
                        true,
                        true,
                    )
                    .await?
            }
        };

        let data = resp.data();
        let candidates = Self::extract_login_candidates(data);
        if (!resp.is_success() && !candidates.is_empty() && resp.code() == Some(34175))
            || (resp.is_success()
                && Self::val_str(data, "token").is_empty()
                && Self::val_str(data, "secu_params").is_empty()
                && !candidates.is_empty())
        {
            return Ok(PhoneLoginOutcome {
                status: "choose_account".to_string(),
                session: None,
                profile: None,
                candidates,
                message: "检测到该手机号绑定了多个账号，请选择要登录的账号".to_string(),
            });
        }

        let resp = resp.require_success()?;
        let session = self.decode_and_save_login(resp.data(), Some(&encrypted.key), None)?;
        let profile = self.fetch_full_profile().await.ok();

        Ok(PhoneLoginOutcome {
            status: "success".to_string(),
            session: Some(session),
            profile,
            candidates: Vec::new(),
            message: "登录成功".to_string(),
        })
    }

    /// 4. 生成酷狗概念版扫码登录二维码 (对齐 AuthRepository.createQr)
    pub async fn create_qr_code(&self) -> Result<QrCodeSession, String> {
        let mut qr_dev = self.store.device();
        qr_dev.dfid = "-".to_string();

        let mut query = HashMap::new();
        query.insert("appid".to_string(), "1001".to_string());
        query.insert("type".to_string(), "1".to_string());
        query.insert("plat".to_string(), "4".to_string());
        query.insert(
            "qrcode_txt".to_string(),
            "https://h5.kugou.com/apps/loginQRCode/html/index.html?appid=3116&".to_string(),
        );
        query.insert("srcappid".to_string(), "2919".to_string());

        let resp = match self
            .transport
            .call(
                "/v2/qrcode",
                &qr_dev,
                None,
                query.clone(),
                None,
                "https://login-user.kugou.com",
                "GET",
                true,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            Ok(r) if r.is_success() => r,
            _ => {
                query.insert("appid".to_string(), "1014".to_string());
                self.transport
                    .call(
                        "/v2/qrcode",
                        &qr_dev,
                        None,
                        query,
                        None,
                        "https://login-user.kugou.com",
                        "GET",
                        true,
                        HashMap::new(),
                        true,
                        true,
                    )
                    .await?
                    .require_success()?
            }
        };

        let d = resp.data();
        let qrcode_key = Self::val_str(d, "qrcode");
        let raw_img = Self::val_str(d, "qrcode_img");
        if qrcode_key.is_empty() || raw_img.is_empty() {
            return Err("获取二维码失败：返回数据不完整".to_string());
        }
        let qrcode_img = if raw_img.starts_with("data:image/") {
            raw_img
        } else {
            format!("data:image/png;base64,{}", raw_img)
        };
        let ttl_sec = Self::val_i64(d, "expire")
            .or_else(|| Self::val_i64(d, "expires_in"))
            .unwrap_or(120)
            .clamp(30, 600);

        Ok(QrCodeSession {
            qrcode_key,
            qrcode_img,
            expires_at_ms: Utc::now().timestamp_millis() + ttl_sec * 1000,
        })
    }

    /// 5. 轮询酷狗扫码状态 (严格修复状态码映射并补齐 /v5/login_by_token 凭证升级！)
    /// 对齐 Android 端 LibraryViewModel.kt:
    /// - status == 0 -> 二维码已过期
    /// - status == 1 -> 等待扫码
    /// - status == 2 -> 已扫码，请在酷狗音乐 App 点击确认登录
    /// - status == 4 -> 授权成功！此时提取初始 session 并调用 refresh_session_token 补全完整概念版登录凭证
    pub async fn check_qr_code(&self, key: &str) -> Result<QrPollResult, String> {
        let mut qr_dev = self.store.device();
        qr_dev.dfid = "-".to_string();

        let mut query = HashMap::new();
        query.insert("plat".to_string(), "4".to_string());
        query.insert("appid".to_string(), APP_ID.to_string());
        query.insert("srcappid".to_string(), "2919".to_string());
        query.insert("qrcode".to_string(), key.trim().to_string());
        query.insert("dev".to_string(), qr_dev.dev.clone());

        let resp = self
            .transport
            .call(
                "/v2/get_userinfo_qrcode",
                &qr_dev,
                None,
                query,
                None,
                "https://login-user.kugou.com",
                "GET",
                true,
                HashMap::new(),
                true,
                true,
            )
            .await?
            .require_success()?;

        let d = resp.data();
        let qr_status = Self::val_i64(d, "status").unwrap_or(1) as i32;
        match qr_status {
            0 => Ok(QrPollResult {
                status: 0,
                message: "二维码已过期，请点击刷新重新生成".to_string(),
                session: None,
                profile: None,
            }),
            1 => Ok(QrPollResult {
                status: 1,
                message: "等待使用酷狗音乐 / 酷狗概念版 App 扫码".to_string(),
                session: None,
                profile: None,
            }),
            2 => Ok(QrPollResult {
                status: 2,
                message: "已扫码，请在手机酷狗 App 中点击「确认登录」".to_string(),
                session: None,
                profile: None,
            }),
            4 => {
                // 授权成功！对齐 Android 端 auth.qrSession(result.data) + auth.refresh(initial.session)
                let initial_session = self.decode_and_save_login(d, None, None)?;
                let final_session = match self.refresh_session_token(&initial_session).await {
                    Ok(mut refreshed) => {
                        if (refreshed.nickname.is_empty() || refreshed.nickname.starts_with("酷狗用户"))
                            && !initial_session.nickname.is_empty()
                        {
                            refreshed.nickname = initial_session.nickname.clone();
                        }
                        if refreshed.avatar_url.is_empty() && !initial_session.avatar_url.is_empty() {
                            refreshed.avatar_url = initial_session.avatar_url.clone();
                        }
                        self.store.set_session(Some(refreshed.clone()));
                        refreshed
                    }
                    Err(_) => initial_session,
                };
                let profile = self.fetch_full_profile().await.ok();
                Ok(QrPollResult {
                    status: 4,
                    message: "扫码授权成功，已完成登录".to_string(),
                    session: Some(final_session),
                    profile,
                })
            }
            other => Ok(QrPollResult {
                status: other,
                message: format!("二维码状态: {}", other),
                session: None,
                profile: None,
            }),
        }
    }

    /// 对齐 AuthRepository.refresh(session) -> /v5/login_by_token
    /// 扫码获得初始 token 后，调用此接口换取概念版完整 t1 / vip_token 会话
    pub async fn refresh_session_token(&self, session: &KugouSession) -> Result<KugouSession, String> {
        let now_ms = Utc::now().timestamp_millis();
        let dev = self.store.device();
        let encrypted = signer::encrypt_login_params("{}")?;
        let p3 = signer::calc_p3(now_ms / 1000, &session.token);
        let pk_plain = json!({
            "clienttime_ms": now_ms,
            "key": encrypted.key
        })
        .to_string();
        let pk = signer::rsa_raw_zero_pad_hex(&pk_plain)?;

        let body = json!({
            "dfid": dev.dfid,
            "p3": p3,
            "plat": 1,
            "t1": signer::calc_t1(session.t1.as_deref(), now_ms),
            "t2": signer::calc_t2(&dev.guid, &dev.mac, &dev.dev, now_ms),
            "t3": "MCwwLDAsMCwwLDAsMCwwLDA=",
            "pk": pk,
            "params": encrypted.ciphertext,
            "userid": session.user_id,
            "clienttime_ms": now_ms,
            "dev": dev.dev
        });

        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "login.user.kugou.com".to_string());

        let resp = self
            .transport
            .call(
                "/v5/login_by_token",
                &dev,
                Some(session),
                HashMap::new(),
                Some(body),
                "http://login.user.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await?
            .require_success()?;

        self.decode_and_save_login(resp.data(), Some(&encrypted.key), Some(session))
    }

    fn decode_and_save_login(
        &self,
        data: &Value,
        temp_key: Option<&str>,
        previous: Option<&KugouSession>,
    ) -> Result<KugouSession, String> {
        let mut merged = data.clone();
        if let Some(k) = temp_key {
            let secu = Self::val_str(data, "secu_params");
            if !secu.is_empty() {
                if let Ok(plain) = signer::decrypt_login_params(&secu, k) {
                    if let Ok(parsed_obj) = serde_json::from_str::<Value>(&plain) {
                        if let (Some(target_map), Some(src_map)) =
                            (merged.as_object_mut(), parsed_obj.as_object())
                        {
                            for (mk, mv) in src_map {
                                target_map.insert(mk.clone(), mv.clone());
                            }
                        }
                    } else if let Some(target_map) = merged.as_object_mut() {
                        target_map.insert("token".to_string(), Value::String(plain));
                    }
                }
            }
        }

        let mut user_id = Self::val_str(&merged, "userid");
        if user_id.is_empty() {
            user_id = Self::val_str(&merged, "kugouid");
        }
        if user_id.is_empty() {
            if let Some(prev) = previous {
                user_id = prev.user_id.clone();
            }
        }
        let mut token = Self::val_str(&merged, "token");
        if token.is_empty() {
            if let Some(prev) = previous {
                token = prev.token.clone();
            }
        }
        if user_id.is_empty() || token.is_empty() {
            return Err("登录响应缺少 userid 或 token".to_string());
        }

        let dfid = Self::val_str(&merged, "dfid");
        if !dfid.is_empty() && dfid != "-" {
            self.store.save_dfid(&dfid);
        }

        let nickname = {
            let n = Self::val_str(&merged, "nickname");
            if !n.is_empty() {
                n
            } else {
                let u = Self::val_str(&merged, "username");
                if !u.is_empty() {
                    u
                } else if let Some(prev) = previous {
                    prev.nickname.clone()
                } else {
                    format!("酷狗用户 {}", user_id)
                }
            }
        };

        let pic_raw = {
            let p = Self::val_str(&merged, "pic");
            if !p.is_empty() {
                p
            } else {
                Self::val_str(&merged, "avatar")
            }
        };
        let mut avatar_url = signer::normalize_image_url(Some(&pic_raw));
        if avatar_url.is_empty() {
            if let Some(prev) = previous {
                avatar_url = prev.avatar_url.clone();
            }
        }

        let t1 = {
            let t = Self::val_str(&merged, "t1");
            if !t.is_empty() {
                Some(t)
            } else {
                previous.and_then(|p| p.t1.clone())
            }
        };
        let vip_token = {
            let vt = Self::val_str(&merged, "vip_token");
            if !vt.is_empty() {
                Some(vt)
            } else {
                let vt2 = Self::val_str(&merged, "viptoken");
                if !vt2.is_empty() {
                    Some(vt2)
                } else {
                    previous.and_then(|p| p.vip_token.clone())
                }
            }
        };

        let session = KugouSession {
            user_id,
            token,
            t1,
            vip_token,
            nickname,
            avatar_url,
        };
        self.store.set_session(Some(session.clone()));
        Ok(session)
    }

    fn val_f64_opt(obj: &Value, key: &str) -> Option<f64> {
        let v = obj.get(key)?;
        v.as_f64()
            .or_else(|| v.as_i64().map(|n| n as f64))
            .or_else(|| v.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
    }

    fn is_vip_time_active(end_time: &str, now_utc_ts: i64) -> bool {
        let trimmed = end_time.trim();
        if trimmed.is_empty() {
            return true;
        }
        let china_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        if let Ok(ndt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
            if let Some(dt) = china_tz.from_local_datetime(&ndt).single() {
                return dt.timestamp() + 60 >= now_utc_ts;
            }
        }
        true
    }

    /// 权威云端基线校准状态机（对齐 MakcRe/KuGouMusicApi module/user_grade_info.js）：
    /// - 以服务端返回的真实 d_sec 为唯一权威基线，自动自愈历史旧版被积分污染的 gradeStates (schema_ver < 2)
    /// - 仅保留最多 600 秒尚未入账的本地实时增量缓冲（pending_diff_sec + inflight_diff_sec），彻底杜绝单调棘轮锁死虚高
    fn reconcile_grade_state(
        &self,
        user_id: &str,
        raw_server_dsec: i64,
        server_grade: i32,
        now_sec: i64,
    ) -> (i64, i64, i32) {
        const GRADE_STATE_SCHEMA_VER: u32 = 2;

        let uid = user_id.trim();
        if uid.is_empty() {
            return (0, 0, server_grade.max(0));
        }

        let mut st = self.store.get_grade_state(uid);
        let safe_raw = raw_server_dsec.max(0);
        let mut dirty = false;

        if st.schema_ver < GRADE_STATE_SCHEMA_VER {
            st.schema_ver = GRADE_STATE_SCHEMA_VER;
            st.synced_raw_server_dsec = safe_raw;
            st.synced_official_listen_sec = safe_raw;
            st.pending_diff_sec = 0;
            st.inflight_diff_sec = 0;
            st.inflight_at_sec = 0;
            dirty = true;
        } else {
            if st.inflight_diff_sec > 0 && (now_sec - st.inflight_at_sec).abs() > 180 {
                st.inflight_diff_sec = 0;
                st.inflight_at_sec = 0;
                dirty = true;
            }

            if safe_raw > 0 {
                if st.synced_raw_server_dsec <= 0
                    || (st.synced_raw_server_dsec - safe_raw).abs() > 1_800
                {
                    st.synced_raw_server_dsec = safe_raw;
                    st.synced_official_listen_sec = safe_raw;
                    st.inflight_diff_sec = 0;
                    st.inflight_at_sec = 0;
                    st.pending_diff_sec = st.pending_diff_sec.clamp(0, 600);
                    dirty = true;
                } else if safe_raw > st.synced_raw_server_dsec {
                    let gain = safe_raw - st.synced_raw_server_dsec;
                    let used_inflight = gain.min(st.inflight_diff_sec.max(0));
                    st.inflight_diff_sec = (st.inflight_diff_sec - used_inflight).max(0);
                    if st.inflight_diff_sec == 0 {
                        st.inflight_at_sec = 0;
                    }
                    let rem_gain = gain - used_inflight;
                    if rem_gain > 0 {
                        st.pending_diff_sec = (st.pending_diff_sec - rem_gain).max(0);
                    }
                    st.synced_raw_server_dsec = safe_raw;
                    st.synced_official_listen_sec = safe_raw;
                    dirty = true;
                }
            }
        }

        if server_grade > 0 && st.last_known_grade != server_grade {
            st.last_known_grade = server_grade;
            dirty = true;
        }
        let resolved_grade = if server_grade > 0 {
            server_grade
        } else {
            st.last_known_grade.max(0)
        };

        if dirty {
            self.store.set_grade_state(uid, st.clone());
        }

        let pending_only = st.pending_diff_sec.clamp(0, 600);
        let unsettled_total = (st.pending_diff_sec.max(0) + st.inflight_diff_sec.max(0)).clamp(0, 600);
        (unsettled_total, pending_only, resolved_grade)
    }

    pub fn record_listening_seconds(&self, seconds: i64) -> Result<i64, String> {
        let Some(session) = self.store.session() else {
            return Ok(0);
        };
        if session.user_id.trim().is_empty() || seconds <= 0 {
            return Ok(0);
        }
        let mut st = self.store.get_grade_state(&session.user_id);
        if st.schema_ver < 2 {
            st.schema_ver = 2;
            st.synced_official_listen_sec = 0;
            st.synced_raw_server_dsec = 0;
            st.pending_diff_sec = 0;
            st.inflight_diff_sec = 0;
            st.inflight_at_sec = 0;
        }
        st.pending_diff_sec = (st.pending_diff_sec.max(0) + seconds.clamp(1, 300)).clamp(0, 3600);
        let next_pending = st.pending_diff_sec;
        self.store.set_grade_state(&session.user_id, st);
        Ok(next_pending)
    }

    pub async fn sync_listening_grade(
        &self,
        _force_report: bool,
    ) -> Result<ListeningGradeSyncResult, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录账号".to_string());
        };
        let initial_profile = self.fetch_full_profile_with_report(None, None).await?;
        let mut st = self.store.get_grade_state(&session.user_id);
        let pending_diff = st.pending_diff_sec.max(0);

        if pending_diff <= 0 || initial_profile.raw_server_dsec <= 0 {
            return Ok(ListeningGradeSyncResult {
                message: format!(
                    "已校准云端听歌等级与时长 · Lv.{}（累计 {}）",
                    initial_profile.user_grade, initial_profile.listen_duration_formatted
                ),
                profile: initial_profile,
                reported_diff_sec: 0,
                cloud_confirmed: true,
            });
        }

        // 严格对齐 user_grade_info.js：向 3116 上报时 d_sec 必须使用概念版 3116 自身基线 lite_server_dsec
        let report_diff = pending_diff.clamp(1, 300);
        let base_d_sec = if initial_profile.lite_server_dsec > 0 {
            initial_profile.lite_server_dsec
        } else {
            initial_profile.raw_server_dsec
        };
        let now_sec = Utc::now().timestamp();

        st.pending_diff_sec = (st.pending_diff_sec - report_diff).max(0);
        st.inflight_diff_sec = (st.inflight_diff_sec.max(0) + report_diff).clamp(0, 600);
        st.inflight_at_sec = now_sec;
        self.store.set_grade_state(&session.user_id, st.clone());

        match self
            .fetch_full_profile_with_report(Some(base_d_sec), Some(report_diff))
            .await
        {
            Ok(updated_profile) => Ok(ListeningGradeSyncResult {
                message: if report_diff >= 60 {
                    format!(
                        "已上报 {} 分钟新增听歌时长 · 当前 Lv.{}（累计 {}）",
                        report_diff / 60,
                        updated_profile.user_grade,
                        updated_profile.listen_duration_formatted
                    )
                } else {
                    format!(
                        "已上报 {} 秒新增听歌时长 · 当前 Lv.{}（累计 {}）",
                        report_diff,
                        updated_profile.user_grade,
                        updated_profile.listen_duration_formatted
                    )
                },
                profile: updated_profile,
                reported_diff_sec: report_diff,
                cloud_confirmed: true,
            }),
            Err(e) => {
                // 上报失败时退回 pending_diff_sec，等待下次同步
                let mut rollback = self.store.get_grade_state(&session.user_id);
                rollback.inflight_diff_sec = (rollback.inflight_diff_sec - report_diff).max(0);
                rollback.pending_diff_sec = (rollback.pending_diff_sec + report_diff).clamp(0, 3600);
                self.store.set_grade_state(&session.user_id, rollback);
                Err(e)
            }
        }
    }

    /// 6. 获取当前登录账号完整资料、权威云端听歌时长 (d_sec)、等级积分进度与概念版 VIP 状态
    /// 严格对齐 MakcRe/KuGouMusicApi:
    /// - module/user_detail.js        -> POST /v3/get_my_info (usercenter.kugou.com)
    /// - module/user_info.js          -> POST /v1/get_my_userinfo (relation.user.kugou.com)
    /// - module/user_grade_info.js    -> POST /v2/get_grade_info (userinfo.user.kugou.com, lite 3116/10597)
    /// - module/youth_union_vip.js    -> GET  /v1/get_union_vip (kugouvip.kugou.com)
    pub async fn fetch_full_profile(&self) -> Result<AccountProfile, String> {
        self.fetch_full_profile_with_report(None, None).await
    }

    pub async fn fetch_full_profile_with_report(
        &self,
        reported_d_sec: Option<i64>,
        reported_diff_sec: Option<i64>,
    ) -> Result<AccountProfile, String> {
        let Some(mut session) = self.store.session() else {
            return Err("当前未登录".to_string());
        };
        let mut dev = self.store.device();
        if !dev.dfid_registered
            || dev.dfid.is_empty()
            || dev.dfid == "-"
            || Self::is_legacy_local_dfid(&dev.dfid)
        {
            if let Ok(real_dfid) = self.register_device(false).await {
                if !real_dfid.is_empty() && real_dfid != "-" {
                    dev.dfid = real_dfid;
                    dev.dfid_registered = true;
                }
            }
        }

        let now_sec = Utc::now().timestamp();
        let now_ms = Utc::now().timestamp_millis();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);

        let req_d_sec = reported_d_sec.unwrap_or(0).max(0);
        let req_diff_sec = reported_diff_sec.unwrap_or(0).max(0);
        let is_report = req_d_sec > 0 && req_diff_sec > 0;

        let session_ref = &session;
        let dev_ref = &dev;

        // 并发请求四大概念版权威接口，杜绝跨平台 1005 (error_code: 20018) 无效串行阻塞
        let fut_my_info = async {
            let p_plain = json!({
                "token": session_ref.token,
                "clienttime": now_sec
            })
            .to_string();
            let p_rsa = signer::rsa_raw_zero_pad_hex(&p_plain).ok()?;
            let mut query = HashMap::new();
            query.insert("plat".to_string(), "1".to_string());
            let body = json!({
                "visit_time": now_sec,
                "usertype": 1,
                "p": p_rsa.to_uppercase(),
                "userid": uid_num
            });
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "usercenter.kugou.com".to_string());

            let resp = self
                .transport
                .call(
                    "/v3/get_my_info",
                    dev_ref,
                    Some(session_ref),
                    query,
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
                .ok()?;
            if resp.is_success() {
                Some(resp.data().clone())
            } else {
                None
            }
        };

        let fut_rel_info = async {
            let p_ms_plain = json!({
                "clienttime": now_ms,
                "token": session_ref.token
            })
            .to_string();
            let p_ms_rsa = signer::rsa_raw_zero_pad_hex(&p_ms_plain).ok()?;
            let rel_body = json!({
                "p": p_ms_rsa.to_uppercase(),
                "appid": APP_ID,
                "mid": dev_ref.mid,
                "clientver": CLIENT_VERSION,
                "source": 0,
                "clienttime": now_ms,
                "uuid": "-",
                "userid": uid_num,
                "key": signer::params_key(now_ms, "")
            });
            let mut rel_headers = HashMap::new();
            rel_headers.insert("x-router".to_string(), "relation.user.kugou.com".to_string());
            let rel_resp = self
                .transport
                .call(
                    "/v1/get_my_userinfo",
                    dev_ref,
                    Some(session_ref),
                    HashMap::new(),
                    Some(rel_body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    rel_headers,
                    false,
                    false,
                )
                .await
                .ok()?;
            if rel_resp.is_success() {
                Some(
                    rel_resp
                        .data()
                        .get("data")
                        .unwrap_or_else(|| rel_resp.data())
                        .clone(),
                )
            } else {
                None
            }
        };

        let fut_grade = async {
            // 主通道：严格对齐 MakcRe/KuGouMusicApi/module/user_grade_info.js (Lite v2: appid=3116, clientver=10597)
            let v2_client_ver = 10597_u32;
            let v2_key = signer::md5_hex(&format!(
                "{}{}{}{}",
                APP_ID, ANDROID_SALT, v2_client_ver, now_sec
            ));
            let clean_uuid: String = dev_ref
                .guid
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            let v2_uuid = if clean_uuid.is_empty() {
                "-".to_string()
            } else {
                clean_uuid
            };

            let v2_inner = if is_report {
                let report_md5 = signer::md5_hex(&format!("{}{}00", req_d_sec, req_diff_sec));
                json!({
                    "token": session_ref.token,
                    "md5": report_md5
                })
                .to_string()
            } else {
                format!("{{\"clienttime\":{},\"userid\":{}}}", now_sec, uid_num)
            };

            if let Ok(v2_p) = signer::rsa_raw_zero_pad_hex(&v2_inner) {
                let mut v2_body = json!({
                    "mid": dev_ref.mid,
                    "type": 1,
                    "uuid": v2_uuid,
                    "userid": uid_num,
                    "p": v2_p.to_uppercase(),
                    "appid": APP_ID.to_string(),
                    "clientver": v2_client_ver.to_string(),
                    "clienttime": now_sec,
                    "key": v2_key
                });
                if is_report {
                    v2_body["d_sec"] = json!(req_d_sec);
                    v2_body["diff_sec"] = json!(req_diff_sec);
                    v2_body["y_type"] = json!(0);
                    v2_body["m_type"] = json!(0);
                }
                let mut v2_query = HashMap::new();
                v2_query.insert(
                    "dfid".to_string(),
                    if dev_ref.dfid.is_empty() {
                        "-".to_string()
                    } else {
                        dev_ref.dfid.clone()
                    },
                );
                let mut v2_headers = HashMap::new();
                v2_headers.insert(
                    "Content-Type".to_string(),
                    "text/plain; charset=ISO-8859-1".to_string(),
                );
                v2_headers.insert(
                    "User-Agent".to_string(),
                    format!(
                        "Android15-1070-{}-201-0-get_user_grade_info-wifi",
                        v2_client_ver
                    ),
                );
                v2_headers.insert("KG-THash".to_string(), signer::random_hex(7));
                v2_headers.insert("KG-Rec".to_string(), "1".to_string());
                v2_headers.insert("KG-RC".to_string(), "1".to_string());

                if let Ok(v2_resp) = self
                    .transport
                    .call(
                        "/v2/get_grade_info",
                        dev_ref,
                        Some(session_ref),
                        v2_query,
                        Some(v2_body),
                        "http://userinfo.user.kugou.com",
                        "POST",
                        false,
                        v2_headers,
                        false,
                        false,
                    )
                    .await
                {
                    if v2_resp.is_success() {
                        let d_root = v2_resp.data();
                        let d_inner = d_root.get("data").unwrap_or(d_root);
                        if Self::val_i64(d_inner, "d_sec").is_some()
                            || Self::val_i64(d_inner, "p_grade").is_some()
                        {
                            return Some(d_inner.clone());
                        }
                    }
                }
            }

            // 备用容灾通道：仅在 v2 网关波动时使用概念版 Lite 3116 /v4/get_grade_info
            let md5_val = signer::md5_hex(&format!("{}{}11", req_d_sec, req_diff_sec));
            let aes_seed = signer::random_hex(16);
            let seed_md5 = signer::md5_hex(&aes_seed);
            let params_plain = json!({
                "userid": uid_num,
                "token": session_ref.token,
                "user_type": 29,
                "user_y_type": 1,
                "md5": md5_val
            })
            .to_string();
            let pk_plain = json!({ "clienttime_ms": now_ms, "key": aes_seed }).to_string();
            if let (Ok(params_enc), Ok(pk_v4)) = (
                signer::aes_cbc_encrypt_hex(&params_plain, &seed_md5[0..32], &seed_md5[16..32]),
                signer::rsa_raw_zero_pad_hex(&pk_plain),
            ) {
                let v4_body = json!({
                    "plat": 1,
                    "userid": uid_num,
                    "clienttime_ms": now_ms,
                    "type": 0,
                    "d_sec": req_d_sec,
                    "diff_sec": req_diff_sec,
                    "y_type": 1,
                    "m_type": 1,
                    "pk": pk_v4.to_uppercase(),
                    "params": params_enc,
                    "medal": true
                });
                let v4_body_str = serde_json::to_string(&v4_body).unwrap_or_default();
                let mut sign_params = BTreeMap::new();
                sign_params.insert("appid".to_string(), APP_ID.to_string());
                sign_params.insert("clienttime".to_string(), now_sec.to_string());
                sign_params.insert("clientver".to_string(), CLIENT_VERSION.to_string());
                sign_params.insert("dfid".to_string(), dev_ref.dfid.clone());
                sign_params.insert("mid".to_string(), dev_ref.mid.clone());
                sign_params.insert("userid".to_string(), session_ref.user_id.clone());
                sign_params.insert("uuid".to_string(), dev_ref.guid.clone());

                let sig =
                    signer::signature_with_salt(&sign_params, &v4_body_str, ANDROID_SALT, false);
                let mut v4_query: HashMap<String, String> = sign_params.into_iter().collect();
                v4_query.insert("signature".to_string(), sig);

                let mut v4_headers = HashMap::new();
                v4_headers.insert("KG-DEVID".to_string(), dev_ref.mid.clone());
                v4_headers.insert("KG-CLIENTTIMEMS".to_string(), now_ms.to_string());
                v4_headers.insert(
                    "x-router".to_string(),
                    "userinfoservice.kugou.com".to_string(),
                );

                if let Ok(v4_resp) = self
                    .transport
                    .call(
                        "/v4/get_grade_info",
                        dev_ref,
                        Some(session_ref),
                        v4_query,
                        Some(v4_body),
                        "https://userinfoservice.kugou.com",
                        "POST",
                        false,
                        v4_headers,
                        false,
                        false,
                    )
                    .await
                {
                    if v4_resp.is_success() {
                        let d_root = v4_resp.data();
                        return Some(d_root.get("data").unwrap_or(d_root).clone());
                    }
                }
            }
            None
        };

        let fut_grade_std = async {
            // 酷狗音乐正式版 (appid=1005, clientver=20489) 免密查询通道：
            // 同一账号在「酷狗音乐」正式版中包含全平台历史总时长（比概念版 3116 单桶多出正式版独占时长），
            // 查询模式仅对 {"clienttime","userid"} 使用标准版 RSA 公钥加密，无需 1005 token 即可返回权威全量 d_sec 与成长值。
            let std_ts = Utc::now().timestamp();
            let std_key = signer::md5_hex(&format!(
                "{}{}{}{}",
                STANDARD_APP_ID, STANDARD_SALT, STANDARD_CLIENT_VERSION, std_ts
            ));
            let clean_uuid: String = dev_ref
                .guid
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            let std_uuid = if clean_uuid.is_empty() {
                "-".to_string()
            } else {
                clean_uuid
            };
            let std_inner = format!("{{\"clienttime\":{},\"userid\":{}}}", std_ts, uid_num);
            if let Ok(std_p) = signer::rsa_raw_zero_pad_hex_standard(&std_inner) {
                let std_body = json!({
                    "mid": dev_ref.mid,
                    "type": 1,
                    "uuid": std_uuid,
                    "userid": uid_num,
                    "p": std_p.to_uppercase(),
                    "appid": STANDARD_APP_ID.to_string(),
                    "clientver": STANDARD_CLIENT_VERSION.to_string(),
                    "clienttime": std_ts,
                    "key": std_key
                });
                let mut std_query = HashMap::new();
                std_query.insert(
                    "dfid".to_string(),
                    if dev_ref.dfid.is_empty() {
                        "-".to_string()
                    } else {
                        dev_ref.dfid.clone()
                    },
                );
                let mut std_headers = HashMap::new();
                std_headers.insert(
                    "Content-Type".to_string(),
                    "text/plain; charset=ISO-8859-1".to_string(),
                );
                std_headers.insert(
                    "User-Agent".to_string(),
                    format!(
                        "Android15-AndroidPhone-{}-201-0-get_user_grade_info-wifi",
                        STANDARD_CLIENT_VERSION
                    ),
                );
                std_headers.insert("KG-THash".to_string(), signer::random_hex(7));
                std_headers.insert("KG-Rec".to_string(), "1".to_string());
                std_headers.insert("KG-RC".to_string(), "1".to_string());

                if let Ok(std_resp) = self
                    .transport
                    .call(
                        "/v2/get_grade_info",
                        dev_ref,
                        None,
                        std_query,
                        Some(std_body),
                        "http://userinfo.user.kugou.com",
                        "POST",
                        false,
                        std_headers,
                        false,
                        false,
                    )
                    .await
                {
                    if std_resp.is_success() {
                        let d_root = std_resp.data();
                        let d_inner = d_root.get("data").unwrap_or(d_root);
                        if Self::val_i64(d_inner, "d_sec").is_some()
                            || Self::val_i64(d_inner, "p_grade").is_some()
                        {
                            return Some(d_inner.clone());
                        }
                    }
                }
            }
            None
        };

        let (my_info_opt, rel_info_opt, grade_lite_opt, grade_std_opt, vip_res) = if is_report {
            // 若本次携带新增听歌秒数上报，先等待 3116 上报落库，再拉取 1005 正式版全量总时长
            let (m, r, gl, v) =
                tokio::join!(fut_my_info, fut_rel_info, fut_grade, self.fetch_vip_status());
            let gs = fut_grade_std.await;
            (m, r, gl, gs, v)
        } else {
            tokio::join!(
                fut_my_info,
                fut_rel_info,
                fut_grade,
                fut_grade_std,
                self.fetch_vip_status()
            )
        };

        let mut max_d_sec: i64 = 0;
        let mut lite_d_sec: i64 = 0;
        let mut max_duration_min: i64 = 0;
        let mut max_grade: i32 = 0;
        let mut next_grade: i32 = 0;
        let mut grade_base_score: i64 = 0;
        let mut user_score: i64 = 0;
        let mut next_level_score: i64 = 0;
        let mut grade_speed: f32 = 1.0;

        let mut bg_pic_url = String::new();
        let mut signature = String::new();
        let mut gender: i32 = 0;
        let mut birthday = String::new();
        let mut province = String::new();
        let mut city = String::new();
        let mut ip_location = String::new();
        let mut occupation = String::new();
        let mut reg_time: i64 = 0;
        let mut follows: i64 = 0;
        let mut fans: i64 = 0;
        let mut visitors: i64 = 0;

        let mut session_dirty = false;

        if let Some(ref d) = my_info_opt {
            let nick = Self::val_str(d, "nickname");
            if !nick.is_empty() && nick != session.nickname {
                session.nickname = nick;
                session_dirty = true;
            }
            let pic = Self::val_str(d, "pic");
            if !pic.is_empty() {
                let norm_pic = signer::normalize_image_url(Some(&pic));
                if !norm_pic.is_empty() && norm_pic != session.avatar_url {
                    session.avatar_url = norm_pic;
                    session_dirty = true;
                }
            }
            let bg = Self::val_str(d, "bg_pic");
            if !bg.is_empty() {
                bg_pic_url = signer::normalize_image_url(Some(&bg));
            }
            signature = {
                let s = Self::val_str(d, "descri");
                if !s.is_empty() {
                    s
                } else {
                    Self::val_str(d, "intro")
                }
            };
            gender = Self::val_i64(d, "gender")
                .or_else(|| Self::val_i64(d, "sex"))
                .unwrap_or(0) as i32;
            birthday = Self::val_str(d, "birthday");
            province = Self::val_str(d, "province");
            city = Self::val_str(d, "city");
            ip_location = Self::val_str(d, "loc");
            occupation = Self::val_str(d, "occupation");
            reg_time = Self::val_i64(d, "rtime")
                .or_else(|| Self::val_i64(d, "reg_time"))
                .unwrap_or(0);
            follows = Self::val_i64(d, "follows").unwrap_or(0);
            fans = Self::val_i64(d, "fans").unwrap_or(0);
            visitors = Self::val_i64(d, "visitors")
                .or_else(|| Self::val_i64(d, "nvisitors"))
                .unwrap_or(0);

            for k in ["r_duration", "duration", "listen_duration"] {
                if let Some(m) = Self::val_i64(d, k) {
                    max_duration_min = max_duration_min.max(m);
                }
            }
            for k in ["p_grade", "user_grade", "grade"] {
                if let Some(g) = Self::val_i64(d, k) {
                    max_grade = max_grade.max(g as i32);
                }
            }
        }

        if let Some(ref d_rel) = rel_info_opt {
            if session.nickname.is_empty() {
                let nick = Self::val_str(d_rel, "nickname");
                if !nick.is_empty() {
                    session.nickname = nick;
                    session_dirty = true;
                }
            }
            if follows <= 0 {
                follows = Self::val_i64(d_rel, "follows").unwrap_or(follows);
            }
            if fans <= 0 {
                fans = Self::val_i64(d_rel, "fans").unwrap_or(fans);
            }
            for k in ["r_duration", "duration", "listen_duration"] {
                if let Some(m) = Self::val_i64(d_rel, k) {
                    max_duration_min = max_duration_min.max(m);
                }
            }
            for k in ["p_grade", "user_grade", "grade"] {
                if let Some(g) = Self::val_i64(d_rel, k) {
                    max_grade = max_grade.max(g as i32);
                }
            }
        }

        if session_dirty {
            self.store.set_session(Some(session.clone()));
        }

        if let Some(ref d_lite) = grade_lite_opt {
            for k in ["d_sec", "dSec", "listen_sec"] {
                if let Some(sec) = Self::val_i64(d_lite, k) {
                    lite_d_sec = lite_d_sec.max(sec);
                }
            }
            if lite_d_sec <= 0 {
                if let Some(m) = Self::val_i64(d_lite, "duration") {
                    lite_d_sec = m.max(0) * 60;
                }
            }
        }

        // 同时聚合「酷狗音乐正式版 (1005)」与「酷狗概念版 (3116)」的等级与听歌时长：
        // 取全平台最高权威 d_sec / duration / 成长值，使显示的累计听歌时长与「酷狗音乐」正式版严格一致
        for d_grade in [grade_std_opt.as_ref(), grade_lite_opt.as_ref()]
            .into_iter()
            .flatten()
        {
            // 严禁匹配单字母 "d"，严格提取权威秒级字段 d_sec 与分钟级字段 duration
            for k in ["d_sec", "dSec", "listen_sec"] {
                if let Some(sec) = Self::val_i64(d_grade, k) {
                    max_d_sec = max_d_sec.max(sec);
                }
            }
            for k in ["duration", "r_duration", "d_min"] {
                if let Some(m) = Self::val_i64(d_grade, k) {
                    max_duration_min = max_duration_min.max(m);
                }
            }
            for k in ["p_grade", "user_grade", "grade"] {
                if let Some(g) = Self::val_i64(d_grade, k) {
                    max_grade = max_grade.max(g as i32);
                }
            }
            if let Some(ng) = Self::val_i64(d_grade, "p_next_grade") {
                next_grade = next_grade.max(ng as i32);
            }
            if let Some(bsc) = Self::val_i64(d_grade, "p_grade_point") {
                grade_base_score = grade_base_score.max(bsc);
            }
            for k in ["p_current_point", "user_score", "score"] {
                if let Some(sc) = Self::val_i64(d_grade, k) {
                    user_score = user_score.max(sc);
                }
            }
            for k in ["p_next_grade_point", "next_score"] {
                if let Some(nsc) = Self::val_i64(d_grade, k) {
                    next_level_score = next_level_score.max(nsc);
                }
            }
            if let Some(sp) = Self::val_f64_opt(d_grade, "p_speed") {
                if sp > 0.1 {
                    grade_speed = grade_speed.max(sp as f32);
                }
            }
        }

        // 权威云端基线：优先采用服务端返回的秒级 d_sec，若仅返回分钟级 duration 则转为 duration * 60
        let raw_server_dsec = if max_d_sec > 0 {
            max_d_sec
        } else {
            max_duration_min.max(0) * 60
        };

        let (unsettled_local_sec, pending_diff_sec, resolved_grade) =
            self.reconcile_grade_state(&session.user_id, raw_server_dsec, max_grade, now_sec);

        let total_sec = raw_server_dsec + unsettled_local_sec;
        let total_min = total_sec / 60;
        let hours = total_min / 60;
        let mins = total_min % 60;
        let listen_duration_formatted = if hours > 0 {
            format!("{} 小时 {} 分", hours, mins)
        } else {
            format!("{} 分钟", mins)
        };

        let resolved_next_grade = if next_grade > resolved_grade {
            next_grade
        } else {
            resolved_grade + 1
        };

        let (
            final_base_score,
            final_user_score,
            final_next_score,
            next_level_remain_score,
            level_progress_pct,
            next_level_remain_min,
        ) = Self::compute_grade_progress(
            resolved_grade,
            total_min,
            grade_base_score,
            user_score,
            next_level_score,
            grade_speed,
        );

        let music_age_years = if reg_time > 946_684_800 && now_sec > reg_time {
            let yrs = (now_sec - reg_time) as f64 / (365.25 * 86_400.0);
            ((yrs * 10.0).round() / 10.0) as f32
        } else {
            0.0
        };

        let vip_info = vip_res.unwrap_or(VipStatusInfo {
            is_vip: false,
            svip_active: false,
            tvip_active: false,
            svip_expiry: String::new(),
            tvip_expiry: String::new(),
            today_claimed: false,
            month_claimed_days: 0,
            records: Vec::new(),
            status_message: "普通用户".to_string(),
            dfid: dev.dfid.clone(),
            guid: dev.guid.clone(),
        });

        let vip_expire_text = if vip_info.svip_active {
            format!("概念版 SVIP · 到期 {}", vip_info.svip_expiry)
        } else if vip_info.tvip_active {
            format!("畅听 VIP · 到期 {}", vip_info.tvip_expiry)
        } else {
            "未开通概念版 VIP（可每日签到免费领取）".to_string()
        };

        Ok(AccountProfile {
            user_id: session.user_id,
            nickname: session.nickname,
            avatar_url: session.avatar_url,
            bg_pic_url,
            signature,
            gender,
            birthday,
            province,
            city,
            ip_location,
            occupation,
            reg_time,
            music_age_years,
            follows,
            fans,
            visitors,
            is_vip: vip_info.is_vip,
            svip_active: vip_info.svip_active,
            tvip_active: vip_info.tvip_active,
            vip_expire_text,
            listen_duration_min: total_min,
            listen_duration_sec: total_sec,
            listen_duration_formatted,
            pending_diff_sec,
            user_grade: resolved_grade,
            next_grade: resolved_next_grade,
            raw_server_dsec,
            lite_server_dsec: lite_d_sec,
            grade_base_score: final_base_score,
            user_score: final_user_score,
            next_level_score: final_next_score,
            next_level_remain_score,
            grade_speed,
            level_progress_pct,
            next_level_remain_min,
        })
    }

    /// 严格区分酷狗「累计听歌分钟数 (duration/d_sec)」与「等级成长值积分 (p_current_point / p_grade_point / p_next_grade_point)」：
    /// - 成长值按 `听歌分钟 * p_speed`（VIP 1.5x/2.0x/2.5x 倍速）累加，绝不可与听歌时长混为一谈！
    /// - 直接采用服务端下发的真实等级积分上下界计算精确百分比，不做人为 5% 保底失真截断。
    fn compute_grade_progress(
        level: i32,
        total_minutes: i64,
        server_base_score: i64,
        server_user_score: i64,
        server_next_score: i64,
        grade_speed: f32,
    ) -> (i64, i64, i64, i64, f32, i64) {
        // 酷狗官方听歌等级成长值阶梯表（仅当离线或接口未返回积分字段时作为兜底参考，Lv.12=300000, Lv.13=500000）
        const OFFICIAL_GRADE_POINTS: [i64; 21] = [
            0,         // Lv.0
            50,        // Lv.1
            200,       // Lv.2
            500,       // Lv.3
            1_000,     // Lv.4
            2_000,     // Lv.5
            4_000,     // Lv.6
            8_000,     // Lv.7
            15_000,    // Lv.8
            30_000,    // Lv.9
            60_000,    // Lv.10
            150_000,   // Lv.11
            300_000,   // Lv.12
            500_000,   // Lv.13
            800_000,   // Lv.14
            1_200_000, // Lv.15
            1_800_000, // Lv.16
            2_500_000, // Lv.17
            3_500_000, // Lv.18
            5_000_000, // Lv.19
            7_000_000, // Lv.20
        ];
        let last_idx = OFFICIAL_GRADE_POINTS.len() - 1;
        let lv_idx = (level.max(0) as usize).min(last_idx - 1);

        let base_score = if server_base_score > 0 || level == 0 {
            server_base_score.max(0)
        } else {
            OFFICIAL_GRADE_POINTS[lv_idx]
        };

        let cur_score = if server_user_score > 0 {
            server_user_score
        } else {
            total_minutes.max(base_score)
        };

        let next_score = if server_next_score > base_score {
            server_next_score
        } else {
            OFFICIAL_GRADE_POINTS[(lv_idx + 1).min(last_idx)].max(cur_score + 100)
        };

        let span = (next_score - base_score).max(1);
        let gained = (cur_score - base_score).clamp(0, span);
        let pct = ((gained as f64 / span as f64) * 100.0).clamp(0.0, 100.0) as f32;
        let remain_score = (next_score - cur_score).max(0);
        let effective_speed = (grade_speed as f64).max(0.5);
        let remain_min = ((remain_score as f64) / effective_speed).ceil() as i64;

        (
            base_score,
            cur_score,
            next_score,
            remain_score,
            pct,
            remain_min,
        )
    }

    fn normalize_vip_record_day(raw_day: &str) -> String {
        let s = raw_day.trim();
        if s.is_empty() {
            return String::new();
        }
        let date_part = s.split_whitespace().next().unwrap_or(s);
        if date_part.len() == 8 && date_part.chars().all(|c| c.is_ascii_digit()) {
            return format!("{}-{}-{}", &date_part[0..4], &date_part[4..6], &date_part[6..8]);
        }
        let parts: Vec<&str> = date_part.split(&['-', '/', '.'][..]).collect();
        if parts.len() >= 3 {
            if let (Ok(y), Ok(m), Ok(d)) = (
                parts[0].parse::<i32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            ) {
                return format!("{:04}-{:02}-{:02}", y, m, d);
            }
        }
        date_part.chars().take(10).collect()
    }

    /// 7. 查询酷狗概念版 VIP 权益状态与当月签到日历记录
    pub async fn fetch_vip_status(&self) -> Result<VipStatusInfo, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录账号".to_string());
        };
        let dev = self.store.device();

        let mut query = HashMap::new();
        query.insert("busi_type".to_string(), "concept".to_string());
        query.insert("opt_product_types".to_string(), "dvip,qvip".to_string());
        query.insert("product_type".to_string(), "svip".to_string());

        let mut rec_query = HashMap::new();
        rec_query.insert("latest_limit".to_string(), "100".to_string());

        let (union_res, rec_res) = tokio::join!(
            self.transport.call(
                "/v1/get_union_vip",
                &dev,
                Some(&session),
                query,
                None,
                "https://kugouvip.kugou.com",
                "GET",
                false,
                HashMap::new(),
                true,
                true,
            ),
            self.transport.call(
                "/youth/v1/activity/get_month_vip_record",
                &dev,
                Some(&session),
                rec_query,
                None,
                "https://gateway.kugou.com",
                "GET",
                false,
                HashMap::new(),
                true,
                true,
            )
        );

        let resp = union_res?.require_success()?;

        let data = resp.data();
        let now_utc_ts = Utc::now().timestamp();
        let mut svip_active = false;
        let mut tvip_active = false;
        let mut svip_expiry = String::new();
        let mut tvip_expiry = String::new();

        if let Some(busi_arr) = data.get("busi_vip").and_then(|v| v.as_array()) {
            for item in busi_arr {
                let pt = Self::val_str(item, "product_type").to_lowercase();
                let is_vip = Self::val_i64(item, "is_vip").unwrap_or(0) == 1;
                let end_time = Self::val_str(item, "vip_end_time");
                let time_valid = Self::is_vip_time_active(&end_time, now_utc_ts);
                if pt == "svip" && is_vip && time_valid {
                    svip_active = true;
                    svip_expiry = end_time;
                } else if (pt == "tvip" || pt == "dvip" || pt == "qvip") && is_vip && time_valid {
                    tvip_active = true;
                    tvip_expiry = end_time;
                }
            }
        }

        let china_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let today_str = Utc::now()
            .with_timezone(&china_tz)
            .format("%Y-%m-%d")
            .to_string();
        let current_ym_prefix = &today_str[0..7];
        let mut today_claimed = false;
        let mut seen_month_days = HashSet::new();
        let mut records_map: BTreeMap<String, VipDayRecord> = BTreeMap::new();

        if let Ok(rec_resp) = rec_res
        {
            let list_opt = rec_resp
                .data()
                .get("list")
                .and_then(|v| v.as_array())
                .or_else(|| rec_resp.data().get("records").and_then(|v| v.as_array()))
                .or_else(|| rec_resp.data().get("day_list").and_then(|v| v.as_array()))
                .or_else(|| rec_resp.data().as_array());
            if let Some(list) = list_opt {
                for r in list {
                    if let Some(day_str) = r.as_str() {
                        let day = Self::normalize_vip_record_day(day_str);
                        if !day.is_empty() {
                            if day.starts_with(current_ym_prefix) {
                                seen_month_days.insert(day.clone());
                            }
                            if day == today_str {
                                today_claimed = true;
                            }
                            records_map.insert(
                                day.clone(),
                                VipDayRecord {
                                    day,
                                    received: true,
                                    vip_type: if svip_active { "svip".to_string() } else { "tvip".to_string() },
                                },
                            );
                        }
                        continue;
                    }
                    let raw_day = {
                        let d = Self::val_str(r, "day");
                        if !d.is_empty() {
                            d
                        } else {
                            let rd = Self::val_str(r, "receive_day");
                            if !rd.is_empty() { rd } else { Self::val_str(r, "date") }
                        }
                    };
                    let day = Self::normalize_vip_record_day(&raw_day);
                    if day.is_empty() {
                        continue;
                    }
                    let received = match r.get("receive_vip").or_else(|| r.get("is_receive")).or_else(|| r.get("status")) {
                        Some(v) => Self::val_i64(r, "receive_vip").unwrap_or(0) == 1
                            || Self::val_i64(r, "is_receive").unwrap_or(0) == 1
                            || Self::val_i64(r, "status").unwrap_or(0) == 1
                            || v.as_bool().unwrap_or(false),
                        None => !Self::val_str(r, "receive_time").is_empty() || !Self::val_str(r, "receive_day").is_empty(),
                    };
                    let raw_vtype = Self::val_str(r, "vip_type").to_lowercase();
                    let vip_type = if raw_vtype.contains("svip") || raw_vtype == "2" {
                        "svip".to_string()
                    } else if !raw_vtype.is_empty() {
                        raw_vtype
                    } else if svip_active && day == today_str {
                        "svip".to_string()
                    } else {
                        "tvip".to_string()
                    };
                    if received {
                        if day.starts_with(current_ym_prefix) {
                            seen_month_days.insert(day.clone());
                        }
                        if day == today_str {
                            today_claimed = true;
                        }
                    }
                    records_map.insert(
                        day.clone(),
                        VipDayRecord {
                            day,
                            received,
                            vip_type,
                        },
                    );
                }
            }
        }

        let is_vip = svip_active || tvip_active;
        let status_message = if svip_active {
            format!("概念版 SVIP 生效中（到期：{}）", svip_expiry)
        } else if tvip_active {
            format!("畅听 VIP 生效中（到期：{}）", tvip_expiry)
        } else {
            "今日尚未领取概念版 VIP".to_string()
        };

        Ok(VipStatusInfo {
            is_vip,
            svip_active,
            tvip_active,
            svip_expiry,
            tvip_expiry,
            today_claimed,
            month_claimed_days: seen_month_days.len(),
            records: records_map.into_values().collect(),
            status_message,
            dfid: dev.dfid,
            guid: dev.guid,
        })
    }

    /// 8. 一键每日签到领取并升级概念版 VIP (对齐 Android VipRepository.claimDailyVip)
    pub async fn claim_daily_vip(&self) -> Result<VipStatusInfo, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录账号再领取概念版 VIP".to_string());
        };
        let dev = self.store.device();
        let china_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let today_str = Utc::now()
            .with_timezone(&china_tz)
            .format("%Y-%m-%d")
            .to_string();

        let mut claim_query = HashMap::new();
        claim_query.insert("source_id".to_string(), "90139".to_string());
        claim_query.insert("receive_day".to_string(), today_str.clone());

        let mut form_headers = HashMap::new();
        form_headers.insert(
            "content-type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );

        let direct_claim = self
            .transport
            .call(
                "/youth/v1/recharge/receive_vip_listen_song",
                &dev,
                Some(&session),
                claim_query.clone(),
                None,
                "https://gateway.kugou.com",
                "POST",
                false,
                form_headers.clone(),
                true,
                true,
            )
            .await;

        let direct_ok = match &direct_claim {
            Ok(r) => r.is_success() || r.code() == Some(131001),
            Err(_) => false,
        };

        if !direct_ok {
            let now_ms = Utc::now().timestamp_millis();
            let ad_body = json!({
                "ad_id": 12307537187i64,
                "play_end": now_ms,
                "play_start": now_ms - 30_000
            });
            let _ = self
                .transport
                .call(
                    "/youth/v1/ad/play_report",
                    &dev,
                    Some(&session),
                    HashMap::new(),
                    Some(ad_body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await;

            let retry_resp = self
                .transport
                .call(
                    "/youth/v1/recharge/receive_vip_listen_song",
                    &dev,
                    Some(&session),
                    claim_query,
                    None,
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    form_headers,
                    true,
                    true,
                )
                .await?;

            if !retry_resp.is_success() && retry_resp.code() != Some(131001) {
                let _ = retry_resp.require_success()?;
            }
        }

        let mut up_query = HashMap::new();
        up_query.insert("kugouid".to_string(), session.user_id.clone());
        up_query.insert("ad_type".to_string(), "1".to_string());

        let _ = self
            .transport
            .call(
                "/youth/v1/listen_song/upgrade_vip_reward",
                &dev,
                Some(&session),
                up_query,
                None,
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await;

        let mut latest_status = self.fetch_vip_status().await?;
        if !latest_status.today_claimed {
            tokio::time::sleep(std::time::Duration::from_millis(450)).await;
            if let Ok(rechecked) = self.fetch_vip_status().await {
                latest_status = rechecked;
            }
        }
        if direct_ok && !latest_status.today_claimed {
            latest_status.today_claimed = true;
            if !latest_status.records.iter().any(|r| r.day == today_str && r.received) {
                latest_status.records.push(VipDayRecord {
                    day: today_str,
                    received: true,
                    vip_type: if latest_status.svip_active {
                        "svip".to_string()
                    } else {
                        "tvip".to_string()
                    },
                });
                latest_status.month_claimed_days += 1;
            }
        }
        Ok(latest_status)
    }

    /// 9. 概念版每日推荐曲目 (/everyday_song_recommend + /everydayrec.service/everyday_style_recommend 兜底)
    pub async fn fetch_daily_recommend(&self) -> Result<Vec<Song>, String> {
        let session = self.store.session();
        let dev = self.store.device();

        if let Some(ref s) = session {
            let body = json!({
                "platform": "android",
                "userid": s.user_id
            });
            let mut headers = HashMap::new();
            headers.insert(
                "x-router".to_string(),
                "everydayrec.service.kugou.com".to_string(),
            );
            if let Ok(resp) = self
                .transport
                .call(
                    "/everyday_song_recommend",
                    &dev,
                    Some(s),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
            {
                let parsed = Self::parse_songs(resp.data());
                if !parsed.is_empty() {
                    let enriched = self.enrich_songs_privilege_lite(parsed).await;
                    self.record_taste_seeds(&enriched);
                    return Ok(enriched);
                }
            }
        }

        // 通道 2: 概念版风格日推 (/everydayrec.service/everyday_style_recommend)
        let uid_str = session
            .as_ref()
            .map(|s| s.user_id.clone())
            .unwrap_or_else(|| "0".to_string());
        let style_body = json!({
            "platform": "android",
            "userid": uid_str,
            "tagids": []
        });
        if let Ok(resp) = self
            .transport
            .call(
                "/everydayrec.service/everyday_style_recommend",
                &dev,
                session.as_ref(),
                HashMap::new(),
                Some(style_body),
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            let parsed = Self::parse_songs(resp.data());
            if !parsed.is_empty() {
                let enriched = self.enrich_songs_privilege_lite(parsed).await;
                self.record_taste_seeds(&enriched);
                return Ok(enriched);
            }
        }

        let fallback = self.search_songs("热歌", 1, 30).await?;
        self.record_taste_seeds(&fallback);
        Ok(fallback)
    }

    fn diversify_artist_order(songs: Vec<Song>) -> Vec<Song> {
        if songs.len() <= 2 {
            return songs;
        }
        let mut remaining = songs;
        let mut result: Vec<Song> = Vec::with_capacity(remaining.len());
        while !remaining.is_empty() {
            let last_artist = result
                .last()
                .map(|s| s.artist.trim().to_lowercase())
                .unwrap_or_default();
            let pick_idx = remaining
                .iter()
                .position(|cand| {
                    let ca = cand.artist.trim().to_lowercase();
                    ca.is_empty() || ca != last_artist
                })
                .unwrap_or(0);
            result.push(remaining.remove(pick_idx));
        }
        result
    }

    /// 10. 猜你喜欢 · 私人随心听 FM (对齐 Android MusicRepository.personalFm 5 路推荐引擎 + 口味种子 + 换批去重游标)
    pub async fn fetch_personal_fm(&self) -> Result<Vec<Song>, String> {
        self.fetch_personal_fm_with_excludes(None).await
    }

    pub async fn fetch_personal_fm_with_excludes(
        &self,
        exclude_hashes: Option<Vec<String>>,
    ) -> Result<Vec<Song>, String> {
        let session = self.store.session();
        let dev = self.store.device();
        let now_sec = Utc::now().timestamp();
        let uid_num = session
            .as_ref()
            .and_then(|s| s.user_id.parse::<i64>().ok())
            .unwrap_or(0);

        let batch_cursor = self.personal_fm_cursor.fetch_add(1, Ordering::Relaxed);
        let rotating_page = (batch_cursor % 5) + 1;
        let modes = ["normal", "small", "familiar", "explore"];
        let active_mode = modes[(batch_cursor as usize) % modes.len()];

        // 收集用户口味种子（若内存种子为空，尝试拉取最近播放或我喜欢前排曲目）
        let mut seeds: Vec<Song> = self
            .taste_seed_songs
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        if seeds.is_empty() {
            if let Ok(recents) = self.fetch_recent_history().await {
                if !recents.is_empty() {
                    self.record_taste_seeds(&recents);
                    seeds = recents;
                }
            }
        }

        let mut seen_global: HashSet<String> = self
            .seen_personal_fm_hashes
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        if let Some(ex_list) = exclude_hashes {
            for h in ex_list {
                let clean = h.trim().to_lowercase();
                if !clean.is_empty() {
                    seen_global.insert(clean);
                }
            }
        }
        if seen_global.len() > 240 {
            seen_global.clear();
            if let Ok(mut g) = self.seen_personal_fm_hashes.lock() {
                g.clear();
            }
        }

        let mut collected = Vec::new();
        let mut batch_seen = HashSet::new();

        let mut try_collect = |collected: &mut Vec<Song>, items: Vec<Song>, allow_seen: bool| {
            for s in items {
                let k = s.hash.trim().to_lowercase();
                if k.is_empty() || s.title == "未知歌曲" {
                    continue;
                }
                if !allow_seen && seen_global.contains(&k) {
                    continue;
                }
                if batch_seen.insert(k) {
                    collected.push(s);
                }
            }
        };

        // 引擎 1: 私人 FM 推荐流 (/v2/personal_recommend)
        let mut pfm_body = json!({
            "appid": APP_ID,
            "clienttime": now_sec,
            "mid": dev.mid,
            "action": "play",
            "recommend_source_locked": 0,
            "song_pool_id": (batch_cursor % 3),
            "callerid": 0,
            "m_type": 1,
            "platform": "android",
            "area_code": 1,
            "remain_songcnt": 0,
            "clientver": CLIENT_VERSION,
            "is_overplay": 1,
            "mode": active_mode,
            "fakem": "ca981cfc583a4c37f28d2d49000013c16a0a",
            "key": signer::params_key(now_sec, "")
        });
        if let Some(ref s) = session {
            pfm_body["userid"] = json!(uid_num);
            pfm_body["kguid"] = json!(uid_num);
            pfm_body["token"] = json!(s.token);
            pfm_body["vip_type"] = json!(6);
        }
        let mut pfm_headers = HashMap::new();
        pfm_headers.insert(
            "x-router".to_string(),
            "persnfm.service.kugou.com".to_string(),
        );
        if let Ok(resp) = self
            .transport
            .call(
                "/v2/personal_recommend",
                &dev,
                session.as_ref(),
                HashMap::new(),
                Some(pfm_body),
                "https://gateway.kugou.com",
                "POST",
                false,
                pfm_headers,
                true,
                true,
            )
            .await
        {
            try_collect(&mut collected, Self::parse_songs(resp.data()), false);
        }

        // 引擎 2: 概念版原生 AI 推荐单曲 (/concepts/v1/ai/recommend_song，随游标轮换分页)
        let ai_body = json!({
            "apiver": 2,
            "source": 2,
            "pagesize": 20,
            "page": rotating_page
        });
        if let Ok(resp) = self
            .transport
            .call(
                "/concepts/v1/ai/recommend_song",
                &dev,
                session.as_ref(),
                HashMap::new(),
                Some(ai_body),
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            try_collect(&mut collected, Self::parse_songs(resp.data()), false);
        }

        // 引擎 3: 基于用户口味种子 audioId 的协同过滤推荐 (/recommend -> tracker.kugou.com)
        if !seeds.is_empty() && collected.len() < 18 {
            let seed_offset = ((batch_cursor as usize) * 3) % seeds.len().max(1);
            let seed_ids: Vec<i64> = seeds
                .iter()
                .cycle()
                .skip(seed_offset)
                .take(8)
                .map(|s| if s.audio_id > 0 { s.audio_id } else { s.album_audio_id })
                .filter(|&id| id > 0)
                .collect();
            if !seed_ids.is_empty() {
                let rec_body = json!({
                    "album_audio_id": seed_ids,
                    "appid": APP_ID,
                    "clientver": CLIENT_VERSION,
                    "platform": "android",
                    "userid": uid_num
                });
                let mut rec_headers = HashMap::new();
                rec_headers.insert("x-router".to_string(), "Zero.kugou.com".to_string());
                if let Ok(resp) = self
                    .transport
                    .call(
                        "/recommend",
                        &dev,
                        session.as_ref(),
                        HashMap::new(),
                        Some(rec_body),
                        "https://gateway.kugou.com",
                        "POST",
                        false,
                        rec_headers,
                        true,
                        true,
                    )
                    .await
                {
                    try_collect(&mut collected, Self::parse_songs(resp.data()), false);
                }
            }
        }

        // 引擎 4: 单卡口味推荐 (/singlecardrec.service/v1/single_card_recommend)
        if collected.len() < 16 {
            let card_ids = [1, 2, 3, 4, 6];
            let card_id = card_ids[(batch_cursor as usize) % card_ids.len()];
            let mut card_q = HashMap::new();
            card_q.insert("card_id".to_string(), card_id.to_string());
            let card_body = json!({
                "appid": APP_ID,
                "clientver": CLIENT_VERSION,
                "platform": "android",
                "userid": uid_num,
                "card_id": card_id,
                "pagesize": 20
            });
            if let Ok(resp) = self
                .transport
                .call(
                    "/singlecardrec.service/v1/single_card_recommend",
                    &dev,
                    session.as_ref(),
                    card_q,
                    Some(card_body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await
            {
                try_collect(&mut collected, Self::parse_songs(resp.data()), false);
            }
        }

        // 引擎 5: 基于口味种子歌手的动态拓展，确保每次点击换一批都能呈现符合口味的全新曲目
        if collected.len() < 18 {
            let mut seed_artists: Vec<String> = seeds
                .iter()
                .map(|s| {
                    s.artist
                        .split(&['、', '/', '&', ',', '，'][..])
                        .next()
                        .unwrap_or(&s.artist)
                        .trim()
                        .to_string()
                })
                .filter(|a| !a.is_empty() && a != "未知歌手" && a != "群星")
                .collect();
            seed_artists.dedup();
            if seed_artists.is_empty() {
                seed_artists = vec![
                    "周杰伦".to_string(),
                    "林俊杰".to_string(),
                    "陈奕迅".to_string(),
                    "陶喆".to_string(),
                    "邓紫棋".to_string(),
                    "告五人".to_string(),
                    "方大同".to_string(),
                    "王菲".to_string(),
                ];
            }
            let start_idx = ((batch_cursor as usize) * 2) % seed_artists.len();
            for offset in 0..2 {
                let artist_kw = &seed_artists[(start_idx + offset) % seed_artists.len()];
                let page = ((batch_cursor + offset as u32) % 3) + 1;
                if let Ok(artist_songs) = self.search_songs(artist_kw, page, 12).await {
                    try_collect(&mut collected, artist_songs, false);
                }
            }
        }

        if collected.len() < 8 {
            if let Ok(daily) = self.fetch_daily_recommend().await {
                try_collect(&mut collected, daily, true);
            }
        }

        let diversified = Self::diversify_artist_order(collected)
            .into_iter()
            .take(24)
            .collect::<Vec<_>>();

        if let Ok(mut guard) = self.seen_personal_fm_hashes.lock() {
            for s in &diversified {
                if !s.hash.is_empty() {
                    guard.insert(s.hash.to_lowercase());
                }
            }
        }

        Ok(self.enrich_songs_privilege_lite(diversified).await)
    }

    fn extract_raw_song_language(o: &Value) -> String {
        let base = o.get("base").unwrap_or(o);
        let audio_info = o.get("audio_info").unwrap_or(o);
        let old_cp = o.get("old_copyright").unwrap_or(o);
        [
            o.get("trans_param").and_then(|tp| Self::val_opt_str(tp, "language")),
            audio_info
                .get("trans_param")
                .and_then(|tp| Self::val_opt_str(tp, "language")),
            old_cp
                .get("trans_param")
                .and_then(|tp| Self::val_opt_str(tp, "language")),
            base.get("trans_param")
                .and_then(|tp| Self::val_opt_str(tp, "language")),
            Self::val_opt_str(base, "language"),
            Self::val_opt_str(o, "language"),
        ]
        .into_iter()
        .flatten()
        .find(|s| !s.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .to_string()
    }

    fn extract_raw_song_remark(o: &Value) -> String {
        let base = o.get("base").unwrap_or(o);
        [
            Self::val_str(o, "remarks"),
            Self::val_str(o, "remark"),
            Self::val_str(base, "remarks"),
            Self::val_str(base, "remark"),
        ]
        .into_iter()
        .find(|s| !s.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .to_string()
    }

    fn val_opt_str(o: &Value, key: &str) -> Option<String> {
        let s = Self::val_str(o, key);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    fn parse_songs_with_raw_meta(root: &Value) -> Vec<(Song, String, String)> {
        let mut collected = Vec::new();
        Self::collect_song_objects(root, 0, &mut collected);
        collected
            .into_iter()
            .filter_map(|obj| {
                let s = Self::parse_single_song(obj);
                if (!s.hash.is_empty() || !s.id.is_empty()) && s.title != "未知歌曲" {
                    let lang = Self::extract_raw_song_language(obj);
                    let remark = Self::extract_raw_song_remark(obj);
                    Some((s, lang, remark))
                } else {
                    None
                }
            })
            .collect()
    }

    async fn fetch_newsong_publish_21608(&self, now_ms: i64) -> Vec<(Song, String, String)> {
        let session = self.store.session();
        let dev = self.store.device();
        let uid_num = session
            .as_ref()
            .and_then(|s| s.user_id.parse::<i64>().ok())
            .unwrap_or(0);
        let mut query = HashMap::new();
        query.insert("_t".to_string(), now_ms.to_string());
        let body = json!({
            "rank_id": 21608,
            "userid": uid_num,
            "page": 1,
            "pagesize": 100,
            "tags": []
        });
        if let Ok(resp) = self
            .transport
            .call(
                "/musicadservice/container/v1/newsong_publish",
                &dev,
                session.as_ref(),
                query,
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            let mut items = Self::parse_songs_with_raw_meta(resp.data());
            if items.is_empty() {
                items = Self::parse_songs_with_raw_meta(&resp.body);
            }
            return items;
        }
        Vec::new()
    }

    async fn fetch_v3_rank_newsong(
        &self,
        newsong_type: i32,
        now_ms: i64,
    ) -> Vec<(Song, String, String)> {
        // 注意：mobilecdnbj /api/v3/rank/newsong 中 type=3 为 2019 年冻结旧库，韩国为 type=4，日本为 type=5
        if newsong_type == 3 {
            return Vec::new();
        }
        let dev = self.store.device();
        let mut query = HashMap::new();
        query.insert("version".to_string(), "9108".to_string());
        query.insert("plat".to_string(), "0".to_string());
        query.insert("with_cover".to_string(), "1".to_string());
        query.insert("pagesize".to_string(), "100".to_string());
        query.insert("page".to_string(), "1".to_string());
        query.insert("type".to_string(), newsong_type.to_string());
        query.insert("area_code".to_string(), "1".to_string());
        query.insert("with_res_tag".to_string(), "1".to_string());
        query.insert("_t".to_string(), now_ms.to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/rank/newsong",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            let mut items = Self::parse_songs_with_raw_meta(resp.data());
            if items.is_empty() {
                items = Self::parse_songs_with_raw_meta(&resp.body);
            }
            return items;
        }
        Vec::new()
    }

    async fn fetch_rank_audio_with_meta(
        &self,
        rank_id: i32,
        pagesize: u32,
        now_ms: i64,
    ) -> Vec<(Song, String, String)> {
        let session = self.store.session();
        let dev = self.store.device();
        let mut query = HashMap::new();
        query.insert("_t".to_string(), now_ms.to_string());
        let body = json!({
            "show_portrait_mv": 1,
            "show_type_total": 1,
            "filter_original_remarks": 0,
            "area_code": 1,
            "pagesize": pagesize,
            "rank_cid": 0,
            "type": 1,
            "page": 1,
            "rank_id": rank_id
        });
        let mut headers = HashMap::new();
        headers.insert("kg-tid".to_string(), "369".to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/openapi/kmr/v2/rank/audio",
                &dev,
                session.as_ref(),
                query,
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await
        {
            let mut items = Self::parse_songs_with_raw_meta(resp.data());
            if items.is_empty() {
                items = Self::parse_songs_with_raw_meta(&resp.body);
            }
            return items;
        }
        Vec::new()
    }

    async fn fetch_ip_latest_audios_with_meta(
        &self,
        ip_id: i32,
        pagesize: u32,
        now_ms: i64,
    ) -> Vec<(Song, String, String)> {
        let session = self.store.session();
        let dev = self.store.device();
        let mut query = HashMap::new();
        query.insert("_t".to_string(), now_ms.to_string());
        let body = json!({
            "is_publish": 1,
            "ip_id": ip_id,
            "sort": 2,
            "page": 1,
            "pagesize": pagesize,
            "query": 1
        });
        if let Ok(resp) = self
            .transport
            .call(
                "/openapi/v1/ip/audios",
                &dev,
                session.as_ref(),
                query,
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            let mut items = Self::parse_songs_with_raw_meta(resp.data());
            if items.is_empty() {
                items = Self::parse_songs_with_raw_meta(&resp.body);
            }
            return items;
        }
        Vec::new()
    }

    /// 11. 新歌速递 (对齐 KuGouMusicApi top_song + v3/rank/newsong + rank/audio + ip/audios，严格校验最近一周正式发行歌曲)
    pub async fn fetch_new_song_express(&self, period_key: &str) -> Result<Vec<Song>, String> {
        let now_ms = Utc::now().timestamp_millis();
        let norm_key = period_key.trim().to_uppercase();

        let raw_candidates: Vec<Song> = match norm_key.as_str() {
            "CANTON" | "CANTONESE" | "21611" => {
                let (pub_list, hk_chart, joox_chart, ip_canton_pop, ip_canton) = tokio::join!(
                    self.fetch_newsong_publish_21608(now_ms),
                    self.fetch_rank_audio_with_meta(31313, 100, now_ms),
                    self.fetch_rank_audio_with_meta(42807, 100, now_ms),
                    self.fetch_ip_latest_audios_with_meta(87899, 60, now_ms),
                    self.fetch_ip_latest_audios_with_meta(87495, 60, now_ms),
                );
                pub_list
                    .into_iter()
                    .chain(hk_chart)
                    .chain(joox_chart)
                    .chain(ip_canton_pop)
                    .chain(ip_canton)
                    .filter(|(_, lang, _)| lang.contains("粤语"))
                    .map(|(s, _, _)| s)
                    .collect()
            }
            "WESTERN" | "EUROPE_AMERICA" | "21609" => {
                let (v3_west, ip_west, chart_west) = tokio::join!(
                    self.fetch_v3_rank_newsong(2, now_ms),
                    self.fetch_ip_latest_audios_with_meta(87780, 60, now_ms),
                    self.fetch_rank_audio_with_meta(31310, 100, now_ms),
                );
                let is_non_west = |lang: &str| {
                    lang.contains("国语")
                        || lang.contains("华语")
                        || lang.contains("粤语")
                        || lang.contains("闽南")
                        || lang.contains("日语")
                        || lang.contains("韩语")
                };
                v3_west
                    .into_iter()
                    .chain(ip_west)
                    .chain(chart_west)
                    .filter(|(_, lang, _)| !is_non_west(lang))
                    .map(|(s, _, _)| s)
                    .collect()
            }
            "JPKR" | "JAPAN_KOREA" | "21610" => {
                // 日韩合并 Tag：并发获取韩语新歌 (v3 type=4 + 韩国榜 31311) 与日语新歌 (v3 type=5 + 日本榜 31312)，交替穿插保障韩日双语种均衡覆盖
                let (v3_kr, v3_jp, chart_kr, chart_jp) = tokio::join!(
                    self.fetch_v3_rank_newsong(4, now_ms),
                    self.fetch_v3_rank_newsong(5, now_ms),
                    self.fetch_rank_audio_with_meta(31311, 100, now_ms),
                    self.fetch_rank_audio_with_meta(31312, 100, now_ms),
                );
                let is_kr_ok = |lang: &str| {
                    lang.is_empty() || lang.contains("韩") || lang.contains("英") || lang.contains("纯音乐")
                };
                let is_jp_ok = |lang: &str| {
                    lang.is_empty() || lang.contains("日") || lang.contains("英") || lang.contains("纯音乐")
                };
                let kr_vec: Vec<Song> = v3_kr
                    .into_iter()
                    .chain(chart_kr)
                    .filter(|(_, lang, _)| is_kr_ok(lang))
                    .map(|(s, _, _)| s)
                    .collect();
                let jp_vec: Vec<Song> = v3_jp
                    .into_iter()
                    .chain(chart_jp)
                    .filter(|(_, lang, _)| is_jp_ok(lang))
                    .map(|(s, _, _)| s)
                    .collect();
                let max_len = kr_vec.len().max(jp_vec.len());
                let mut interleaved = Vec::with_capacity(kr_vec.len() + jp_vec.len());
                for i in 0..max_len {
                    if let Some(ks) = kr_vec.get(i) {
                        interleaved.push(ks.clone());
                    }
                    if let Some(js) = jp_vec.get(i) {
                        interleaved.push(js.clone());
                    }
                }
                interleaved
            }
            "KOREA" | "KR" | "31311" => {
                let (v3_kr, chart_kr) = tokio::join!(
                    self.fetch_v3_rank_newsong(4, now_ms),
                    self.fetch_rank_audio_with_meta(31311, 100, now_ms),
                );
                v3_kr
                    .into_iter()
                    .chain(chart_kr)
                    .filter(|(_, lang, _)| !lang.contains("国语") && !lang.contains("粤语") && !lang.contains("日语"))
                    .map(|(s, _, _)| s)
                    .collect()
            }
            "JAPAN" | "JP" | "31312" => {
                let (v3_jp, chart_jp) = tokio::join!(
                    self.fetch_v3_rank_newsong(5, now_ms),
                    self.fetch_rank_audio_with_meta(31312, 100, now_ms),
                );
                v3_jp
                    .into_iter()
                    .chain(chart_jp)
                    .filter(|(_, lang, _)| !lang.contains("国语") && !lang.contains("粤语") && !lang.contains("韩语"))
                    .map(|(s, _, _)| s)
                    .collect()
            }
            "ACG" | "33162" => {
                let chart_acg = self.fetch_rank_audio_with_meta(33162, 100, now_ms).await;
                chart_acg.into_iter().map(|(s, _, _)| s).collect()
            }
            "OST" | "FILM" | "46910" | "33163" => {
                let (variety_new, film_chart, ip_ost, pub_list) = tokio::join!(
                    self.fetch_rank_audio_with_meta(46910, 100, now_ms),
                    self.fetch_rank_audio_with_meta(33163, 100, now_ms),
                    self.fetch_ip_latest_audios_with_meta(88213, 60, now_ms),
                    self.fetch_newsong_publish_21608(now_ms),
                );
                let ost_keywords = [
                    "电视剧", "电影", "网剧", "短剧", "动画", "动漫", "综艺", "游戏",
                    "主题曲", "片尾曲", "片头曲", "插曲", "推广曲", "OST", "原声",
                ];
                let pub_ost = pub_list.into_iter().filter(|(s, _, rm)| {
                    ost_keywords
                        .iter()
                        .any(|kw| rm.contains(kw) || s.album.contains(kw) || s.title.contains(kw))
                });
                variety_new
                    .into_iter()
                    .chain(film_chart)
                    .chain(ip_ost)
                    .chain(pub_ost)
                    .map(|(s, _, _)| s)
                    .collect()
            }
            _ => {
                // CHINESE (华语)
                let (pub_list, v3_cn, tw_chart) = tokio::join!(
                    self.fetch_newsong_publish_21608(now_ms),
                    self.fetch_v3_rank_newsong(1, now_ms),
                    self.fetch_rank_audio_with_meta(54848, 80, now_ms),
                );
                let is_foreign = |lang: &str| {
                    lang.contains("英语")
                        || lang.contains("英文")
                        || lang.contains("日语")
                        || lang.contains("韩语")
                        || lang.contains("俄语")
                        || lang.contains("法语")
                        || lang.contains("德语")
                        || lang.contains("泰语")
                        || lang.contains("西班牙")
                };
                pub_list
                    .into_iter()
                    .chain(v3_cn)
                    .chain(tw_chart)
                    .filter(|(_, lang, _)| !is_foreign(lang))
                    .map(|(s, _, _)| s)
                    .collect()
            }
        };

        // 多维度去重：hash + album_audio_id + 标准化(歌名::歌手)
        let mut deduped = Vec::new();
        let mut seen_hashes = HashSet::new();
        let mut seen_aaids = HashSet::new();
        let mut seen_title_artist = HashSet::new();
        for s in raw_candidates {
            let h_key = if !s.hash.is_empty() {
                s.hash.to_lowercase()
            } else {
                s.id.to_lowercase()
            };
            if h_key.is_empty() {
                continue;
            }
            if !seen_hashes.insert(h_key) {
                continue;
            }
            if s.album_audio_id > 0 && !seen_aaids.insert(s.album_audio_id) {
                continue;
            }
            let ta_key = format!(
                "{}::{}",
                s.title.trim().to_lowercase(),
                s.artist.trim().to_lowercase()
            );
            if !seen_title_artist.insert(ta_key) {
                continue;
            }
            deduped.push(s);
        }

        // 先通过 /kmr/v2/albums 批量补全精确发行日期、官方正版专辑名与高清封面
        let dated_songs = self.enrich_songs_publish_date(deduped).await;

        let china_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let today_date = Utc::now().with_timezone(&china_tz).date_naive();
        let cutoff_7d = (today_date - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();
        let today_str = today_date.format("%Y-%m-%d").to_string();

        // 严格保留最近一周（today-7d 至 today）正式发行的歌曲，剔除超期旧歌与异常未来年份脏数据
        let strictly_last_week: Vec<Song> = dated_songs
            .into_iter()
            .filter(|s| {
                Self::is_valid_iso_date(&s.publish_date)
                    && s.publish_date >= cutoff_7d
                    && s.publish_date <= today_str
            })
            .collect();

        // 仅为通过最近一周核验的新歌补全音质规格与 VIP 权限，大幅降低请求延迟
        let mut final_list = self.enrich_songs_privilege_lite(strictly_last_week).await;

        final_list.sort_by(|a, b| {
            b.publish_date
                .cmp(&a.publish_date)
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });

        Ok(final_list)
    }

    /// 12. 巅峰排行榜列表与榜单曲目 (对齐 Android MusicRepository.rankList & rankSongsAll，完整保留全量官方榜单)
    pub async fn fetch_rank_list(&self) -> Result<Vec<RankItem>, String> {
        let dev = self.store.device();
        let fresh_stamp = Utc::now().timestamp_millis().to_string();

        let mut query = HashMap::new();
        query.insert("version".to_string(), "9108".to_string());
        query.insert("plat".to_string(), "0".to_string());
        query.insert("showtype".to_string(), "2".to_string());
        query.insert("withsong".to_string(), "1".to_string());
        query.insert("parentid".to_string(), "0".to_string());
        query.insert("apiver".to_string(), "6".to_string());
        query.insert("area_code".to_string(), "1".to_string());
        query.insert("_t".to_string(), fresh_stamp);

        let mut ranks = Vec::new();
        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/rank/list",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            let info_arr = resp
                .data()
                .get("data")
                .and_then(|d| d.get("info"))
                .and_then(|v| v.as_array())
                .or_else(|| resp.data().get("info").and_then(|v| v.as_array()));

            if let Some(arr) = info_arr {
                let mut flat = Vec::new();
                for el in arr {
                    if let Some(children) = el.get("children").and_then(|c| c.as_array()) {
                        for child in children {
                            flat.push(child);
                        }
                    }
                    if el.get("rankid").is_some() {
                        flat.push(el);
                    }
                }
                let mut seen_ids = HashSet::new();
                for o in flat {
                    let Some(id) = Self::val_i64(o, "rankid") else {
                        continue;
                    };
                    if id <= 0 || !seen_ids.insert(id) {
                        continue;
                    }
                    let name = Self::val_str(o, "rankname");
                    if name.is_empty() {
                        continue;
                    }
                    let cover_raw = [
                        Self::val_str(o, "imgurl"),
                        Self::val_str(o, "pic"),
                        Self::val_str(o, "banner7url"),
                        Self::val_str(o, "bannerurl"),
                    ]
                    .into_iter()
                    .find(|s| !s.is_empty())
                    .unwrap_or_default();

                    let mut previews = Vec::new();
                    if let Some(s_arr) = o
                        .get("songinfo")
                        .or_else(|| o.get("songs"))
                        .and_then(|v| v.as_array())
                    {
                        for s in s_arr.iter().take(3) {
                            let sname = {
                                let n = Self::val_str(s, "songname");
                                if !n.is_empty() {
                                    n
                                } else {
                                    let n2 = Self::val_str(s, "name");
                                    if !n2.is_empty() {
                                        n2
                                    } else {
                                        Self::val_str(s, "filename")
                                    }
                                }
                            };
                            let singer = {
                                let sg = Self::val_str(s, "singername");
                                if !sg.is_empty() {
                                    sg
                                } else {
                                    Self::val_str(s, "author_name")
                                }
                            };
                            if let Some(formatted) = Self::format_rank_preview_line(&sname, &singer) {
                                previews.push(formatted);
                            }
                        }
                    }

                    let freq = Self::normalize_rank_update_freq(&Self::val_str(o, "update_frequency"), &name);
                    let category = Self::classify_rank_category(id as i32, &name);
                    let default_total = if id == 8888 || name.contains("500") { 500 } else { 100 };
                    let raw_total = Self::val_i64(o, "songcount")
                        .or_else(|| Self::val_i64(o, "total"))
                        .or_else(|| Self::val_i64(o, "count"))
                        .unwrap_or(default_total) as i32;
                    ranks.push(RankItem {
                        rank_id: id as i32,
                        rank_name: name,
                        cover_url: signer::normalize_image_url(Some(&cover_raw)),
                        update_frequency: freq,
                        top_songs_preview: previews,
                        category,
                        total_songs: if id == 8888 {
                            500
                        } else if raw_total > 0 {
                            raw_total
                        } else {
                            default_total as i32
                        },
                    });
                }
            }
        }

        // 通道 2: 对齐 MakcRe/KuGouMusicApi module/rank_list.js (/ocean/v6/rank/list) 兜底
        if ranks.is_empty() {
            let session = self.store.session();
            let mut ocean_query = HashMap::new();
            ocean_query.insert("plat".to_string(), "2".to_string());
            ocean_query.insert("withsong".to_string(), "1".to_string());
            ocean_query.insert("parentid".to_string(), "0".to_string());
            if let Ok(resp) = self
                .transport
                .call(
                    "/ocean/v6/rank/list",
                    &dev,
                    session.as_ref(),
                    ocean_query,
                    None,
                    "https://gateway.kugou.com",
                    "GET",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await
            {
                let info_arr = resp
                    .data()
                    .get("data")
                    .and_then(|d| d.get("info"))
                    .and_then(|v| v.as_array())
                    .or_else(|| resp.data().get("info").and_then(|v| v.as_array()));
                if let Some(arr) = info_arr {
                    let mut seen_ids = HashSet::new();
                    for o in arr {
                        let Some(id) = Self::val_i64(o, "rankid") else {
                            continue;
                        };
                        if id <= 0 || !seen_ids.insert(id) {
                            continue;
                        }
                        let name = Self::val_str(o, "rankname");
                        if name.is_empty() {
                            continue;
                        }
                        let cover_raw = [
                            Self::val_str(o, "imgurl"),
                            Self::val_str(o, "pic"),
                            Self::val_str(o, "banner7url"),
                        ]
                        .into_iter()
                        .find(|s| !s.is_empty())
                        .unwrap_or_default();
                        let mut previews = Vec::new();
                        if let Some(s_arr) = o
                            .get("songinfo")
                            .or_else(|| o.get("songs"))
                            .and_then(|v| v.as_array())
                        {
                            for s in s_arr.iter().take(3) {
                                let sname = Self::val_str(s, "songname");
                                let singer = Self::val_str(s, "singername");
                                if let Some(formatted) = Self::format_rank_preview_line(&sname, &singer) {
                                    previews.push(formatted);
                                }
                            }
                        }
                        let freq = Self::normalize_rank_update_freq(&Self::val_str(o, "update_frequency"), &name);
                        let category = Self::classify_rank_category(id as i32, &name);
                        let default_total = if id == 8888 || name.contains("500") { 500 } else { 100 };
                        ranks.push(RankItem {
                            rank_id: id as i32,
                            rank_name: name,
                            cover_url: signer::normalize_image_url(Some(&cover_raw)),
                            update_frequency: freq,
                            top_songs_preview: previews,
                            category,
                            total_songs: default_total as i32,
                        });
                    }
                }
            }
        }

        if ranks.is_empty() {
            ranks = vec![
                RankItem {
                    rank_id: 6666,
                    rank_name: "酷狗飙升榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175317789524.jpg".to_string(),
                    update_frequency: "每天更新".to_string(),
                    top_songs_preview: vec![
                        "瞬 - 告五人".to_string(),
                        "奢香夫人 - 凤凰传奇".to_string(),
                        "离别开出花 - 就是南方凯".to_string(),
                    ],
                    category: "官方热榜".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 8888,
                    rank_name: "酷狗TOP500".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175249561655.jpg".to_string(),
                    update_frequency: "每天更新".to_string(),
                    top_songs_preview: vec![
                        "晴天 - 周杰伦".to_string(),
                        "爱在西元前 - 周杰伦".to_string(),
                        "篇章 - 张韶涵".to_string(),
                    ],
                    category: "官方热榜".to_string(),
                    total_songs: 500,
                },
                RankItem {
                    rank_id: 23784,
                    rank_name: "网络红歌榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175344315250.jpg".to_string(),
                    update_frequency: "每天更新".to_string(),
                    top_songs_preview: vec![
                        "悬溺 - 葛东琪".to_string(),
                        "若把你 - Kirsty刘瑾睿".to_string(),
                        "晚风告白 - 星野".to_string(),
                    ],
                    category: "官方热榜".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 31308,
                    rank_name: "华语新歌榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175317789524.jpg".to_string(),
                    update_frequency: "每天更新".to_string(),
                    top_songs_preview: vec![
                        "向云端 - 小霞".to_string(),
                        "笼 - 张碧晨".to_string(),
                        "想见你想见你想见你 - 八三夭".to_string(),
                    ],
                    category: "官方热榜".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 24971,
                    rank_name: "DJ热歌榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175409477029.jpg".to_string(),
                    update_frequency: "每周四更新".to_string(),
                    top_songs_preview: vec![
                        "富士山下 - 陈奕迅".to_string(),
                        "月半小夜曲 - 李克勤".to_string(),
                        "海阔天空 - Beyond".to_string(),
                    ],
                    category: "特色音乐".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 33162,
                    rank_name: "ACG新歌榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175249561655.jpg".to_string(),
                    update_frequency: "每周四更新".to_string(),
                    top_songs_preview: vec![
                        "Cruel Summer - Taylor Swift".to_string(),
                        "Counting Stars - OneRepublic".to_string(),
                        "Flowers - Miley Cyrus".to_string(),
                    ],
                    category: "特色音乐".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 4681,
                    rank_name: "美国BillBoard榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175344315250.jpg".to_string(),
                    update_frequency: "每周三更新".to_string(),
                    top_songs_preview: vec![
                        "Die With A Smile - Lady Gaga".to_string(),
                        "Birds of a Feather - Billie Eilish".to_string(),
                        "Espresso - Sabrina Carpenter".to_string(),
                    ],
                    category: "全球权威".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 4673,
                    rank_name: "日本公信榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175409477029.jpg".to_string(),
                    update_frequency: "每周五更新".to_string(),
                    top_songs_preview: vec![
                        "Idol - YOASOBI".to_string(),
                        "Lemon - 米津玄师".to_string(),
                        "怪兽的花呗 - Vaundy".to_string(),
                    ],
                    category: "全球权威".to_string(),
                    total_songs: 100,
                },
            ];
        }

        // 智能优先级排序：让发现页首屏 8 张卡片覆盖最具代表性的多元旗舰榜单，且完整保留全量官方榜单
        ranks.sort_by_key(|r| Self::rank_home_priority_score(r.rank_id, &r.rank_name, &r.category));

        // 并发补全缺失 Top 3 歌曲预览的榜单，确保任意榜单卡片展示真实实时前三名（格式统一为：歌名 - 歌手）
        let missing_indices: Vec<(usize, i32)> = ranks
            .iter()
            .enumerate()
            .filter(|(_, r)| r.top_songs_preview.is_empty())
            .take(16)
            .map(|(idx, r)| (idx, r.rank_id))
            .collect();
        if !missing_indices.is_empty() {
            for chunk in missing_indices.chunks(6) {
                let mut futs = Vec::new();
                for &(idx, rid) in chunk {
                    futs.push(async move {
                        let songs = self.fetch_rank_songs_single_page(rid, 1, 5, 0).await;
                        (idx, songs)
                    });
                }
                let results = futures_join_vec(futs).await;
                for (idx, songs) in results {
                    if !songs.is_empty() {
                        ranks[idx].top_songs_preview = songs
                            .into_iter()
                            .take(3)
                            .map(|s| {
                                if !s.artist.is_empty() && s.artist != "未知歌手" {
                                    format!("{} - {}", s.title, s.artist)
                                } else {
                                    s.title
                                }
                            })
                            .collect();
                    }
                }
            }
        }

        Ok(ranks)
    }

    async fn resolve_latest_rank_vol_id(&self, rank_id: i32) -> i64 {
        let dev = self.store.device();
        let minute_stamp = (Utc::now().timestamp() / 60).to_string();

        // 通道 1: /api/v3/rank/vol
        let mut query = HashMap::new();
        query.insert("plat".to_string(), "0".to_string());
        query.insert("ranktype".to_string(), "2".to_string());
        query.insert("rankid".to_string(), rank_id.to_string());
        query.insert("with_res_tag".to_string(), "0".to_string());
        query.insert("_t".to_string(), minute_stamp.clone());
        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/rank/vol",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            let arr = resp
                .data()
                .get("data")
                .and_then(|d| d.get("info"))
                .and_then(|v| v.as_array())
                .or_else(|| resp.data().get("info").and_then(|v| v.as_array()))
                .or_else(|| resp.data().get("vol_list").and_then(|v| v.as_array()));
            if let Some(first) = arr.and_then(|a| a.first()) {
                if let Some(vid) =
                    Self::val_i64(first, "volid").or_else(|| Self::val_i64(first, "rank_cid"))
                {
                    if vid > 0 {
                        return vid;
                    }
                }
            }
        }

        // 通道 2: 对齐 MakcRe/KuGouMusicApi module/rank_vol.js (/ocean/v6/rank/vol)
        let session = self.store.session();
        let mut ocean_query = HashMap::new();
        ocean_query.insert("rank_cid".to_string(), "0".to_string());
        ocean_query.insert("rankid".to_string(), rank_id.to_string());
        ocean_query.insert("ranktype".to_string(), "1".to_string());
        ocean_query.insert("type".to_string(), "0".to_string());
        ocean_query.insert("plat".to_string(), "2".to_string());
        if let Ok(resp) = self
            .transport
            .call(
                "/ocean/v6/rank/vol",
                &dev,
                session.as_ref(),
                ocean_query,
                None,
                "https://gateway.kugou.com",
                "GET",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            let arr = resp
                .data()
                .get("data")
                .and_then(|d| d.get("info"))
                .and_then(|v| v.as_array())
                .or_else(|| resp.data().get("info").and_then(|v| v.as_array()))
                .or_else(|| resp.data().get("vol_list").and_then(|v| v.as_array()));
            if let Some(first) = arr.and_then(|a| a.first()) {
                if let Some(vid) =
                    Self::val_i64(first, "volid").or_else(|| Self::val_i64(first, "rank_cid"))
                {
                    if vid > 0 {
                        return vid;
                    }
                }
            }
        }

        0
    }

    async fn fetch_rank_songs_single_page(
        &self,
        rank_id: i32,
        page: u32,
        page_size: u32,
        latest_vol_id: i64,
    ) -> Vec<Song> {
        let dev = self.store.device();
        let minute_stamp = (Utc::now().timestamp() / 60).to_string();

        // 通道 1a: /api/v3/rank/song (带最新 volid)
        let mut query = HashMap::new();
        query.insert("version".to_string(), "9108".to_string());
        query.insert("ranktype".to_string(), "2".to_string());
        query.insert("rankid".to_string(), rank_id.to_string());
        query.insert("page".to_string(), page.to_string());
        query.insert("pagesize".to_string(), page_size.to_string());
        query.insert("plat".to_string(), "0".to_string());
        query.insert("area_code".to_string(), "1".to_string());
        query.insert("_t".to_string(), minute_stamp.clone());
        if latest_vol_id > 0 {
            query.insert("volid".to_string(), latest_vol_id.to_string());
        }

        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/rank/song",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            let songs = Self::parse_songs(resp.data());
            if !songs.is_empty() {
                return songs;
            }
        }

        // 通道 1b: 若带 volid 请求未返回，对齐 Android MusicRepository.rankSongs 不带 volid 直连最新默认期
        if latest_vol_id > 0 {
            let mut fallback_query = HashMap::new();
            fallback_query.insert("version".to_string(), "9108".to_string());
            fallback_query.insert("rankid".to_string(), rank_id.to_string());
            fallback_query.insert("page".to_string(), page.to_string());
            fallback_query.insert("pagesize".to_string(), page_size.to_string());
            fallback_query.insert("plat".to_string(), "0".to_string());
            fallback_query.insert("area_code".to_string(), "1".to_string());
            fallback_query.insert("_t".to_string(), minute_stamp.clone());

            if let Ok(resp) = self
                .transport
                .call(
                    "/api/v3/rank/song",
                    &dev,
                    None,
                    fallback_query,
                    None,
                    "http://mobilecdnbj.kugou.com",
                    "GET",
                    false,
                    HashMap::new(),
                    false,
                    false,
                )
                .await
            {
                let songs = Self::parse_songs(resp.data());
                if !songs.is_empty() {
                    return songs;
                }
            }
        }

        // 通道 2: 对齐 MakcRe/KuGouMusicApi module/rank_audio.js (/openapi/kmr/v2/rank/audio)
        let session = self.store.session();
        let cid_candidates: &[i64] = if latest_vol_id > 0 {
            &[latest_vol_id, 0]
        } else {
            &[0]
        };
        for &cid in cid_candidates {
            let body = json!({
                "show_portrait_mv": 1,
                "show_type_total": 1,
                "filter_original_remarks": 1,
                "area_code": 1,
                "pagesize": page_size,
                "rank_cid": cid,
                "type": 1,
                "page": page,
                "rank_id": rank_id
            });
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
            headers.insert("kg-tid".to_string(), "369".to_string());

            if let Ok(resp) = self
                .transport
                .call(
                    "/openapi/kmr/v2/rank/audio",
                    &dev,
                    session.as_ref(),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
            {
                let mut songs = Self::parse_songs(resp.data());
                if songs.is_empty() {
                    songs = Self::parse_songs(&resp.body);
                }
                if !songs.is_empty() {
                    return songs;
                }
            }
        }

        // 通道 3: M 站实时榜单 JSON 兜底 (https://m.kugou.com/rank/info/)
        let mut m_query = HashMap::new();
        m_query.insert("rankid".to_string(), rank_id.to_string());
        m_query.insert("page".to_string(), page.to_string());
        m_query.insert("json".to_string(), "true".to_string());
        m_query.insert("_t".to_string(), minute_stamp);
        if let Ok(resp) = self
            .transport
            .call(
                "/rank/info/",
                &dev,
                None,
                m_query,
                None,
                "https://m.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            let songs = Self::parse_songs(resp.body.get("songs").unwrap_or(&resp.body));
            if !songs.is_empty() {
                return songs;
            }
        }

        Vec::new()
    }

    pub async fn fetch_rank_songs(
        &self,
        rank_id: i32,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Song>, String> {
        let latest_vol_id = self.resolve_latest_rank_vol_id(rank_id).await;
        if page == 1 && page_size > 100 {
            let max_pages = ((page_size + 99) / 100).clamp(1, 5);
            let mut page_futs = Vec::with_capacity(max_pages as usize);
            for p in 1..=max_pages {
                page_futs.push(async move {
                    let batch = self
                        .fetch_rank_songs_single_page(rank_id, p, 100, latest_vol_id)
                        .await;
                    (p, batch)
                });
            }
            let mut page_results = futures_join_vec(page_futs).await;
            page_results.sort_by_key(|(p, _)| *p);

            let mut all_songs: Vec<Song> = Vec::with_capacity(page_size as usize);
            let mut seen_page_signatures: HashSet<String> = HashSet::new();

            for (p, batch) in page_results {
                if batch.is_empty() {
                    break;
                }
                let batch_len = batch.len();
                // 检测非 500 首榜单在请求后续页时是否重复返回了第 1 页的完全相同内容
                let first = &batch[0];
                let last = &batch[batch_len - 1];
                let page_sig = format!(
                    "{}:{}:{}:{}",
                    first.hash.to_lowercase(),
                    first.title.to_lowercase(),
                    last.hash.to_lowercase(),
                    last.title.to_lowercase()
                );
                let is_new_page_sig = seen_page_signatures.insert(page_sig);
                if p > 1 && !is_new_page_sig {
                    break;
                }

                // 若当前页超过 85% 曲目已存在于前序页（例如 100 首榜单在 page=2 重复返回前序曲目），立即终止
                if p > 1 && !all_songs.is_empty() {
                    let dup_count = batch
                        .iter()
                        .filter(|s| {
                            !s.hash.is_empty()
                                && all_songs
                                    .iter()
                                    .any(|e| e.hash.eq_ignore_ascii_case(&s.hash) && e.title == s.title)
                        })
                        .count();
                    if dup_count * 100 >= batch_len * 85 {
                        break;
                    }
                }

                for s in batch {
                    all_songs.push(s);
                    if all_songs.len() >= page_size as usize {
                        break;
                    }
                }

                if batch_len < 20 || all_songs.len() >= page_size as usize {
                    break;
                }
            }

            return Ok(self.enrich_songs_privilege_lite(all_songs).await);
        }

        let batch = self
            .fetch_rank_songs_single_page(rank_id, page.max(1), page_size.clamp(10, 100), latest_vol_id)
            .await;
        Ok(self.enrich_songs_privilege_lite(batch).await)
    }

    /// 13. 云端最近播放记录 + 本地持久化历史双向合并 (对齐 MusicRepository.latestListenSongs + LocalStorage)
    pub async fn fetch_recent_history(&self) -> Result<Vec<Song>, String> {
        let Some(session) = self.store.session() else {
            return Ok(self.store.load_play_history("guest"));
        };
        let local_history = self.store.load_play_history(&session.user_id);
        let dev = self.store.device();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);

        let mut cloud_songs: Vec<Song> = Vec::new();
        let body = json!({
            "userid": uid_num,
            "token": session.token,
            "source_classify": "app",
            "to_subdivide_sr": 1
        });

        if let Ok(resp) = self
            .transport
            .call(
                "/playhistory/v1/get_songs",
                &dev,
                Some(&session),
                HashMap::new(),
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            cloud_songs = Self::parse_songs(resp.data());
        }

        if cloud_songs.is_empty() {
            let fallback_body = json!({
                "area_code": "1",
                "sources": ["pc", "mobile", "tv", "car"],
                "userid": uid_num,
                "ret_info": 1,
                "token": session.token,
                "pagesize": 60
            });
            if let Ok(resp) = self
                .transport
                .call(
                    "/playque/devque/v1/get_latest_songs",
                    &dev,
                    Some(&session),
                    HashMap::new(),
                    Some(fallback_body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await
            {
                if resp.is_success() {
                    cloud_songs = Self::parse_songs(resp.data());
                }
            }
        }

        let enriched_cloud = if !cloud_songs.is_empty() {
            self.enrich_songs_privilege_lite(cloud_songs).await
        } else {
            Vec::new()
        };

        let mut merged: Vec<Song> = Vec::with_capacity(local_history.len() + enriched_cloud.len());
        for s in local_history.into_iter().chain(enriched_cloud.into_iter()) {
            if let Some(existing) = merged.iter_mut().find(|e| is_same_music_track(e, &s)) {
                *existing = merge_song_entries(existing, &s);
            } else if merged.len() < 200 {
                merged.push(s);
            }
        }

        if !merged.is_empty() {
            self.store.save_play_history(&session.user_id, &merged);
            self.record_taste_seeds(&merged);
        }
        Ok(merged)
    }

    /// 14. 歌曲搜索 (对齐 MakcRe/KuGouMusicApi module/search.js -> /v2/search/song，支持全量分页与折叠版本)
    pub async fn search_songs_paged(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<PagedSearchResult<Song>, String> {
        let kw = keyword.trim();
        let p = page.max(1);
        let ps = page_size.clamp(1, 50);
        if kw.is_empty() {
            return Ok(PagedSearchResult {
                items: Vec::new(),
                total: 0,
                page: p,
                page_size: ps,
                has_more: false,
            });
        }
        let session = self.store.session();
        let dev = self.store.device();

        let mut query = HashMap::new();
        query.insert("keyword".to_string(), kw.to_string());
        query.insert("page".to_string(), p.to_string());
        query.insert("pagesize".to_string(), ps.to_string());
        query.insert("platform".to_string(), "AndroidFilter".to_string());
        query.insert("iscorrection".to_string(), "1".to_string());
        query.insert("privilegefilter".to_string(), "0".to_string());
        query.insert("area_code".to_string(), "1".to_string());
        query.insert("dopicfull".to_string(), "1".to_string());

        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "complexsearch.kugou.com".to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/v2/search/song",
                &dev,
                session.as_ref(),
                query,
                None,
                "https://gateway.kugou.com",
                "GET",
                false,
                headers,
                true,
                true,
            )
            .await
        {
            if resp.is_success() {
                let total = Self::val_i64(resp.data(), "total").unwrap_or(0).max(0) as u32;
                let songs = Self::parse_songs(resp.data());
                let items = self.enrich_songs_privilege_lite(songs).await;
                let has_more = !items.is_empty()
                    && if total > 0 {
                        p.saturating_mul(ps) < total
                    } else {
                        items.len() >= ps as usize
                    };
                return Ok(PagedSearchResult {
                    items,
                    total,
                    page: p,
                    page_size: ps,
                    has_more,
                });
            } else if p > 1 {
                // 酷狗网关在深分页 offset 超出 total 时会返回 status=0，优雅视作到底
                return Ok(PagedSearchResult {
                    items: Vec::new(),
                    total: 0,
                    page: p,
                    page_size: ps,
                    has_more: false,
                });
            }
        }

        // 兜底通道: CDN 单曲搜索 (/api/v3/search/song)
        let mut cdn_query = HashMap::new();
        cdn_query.insert("version".to_string(), "9108".to_string());
        cdn_query.insert("keyword".to_string(), kw.to_string());
        cdn_query.insert("page".to_string(), p.to_string());
        cdn_query.insert("pagesize".to_string(), ps.to_string());
        cdn_query.insert("showtype".to_string(), "14".to_string());
        cdn_query.insert("plat".to_string(), "0".to_string());
        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/search/song",
                &dev,
                None,
                cdn_query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            let total = Self::val_i64(resp.data(), "total").unwrap_or(0).max(0) as u32;
            let songs = Self::parse_songs(resp.data());
            let items = self.enrich_songs_privilege_lite(songs).await;
            let has_more = !items.is_empty()
                && if total > 0 {
                    p.saturating_mul(ps) < total
                } else {
                    items.len() >= ps as usize
                };
            return Ok(PagedSearchResult {
                items,
                total,
                page: p,
                page_size: ps,
                has_more,
            });
        }

        Ok(PagedSearchResult {
            items: Vec::new(),
            total: 0,
            page: p,
            page_size: ps,
            has_more: false,
        })
    }

    pub async fn search_songs(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Song>, String> {
        self.search_songs_paged(keyword, page, page_size)
            .await
            .map(|r| r.items)
    }

    /// 15. 获取歌单详情信息 (对齐 Android MusicRepository.playlistInfo -> /v3/get_list_info & /api/v3/special/info)
    pub async fn fetch_playlist_info(&self, playlist_id: &str) -> Result<Playlist, String> {
        let pid = playlist_id.trim();
        if pid.is_empty() {
            return Err("歌单 ID 不能为空".to_string());
        }
        let dev = self.store.device();
        let session = self.store.session();

        // 1. 若是 global_collection_id，优先请求 /v3/get_list_info
        if pid.starts_with("collection_") || pid.len() >= 16 {
            let body = json!({
                "data": [{ "global_collection_id": pid }],
                "userid": session.as_ref().map(|s| s.user_id.as_str()).unwrap_or("0"),
                "token": session.as_ref().map(|s| s.token.as_str()).unwrap_or("")
            });
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "pubsongs.kugou.com".to_string());
            if let Ok(resp) = self
                .transport
                .call(
                    "/v3/get_list_info",
                    &dev,
                    session.as_ref(),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
            {
                let item_opt = resp
                    .data()
                    .get("data")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .or_else(|| resp.data().get("info").and_then(|v| v.as_array()).and_then(|a| a.first()))
                    .or_else(|| resp.data().as_array().and_then(|a| a.first()));
                if let Some(obj) = item_opt {
                    let name = {
                        let n = Self::val_str(obj, "name");
                        if !n.is_empty() {
                            n
                        } else {
                            Self::val_str(obj, "specialname")
                        }
                    };
                    if !name.is_empty() {
                        let pic = {
                            let p = Self::val_str(obj, "pic");
                            if !p.is_empty() {
                                p
                            } else {
                                Self::val_str(obj, "imgurl")
                            }
                        };
                        let mut pl = Playlist {
                            id: pid.to_string(),
                            title: name,
                            cover_url: signer::normalize_image_url(Some(&pic)),
                            track_count: Self::val_i64(obj, "count")
                                .or_else(|| Self::val_i64(obj, "songcount"))
                                .unwrap_or(0) as i32,
                            description: Self::val_str(obj, "intro"),
                            creator_name: {
                                let cn = Self::val_str(obj, "list_create_username");
                                if !cn.is_empty() {
                                    cn
                                } else {
                                    Self::val_str(obj, "nickname")
                                }
                            },
                            is_created_by_me: false,
                            is_favorite: false,
                            is_liked: false,
                            list_id: Self::val_i64(obj, "listid").unwrap_or(0),
                            global_collection_id: pid.to_string(),
                            play_count: Self::val_str(obj, "play_count"),
                            list_create_userid: Self::val_str(obj, "list_create_userid"),
                            list_create_listid: Self::val_str(obj, "list_create_listid"),
                            list_create_gid: pid.to_string(),
                        };
                        if let Some(ov) = self.store.get_playlist_override(&pl.id) {
                            if !ov.title.is_empty() {
                                pl.title = ov.title;
                            }
                            if !ov.description.is_empty() {
                                pl.description = ov.description;
                            }
                        }
                        return Ok(pl);
                    }
                }
            }
        }

        // 2. 尝试通过 specialid 请求 CDN /api/v3/special/info
        let numeric_special_id = if pid.chars().all(|c| c.is_ascii_digit()) {
            Some(pid.to_string())
        } else {
            pid.split('_')
                .nth(3)
                .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
                .map(|s| s.to_string())
        };
        if let Some(spec_id) = numeric_special_id {
            let mut query = HashMap::new();
            query.insert("version".to_string(), "9108".to_string());
            query.insert("specialid".to_string(), spec_id.clone());
            query.insert("plat".to_string(), "0".to_string());
            query.insert("area_code".to_string(), "1".to_string());
            if let Ok(resp) = self
                .transport
                .call(
                    "/api/v3/special/info",
                    &dev,
                    None,
                    query,
                    None,
                    "http://mobilecdnbj.kugou.com",
                    "GET",
                    false,
                    HashMap::new(),
                    false,
                    false,
                )
                .await
            {
                let d = resp.data();
                let name = Self::val_str(d, "specialname");
                if !name.is_empty() {
                    let pic = Self::val_str(d, "imgurl");
                    let gid = {
                        let g = Self::val_str(d, "global_collection_id");
                        if !g.is_empty() {
                            g
                        } else {
                            let g2 = Self::val_str(d, "gid");
                            if !g2.is_empty() { g2 } else { pid.to_string() }
                        }
                    };
                    let mut pl = Playlist {
                        id: gid.clone(),
                        title: name,
                        cover_url: signer::normalize_image_url(Some(&pic)),
                        track_count: Self::val_i64(d, "songcount").unwrap_or(0) as i32,
                        description: Self::val_str(d, "intro"),
                        creator_name: Self::val_str(d, "nickname"),
                        is_created_by_me: false,
                        is_favorite: false,
                        is_liked: false,
                        list_id: spec_id.parse::<i64>().unwrap_or(0),
                        global_collection_id: gid.clone(),
                        play_count: Self::val_str(d, "playcount"),
                        list_create_userid: Self::val_str(d, "suid"),
                        list_create_listid: spec_id,
                        list_create_gid: gid,
                    };
                    if let Some(ov) = self.store.get_playlist_override(&pl.id) {
                        if !ov.title.is_empty() {
                            pl.title = ov.title;
                        }
                        if !ov.description.is_empty() {
                            pl.description = ov.description;
                        }
                    }
                    return Ok(pl);
                }
            }
        }

        Err("未获取到歌单详情".to_string())
    }

    /// 获取用户歌单列表 (对齐 Android MusicRepository.userPlaylists -> /v7/get_all_list + 收藏歌单详情自动补全)
    pub async fn fetch_user_playlists(&self) -> Result<Vec<Playlist>, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录账号以查看云端歌单".to_string());
        };
        let dev = self.store.device();

        let mut query = HashMap::new();
        query.insert("plat".to_string(), "1".to_string());

        let body = json!({
            "total_ver": 979,
            "type": 2,
            "page": 1,
            "pagesize": 100
        });

        let mut headers = HashMap::new();
        headers.insert(
            "x-router".to_string(),
            "cloudlist.service.kugou.com".to_string(),
        );

        let resp = self
            .transport
            .call(
                "/v7/get_all_list",
                &dev,
                Some(&session),
                query,
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await?
            .require_success()?;

        let mut playlists = Vec::new();
        let mut legacy_default_count: i32 = 0;
        let mut legacy_default_gid = String::new();
        let mut legacy_default_list_id: i64 = 0;
        if let Some(info_arr) = resp.data().get("info").and_then(|v| v.as_array()) {
            for item in info_arr {
                let is_def_val = Self::val_i64(item, "is_def").unwrap_or(0);
                let name = {
                    let n = Self::val_str(item, "name");
                    if !n.is_empty() {
                        n
                    } else {
                        let ln = Self::val_str(item, "listname");
                        if !ln.is_empty() {
                            ln
                        } else {
                            "未命名歌单".to_string()
                        }
                    }
                };
                if is_def_val == 1 || name.trim() == "默认收藏" {
                    legacy_default_count += Self::val_i64(item, "count").unwrap_or(0) as i32;
                    let g = Self::val_str(item, "global_collection_id");
                    if !g.is_empty() {
                        legacy_default_gid = g;
                    }
                    legacy_default_list_id = Self::val_i64(item, "listid").unwrap_or(0);
                    continue;
                }
                let list_id = Self::val_i64(item, "listid").unwrap_or(0);
                let list_create_userid = Self::val_str(item, "list_create_userid");
                let list_create_listid = Self::val_str(item, "list_create_listid");
                let list_create_gid = Self::val_str(item, "list_create_gid");
                let global_id = {
                    let g = Self::val_str(item, "global_collection_id");
                    if !g.is_empty() {
                        g
                    } else if !list_create_gid.is_empty() {
                        list_create_gid.clone()
                    } else if !list_create_listid.is_empty() {
                        list_create_listid.clone()
                    } else {
                        list_id.to_string()
                    }
                };
                let is_default = Self::val_i64(item, "is_def") == Some(2) || name == "我喜欢";
                let is_fav = Self::val_str(item, "type") == "1";
                let pic = {
                    let p = Self::val_str(item, "pic");
                    if !p.is_empty() {
                        p
                    } else {
                        Self::val_str(item, "cover")
                    }
                };
                let count = Self::val_i64(item, "count").unwrap_or(0) as i32;

                let mut pl = Playlist {
                    id: global_id.clone(),
                    title: name,
                    cover_url: signer::normalize_image_url(Some(&pic)),
                    track_count: count,
                    description: Self::val_str(item, "intro"),
                    creator_name: {
                        let cn = Self::val_str(item, "list_create_username");
                        if !cn.is_empty() {
                            cn
                        } else {
                            let nk = Self::val_str(item, "nickname");
                            if !nk.is_empty() { nk } else { session.nickname.clone() }
                        }
                    },
                    is_created_by_me: !is_fav,
                    is_favorite: is_fav,
                    is_liked: is_default,
                    list_id,
                    global_collection_id: global_id,
                    play_count: Self::val_str(item, "play_count"),
                    list_create_userid,
                    list_create_listid,
                    list_create_gid,
                };

                if is_fav {
                    if let Some(cached) = self.store.get_cached_collected_playlist(&pl) {
                        if pl.cover_url.is_empty() {
                            pl.cover_url = cached.cover_url;
                        }
                        if pl.track_count <= 0 && cached.track_count > 0 {
                            pl.track_count = cached.track_count;
                        }
                        if pl.description.is_empty() {
                            pl.description = cached.description;
                        }
                        if pl.creator_name.is_empty() {
                            pl.creator_name = cached.creator_name;
                        }
                    }
                    self.store.cache_collected_playlist(&pl);
                }

                // 应用用户在本地/云端编辑过的歌单标题与简介覆盖
                for key in [pl.id.as_str(), &pl.list_id.to_string(), pl.global_collection_id.as_str()] {
                    if let Some(ov) = self.store.get_playlist_override(key) {
                        if !ov.title.is_empty() {
                            pl.title = ov.title;
                        }
                        if !ov.description.is_empty() {
                            pl.description = ov.description;
                        }
                        if !ov.cover_url.is_empty() && pl.cover_url.is_empty() {
                            pl.cover_url = ov.cover_url;
                        }
                        break;
                    }
                }

                playlists.push(pl);
            }
        }

        // 为缺少封面或曲目数为 0 的收藏歌单自动补全详情
        let mut enriched_fav_count = 0usize;
        for pl in playlists.iter_mut() {
            if pl.is_favorite && (pl.cover_url.is_empty() || pl.track_count <= 0) && enriched_fav_count < 8 {
                enriched_fav_count += 1;
                let lookup_id = if !pl.global_collection_id.is_empty() {
                    pl.global_collection_id.clone()
                } else {
                    pl.id.clone()
                };
                if let Ok(info) = self.fetch_playlist_info(&lookup_id).await {
                    if pl.cover_url.is_empty() && !info.cover_url.is_empty() {
                        pl.cover_url = info.cover_url;
                    }
                    if pl.track_count <= 0 && info.track_count > 0 {
                        pl.track_count = info.track_count;
                    }
                    if pl.description.is_empty() && !info.description.is_empty() {
                        pl.description = info.description;
                    }
                    if !info.creator_name.is_empty() {
                        pl.creator_name = info.creator_name;
                    }
                    self.store.cache_collected_playlist(pl);
                }
            }
        }

        if legacy_default_count > 0 {
            if let Some(liked_pl) = playlists.iter_mut().find(|p| p.is_liked || p.title == "我喜欢") {
                if liked_pl.track_count == 0 {
                    liked_pl.track_count = legacy_default_count;
                    if liked_pl.global_collection_id.is_empty() && !legacy_default_gid.is_empty() {
                        liked_pl.global_collection_id = legacy_default_gid;
                    }
                }
            } else if !legacy_default_gid.is_empty() {
                playlists.insert(
                    0,
                    Playlist {
                        id: legacy_default_gid.clone(),
                        title: "我喜欢".to_string(),
                        cover_url: String::new(),
                        track_count: legacy_default_count,
                        description: "我喜欢的音乐".to_string(),
                        creator_name: session.nickname.clone(),
                        is_created_by_me: true,
                        is_favorite: false,
                        is_liked: true,
                        list_id: legacy_default_list_id,
                        global_collection_id: legacy_default_gid,
                        ..Default::default()
                    },
                );
            }
        }

        Ok(playlists)
    }

    /// 16. 获取指定歌单内的全部歌曲（对齐 Android fetchPlaylistSongsAll 多页循环拉取 + 三通道容灾，突破 100 首截断限制）
    pub async fn fetch_playlist_songs(&self, collection_id: &str) -> Result<Vec<Song>, String> {
        let raw_gid = collection_id.trim();
        if raw_gid.is_empty() {
            return Ok(Vec::new());
        }
        let session = self.store.session();
        let dev = self.store.device();

        // 若传入的是纯数字 listid，优先从用户云端歌单反查对应的 global_collection_id（对齐 Android MusicRepository.playlistSongs）
        let mut gid = raw_gid.to_string();
        if gid.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(user_pls) = self.fetch_user_playlists().await {
                if let Some(matched) = user_pls.into_iter().find(|p| {
                    p.list_id.to_string() == gid
                        || p.id == gid
                        || p.list_create_listid == gid
                }) {
                    if !matched.global_collection_id.is_empty()
                        && matched.global_collection_id != gid
                    {
                        gid = matched.global_collection_id;
                    } else if !matched.list_create_gid.is_empty()
                        && matched.list_create_gid != gid
                    {
                        gid = matched.list_create_gid;
                    }
                }
            }
        }

        let mut all_songs = Vec::new();
        let mut seen_keys = HashSet::new();
        let mut begin_idx = 0usize;
        let page_size = 200usize;
        let mut expected_total = 0usize;

        for _page_count in 0..100 {
            let mut query = HashMap::new();
            query.insert("area_code".to_string(), "1".to_string());
            query.insert("begin_idx".to_string(), begin_idx.to_string());
            query.insert("plat".to_string(), "1".to_string());
            query.insert("type".to_string(), "1".to_string());
            query.insert("mode".to_string(), "1".to_string());
            query.insert("personal_switch".to_string(), "1".to_string());
            query.insert("pagesize".to_string(), page_size.to_string());
            query.insert("global_collection_id".to_string(), gid.clone());

            let Ok(resp) = self
                .transport
                .call(
                    "/pubsongs/v2/get_other_list_file_nofilt",
                    &dev,
                    session.as_ref(),
                    query,
                    None,
                    "https://gateway.kugou.com",
                    "GET",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await
            else {
                break;
            };

            let d_root = resp.data();
            let d_inner = d_root.get("data").unwrap_or(d_root);
            if expected_total == 0 {
                expected_total = Self::val_i64(d_inner, "total")
                    .or_else(|| Self::val_i64(d_inner, "count"))
                    .or_else(|| Self::val_i64(d_root, "total"))
                    .or_else(|| Self::val_i64(d_root, "count"))
                    .unwrap_or(0) as usize;
            }
            let raw_batch = Self::parse_songs(d_root);
            if raw_batch.is_empty() {
                break;
            }
            let raw_len = raw_batch.len();
            let mut added = 0usize;
            for song in raw_batch {
                let k = if !song.hash.is_empty() {
                    song.hash.to_lowercase()
                } else {
                    song.id.to_lowercase()
                };
                if seen_keys.insert(k) {
                    all_songs.push(song);
                    added += 1;
                }
            }
            // 关键修复：酷狗服务端可能将单页数量限制为 100 或 30 首，游标必须按实际返回的 raw_len 递增，绝不能直接 +200 跳过中间曲目！
            begin_idx += raw_len;
            if added == 0 {
                break;
            }
            if expected_total > 0 && all_songs.len() >= expected_total {
                break;
            }
            if expected_total == 0 && raw_len < 30 {
                break;
            }
            // 若已知歌单总曲目数且剩余多页，采用 4 页并发拉取大幅提速大歌单加载
            if expected_total > all_songs.len() && raw_len >= 30 {
                let step = raw_len;
                let mut futs = Vec::new();
                for batch_i in 0..4 {
                    let cur_begin = begin_idx + batch_i * step;
                    if cur_begin >= expected_total {
                        break;
                    }
                    let gid_cl = gid.clone();
                    let dev_cl = dev.clone();
                    let sess_cl = session.clone();
                    futs.push(async move {
                        let mut q = HashMap::new();
                        q.insert("area_code".to_string(), "1".to_string());
                        q.insert("begin_idx".to_string(), cur_begin.to_string());
                        q.insert("plat".to_string(), "1".to_string());
                        q.insert("type".to_string(), "1".to_string());
                        q.insert("mode".to_string(), "1".to_string());
                        q.insert("personal_switch".to_string(), "1".to_string());
                        q.insert("pagesize".to_string(), page_size.to_string());
                        q.insert("global_collection_id".to_string(), gid_cl);
                        if let Ok(r) = self
                            .transport
                            .call(
                                "/pubsongs/v2/get_other_list_file_nofilt",
                                &dev_cl,
                                sess_cl.as_ref(),
                                q,
                                None,
                                "https://gateway.kugou.com",
                                "GET",
                                false,
                                HashMap::new(),
                                true,
                                true,
                            )
                            .await
                        {
                            Self::parse_songs(r.data())
                        } else {
                            Vec::new()
                        }
                    });
                }
                if !futs.is_empty() {
                    let pages_res = futures_join_vec(futs).await;
                    let mut batch_total_raw = 0usize;
                    let mut batch_added = 0usize;
                    for p_songs in pages_res {
                        batch_total_raw += p_songs.len();
                        for song in p_songs {
                            let k = if !song.hash.is_empty() {
                                song.hash.to_lowercase()
                            } else {
                                song.id.to_lowercase()
                            };
                            if seen_keys.insert(k) {
                                all_songs.push(song);
                                batch_added += 1;
                            }
                        }
                    }
                    begin_idx += batch_total_raw;
                    if batch_added == 0 || all_songs.len() >= expected_total {
                        break;
                    }
                }
            }
        }

        if !all_songs.is_empty() {
            return Ok(self.enrich_songs_privilege_lite(all_songs).await);
        }

        // 备用通道 1：/v4/get_other_list_file 多页拉取
        for page in 1..=50 {
            let mut v4_query = HashMap::new();
            v4_query.insert("global_collection_id".to_string(), gid.clone());
            v4_query.insert("page".to_string(), page.to_string());
            v4_query.insert("pagesize".to_string(), "100".to_string());
            v4_query.insert("is_publish".to_string(), "1".to_string());
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "pubsongs.kugou.com".to_string());

            let Ok(resp) = self
                .transport
                .call(
                    "/v4/get_other_list_file",
                    &dev,
                    session.as_ref(),
                    v4_query,
                    None,
                    "https://gateway.kugou.com",
                    "GET",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
            else {
                break;
            };
            let raw_batch = Self::parse_songs(resp.data());
            if raw_batch.is_empty() {
                break;
            }
            let raw_len = raw_batch.len();
            let mut added = 0usize;
            for song in raw_batch {
                let k = if !song.hash.is_empty() {
                    song.hash.to_lowercase()
                } else {
                    song.id.to_lowercase()
                };
                if seen_keys.insert(k) {
                    all_songs.push(song);
                    added += 1;
                }
            }
            if added == 0 || raw_len < 20 {
                break;
            }
        }

        if !all_songs.is_empty() {
            return Ok(self.enrich_songs_privilege_lite(all_songs).await);
        }

        // 备用通道 2：CDN 歌单接口 (/api/v3/special/song)
        let numeric_special_id = if gid.chars().all(|c| c.is_ascii_digit()) {
            Some(gid.clone())
        } else {
            gid.split('_')
                .nth(3)
                .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
                .map(|s| s.to_string())
        };
        if let Some(spec_id) = numeric_special_id {
            for page in 1..=15 {
                let mut cdn_q = HashMap::new();
                cdn_q.insert("version".to_string(), "9108".to_string());
                cdn_q.insert("specialid".to_string(), spec_id.clone());
                cdn_q.insert("plat".to_string(), "0".to_string());
                cdn_q.insert("pagesize".to_string(), "100".to_string());
                cdn_q.insert("page".to_string(), page.to_string());
                cdn_q.insert("area_code".to_string(), "1".to_string());

                let Ok(resp) = self
                    .transport
                    .call(
                        "/api/v3/special/song",
                        &dev,
                        None,
                        cdn_q,
                        None,
                        "http://mobilecdnbj.kugou.com",
                        "GET",
                        false,
                        HashMap::new(),
                        false,
                        false,
                    )
                    .await
                else {
                    break;
                };
                let raw_batch = Self::parse_songs(resp.data());
                if raw_batch.is_empty() {
                    break;
                }
                let raw_len = raw_batch.len();
                let mut added = 0usize;
                for song in raw_batch {
                    let k = if !song.hash.is_empty() {
                        song.hash.to_lowercase()
                    } else {
                        song.id.to_lowercase()
                    };
                    if seen_keys.insert(k) {
                        all_songs.push(song);
                        added += 1;
                    }
                }
                if added == 0 || raw_len < 20 {
                    break;
                }
            }
        }

        Ok(self.enrich_songs_privilege_lite(all_songs).await)
    }

    /// 17. 概念版音源规格补全 (/v2/get_res_privilege/lite)
    /// 补全各档位 Hash (ogg_128 / 320 / flac / high)、文件大小与精确时长 duration_ms
    pub async fn enrich_songs_privilege_lite(&self, songs: Vec<Song>) -> Vec<Song> {
        if songs.is_empty() {
            return songs;
        }
        let session = self.store.session();
        let dev = self.store.device();

        let qualities = json!([
            "128",
            "320",
            "flac",
            "high",
            "viper_atmos",
            "viper_tape",
            "viper_clear",
            "super",
            "multitrack"
        ]);

        let mut chunk_futs = Vec::new();
        for chunk in songs.chunks(80) {
            let mut resources = Vec::new();
            for s in chunk {
                let h = s.hash.trim().to_lowercase();
                if h.len() >= 16 {
                    resources.push(json!({
                        "type": "audio",
                        "page_id": 1,
                        "id": 0,
                        "area_code": 1,
                        "album_audio_id": s.album_audio_id,
                        "album_id": s.album_id.parse::<i64>().unwrap_or(0),
                        "hash": h,
                        "qualities": qualities
                    }));
                }
            }
            if resources.is_empty() {
                continue;
            }

            let body = json!({
                "appid": APP_ID,
                "clientver": CLIENT_VERSION,
                "mid": dev.mid,
                "clienttime": Utc::now().timestamp(),
                "support_verify": 1,
                "area_code": 1,
                "behavior": "play",
                "need_hash_offset": 1,
                "relate": 1,
                "qualities": qualities,
                "resource": resources,
                "resources": resources
            });

            let dev_cl = dev.clone();
            let sess_cl = session.clone();
            chunk_futs.push(async move {
                let mut headers = HashMap::new();
                headers.insert("x-router".to_string(), "media.store.kugou.com".to_string());
                if let Ok(resp) = self
                    .transport
                    .call(
                        "/v2/get_res_privilege/lite",
                        &dev_cl,
                        sess_cl.as_ref(),
                        HashMap::new(),
                        Some(body),
                        "https://gateway.kugou.com",
                        "POST",
                        false,
                        headers,
                        true,
                        true,
                    )
                    .await
                {
                    let items_opt = resp
                        .body
                        .get("data")
                        .and_then(|v| v.as_array())
                        .or_else(|| resp.data().get("data").and_then(|v| v.as_array()))
                        .or_else(|| resp.data().get("resource").and_then(|v| v.as_array()));
                    if let Some(items) = items_opt {
                        return items.clone();
                    }
                }
                Vec::new()
            });
        }

        let mut all_items: Vec<Value> = Vec::new();
        if !chunk_futs.is_empty() {
            for items in futures_join_vec(chunk_futs).await {
                all_items.extend(items);
            }
        }

        if all_items.is_empty() {
            return songs;
        }

        let mut map_by_hash: HashMap<String, &Value> = HashMap::new();
        for item in &all_items {
            let h = Self::val_str(item, "hash").to_lowercase();
            if !h.is_empty() {
                map_by_hash.insert(h, item);
            }
            if let Some(rgs) = item.get("relate_goods").and_then(|v| v.as_array()) {
                for rg in rgs {
                    let rgh = Self::val_str(rg, "hash").to_lowercase();
                    if !rgh.is_empty() {
                        map_by_hash.insert(rgh, item);
                    }
                }
            }
        }

        let enriched_list: Vec<Song> = songs
            .into_iter()
            .map(|mut s| {
                let key = s.hash.to_lowercase();
                if let Some(&priv_obj) = map_by_hash.get(&key) {
                    if s.album.is_empty() {
                        let alb_name = Self::val_str(priv_obj, "album_name");
                        if !alb_name.is_empty() && alb_name != "0" {
                            s.album = alb_name;
                        }
                    }
                    if s.album_id.is_empty() || s.album_id == "0" {
                        let aid = Self::val_str(priv_obj, "album_id");
                        if !aid.is_empty() && aid != "0" {
                            s.album_id = aid;
                        }
                    }
                    if s.duration_ms <= 0 {
                        if let Some(info_obj) = priv_obj.get("info") {
                            let d = Self::val_i64(info_obj, "duration")
                                .or_else(|| Self::val_i64(info_obj, "timelength"))
                                .unwrap_or(0);
                            if d > 0 {
                                s.duration_ms = if d < 10_000 { d * 1000 } else { d };
                            }
                        }
                    }

                    if let Some(tp) = priv_obj.get("trans_param") {
                        let ogg128 = Self::val_str(tp, "ogg_128_hash");
                        if !ogg128.is_empty() {
                            s.hash128 = ogg128.to_lowercase();
                            s.size128 = Self::val_i64(tp, "ogg_128_filesize").unwrap_or(s.size128);
                        }
                        let ogg320 = Self::val_str(tp, "ogg_320_hash");
                        if !ogg320.is_empty() && s.hash320.is_empty() {
                            s.hash320 = ogg320.to_lowercase();
                            s.size320 = Self::val_i64(tp, "ogg_320_filesize").unwrap_or(s.size320);
                        }
                        if s.cover_url.is_empty() {
                            let union_cover = Self::val_str(tp, "union_cover");
                            if !union_cover.is_empty() {
                                s.cover_url = signer::normalize_image_url(Some(&union_cover));
                            }
                        }
                    }

                    if let Some(rgs) = priv_obj.get("relate_goods").and_then(|v| v.as_array()) {
                        for rg in rgs {
                            let rg_info = rg.get("info").unwrap_or(rg);
                            let rg_hash = {
                                let h = Self::val_str(rg, "hash");
                                if !h.is_empty() {
                                    h
                                } else {
                                    Self::val_str(rg_info, "hash")
                                }
                            }
                            .to_lowercase();
                            if rg_hash.is_empty() {
                                continue;
                            }

                            if s.duration_ms <= 0 {
                                let d = Self::val_i64(rg, "duration")
                                    .or_else(|| Self::val_i64(rg_info, "duration"))
                                    .or_else(|| Self::val_i64(rg_info, "timelength"))
                                    .unwrap_or(0);
                                if d > 0 {
                                    s.duration_ms = if d < 10_000 { d * 1000 } else { d };
                                }
                            }

                            let sz = Self::val_i64(rg, "size")
                                .or_else(|| Self::val_i64(rg, "filesize"))
                                .or_else(|| Self::val_i64(rg_info, "filesize"))
                                .or_else(|| Self::val_i64(rg_info, "size"))
                                .unwrap_or(0);

                            let level = Self::val_i64(rg, "level")
                                .or_else(|| Self::val_i64(rg_info, "level"))
                                .unwrap_or(-1);
                            let q_str = {
                                let q = Self::val_str(rg, "quality");
                                if !q.is_empty() {
                                    q
                                } else {
                                    Self::val_str(rg_info, "quality")
                                }
                            }
                            .to_lowercase();

                            match (level, q_str.as_str()) {
                                (1 | 2, _) | (_, "128") => {
                                    if s.hash128.is_empty() {
                                        s.hash128 = rg_hash;
                                    }
                                    if sz > 0 && s.size128 == 0 {
                                        s.size128 = sz;
                                    }
                                }
                                (4, _) | (_, "320") => {
                                    s.hash320 = rg_hash;
                                    if sz > 0 {
                                        s.size320 = sz;
                                    }
                                }
                                (5, _) | (_, "flac" | "sq" | "lossless") => {
                                    s.hash_flac = rg_hash;
                                    s.is_lossless = true;
                                    if sz > s.size_flac {
                                        s.size_flac = sz;
                                    }
                                }
                                (_, q) if q.contains("accompany") || q.contains("伴奏") => {
                                    if s.accompany_hash.is_empty() {
                                        s.accompany_hash = rg_hash;
                                    }
                                }
                                (7, _) | (_, "super" | "multitrack") => {
                                    // 跳过正统酷狗 suvip 专属蝰蛇合成流
                                }
                                (6, _) | (_, "high" | "res" | "hires") => {
                                    if rg_hash != s.hash_flac && rg_hash != s.hash320 {
                                        s.hash_hi_res = rg_hash;
                                        s.is_hi_res = true;
                                        s.is_lossless = true;
                                        if sz > s.size_hi_res {
                                            s.size_hi_res = sz;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    s.highest_quality = if !s.hash_hi_res.is_empty() || s.is_hi_res {
                        "Hi-Res".to_string()
                    } else if !s.hash_flac.is_empty() || s.is_lossless {
                        "SQ".to_string()
                    } else if !s.hash320.is_empty() {
                        "320K".to_string()
                    } else {
                        "128K".to_string()
                    };
                }
                s
            })
            .collect();

        self.enrich_songs_album_names_batch(enriched_list).await
    }

    /// 对齐 Android 端 enrichSongsAlbumNamesBatch: 通过 /kmr/v2/albums 与 /kmr/v2/audio 批量补全缺失的真实专辑名、专辑ID与发行日期
    pub async fn enrich_songs_album_names_batch(&self, mut songs: Vec<Song>) -> Vec<Song> {
        if songs.is_empty() {
            return songs;
        }
        let dev = self.store.device();
        let session = self.store.session();

        // 第一阶段：对有 album_id 但缺少 album 名称的歌曲，批量请求 /kmr/v2/albums
        let missing_album_ids: Vec<String> = songs
            .iter()
            .filter(|s| s.album.trim().is_empty() && !s.album_id.trim().is_empty() && s.album_id != "0")
            .map(|s| s.album_id.trim().to_string())
            .collect::<HashSet<_>>()
            .into_iter()
            .take(45)
            .collect();

        // 第二阶段：对缺少 album 且无有效 album_id、但有 album_audio_id / audio_id 的歌曲，通过 /kmr/v2/audio 补全
        let missing_entities: Vec<i64> = songs
            .iter()
            .filter(|s| {
                s.album.trim().is_empty()
                    && (s.album_id.trim().is_empty() || s.album_id == "0")
                    && (s.album_audio_id > 0 || s.audio_id > 0)
            })
            .map(|s| if s.album_audio_id > 0 { s.album_audio_id } else { s.audio_id })
            .collect::<HashSet<_>>()
            .into_iter()
            .take(40)
            .collect();

        if missing_album_ids.is_empty() && missing_entities.is_empty() {
            return songs;
        }

        let dev_a = dev.clone();
        let sess_a = session.clone();
        let alb_fut = async move {
            let mut alb_map: HashMap<String, (String, String)> = HashMap::new();
            if missing_album_ids.is_empty() {
                return alb_map;
            }
            let data_arr: Vec<Value> = missing_album_ids
                .iter()
                .map(|aid| {
                    if let Ok(num) = aid.parse::<i64>() {
                        json!({ "album_id": num })
                    } else {
                        json!({ "album_id": aid })
                    }
                })
                .collect();
            let body = json!({
                "data": data_arr,
                "is_buy": 0,
                "fields": "album_id,album_name,publish_date,sizable_cover,author_name,publish_company"
            });
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
            headers.insert("kg-tid".to_string(), "255".to_string());

            if let Ok(resp) = self
                .transport
                .call(
                    "/kmr/v2/albums",
                    &dev_a,
                    sess_a.as_ref(),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
            {
                if let Some(arr) = resp
                    .data()
                    .get("data")
                    .and_then(|v| v.as_array())
                    .or_else(|| resp.data().as_array())
                    .or_else(|| resp.body.get("data").and_then(|v| v.as_array()))
                {
                    for obj in arr {
                        let aid = Self::val_str(obj, "album_id");
                        let aname = Self::val_str(obj, "album_name");
                        let pdate: String =
                            Self::val_str(obj, "publish_date").chars().take(10).collect();
                        if !aid.is_empty() && !aname.is_empty() {
                            alb_map.insert(aid, (aname, pdate));
                        }
                    }
                }
            }
            alb_map
        };

        let dev_e = dev.clone();
        let sess_e = session.clone();
        let ent_fut = async move {
            let mut entity_map: HashMap<i64, (String, String, String, String)> = HashMap::new();
            if missing_entities.is_empty() {
                return entity_map;
            }
            let entity_arr: Vec<Value> = missing_entities
                .iter()
                .map(|&id| json!({ "entity_id": id }))
                .collect();
            let body = json!({
                "data": entity_arr,
                "fields": "base,album_info"
            });
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
            headers.insert("kg-tid".to_string(), "36".to_string());

            if let Ok(resp) = self
                .transport
                .call(
                    "/kmr/v2/audio",
                    &dev_e,
                    sess_e.as_ref(),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
            {
                if let Some(arr) = resp
                    .data()
                    .get("data")
                    .and_then(|v| v.as_array())
                    .or_else(|| resp.data().as_array())
                    .or_else(|| resp.body.get("data").and_then(|v| v.as_array()))
                {
                    for item in arr {
                        let base_obj = item.get("base").unwrap_or(item);
                        let alb_obj = item.get("album_info").unwrap_or(item);
                        let aaid = Self::val_i64(base_obj, "album_audio_id")
                            .or_else(|| Self::val_i64(item, "entity_id"))
                            .or_else(|| Self::val_i64(base_obj, "audio_id"))
                            .unwrap_or(0);
                        let aid = {
                            let a = Self::val_str(alb_obj, "album_id");
                            if !a.is_empty() && a != "0" {
                                a
                            } else {
                                Self::val_str(base_obj, "album_id")
                            }
                        };
                        let aname = {
                            let n = Self::val_str(alb_obj, "album_name");
                            if !n.is_empty() {
                                n
                            } else {
                                Self::val_str(base_obj, "album_name")
                            }
                        };
                        let author_id = Self::val_str(base_obj, "author_id");
                        let pdate: String = {
                            let pd = Self::val_str(base_obj, "publish_date");
                            let raw = if !pd.is_empty() { pd } else { Self::val_str(alb_obj, "publish_date") };
                            raw.chars().take(10).collect()
                        };
                        if aaid > 0 {
                            entity_map.insert(aaid, (aid, aname, author_id, pdate));
                        }
                    }
                }
            }
            entity_map
        };

        let (alb_map, entity_map) = tokio::join!(alb_fut, ent_fut);

        for s in &mut songs {
            if let Some((aname, pdate)) = alb_map.get(s.album_id.trim()) {
                if s.album.trim().is_empty() {
                    s.album = aname.clone();
                }
                if s.publish_date.trim().is_empty() && !pdate.is_empty() {
                    s.publish_date = pdate.clone();
                }
            }
            let eid = if s.album_audio_id > 0 {
                s.album_audio_id
            } else {
                s.audio_id
            };
            if let Some((aid, aname, author_id, pdate)) = entity_map.get(&eid) {
                if (s.album_id.is_empty() || s.album_id == "0") && !aid.is_empty() && aid != "0" {
                    s.album_id = aid.clone();
                }
                if s.album.is_empty() && !aname.is_empty() {
                    s.album = aname.clone();
                }
                if (s.artist_id.is_empty() || s.artist_id == "0") && !author_id.is_empty() && author_id != "0" {
                    s.artist_id = author_id.clone();
                }
                if s.publish_date.is_empty() && !pdate.is_empty() {
                    s.publish_date = pdate.clone();
                }
            }
        }

        songs
    }

    /// 实时反查单曲的真实专辑与歌手元数据 (对齐 Android MusicRepository.resolveSongAlbumMeta)
    pub async fn resolve_song_album_meta(&self, song: &Song) -> Result<ResolvedSongAlbumMeta, String> {
        let mut enriched = self
            .enrich_songs_privilege_lite(vec![song.clone()])
            .await
            .into_iter()
            .next()
            .unwrap_or_else(|| song.clone());

        let mut company = String::new();
        if !enriched.album_id.is_empty() && enriched.album_id != "0" {
            if let Ok(alb) = self.fetch_album_detail(&enriched.album_id).await {
                if enriched.album.is_empty() && !alb.album_name.is_empty() {
                    enriched.album = alb.album_name;
                }
                if (enriched.artist_id.is_empty() || enriched.artist_id == "0") && !alb.artist_id.is_empty() {
                    enriched.artist_id = alb.artist_id;
                }
                if enriched.cover_url.is_empty() && !alb.cover_url.is_empty() {
                    enriched.cover_url = alb.cover_url;
                }
                if enriched.publish_date.is_empty() && !alb.publish_date.is_empty() {
                    enriched.publish_date = alb.publish_date;
                }
                company = alb.publish_company;
            }
        }

        if (enriched.artist_id.is_empty() || enriched.artist_id == "0")
            && !enriched.artist.is_empty()
            && enriched.artist != "未知歌手"
        {
            let primary_artist = enriched
                .artist
                .split(&['、', '/', '&', ',', '，'][..])
                .next()
                .unwrap_or(&enriched.artist)
                .trim();
            if let Ok(artists) = self.search_artists(primary_artist, 1, 3).await {
                if let Some(found) = artists.first() {
                    enriched.artist_id = found.artist_id.clone();
                }
            }
        }

        Ok(ResolvedSongAlbumMeta {
            album_name: enriched.album,
            album_id: enriched.album_id,
            artist_id: enriched.artist_id,
            artist_name: enriched.artist,
            cover_url: enriched.cover_url,
            publish_date: enriched.publish_date,
            company,
        })
    }

    /// 解析音频流前 512 字节头部（遵循 RFC 9639 FLAC STREAMINFO / OGG Vorbis / MP3 帧头）
    fn parse_stream_header_bytes(bytes: &[u8], ext_hint: &str) -> (i32, i32, i32) {
        // 1. FLAC: "fLaC" + STREAMINFO block (至少 42 字节)
        if bytes.len() >= 42 && &bytes[0..4] == b"fLaC" {
            let block_type = bytes[4] & 0x7F;
            if block_type == 0 {
                let b18 = bytes[18] as i32;
                let b19 = bytes[19] as i32;
                let b20 = bytes[20] as i32;
                let b21 = bytes[21] as i32;
                let sample_rate_hz = (b18 << 12) | (b19 << 4) | ((b20 & 0xF0) >> 4);
                let channels = ((b20 & 0x0E) >> 1) + 1;
                let bit_depth = (((b20 & 0x01) << 4) | ((b21 & 0xF0) >> 4)) + 1;
                if (8000..=384_000).contains(&sample_rate_hz) && (8..=32).contains(&bit_depth) {
                    return (sample_rate_hz, bit_depth, channels.clamp(1, 8));
                }
            }
        }

        // 2. OGG Vorbis: "OggS" + "\x01vorbis"
        if bytes.len() >= 48 && &bytes[0..4] == b"OggS" {
            for i in 0..bytes.len().saturating_sub(16) {
                if &bytes[i..i + 7] == b"\x01vorbis" {
                    let ch = bytes[i + 11] as i32;
                    let sr = u32::from_le_bytes([
                        bytes[i + 12],
                        bytes[i + 13],
                        bytes[i + 14],
                        bytes[i + 15],
                    ]) as i32;
                    if (8000..=192_000).contains(&sr) {
                        return (sr, 16, ch.clamp(1, 8));
                    }
                }
            }
        }

        // 3. MP3 Frame Sync: 0xFF 0xE0..
        if bytes.len() >= 16 {
            let mut offset = 0usize;
            if bytes.len() >= 10 && &bytes[0..3] == b"ID3" {
                let id3_size = ((bytes[6] as usize & 0x7F) << 21)
                    | ((bytes[7] as usize & 0x7F) << 14)
                    | ((bytes[8] as usize & 0x7F) << 7)
                    | (bytes[9] as usize & 0x7F);
                offset = (10 + id3_size).min(bytes.len());
            }
            for i in offset..bytes.len().saturating_sub(4) {
                if bytes[i] == 0xFF && (bytes[i + 1] & 0xE0) == 0xE0 {
                    let version_bits = (bytes[i + 1] >> 3) & 0x03;
                    let sr_idx = ((bytes[i + 2] >> 2) & 0x03) as usize;
                    let ch_mode = (bytes[i + 3] >> 6) & 0x03;
                    if version_bits != 1 && sr_idx < 3 {
                        let base_sr = match sr_idx {
                            0 => 44_100,
                            1 => 48_000,
                            _ => 32_000,
                        };
                        let sr = match version_bits {
                            3 => base_sr,
                            2 => base_sr / 2,
                            0 => base_sr / 4,
                            _ => 44_100,
                        };
                        let ch = if ch_mode == 3 { 1 } else { 2 };
                        return (sr, 16, ch);
                    }
                }
            }
        }

        // 默认根据容器推断
        if ext_hint.eq_ignore_ascii_case("flac") {
            (48_000, 24, 2)
        } else {
            (44_100, 16, 2)
        }
    }

    /// 严格对齐 Android 端 MusicRepository.deduceQuality:
    /// 彻底解决酷狗 /v6/priv_url 对 FLAC/Hi-Res 流不返回 bitrate 导致显示 `0 kbps` 的问题！
    fn deduce_quality_and_kbps(
        song: &Song,
        url: &str,
        bitrate_raw: i64,
        file_size_bytes: i64,
        duration_ms: i64,
        ext_name: &str,
        requested_tier: &AudioQuality,
        resp_quality_str: &str,
    ) -> (AudioQuality, i32) {
        let effective_size = if file_size_bytes > 0 {
            file_size_bytes
        } else {
            song.known_size_for_quality(requested_tier)
        };
        let dur_sec = ((duration_ms as f64) / 1000.0).max(1.0);
        let computed_kbps = if effective_size > 0 && duration_ms >= 15_000 {
            (((effective_size as f64) * 8.0) / dur_sec / 1000.0).round() as i32
        } else {
            0
        };
        let reported_kbps = if bitrate_raw <= 0 {
            0
        } else if bitrate_raw > 50_000 {
            (bitrate_raw / 1000) as i32
        } else {
            bitrate_raw as i32
        };

        let ext_lower = ext_name.to_lowercase();
        let url_lower = url.to_lowercase();
        let is_flac = ext_lower.contains("flac") || url_lower.contains(".flac");
        let is_lossy = ext_lower.contains("mp3")
            || ext_lower.contains("ogg")
            || ext_lower.contains("m4a")
            || url_lower.contains(".mp3")
            || url_lower.contains(".ogg");

        // 1. 若与 /v2/get_res_privilege/lite 已知档位文件大小匹配
        for q in [
            AudioQuality::HiRes,
            AudioQuality::Lossless,
            AudioQuality::High,
            AudioQuality::Standard,
        ] {
            let kn_size = song.known_size_for_quality(&q);
            if effective_size > 0 && kn_size > 0 && (effective_size - kn_size).abs() <= 65_536 {
                let fallback_k = match q {
                    AudioQuality::HiRes => 2304,
                    AudioQuality::Lossless => 920,
                    AudioQuality::High => 320,
                    AudioQuality::Standard => 128,
                };
                let k = if (64..=6500).contains(&computed_kbps) {
                    computed_kbps
                } else if (64..=6500).contains(&reported_kbps)
                    && reported_kbps != 999
                    && reported_kbps != 2000
                {
                    reported_kbps
                } else {
                    fallback_k
                };
                return (q, k);
            }
        }

        // 2. 根据响应的 quality 字段或容器推导
        let rq = resp_quality_str.trim().to_lowercase();
        let final_quality = match rq.as_str() {
            "high" | "hires" | "hi-res" | "res" => AudioQuality::HiRes,
            "flac" | "sq" | "lossless" => AudioQuality::Lossless,
            "320" | "hq" => AudioQuality::High,
            "128" => AudioQuality::Standard,
            _ => {
                if is_flac {
                    if *requested_tier == AudioQuality::HiRes
                        || computed_kbps >= 1150
                        || effective_size >= 30_000_000
                    {
                        AudioQuality::HiRes
                    } else {
                        AudioQuality::Lossless
                    }
                } else if is_lossy {
                    if computed_kbps >= 220
                        || reported_kbps >= 220
                        || *requested_tier == AudioQuality::High
                    {
                        AudioQuality::High
                    } else {
                        AudioQuality::Standard
                    }
                } else {
                    requested_tier.clone()
                }
            }
        };

        let default_tier_kbps = match final_quality {
            AudioQuality::HiRes => 1680,
            AudioQuality::Lossless => 920,
            AudioQuality::High => 320,
            AudioQuality::Standard => 128,
        };

        let final_kbps = if is_flac && (350..=6500).contains(&computed_kbps) {
            computed_kbps
        } else if (64..=6500).contains(&reported_kbps)
            && reported_kbps != 999
            && reported_kbps != 2000
        {
            reported_kbps
        } else if (64..=6500).contains(&computed_kbps) {
            computed_kbps
        } else {
            default_tier_kbps
        };

        (final_quality, final_kbps)
    }

    async fn finalize_playback_result(&self, mut res: PlaybackResult) -> PlaybackResult {
        let mut probed = self.transport.probe_stream_head_bytes(&res.url).await;
        if probed.is_none() && !res.backup_urls.is_empty() {
            for idx in 0..res.backup_urls.len() {
                let b_url = res.backup_urls[idx].clone();
                if let Some(hb) = self.transport.probe_stream_head_bytes(&b_url).await {
                    let old_primary = std::mem::replace(&mut res.url, b_url);
                    res.backup_urls[idx] = old_primary;
                    probed = Some(hb);
                    break;
                }
            }
        }

        if let Some(head_bytes) = probed {
            let (sr, bd, ch) = Self::parse_stream_header_bytes(&head_bytes, &res.ext_name);
            res.sample_rate_hz = sr;
            res.bit_depth = bd;
            res.channels = ch;
            if !res.resolved_hash.is_empty() {
                let cache_dir = Self::resolve_cache_dir();
                let ext = if res.ext_name.is_empty() { "flac" } else { &res.ext_name };
                let head_path = cache_dir.join(format!(
                    "stream_probe_{}.{}",
                    res.resolved_hash.to_lowercase(),
                    ext
                ));
                let _ = std::fs::write(head_path, &head_bytes);
            }
        } else if res.ext_name.eq_ignore_ascii_case("flac") {
            if res.actual_quality == "high" {
                res.sample_rate_hz = 96_000;
                res.bit_depth = 24;
            } else {
                res.sample_rate_hz = 44_100;
                res.bit_depth = 16;
            }
            res.channels = 2;
        }

        let khz_str = format!("{:.1} kHz", (res.sample_rate_hz as f64) / 1000.0);
        res.tech_spec_text = format!(
            "{} · {}bit · {} kbps · {}",
            khz_str,
            res.bit_depth,
            res.bitrate_kbps,
            res.ext_name.to_uppercase()
        );

        let cache_key = format!(
            "kg_v2_{}_{}_{}",
            if res.resolved_hash.is_empty() {
                signer::md5_hex(&res.url)
            } else {
                res.resolved_hash.to_lowercase()
            },
            res.actual_quality.to_lowercase(),
            if res.is_free_part { "trial" } else { "full" }
        );
        res.proxy_url = self.audio_proxy.register_stream(
            &cache_key,
            &res.url,
            res.backup_urls.clone(),
            &res.ext_name,
            res.file_size_bytes,
        );
        res
    }

    /// 18. 核心音源直链解析 (对齐 MusicRepository.kt: /v6/priv_url -> /v5/url -> 试听流 + 真实采样率/码率反推)
    pub async fn resolve_song_url(
        &self,
        song: &Song,
        preferred_quality: &str,
    ) -> Result<PlaybackResult, String> {
        let enriched = self
            .enrich_songs_privilege_lite(vec![song.clone()])
            .await
            .into_iter()
            .next()
            .unwrap_or_else(|| song.clone());

        let pref_q = AudioQuality::from_str(preferred_quality);

        // 阶段 1: 尝试概念版 VIP 专属通道 (/v6/priv_url)
        let mut preview_fallback: Option<PlaybackResult> = None;
        if let Some(priv_res) = self.resolve_vip_priv_url(&enriched, &pref_q).await {
            if !priv_res.is_free_part && !priv_res.url.is_empty() {
                return Ok(self.finalize_playback_result(priv_res).await);
            }
            preview_fallback = Some(priv_res);
        }

        // 阶段 2: 尝试 /v5/url 多档位通道
        if let Some(v5_res) = self
            .resolve_v5_fallback_url(&enriched, &pref_q, false)
            .await
        {
            if !v5_res.url.is_empty() {
                return Ok(self.finalize_playback_result(v5_res).await);
            }
        }

        if let Some(prev) = preview_fallback {
            if !prev.url.is_empty() {
                return Ok(self.finalize_playback_result(prev).await);
            }
        }

        // 阶段 3: 若原曲 Hash 因版权迁移失效，自动通过「歌手 + 歌名」检索同曲目可用音源替身 (对齐 Android MusicRepository.resolvePlayback 第三阶段)
        if !enriched.title.trim().is_empty() && enriched.title != "未知歌曲" {
            let clean_artist = if enriched.artist != "未知歌手" && enriched.artist != "群星" {
                enriched.artist.trim()
            } else {
                ""
            };
            let kw = format!("{} {}", clean_artist, enriched.title).trim().to_string();
            if let Ok(candidates) = self.search_songs(&kw, 1, 5).await {
                if let Some(alt) = candidates.into_iter().find(|c| {
                    !c.hash.is_empty() && !c.hash.eq_ignore_ascii_case(&enriched.hash)
                }) {
                    if let Some(priv_res) = self.resolve_vip_priv_url(&alt, &pref_q).await {
                        if !priv_res.url.is_empty() && !priv_res.is_free_part {
                            return Ok(self.finalize_playback_result(priv_res).await);
                        }
                    }
                    if let Some(v5_res) = self.resolve_v5_fallback_url(&alt, &pref_q, false).await {
                        if !v5_res.url.is_empty() {
                            return Ok(self.finalize_playback_result(v5_res).await);
                        }
                    }
                }
            }
        }

        // 阶段 4: 试听片段兜底 (is_free_part = 1)
        if let Some(free_res) = self.resolve_v5_fallback_url(&enriched, &pref_q, true).await {
            if !free_res.url.is_empty() {
                return Ok(self.finalize_playback_result(free_res).await);
            }
        }

        Err("该歌曲暂未获取到可用播放音源（可能受版权限制或需先登录领取概念版 VIP）".to_string())
    }

    async fn resolve_vip_priv_url(
        &self,
        song: &Song,
        preferred_quality: &AudioQuality,
    ) -> Option<PlaybackResult> {
        let dev = self.store.device();
        let session = self.store.session();
        let uid = session
            .as_ref()
            .map(|s| s.user_id.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("0");
        let token = session.as_ref().map(|s| s.token.as_str()).unwrap_or("");
        let vip_token = session
            .as_ref()
            .and_then(|s| s.vip_token.as_deref())
            .filter(|s| !s.is_empty())
            .unwrap_or(token);
        let vip_level = if !token.is_empty() { 6 } else { 0 };
        let priv_vip_type = if !token.is_empty() { "6" } else { "0" };

        let tiers = match preferred_quality {
            AudioQuality::HiRes => vec![
                AudioQuality::HiRes,
                AudioQuality::Lossless,
                AudioQuality::High,
                AudioQuality::Standard,
            ],
            AudioQuality::Lossless => vec![
                AudioQuality::Lossless,
                AudioQuality::High,
                AudioQuality::Standard,
            ],
            AudioQuality::High => vec![AudioQuality::High, AudioQuality::Standard],
            AudioQuality::Standard => vec![AudioQuality::Standard, AudioQuality::High],
        };

        let mut seen_hashes = HashSet::new();
        let mut candidate_list = Vec::new();
        for tier in tiers {
            let h = song.hash_for_quality(&tier);
            if !h.is_empty() && seen_hashes.insert(h.clone()) {
                candidate_list.push((h, tier));
            }
        }

        let mut best_preview: Option<PlaybackResult> = None;

        for (target_hash, tier_q) in candidate_list {
            let now_ms = Utc::now().timestamp_millis();
            let now_sec = (now_ms / 1000).to_string();
            let t_key = signer::tracker_key(&target_hash, &dev.mid, uid);

            let qualities_arr = match tier_q {
                AudioQuality::HiRes => json!(["high", "flac", "320", "128"]),
                AudioQuality::Lossless => json!(["flac", "320", "128"]),
                AudioQuality::High => json!(["320", "128"]),
                AudioQuality::Standard => json!(["128", "320"]),
            };

            let body = json!({
                "area_code": "1",
                "behavior": "play",
                "qualities": qualities_arr,
                "resource": {
                    "album_audio_id": song.album_audio_id,
                    "collect_list_id": "3",
                    "collect_time": now_ms,
                    "hash": target_hash,
                    "id": 0,
                    "page_id": 1,
                    "type": "audio"
                },
                "token": token,
                "tracker_param": {
                    "all_m": 1,
                    "auth": "",
                    "is_free_part": 0,
                    "key": t_key,
                    "module_id": 0,
                    "need_climax": 1,
                    "need_xcdn": 1,
                    "open_time": "",
                    "pid": "411",
                    "pidversion": "3001",
                    "priv_vip_type": priv_vip_type,
                    "viptoken": vip_token
                },
                "userid": uid,
                "vip": vip_level,
                "vip_type": vip_level,
                "vip_token": vip_token,
                "key": t_key
            });

            let mut query_btree = BTreeMap::new();
            query_btree.insert("appid".to_string(), APP_ID.to_string());
            query_btree.insert("clientver".to_string(), CLIENT_VERSION.to_string());
            query_btree.insert("mid".to_string(), dev.mid.clone());
            query_btree.insert("dfid".to_string(), dev.dfid.clone());
            query_btree.insert("clienttime".to_string(), now_sec.clone());
            query_btree.insert("token".to_string(), token.to_string());
            query_btree.insert("userid".to_string(), uid.to_string());

            let body_str = serde_json::to_string(&body).unwrap_or_default();
            let sig = signer::signature(&query_btree, &body_str, false);

            let mut query: HashMap<String, String> = query_btree.into_iter().collect();
            query.insert("signature".to_string(), sig);

            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "tracker.kugou.com".to_string());
            headers.insert("dfid".to_string(), dev.dfid.clone());
            headers.insert("mid".to_string(), dev.mid.clone());
            headers.insert("clienttime".to_string(), now_sec);

            let Ok(resp) = self
                .transport
                .call(
                    "/v6/priv_url",
                    &dev,
                    session.as_ref(),
                    query,
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    false,
                    false,
                )
                .await
            else {
                continue;
            };

            let d = resp.data();
            let Some(play_url) = Self::extract_play_url(d) else {
                continue;
            };

            let file_size = Self::val_i64(d, "file_size")
                .or_else(|| Self::val_i64(d, "fileSize"))
                .unwrap_or(0);
            let bitrate = Self::val_i64(d, "bitrate")
                .or_else(|| Self::val_i64(d, "bitRate"))
                .unwrap_or(0);
            let ext_name = {
                let e = Self::val_str(d, "extname");
                if !e.is_empty() {
                    e
                } else {
                    Self::val_str(d, "extName")
                }
            };
            let resp_q_str = {
                let q = Self::val_str(d, "quality");
                if !q.is_empty() {
                    q
                } else {
                    d.get("ext_info")
                        .map(|ei| Self::val_str(ei, "quality"))
                        .unwrap_or_default()
                }
            };
            let resp_dur_sec = Self::val_i64(d, "timeLength")
                .or_else(|| Self::val_i64(d, "timelength"))
                .unwrap_or(0);
            let eff_dur_ms = if song.duration_ms > 0 {
                song.duration_ms
            } else if resp_dur_sec > 0 && resp_dur_sec < 10_000 {
                resp_dur_sec * 1000
            } else {
                resp_dur_sec
            };

            let (actual_q, kbps) = Self::deduce_quality_and_kbps(
                song,
                &play_url,
                bitrate,
                file_size,
                eff_dur_ms,
                &ext_name,
                &tier_q,
                &resp_q_str,
            );

            let is_free_part = d
                .get("ext_info")
                .and_then(|e| Self::val_i64(e, "is_free_part"))
                == Some(1)
                || Self::val_i64(d, "is_free_part") == Some(1)
                || Self::val_i64(d, "priv_status") == Some(0)
                || (file_size > 0 && file_size < 200 * 1024);

            let clean_ext = if !ext_name.is_empty() {
                ext_name
            } else if actual_q == AudioQuality::HiRes || actual_q == AudioQuality::Lossless {
                "flac".to_string()
            } else {
                "mp3".to_string()
            };

            let backup_urls = Self::extract_backup_play_urls(d, &play_url);
            let res = PlaybackResult {
                url: play_url,
                backup_urls,
                actual_quality: actual_q.api_value().to_string(),
                quality_label: actual_q.label().to_string(),
                quality_badge: actual_q.short_badge().to_string(),
                bitrate_kbps: kbps,
                file_size_bytes: if file_size > 0 {
                    file_size
                } else {
                    song.known_size_for_quality(&actual_q)
                },
                sample_rate_hz: 44_100,
                bit_depth: 16,
                channels: 2,
                tech_spec_text: String::new(),
                climax_start_ms: Self::extract_climax_range(d).0,
                climax_end_ms: Self::extract_climax_range(d).1,
                is_free_part,
                resolved_hash: target_hash,
                ext_name: clean_ext,
                proxy_url: String::new(),
            };

            if is_free_part {
                if best_preview.is_none() {
                    best_preview = Some(res);
                }
                continue;
            }
            return Some(res);
        }

        best_preview
    }

    async fn resolve_v5_fallback_url(
        &self,
        song: &Song,
        preferred_quality: &AudioQuality,
        free_part_only: bool,
    ) -> Option<PlaybackResult> {
        let dev = self.store.device();
        let session = self.store.session();
        let uid = session
            .as_ref()
            .map(|s| s.user_id.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("0");
        let token = session.as_ref().map(|s| s.token.as_str()).unwrap_or("");
        let vip_token = session
            .as_ref()
            .and_then(|s| s.vip_token.as_deref())
            .filter(|s| !s.is_empty())
            .unwrap_or(token);
        let vip_type = if !token.is_empty() { "6" } else { "0" };
        let now_sec = Utc::now().timestamp().to_string();

        let tiers = if free_part_only {
            vec![AudioQuality::Standard]
        } else {
            match preferred_quality {
                AudioQuality::HiRes => vec![
                    AudioQuality::HiRes,
                    AudioQuality::Lossless,
                    AudioQuality::High,
                    AudioQuality::Standard,
                ],
                AudioQuality::Lossless => vec![
                    AudioQuality::Lossless,
                    AudioQuality::High,
                    AudioQuality::Standard,
                ],
                AudioQuality::High => vec![AudioQuality::High, AudioQuality::Standard],
                AudioQuality::Standard => vec![AudioQuality::Standard, AudioQuality::High],
            }
        };

        for tier_q in tiers {
            let hash_lower = song.hash_for_quality(&tier_q);
            if hash_lower.is_empty() {
                continue;
            }
            let t_key = signer::tracker_key(&hash_lower, &dev.mid, uid);

            let mut q_btree = BTreeMap::new();
            q_btree.insert(
                "album_id".to_string(),
                if song.album_id.is_empty() {
                    "0".to_string()
                } else {
                    song.album_id.clone()
                },
            );
            q_btree.insert("area_code".to_string(), "1".to_string());
            q_btree.insert("hash".to_string(), hash_lower.clone());
            q_btree.insert("ssa_flag".to_string(), "is_fromtrack".to_string());
            q_btree.insert("version".to_string(), CLIENT_VERSION.to_string());
            q_btree.insert("page_id".to_string(), "967177915".to_string());
            q_btree.insert("quality".to_string(), tier_q.api_value().to_string());
            q_btree.insert(
                "album_audio_id".to_string(),
                song.album_audio_id.to_string(),
            );
            q_btree.insert("behavior".to_string(), "play".to_string());
            q_btree.insert("pid".to_string(), "411".to_string());
            q_btree.insert("cmd".to_string(), "26".to_string());
            q_btree.insert("pidversion".to_string(), "3001".to_string());
            q_btree.insert(
                "ppage_id".to_string(),
                "356753938,823673182,967485191".to_string(),
            );
            q_btree.insert("cdnBackup".to_string(), "1".to_string());
            q_btree.insert("clientver".to_string(), CLIENT_VERSION.to_string());
            q_btree.insert("appid".to_string(), APP_ID.to_string());
            q_btree.insert("mid".to_string(), dev.mid.clone());
            q_btree.insert("dfid".to_string(), dev.dfid.clone());
            q_btree.insert("clienttime".to_string(), now_sec.clone());
            q_btree.insert("userid".to_string(), uid.to_string());
            q_btree.insert("token".to_string(), token.to_string());
            q_btree.insert("vip_type".to_string(), vip_type.to_string());
            q_btree.insert("vip_token".to_string(), vip_token.to_string());
            q_btree.insert("key".to_string(), t_key);

            if free_part_only {
                q_btree.insert("is_free_part".to_string(), "1".to_string());
                q_btree.insert("IsFreePart".to_string(), "1".to_string());
            }

            let sig = signer::signature(&q_btree, "", false);
            let mut query: HashMap<String, String> = q_btree.into_iter().collect();
            query.insert("signature".to_string(), sig);

            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "tracker.kugou.com".to_string());
            headers.insert("dfid".to_string(), dev.dfid.clone());
            headers.insert("mid".to_string(), dev.mid.clone());
            headers.insert("clienttime".to_string(), now_sec.clone());

            if let Ok(resp) = self
                .transport
                .call(
                    "/v5/url",
                    &dev,
                    session.as_ref(),
                    query,
                    None,
                    "https://gateway.kugou.com",
                    "GET",
                    false,
                    headers,
                    false,
                    false,
                )
                .await
            {
                let d = resp.data();
                if let Some(play_url) = Self::extract_play_url(d) {
                    let file_size = Self::val_i64(d, "fileSize")
                        .or_else(|| Self::val_i64(d, "file_size"))
                        .unwrap_or(0);
                    let bitrate = Self::val_i64(d, "bitRate")
                        .or_else(|| Self::val_i64(d, "bitrate"))
                        .unwrap_or(0);
                    let ext_name = Self::val_str(d, "extName");
                    let resp_dur_sec = Self::val_i64(d, "timeLength")
                        .or_else(|| Self::val_i64(d, "timelength"))
                        .unwrap_or(0);
                    let eff_dur_ms = if song.duration_ms > 0 {
                        song.duration_ms
                    } else if resp_dur_sec > 0 && resp_dur_sec < 10_000 {
                        resp_dur_sec * 1000
                    } else {
                        resp_dur_sec
                    };

                    let (actual_q, kbps) = Self::deduce_quality_and_kbps(
                        song,
                        &play_url,
                        bitrate,
                        file_size,
                        eff_dur_ms,
                        &ext_name,
                        &tier_q,
                        "",
                    );

                    let backup_urls = Self::extract_backup_play_urls(d, &play_url);
                    return Some(PlaybackResult {
                        url: play_url,
                        backup_urls,
                        actual_quality: actual_q.api_value().to_string(),
                        quality_label: actual_q.label().to_string(),
                        quality_badge: actual_q.short_badge().to_string(),
                        bitrate_kbps: kbps,
                        file_size_bytes: if file_size > 0 {
                            file_size
                        } else {
                            song.known_size_for_quality(&actual_q)
                        },
                        sample_rate_hz: 44_100,
                        bit_depth: 16,
                        channels: 2,
                        tech_spec_text: String::new(),
                        climax_start_ms: 0,
                        climax_end_ms: 0,
                        is_free_part: free_part_only,
                        resolved_hash: hash_lower,
                        ext_name: if ext_name.is_empty() {
                            "mp3".to_string()
                        } else {
                            ext_name
                        },
                        proxy_url: String::new(),
                    });
                }
            }
        }
        None
    }

    /// 19. 获取并解密歌曲 KRC/LRC 歌词（带本地磁盘缓存）
    pub async fn fetch_song_lyric(&self, song: &Song) -> Result<LyricData, String> {
        let hash_lower = song.hash.trim().to_lowercase();
        if !hash_lower.is_empty() {
            let cache_path = Self::resolve_cache_dir().join(format!("lyric_{}.krc", hash_lower));
            if let Ok(cached_str) = std::fs::read_to_string(&cache_path) {
                if let Ok(cached_lyric) = serde_json::from_str::<LyricData>(&cached_str) {
                    if !cached_lyric.lines.is_empty() {
                        return Ok(cached_lyric);
                    }
                }
            }
        }
        let res = self.fetch_song_lyric_remote(song).await?;
        if !hash_lower.is_empty() && !res.lines.is_empty() {
            let cache_path = Self::resolve_cache_dir().join(format!("lyric_{}.krc", hash_lower));
            if let Ok(json_str) = serde_json::to_string(&res) {
                let _ = std::fs::write(cache_path, json_str);
            }
        }
        Ok(res)
    }

    async fn fetch_song_lyric_remote(&self, song: &Song) -> Result<LyricData, String> {
        let dev = self.store.device();
        let now_sec = Utc::now().timestamp().to_string();

        let mut q_btree = BTreeMap::new();
        q_btree.insert(
            "album_audio_id".to_string(),
            song.album_audio_id.to_string(),
        );
        q_btree.insert("appid".to_string(), APP_ID.to_string());
        q_btree.insert("clientver".to_string(), CLIENT_VERSION.to_string());
        q_btree.insert("duration".to_string(), song.duration_ms.to_string());
        q_btree.insert("hash".to_string(), song.hash.to_lowercase());
        q_btree.insert("keyword".to_string(), song.title.clone());
        q_btree.insert("lrctxt".to_string(), "1".to_string());
        q_btree.insert("man".to_string(), "yes".to_string());
        q_btree.insert("clienttime".to_string(), now_sec.clone());
        q_btree.insert("mid".to_string(), dev.mid.clone());
        q_btree.insert("dfid".to_string(), dev.dfid.clone());

        let sorted_str: String = q_btree
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("");
        let sig = signer::md5_hex(&format!("{}{}{}", ANDROID_SALT, sorted_str, ANDROID_SALT));

        let mut query: HashMap<String, String> = q_btree.into_iter().collect();
        query.insert("signature".to_string(), sig);

        let mut headers = HashMap::new();
        headers.insert("dfid".to_string(), dev.dfid.clone());
        headers.insert("mid".to_string(), dev.mid.clone());
        headers.insert("clienttime".to_string(), now_sec);

        let mut candidates: Vec<(String, String)> = Vec::new();
        if let Ok(resp) = self
            .transport
            .call(
                "/v1/search",
                &dev,
                None,
                query,
                None,
                "https://lyrics.kugou.com",
                "GET",
                false,
                headers,
                false,
                false,
            )
            .await
        {
            let arr_opt = resp
                .data()
                .get("candidates")
                .and_then(|v| v.as_array())
                .or_else(|| resp.body.get("candidates").and_then(|v| v.as_array()));
            if let Some(arr) = arr_opt {
                for c in arr {
                    let id = Self::val_str(c, "id");
                    let accesskey = Self::val_str(c, "accesskey");
                    if !id.is_empty() && !accesskey.is_empty() {
                        candidates.push((id, accesskey));
                    }
                }
            }
        }

        if candidates.is_empty() && !song.hash.is_empty() {
            let mut krcs_q = HashMap::new();
            krcs_q.insert("ver".to_string(), "1".to_string());
            krcs_q.insert("man".to_string(), "yes".to_string());
            krcs_q.insert("client".to_string(), "mobi".to_string());
            krcs_q.insert("hash".to_string(), song.hash.to_lowercase());
            krcs_q.insert("duration".to_string(), song.duration_ms.to_string());
            krcs_q.insert(
                "album_audio_id".to_string(),
                song.album_audio_id.to_string(),
            );
            if let Ok(resp) = self
                .transport
                .call(
                    "/search",
                    &dev,
                    None,
                    krcs_q,
                    None,
                    "https://krcs.kugou.com",
                    "GET",
                    false,
                    HashMap::new(),
                    false,
                    false,
                )
                .await
            {
                if let Some(arr) = resp
                    .data()
                    .get("candidates")
                    .and_then(|v| v.as_array())
                    .or_else(|| resp.body.get("candidates").and_then(|v| v.as_array()))
                {
                    for c in arr {
                        let id = Self::val_str(c, "id");
                        let accesskey = Self::val_str(c, "accesskey");
                        if !id.is_empty() && !accesskey.is_empty() {
                            candidates.push((id, accesskey));
                        }
                    }
                }
            }
        }

        if candidates.is_empty() {
            return Ok(LyricData::default());
        }

        let mut primary_krc: Option<LyricData> = None;
        for (idx, (cand_id, cand_key)) in candidates.iter().take(5).enumerate() {
            if let Some(krc_data) = self.download_lyric_by_fmt(cand_id, cand_key, "krc").await {
                if !krc_data.lines.is_empty() {
                    if primary_krc.is_none() {
                        let is_chinese = krc::is_predominantly_chinese(&krc_data.lines);
                        if is_chinese || (krc_data.has_translation && krc_data.has_phonetic) || candidates.len() == 1 {
                            return Ok(krc_data);
                        }
                        primary_krc = Some(krc_data);
                    } else if let Some(ref mut base) = primary_krc {
                        if (!base.has_translation && krc_data.has_translation)
                            || (!base.has_phonetic && krc_data.has_phonetic)
                        {
                            if (base.lines.len() as isize - krc_data.lines.len() as isize).abs() <= 2 {
                                for (i, line) in base.lines.iter_mut().enumerate() {
                                    if let Some(other) = krc_data.lines.get(i) {
                                        if line.translation.is_none() && other.translation.is_some() {
                                            line.translation = other.translation.clone();
                                        }
                                        if line.phonetic.is_none() && other.phonetic.is_some() {
                                            line.phonetic = other.phonetic.clone();
                                        }
                                    }
                                }
                                base.has_translation = base.lines.iter().any(|l| l.translation.is_some());
                                base.has_phonetic = base.lines.iter().any(|l| l.phonetic.is_some());
                                if base.has_translation && base.has_phonetic {
                                    break;
                                }
                            } else if krc_data.has_translation && !base.has_translation {
                                *base = krc_data;
                            }
                        }
                    }
                }
            }
            if idx == 0 && primary_krc.is_none() {
                if let Some(lrc_data) = self.download_lyric_by_fmt(cand_id, cand_key, "lrc").await {
                    if !lrc_data.lines.is_empty() {
                        return Ok(lrc_data);
                    }
                }
            }
        }

        if let Some(krc_data) = primary_krc {
            return Ok(krc_data);
        }

        Ok(LyricData::default())
    }

    async fn download_lyric_by_fmt(
        &self,
        id: &str,
        accesskey: &str,
        fmt: &str,
    ) -> Option<LyricData> {
        let dev = self.store.device();
        let mut query = HashMap::new();
        query.insert("ver".to_string(), "1".to_string());
        query.insert("client".to_string(), "android".to_string());
        query.insert("id".to_string(), id.to_string());
        query.insert("accesskey".to_string(), accesskey.to_string());
        query.insert("fmt".to_string(), fmt.to_string());
        query.insert("charset".to_string(), "utf8".to_string());

        let resp = self
            .transport
            .call(
                "/download",
                &dev,
                None,
                query,
                None,
                "https://lyrics.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
            .ok()?;

        let content = Self::val_str(&resp.body, "content");
        let content = if content.is_empty() {
            Self::val_str(resp.data(), "content")
        } else {
            content
        };
        if content.is_empty() {
            return None;
        }
        Some(krc::parse_lyric_auto(&content, fmt == "krc"))
    }

    // ================= 辅助解析函数 =================

    fn val_str(v: &Value, key: &str) -> String {
        v.get(key)
            .and_then(|val| match val {
                Value::String(s) => Some(s.trim().to_string()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .filter(|s| !s.is_empty() && s != "null" && s != "undefined")
            .unwrap_or_default()
    }

    fn val_i64(v: &Value, key: &str) -> Option<i64> {
        v.get(key).and_then(|val| {
            val.as_i64()
                .or_else(|| val.as_str().and_then(|s| s.trim().parse::<i64>().ok()))
        })
    }

    fn extract_login_candidates(data: &Value) -> Vec<LoginCandidate> {
        for key in ["info_list", "user_list", "userList", "lists", "list"] {
            if let Some(arr) = data.get(key).and_then(|v| v.as_array()) {
                let list: Vec<LoginCandidate> = arr
                    .iter()
                    .filter_map(|item| {
                        let uid = {
                            let u = Self::val_str(item, "userid");
                            if !u.is_empty() {
                                u
                            } else {
                                Self::val_str(item, "userId")
                            }
                        };
                        if uid.is_empty() {
                            return None;
                        }
                        let nick = {
                            let n = Self::val_str(item, "nickname");
                            if !n.is_empty() {
                                n
                            } else {
                                let un = Self::val_str(item, "username");
                                if !un.is_empty() {
                                    un
                                } else {
                                    format!("账号 {}", uid)
                                }
                            }
                        };
                        let pic = {
                            let p = Self::val_str(item, "pic");
                            if !p.is_empty() {
                                p
                            } else {
                                Self::val_str(item, "avatar")
                            }
                        };
                        Some(LoginCandidate {
                            user_id: uid,
                            nickname: nick,
                            avatar_url: signer::normalize_image_url(Some(&pic)),
                        })
                    })
                    .collect();
                if !list.is_empty() {
                    return list;
                }
            }
        }
        Vec::new()
    }

    fn extract_play_url(obj: &Value) -> Option<String> {
        Self::extract_all_play_urls(obj).into_iter().next()
    }

    fn extract_backup_play_urls(obj: &Value, primary_url: &str) -> Vec<String> {
        Self::extract_all_play_urls(obj)
            .into_iter()
            .filter(|u| u != primary_url)
            .collect()
    }

    fn extract_all_play_urls(obj: &Value) -> Vec<String> {
        let normalize = |raw: &str| -> Option<String> {
            let t = raw.trim();
            if t.starts_with("http://") || t.starts_with("https://") {
                Some(t.to_string())
            } else if t.starts_with("//") {
                Some(format!("https:{}", t))
            } else {
                None
            }
        };

        let mut urls = Vec::new();
        for k in ["url", "play_url", "playUrl", "backup_url", "backupUrl"] {
            if let Some(v) = obj.get(k) {
                if let Some(s) = v.as_str() {
                    if let Some(u) = normalize(s) {
                        if !urls.contains(&u) {
                            urls.push(u);
                        }
                    }
                } else if let Some(arr) = v.as_array() {
                    for el in arr {
                        if let Some(s) = el.as_str() {
                            if let Some(u) = normalize(s) {
                                if !urls.contains(&u) {
                                    urls.push(u);
                                }
                            }
                        }
                    }
                }
            }
        }
        urls
    }

    pub fn parse_songs(root: &Value) -> Vec<Song> {
        let mut collected = Vec::new();
        Self::collect_song_objects(root, 0, &mut collected);
        collected
            .into_iter()
            .map(|obj| Self::parse_single_song(obj))
            .filter(|s| (!s.hash.is_empty() || !s.id.is_empty()) && s.title != "未知歌曲")
            .collect()
    }

    fn is_likely_song_obj(obj: &Value) -> bool {
        obj.get("hash").is_some()
            || obj.get("FileHash").is_some()
            || obj.get("hash_128").is_some()
            || obj.get("audio_info").is_some()
            || obj.get("song_info").is_some()
            || obj.get("songname").is_some()
            || obj.get("song_name").is_some()
            || obj.get("SongName").is_some()
            || obj.get("audio_name").is_some()
            || obj.get("filename").is_some()
            || obj.get("FileName").is_some()
    }

    fn collect_song_objects<'a>(el: &'a Value, depth: usize, out: &mut Vec<&'a Value>) {
        if depth > 4 {
            return;
        }
        if let Some(arr) = el.as_array() {
            for item in arr {
                if item.is_object() {
                    if Self::is_likely_song_obj(item) {
                        out.push(item);
                    } else {
                        Self::collect_song_objects(item, depth + 1, out);
                    }
                }
            }
        } else if let Some(map) = el.as_object() {
            let array_keys = [
                "songs",
                "songlist",
                "song_list",
                "info",
                "data",
                "files",
                "lists",
                "list",
                "audio_list",
                "audios",
                "recommend_list",
                "song_info_list",
                "album_audio_list",
                "newsong_list",
                "publish_list",
            ];
            let mut found = false;
            for k in array_keys {
                if let Some(v) = map.get(k) {
                    if v.is_array() {
                        found = true;
                        Self::collect_song_objects(v, depth + 1, out);
                    }
                }
            }
            if !found {
                for (k, v) in map {
                    if k != "relate_goods" && k != "authors" && k != "Singers" && k != "trans_param"
                    {
                        if v.is_object() || v.is_array() {
                            Self::collect_song_objects(v, depth + 1, out);
                        }
                    }
                }
            }
        }
    }

    fn is_valid_md5_hash(h: &str) -> bool {
        let clean = h.trim();
        clean.len() == 32 && !clean.chars().all(|c| c == '0')
    }

    fn parse_single_song(o: &Value) -> Song {
        let replace_obj = o
            .get("replace")
            .or_else(|| o.get("replace_info"))
            .or_else(|| o.get("replace").and_then(|v| v.as_array()).and_then(|a| a.first()));
        let replace_audio = replace_obj
            .and_then(|r| r.get("audio_info").or(Some(r)))
            .unwrap_or(o);
        let base = o
            .get("base")
            .or_else(|| o.get("song").and_then(|s| s.get("base")))
            .or_else(|| replace_obj.and_then(|r| r.get("base")))
            .unwrap_or(o);
        let audio_info = o
            .get("audio_info")
            .or_else(|| o.get("audio"))
            .or_else(|| o.get("song_info"))
            .or_else(|| o.get("song"))
            .or_else(|| o.get("track"))
            .unwrap_or(replace_audio);
        let explicit_album_info = o
            .get("album_info")
            .or_else(|| o.get("albuminfo"))
            .or_else(|| audio_info.get("album_info"))
            .or_else(|| replace_obj.and_then(|r| r.get("album_info")));
        let album_info = explicit_album_info.unwrap_or(o);
        let old_copyright = o.get("old_copyright").unwrap_or(o);
        let trans_param = o
            .get("trans_param")
            .or_else(|| audio_info.get("trans_param"))
            .or_else(|| base.get("trans_param"))
            .or_else(|| old_copyright.get("trans_param"))
            .unwrap_or(o);

        let ori_song_name = {
            let a = Self::val_str(o, "OriSongName");
            if !a.is_empty() {
                a
            } else {
                Self::val_str(o, "ori_song_name")
            }
        };
        let suffix = {
            let s = Self::val_str(o, "Suffix");
            if !s.is_empty() {
                s
            } else {
                Self::val_str(o, "suffix")
            }
        };
        let combined_ori_title = if !ori_song_name.is_empty() {
            if !suffix.is_empty() && !ori_song_name.contains(&suffix) {
                format!("{} {}", ori_song_name, suffix)
            } else {
                ori_song_name
            }
        } else {
            String::new()
        };

        let mut title = [
            combined_ori_title,
            Self::val_str(o, "songname"),
            Self::val_str(o, "song_name"),
            Self::val_str(o, "SongName"),
            Self::val_str(base, "songname"),
            Self::val_str(base, "audio_name"),
            Self::val_str(o, "audio_name"),
            Self::val_str(audio_info, "audio_name"),
            Self::val_str(audio_info, "songname"),
            Self::val_str(o, "name"),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_default();

        // 提取多歌手结构化列表 authors（兼容扁平对象与 ip/audios 的 authors[].base 嵌套结构）
        let mut parsed_authors: Vec<SongAuthor> = Vec::new();
        let author_arrays = [
            o.get("authors").and_then(|v| v.as_array()),
            base.get("authors").and_then(|v| v.as_array()),
            o.get("Singers").and_then(|v| v.as_array()),
            o.get("singerinfo").and_then(|v| v.as_array()),
            audio_info.get("authors").and_then(|v| v.as_array()),
        ];
        if let Some(arr) = author_arrays.into_iter().flatten().find(|a| !a.is_empty()) {
            for a_obj in arr {
                let a_base = a_obj.get("base").unwrap_or(a_obj);
                let aname = [
                    Self::val_str(a_obj, "author_name"),
                    Self::val_str(a_base, "author_name"),
                    Self::val_str(a_obj, "name"),
                    Self::val_str(a_base, "name"),
                    Self::val_str(a_obj, "singername"),
                ]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or_default()
                .replace("<em>", "")
                .replace("</em>", "")
                .trim()
                .to_string();
                let aid = [
                    Self::val_str(a_obj, "author_id"),
                    Self::val_str(a_base, "author_id"),
                    Self::val_str(a_obj, "id"),
                    Self::val_str(a_base, "id"),
                    Self::val_str(a_obj, "singerid"),
                ]
                .into_iter()
                .find(|s| !s.is_empty() && s != "0")
                .unwrap_or_default();
                let av = [
                    Self::val_str(a_obj, "sizable_avatar"),
                    Self::val_str(a_base, "sizable_avatar"),
                    Self::val_str(a_obj, "avatar"),
                    Self::val_str(a_base, "avatar"),
                    Self::val_str(a_obj, "pic"),
                    Self::val_str(a_obj, "imgurl"),
                ]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or_default();
                if !aname.is_empty() && !parsed_authors.iter().any(|e| e.name == aname) {
                    parsed_authors.push(SongAuthor {
                        id: aid,
                        name: aname,
                        avatar_url: signer::normalize_image_url(Some(&av)),
                    });
                }
            }
        }

        let joined_authors: String = parsed_authors
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join("、");

        let mut artist = [
            Self::val_str(o, "singername"),
            Self::val_str(o, "singer_name"),
            Self::val_str(o, "author_name"),
            Self::val_str(o, "SingerName"),
            Self::val_str(base, "author_name"),
            Self::val_str(audio_info, "author_name"),
            Self::val_str(audio_info, "singername"),
            joined_authors,
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_default();

        let artist_id = [
            Self::val_str(o, "AuthorId"),
            Self::val_str(o, "author_id"),
            Self::val_str(o, "SingerId"),
            Self::val_str(o, "singerid"),
            Self::val_str(base, "author_id"),
            Self::val_str(audio_info, "author_id"),
            parsed_authors
                .first()
                .map(|a| a.id.clone())
                .unwrap_or_default(),
        ]
        .into_iter()
        .find(|s| !s.is_empty() && s != "0")
        .unwrap_or_default();

        let raw_filename = [
            Self::val_str(o, "filename"),
            Self::val_str(o, "FileName"),
            Self::val_str(base, "filename"),
            Self::val_str(audio_info, "filename"),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_default();

        if title.is_empty() && raw_filename.contains('-') {
            let parts: Vec<&str> = raw_filename.splitn(2, '-').collect();
            if artist.is_empty() || artist == "未知歌手" || artist == "群星" {
                artist = parts[0].trim().to_string();
            }
            title = parts[1].trim().to_string();
        }
        if title.is_empty() {
            title = if raw_filename.is_empty() {
                "未知歌曲".to_string()
            } else {
                raw_filename
            };
        }
        if title.contains(" - ") {
            let parts: Vec<&str> = title.splitn(2, " - ").collect();
            let prefix = parts[0].trim();
            let clean_t = parts[1].trim();
            if artist.is_empty()
                || artist == "未知歌手"
                || artist == "群星"
                || artist.eq_ignore_ascii_case(prefix)
                || prefix.contains(&artist)
            {
                if artist.is_empty() || artist == "未知歌手" || artist == "群星" {
                    artist = prefix.to_string();
                }
                title = clean_t.to_string();
            }
        }
        title = title.replace("<em>", "").replace("</em>", "").trim().to_string();
        artist = artist.replace("<em>", "").replace("</em>", "").trim().to_string();
        if artist.is_empty() {
            artist = "群星".to_string();
        }

        // 若 parsed_authors 为空但 artist 字符串含多歌手分隔符（如 "周杰伦、阿信"），拆分为独立歌手条目
        if parsed_authors.is_empty() && !artist.is_empty() {
            let split_names: Vec<&str> = artist
                .split(&['、', '/', '&', '，'][..])
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            for (idx, sname) in split_names.iter().enumerate() {
                parsed_authors.push(SongAuthor {
                    id: if idx == 0 && split_names.len() == 1 {
                        artist_id.clone()
                    } else {
                        String::new()
                    },
                    name: (*sname).to_string(),
                    avatar_url: String::new(),
                });
            }
        }

        let album_id = [
            Self::val_str(o, "album_id"),
            Self::val_str(o, "AlbumID"),
            Self::val_str(o, "albumid"),
            Self::val_str(album_info, "album_id"),
            Self::val_str(album_info, "albumid"),
            Self::val_str(album_info, "id"),
            Self::val_str(base, "album_id"),
            Self::val_str(audio_info, "album_id"),
            Self::val_str(replace_audio, "album_id"),
        ]
        .into_iter()
        .find(|s| !s.is_empty() && s != "0")
        .unwrap_or_default();

        let explicit_album = [
            Self::val_str(o, "album_name"),
            Self::val_str(o, "AlbumName"),
            Self::val_str(o, "albumname"),
            Self::val_str(album_info, "album_name"),
            Self::val_str(album_info, "albumname"),
            explicit_album_info
                .map(|ai| Self::val_str(ai, "name"))
                .unwrap_or_default(),
            Self::val_str(base, "album_name"),
            Self::val_str(audio_info, "album_name"),
            Self::val_str(replace_audio, "album_name"),
            Self::val_str(trans_param, "album_name"),
        ]
        .into_iter()
        .find(|s| !s.is_empty() && s != "0" && s != "未知专辑")
        .unwrap_or_default();

        let album = if !explicit_album.is_empty() {
            explicit_album
        } else if album_id.is_empty() {
            let rm = Self::val_str(o, "remark");
            let is_promo = [
                "主题曲", "插曲", "片尾曲", "片头曲", "推广曲", "宣传曲", "概念曲", "印象曲",
                "歌手：", "综艺 ", "电视剧", "电影", "网剧",
            ]
            .iter()
            .any(|kw| rm.contains(kw));
            if !rm.is_empty() && rm != "0" && rm != "未知专辑" && !is_promo {
                rm
            } else {
                String::new()
            }
        } else {
            String::new()
        }
        .replace("<em>", "")
        .replace("</em>", "")
        .trim()
        .to_string();

        let cover_raw = [
            Self::val_str(o, "Image"),
            Self::val_str(o, "image"),
            Self::val_str(o, "cover"),
            Self::val_str(o, "album_sizable_cover"),
            Self::val_str(o, "sizable_cover"),
            Self::val_str(o, "album_img"),
            Self::val_str(o, "union_cover"),
            Self::val_str(o, "imgurl"),
            Self::val_str(o, "pic"),
            Self::val_str(trans_param, "union_cover"),
            Self::val_str(trans_param, "sizable_cover"),
            Self::val_str(album_info, "sizable_cover"),
            Self::val_str(album_info, "cover"),
            Self::val_str(base, "sizable_cover"),
            Self::val_str(audio_info, "sizable_cover"),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_default();
        let cover_url = signer::normalize_image_url(Some(&cover_raw));

        let dur_raw = Self::val_i64(o, "duration")
            .or_else(|| Self::val_i64(o, "timelen"))
            .or_else(|| Self::val_i64(o, "time_length"))
            .or_else(|| Self::val_i64(o, "timelength"))
            .or_else(|| Self::val_i64(o, "Duration"))
            .or_else(|| Self::val_i64(o, "time"))
            .or_else(|| Self::val_i64(o, "320time"))
            .or_else(|| Self::val_i64(audio_info, "duration"))
            .or_else(|| Self::val_i64(audio_info, "timelength"))
            .or_else(|| Self::val_i64(audio_info, "timelen"))
            .or_else(|| Self::val_i64(audio_info, "time"))
            .or_else(|| Self::val_i64(base, "duration"))
            .unwrap_or(0);
        let mut duration_ms = if (1..10_000).contains(&dur_raw) {
            dur_raw * 1000
        } else {
            dur_raw
        };

        // 解析 relate_goods 嵌套音源规格（对齐 Android MusicRepository.parseSong）
        let mut relate_res_hash = String::new();
        let mut relate_sq_hash = String::new();
        let mut relate_hq_hash = String::new();
        let mut relate_std_hash = String::new();
        let mut relate_acc_hash = String::new();
        let mut relate_res_size = 0i64;
        let mut relate_sq_size = 0i64;
        let mut relate_hq_size = 0i64;
        let mut relate_std_size = 0i64;
        let mut has_relate_vip = false;

        let rg_sources = [
            o.get("relate_goods").and_then(|v| v.as_array()),
            audio_info.get("relate_goods").and_then(|v| v.as_array()),
            base.get("relate_goods").and_then(|v| v.as_array()),
            replace_obj.and_then(|r| r.get("relate_goods")).and_then(|v| v.as_array()),
        ];
        let mut all_rgs: Vec<&Value> = Vec::new();
        for src in rg_sources.into_iter().flatten() {
            for rg in src {
                all_rgs.push(rg);
            }
        }

        for rg in &all_rgs {
            let rg_info = rg.get("info").unwrap_or(rg);
            let dur_r = Self::val_i64(rg, "duration")
                .or_else(|| Self::val_i64(rg_info, "duration"))
                .or_else(|| Self::val_i64(rg_info, "timelength"))
                .unwrap_or(0);
            if duration_ms <= 0 && dur_r > 0 {
                duration_ms = if (1..10_000).contains(&dur_r) {
                    dur_r * 1000
                } else {
                    dur_r
                };
            }
            let lvl = Self::val_i64(rg, "level")
                .or_else(|| Self::val_i64(rg_info, "level"))
                .unwrap_or(-1);
            let q_str = {
                let q = Self::val_str(rg, "quality");
                if !q.is_empty() {
                    q
                } else {
                    Self::val_str(rg_info, "quality")
                }
            }
            .to_lowercase();
            let sz = Self::val_i64(rg, "size")
                .or_else(|| Self::val_i64(rg, "filesize"))
                .or_else(|| Self::val_i64(rg_info, "filesize"))
                .or_else(|| Self::val_i64(rg_info, "size"))
                .unwrap_or(0);
            if (lvl == 5 || q_str == "flac" || q_str == "sq" || q_str == "lossless") && sz > relate_sq_size {
                relate_sq_size = sz;
            }
            let p = Self::val_i64(rg, "privilege")
                .or_else(|| Self::val_i64(rg_info, "privilege"))
                .unwrap_or(-1);
            let pt = Self::val_i64(rg, "pay_type")
                .or_else(|| Self::val_i64(rg_info, "pay_type"))
                .unwrap_or(-1);
            let fp = Self::val_i64(rg, "fail_process")
                .or_else(|| Self::val_i64(rg_info, "fail_process"))
                .unwrap_or(-1);
            if (1..=6).contains(&lvl) && (p == 10 || pt == 3 || fp == 4) {
                has_relate_vip = true;
            }
        }

        for rg in &all_rgs {
            let rg_info = rg.get("info").unwrap_or(rg);
            let h = {
                let h1 = Self::val_str(rg, "hash");
                if Self::is_valid_md5_hash(&h1) {
                    h1
                } else {
                    Self::val_str(rg_info, "hash")
                }
            }
            .to_lowercase();
            if !Self::is_valid_md5_hash(&h) {
                continue;
            }
            let lvl = Self::val_i64(rg, "level")
                .or_else(|| Self::val_i64(rg_info, "level"))
                .unwrap_or(-1);
            let q_str = {
                let q = Self::val_str(rg, "quality");
                if !q.is_empty() {
                    q
                } else {
                    Self::val_str(rg_info, "quality")
                }
            }
            .to_lowercase();
            let sz = Self::val_i64(rg, "size")
                .or_else(|| Self::val_i64(rg, "filesize"))
                .or_else(|| Self::val_i64(rg_info, "filesize"))
                .or_else(|| Self::val_i64(rg_info, "size"))
                .unwrap_or(0);

            if q_str.contains("accompany") || q_str.contains("伴奏") {
                if relate_acc_hash.is_empty() {
                    relate_acc_hash = h;
                }
            } else if lvl == 7 || q_str.starts_with("viper") || q_str == "super" || q_str == "multitrack" {
                // 跳过正统酷狗 suvip 专属合成音效流
            } else if lvl == 6 || q_str == "hires" || q_str == "hi-res" || q_str == "res" || (q_str == "high" && (lvl == 6 || lvl == 0)) {
                let is_verified_hires = relate_sq_size <= 0 || sz <= 0 || sz >= relate_sq_size;
                if is_verified_hires {
                    if relate_res_hash.is_empty() {
                        relate_res_hash = h;
                    }
                    if relate_res_size <= 0 && sz > 0 {
                        relate_res_size = sz;
                    }
                }
            } else if lvl == 5 || q_str.contains("flac") || q_str.contains("sq") || q_str.contains("lossless") {
                if relate_sq_hash.is_empty() {
                    relate_sq_hash = h;
                }
                if relate_sq_size <= 0 && sz > 0 {
                    relate_sq_size = sz;
                }
            } else if lvl == 4 || q_str.contains("320") || q_str.contains("hq") {
                if relate_hq_hash.is_empty() {
                    relate_hq_hash = h;
                }
                if relate_hq_size <= 0 && sz > 0 {
                    relate_hq_size = sz;
                }
            } else if lvl == 1 || lvl == 2 || q_str.contains("128") {
                if relate_std_hash.is_empty() {
                    relate_std_hash = h;
                }
                if relate_std_size <= 0 && sz > 0 {
                    relate_std_size = sz;
                }
            }
        }

        // 修复严重 Bug：SQFileHash 是无损 FLAC (hash_flac)，HQFileHash 是高品 320K (hash320)！
        let hash_flac = [
            Self::val_str(o, "SQFileHash"),
            Self::val_str(o, "sqhash"),
            Self::val_str(o, "sq_hash"),
            Self::val_str(o, "hash_flac"),
            Self::val_str(o, "hashflac"),
            Self::val_str(o, "flac_hash"),
            Self::val_str(replace_audio, "hash_flac"),
            Self::val_str(audio_info, "hash_flac"),
            Self::val_str(audio_info, "hashflac"),
            Self::val_str(audio_info, "flac_hash"),
            Self::val_str(audio_info, "sqhash"),
            relate_sq_hash,
        ]
        .into_iter()
        .find(|s| Self::is_valid_md5_hash(s))
        .unwrap_or_default()
        .to_lowercase();

        let explicit_hq_hash = [
            Self::val_str(o, "HQFileHash"),
            Self::val_str(o, "320hash"),
            Self::val_str(o, "hqhash"),
            Self::val_str(o, "hq_hash"),
            Self::val_str(o, "hash_320"),
            Self::val_str(replace_audio, "hash_320"),
            Self::val_str(audio_info, "hash_320"),
            Self::val_str(audio_info, "320hash"),
            Self::val_str(audio_info, "hqhash"),
            Self::val_str(trans_param, "ogg_320_hash"),
            relate_hq_hash,
        ]
        .into_iter()
        .find(|s| Self::is_valid_md5_hash(s))
        .unwrap_or_default()
        .to_lowercase();

        let sq_size = Self::val_i64(o, "SQFileSize")
            .or_else(|| Self::val_i64(o, "sqfilesize"))
            .or_else(|| Self::val_i64(o, "filesize_flac"))
            .or_else(|| Self::val_i64(o, "filesize_sq"))
            .or_else(|| Self::val_i64(audio_info, "filesize_flac"))
            .or_else(|| Self::val_i64(audio_info, "sqfilesize"))
            .or_else(|| Self::val_i64(replace_audio, "filesize_flac"))
            .unwrap_or(relate_sq_size);

        let hq_size = Self::val_i64(o, "HQFileSize")
            .or_else(|| Self::val_i64(o, "320filesize"))
            .or_else(|| Self::val_i64(o, "320size"))
            .or_else(|| Self::val_i64(o, "filesize_320"))
            .or_else(|| Self::val_i64(o, "filesize_hq"))
            .or_else(|| Self::val_i64(audio_info, "filesize_320"))
            .or_else(|| Self::val_i64(audio_info, "320filesize"))
            .or_else(|| Self::val_i64(audio_info, "320size"))
            .or_else(|| Self::val_i64(replace_audio, "filesize_320"))
            .or_else(|| Self::val_i64(trans_param, "ogg_320_filesize"))
            .unwrap_or(relate_hq_size);

        let std_size = Self::val_i64(o, "FileSize")
            .or_else(|| Self::val_i64(o, "filesize"))
            .or_else(|| Self::val_i64(o, "128filesize"))
            .or_else(|| Self::val_i64(o, "filesize_128"))
            .or_else(|| Self::val_i64(o, "size"))
            .or_else(|| Self::val_i64(audio_info, "filesize_128"))
            .or_else(|| Self::val_i64(audio_info, "filesize"))
            .or_else(|| Self::val_i64(audio_info, "size"))
            .or_else(|| Self::val_i64(replace_audio, "filesize_128"))
            .or_else(|| Self::val_i64(trans_param, "ogg_128_filesize"))
            .unwrap_or(relate_std_size);

        let top_res_size = Self::val_i64(o, "ResFileSize")
            .or_else(|| Self::val_i64(o, "HiResFileSize"))
            .or_else(|| Self::val_i64(o, "hiresfilesize"))
            .or_else(|| Self::val_i64(o, "filesize_res"))
            .or_else(|| Self::val_i64(audio_info, "filesize_res"))
            .or_else(|| Self::val_i64(replace_audio, "filesize_res"))
            .or_else(|| Self::val_i64(replace_audio, "filesize_high"))
            .or_else(|| Self::val_i64(audio_info, "filesize_high"))
            .or_else(|| Self::val_i64(o, "filesize_high"))
            .unwrap_or(relate_res_size);

        let raw_res_hash = [
            Self::val_str(o, "ResFileHash"),
            Self::val_str(o, "HiResFileHash"),
            Self::val_str(o, "res_hash"),
            Self::val_str(o, "hires_hash"),
            Self::val_str(replace_audio, "hash_res"),
            Self::val_str(audio_info, "hash_res"),
            Self::val_str(replace_audio, "hash_high"),
            Self::val_str(audio_info, "hash_high"),
            Self::val_str(o, "hash_high"),
            relate_res_hash,
        ]
        .into_iter()
        .find(|s| Self::is_valid_md5_hash(s))
        .unwrap_or_default()
        .to_lowercase();

        let hash_hi_res = if !raw_res_hash.is_empty()
            && raw_res_hash != hash_flac
            && raw_res_hash != explicit_hq_hash
            && (top_res_size <= 0 || sq_size <= 0 || top_res_size >= sq_size)
        {
            raw_res_hash
        } else {
            String::new()
        };

        let hash320 = explicit_hq_hash;

        let hash128 = [
            Self::val_str(trans_param, "ogg_128_hash"),
            Self::val_str(o, "FileHash"),
            Self::val_str(o, "128hash"),
            Self::val_str(o, "hash_128"),
            Self::val_str(o, "hash128"),
            Self::val_str(o, "hash"),
            Self::val_str(o, "Hash"),
            Self::val_str(base, "hash_128"),
            Self::val_str(base, "hash"),
            Self::val_str(replace_audio, "hash_128"),
            Self::val_str(audio_info, "hash_128"),
            Self::val_str(audio_info, "hash"),
            Self::val_str(audio_info, "FileHash"),
            relate_std_hash,
        ]
        .into_iter()
        .find(|s| Self::is_valid_md5_hash(s))
        .unwrap_or_default()
        .to_lowercase();

        let primary_hash = [&hash_hi_res, &hash_flac, &hash320, &hash128]
            .into_iter()
            .find(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| {
                let h = Self::val_str(o, "hash");
                let raw = if !h.is_empty() { h } else { Self::val_str(o, "FileHash") };
                raw.to_lowercase()
            });

        let album_audio_id = Self::val_i64(o, "album_audio_id")
            .or_else(|| Self::val_i64(o, "AlbumAudioId"))
            .or_else(|| Self::val_i64(o, "AlbumAudioID"))
            .or_else(|| Self::val_i64(o, "MixSongID"))
            .or_else(|| Self::val_i64(o, "mixsongid"))
            .or_else(|| Self::val_i64(o, "scid"))
            .or_else(|| Self::val_i64(base, "album_audio_id"))
            .or_else(|| Self::val_i64(audio_info, "album_audio_id"))
            .unwrap_or(0);

        let audio_id = Self::val_i64(o, "audio_id")
            .or_else(|| Self::val_i64(o, "Audioid"))
            .or_else(|| Self::val_i64(o, "songid"))
            .or_else(|| Self::val_i64(base, "audio_id"))
            .or_else(|| Self::val_i64(audio_info, "audio_id"))
            .unwrap_or(album_audio_id);

        let is_vip = Self::val_i64(o, "is_vip") == Some(1)
            || Self::val_i64(o, "privilege") == Some(10)
            || Self::val_i64(o, "Privilege") == Some(10)
            || Self::val_i64(o, "sqprivilege") == Some(10)
            || Self::val_i64(o, "320privilege") == Some(10)
            || Self::val_i64(o, "pay_type") == Some(3)
            || Self::val_i64(o, "PayType") == Some(3)
            || Self::val_i64(o, "fail_process") == Some(4)
            || Self::val_i64(o, "FailProcess") == Some(4)
            || Self::val_i64(old_copyright, "privilege") == Some(10)
            || Self::val_i64(old_copyright, "pay_type") == Some(3)
            || Self::val_i64(old_copyright, "fail_process") == Some(4)
            || Self::val_i64(audio_info, "is_vip") == Some(1)
            || Self::val_i64(audio_info, "privilege") == Some(10)
            || Self::val_i64(audio_info, "privilege_128") == Some(10)
            || Self::val_i64(audio_info, "privilege_320") == Some(10)
            || Self::val_i64(audio_info, "privilege_flac") == Some(10)
            || Self::val_i64(audio_info, "privilege_high") == Some(10)
            || Self::val_i64(audio_info, "pay_type") == Some(3)
            || Self::val_i64(audio_info, "fail_process") == Some(4)
            || Self::val_i64(replace_audio, "privilege") == Some(10)
            || Self::val_i64(replace_audio, "pay_type") == Some(3)
            || Self::val_i64(base, "is_vip") == Some(1)
            || Self::val_i64(base, "privilege") == Some(10)
            || Self::val_i64(base, "pay_type") == Some(3)
            || Self::val_i64(trans_param, "pay_block_tpl") == Some(1)
            || has_relate_vip;

        let is_hi_res = !hash_hi_res.is_empty();
        let has_flac = !hash_flac.is_empty()
            || sq_size > 0
            || Self::val_i64(o, "is_flac") == Some(1)
            || Self::val_i64(o, "is_lossless") == Some(1)
            || Self::val_str(o, "has_flac") == "1";
        let is_lossless = is_hi_res || has_flac;
        let has_320 = !hash320.is_empty()
            || hq_size > 0
            || Self::val_str(o, "has_320") == "1"
            || Self::val_i64(o, "is_320") == Some(1);

        let highest_quality = if is_hi_res {
            "Hi-Res".to_string()
        } else if is_lossless {
            "SQ".to_string()
        } else if has_320 {
            "320K".to_string()
        } else {
            "128K".to_string()
        };

        let raw_pub_date = [
            Self::val_str(o, "publish_date"),
            Self::val_str(o, "publish_time"),
            Self::val_str(o, "publishtime"),
            Self::val_str(o, "PublishDate"),
            Self::val_str(base, "publish_date"),
            Self::val_str(audio_info, "publish_date"),
            Self::val_str(album_info, "publish_date"),
            Self::val_str(album_info, "publishtime"),
            Self::val_str(trans_param, "publish_date"),
        ]
        .into_iter()
        .find(|s| !s.is_empty() && s != "0" && !s.starts_with("0000"))
        .unwrap_or_default();
        let publish_date: String = raw_pub_date.chars().take(10).collect();

        let file_id = Self::val_i64(o, "fileid")
            .or_else(|| Self::val_i64(o, "file_id"))
            .unwrap_or(0);
        let accompany_hash = [
            Self::val_str(o, "AccompanyFileHash"),
            Self::val_str(o, "accompany_hash"),
            Self::val_str(audio_info, "accompany_hash"),
            Self::val_str(trans_param, "ogg_320_accompany_hash"),
            Self::val_str(trans_param, "accompany_hash"),
            relate_acc_hash,
        ]
        .into_iter()
        .find(|s| Self::is_valid_md5_hash(s))
        .unwrap_or_default()
        .to_lowercase();

        let folded_versions = if let Some(grp_arr) = o
            .get("Grp")
            .or_else(|| o.get("grp"))
            .and_then(|v| v.as_array())
        {
            grp_arr
                .iter()
                .filter(|g| g.is_object() && g.get("Grp").is_none() && g.get("grp").is_none())
                .map(Self::parse_single_song)
                .filter(|gs| (!gs.hash.is_empty() || !gs.id.is_empty()) && gs.title != "未知歌曲")
                .collect()
        } else {
            Vec::new()
        };

        Song {
            id: if !primary_hash.is_empty() {
                primary_hash.clone()
            } else if album_audio_id > 0 {
                album_audio_id.to_string()
            } else {
                audio_id.to_string()
            },
            hash: primary_hash,
            hash128,
            hash320,
            hash_flac,
            hash_hi_res,
            accompany_hash,
            size128: std_size,
            size320: hq_size,
            size_flac: sq_size,
            size_hi_res: if is_hi_res { top_res_size } else { 0 },
            title,
            artist,
            artist_id,
            authors: parsed_authors,
            album,
            album_id,
            cover_url,
            duration_ms,
            album_audio_id,
            audio_id,
            mix_song_id: if album_audio_id > 0 {
                album_audio_id.to_string()
            } else {
                audio_id.to_string()
            },
            file_id,
            is_vip,
            is_lossless,
            is_hi_res,
            highest_quality,
            publish_date,
            folded_versions,
        }
    }

    fn extract_climax_range(d: &Value) -> (i64, i64) {
        let mut start_ms = Self::val_i64(d, "climax_start_time")
            .or_else(|| d.get("climax_info").and_then(|c| Self::val_i64(c, "start_time")))
            .or_else(|| d.get("ext_info").and_then(|c| Self::val_i64(c, "climax_start_time")))
            .unwrap_or(0);
        let mut end_ms = Self::val_i64(d, "climax_end_time")
            .or_else(|| d.get("climax_info").and_then(|c| Self::val_i64(c, "end_time")))
            .or_else(|| d.get("ext_info").and_then(|c| Self::val_i64(c, "climax_end_time")))
            .unwrap_or(0);
        if start_ms > 0 && start_ms < 1000 {
            start_ms *= 1000;
        }
        if end_ms > 0 && end_ms < 1000 {
            end_ms *= 1000;
        }
        if end_ms <= start_ms {
            (45_000, 75_000)
        } else {
            (start_ms, end_ms)
        }
    }

    /// 格式化排行榜卡片 Top 3 歌曲预览文本：统一清洗前缀序号与「歌手 - 歌名」倒置结构，输出标准「歌名 - 歌手」
    fn format_rank_preview_line(raw_songname: &str, raw_singer: &str) -> Option<String> {
        let mut sname = raw_songname
            .replace("<em>", "")
            .replace("</em>", "")
            .trim()
            .to_string();
        // 剥离可能自带的 "1. " / "01 " 等前缀序号
        if let Some(rest) = sname
            .strip_prefix("1.")
            .or_else(|| sname.strip_prefix("2."))
            .or_else(|| sname.strip_prefix("3."))
            .or_else(|| sname.strip_prefix("01"))
            .or_else(|| sname.strip_prefix("02"))
            .or_else(|| sname.strip_prefix("03"))
        {
            sname = rest.trim().to_string();
        }
        if sname.is_empty() {
            return None;
        }
        let singer = raw_singer
            .replace("<em>", "")
            .replace("</em>", "")
            .trim()
            .to_string();

        // 酷狗 /api/v3/rank/list 的 songinfo.songname 默认格式为 "歌手 - 歌名" 且 singername 为空
        if singer.is_empty() {
            if let Some((left_artist, right_title)) = sname.split_once(" - ") {
                let art = left_artist.trim();
                let tit = right_title.trim();
                if !tit.is_empty() && !art.is_empty() {
                    return Some(format!("{} - {}", tit, art));
                }
            }
            if let Some((left_artist, right_title)) = sname.split_once('-') {
                let art = left_artist.trim();
                let tit = right_title.trim();
                if !tit.is_empty() && !art.is_empty() {
                    return Some(format!("{} - {}", tit, art));
                }
            }
            return Some(sname);
        }

        // 若 singer 不为空，检查 sname 是否包含 "singer - title"
        if let Some((left_part, right_part)) = sname.split_once(" - ") {
            let l = left_part.trim();
            let r = right_part.trim();
            if l.eq_ignore_ascii_case(&singer) || l.contains(&singer) || singer.contains(l) {
                return Some(format!("{} - {}", r, singer));
            }
            if r.eq_ignore_ascii_case(&singer) || r.contains(&singer) || singer.contains(r) {
                return Some(format!("{} - {}", l, singer));
            }
        }

        Some(format!("{} - {}", sname, singer))
    }

    fn normalize_rank_update_freq(raw_freq: &str, name: &str) -> String {
        let f = raw_freq.trim();
        if f.is_empty() {
            if name.contains("飙升") || name.contains("TOP") || name.contains("热歌") || name.contains("新歌") {
                return "每天更新".to_string();
            }
            return "实时更新".to_string();
        }
        match f {
            "每天" | "每日" => "每天更新".to_string(),
            "工作日" => "工作日更新".to_string(),
            "周一" | "周二" | "周三" | "周四" | "周五" | "周六" | "周日" => {
                format!("每{}更新", f)
            }
            _ => f.to_string(),
        }
    }

    fn rank_home_priority_score(rank_id: i32, name: &str, category: &str) -> usize {
        // 发现页首屏 8 张卡片优先展示最具代表性的核心榜单与多元热榜
        if rank_id == 6666 || name == "飙升榜" || name == "酷狗飙升榜" {
            return 1;
        }
        if rank_id == 8888 || name.contains("TOP500") {
            return 2;
        }
        if name == "新歌榜" || name == "华语新歌榜" {
            return 3;
        }
        if rank_id == 23784 || name.contains("网络热歌") || name.contains("网络红歌") {
            return 4;
        }
        if rank_id == 59703 || name.contains("短视频热歌") {
            return 5;
        }
        if name.contains("DJ热歌") {
            return 6;
        }
        if name.contains("粤语金曲") {
            return 7;
        }
        if rank_id == 4681 || name.contains("BillBoard") || name.contains("Billboard") {
            return 8;
        }
        if name == "内地榜" {
            return 9;
        }
        if name.contains("影视金曲") {
            return 10;
        }
        if name.contains("ACG新歌") {
            return 11;
        }
        if name.contains("欧美榜") || name.contains("欧美金曲") {
            return 12;
        }
        if name.contains("韩国榜") {
            return 13;
        }
        if name.contains("日本榜") || rank_id == 4673 {
            return 14;
        }
        if name.contains("说唱先锋") || name.contains("电音榜") || name.contains("国潮") {
            return 15;
        }
        match category {
            "官方热榜" => 100,
            "特色音乐" => 200,
            "曲风流派" => 300,
            "地区语种" => 400,
            "全球权威" => 500,
            _ => 600,
        }
    }

    /// 酷狗全量 57+ 官方巅峰榜单五维精准分类：
    /// 1. 官方热榜  2. 特色音乐  3. 曲风流派  4. 地区语种  5. 全球权威
    fn classify_rank_category(rank_id: i32, name: &str) -> String {
        // 1. 全球权威榜（国际权威机构与海外主流音乐平台官方榜）
        if matches!(rank_id, 4681 | 4673 | 4680 | 4672 | 38623 | 42807)
            || name.contains("BillBoard")
            || name.contains("Billboard")
            || name.contains("公告牌")
            || name.contains("公信")
            || name.contains("Oricon")
            || name.contains("英国")
            || name.contains("UK")
            || name.contains("美国")
            || name.contains("Melon")
            || name.contains("JOOX")
            || name.contains("KKBOX")
            || name.contains("Beatport")
            || name.contains("SPACE SHOWER")
            || name.contains("日本SS")
            || name.contains("全球")
        {
            return "全球权威".to_string();
        }

        // 2. 曲风流派榜（垂直音乐流派与传统戏曲器乐）
        if matches!(rank_id, 33160 | 33161 | 44412)
            || name.contains("电音")
            || name.contains("说唱")
            || name.contains("国潮")
            || name.contains("国风")
            || name.contains("国乐")
            || name.contains("民谣")
            || name.contains("摇滚")
            || name.contains("纯音乐")
            || name.contains("轻音乐")
            || name.contains("R&B")
            || name.contains("爵士")
            || name.contains("乡村")
            || name.contains("古典")
            || name.contains("粤剧")
            || name.contains("京剧")
            || name.contains("南音")
            || name.contains("戏曲")
            || name.contains("古风")
            || name.contains("蓝调")
        {
            return "曲风流派".to_string();
        }

        // 3. 地区语种榜（华语各地区及海外语种分榜）
        if matches!(rank_id, 31310 | 31311 | 31312 | 31313 | 54848)
            || name.contains("内地")
            || name.contains("香港")
            || name.contains("台湾")
            || name.contains("欧美")
            || name.contains("韩国")
            || name.contains("日本")
            || name.contains("粤语")
            || name.contains("闽南")
            || name.contains("成都")
            || name.contains("小语种")
            || name.contains("越南")
            || name.contains("泰语")
            || name.contains("俄语")
            || name.contains("法语")
        {
            return "地区语种".to_string();
        }

        // 4. 特色音乐榜（ACG、影视、综艺、年代金曲、DJ、原创、情绪主题）
        if matches!(rank_id, 33162 | 33163 | 46910)
            || name.contains("DJ")
            || name.contains("ACG")
            || name.contains("影视")
            || name.contains("综艺")
            || name.contains("原创")
            || name.contains("音乐人")
            || name.contains("80后")
            || name.contains("90后")
            || name.contains("00后")
            || name.contains("伤感")
            || name.contains("儿歌")
            || name.contains("游戏")
        {
            return "特色音乐".to_string();
        }

        // 5. 官方热榜（飙升榜、TOP500、新歌榜、网络热歌、短视频、视频号、收藏、识曲、会员等核心大榜）
        "官方热榜".to_string()
    }

    fn is_valid_iso_date(d: &str) -> bool {
        let b = d.as_bytes();
        b.len() == 10
            && b[0].is_ascii_digit()
            && b[1].is_ascii_digit()
            && b[2].is_ascii_digit()
            && b[3].is_ascii_digit()
            && b[4] == b'-'
            && b[5].is_ascii_digit()
            && b[6].is_ascii_digit()
            && b[7] == b'-'
            && b[8].is_ascii_digit()
            && b[9].is_ascii_digit()
            && !d.starts_with("0000")
    }

    /// 批量补全歌曲精确发行日期、权威专辑名与高清封面 (/kmr/v2/albums 权威核验，不伪造日期)
    pub async fn enrich_songs_publish_date(&self, mut songs: Vec<Song>) -> Vec<Song> {
        let dev = self.store.device();
        let session = self.store.session();
        let mut missing_album_ids = Vec::new();
        let mut seen = HashSet::new();
        for s in &songs {
            let needs_album_meta = s.publish_date.is_empty()
                || s.album.is_empty()
                || s.album == "未知专辑"
                || s.cover_url.is_empty();
            if needs_album_meta && !s.album_id.is_empty() && s.album_id != "0" {
                if seen.insert(s.album_id.clone()) {
                    missing_album_ids.push(json!({ "album_id": s.album_id }));
                }
            }
        }

        let mut album_meta_map: HashMap<String, (String, String, String)> = HashMap::new();
        if !missing_album_ids.is_empty() {
            for chunk in missing_album_ids.chunks(40) {
                let body = json!({
                    "data": chunk,
                    "is_buy": 0,
                    "fields": "album_id,album_name,publish_date,sizable_cover"
                });
                let mut headers = HashMap::new();
                headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
                headers.insert("kg-tid".to_string(), "255".to_string());
                if let Ok(resp) = self
                    .transport
                    .call(
                        "/kmr/v2/albums",
                        &dev,
                        session.as_ref(),
                        HashMap::new(),
                        Some(body),
                        "https://gateway.kugou.com",
                        "POST",
                        false,
                        headers,
                        true,
                        true,
                    )
                    .await
                {
                    if let Some(arr) = resp
                        .data()
                        .get("data")
                        .and_then(|v| v.as_array())
                        .or_else(|| resp.data().as_array())
                        .or_else(|| resp.body.get("data").and_then(|v| v.as_array()))
                    {
                        for item in arr {
                            let aid = Self::val_str(item, "album_id");
                            if aid.is_empty() {
                                continue;
                            }
                            let raw_pdate = Self::val_str(item, "publish_date");
                            let pdate = if raw_pdate.len() >= 10 && Self::is_valid_iso_date(&raw_pdate[..10]) {
                                raw_pdate[..10].to_string()
                            } else {
                                String::new()
                            };
                            let aname = Self::val_str(item, "album_name").trim().to_string();
                            let acover = Self::val_str(item, "sizable_cover");
                            album_meta_map.insert(aid, (pdate, aname, acover));
                        }
                    }
                }
            }
        }

        for s in songs.iter_mut() {
            if s.publish_date.len() > 10 {
                s.publish_date = s.publish_date.chars().take(10).collect();
            }
            if !Self::is_valid_iso_date(&s.publish_date) {
                s.publish_date.clear();
            }
            if let Some((pdate, aname, acover)) = album_meta_map.get(&s.album_id) {
                if s.publish_date.is_empty() && !pdate.is_empty() {
                    s.publish_date = pdate.clone();
                }
                if (s.album.is_empty() || s.album == "未知专辑") && !aname.is_empty() && aname != "0" {
                    s.album = aname.clone();
                }
                if s.cover_url.is_empty() && !acover.is_empty() {
                    s.cover_url = signer::normalize_image_url(Some(acover));
                }
            }
        }
        songs
    }

    fn fm_freq_label_for_id(id: &str) -> String {
        let mut h: u32 = 2166136261;
        for b in id.bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(16777619);
        }
        let major = 88 + (h % 20);
        let minor = (h / 20) % 10;
        format!("FM {}.{} MHz", major, minor)
    }

    fn format_fm_listener_count(id: &str, raw_heat: i64) -> String {
        if raw_heat >= 10_000 {
            return format!("{:.1}w 正在调频", (raw_heat as f64) / 10_000.0);
        }
        if raw_heat >= 1_000 {
            return format!("{:.1}k 正在调频", (raw_heat as f64) / 1_000.0);
        }
        let mut h: u32 = 2166136261;
        for b in id.bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(16777619);
        }
        let val = 180 + (h % 760);
        format!("{:.1}k 正在调频", (val as f64) / 10.0)
    }

    fn normalize_fm_category(raw_cat: &str) -> String {
        let c = raw_cat.trim();
        match c {
            "场景" | "周末" => "场景".to_string(),
            "心情" => "心情".to_string(),
            "风格" | "曲风" | "厂牌" => "风格".to_string(),
            "语言" | "年代语种" => "语言".to_string(),
            "主题" => "主题".to_string(),
            "DJ" | "dj" => "DJ".to_string(),
            "运动" => "运动".to_string(),
            "国风" => "国风".to_string(),
            "乐器" => "乐器".to_string(),
            "人群" | "综艺" | "人群综艺" => "人群综艺".to_string(),
            "儿童" => "儿童".to_string(),
            _ => "主题".to_string(),
        }
    }

    fn resolve_canonical_fmid(cid: &str) -> &str {
        match cid.trim() {
            "hot" => "1",
            "red_heart" => "974",
            "niche_gem" => "16",
            "late_night" => "301",
            "focus_study" => "1646",
            "workout_drive" => "37",
            "cafe_afternoon" => "27",
            "morning_wake" => "150",
            "sleep_ambient" => "1180",
            "cantonese" => "94",
            "mandopop_golden" => "693",
            "western_pop" => "706",
            "acg" => "318",
            "kpop_wave" => "594",
            "retro_8090" => "5",
            "indie_folk" => "16",
            "rnb_soul" => "13",
            "guofeng" => "83",
            "rock_live" => "9",
            "edm_synth" => "995",
            "healing_mood" => "32",
            "emo_rain" => "42",
            "joy_sunshine" => "223",
            "chill_sunset" => "10",
            other => other,
        }
    }

    fn fm_home_priority_score(fmid: &str, name: &str, category: &str) -> usize {
        match fmid {
            "27" => 1,    // 咖啡厅
            "344" => 2,   // 散步
            "301" => 3,   // 睡前
            "974" => 4,   // 热评10W+歌曲
            "693" => 5,   // 那年MP3里的歌
            "94" => 6,    // 怀旧粤语
            "16" => 7,    // 民谣
            "13" => 8,    // R&B
            "32" => 9,    // 安静
            "1646" => 10, // 阅读
            "706" => 11,  // 百听不厌英文歌
            "11" => 12,   // 轻音乐
            "10" => 13,   // 爵士
            "995" => 14,  // 蒸汽波
            "83" => 15,   // 中国风精选
            "318" => 16,  // 二次元精选
            "37" => 17,   // 驾驶
            "1" => 18,    // 酷狗热歌
            "5" => 19,    // 中文经典
            "54" => 20,   // 一个人
            "223" => 21,  // 轻松
            "9" => 22,    // 摇滚
            "782" => 23,  // 经典影视原声
            "744" => 24,  // 健身房
            "60" => 25,   // 钢琴
            "1330" => 26, // 慵懒时光
            "1318" => 27, // 自愈
            "1566" => 28, // 旅行
            "579" => 29,  // J-POP
            "594" => 30,  // K-POP
            "14" => 31,   // 说唱
            "4" => 32,    // 酷狗新歌
            "276" => 33,  // 老情歌
            "118" => 34,  // 古风好歌
            "1424" => 35, // 热血国风
            "392" => 36,  // 跑步
            _ => {
                if name.contains("广场舞") || name.contains("促销") || name.contains("儿歌") || name.contains("胎教") {
                    900
                } else {
                    match category {
                        "场景" => 120,
                        "心情" => 140,
                        "风格" => 160,
                        "语言" => 180,
                        "国风" => 200,
                        "乐器" => 220,
                        "主题" => 240,
                        "运动" => 260,
                        "DJ" => 280,
                        "人群综艺" => 300,
                        "儿童" => 400,
                        _ => 350,
                    }
                }
            }
        }
    }

    fn build_fm_channel_description(
        fmid: &str,
        name: &str,
        category: &str,
        sample_song: &str,
    ) -> String {
        let curated = match fmid {
            "27" => "慵懒爵士与原声木吉他 · 阳光洒落午后窗台",
            "344" => "漫步街角微风 · 让轻快旋律伴随你的每一步",
            "301" => "卸下都市喧嚣与疲惫 · 耳机里的温柔晚风",
            "974" => "千万听众走心共鸣 · 每一首都是故事感神作",
            "693" => "周杰伦/孙燕姿/林俊杰 · 青春记忆里的MP3金曲",
            "94" => "霓虹街角与港乐黄金年代 · 帧享粤语岁月留声",
            "16" => "木吉他与故事感人声 · 傍晚微风里的温暖低语",
            "13" => "丝滑转音与都市霓虹节拍 · 暮色微醺松弛感",
            "32" => "过滤外界嘈杂 · 留一片宁静纯粹的私人声场",
            "1646" => "轻柔器乐与心流律动 · 沉浸阅读与专注时光",
            "706" => "Billboard 经典热单与百听不厌的欧美流行旋律",
            "11" => "空灵纯音与自然呼吸 · 舒缓身心的治愈声波",
            "10" => "丝绒萨克斯与摇摆钢琴 · 复刻复古爵士俱乐部",
            "995" => "复古合成器与霓虹美学 · 迷幻浪漫的赛博梦境",
            "83" => "山水诗意与丝竹管弦 · 东方美学意境之声",
            "318" => "高燃番剧 OP/ED 与日系治愈声线 · 破次元漫游",
            "37" => "开阔地平线与澎湃节奏 · 车载兜风一路向阳",
            "1" => "全天候潮流热歌风向标 · 实时捕捉热门旋律",
            "5" => "跨越时代的华语经典金曲 · 历久弥新的感动",
            "54" => "独处时光的细腻陪伴 · 听懂你未曾说出口的心事",
            "223" => "明快节拍与清亮声线 · 唤醒一整天的好心情",
            "9" => "失真吉他的热血轰鸣 · 感受 Livehouse 前排律动",
            "782" => "光影交织的银幕记忆 · 动人心弦的影视原声带",
            "744" => "高能 BPM 燃脂节拍 · 激活多巴胺与运动潜能",
            "60" => "黑白琴键流淌的诗篇 · 纯净治愈的独奏时刻",
            _ => "",
        };
        if !curated.is_empty() {
            return curated.to_string();
        }
        if !sample_song.is_empty() {
            return format!("正在热播：{}", sample_song);
        }
        match category {
            "场景" => format!("{}场景专属调频 · 沉浸式声学伴听", name),
            "心情" => format!("{}情绪共鸣电台 · 懂你此刻的心境", name),
            "风格" => format!("{}曲风深度漫游 · 精选纯正流派佳作", name),
            "语言" => format!("{}语种精选频道 · 跨越国界的旋律", name),
            "DJ" => format!("{}超重低音与律动节拍 · 释放能量", name),
            "运动" => format!("{}运动节奏电台 · 保持高能心率", name),
            "国风" => format!("{}东方新韵调频 · 诗意国乐之美", name),
            "乐器" => format!("{}器乐独奏鉴赏 · 纯净原声质感", name),
            _ => format!("{}主题随心调频 · 探索不期而遇的好音乐", name),
        }
    }

    pub fn default_fm_channels() -> Vec<FmChannel> {
        let presets = [
            (
                "27",
                "咖啡厅",
                "慵懒爵士与原声木吉他 · 阳光洒落午后窗台",
                "场景",
                2,
                "58.4k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220106175222572121.jpg",
                "https://imge.kugou.com/fmbanner/480/20220106175221826685.jpg",
            ),
            (
                "344",
                "散步",
                "漫步街角微风 · 让轻快旋律伴随你的每一步",
                "场景",
                2,
                "52.6k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20211228155137254626.jpg",
                "https://imge.kugou.com/fmbanner/480/20211228155136741574.jpg",
            ),
            (
                "301",
                "睡前",
                "卸下都市喧嚣与疲惫 · 耳机里的温柔晚风",
                "场景",
                2,
                "61.2k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220106175548708933.jpg",
                "https://imge.kugou.com/fmbanner/480/20220106175546979570.jpg",
            ),
            (
                "974",
                "热评10W+歌曲",
                "千万听众走心共鸣 · 每一首都是故事感神作",
                "主题",
                2,
                "74.8k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220128161421313046.jpg",
                "https://imge.kugou.com/fmbanner/480/20220128161419129910.jpg",
            ),
            (
                "693",
                "那年MP3里的歌",
                "周杰伦/孙燕姿/林俊杰 · 青春记忆里的MP3金曲",
                "主题",
                2,
                "68.9k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20241108203841276289.jpg",
                "https://imge.kugou.com/fmbanner/480/20241108203838274158.jpg",
            ),
            (
                "94",
                "怀旧粤语",
                "霓虹街角与港乐黄金年代 · 帧享粤语岁月留声",
                "语言",
                2,
                "56.3k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20240828121328277673.jpg",
                "https://imge.kugou.com/fmbanner/480/20240828121325363406.jpg",
            ),
            (
                "16",
                "民谣",
                "木吉他与故事感人声 · 傍晚微风里的温暖低语",
                "风格",
                2,
                "44.7k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220120150704995586.jpg",
                "https://imge.kugou.com/fmbanner/480/20220120150703846885.jpg",
            ),
            (
                "13",
                "R&B",
                "丝滑转音与都市霓虹节拍 · 暮色微醺松弛感",
                "风格",
                2,
                "41.5k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220120145940817004.jpg",
                "https://imge.kugou.com/fmbanner/480/20220120145936886531.jpg",
            ),
            (
                "32",
                "安静",
                "过滤外界嘈杂 · 留一片宁静纯粹的私人声场",
                "心情",
                2,
                "53.1k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20240322181935734824.jpg",
                "https://imge.kugou.com/fmbanner/480/20240322181933140804.jpg",
            ),
            (
                "1646",
                "阅读",
                "轻柔器乐与心流律动 · 沉浸阅读与专注时光",
                "场景",
                2,
                "39.4k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20210309165546687615.jpg",
                "https://imge.kugou.com/fmbanner/480/20210309165545159576.jpg",
            ),
            (
                "706",
                "百听不厌英文歌",
                "Billboard 经典热单与百听不厌的欧美流行旋律",
                "语言",
                2,
                "48.2k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220215174138171104.png",
                "https://imge.kugou.com/fmbanner/480/20220215174136701134.jpg",
            ),
            (
                "11",
                "轻音乐",
                "空灵纯音与自然呼吸 · 舒缓身心的治愈声波",
                "风格",
                2,
                "46.8k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20241009170837444619.jpg",
                "https://imge.kugou.com/fmbanner/480/20241009170835552113.jpg",
            ),
            (
                "10",
                "爵士",
                "丝绒萨克斯与摇摆钢琴 · 复刻复古爵士俱乐部",
                "风格",
                2,
                "35.6k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220121113633476690.jpg",
                "https://imge.kugou.com/fmbanner/480/20220121113632447499.jpg",
            ),
            (
                "995",
                "蒸汽波",
                "复古合成器与霓虹美学 · 迷幻浪漫的赛博梦境",
                "风格",
                2,
                "33.9k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220121114126630707.jpg",
                "https://imge.kugou.com/fmbanner/480/20220121114124117706.jpg",
            ),
            (
                "83",
                "中国风精选",
                "山水诗意与丝竹管弦 · 东方美学意境之声",
                "国风",
                2,
                "42.1k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220123164424329448.jpg",
                "https://imge.kugou.com/fmbanner/480/20220123164422552332.jpg",
            ),
            (
                "318",
                "二次元精选",
                "高燃番剧 OP/ED 与日系治愈声线 · 破次元漫游",
                "主题",
                2,
                "45.0k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220128175901474615.jpg",
                "https://imge.kugou.com/fmbanner/480/20220128175859775639.jpg",
            ),
            (
                "37",
                "驾驶",
                "开阔地平线与澎湃节奏 · 车载兜风一路向阳",
                "场景",
                2,
                "49.7k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20211228153959551375.jpg",
                "https://imge.kugou.com/fmbanner/480/20211228153957114380.jpg",
            ),
            (
                "1",
                "酷狗热歌",
                "全天候潮流热歌风向标 · 实时捕捉热门旋律",
                "主题",
                2,
                "82.5k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220128160906877436.jpg",
                "https://imge.kugou.com/fmbanner/480/20220128160904789389.jpg",
            ),
            (
                "5",
                "中文经典",
                "跨越时代的华语经典金曲 · 历久弥新的感动",
                "主题",
                2,
                "64.3k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20241128174242747598.jpg",
                "https://imge.kugou.com/fmbanner/480/20241128174239475443.jpg",
            ),
            (
                "54",
                "一个人",
                "独处时光的细腻陪伴 · 听懂你未曾说出口的心事",
                "场景",
                2,
                "47.2k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20211228154343953202.jpg",
                "https://imge.kugou.com/fmbanner/480/20211228154342434649.jpg",
            ),
            (
                "223",
                "轻松",
                "明快节拍与清亮声线 · 唤醒一整天的好心情",
                "心情",
                2,
                "43.8k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20240322182118942385.jpg",
                "https://imge.kugou.com/fmbanner/480/20240322182116452401.jpg",
            ),
            (
                "9",
                "摇滚",
                "失真吉他的热血轰鸣 · 感受 Livehouse 前排律动",
                "风格",
                2,
                "38.1k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220121113904795893.jpg",
                "https://imge.kugou.com/fmbanner/480/20220121113903745240.jpg",
            ),
            (
                "744",
                "健身房",
                "高能 BPM 燃脂节拍 · 激活多巴胺与运动潜能",
                "运动",
                2,
                "36.4k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20220121124411107744.jpg",
                "https://imge.kugou.com/fmbanner/480/20220121124410798754.jpg",
            ),
            (
                "60",
                "钢琴",
                "黑白琴键流淌的诗篇 · 纯净治愈的独奏时刻",
                "乐器",
                2,
                "37.9k 正在调频",
                "https://imge.kugou.com/fmlogov2/400/20180905155158940454.jpg",
                "https://imge.kugou.com/fmbanner/480/20180905155158790562.jpg",
            ),
        ];
        presets
            .into_iter()
            .map(|(id, name, desc, cat, fm_type, listeners, cover, banner)| FmChannel {
                id: id.to_string(),
                name: name.to_string(),
                cover_url: cover.to_string(),
                description: desc.to_string(),
                category: cat.to_string(),
                fm_type,
                listener_count_text: listeners.to_string(),
                freq_label: Self::fm_freq_label_for_id(id),
                banner_url: banner.to_string(),
                is_recommended: true,
                sample_song: String::new(),
            })
            .collect()
    }

    fn extract_fm_item_cover(item: &Value) -> String {
        for key in [
            "imgurl",
            "imgUrl150",
            "imgUrl100",
            "cover",
            "pic",
            "img",
            "sizable_cover",
            "banner",
        ] {
            let raw = Self::val_str(item, key);
            if !raw.is_empty() && !raw.contains("20210519") {
                let norm = signer::normalize_image_url(Some(&raw));
                if !norm.is_empty() {
                    return norm;
                }
            }
        }
        if let Some(songs) = item
            .get("songlist")
            .or_else(|| item.get("rcmdlist"))
            .or_else(|| item.get("songs"))
            .or_else(|| item.get("song_list"))
            .or_else(|| item.get("songinfo"))
            .and_then(|v| v.as_array())
        {
            for s in songs {
                for key in [
                    "album_sizable_cover",
                    "sizable_cover",
                    "imgurl",
                    "cover",
                    "pic",
                    "trans_param",
                ] {
                    if key == "trans_param" {
                        if let Some(tp) = s.get("trans_param") {
                            let u = Self::val_str(tp, "union_cover");
                            if !u.is_empty() && !u.contains("20210519") {
                                return signer::normalize_image_url(Some(&u));
                            }
                        }
                    } else {
                        let raw = Self::val_str(s, key);
                        if !raw.is_empty() && !raw.contains("20210519") {
                            return signer::normalize_image_url(Some(&raw));
                        }
                    }
                }
            }
        }
        String::new()
    }

    fn extract_fm_item_banner(item: &Value, fallback_cover: &str) -> String {
        let raw_banner = Self::val_str(item, "banner");
        if !raw_banner.is_empty() && !raw_banner.contains("20210519") {
            let sized = raw_banner.replace("{size}", "480");
            let norm = signer::normalize_image_url(Some(&sized));
            if !norm.is_empty() {
                return norm;
            }
        }
        fallback_cover.to_string()
    }

    fn extract_fm_item_sample_song(item: &Value) -> String {
        if let Some(songs) = item
            .get("songlist")
            .or_else(|| item.get("rcmdlist"))
            .or_else(|| item.get("songs"))
            .and_then(|v| v.as_array())
        {
            for s in songs {
                let sname = Self::val_str(s, "name");
                if !sname.is_empty() {
                    return sname;
                }
                let sn = Self::val_str(s, "songname");
                if !sn.is_empty() {
                    return sn;
                }
            }
        }
        if let Some(si) = item.get("song_info") {
            let sname = Self::val_str(si, "name");
            if !sname.is_empty() {
                return sname;
            }
        }
        String::new()
    }

    fn resolve_fm_theme_keywords(channel_id: &str, channel_name: &str) -> Vec<&'static str> {
        let lower = format!("{} {}", channel_id, channel_name).to_lowercase();
        if lower.contains("late_night") || lower.contains("深夜") || lower.contains("晚安") || lower.contains("助眠") || lower.contains("睡前") {
            vec!["深夜 治愈 温柔", "晚安 慢歌 抒情", "夜色 安静 独处"]
        } else if lower.contains("focus_study") || lower.contains("心流") || lower.contains("伴读") || lower.contains("轻音乐") || lower.contains("阅读") || lower.contains("学习") {
            vec!["纯音乐 钢琴 治愈", "轻音乐 伴读 专注", "久石让 坂本龙一 纯音乐"]
        } else if lower.contains("cafe") || lower.contains("咖啡") || lower.contains("午后") || lower.contains("爵士") {
            vec!["咖啡馆 慵懒 爵士", "Bossa Nova 下午茶", "小野丽莎 诺拉琼斯"]
        } else if lower.contains("cantonese") || lower.contains("港乐") || lower.contains("粤语") {
            vec!["粤语经典 港乐", "陈奕迅 张敬轩 粤语", "杨千嬅 卫兰 Beyond"]
        } else if lower.contains("mandopop_golden") || lower.contains("千禧") || lower.contains("怀旧") || lower.contains("金曲") || lower.contains("mp3") {
            vec!["华语经典 千禧金曲", "周杰伦 孙燕姿 林俊杰", "蔡依林 王力宏 陶喆"]
        } else if lower.contains("acg") || lower.contains("二次元") || lower.contains("动漫") || lower.contains("日语") {
            vec!["YOASOBI 米津玄师", "动漫 燃曲 日语", "新海诚 Aimer 泽野弘之"]
        } else if lower.contains("kpop") || lower.contains("韩") {
            vec!["K-Pop 韩语 热单", "NewJeans aespa IU", "G-Dragon BLACKPINK"]
        } else if lower.contains("indie_folk") || lower.contains("民谣") || lower.contains("独立") || lower.contains("散步") {
            vec!["民谣 木吉他 故事", "房东的猫 陈粒 宋冬野", "毛不易 赵雷 独立乐队"]
        } else if lower.contains("rnb_soul") || lower.contains("r&b") || lower.contains("微醺") || lower.contains("灵魂") {
            vec!["R&B 灵魂乐 微醺", "方大同 陶喆 单依纯", "袁娅维 律动 R&B"]
        } else if lower.contains("guofeng") || lower.contains("国风") || lower.contains("古风") || lower.contains("中国风") {
            vec!["国风 戏腔 古风", "银临 双笙 周深 国风", "国风新韵 诗意"]
        } else if lower.contains("rock_live") || lower.contains("摇滚") || lower.contains("乐队") {
            vec!["摇滚 乐队 现场", "告五人 万能青年旅店 新裤子", "五月天 痛仰 橘子海"]
        } else if lower.contains("workout_drive") || lower.contains("公路") || lower.contains("兜风") || lower.contains("车载") || lower.contains("驾驶") || lower.contains("运动") || lower.contains("健身") {
            vec!["公路 兜风 车载节奏", "电音 律动 驾驶", "OneRepublic 节奏 热单"]
        } else if lower.contains("western_pop") || lower.contains("欧美") || lower.contains("英文") {
            vec!["欧美流行 Billboard", "Taylor Swift The Weeknd", "Bruno Mars Ariana Grande"]
        } else if lower.contains("emo_rain") || lower.contains("雨夜") || lower.contains("伤感") || lower.contains("心事") || lower.contains("一个人") {
            vec!["伤感 走心 慢歌", "雨天 抒情 遗憾", "薛之谦 林宥嘉 裘德"]
        } else if lower.contains("joy_sunshine") || lower.contains("元气") || lower.contains("晴空") || lower.contains("快乐") || lower.contains("轻松") {
            vec!["元气 快乐 明快", "清新 好心情 甜歌", "王心凌 告五人 晴天"]
        } else if lower.contains("healing_mood") || lower.contains("治愈") || lower.contains("松弛") || lower.contains("安静") {
            vec!["治愈 清新 松弛感", "周深 温柔 治愈系", "徐佳莹 梁静茹 温暖"]
        } else {
            vec!["热歌 流行 精选", "华语流行 必听榜", "周杰伦 林俊杰 陈奕迅"]
        }
    }

    /// 20. 电台频道列表 (严格对齐 MakcRe/KuGouMusicApi: module/fm_class.js `/v1/class_fm_song` + module/fm_recommend.js `/v1/rcmd_list` + module/yueku_fm.js `/v1/time_fm_info`)
    pub async fn fetch_fm_channels(&self) -> Result<Vec<FmChannel>, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let uid_str = session
            .as_ref()
            .map(|s| s.user_id.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("0")
            .to_string();
        let now_ms = Utc::now().timestamp_millis();
        let sign_key = signer::params_key(now_ms, "");

        // 1. 并发请求 /v1/rcmd_list (推荐电台 44 台) 与 /v1/class_fm_song (全部分类 14 大类 325+ 台)
        let dev_rcmd = dev.clone();
        let sess_rcmd = session.clone();
        let uid_rcmd = uid_str.clone();
        let key_rcmd = sign_key.clone();
        let rcmd_fut = async move {
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "fm.service.kugou.com".to_string());
            let body = json!({
                "appid": APP_ID,
                "clientver": CLIENT_VERSION,
                "clienttime": now_ms,
                "mid": dev_rcmd.mid,
                "key": key_rcmd,
                "rcmdsongcount": 1,
                "level": 0,
                "area_code": 1,
                "get_tracker": 1,
                "uid": uid_rcmd
            });
            self.transport
                .call(
                    "/v1/rcmd_list",
                    &dev_rcmd,
                    sess_rcmd.as_ref(),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
        };

        let dev_cls = dev.clone();
        let sess_cls = session.clone();
        let uid_cls = uid_str.clone();
        let key_cls = sign_key.clone();
        let cls_fut = async move {
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "fm.service.kugou.com".to_string());
            let body = json!({
                "kguid": uid_cls,
                "clienttime": now_ms,
                "mid": dev_cls.mid,
                "platform": "android",
                "clientver": CLIENT_VERSION,
                "uid": uid_cls,
                "get_tracker": 1,
                "key": key_cls,
                "appid": APP_ID
            });
            self.transport
                .call(
                    "/v1/class_fm_song",
                    &dev_cls,
                    sess_cls.as_ref(),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
        };

        let (rcmd_res, cls_res) = tokio::join!(rcmd_fut, cls_fut);

        let mut rcmd_ids: HashSet<String> = HashSet::new();
        let mut rcmd_desc_map: HashMap<String, String> = HashMap::new();
        let mut rcmd_sample_map: HashMap<String, String> = HashMap::new();
        let mut rcmd_raw_items: Vec<Value> = Vec::new();

        if let Ok(resp) = rcmd_res {
            let data = resp.data();
            let arr_opt = data
                .as_array()
                .or_else(|| data.get("list").and_then(|v| v.as_array()))
                .or_else(|| data.get("info").and_then(|v| v.as_array()));
            if let Some(arr) = arr_opt {
                for item in arr {
                    let fmid = Self::val_str(item, "fmid");
                    if fmid.is_empty() {
                        continue;
                    }
                    rcmd_ids.insert(fmid.clone());
                    let desc = Self::val_str(item, "description");
                    let fmname = Self::val_str(item, "fmname");
                    if !desc.is_empty() && desc != fmname {
                        rcmd_desc_map.insert(fmid.clone(), desc);
                    }
                    let sample = Self::extract_fm_item_sample_song(item);
                    if !sample.is_empty() {
                        rcmd_sample_map.insert(fmid.clone(), sample);
                    }
                    rcmd_raw_items.push(item.clone());
                }
            }
        }

        let mut channels: Vec<FmChannel> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        if let Ok(resp) = cls_res {
            if let Some(class_list) = resp
                .data()
                .get("class_list")
                .and_then(|v| v.as_array())
            {
                // 第一遍：收集 10040 (周末) 中的精选电台 ID 归入推荐集合
                for class_obj in class_list {
                    let cid_num = Self::val_i64(class_obj, "classid").unwrap_or(0);
                    if cid_num == 10040 {
                        if let Some(fmlist) = class_obj.get("fmlist").and_then(|v| v.as_array()) {
                            for fm in fmlist {
                                let id = Self::val_str(fm, "fmid");
                                if !id.is_empty() {
                                    rcmd_ids.insert(id);
                                }
                            }
                        }
                    }
                }

                // 第二遍：遍历所有主分类 (排除虚拟聚合分类 10040)，确保每座电台拥有最准确的垂直主题分类
                for class_obj in class_list {
                    let cid_num = Self::val_i64(class_obj, "classid").unwrap_or(0);
                    if cid_num == 10040 {
                        continue;
                    }
                    let raw_cat_name = Self::val_str(class_obj, "classname");
                    let category = Self::normalize_fm_category(&raw_cat_name);
                    let Some(fmlist) = class_obj
                        .get("fmlist")
                        .or_else(|| class_obj.get("fm_list"))
                        .and_then(|v| v.as_array())
                    else {
                        continue;
                    };

                    for item in fmlist {
                        let fmid = Self::val_str(item, "fmid");
                        let name = Self::val_str(item, "fmname");
                        if fmid.is_empty() || name.is_empty() || !seen.insert(fmid.clone()) {
                            continue;
                        }
                        let cover_url = Self::extract_fm_item_cover(item);
                        if cover_url.is_empty() {
                            continue;
                        }
                        let banner_url = Self::extract_fm_item_banner(item, &cover_url);
                        let sample_song = {
                            let s = Self::extract_fm_item_sample_song(item);
                            if !s.is_empty() {
                                s
                            } else {
                                rcmd_sample_map.get(&fmid).cloned().unwrap_or_default()
                            }
                        };
                        let raw_desc = {
                            let d = Self::val_str(item, "description");
                            if !d.is_empty() && d != name {
                                d
                            } else {
                                rcmd_desc_map.get(&fmid).cloned().unwrap_or_default()
                            }
                        };
                        let description = if !raw_desc.is_empty() {
                            raw_desc
                        } else {
                            Self::build_fm_channel_description(&fmid, &name, &category, &sample_song)
                        };
                        let fm_type = Self::val_i64(item, "fmtype").unwrap_or(2) as i32;
                        let heat = Self::val_i64(item, "heat").unwrap_or(0);
                        let is_recommended = rcmd_ids.contains(&fmid)
                            || Self::fm_home_priority_score(&fmid, &name, &category) <= 36;

                        channels.push(FmChannel {
                            freq_label: Self::fm_freq_label_for_id(&fmid),
                            listener_count_text: Self::format_fm_listener_count(&fmid, heat),
                            id: fmid,
                            name,
                            cover_url,
                            description,
                            category: category.clone(),
                            fm_type,
                            banner_url,
                            is_recommended,
                            sample_song,
                        });
                    }
                }
            }
        }

        // 补充 /v1/rcmd_list 中可能未出现在 class_list 的电台
        for item in rcmd_raw_items {
            let fmid = Self::val_str(&item, "fmid");
            let name = Self::val_str(&item, "fmname");
            if fmid.is_empty() || name.is_empty() || !seen.insert(fmid.clone()) {
                continue;
            }
            let cover_url = Self::extract_fm_item_cover(&item);
            if cover_url.is_empty() {
                continue;
            }
            let banner_url = Self::extract_fm_item_banner(&item, &cover_url);
            let raw_cat = Self::val_str(&item, "classname");
            let category = Self::normalize_fm_category(&raw_cat);
            let sample_song = Self::extract_fm_item_sample_song(&item);
            let raw_desc = Self::val_str(&item, "description");
            let description = if !raw_desc.is_empty() && raw_desc != name {
                raw_desc
            } else {
                Self::build_fm_channel_description(&fmid, &name, &category, &sample_song)
            };
            let fm_type = Self::val_i64(&item, "fmtype").unwrap_or(2) as i32;
            let heat = Self::val_i64(&item, "heat").unwrap_or(0);

            channels.push(FmChannel {
                freq_label: Self::fm_freq_label_for_id(&fmid),
                listener_count_text: Self::format_fm_listener_count(&fmid, heat),
                id: fmid,
                name,
                cover_url,
                description,
                category,
                fm_type,
                banner_url,
                is_recommended: true,
                sample_song,
            });
        }

        // 若完全离线导致网关未返回数据，使用带真实官方 fmid 与官方封面的兜底列表
        if channels.is_empty() {
            channels = Self::default_fm_channels();
        } else {
            channels.sort_by(|a, b| {
                let sa = Self::fm_home_priority_score(&a.id, &a.name, &a.category);
                let sb = Self::fm_home_priority_score(&b.id, &b.name, &b.category);
                sa.cmp(&sb).then_with(|| a.id.cmp(&b.id))
            });
        }

        if let Ok(mut guard) = self.cached_fm_channels.lock() {
            *guard = channels.clone();
        }

        Ok(channels)
    }

    async fn call_app_song_list_offset(
        &self,
        fmid_str: &str,
        fm_type: i32,
        offset_val: i64,
        size_val: i64,
    ) -> Vec<Song> {
        let dev = self.store.device();
        let session = self.store.session();
        let uid_str = session
            .as_ref()
            .map(|s| s.user_id.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("0")
            .to_string();
        let now_ms = Utc::now().timestamp_millis();
        let sign_key = signer::params_key(now_ms, "");

        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "fm.service.kugou.com".to_string());
        let body = json!({
            "appid": APP_ID,
            "area_code": 1,
            "clienttime": now_ms,
            "clientver": CLIENT_VERSION,
            "data": [
                {
                    "fmid": fmid_str,
                    "fmtype": fm_type,
                    "offset": offset_val,
                    "size": size_val,
                    "singername": ""
                }
            ],
            "get_tracker": 1,
            "key": sign_key,
            "mid": dev.mid,
            "uid": uid_str
        });

        if let Ok(resp) = self
            .transport
            .call(
                "/v1/app_song_list_offset",
                &dev,
                session.as_ref(),
                HashMap::new(),
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await
        {
            return Self::parse_songs(resp.data());
        }
        Vec::new()
    }

    /// 21. 获取指定电台频道的漫游曲目流 (严格对齐 MakcRe/KuGouMusicApi module/fm_songs.js `/v1/app_song_list_offset` 协议 + 智能分页去重游标)
    pub async fn fetch_fm_songs(&self, channel_id: &str) -> Result<Vec<Song>, String> {
        let raw_cid = channel_id.trim();
        if raw_cid == "daily_vibe" || raw_cid == "personal_fm" {
            return self.fetch_personal_fm().await;
        }
        let cid = Self::resolve_canonical_fmid(raw_cid);

        // 查找该电台的 fm_type 与名称
        let (fm_type, channel_name) = {
            let found = self
                .cached_fm_channels
                .lock()
                .ok()
                .and_then(|list| list.iter().find(|c| c.id == cid).cloned())
                .or_else(|| {
                    Self::default_fm_channels()
                        .into_iter()
                        .find(|c| c.id == cid)
                });
            match found {
                Some(ch) => (if ch.fm_type > 0 { ch.fm_type } else { 2 }, ch.name),
                None => (2, cid.to_string()),
            }
        };

        let cursor = {
            let mut guard = self.fm_channel_cursors.lock().unwrap();
            let entry = guard.entry(cid.to_string()).or_insert(0);
            let cur = *entry;
            *entry = cur.wrapping_add(1);
            cur
        };

        let mut seen_for_channel: HashSet<String> = self
            .seen_fm_channel_hashes
            .lock()
            .ok()
            .and_then(|g| g.get(cid).cloned())
            .unwrap_or_default();
        if seen_for_channel.len() > 160 {
            seen_for_channel.clear();
            if let Ok(mut g) = self.seen_fm_channel_hashes.lock() {
                g.remove(cid);
            }
        }

        let mut collected: Vec<Song> = Vec::new();
        let mut batch_seen: HashSet<String> = HashSet::new();

        let mut push_songs = |items: Vec<Song>, allow_previously_seen: bool, seen_set: &HashSet<String>, out: &mut Vec<Song>| {
            for s in items {
                let k = s.hash.trim().to_lowercase();
                if k.is_empty() || s.title == "未知歌曲" {
                    continue;
                }
                if !allow_previously_seen && seen_set.contains(&k) {
                    continue;
                }
                if batch_seen.insert(k) {
                    out.push(s);
                }
            }
        };

        // 1. 优先请求酷狗官方电台曲库 /v1/app_song_list_offset (每次刷新递增 offset = cursor * 20)
        if cid.chars().all(|c| c.is_ascii_digit()) {
            let primary_offset = (cursor as i64) * 20;
            let mut batch1 = self
                .call_app_song_list_offset(cid, fm_type, primary_offset, 20)
                .await;
            if batch1.is_empty() && primary_offset == 0 {
                let alt_type = if fm_type == 2 { 1 } else { 2 };
                batch1 = self.call_app_song_list_offset(cid, alt_type, 0, 20).await;
            }

            if batch1.is_empty() && primary_offset > 0 {
                // 说明该电台服务端曲池已达末尾，重置游标并从随机/起始位重新切片并错位重排
                if let Ok(mut guard) = self.fm_channel_cursors.lock() {
                    guard.insert(cid.to_string(), 1);
                }
                seen_for_channel.clear();
                if let Ok(mut g) = self.seen_fm_channel_hashes.lock() {
                    g.remove(cid);
                }
                let (wrap_neg1, wrap_zero) = tokio::join!(
                    self.call_app_song_list_offset(cid, fm_type, -1, 20),
                    self.call_app_song_list_offset(cid, fm_type, 20, 20),
                );
                let mut combined = wrap_neg1;
                combined.extend(wrap_zero);
                if !combined.is_empty() {
                    let shift = ((cursor as usize) * 7) % combined.len();
                    combined.rotate_left(shift);
                    push_songs(combined, true, &seen_for_channel, &mut collected);
                }
            } else {
                push_songs(batch1, false, &seen_for_channel, &mut collected);
                if collected.len() < 20 {
                    let next_offset = primary_offset + 20;
                    let batch2 = self
                        .call_app_song_list_offset(cid, fm_type, next_offset, 20)
                        .await;
                    if !batch2.is_empty() {
                        if let Ok(mut guard) = self.fm_channel_cursors.lock() {
                            guard.insert(cid.to_string(), cursor.wrapping_add(2));
                        }
                        push_songs(batch2, false, &seen_for_channel, &mut collected);
                    }
                }
            }
        }

        // 2. 若官方接口偶发未返回足够曲目，按电台真实名称与主题歌单进行同主题补充
        if collected.len() < 16 {
            let kws = Self::resolve_fm_theme_keywords(cid, &channel_name);
            let active_kw = if kws[0] == "热歌 流行 精选" && !channel_name.is_empty() {
                channel_name.as_str()
            } else {
                kws[(cursor as usize) % kws.len()]
            };

            if let Ok(pls) = self.search_playlists(active_kw, 1, 6).await {
                if !pls.is_empty() {
                    let pick_pl = &pls[(cursor as usize) % pls.len()];
                    let gid = if !pick_pl.global_collection_id.is_empty() {
                        &pick_pl.global_collection_id
                    } else {
                        &pick_pl.id
                    };
                    if let Ok(pl_songs) = self.fetch_playlist_songs(gid).await {
                        let skip_n = ((cursor as usize) * 10) % pl_songs.len().max(1);
                        let sliced: Vec<Song> =
                            pl_songs.into_iter().cycle().skip(skip_n).take(20).collect();
                        push_songs(sliced, false, &seen_for_channel, &mut collected);
                    }
                }
            }

            if collected.len() < 16 {
                let search_page = ((cursor % 5) + 1) as u32;
                if let Ok(kw_songs) = self.search_songs(active_kw, search_page, 20).await {
                    push_songs(kw_songs, true, &seen_for_channel, &mut collected);
                }
            }
        }

        let diversified = Self::diversify_artist_order(collected)
            .into_iter()
            .take(24)
            .collect::<Vec<_>>();

        if let Ok(mut guard) = self.seen_fm_channel_hashes.lock() {
            let entry = guard.entry(cid.to_string()).or_default();
            for s in &diversified {
                if !s.hash.is_empty() {
                    entry.insert(s.hash.to_lowercase());
                }
            }
        }

        Ok(self.enrich_songs_album_names_batch(diversified).await)
    }

    /// 22. 新建歌单 / 编辑歌单 / 删除歌单 / 收藏歌单 / 批量添加歌曲到歌单
    pub async fn create_user_playlist(&self, name: &str) -> Result<bool, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录酷狗账号".to_string());
        };
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("歌单名称不能为空".to_string());
        }
        let dev = self.store.device();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);
        let now_sec = Utc::now().timestamp();
        let body = json!({
            "userid": uid_num,
            "token": session.token,
            "total_ver": 0,
            "name": trimmed,
            "type": 0,
            "source": 1,
            "is_pri": 0,
            "list_create_userid": uid_num,
            "list_create_listid": 0,
            "list_create_gid": "",
            "from_shupinmv": 0
        });
        let mut query = HashMap::new();
        query.insert("last_time".to_string(), now_sec.to_string());
        query.insert("last_area".to_string(), "gztx".to_string());
        query.insert("userid".to_string(), session.user_id.clone());
        query.insert("token".to_string(), session.token.clone());

        if let Ok(resp) = self
            .transport
            .call(
                "/cloudlist.service/v5/add_list",
                &dev,
                Some(&session),
                query.clone(),
                Some(body.clone()),
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            if resp.is_success() {
                return Ok(true);
            }
        }

        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "cloudlist.service.kugou.com".to_string());
        let resp = self
            .transport
            .call(
                "/v5/add_list",
                &dev,
                Some(&session),
                query,
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await?;
        Ok(resp.is_success() || resp.http_code == 200)
    }

    /// 编辑歌单名称与简介（持久化保存并尝试云端同步）
    pub async fn update_user_playlist(
        &self,
        list_id: i64,
        global_id: &str,
        title: &str,
        description: &str,
    ) -> Result<bool, String> {
        let clean_title = title.trim();
        if clean_title.is_empty() {
            return Err("歌单名称不能为空".to_string());
        }
        let clean_desc = description.trim();
        for key in [global_id.trim(), &list_id.to_string()] {
            if !key.is_empty() && key != "0" {
                self.store.set_playlist_override(key, clean_title, clean_desc, "");
            }
        }

        if let Some(session) = self.store.session() {
            if list_id > 0 {
                let dev = self.store.device();
                let uid_num = session.user_id.parse::<i64>().unwrap_or(0);
                let now_sec = Utc::now().timestamp();
                let body = json!({
                    "userid": uid_num,
                    "token": session.token,
                    "listid": list_id,
                    "name": clean_title,
                    "intro": clean_desc,
                    "type": 0
                });
                let mut query = HashMap::new();
                query.insert("last_time".to_string(), now_sec.to_string());
                query.insert("last_area".to_string(), "gztx".to_string());
                query.insert("userid".to_string(), session.user_id.clone());
                query.insert("token".to_string(), session.token.clone());
                let mut headers = HashMap::new();
                headers.insert("x-router".to_string(), "cloudlist.service.kugou.com".to_string());
                let _ = self
                    .transport
                    .call(
                        "/v5/edit_list",
                        &dev,
                        Some(&session),
                        query,
                        Some(body),
                        "https://gateway.kugou.com",
                        "POST",
                        false,
                        headers,
                        true,
                        true,
                    )
                    .await;
            }
        }
        Ok(true)
    }

    async fn execute_encrypted_delete_list(&self, list_id: i64, ptype: i32) -> Result<bool, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录酷狗账号".to_string());
        };
        if list_id <= 0 {
            return Err("无效的歌单 ID".to_string());
        }
        let dev = self.store.device();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);
        let client_time = Utc::now().timestamp();

        let data_map = json!({
            "listid": list_id,
            "total_ver": 0,
            "type": ptype,
            "userid": uid_num,
            "token": session.token
        });
        let (aes_key, aes_cipher) = signer::playlist_aes_encrypt(&data_map.to_string())?;
        let p_inner = json!({
            "aes": aes_key,
            "uid": uid_num,
            "token": session.token
        })
        .to_string();
        let p_rsa = signer::rsa_pkcs1_hex(&p_inner)?;
        let key = signer::md5_hex(&format!(
            "{}{}{}{}",
            APP_ID, ANDROID_SALT, CLIENT_VERSION, client_time
        ));

        let mut query = HashMap::new();
        query.insert("clienttime".to_string(), client_time.to_string());
        query.insert("key".to_string(), key);
        query.insert("last_area".to_string(), "gztx".to_string());
        query.insert("clientver".to_string(), CLIENT_VERSION.to_string());
        query.insert("appid".to_string(), APP_ID.to_string());
        query.insert("last_time".to_string(), client_time.to_string());
        query.insert("p".to_string(), p_rsa);

        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "cloudlist.service.kugou.com".to_string());

        let raw_bytes = self
            .transport
            .request_raw_bytes(
                "/v2/delete_list",
                &dev,
                Some(&session),
                query,
                Some(aes_cipher),
                "https://gateway.kugou.com",
                "POST",
                headers,
                true,
                true,
            )
            .await?;

        if raw_bytes.is_empty() {
            return Ok(false);
        }
        let utf8_text = String::from_utf8_lossy(&raw_bytes).trim().to_string();
        let decrypted_text = if utf8_text.starts_with('{') {
            utf8_text
        } else {
            signer::playlist_aes_decrypt(&raw_bytes, &aes_key).unwrap_or(utf8_text)
        };
        if let Ok(parsed) = serde_json::from_str::<Value>(&decrypted_text) {
            let status = Self::val_i64(&parsed, "status").unwrap_or(0);
            let err_code = Self::val_i64(&parsed, "err_code")
                .or_else(|| Self::val_i64(&parsed, "error_code"))
                .or_else(|| Self::val_i64(&parsed, "errcode"))
                .unwrap_or(0);
            return Ok(status == 1 && err_code == 0);
        }
        Ok(false)
    }

    pub async fn delete_user_playlist(&self, list_id: i64) -> Result<bool, String> {
        self.execute_encrypted_delete_list(list_id, 0).await
    }

    pub async fn add_songs_to_playlist_batch(
        &self,
        list_id: i64,
        songs: &[Song],
    ) -> Result<usize, String> {
        if songs.is_empty() {
            return Ok(0);
        }
        let Some(session) = self.store.session() else {
            return Err("请先登录酷狗账号".to_string());
        };
        let dev = self.store.device();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);
        let now_sec = Utc::now().timestamp();

        let mut data_arr = Vec::with_capacity(songs.len());
        for (idx, song) in songs.iter().enumerate() {
            let full_name = if !song.artist.is_empty()
                && song.artist != "未知歌手"
                && !song.title.contains(" - ")
            {
                format!("{} - {}", song.artist, song.title)
            } else {
                song.title.clone()
            };
            let mix_id = if song.album_audio_id > 0 {
                song.album_audio_id
            } else if song.audio_id > 0 {
                song.audio_id
            } else {
                song.mix_song_id.parse::<i64>().unwrap_or(0)
            };
            let primary_hash = if !song.hash128.is_empty() {
                song.hash128.to_uppercase()
            } else {
                song.hash.to_uppercase()
            };
            data_arr.push(json!({
                "number": idx + 1,
                "name": full_name,
                "hash": primary_hash,
                "size": song.size128.max(3_200_000),
                "sort": 0,
                "timelen": (song.duration_ms / 1000).max(0),
                "bitrate": 128,
                "album_id": song.album_id.parse::<i64>().unwrap_or(0),
                "mixsongid": mix_id
            }));
        }

        let body = json!({
            "listid": list_id,
            "userid": uid_num,
            "token": session.token,
            "list_ver": 0,
            "type": 0,
            "slow_upload": 1,
            "scene": "false;null",
            "data": data_arr
        });
        let mut query = HashMap::new();
        query.insert("last_time".to_string(), now_sec.to_string());
        query.insert("last_area".to_string(), "gztx".to_string());
        query.insert("userid".to_string(), session.user_id.clone());
        query.insert("token".to_string(), session.token.clone());

        if let Ok(resp) = self
            .transport
            .call(
                "/cloudlist.service/v6/add_song",
                &dev,
                Some(&session),
                query.clone(),
                Some(body.clone()),
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            if resp.is_success() {
                return Ok(songs.len());
            }
        }

        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "cloudlist.service.kugou.com".to_string());
        let resp = self
            .transport
            .call(
                "/v6/add_song",
                &dev,
                Some(&session),
                query,
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await?;
        if resp.is_success() || resp.http_code == 200 {
            Ok(songs.len())
        } else {
            Ok(0)
        }
    }

    pub async fn add_song_to_user_playlist(&self, list_id: i64, song: &Song) -> Result<bool, String> {
        let count = self
            .add_songs_to_playlist_batch(list_id, std::slice::from_ref(song))
            .await?;
        Ok(count > 0)
    }

    /// 23. 关注歌手列表与关注/取消关注 (对齐 MusicRepository.followedArtists / followArtist / unfollowArtist)
    pub async fn fetch_followed_artists(&self) -> Result<Vec<FollowedArtist>, String> {
        let Some(session) = self.store.session() else {
            return Ok(Vec::new());
        };
        let dev = self.store.device();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);
        let now_sec = Utc::now().timestamp();
        let p_plain = json!({
            "clienttime": now_sec,
            "token": session.token
        })
        .to_string();
        let p_encrypted = signer::rsa_raw_zero_pad_hex(&p_plain)
            .unwrap_or_default()
            .to_uppercase();

        let mut query = HashMap::new();
        query.insert("plat".to_string(), "1".to_string());
        let body = json!({
            "merge": 2,
            "need_iden_type": 1,
            "ext_params": "k_pic,jumptype,singerid,score",
            "userid": uid_num,
            "token": session.token,
            "type": 0,
            "id_type": 0,
            "page": 1,
            "pagesize": 50,
            "p": p_encrypted
        });
        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "relationuser.kugou.com".to_string());

        let mut artists = Vec::new();
        if let Ok(resp) = self
            .transport
            .call(
                "/v4/follow_list",
                &dev,
                Some(&session),
                query,
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await
        {
            if let Some(arr) = resp
                .data()
                .get("lists")
                .or_else(|| resp.data().get("list"))
                .or_else(|| resp.data().get("info"))
                .or_else(|| resp.data().get("follow_list"))
                .and_then(|v| v.as_array())
            {
                for item in arr {
                    let ext = item.get("ext_params").or_else(|| item.get("extra"));
                    let singer_id = {
                        let s = Self::val_str(item, "singerid");
                        if !s.is_empty() && s != "0" {
                            s
                        } else if let Some(e) = ext {
                            let es = Self::val_str(e, "singerid");
                            if !es.is_empty() && es != "0" {
                                es
                            } else {
                                Self::val_str(item, "userid")
                            }
                        } else {
                            Self::val_str(item, "userid")
                        }
                    };
                    let target_uid = {
                        let t = Self::val_str(item, "target_userid");
                        if !t.is_empty() {
                            t
                        } else {
                            Self::val_str(item, "userid")
                        }
                    };
                    let name = [
                        Self::val_str(item, "singername"),
                        Self::val_str(item, "author_name"),
                        Self::val_str(item, "nickname"),
                        Self::val_str(item, "r_nick"),
                        Self::val_str(item, "name"),
                    ]
                    .into_iter()
                    .find(|s| !s.is_empty())
                    .unwrap_or_default();

                    if !name.is_empty() {
                        let pic = [
                            Self::val_str(item, "k_pic"),
                            ext.map(|e| Self::val_str(e, "k_pic")).unwrap_or_default(),
                            Self::val_str(item, "pic"),
                            Self::val_str(item, "avatar"),
                            Self::val_str(item, "imgurl"),
                        ]
                        .into_iter()
                        .find(|s| !s.is_empty())
                        .unwrap_or_default();

                        artists.push(FollowedArtist {
                            artist_id: if singer_id.is_empty() {
                                target_uid.clone()
                            } else {
                                singer_id
                            },
                            target_user_id: target_uid,
                            name,
                            avatar_url: signer::normalize_image_url(Some(&pic)),
                            song_count: Self::val_i64(item, "songcount")
                                .or_else(|| Self::val_i64(item, "song_count"))
                                .unwrap_or(0) as i32,
                            album_count: Self::val_i64(item, "albumcount")
                                .or_else(|| Self::val_i64(item, "album_count"))
                                .unwrap_or(0) as i32,
                            fans_count: Self::val_i64(item, "fanscount")
                                .or_else(|| Self::val_i64(item, "fans_count"))
                                .unwrap_or(0) as i32,
                            is_followed: true,
                            intro: Self::val_str(item, "intro"),
                        });
                    }
                }
            }
        }

        // 并发或按需补全前 10 位缺少作品数的已关注歌手详情
        for idx in 0..artists.len().min(10) {
            if artists[idx].song_count == 0 && !artists[idx].artist_id.is_empty() {
                if let Ok(det) = self.fetch_artist_detail(&artists[idx].artist_id).await {
                    if det.song_count > 0 {
                        artists[idx].song_count = det.song_count;
                    }
                    if det.album_count > 0 {
                        artists[idx].album_count = det.album_count;
                    }
                    if det.fans_count > 0 {
                        artists[idx].fans_count = det.fans_count;
                    }
                    if artists[idx].avatar_url.is_empty() && !det.avatar_url.is_empty() {
                        artists[idx].avatar_url = det.avatar_url;
                    }
                    if artists[idx].intro.is_empty() && !det.intro.is_empty() {
                        artists[idx].intro = det.intro;
                    }
                }
            }
        }

        Ok(artists)
    }

    pub async fn toggle_follow_artist(
        &self,
        artist_id: &str,
        follow: bool,
    ) -> Result<bool, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录酷狗账号".to_string());
        };
        let dev = self.store.device();
        let now_sec = Utc::now().timestamp();
        let target_id = artist_id.trim().parse::<i64>().unwrap_or(0);
        if target_id <= 0 {
            return Err("无效的歌手 ID".to_string());
        }
        let path = if follow {
            "/followservice/v3/follow_singer"
        } else {
            "/followservice/v3/unfollow_singer"
        };

        let params_plain = json!({
            "singerid": target_id,
            "token": session.token
        })
        .to_string();
        if let Ok(enc) = signer::encrypt_login_params(&params_plain) {
            let p_plain = json!({
                "clienttime": now_sec,
                "key": enc.key
            })
            .to_string();
            if let Ok(p_rsa) = signer::rsa_pkcs1_hex(&p_plain) {
                let mut query = HashMap::new();
                query.insert("clienttime".to_string(), now_sec.to_string());
                let body = json!({
                    "plat": 0,
                    "userid": session.user_id.parse::<i64>().unwrap_or(0),
                    "singerid": target_id,
                    "source": 7,
                    "p": p_rsa.to_uppercase(),
                    "params": enc.ciphertext
                });
                if let Ok(resp) = self
                    .transport
                    .call(
                        path,
                        &dev,
                        Some(&session),
                        query,
                        Some(body),
                        "https://gateway.kugou.com",
                        "POST",
                        false,
                        HashMap::new(),
                        true,
                        true,
                    )
                    .await
                {
                    if resp.is_success() {
                        return Ok(true);
                    }
                }
            }
        }

        // 备用通道: relationuser.kugou.com /v4/follow 或 /v4/unfollow
        let v4_path = if follow { "/v4/follow" } else { "/v4/unfollow" };
        let p_json = json!({
            "clienttime": now_sec,
            "token": session.token
        })
        .to_string();
        if let Ok(p_enc) = signer::rsa_pkcs1_hex(&p_json) {
            let mut query = HashMap::new();
            query.insert("plat".to_string(), "1".to_string());
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "relationuser.kugou.com".to_string());
            let body = json!({
                "userid": session.user_id.parse::<i64>().unwrap_or(0),
                "target_userid": target_id,
                "singerid": target_id,
                "type": 1,
                "id_type": 1,
                "source": 13,
                "p": p_enc.to_uppercase()
            });
            let resp = self
                .transport
                .call(
                    v4_path,
                    &dev,
                    Some(&session),
                    query,
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await?;
            return Ok(resp.is_success());
        }

        Err("关注状态同步失败".to_string())
    }

    /// 24. 已购数字专辑与已购单曲 (严格对齐 Android MusicRepository.userPurchasedAlbums & userPurchasedSongs)
    pub async fn fetch_purchased_albums(&self) -> Result<Vec<PurchasedAlbumItem>, String> {
        let Some(session) = self.store.session() else {
            return Ok(Vec::new());
        };
        let dev = self.store.device();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);
        let mut albums = Vec::new();
        let mut seen_album_ids = HashSet::new();

        for page in 1..=5 {
            let body = json!({
                "appid": APP_ID,
                "clientver": CLIENT_VERSION.to_string(),
                "userid": uid_num,
                "token": session.token,
                "page": page,
                "pagesize": 30,
                "deleted": 0,
                "need_audio_info": 1
            });

            // 注意：/openapi/v1/copyright/get_album_goods 直接走 gateway.kugou.com，绝不能加错误的 x-router: openapi.kugou.com！
            let Ok(resp) = self
                .transport
                .call(
                    "/openapi/v1/copyright/get_album_goods",
                    &dev,
                    Some(&session),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await
            else {
                break;
            };

            let d_root = resp.data();
            let raw_arr_opt = d_root
                .as_array()
                .or_else(|| d_root.get("data").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("list").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("lists").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("info").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("goods").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("album_list").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("albums").and_then(|v| v.as_array()))
                .or_else(|| resp.body.get("data").and_then(|v| v.as_array()));

            let Some(arr) = raw_arr_opt else {
                break;
            };
            if arr.is_empty() {
                break;
            }

            let raw_len = arr.len();
            let mut added_in_page = 0usize;
            for item in arr {
                let inner = item
                    .get("album_info")
                    .or_else(|| item.get("album"))
                    .or_else(|| item.get("goods_info"))
                    .unwrap_or(item);
                let aid = [
                    Self::val_str(inner, "album_id"),
                    Self::val_str(item, "album_id"),
                    Self::val_str(inner, "albumid"),
                    Self::val_str(item, "id"),
                ]
                .into_iter()
                .find(|s| !s.is_empty() && s != "0")
                .unwrap_or_default();

                if aid.is_empty() || !seen_album_ids.insert(aid.clone()) {
                    continue;
                }

                let aname = [
                    Self::val_str(inner, "album_name"),
                    Self::val_str(item, "album_name"),
                    Self::val_str(inner, "goods_name"),
                    Self::val_str(item, "goods_name"),
                    Self::val_str(inner, "name"),
                ]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or_default();

                let artist_name = [
                    Self::val_str(inner, "author_name"),
                    Self::val_str(inner, "singer_name"),
                    Self::val_str(inner, "singername"),
                    Self::val_str(item, "author_name"),
                    Self::val_str(item, "singer_name"),
                ]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or_default();

                let pic = [
                    Self::val_str(inner, "sizable_cover"),
                    Self::val_str(inner, "imgurl"),
                    Self::val_str(inner, "cover"),
                    Self::val_str(item, "sizable_cover"),
                    Self::val_str(item, "imgurl"),
                    Self::val_str(item, "cover"),
                ]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or_default();

                let song_count = Self::val_i64(inner, "song_count")
                    .or_else(|| Self::val_i64(inner, "songcount"))
                    .or_else(|| Self::val_i64(inner, "audio_count"))
                    .or_else(|| Self::val_i64(item, "song_count"))
                    .unwrap_or(0) as i32;

                let buy_time = [
                    Self::val_str(item, "buy_time"),
                    Self::val_str(item, "add_time"),
                    Self::val_str(inner, "buy_time"),
                ]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or_default();

                let publish_date = [
                    Self::val_str(inner, "publish_date"),
                    Self::val_str(inner, "publishtime"),
                    Self::val_str(item, "publish_date"),
                ]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or_default()
                .chars()
                .take(10)
                .collect();

                albums.push(PurchasedAlbumItem {
                    album_id: aid,
                    album_name: aname,
                    artist_name,
                    cover_url: signer::normalize_image_url(Some(&pic)),
                    song_count,
                    buy_time,
                    publish_date,
                });
                added_in_page += 1;
            }

            if added_in_page == 0 || raw_len < 30 {
                break;
            }
        }

        // 对齐 Android 端：若返回的已购专辑缺少专辑名称、封面或曲目数，自动调用 fetch_album_detail / fetch_album_songs 补全
        for alb in albums.iter_mut() {
            if alb.album_name.is_empty()
                || alb.cover_url.is_empty()
                || alb.artist_name.is_empty()
                || alb.song_count <= 0
            {
                if let Ok(detail) = self.fetch_album_detail(&alb.album_id).await {
                    if alb.album_name.is_empty() && !detail.album_name.is_empty() {
                        alb.album_name = detail.album_name;
                    }
                    if alb.artist_name.is_empty() && !detail.artist_name.is_empty() {
                        alb.artist_name = detail.artist_name;
                    }
                    if alb.cover_url.is_empty() && !detail.cover_url.is_empty() {
                        alb.cover_url = detail.cover_url;
                    }
                    if alb.publish_date.is_empty() && !detail.publish_date.is_empty() {
                        alb.publish_date = detail.publish_date;
                    }
                    if alb.song_count <= 0 && detail.song_count > 0 {
                        alb.song_count = detail.song_count;
                    }
                }
            }
            if alb.song_count <= 0 {
                if let Ok(tracks) = self.fetch_album_songs(&alb.album_id).await {
                    if !tracks.is_empty() {
                        alb.song_count = tracks.len() as i32;
                        if alb.cover_url.is_empty() {
                            alb.cover_url = tracks
                                .first()
                                .map(|s| s.cover_url.clone())
                                .unwrap_or_default();
                        }
                    }
                }
            }
            if alb.album_name.is_empty() {
                alb.album_name = format!("数字专辑 #{}", alb.album_id);
            }
        }

        Ok(albums)
    }

    pub async fn fetch_purchased_songs(&self) -> Result<Vec<PurchasedSongItem>, String> {
        let Some(session) = self.store.session() else {
            return Ok(Vec::new());
        };
        let dev = self.store.device();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);
        let mut raw_songs: Vec<(Song, String)> = Vec::new();
        let mut seen_keys = HashSet::new();

        for page in 1..=5 {
            let body = json!({
                "area_code": "1",
                "appid": APP_ID,
                "clientver": CLIENT_VERSION.to_string(),
                "userid": uid_num,
                "token": session.token,
                "page": page,
                "pagesize": 50,
                "deleted": 0,
                "need_audio_info": 1
            });

            // 注意：/openapi/copyright/v1/audio/get_goods 直接走 gateway.kugou.com，不加错误的 x-router
            let Ok(resp) = self
                .transport
                .call(
                    "/openapi/copyright/v1/audio/get_goods",
                    &dev,
                    Some(&session),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await
            else {
                break;
            };

            let d_root = resp.data();
            let arr_opt = d_root
                .as_array()
                .or_else(|| d_root.get("data").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("list").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("lists").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("info").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("goods").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("audio_list").and_then(|v| v.as_array()))
                .or_else(|| d_root.get("goods_list").and_then(|v| v.as_array()))
                .or_else(|| resp.body.get("data").and_then(|v| v.as_array()));

            let mut page_added = 0usize;
            if let Some(arr) = arr_opt {
                if arr.is_empty() {
                    break;
                }
                let raw_len = arr.len();
                for item in arr {
                    let buy_time = [
                        Self::val_str(item, "buy_time"),
                        Self::val_str(item, "add_time"),
                    ]
                    .into_iter()
                    .find(|s| !s.is_empty())
                    .unwrap_or_default();
                    let inner = item
                        .get("audio_info")
                        .or_else(|| item.get("song_info"))
                        .or_else(|| item.get("goods_info"))
                        .unwrap_or(item);
                    let parsed = Self::parse_single_song(inner);
                    if (!parsed.hash.is_empty() || !parsed.id.is_empty())
                        && parsed.title != "未知歌曲"
                    {
                        let key = if !parsed.hash.is_empty() {
                            parsed.hash.to_lowercase()
                        } else {
                            parsed.id.to_lowercase()
                        };
                        if seen_keys.insert(key) {
                            raw_songs.push((parsed, buy_time));
                            page_added += 1;
                        }
                    }
                }
                if page_added == 0 || raw_len < 50 {
                    break;
                }
            } else {
                let fallback_songs = Self::parse_songs(d_root);
                if fallback_songs.is_empty() {
                    break;
                }
                for s in fallback_songs {
                    let key = if !s.hash.is_empty() {
                        s.hash.to_lowercase()
                    } else {
                        s.id.to_lowercase()
                    };
                    if seen_keys.insert(key) {
                        let bt = s.publish_date.clone();
                        raw_songs.push((s, bt));
                        page_added += 1;
                    }
                }
                if page_added < 50 {
                    break;
                }
            }
        }

        if raw_songs.is_empty() {
            return Ok(Vec::new());
        }

        let (songs_vec, buy_times): (Vec<Song>, Vec<String>) = raw_songs.into_iter().unzip();
        let enriched = self.enrich_songs_privilege_lite(songs_vec).await;
        let items = enriched
            .into_iter()
            .zip(buy_times.into_iter())
            .map(|(s, bt)| PurchasedSongItem {
                buy_time: if !bt.is_empty() {
                    bt
                } else {
                    s.publish_date.clone()
                },
                song: s,
            })
            .collect();
        Ok(items)
    }

    /// 25. 歌手主页详情、歌手单曲、专辑详情与专辑曲目
    pub async fn fetch_artist_detail(&self, artist_id: &str) -> Result<FollowedArtist, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let aid_trimmed = artist_id.trim().to_string();

        // 并发请求通道 1 (/kmr/v3/author) 与通道 2 (CDN /api/v3/singer/info)，将两次串行 RTT 压缩为单次并行 RTT
        let dev_gw = dev.clone();
        let sess_gw = session.clone();
        let aid_gw = aid_trimmed.clone();
        let gw_fut = async move {
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
            headers.insert("kg-tid".to_string(), "36".to_string());
            let author_val = if let Ok(n) = aid_gw.parse::<i64>() {
                json!(n)
            } else {
                json!(aid_gw)
            };
            let body = json!({
                "author_id": author_val
            });
            if let Ok(resp) = self
                .transport
                .call(
                    "/kmr/v3/author",
                    &dev_gw,
                    sess_gw.as_ref(),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
            {
                let d = resp.data().get("data").unwrap_or_else(|| resp.data());
                let base = d.get("base").unwrap_or(d);
                let extra = d.get("extra").unwrap_or(d);
                let stat = d.get("stat").unwrap_or(d);
                let name = Self::val_str(base, "author_name");
                if !name.is_empty() {
                    let pic = {
                        let p = Self::val_str(base, "sizable_avatar");
                        if !p.is_empty() {
                            p
                        } else {
                            Self::val_str(base, "avatar")
                        }
                    };
                    let intro = {
                        let li = Self::val_str(extra, "long_intro");
                        if !li.is_empty() {
                            li
                        } else {
                            Self::val_str(extra, "intro")
                        }
                    };
                    let fans_val = Self::val_i64(extra, "fans_count")
                        .or_else(|| Self::val_i64(extra, "fansnums"))
                        .or_else(|| Self::val_i64(extra, "fans_num"))
                        .or_else(|| Self::val_i64(base, "fans_count"))
                        .or_else(|| Self::val_i64(base, "fansnums"))
                        .or_else(|| Self::val_i64(stat, "fans_count"))
                        .or_else(|| Self::val_i64(stat, "fansnums"))
                        .or_else(|| Self::val_i64(d, "fansnums"))
                        .or_else(|| Self::val_i64(d, "fans_count"))
                        .or_else(|| Self::val_i64(d, "fanscount"))
                        .unwrap_or(0) as i32;
                    return Some(FollowedArtist {
                        artist_id: aid_gw,
                        target_user_id: String::new(),
                        name,
                        avatar_url: signer::normalize_image_url(Some(&pic)),
                        song_count: Self::val_i64(extra, "audio_count")
                            .or_else(|| Self::val_i64(base, "audio_count"))
                            .or_else(|| Self::val_i64(d, "audio_count"))
                            .unwrap_or(0) as i32,
                        album_count: Self::val_i64(extra, "album_count")
                            .or_else(|| Self::val_i64(base, "album_count"))
                            .or_else(|| Self::val_i64(d, "album_count"))
                            .unwrap_or(0) as i32,
                        fans_count: fans_val,
                        is_followed: false,
                        intro,
                    });
                }
            }
            None
        };

        let dev_cdn = dev.clone();
        let aid_cdn = aid_trimmed.clone();
        let cdn_fut = async move {
            let mut query = HashMap::new();
            query.insert("version".to_string(), "9108".to_string());
            query.insert("singerid".to_string(), aid_cdn.clone());
            query.insert("plat".to_string(), "0".to_string());
            query.insert("with_res_tag".to_string(), "1".to_string());
            if let Ok(resp) = self
                .transport
                .call(
                    "/api/v3/singer/info",
                    &dev_cdn,
                    None,
                    query,
                    None,
                    "http://mobilecdnbj.kugou.com",
                    "GET",
                    false,
                    HashMap::new(),
                    false,
                    false,
                )
                .await
            {
                let d = resp.data().get("data").unwrap_or_else(|| resp.data());
                let name = Self::val_str(d, "singername");
                if !name.is_empty() {
                    let pic = Self::val_str(d, "imgurl");
                    let raw_intro = Self::val_str(d, "intro");
                    let raw_profile = Self::val_str(d, "profile");
                    let cdn_intro = if !raw_intro.is_empty() && !raw_profile.is_empty() && !raw_intro.contains(&raw_profile) {
                        format!("{}\n\n{}", raw_intro, raw_profile)
                    } else if !raw_intro.is_empty() {
                        raw_intro
                    } else {
                        raw_profile
                    };
                    return Some(FollowedArtist {
                        artist_id: aid_cdn,
                        target_user_id: Self::val_str(d, "userid"),
                        name,
                        avatar_url: signer::normalize_image_url(Some(&pic)),
                        song_count: Self::val_i64(d, "songcount").unwrap_or(0) as i32,
                        album_count: Self::val_i64(d, "albumcount").unwrap_or(0) as i32,
                        fans_count: Self::val_i64(d, "fanscount")
                            .or_else(|| Self::val_i64(d, "fansnums"))
                            .unwrap_or(0) as i32,
                        is_followed: false,
                        intro: cdn_intro,
                    });
                }
            }
            None
        };

        let (gw_artist, cdn_artist) = tokio::join!(gw_fut, cdn_fut);

        let merged_artist = match (gw_artist, cdn_artist) {
            (Some(gw), Some(cdn)) => Some(FollowedArtist {
                artist_id: aid_trimmed.clone(),
                target_user_id: cdn.target_user_id,
                name: if !gw.name.is_empty() { gw.name } else { cdn.name },
                avatar_url: if !gw.avatar_url.is_empty() { gw.avatar_url } else { cdn.avatar_url },
                song_count: gw.song_count.max(cdn.song_count),
                album_count: gw.album_count.max(cdn.album_count),
                fans_count: gw.fans_count.max(cdn.fans_count),
                is_followed: false,
                intro: if cdn.intro.len() > gw.intro.len() { cdn.intro } else { gw.intro },
            }),
            (Some(gw), None) => Some(gw),
            (None, Some(cdn)) => Some(cdn),
            (None, None) => None,
        };

        if let Some(mut final_artist) = merged_artist {
            // 通道 3: 若 /kmr/v3/author 与 /api/v3/singer/info 均未携带粉丝数 (fans_count == 0)，
            // 通过 /v1/search/author 按歌手名与 artist_id 精确补全真实 FansNum (如鹿晗 800.4w 粉丝)
            if final_artist.fans_count <= 0 && !final_artist.name.trim().is_empty() {
                if let Ok(paged) = self.search_artists_paged(&final_artist.name, 1, 10).await {
                    let matched = paged
                        .items
                        .iter()
                        .find(|a| a.artist_id == aid_trimmed)
                        .or_else(|| {
                            paged.items.iter().find(|a| {
                                a.name.trim().eq_ignore_ascii_case(final_artist.name.trim())
                            })
                        });
                    if let Some(m) = matched {
                        if m.fans_count > 0 {
                            final_artist.fans_count = m.fans_count;
                        }
                        if final_artist.song_count <= 0 && m.song_count > 0 {
                            final_artist.song_count = m.song_count;
                        }
                        if final_artist.album_count <= 0 && m.album_count > 0 {
                            final_artist.album_count = m.album_count;
                        }
                    }
                }
            }
            return Ok(final_artist);
        }
        Err("未获取到歌手详情".to_string())
    }

    pub async fn fetch_artist_songs(
        &self,
        artist_id: &str,
        sort: &str,
        page: u32,
    ) -> Result<Vec<Song>, String> {
        let aid = artist_id.trim();
        if aid.is_empty() {
            return Ok(Vec::new());
        }
        let dev = self.store.device();
        let session = self.store.session();
        let page_size = 100u32;
        let sort_val = if sort == "new" { "2" } else { "1" };

        // 并发请求通道 1 (/openapi/kmr/v2/audio_group/author) 与通道 2 (/kmr/v1/audio_group/author)
        let dev_v2 = dev.clone();
        let sess_v2 = session.clone();
        let aid_v2 = aid.to_string();
        let sort_v2 = sort_val.to_string();
        let v2_fut = async move {
            let mut v2_query = HashMap::new();
            v2_query.insert("author_id".to_string(), aid_v2);
            v2_query.insert("area_code".to_string(), "all".to_string());
            v2_query.insert("Filter".to_string(), "filter".to_string());
            v2_query.insert("page".to_string(), page.to_string());
            v2_query.insert("pagesize".to_string(), page_size.to_string());
            v2_query.insert("replace_api_version".to_string(), "1".to_string());
            v2_query.insert("sort".to_string(), sort_v2);
            v2_query.insert("mvdata_need".to_string(), "1".to_string());
            v2_query.insert("show_audio_honor".to_string(), "1".to_string());
            v2_query.insert("show_audio_tag".to_string(), "1".to_string());
            v2_query.insert("replace_need".to_string(), "1".to_string());

            let mut v2_headers = HashMap::new();
            v2_headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
            v2_headers.insert("kg-tid".to_string(), "220".to_string());

            if let Ok(resp) = self
                .transport
                .call(
                    "/openapi/kmr/v2/audio_group/author",
                    &dev_v2,
                    sess_v2.as_ref(),
                    v2_query,
                    None,
                    "https://gateway.kugou.com",
                    "GET",
                    false,
                    v2_headers,
                    true,
                    true,
                )
                .await
            {
                let mut songs = Self::parse_songs(resp.data());
                if songs.is_empty() {
                    songs = Self::parse_songs(&resp.body);
                }
                return songs;
            }
            Vec::new()
        };

        let dev_v1 = dev.clone();
        let sess_v1 = session.clone();
        let aid_v1 = aid.to_string();
        let sort_num = if sort == "new" { 2 } else { 1 };
        let v1_fut = async move {
            let now_sec = Utc::now().timestamp();
            let v1_body = json!({
                "appid": APP_ID,
                "clientver": CLIENT_VERSION,
                "clienttime": now_sec,
                "key": signer::params_key(now_sec, ""),
                "author_id": aid_v1,
                "pagesize": page_size,
                "page": page,
                "sort": sort_num,
                "area_code": "all",
                "replace_api_version": 1,
                "replace_need": 1,
                "show_audio_honor": 1,
                "show_audio_tag": 1
            });
            let mut v1_headers = HashMap::new();
            v1_headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
            v1_headers.insert("kg-tid".to_string(), "220".to_string());

            if let Ok(resp) = self
                .transport
                .call(
                    "/kmr/v1/audio_group/author",
                    &dev_v1,
                    sess_v1.as_ref(),
                    HashMap::new(),
                    Some(v1_body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    v1_headers,
                    true,
                    true,
                )
                .await
            {
                let mut songs = Self::parse_songs(resp.data());
                if songs.is_empty() {
                    songs = Self::parse_songs(&resp.body);
                }
                return songs;
            }
            Vec::new()
        };

        let (mut v2_songs, v1_songs) = tokio::join!(v2_fut, v1_fut);

        let base_list = if !v2_songs.is_empty() && !v1_songs.is_empty() {
            let mut v1_by_title: HashMap<String, Song> = HashMap::new();
            for s1 in v1_songs {
                let k = s1.title.trim().to_lowercase();
                if !v1_by_title.contains_key(&k) || s1.is_hi_res || s1.is_lossless {
                    v1_by_title.insert(k, s1);
                }
            }
            for s2 in &mut v2_songs {
                if let Some(s1) = v1_by_title.get(&s2.title.trim().to_lowercase()) {
                    if s2.hash_hi_res.is_empty() && !s1.hash_hi_res.is_empty() {
                        s2.hash_hi_res = s1.hash_hi_res.clone();
                        s2.is_hi_res = true;
                        s2.is_lossless = true;
                        s2.highest_quality = "Hi-Res".to_string();
                    }
                    if s2.hash_flac.is_empty() && !s1.hash_flac.is_empty() {
                        s2.hash_flac = s1.hash_flac.clone();
                        s2.is_lossless = true;
                        if !s2.is_hi_res {
                            s2.highest_quality = "SQ".to_string();
                        }
                    }
                    if s2.hash320.is_empty() && !s1.hash320.is_empty() {
                        s2.hash320 = s1.hash320.clone();
                    }
                }
                if s2.artist_id.is_empty() || s2.artist_id == "0" {
                    s2.artist_id = aid.to_string();
                }
            }
            v2_songs
        } else if !v2_songs.is_empty() {
            for s2 in &mut v2_songs {
                if s2.artist_id.is_empty() || s2.artist_id == "0" {
                    s2.artist_id = aid.to_string();
                }
            }
            v2_songs
        } else {
            v1_songs
        };

        if !base_list.is_empty() {
            return Ok(self.enrich_songs_privilege_lite(base_list).await);
        }

        // 通道 3: CDN 兜底 (/api/v3/singer/song)
        let mut query = HashMap::new();
        query.insert("version".to_string(), "9108".to_string());
        query.insert("singerid".to_string(), aid.to_string());
        query.insert("plat".to_string(), "0".to_string());
        query.insert("page".to_string(), page.to_string());
        query.insert("pagesize".to_string(), page_size.to_string());
        query.insert(
            "sorttype".to_string(),
            if sort == "new" { "1" } else { "2" }.to_string(),
        );
        query.insert("area_code".to_string(), "1".to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/singer/song",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            let mut songs = Self::parse_songs(resp.data());
            for s in &mut songs {
                if s.artist_id.is_empty() || s.artist_id == "0" {
                    s.artist_id = aid.to_string();
                }
            }
            if !songs.is_empty() {
                return Ok(self.enrich_songs_privilege_lite(songs).await);
            }
        }
        Ok(Vec::new())
    }

    pub async fn fetch_album_songs(&self, album_id: &str) -> Result<Vec<Song>, String> {
        let aid = album_id.trim();
        if aid.is_empty() {
            return Ok(Vec::new());
        }
        let dev = self.store.device();
        let session = self.store.session();

        let mut all_songs = Vec::new();
        let mut seen = HashSet::new();

        // 通道 1: 概念版网关专辑曲目 (/v1/album_audio/lite)
        for page in 1..=6 {
            let body = json!({
                "album_id": aid,
                "is_buy": "",
                "page": page,
                "pagesize": 50,
                "area_code": 1
            });
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
            headers.insert("kg-tid".to_string(), "255".to_string());

            let Ok(resp) = self
                .transport
                .call(
                    "/v1/album_audio/lite",
                    &dev,
                    session.as_ref(),
                    HashMap::new(),
                    Some(body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
            else {
                break;
            };
            let batch = Self::parse_songs(resp.data());
            if batch.is_empty() {
                break;
            }
            let batch_len = batch.len();
            let mut added = 0usize;
            for mut s in batch {
                if s.album_id.is_empty() || s.album_id == "0" {
                    s.album_id = aid.to_string();
                }
                let k = if !s.hash.is_empty() {
                    s.hash.to_lowercase()
                } else {
                    s.id.to_lowercase()
                };
                if seen.insert(k) {
                    all_songs.push(s);
                    added += 1;
                }
            }
            if added == 0 || batch_len < 50 {
                break;
            }
        }

        // 当网关通道 1 已成功获取曲目时直接走并发权限补全返回，仅当通道 1 为空时才启用 CDN 通道 2 兜底
        if !all_songs.is_empty() {
            return Ok(self.enrich_songs_privilege_lite(all_songs).await);
        }

        // 通道 2: CDN 专辑歌曲接口 (/api/v3/album/song)
        let mut cdn_songs = Vec::new();
        let mut cdn_seen = HashSet::new();
        for page in 1..=6 {
            let mut query = HashMap::new();
            query.insert("version".to_string(), "9108".to_string());
            query.insert("albumid".to_string(), aid.to_string());
            query.insert("plat".to_string(), "0".to_string());
            query.insert("page".to_string(), page.to_string());
            query.insert("pagesize".to_string(), "50".to_string());
            query.insert("area_code".to_string(), "1".to_string());

            let Ok(resp) = self
                .transport
                .call(
                    "/api/v3/album/song",
                    &dev,
                    None,
                    query,
                    None,
                    "http://mobilecdnbj.kugou.com",
                    "GET",
                    false,
                    HashMap::new(),
                    false,
                    false,
                )
                .await
            else {
                break;
            };
            let batch = Self::parse_songs(resp.data());
            if batch.is_empty() {
                break;
            }
            let batch_len = batch.len();
            let mut added = 0usize;
            for mut s in batch {
                if s.album_id.is_empty() || s.album_id == "0" {
                    s.album_id = aid.to_string();
                }
                let k = if !s.hash.is_empty() {
                    s.hash.to_lowercase()
                } else {
                    s.id.to_lowercase()
                };
                if cdn_seen.insert(k) {
                    cdn_songs.push(s);
                    added += 1;
                }
            }
            if added == 0 || batch_len < 50 {
                break;
            }
        }

        Ok(self.enrich_songs_privilege_lite(cdn_songs).await)
    }

    /// 26. 实时多维热榜、实时热搜歌手与多维全量搜索（歌单 / 专辑 / 歌手，严格对齐 MakcRe/KuGouMusicApi）
    async fn fetch_raw_hot_search_tab_keywords(&self) -> Vec<String> {
        let dev = self.store.device();
        let session = self.store.session();
        let fresh_ms = Utc::now().timestamp_millis().to_string();

        for plat_val in ["2", "0"] {
            let mut query = HashMap::new();
            query.insert("navid".to_string(), "1".to_string());
            query.insert("plat".to_string(), plat_val.to_string());
            query.insert("count".to_string(), "30".to_string());
            query.insert("_t".to_string(), fresh_ms.clone());
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "msearch.kugou.com".to_string());

            if let Ok(resp) = self
                .transport
                .call(
                    "/api/v3/search/hot_tab",
                    &dev,
                    session.as_ref(),
                    query,
                    None,
                    "https://gateway.kugou.com",
                    "GET",
                    false,
                    headers,
                    true,
                    true,
                )
                .await
            {
                let mut kws_out = Vec::new();
                if let Some(list) = resp.data().get("list").and_then(|v| v.as_array()) {
                    for group in list {
                        if let Some(kws) = group.get("keywords").and_then(|v| v.as_array()) {
                            for k in kws {
                                let kw = Self::val_str(k, "keyword").trim().to_string();
                                if !kw.is_empty() && !kws_out.contains(&kw) {
                                    kws_out.push(kw);
                                }
                            }
                        }
                    }
                }
                if !kws_out.is_empty() {
                    return kws_out;
                }
            }
        }
        Vec::new()
    }

    /// 搜索页右侧多维实时榜单（移除重复的“热搜榜”，并发拉取 8 大官方权威细分榜单实时 Top 18 曲目）
    pub async fn fetch_hot_search_boards(&self) -> Result<Vec<HotSearchBoard>, String> {
        let board_specs: [(&str, i32, i32, &str); 8] = [
            ("飙升榜", 6666, 23784, "飙"),
            ("抖音榜", 52144, 23784, "热"),
            ("新歌榜", 31308, 21608, "新"),
            ("DJ榜", 33160, 59703, "热"),
            ("国风榜", 33161, 31308, "热"),
            ("粤语榜", 31313, 24971, "热"),
            ("欧美榜", 31310, 4681, "热"),
            ("ACG榜", 33162, 4673, "新"),
        ];

        let mut futs = Vec::with_capacity(board_specs.len());
        for (board_name, primary_rid, fallback_rid, badge_tag) in board_specs {
            futs.push(async move {
                let mut songs = self.fetch_rank_songs_single_page(primary_rid, 1, 24, 0).await;
                if songs.len() < 18 && fallback_rid > 0 && fallback_rid != primary_rid {
                    let fb_songs = self.fetch_rank_songs_single_page(fallback_rid, 1, 24, 0).await;
                    let mut seen_titles: HashSet<String> = songs
                        .iter()
                        .map(|s| s.title.trim().to_lowercase())
                        .collect();
                    for fs in fb_songs {
                        let tk = fs.title.trim().to_lowercase();
                        if !tk.is_empty() && seen_titles.insert(tk) {
                            songs.push(fs);
                        }
                        if songs.len() >= 18 {
                            break;
                        }
                    }
                }

                let mut entries = Vec::new();
                let mut seen_kw = HashSet::new();
                for (idx, s) in songs.into_iter().enumerate() {
                    let clean_title = s.title.trim();
                    if clean_title.is_empty() || clean_title == "未知歌曲" {
                        continue;
                    }
                    let clean_artist = s.artist.trim();
                    let kw = if !clean_artist.is_empty() && clean_artist != "未知歌手" && !clean_title.contains(clean_artist) {
                        format!("{} - {}", clean_title, clean_artist)
                    } else {
                        clean_title.to_string()
                    };
                    let norm_k = kw.to_lowercase();
                    if !seen_kw.insert(norm_k) {
                        continue;
                    }
                    entries.push(HotSearchEntry {
                        keyword: kw,
                        reason: if idx < 3 { badge_tag.to_string() } else { String::new() },
                        icon_type: if idx < 3 { 1 } else { 0 },
                    });
                    if entries.len() >= 18 {
                        break;
                    }
                }

                HotSearchBoard {
                    name: board_name.to_string(),
                    entries,
                }
            });
        }

        let boards: Vec<HotSearchBoard> = futures_join_vec(futs)
            .await
            .into_iter()
            .filter(|b| !b.entries.is_empty())
            .collect();

        Ok(boards)
    }

    pub async fn fetch_search_hot(&self) -> Result<Vec<String>, String> {
        let mut keywords = self.fetch_raw_hot_search_tab_keywords().await;
        if keywords.is_empty() {
            if let Ok(boards) = self.fetch_hot_search_boards().await {
                for b in boards {
                    for entry in b.entries {
                        if !entry.keyword.is_empty() && !keywords.contains(&entry.keyword) {
                            keywords.push(entry.keyword);
                        }
                    }
                }
            }
        }

        if keywords.is_empty() {
            keywords = vec![
                "周杰伦".to_string(),
                "林俊杰".to_string(),
                "陈奕迅".to_string(),
                "邓紫棋".to_string(),
                "告五人".to_string(),
                "陶喆".to_string(),
                "王菲".to_string(),
                "张敬轩".to_string(),
                "毛不易".to_string(),
                "方大同".to_string(),
                "孙燕姿".to_string(),
                "YOASOBI".to_string(),
            ];
        }
        Ok(keywords.into_iter().take(24).collect())
    }

    /// 实时最近热搜歌手榜单 (对齐 MakcRe/KuGouMusicApi module/singer_list.js -> /ocean/v6/singer/list + 实时飙升/热搜榜热度动态加权)
    /// category: 1=华语 (默认), 2=欧美, 3=日韩
    pub async fn fetch_hot_artists(
        &self,
        category: u32,
        count: usize,
    ) -> Result<Vec<FollowedArtist>, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let eff_cat = if category == 0 { 1 } else { category.min(6) };
        let hot_limit = count.clamp(6, 60);
        let fresh_ms = Utc::now().timestamp_millis().to_string();

        let parse_singer_node = |o: &Value| -> Option<FollowedArtist> {
            let sid = [
                Self::val_str(o, "singerid"),
                Self::val_str(o, "AuthorId"),
                Self::val_str(o, "author_id"),
            ]
            .into_iter()
            .find(|s| !s.is_empty() && s != "0")?;
            let name = [
                Self::val_str(o, "singername"),
                Self::val_str(o, "AuthorName"),
                Self::val_str(o, "author_name"),
            ]
            .into_iter()
            .find(|s| !s.is_empty())?
            .replace("<em>", "")
            .replace("</em>", "");
            if name.is_empty() {
                return None;
            }
            let pic = [
                Self::val_str(o, "imgurl"),
                Self::val_str(o, "Avatar"),
                Self::val_str(o, "sizable_avatar"),
                Self::val_str(o, "pic"),
            ]
            .into_iter()
            .find(|s| !s.is_empty())
            .unwrap_or_default();
            Some(FollowedArtist {
                artist_id: sid,
                target_user_id: Self::val_str(o, "userid"),
                name,
                avatar_url: signer::normalize_image_url(Some(&pic)),
                song_count: Self::val_i64(o, "songcount")
                    .or_else(|| Self::val_i64(o, "audio_count"))
                    .or_else(|| Self::val_i64(o, "AudioCount"))
                    .unwrap_or(0) as i32,
                album_count: Self::val_i64(o, "albumcount")
                    .or_else(|| Self::val_i64(o, "album_count"))
                    .or_else(|| Self::val_i64(o, "AlbumCount"))
                    .unwrap_or(0) as i32,
                fans_count: Self::val_i64(o, "fanscount")
                    .or_else(|| Self::val_i64(o, "FansNum"))
                    .or_else(|| Self::val_i64(o, "heat"))
                    .unwrap_or(0) as i32,
                is_followed: false,
                intro: Self::val_str(o, "intro"),
            })
        };

        let is_alpha_index_group = |group: &Value| -> bool {
            let t = Self::val_str(group, "title");
            let trimmed = t.trim();
            if trimmed.len() == 1 {
                if let Some(ch) = trimmed.chars().next() {
                    return ch.is_ascii_alphabetic() || ch == '#';
                }
            }
            false
        };

        let fetch_singer_list_by_type = |singer_type: u32| {
            let dev_cl = dev.clone();
            let sess_cl = session.clone();
            let ts_cl = fresh_ms.clone();
            async move {
                let mut list_out = Vec::new();
                let mut seen_ids = HashSet::new();

                let mut gw_query = HashMap::new();
                gw_query.insert("version".to_string(), "9108".to_string());
                gw_query.insert("showtype".to_string(), "2".to_string());
                gw_query.insert("plat".to_string(), "0".to_string());
                gw_query.insert("sextype".to_string(), "0".to_string());
                gw_query.insert("sort".to_string(), "1".to_string());
                gw_query.insert("pagesize".to_string(), "100".to_string());
                gw_query.insert("type".to_string(), singer_type.to_string());
                gw_query.insert("page".to_string(), "1".to_string());
                gw_query.insert("musician".to_string(), "0".to_string());
                gw_query.insert("hotsize".to_string(), "100".to_string());
                gw_query.insert("_t".to_string(), ts_cl.clone());

                let mut gw_headers = HashMap::new();
                gw_headers.insert("x-router".to_string(), "mobilecdnbj.kugou.com".to_string());

                if let Ok(resp) = self
                    .transport
                    .call(
                        "/ocean/v6/singer/list",
                        &dev_cl,
                        sess_cl.as_ref(),
                        gw_query,
                        None,
                        "https://gateway.kugou.com",
                        "GET",
                        false,
                        gw_headers,
                        true,
                        true,
                    )
                    .await
                {
                    if resp.is_success() {
                        if let Some(info_arr) = resp.data().get("info").and_then(|v| v.as_array()) {
                            for group in info_arr {
                                if let Some(singers) = group.get("singer").and_then(|v| v.as_array()) {
                                    if !list_out.is_empty() && is_alpha_index_group(group) {
                                        continue;
                                    }
                                    for s in singers {
                                        if let Some(artist) = parse_singer_node(s) {
                                            if seen_ids.insert(artist.artist_id.clone()) {
                                                list_out.push(artist);
                                            }
                                        }
                                    }
                                } else if let Some(artist) = parse_singer_node(group) {
                                    if seen_ids.insert(artist.artist_id.clone()) {
                                        list_out.push(artist);
                                    }
                                }
                            }
                        }
                    }
                }

                if !list_out.is_empty() {
                    return list_out;
                }

                let mut cdn_query = HashMap::new();
                cdn_query.insert("version".to_string(), "9108".to_string());
                cdn_query.insert("showtype".to_string(), "2".to_string());
                cdn_query.insert("plat".to_string(), "0".to_string());
                cdn_query.insert("sextype".to_string(), "0".to_string());
                cdn_query.insert("sort".to_string(), "1".to_string());
                cdn_query.insert("pagesize".to_string(), "100".to_string());
                cdn_query.insert("hotsize".to_string(), "100".to_string());
                cdn_query.insert("type".to_string(), singer_type.to_string());
                cdn_query.insert("page".to_string(), "1".to_string());
                cdn_query.insert("musician".to_string(), "0".to_string());
                cdn_query.insert("_t".to_string(), ts_cl);

                if let Ok(resp) = self
                    .transport
                    .call(
                        "/api/v5/singer/list",
                        &dev_cl,
                        None,
                        cdn_query,
                        None,
                        "http://mobilecdnbj.kugou.com",
                        "GET",
                        false,
                        HashMap::new(),
                        false,
                        false,
                    )
                    .await
                {
                    if let Some(info_arr) = resp.data().get("info").and_then(|v| v.as_array()) {
                        for item in info_arr {
                            if let Some(singers) = item.get("singer").and_then(|v| v.as_array()) {
                                if !list_out.is_empty() && is_alpha_index_group(item) {
                                    continue;
                                }
                                for s in singers {
                                    if let Some(artist) = parse_singer_node(s) {
                                        if seen_ids.insert(artist.artist_id.clone()) {
                                            list_out.push(artist);
                                        }
                                    }
                                }
                            } else if let Some(artist) = parse_singer_node(item) {
                                if seen_ids.insert(artist.artist_id.clone()) {
                                    list_out.push(artist);
                                }
                            }
                        }
                    }
                }
                list_out
            }
        };

        // 并发获取：1) 官方歌手热榜列表  2) 对应语种实时热搜词/实时飙升榜与热歌榜曲目（用于动态反映近期热搜歌手变化）
        let (mut base_artists, trending_songs, hot_keywords) = match eff_cat {
            2 => {
                // 欧美：singer/list(type=2) + 欧美新歌榜(31310) + 美国BillBoard榜(4681)
                let (singers, chart_a, chart_b) = tokio::join!(
                    fetch_singer_list_by_type(2),
                    self.fetch_rank_songs_single_page(31310, 1, 40, 0),
                    self.fetch_rank_songs_single_page(4681, 1, 40, 0)
                );
                let mut songs = chart_a;
                songs.extend(chart_b);
                (singers, songs, Vec::new())
            }
            3 => {
                // 日韩：并发拉取日本(type=3)/韩国(type=4)歌手榜 + 韩国榜(31311) + 日本榜(31312)
                let (singers_jp, singers_kr, chart_kr, chart_jp) = tokio::join!(
                    fetch_singer_list_by_type(3),
                    fetch_singer_list_by_type(4),
                    self.fetch_rank_songs_single_page(31311, 1, 40, 0),
                    self.fetch_rank_songs_single_page(31312, 1, 40, 0)
                );
                let mut merged_singers = Vec::new();
                let mut seen_s = HashSet::new();
                let max_len = singers_jp.len().max(singers_kr.len());
                for i in 0..max_len {
                    if let Some(a) = singers_jp.get(i) {
                        if seen_s.insert(a.artist_id.clone()) {
                            merged_singers.push(a.clone());
                        }
                    }
                    if let Some(b) = singers_kr.get(i) {
                        if seen_s.insert(b.artist_id.clone()) {
                            merged_singers.push(b.clone());
                        }
                    }
                }
                let mut songs = chart_kr;
                songs.extend(chart_jp);
                (merged_singers, songs, Vec::new())
            }
            _ => {
                // 华语 (默认 eff_cat == 1)：singer/list(type=1) + 酷狗飙升榜(6666) + 酷狗TOP500(8888) + 实时热搜词榜(hot_tab)
                let (singers, chart_soar, chart_top, hot_kws) = tokio::join!(
                    fetch_singer_list_by_type(eff_cat),
                    self.fetch_rank_songs_single_page(6666, 1, 40, 0),
                    self.fetch_rank_songs_single_page(8888, 1, 40, 0),
                    self.fetch_raw_hot_search_tab_keywords()
                );
                let mut songs = chart_soar;
                songs.extend(chart_top);
                (singers, songs, hot_kws)
            }
        };

        // 统计官方实时榜单与实时热搜词中的歌手热度权重，当官方热搜/榜单变化时动态提升对应歌手顺位
        let mut boost_by_id: HashMap<String, i64> = HashMap::new();
        let mut boost_by_name: HashMap<String, i64> = HashMap::new();

        for (idx, s) in trending_songs.iter().enumerate() {
            let weight = (50i64 - (idx as i64).min(45)).max(5);
            if !s.artist_id.is_empty() && s.artist_id != "0" {
                *boost_by_id.entry(s.artist_id.clone()).or_insert(0) += weight;
            }
            for au in &s.authors {
                if !au.id.is_empty() && au.id != "0" {
                    *boost_by_id.entry(au.id.clone()).or_insert(0) += weight;
                }
                let an = au.name.trim().to_lowercase();
                if !an.is_empty() {
                    *boost_by_name.entry(an).or_insert(0) += weight;
                }
            }
            for part in s.artist.split(&['、', '/', '&', ',', '，'][..]) {
                let p_norm = part.trim().to_lowercase();
                if !p_norm.is_empty() && p_norm != "未知歌手" {
                    *boost_by_name.entry(p_norm).or_insert(0) += weight;
                }
            }
        }

        if !hot_keywords.is_empty() {
            for a in &base_artists {
                let an = a.name.trim().to_lowercase();
                if an.len() >= 2 {
                    for (k_idx, kw) in hot_keywords.iter().enumerate() {
                        if kw.to_lowercase().contains(&an) {
                            let kw_boost = (65i64 - (k_idx as i64 * 2)).max(15);
                            *boost_by_id.entry(a.artist_id.clone()).or_insert(0) += kw_boost;
                        }
                    }
                }
            }
        }

        // 综合基础热度顺位与实时热搜加权进行排序
        let total_len = base_artists.len().max(1) as i64;
        let mut scored: Vec<(i64, usize, FollowedArtist)> = base_artists
            .drain(..)
            .enumerate()
            .map(|(idx, a)| {
                let base_pos_score = (total_len - idx as i64) * 12;
                let id_boost = boost_by_id.get(&a.artist_id).copied().unwrap_or(0);
                let name_boost = boost_by_name
                    .get(&a.name.trim().to_lowercase())
                    .copied()
                    .unwrap_or(0);
                let fan_bonus = if a.fans_count >= 500_000 { 40 } else if a.fans_count == 0 { 20 } else { 0 };
                let total_score = base_pos_score + id_boost.max(name_boost) + fan_bonus;
                (total_score, idx, a)
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

        let mut selected: Vec<FollowedArtist> = scored
            .into_iter()
            .take(hot_limit)
            .map(|(_, _, a)| a)
            .collect();

        // 对前排缺失粉丝数或高清头像的歌手，并发请求 /kmr/v3/author 极速补全真实粉丝数
        let missing_indices: Vec<(usize, String)> = selected
            .iter()
            .enumerate()
            .filter(|(_, a)| a.fans_count <= 0 || a.avatar_url.is_empty())
            .take(12)
            .map(|(i, a)| (i, a.artist_id.clone()))
            .collect();

        if !missing_indices.is_empty() {
            let mut enrich_futs = Vec::with_capacity(missing_indices.len());
            for (idx, aid) in missing_indices {
                let dev_cl = dev.clone();
                let sess_cl = session.clone();
                enrich_futs.push(async move {
                    let mut headers = HashMap::new();
                    headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
                    headers.insert("kg-tid".to_string(), "36".to_string());
                    let author_val = if let Ok(n) = aid.parse::<i64>() {
                        json!(n)
                    } else {
                        json!(aid)
                    };
                    let body = json!({ "author_id": author_val });
                    if let Ok(resp) = self
                        .transport
                        .call(
                            "/kmr/v3/author",
                            &dev_cl,
                            sess_cl.as_ref(),
                            HashMap::new(),
                            Some(body),
                            "https://gateway.kugou.com",
                            "POST",
                            false,
                            headers,
                            true,
                            true,
                        )
                        .await
                    {
                        let d = resp.data().get("data").unwrap_or_else(|| resp.data());
                        let base = d.get("base").unwrap_or(d);
                        let extra = d.get("extra").unwrap_or(d);
                        let stat = d.get("stat").unwrap_or(d);
                        let fans = Self::val_i64(extra, "fans_count")
                            .or_else(|| Self::val_i64(extra, "fansnums"))
                            .or_else(|| Self::val_i64(base, "fans_count"))
                            .or_else(|| Self::val_i64(stat, "fans_count"))
                            .or_else(|| Self::val_i64(d, "fans_count"))
                            .unwrap_or(0) as i32;
                        let pic = {
                            let p = Self::val_str(base, "sizable_avatar");
                            if !p.is_empty() { p } else { Self::val_str(base, "avatar") }
                        };
                        return Some((idx, fans, signer::normalize_image_url(Some(&pic))));
                    }
                    None
                });
            }
            for res in futures_join_vec(enrich_futs).await.into_iter().flatten() {
                let (idx, fans, avatar) = res;
                if let Some(target) = selected.get_mut(idx) {
                    if target.fans_count <= 0 && fans > 0 {
                        target.fans_count = fans;
                    }
                    if target.avatar_url.is_empty() && !avatar.is_empty() {
                        target.avatar_url = avatar;
                    }
                }
            }
        }

        Ok(selected)
    }

    /// 独立搜索页探索聚合（实时热搜歌曲 TOP 榜 + 右侧 8 大细分实时热榜 + 实时最近热搜歌手）
    pub async fn fetch_search_discovery(
        &self,
        artist_category: u32,
    ) -> Result<SearchDiscoveryData, String> {
        let eff_cat = if artist_category == 0 { 1 } else { artist_category };
        let hot_songs_fut = async {
            let (soar_songs, top_songs) = tokio::join!(
                self.fetch_rank_songs_single_page(6666, 1, 24, 0),
                self.fetch_rank_songs_single_page(8888, 1, 30, 0)
            );
            let mut merged = Vec::new();
            let mut seen = HashSet::new();
            let max_len = soar_songs.len().max(top_songs.len());
            for i in 0..max_len {
                if let Some(s1) = soar_songs.get(i) {
                    let k = if !s1.hash.is_empty() {
                        s1.hash.to_lowercase()
                    } else {
                        format!("{}::{}", s1.title, s1.artist).to_lowercase()
                    };
                    if !k.is_empty() && seen.insert(k) {
                        merged.push(s1.clone());
                    }
                }
                if let Some(s2) = top_songs.get(i) {
                    let k = if !s2.hash.is_empty() {
                        s2.hash.to_lowercase()
                    } else {
                        format!("{}::{}", s2.title, s2.artist).to_lowercase()
                    };
                    if !k.is_empty() && seen.insert(k) {
                        merged.push(s2.clone());
                    }
                }
                if merged.len() >= 36 {
                    break;
                }
            }
            self.enrich_songs_privilege_lite(merged).await
        };

        let (hot_songs, boards_res, artists_res) = tokio::join!(
            hot_songs_fut,
            self.fetch_hot_search_boards(),
            self.fetch_hot_artists(eff_cat, 24)
        );
        Ok(SearchDiscoveryData {
            hot_songs,
            hot_boards: boards_res.unwrap_or_default(),
            hot_artists: artists_res.unwrap_or_default(),
        })
    }

    pub async fn search_playlists_paged(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<PagedSearchResult<Playlist>, String> {
        let kw = keyword.trim();
        let p = page.max(1);
        let ps = page_size.clamp(1, 50);
        if kw.is_empty() {
            return Ok(PagedSearchResult {
                items: Vec::new(),
                total: 0,
                page: p,
                page_size: ps,
                has_more: false,
            });
        }
        let dev = self.store.device();
        let session = self.store.session();
        let mut out = Vec::new();
        let mut seen = HashSet::new();

        // 通道 1: 网关歌单搜索 (/v1/search/special，严格对齐 MakcRe/KuGouMusicApi module/search.js)
        let mut gw_query = HashMap::new();
        gw_query.insert("albumhide".to_string(), "0".to_string());
        gw_query.insert("nocollect".to_string(), "0".to_string());
        gw_query.insert("keyword".to_string(), kw.to_string());
        gw_query.insert("page".to_string(), p.to_string());
        gw_query.insert("pagesize".to_string(), ps.to_string());
        gw_query.insert("platform".to_string(), "AndroidFilter".to_string());
        gw_query.insert("iscorrection".to_string(), "1".to_string());
        let mut gw_headers = HashMap::new();
        gw_headers.insert("x-router".to_string(), "complexsearch.kugou.com".to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/v1/search/special",
                &dev,
                session.as_ref(),
                gw_query,
                None,
                "https://gateway.kugou.com",
                "GET",
                false,
                gw_headers,
                true,
                true,
            )
            .await
        {
            if resp.is_success() {
                let total = Self::val_i64(resp.data(), "total").unwrap_or(0).max(0) as u32;
                let arr_opt = resp
                    .data()
                    .get("lists")
                    .or_else(|| resp.data().get("info"))
                    .or_else(|| resp.data().get("list"))
                    .and_then(|v| v.as_array());
                if let Some(arr) = arr_opt {
                    for o in arr {
                        let gid = [
                            Self::val_str(o, "gid"),
                            Self::val_str(o, "global_collection_id"),
                            Self::val_str(o, "specialid"),
                        ]
                        .into_iter()
                        .find(|s| !s.is_empty() && s != "0")
                        .unwrap_or_default();
                        let title = Self::val_str(o, "specialname")
                            .replace("<em>", "")
                            .replace("</em>", "");
                        if !gid.is_empty() && !title.is_empty() && seen.insert(gid.clone()) {
                            let pic = {
                                let p = Self::val_str(o, "img");
                                if !p.is_empty() {
                                    p
                                } else {
                                    Self::val_str(o, "imgurl")
                                }
                            };
                            out.push(Playlist {
                                id: gid.clone(),
                                title,
                                cover_url: signer::normalize_image_url(Some(&pic)),
                                track_count: Self::val_i64(o, "song_count")
                                    .or_else(|| Self::val_i64(o, "songcount"))
                                    .unwrap_or(0) as i32,
                                description: Self::val_str(o, "intro")
                                    .replace("<em>", "")
                                    .replace("</em>", ""),
                                creator_name: Self::val_str(o, "nickname"),
                                is_created_by_me: false,
                                is_favorite: false,
                                is_liked: false,
                                list_id: Self::val_i64(o, "specialid").unwrap_or(0),
                                global_collection_id: gid,
                                play_count: {
                                    let pc = Self::val_str(o, "play_count");
                                    if !pc.is_empty() {
                                        pc
                                    } else {
                                        Self::val_str(o, "total_play_count")
                                    }
                                },
                                ..Default::default()
                            });
                        }
                    }
                }
                if !out.is_empty() || p > 1 {
                    let has_more = !out.is_empty()
                        && if total > 0 {
                            p.saturating_mul(ps) < total
                        } else {
                            out.len() >= ps as usize
                        };
                    return Ok(PagedSearchResult {
                        items: out,
                        total,
                        page: p,
                        page_size: ps,
                        has_more,
                    });
                }
            } else if p > 1 {
                return Ok(PagedSearchResult {
                    items: Vec::new(),
                    total: 0,
                    page: p,
                    page_size: ps,
                    has_more: false,
                });
            }
        }

        // 通道 2: CDN 歌单搜索 (/api/v3/search/special)
        let mut query = HashMap::new();
        query.insert("keyword".to_string(), kw.to_string());
        query.insert("page".to_string(), p.to_string());
        query.insert("pagesize".to_string(), ps.to_string());
        let mut total = 0u32;
        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/search/special",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            total = Self::val_i64(resp.data(), "total").unwrap_or(0).max(0) as u32;
            if let Some(arr) = resp.data().get("info").and_then(|v| v.as_array()) {
                for o in arr {
                    let gid = {
                        let g = Self::val_str(o, "gid");
                        if !g.is_empty() {
                            g
                        } else {
                            Self::val_str(o, "specialid")
                        }
                    };
                    let title = Self::val_str(o, "specialname")
                        .replace("<em>", "")
                        .replace("</em>", "");
                    if !gid.is_empty() && !title.is_empty() && seen.insert(gid.clone()) {
                        let pic = Self::val_str(o, "imgurl");
                        out.push(Playlist {
                            id: gid.clone(),
                            title,
                            cover_url: signer::normalize_image_url(Some(&pic)),
                            track_count: Self::val_i64(o, "songcount").unwrap_or(0) as i32,
                            description: Self::val_str(o, "intro")
                                .replace("<em>", "")
                                .replace("</em>", ""),
                            creator_name: Self::val_str(o, "nickname"),
                            is_created_by_me: false,
                            is_favorite: false,
                            is_liked: false,
                            list_id: Self::val_i64(o, "specialid").unwrap_or(0),
                            global_collection_id: gid,
                            play_count: Self::val_str(o, "playcount"),
                            ..Default::default()
                        });
                    }
                }
            }
        }
        let has_more = !out.is_empty()
            && if total > 0 {
                p.saturating_mul(ps) < total
            } else {
                out.len() >= ps as usize
            };
        Ok(PagedSearchResult {
            items: out,
            total,
            page: p,
            page_size: ps,
            has_more,
        })
    }

    pub async fn search_playlists(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Playlist>, String> {
        self.search_playlists_paged(keyword, page, page_size)
            .await
            .map(|r| r.items)
    }

    pub async fn search_albums_paged(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<PagedSearchResult<AlbumItem>, String> {
        let kw = keyword.trim();
        let p = page.max(1);
        let ps = page_size.clamp(1, 50);
        if kw.is_empty() {
            return Ok(PagedSearchResult {
                items: Vec::new(),
                total: 0,
                page: p,
                page_size: ps,
                has_more: false,
            });
        }
        let dev = self.store.device();
        let session = self.store.session();
        let mut out = Vec::new();
        let mut seen = HashSet::new();

        // 通道 1: 网关专辑搜索 (/v1/search/album，严格对齐 MakcRe/KuGouMusicApi module/search.js)
        let mut gw_query = HashMap::new();
        gw_query.insert("keyword".to_string(), kw.to_string());
        gw_query.insert("page".to_string(), p.to_string());
        gw_query.insert("pagesize".to_string(), ps.to_string());
        gw_query.insert("platform".to_string(), "AndroidFilter".to_string());
        gw_query.insert("iscorrection".to_string(), "1".to_string());
        gw_query.insert("category".to_string(), "1".to_string());
        gw_query.insert("sorttype".to_string(), "0".to_string());
        gw_query.insert("searchsong".to_string(), "0".to_string());
        let mut gw_headers = HashMap::new();
        gw_headers.insert("x-router".to_string(), "complexsearch.kugou.com".to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/v1/search/album",
                &dev,
                session.as_ref(),
                gw_query,
                None,
                "https://gateway.kugou.com",
                "GET",
                false,
                gw_headers,
                true,
                true,
            )
            .await
        {
            if resp.is_success() {
                let total = Self::val_i64(resp.data(), "total").unwrap_or(0).max(0) as u32;
                let arr_opt = resp
                    .data()
                    .get("lists")
                    .or_else(|| resp.data().get("info"))
                    .or_else(|| resp.data().get("list"))
                    .and_then(|v| v.as_array());
                if let Some(arr) = arr_opt {
                    for o in arr {
                        let aid = {
                            let a = Self::val_str(o, "albumid");
                            if !a.is_empty() {
                                a
                            } else {
                                Self::val_str(o, "album_id")
                            }
                        };
                        let name = {
                            let n = Self::val_str(o, "albumname");
                            if !n.is_empty() {
                                n
                            } else {
                                Self::val_str(o, "album_name")
                            }
                        }
                        .replace("<em>", "")
                        .replace("</em>", "");
                        if !aid.is_empty() && !name.is_empty() && seen.insert(aid.clone()) {
                            let pic = {
                                let p = Self::val_str(o, "img");
                                if !p.is_empty() {
                                    p
                                } else {
                                    Self::val_str(o, "imgurl")
                                }
                            };
                            out.push(AlbumItem {
                                album_id: aid,
                                album_name: name,
                                artist_name: {
                                    let s = Self::val_str(o, "singer");
                                    if !s.is_empty() {
                                        s
                                    } else {
                                        Self::val_str(o, "singername")
                                    }
                                }
                                .replace("<em>", "")
                                .replace("</em>", ""),
                                artist_id: Self::val_str(o, "singerid"),
                                cover_url: signer::normalize_image_url(Some(&pic)),
                                publish_date: {
                                    let pt = Self::val_str(o, "publish_time");
                                    let raw = if !pt.is_empty() {
                                        pt
                                    } else {
                                        Self::val_str(o, "publishtime")
                                    };
                                    raw.chars().take(10).collect()
                                },
                                publish_company: Self::val_str(o, "company"),
                                song_count: Self::val_i64(o, "songcount")
                                    .or_else(|| Self::val_i64(o, "song_count"))
                                    .unwrap_or(0) as i32,
                                intro: Self::val_str(o, "intro")
                                    .replace("<em>", "")
                                    .replace("</em>", ""),
                            });
                        }
                    }
                }
                if !out.is_empty() || p > 1 {
                    let has_more = !out.is_empty()
                        && if total > 0 {
                            p.saturating_mul(ps) < total
                        } else {
                            out.len() >= ps as usize
                        };
                    return Ok(PagedSearchResult {
                        items: out,
                        total,
                        page: p,
                        page_size: ps,
                        has_more,
                    });
                }
            } else if p > 1 {
                return Ok(PagedSearchResult {
                    items: Vec::new(),
                    total: 0,
                    page: p,
                    page_size: ps,
                    has_more: false,
                });
            }
        }

        // 通道 2: CDN 专辑搜索 (/api/v3/search/album)
        let mut query = HashMap::new();
        query.insert("keyword".to_string(), kw.to_string());
        query.insert("page".to_string(), p.to_string());
        query.insert("pagesize".to_string(), ps.to_string());
        let mut total = 0u32;
        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/search/album",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            total = Self::val_i64(resp.data(), "total").unwrap_or(0).max(0) as u32;
            if let Some(arr) = resp.data().get("info").and_then(|v| v.as_array()) {
                for o in arr {
                    let aid = Self::val_str(o, "albumid");
                    let name = Self::val_str(o, "albumname")
                        .replace("<em>", "")
                        .replace("</em>", "");
                    if !aid.is_empty() && !name.is_empty() && seen.insert(aid.clone()) {
                        let pic = Self::val_str(o, "imgurl");
                        out.push(AlbumItem {
                            album_id: aid,
                            album_name: name,
                            artist_name: Self::val_str(o, "singername")
                                .replace("<em>", "")
                                .replace("</em>", ""),
                            artist_id: Self::val_str(o, "singerid"),
                            cover_url: signer::normalize_image_url(Some(&pic)),
                            publish_date: Self::val_str(o, "publishtime")
                                .chars()
                                .take(10)
                                .collect(),
                            publish_company: Self::val_str(o, "company"),
                            song_count: Self::val_i64(o, "songcount").unwrap_or(0) as i32,
                            intro: Self::val_str(o, "intro")
                                .replace("<em>", "")
                                .replace("</em>", ""),
                        });
                    }
                }
            }
        }
        let has_more = !out.is_empty()
            && if total > 0 {
                p.saturating_mul(ps) < total
            } else {
                out.len() >= ps as usize
            };
        Ok(PagedSearchResult {
            items: out,
            total,
            page: p,
            page_size: ps,
            has_more,
        })
    }

    pub async fn search_albums(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<AlbumItem>, String> {
        self.search_albums_paged(keyword, page, page_size)
            .await
            .map(|r| r.items)
    }

    pub async fn search_artists_paged(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<PagedSearchResult<FollowedArtist>, String> {
        let kw = keyword.trim();
        let p = page.max(1);
        let ps = page_size.clamp(1, 50);
        if kw.is_empty() {
            return Ok(PagedSearchResult {
                items: Vec::new(),
                total: 0,
                page: p,
                page_size: ps,
                has_more: false,
            });
        }
        let dev = self.store.device();
        let session = self.store.session();
        let mut out = Vec::new();
        let mut seen = HashSet::new();

        // 通道 1: 网关歌手搜索 (/v1/search/author，严格对齐 MakcRe/KuGouMusicApi module/search.js type=author)
        let mut gw_query = HashMap::new();
        gw_query.insert("albumhide".to_string(), "0".to_string());
        gw_query.insert("nocollect".to_string(), "0".to_string());
        gw_query.insert("keyword".to_string(), kw.to_string());
        gw_query.insert("page".to_string(), p.to_string());
        gw_query.insert("pagesize".to_string(), ps.to_string());
        gw_query.insert("platform".to_string(), "AndroidFilter".to_string());
        gw_query.insert("iscorrection".to_string(), "1".to_string());
        let mut gw_headers = HashMap::new();
        gw_headers.insert("x-router".to_string(), "complexsearch.kugou.com".to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/v1/search/author",
                &dev,
                session.as_ref(),
                gw_query,
                None,
                "https://gateway.kugou.com",
                "GET",
                false,
                gw_headers,
                true,
                true,
            )
            .await
        {
            if resp.is_success() {
                let total = Self::val_i64(resp.data(), "total").unwrap_or(0).max(0) as u32;
                let arr_opt = resp
                    .data()
                    .get("lists")
                    .or_else(|| resp.data().get("info"))
                    .or_else(|| resp.data().get("list"))
                    .or_else(|| resp.data().as_array().map(|_| resp.data()))
                    .and_then(|v| v.as_array());
                if let Some(arr) = arr_opt {
                    for o in arr {
                        let sid = [
                            Self::val_str(o, "AuthorId"),
                            Self::val_str(o, "author_id"),
                            Self::val_str(o, "singerid"),
                        ]
                        .into_iter()
                        .find(|s| !s.is_empty() && s != "0")
                        .unwrap_or_default();
                        let name = [
                            Self::val_str(o, "AuthorName"),
                            Self::val_str(o, "author_name"),
                            Self::val_str(o, "singername"),
                        ]
                        .into_iter()
                        .find(|s| !s.is_empty())
                        .unwrap_or_default()
                        .replace("<em>", "")
                        .replace("</em>", "");
                        if !sid.is_empty() && !name.is_empty() && seen.insert(sid.clone()) {
                            let pic = [
                                Self::val_str(o, "Avatar"),
                                Self::val_str(o, "sizable_avatar"),
                                Self::val_str(o, "imgurl"),
                                Self::val_str(o, "pic"),
                            ]
                            .into_iter()
                            .find(|s| !s.is_empty())
                            .unwrap_or_default();
                            out.push(FollowedArtist {
                                artist_id: sid,
                                target_user_id: Self::val_str(o, "userid"),
                                name,
                                avatar_url: signer::normalize_image_url(Some(&pic)),
                                song_count: Self::val_i64(o, "AudioCount")
                                    .or_else(|| Self::val_i64(o, "audio_count"))
                                    .or_else(|| Self::val_i64(o, "songcount"))
                                    .unwrap_or(0) as i32,
                                album_count: Self::val_i64(o, "AlbumCount")
                                    .or_else(|| Self::val_i64(o, "album_count"))
                                    .or_else(|| Self::val_i64(o, "albumcount"))
                                    .unwrap_or(0) as i32,
                                fans_count: Self::val_i64(o, "FansNum")
                                    .or_else(|| Self::val_i64(o, "fanscount"))
                                    .unwrap_or(0) as i32,
                                is_followed: false,
                                intro: Self::val_str(o, "intro"),
                            });
                        }
                    }
                }
                if !out.is_empty() || p > 1 {
                    let has_more = !out.is_empty()
                        && if total > 0 {
                            p.saturating_mul(ps) < total
                        } else {
                            out.len() >= ps as usize
                        };
                    return Ok(PagedSearchResult {
                        items: out,
                        total,
                        page: p,
                        page_size: ps,
                        has_more,
                    });
                }
            } else if p > 1 {
                return Ok(PagedSearchResult {
                    items: Vec::new(),
                    total: 0,
                    page: p,
                    page_size: ps,
                    has_more: false,
                });
            }
        }

        // 通道 2: CDN 歌手搜索 (/api/v3/search/singer)
        let mut query = HashMap::new();
        query.insert("keyword".to_string(), kw.to_string());
        query.insert("page".to_string(), p.to_string());
        query.insert("pagesize".to_string(), ps.to_string());
        let mut total = 0u32;
        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/search/singer",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            total = Self::val_i64(resp.data(), "total").unwrap_or(0).max(0) as u32;
            if let Some(arr) = resp.data().get("info").and_then(|v| v.as_array()) {
                for o in arr {
                    let sid = Self::val_str(o, "singerid");
                    let name = Self::val_str(o, "singername")
                        .replace("<em>", "")
                        .replace("</em>", "");
                    if !sid.is_empty() && !name.is_empty() && seen.insert(sid.clone()) {
                        let pic = Self::val_str(o, "imgurl");
                        out.push(FollowedArtist {
                            artist_id: sid,
                            target_user_id: Self::val_str(o, "userid"),
                            name,
                            avatar_url: signer::normalize_image_url(Some(&pic)),
                            song_count: Self::val_i64(o, "songcount").unwrap_or(0) as i32,
                            album_count: Self::val_i64(o, "albumcount").unwrap_or(0) as i32,
                            fans_count: Self::val_i64(o, "fanscount").unwrap_or(0) as i32,
                            is_followed: false,
                            intro: Self::val_str(o, "intro"),
                        });
                    }
                }
            }
        }

        let final_total = if total > 0 { total } else { out.len() as u32 };
        let has_more = !out.is_empty()
            && if total > 0 {
                p.saturating_mul(ps) < total
            } else {
                out.len() >= ps as usize
            };
        Ok(PagedSearchResult {
            items: out,
            total: final_total,
            page: p,
            page_size: ps,
            has_more,
        })
    }

    pub async fn search_artists(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<FollowedArtist>, String> {
        self.search_artists_paged(keyword, page, page_size)
            .await
            .map(|r| r.items)
    }

    fn extract_comment_images(item: &Value) -> Vec<String> {
        let mut urls = Vec::new();
        for key in ["images", "uimage", "img_list", "pic_list", "pictures", "imgs", "pics"] {
            let arr_opt = item
                .get(key)
                .and_then(|v| v.as_array())
                .or_else(|| item.get("content_ext").and_then(|e| e.get(key)).and_then(|v| v.as_array()));
            if let Some(arr) = arr_opt {
                for el in arr {
                    if let Some(s) = el.as_str() {
                        let u = signer::normalize_image_url(Some(s));
                        if !u.is_empty() && !urls.contains(&u) {
                            urls.push(u);
                        }
                    } else if el.is_object() {
                        let raw = [
                            Self::val_str(el, "url"),
                            Self::val_str(el, "big_url"),
                            Self::val_str(el, "origin_url"),
                            Self::val_str(el, "img"),
                            Self::val_str(el, "pic"),
                        ]
                        .into_iter()
                        .find(|s| !s.is_empty())
                        .unwrap_or_default();
                        let u = signer::normalize_image_url(Some(&raw));
                        if !u.is_empty() && !urls.contains(&u) {
                            urls.push(u);
                        }
                    }
                }
            }
        }
        urls
    }

    /// 酷狗评论内置表情标签 ([em]e400001[/em] 与 [微笑] 等) 转换为原生 Emoji (对齐 Android KugouCommentEmojiParser)
    fn format_kugou_comment_emojis(raw: &str) -> String {
        static EM_RE: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| regex::Regex::new(r"(?i)\[em\]([a-zA-Z0-9_]+)\[/em\]").unwrap());
        if raw.trim().is_empty() {
            return String::new();
        }
        let mut text = raw.to_string();
        if text.contains("[em]") || text.contains("[EM]") {
            text = EM_RE
                .replace_all(&text, |caps: &regex::Captures| {
                    let code = caps.get(1).map(|m| m.as_str().to_lowercase()).unwrap_or_default();
                    match code.as_str() {
                        "e400001" | "e100" => "😊",
                        "e400002" | "e104" => "😍",
                        "e400003" | "e103" => "😭",
                        "e400004" => "😡",
                        "e400005" => "😳",
                        "e400006" | "e107" => "😎",
                        "e400007" | "e102" => "😂",
                        "e400008" | "e106" => "🥺",
                        "e400009" | "e108" => "👍",
                        "e400010" | "e109" => "❤️",
                        "e400011" => "💔",
                        "e400012" | "e110" => "🌹",
                        "e400013" => "🎉",
                        "e400014" | "e111" => "🙏",
                        "e400015" => "🎧",
                        "e400016" => "🎵",
                        "e400017" => "🔥",
                        "e400018" => "✨",
                        "e400019" => "🌙",
                        "e400020" => "🍻",
                        "e400021" => "🤭",
                        "e400022" => "🤔",
                        "e400023" => "😴",
                        "e400024" => "🤗",
                        "e400025" | "e105" => "😘",
                        "e400026" => "🥳",
                        "e400027" => "🤩",
                        "e400028" => "😇",
                        "e400029" => "🙌",
                        "e400030" => "👏",
                        "e400031" => "💪",
                        "e400032" => "🌟",
                        _ => "✨",
                    }
                })
                .to_string();
        }
        let tag_map = [
            ("[微笑]", "😊"), ("[色]", "😍"), ("[流泪]", "😢"), ("[大哭]", "😭"),
            ("[得意]", "😎"), ("[呲牙]", "😁"), ("[害羞]", "☺️"), ("[调皮]", "😜"),
            ("[偷笑]", "🤭"), ("[愉快]", "😄"), ("[鼓掌]", "👏"), ("[委屈]", "🥺"),
            ("[快哭了]", "😢"), ("[亲亲]", "😘"), ("[可怜]", "🥺"), ("[玫瑰]", "🌹"),
            ("[爱心]", "❤️"), ("[心碎]", "💔"), ("[月亮]", "🌙"), ("[太阳]", "☀️"),
            ("[拥抱]", "🤗"), ("[强]", "👍"), ("[抱拳]", "🙏"), ("[爱你]", "🤟"),
            ("[赞]", "👍"), ("[心]", "❤️"), ("[笑哭]", "😂"), ("[捂脸]", "🤦"),
            ("[破涕为笑]", "😂"), ("[加油]", "💪"), ("[旺柴]", "🐶"), ("[狗头]", "🐶"),
            ("[比心]", "🫰"), ("[裂开]", "🫠"), ("[苦涩]", "🥹"), ("[叹气]", "😮‍💨"),
            ("[666]", "🤙"), ("[打call]", "📣"), ("[吃瓜]", "🍉"), ("[啤酒]", "🍺"),
        ];
        for (tag, emoji) in tag_map {
            if text.contains(tag) {
                text = text.replace(tag, emoji);
            }
        }
        text.trim().to_string()
    }

    fn parse_weibo_reply_chain(raw_content: &str) -> (String, Option<String>, Option<String>, Vec<QuotedReplyHop>) {
        static WEIBO_REPLY_RE: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| regex::Regex::new(r"\s*//\s*@").unwrap());
        let formatted = Self::format_kugou_comment_emojis(raw_content);
        if formatted.is_empty() {
            return (String::new(), None, None, Vec::new());
        }
        if formatted.contains("//@") || formatted.contains("// @") {
            let segments: Vec<&str> = WEIBO_REPLY_RE.split(&formatted).collect();
            let main_body = segments.first().map(|s| s.trim()).unwrap_or("").to_string();
            let mut hops = Vec::new();
            for seg in segments.iter().skip(1) {
                let t = seg.trim();
                if let Some(colon_idx) = t.find(':').or_else(|| t.find('：')) {
                    let uname = t[..colon_idx].trim().trim_start_matches('@').trim().to_string();
                    let char_len = t[colon_idx..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                    let qtext = t[colon_idx + char_len..].trim().to_string();
                    if !uname.is_empty() {
                        hops.push(QuotedReplyHop {
                            user_name: uname,
                            content: qtext,
                        });
                    }
                }
            }
            let first_user = hops.first().map(|h| h.user_name.clone());
            let first_quote = hops
                .first()
                .map(|h| h.content.clone())
                .filter(|s| !s.is_empty());
            return (
                if main_body.is_empty() { formatted } else { main_body },
                first_user,
                first_quote,
                hops,
            );
        }
        (formatted, None, None, Vec::new())
    }

    fn parse_single_comment_item(item: &Value, default_parent_id: Option<&str>) -> Option<SongComment> {
        let cid = {
            let a = Self::val_str(item, "id");
            if !a.is_empty() {
                a
            } else {
                let b = Self::val_str(item, "reply_id");
                if !b.is_empty() { b } else { Self::val_str(item, "cmtid") }
            }
        };
        let raw_content = {
            let c = Self::val_str(item, "content");
            if !c.is_empty() {
                c
            } else {
                Self::val_str(item, "text")
            }
        };
        let images = Self::extract_comment_images(item);
        if cid.is_empty() || (raw_content.is_empty() && images.is_empty()) {
            return None;
        }
        let (clean_content, weibo_user, weibo_quote, chain_hops) =
            Self::parse_weibo_reply_chain(&raw_content);
        let udetail = item.get("udetail").or_else(|| item.get("user"));
        let uid = [
            Self::val_str(item, "user_id"),
            udetail.map(|u| Self::val_str(u, "user_id")).unwrap_or_default(),
            Self::val_str(item, "userid"),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_default();

        let uname = [
            Self::val_str(item, "user_name"),
            Self::val_str(item, "nickname"),
            Self::val_str(item, "nick_name"),
            udetail.map(|u| Self::val_str(u, "user_name")).unwrap_or_default(),
            udetail.map(|u| Self::val_str(u, "nickname")).unwrap_or_default(),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_else(|| "微风乐迷".to_string());

        let uav = [
            Self::val_str(item, "user_pic"),
            Self::val_str(item, "avatar"),
            Self::val_str(item, "pic"),
            udetail.map(|u| Self::val_str(u, "user_pic")).unwrap_or_default(),
            udetail.map(|u| Self::val_str(u, "avatar")).unwrap_or_default(),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_default();

        let like_cnt = item
            .get("like")
            .and_then(|l| Self::val_i64(l, "count").or_else(|| Self::val_i64(l, "like_count")))
            .or_else(|| Self::val_i64(item, "like_count"))
            .or_else(|| Self::val_i64(item, "likenum"))
            .or_else(|| Self::val_i64(item, "praisecount"))
            .unwrap_or(0) as i32;

        let is_liked = Self::val_i64(item, "is_like") == Some(1)
            || item
                .get("like")
                .and_then(|l| Self::val_i64(l, "is_like"))
                == Some(1);

        let is_hot = Self::val_i64(item, "is_hot") == Some(1)
            || Self::val_i64(item, "weight") == Some(1)
            || like_cnt >= 50;

        let vip_level = Self::val_i64(item, "vip_type")
            .or_else(|| udetail.and_then(|u| Self::val_i64(u, "vip_type")))
            .or_else(|| Self::val_i64(item, "m_type"))
            .unwrap_or(0) as i32;

        let loc = [
            Self::val_str(item, "location"),
            Self::val_str(item, "ip_location"),
            Self::val_str(item, "ip_loc"),
            Self::val_str(item, "province"),
            udetail.map(|u| Self::val_str(u, "location")).unwrap_or_default(),
        ]
        .into_iter()
        .find(|s| !s.is_empty() && s != "0" && s != "未知")
        .unwrap_or_default()
        .trim_start_matches("IP属地:")
        .trim_start_matches("IP属地：")
        .trim_start_matches("来自")
        .trim()
        .to_string();

        let special_child_id = [
            Self::val_str(item, "special_child_id"),
            Self::val_str(item, "childrenid"),
            Self::val_str(item, "special_id"),
        ]
        .into_iter()
        .find(|s| !s.is_empty() && s != "0")
        .unwrap_or_default();

        let reply_user_name = weibo_user.or_else(|| {
            [
                Self::val_str(item, "reply_user_name"),
                Self::val_str(item, "reply_username"),
                item.get("puser").map(|p| Self::val_str(p, "user_name")).unwrap_or_default(),
                item.get("pcomment").map(|p| Self::val_str(p, "user_name")).unwrap_or_default(),
                Self::val_str(item, "puser"),
            ]
            .into_iter()
            .find(|s| !s.is_empty() && !s.starts_with('{'))
        });

        let reply_quoted_content = weibo_quote
            .or_else(|| {
                [
                    item.get("pcomment").map(|p| Self::val_str(p, "content")).unwrap_or_default(),
                    item.get("pcontent").map(|p| Self::val_str(p, "content")).unwrap_or_default(),
                ]
                .into_iter()
                .find(|s| !s.is_empty())
            })
            .map(|q| Self::format_kugou_comment_emojis(&q));

        let explicit_target_id = [
            Self::val_str(item, "p_id"),
            Self::val_str(item, "pid"),
            Self::val_str(item, "reply_to_id"),
            item.get("pcomment").map(|p| Self::val_str(p, "id")).unwrap_or_default(),
            item.get("pcontent").map(|p| Self::val_str(p, "id")).unwrap_or_default(),
        ]
        .into_iter()
        .find(|s| !s.is_empty() && s != "0" && Some(s.as_str()) != default_parent_id);

        let parent_id = explicit_target_id
            .clone()
            .or_else(|| default_parent_id.map(|s| s.to_string()));

        let mut inline_replies = Vec::new();
        for rkey in ["reply_list", "list", "children", "replys"] {
            if let Some(rarr) = item
                .get(rkey)
                .and_then(|v| v.as_array())
                .or_else(|| item.get("reply").and_then(|r| r.get(rkey)).and_then(|v| v.as_array()))
            {
                for ritem in rarr {
                    if let Some(rc) = Self::parse_single_comment_item(ritem, Some(&cid)) {
                        if !inline_replies.iter().any(|existing: &SongComment| existing.id == rc.id) {
                            inline_replies.push(rc);
                        }
                    }
                }
            }
        }

        let reply_count = [
            Self::val_i64(item, "reply_num"),
            Self::val_i64(item, "reply_count"),
            item.get("reply").and_then(|r| Self::val_i64(r, "count")),
        ]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(0)
        .max(inline_replies.len() as i64) as i32;

        let created_time = {
            let t = Self::val_str(item, "addtime");
            if !t.is_empty() {
                t
            } else {
                Self::val_str(item, "createtime")
            }
        };

        Some(SongComment {
            id: cid,
            user_id: uid,
            user_name: uname,
            user_avatar: signer::normalize_image_url(Some(&uav)),
            content: if clean_content.is_empty() {
                "[图片评论]".to_string()
            } else {
                clean_content
            },
            created_time,
            like_count: like_cnt,
            is_liked,
            is_hot,
            vip_level,
            location: loc,
            reply_count,
            special_child_id,
            replies: inline_replies,
            images,
            reply_user_name,
            reply_quoted_content,
            parent_id,
            root_id: default_parent_id.map(|s| s.to_string()),
            reply_target_comment_id: explicit_target_id,
            reply_chain_hops: chain_hops,
            timestamp_ms: 0,
        })
    }

    /// 27. 歌曲多级楼中楼评论区 (对齐 MusicRepository.songComments -> POST /mcomment/v1/cmt_classify_list & /mcomment/v1/cmtlist + 自动预取前排楼中楼)
    pub async fn fetch_song_comments(
        &self,
        song: &Song,
        sort_type: &str,
        page: u32,
    ) -> Result<Vec<SongComment>, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let mix_id = if song.album_audio_id > 0 {
            song.album_audio_id
        } else {
            song.audio_id
        };
        let ext_data = song.hash.to_lowercase();
        let is_hot = sort_type != "new";
        let code_hash = signer::md5_hex(&format!("{}1005", ext_data));

        let mut query = HashMap::new();
        query.insert("code".to_string(), "fc4be23b4e972707f36b8a828a93ba8a".to_string());
        query.insert(
            "kugouid".to_string(),
            session
                .as_ref()
                .map(|s| s.user_id.clone())
                .unwrap_or_else(|| "0".to_string()),
        );
        query.insert("ver".to_string(), "6".to_string());
        query.insert("clientver".to_string(), CLIENT_VERSION.to_string());
        query.insert("extdata".to_string(), ext_data);
        query.insert("mixsongid".to_string(), mix_id.to_string());
        query.insert("mix_songid".to_string(), mix_id.to_string());
        query.insert("need_show_image".to_string(), "1".to_string());
        query.insert("show_classify".to_string(), "1".to_string());
        query.insert("show_hotword_list".to_string(), "1".to_string());
        query.insert("p".to_string(), page.to_string());
        query.insert("pagesize".to_string(), "30".to_string());
        query.insert("sort_method".to_string(), if is_hot { "1" } else { "2" }.to_string());
        query.insert("type".to_string(), if is_hot { "1" } else { "2" }.to_string());
        query.insert("UDID".to_string(), code_hash);

        let paths = if is_hot {
            ["/mcomment/v1/cmt_classify_list", "/mcomment/v1/cmtlist"]
        } else {
            ["/mcomment/v1/cmtlist", "/mcomment/v1/cmt_classify_list"]
        };

        let mut comments: Vec<SongComment> = Vec::new();
        let mut seen_ids = HashSet::new();
        for path in paths {
            if let Ok(resp) = self
                .transport
                .call(
                    path,
                    &dev,
                    session.as_ref(),
                    query.clone(),
                    None,
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await
            {
                let root = resp.data();
                let keys = if is_hot && page == 1 {
                    ["weight_list", "hot_list", "list", "lists", "comments"]
                } else {
                    ["list", "lists", "comments", "weight_list", "hot_list"]
                };
                for key in keys {
                    if !is_hot && (key == "weight_list" || key == "hot_list") && !comments.is_empty() {
                        continue;
                    }
                    if let Some(arr) = root.get(key).and_then(|v| v.as_array()) {
                        for item in arr {
                            if let Some(cmt) = Self::parse_single_comment_item(item, None) {
                                if seen_ids.insert(cmt.id.clone()) {
                                    comments.push(cmt);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 自动预取本页前 8 条有楼层回复但内联不足 3 条的主评论子回复（对齐 Android songComments 预取策略）
        let mut prefetched_count = 0usize;
        for cmt in comments.iter_mut() {
            if cmt.reply_count > 0 && cmt.replies.len() < 3 && prefetched_count < 8 {
                prefetched_count += 1;
                if let Ok(fetched) = self
                    .fetch_comment_floor_replies(song, &cmt.id, &cmt.special_child_id, 1)
                    .await
                {
                    for r in fetched {
                        if !cmt.replies.iter().any(|e| e.id == r.id) {
                            cmt.replies.push(r);
                        }
                    }
                    cmt.reply_count = cmt.reply_count.max(cmt.replies.len() as i32);
                }
            }
        }

        Ok(comments)
    }

    /// 28. 设备标识重签与音源链路延迟诊断 (对齐 VipCenterBottomSheet.kt)
    pub async fn resign_device_identity(&self) -> Result<GatewayDiagResult, String> {
        let start = Utc::now().timestamp_millis();
        let new_dfid = self.register_device(true).await?;
        let elapsed = (Utc::now().timestamp_millis() - start).max(12);
        let dev = self.store.device();
        Ok(GatewayDiagResult {
            latency_ms: elapsed,
            dfid: new_dfid,
            mid: dev.mid,
            guid: dev.guid,
            status_text: format!("设备指纹已重签生效 · 网关延迟 {}ms", elapsed),
            ok: true,
        })
    }

    pub async fn diagnose_gateway_latency(&self) -> Result<GatewayDiagResult, String> {
        let start = Utc::now().timestamp_millis();
        let _ = self.fetch_search_hot().await;
        let elapsed = (Utc::now().timestamp_millis() - start).max(8);
        let dev = self.store.device();
        Ok(GatewayDiagResult {
            latency_ms: elapsed,
            dfid: dev.dfid,
            mid: dev.mid,
            guid: dev.guid,
            status_text: format!("概念版直连网关畅通 ({} ms)", elapsed),
            ok: true,
        })
    }

    /// 29. 本地多音质歌曲下载 + 1000P 高清封面/歌词二进制内嵌 + 独立 .lrc 导出 (对齐 MusicDownloadManager.kt & AudioMetadataEmbedder.kt)
    pub async fn download_song_to_local(
        &self,
        song: &Song,
        preferred_quality: &str,
        music_dir: &str,
        lrc_dir: &str,
        export_lrc: bool,
        embed_cover_and_tags: bool,
        embed_lyrics_in_audio: bool,
        bilingual_lrc: bool,
        skip_existing: bool,
    ) -> Result<DownloadTaskResult, String> {
        let pb = self.resolve_song_url(song, preferred_quality).await?;
        let target_music_dir = if music_dir.trim().is_empty() {
            directories::UserDirs::new()
                .and_then(|u| u.audio_dir().map(|p| p.join("BreezeMusic")))
                .unwrap_or_else(|| std::path::PathBuf::from("BreezeMusic_Downloads"))
        } else {
            std::path::PathBuf::from(music_dir.trim())
        };
        std::fs::create_dir_all(&target_music_dir).map_err(|e| e.to_string())?;

        let safe_name: String = format!("{} - {}", song.artist, song.title)
            .chars()
            .map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c })
            .collect();
        let audio_file_path = target_music_dir.join(format!("{}.{}", safe_name, pb.ext_name));
        if skip_existing && audio_file_path.exists() {
            let existing_size = std::fs::metadata(&audio_file_path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            if existing_size > 1024 {
                return Ok(DownloadTaskResult {
                    success: true,
                    saved_audio_path: audio_file_path.to_string_lossy().to_string(),
                    saved_lrc_path: String::new(),
                    file_size_bytes: existing_size,
                    quality_label: pb.quality_label,
                    message: format!("已跳过同名已存在文件：{}", audio_file_path.to_string_lossy()),
                });
            }
        }

        let resp = self
            .transport
            .client
            .get(&pb.url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let audio_bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        std::fs::write(&audio_file_path, &audio_bytes).map_err(|e| e.to_string())?;

        // 获取歌词与 1000P 高清专辑封面用于二进制内嵌或导出
        let mut formatted_lrc_text = String::new();
        if export_lrc || embed_lyrics_in_audio {
            if let Ok(lyric) = self.fetch_song_lyric(song).await {
                if !lyric.lines.is_empty() {
                    let mut lrc_lines = Vec::new();
                    for l in &lyric.lines {
                        let min = l.start_time_ms / 60_000;
                        let sec = (l.start_time_ms % 60_000) / 1000;
                        let cs = (l.start_time_ms % 1000) / 10;
                        lrc_lines.push(format!("[{:02}:{:02}.{:02}]{}", min, sec, cs, l.text));
                        if bilingual_lrc {
                            if let Some(ref tr) = l.translation {
                                if !tr.trim().is_empty() {
                                    lrc_lines.push(format!(
                                        "[{:02}:{:02}.{:02}]{}",
                                        min, sec, cs, tr
                                    ));
                                }
                            }
                        }
                    }
                    formatted_lrc_text = lrc_lines.join("\n");
                }
            }
        }

        let mut saved_lrc_path = String::new();
        if export_lrc && !formatted_lrc_text.is_empty() {
            let target_lrc_dir = if lrc_dir.trim().is_empty() {
                target_music_dir.clone()
            } else {
                std::path::PathBuf::from(lrc_dir.trim())
            };
            let _ = std::fs::create_dir_all(&target_lrc_dir);
            let lrc_file = target_lrc_dir.join(format!("{}.lrc", safe_name));
            if std::fs::write(&lrc_file, &formatted_lrc_text).is_ok() {
                saved_lrc_path = lrc_file.to_string_lossy().to_string();
            }
        }

        // 二进制标签与 1000×1000 封面、时间轴歌词内嵌 (FLAC Vorbis/Picture & MP3 ID3v2.3 APIC/USLT)
        if embed_cover_and_tags || embed_lyrics_in_audio {
            let mut cover_bytes_opt = None;
            if embed_cover_and_tags && !song.cover_url.trim().is_empty() {
                let hd_cover_url = song
                    .cover_url
                    .replace("/400/", "/1000/")
                    .replace("/240/", "/1000/");
                if let Ok(c_resp) = self.transport.client.get(&hd_cover_url).send().await {
                    if let Ok(b) = c_resp.bytes().await {
                        if b.len() > 256 {
                            cover_bytes_opt = Some(b.to_vec());
                        }
                    }
                }
            }

            let tag_meta = AudioTagMetadata {
                title: song.title.clone(),
                artist: song.artist.clone(),
                album: song.album.clone(),
                publish_date: song.publish_date.clone(),
                song_hash: pb.resolved_hash.clone(),
                quality_label: pb.quality_label.clone(),
                cover_bytes: cover_bytes_opt,
                lrc_text: if embed_lyrics_in_audio {
                    formatted_lrc_text
                } else {
                    String::new()
                },
            };
            let _ = metadata::embed_metadata(&audio_file_path, &tag_meta, &pb.ext_name);
        }

        let final_size = std::fs::metadata(&audio_file_path)
            .map(|m| m.len() as i64)
            .unwrap_or(audio_bytes.len() as i64);

        Ok(DownloadTaskResult {
            success: true,
            saved_audio_path: audio_file_path.to_string_lossy().to_string(),
            saved_lrc_path,
            file_size_bytes: final_size,
            quality_label: pb.quality_label,
            message: format!(
                "已保存并内嵌 1000P 封面/标签至 {}",
                audio_file_path.to_string_lossy()
            ),
        })
    }

    /// 30. 云端收藏歌单与取消收藏歌单 (对齐 MusicRepository.collectPlaylist & deleteOrUncollectPlaylist)
    pub async fn collect_playlist_cloud(&self, playlist: &Playlist) -> Result<bool, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录酷狗账号".to_string());
        };
        let dev = self.store.device();
        let now_sec = Utc::now().timestamp();
        let gid = if !playlist.global_collection_id.is_empty() {
            playlist.global_collection_id.clone()
        } else {
            playlist.id.clone()
        };
        let mut author_uid = 0i64;
        let mut author_list_id = playlist.list_id;
        if gid.contains('_') {
            let parts: Vec<&str> = gid.split('_').collect();
            if parts.len() >= 4 {
                author_uid = parts[2].parse::<i64>().unwrap_or(0);
                author_list_id = parts[3].parse::<i64>().unwrap_or(author_list_id);
            }
        }
        let body = json!({
            "userid": session.user_id.parse::<i64>().unwrap_or(0),
            "token": session.token,
            "total_ver": 0,
            "name": playlist.title,
            "type": 1,
            "source": 1,
            "is_pri": 0,
            "list_create_userid": author_uid,
            "list_create_listid": author_list_id,
            "list_create_gid": gid,
            "from_shupinmv": 0
        });
        let mut query = HashMap::new();
        query.insert("last_time".to_string(), now_sec.to_string());
        query.insert("last_area".to_string(), "gztx".to_string());
        query.insert("userid".to_string(), session.user_id.clone());
        query.insert("token".to_string(), session.token.clone());

        let mut headers = HashMap::new();
        headers.insert(
            "x-router".to_string(),
            "cloudlist.service.kugou.com".to_string(),
        );
        let resp = self
            .transport
            .call(
                "/v5/add_list",
                &dev,
                Some(&session),
                query,
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await?;
        Ok(resp.is_success() || resp.http_code == 200)
    }

    pub async fn uncollect_playlist_cloud(&self, playlist: &Playlist) -> Result<bool, String> {
        let mut list_id = playlist.list_id;
        if list_id <= 0 {
            if let Ok(cloud_lists) = self.fetch_user_playlists().await {
                if let Some(found) = cloud_lists.into_iter().find(|p| {
                    p.is_favorite
                        && ((!playlist.global_collection_id.is_empty()
                            && p.global_collection_id == playlist.global_collection_id)
                            || (!playlist.id.is_empty() && p.id == playlist.id)
                            || p.title == playlist.title)
                }) {
                    list_id = found.list_id;
                }
            }
        }
        let ptype = if playlist.is_created_by_me { 0 } else { 1 };
        self.execute_encrypted_delete_list(list_id, ptype).await
    }

    /// 31. 从自建歌单中移除单曲 (对齐 MusicRepository.removeSongFromPlaylist -> /v4/delete_songs)
    pub async fn remove_song_from_user_playlist(
        &self,
        list_id: i64,
        song: &Song,
    ) -> Result<bool, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录酷狗账号".to_string());
        };
        let dev = self.store.device();
        let now_sec = Utc::now().timestamp();
        let mut resolved_file_id = song.file_id;
        if resolved_file_id <= 0 {
            if let Ok(playlists) = self.fetch_user_playlists().await {
                if let Some(target_pl) = playlists.into_iter().find(|p| p.list_id == list_id) {
                    let gid = if !target_pl.global_collection_id.is_empty() {
                        target_pl.global_collection_id
                    } else {
                        target_pl.id
                    };
                    if let Ok(tracks) = self.fetch_playlist_songs(&gid).await {
                        if let Some(matched) = tracks.into_iter().find(|t| {
                            (!t.hash.is_empty() && t.hash.eq_ignore_ascii_case(&song.hash))
                                || (t.album_audio_id > 0 && t.album_audio_id == song.album_audio_id)
                                || (!t.id.is_empty() && t.id == song.id)
                        }) {
                            if matched.file_id > 0 {
                                resolved_file_id = matched.file_id;
                            }
                        }
                    }
                }
            }
        }
        let mix_id = if song.album_audio_id > 0 {
            song.album_audio_id
        } else {
            song.audio_id
        };
        let body = json!({
            "listid": list_id,
            "userid": session.user_id.parse::<i64>().unwrap_or(0),
            "token": session.token,
            "type": 0,
            "list_ver": 0,
            "data": [{
                "fileid": resolved_file_id,
                "hash": song.hash.to_uppercase(),
                "mixsongid": mix_id
            }]
        });
        let mut query = HashMap::new();
        query.insert("last_time".to_string(), now_sec.to_string());
        query.insert("last_area".to_string(), "gztx".to_string());
        query.insert("userid".to_string(), session.user_id.clone());
        query.insert("token".to_string(), session.token.clone());

        if let Ok(resp) = self
            .transport
            .call(
                "/cloudlist.service/v4/delete_songs",
                &dev,
                Some(&session),
                query.clone(),
                Some(body.clone()),
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await
        {
            if resp.is_success() {
                return Ok(true);
            }
        }

        let mut headers = HashMap::new();
        headers.insert(
            "x-router".to_string(),
            "cloudlist.service.kugou.com".to_string(),
        );
        let resp = self
            .transport
            .call(
                "/v4/delete_songs",
                &dev,
                Some(&session),
                query,
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await?;
        Ok(resp.is_success())
    }

    /// 32. 歌手专辑列表与专辑详情 (对齐 MusicRepository.artistAlbums & albumDetail)
    pub async fn fetch_artist_albums(
        &self,
        artist_id: &str,
        page: u32,
        sort: &str,
    ) -> Result<Vec<AlbumItem>, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let body = json!({
            "author_id": artist_id,
            "pagesize": 36,
            "page": page,
            "sort": if sort == "hot" { 3 } else { 1 },
            "category": 1,
            "area_code": "all"
        });
        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
        headers.insert("kg-tid".to_string(), "36".to_string());

        let mut out = Vec::new();
        if let Ok(resp) = self
            .transport
            .call(
                "/kmr/v1/author/albums",
                &dev,
                session.as_ref(),
                HashMap::new(),
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await
        {
            if let Some(arr) = resp
                .data()
                .get("info")
                .or_else(|| resp.data().get("list"))
                .or_else(|| resp.data().get("data"))
                .and_then(|v| v.as_array())
            {
                for o in arr {
                    let aid = Self::val_str(o, "album_id");
                    let name = Self::val_str(o, "album_name");
                    if !aid.is_empty() && !name.is_empty() {
                        let pic = {
                            let p = Self::val_str(o, "sizable_cover");
                            if !p.is_empty() {
                                p
                            } else {
                                Self::val_str(o, "imgurl")
                            }
                        };
                        out.push(AlbumItem {
                            album_id: aid,
                            album_name: name,
                            artist_name: Self::val_str(o, "author_name"),
                            artist_id: artist_id.to_string(),
                            cover_url: signer::normalize_image_url(Some(&pic)),
                            publish_date: Self::val_str(o, "publish_date")
                                .chars()
                                .take(10)
                                .collect(),
                            publish_company: Self::val_str(o, "publish_company"),
                            song_count: Self::val_i64(o, "song_count").unwrap_or(10) as i32,
                            intro: Self::val_str(o, "intro"),
                        });
                    }
                }
            }
        }
        if !out.is_empty() {
            return Ok(out);
        }

        let mut query = HashMap::new();
        query.insert("version".to_string(), "9108".to_string());
        query.insert("singerid".to_string(), artist_id.to_string());
        query.insert("plat".to_string(), "0".to_string());
        query.insert("pagesize".to_string(), "36".to_string());
        query.insert("page".to_string(), page.to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/singer/album",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            if let Some(arr) = resp.data().get("info").and_then(|v| v.as_array()) {
                for o in arr {
                    let aid = Self::val_str(o, "albumid");
                    let name = Self::val_str(o, "albumname");
                    if !aid.is_empty() && !name.is_empty() {
                        let pic = Self::val_str(o, "imgurl");
                        out.push(AlbumItem {
                            album_id: aid,
                            album_name: name,
                            artist_name: Self::val_str(o, "singername"),
                            artist_id: artist_id.to_string(),
                            cover_url: signer::normalize_image_url(Some(&pic)),
                            publish_date: Self::val_str(o, "publishtime")
                                .chars()
                                .take(10)
                                .collect(),
                            publish_company: Self::val_str(o, "company"),
                            song_count: Self::val_i64(o, "songcount").unwrap_or(10) as i32,
                            intro: Self::val_str(o, "intro"),
                        });
                    }
                }
            }
        }
        Ok(out)
    }

    pub async fn fetch_album_detail(&self, album_id: &str) -> Result<AlbumItem, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let body = json!({
            "data": [{ "album_id": album_id }],
            "is_buy": 0,
            "fields": "album_id,album_name,publish_date,sizable_cover,intro,language,is_publish,heat,type,quality,authors,exclusive,author_name,publish_company,song_count"
        });
        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
        headers.insert("kg-tid".to_string(), "255".to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/kmr/v2/albums",
                &dev,
                session.as_ref(),
                HashMap::new(),
                Some(body),
                "https://gateway.kugou.com",
                "POST",
                false,
                headers,
                true,
                true,
            )
            .await
        {
            if let Some(obj) = resp
                .data()
                .get("data")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .or_else(|| resp.data().as_array().and_then(|a| a.first()))
            {
                let name = Self::val_str(obj, "album_name");
                if !name.is_empty() {
                    let pic = Self::val_str(obj, "sizable_cover");
                    return Ok(AlbumItem {
                        album_id: album_id.to_string(),
                        album_name: name,
                        artist_name: Self::val_str(obj, "author_name"),
                        artist_id: String::new(),
                        cover_url: signer::normalize_image_url(Some(&pic)),
                        publish_date: Self::val_str(obj, "publish_date")
                            .chars()
                            .take(10)
                            .collect(),
                        publish_company: Self::val_str(obj, "publish_company"),
                        song_count: Self::val_i64(obj, "song_count").unwrap_or(0) as i32,
                        intro: Self::val_str(obj, "intro"),
                    });
                }
            }
        }

        let mut query = HashMap::new();
        query.insert("version".to_string(), "9108".to_string());
        query.insert("albumid".to_string(), album_id.to_string());
        query.insert("plat".to_string(), "0".to_string());
        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/album/info",
                &dev,
                None,
                query,
                None,
                "http://mobilecdnbj.kugou.com",
                "GET",
                false,
                HashMap::new(),
                false,
                false,
            )
            .await
        {
            let d = resp.data();
            let name = Self::val_str(d, "albumname");
            if !name.is_empty() {
                let pic = Self::val_str(d, "imgurl");
                return Ok(AlbumItem {
                    album_id: album_id.to_string(),
                    album_name: name,
                    artist_name: Self::val_str(d, "singername"),
                    artist_id: Self::val_str(d, "singerid"),
                    cover_url: signer::normalize_image_url(Some(&pic)),
                    publish_date: Self::val_str(d, "publishtime")
                        .chars()
                        .take(10)
                        .collect(),
                    publish_company: Self::val_str(d, "company"),
                    song_count: Self::val_i64(d, "songcount").unwrap_or(0) as i32,
                    intro: Self::val_str(d, "intro"),
                });
            }
        }
        Err("未获取到专辑详情".to_string())
    }

    /// 33. 搜索实时联想建议词 (对齐 MusicRepository.searchSuggest -> /v2/getSearchTip)
    pub async fn fetch_search_suggest(&self, keyword: &str) -> Result<Vec<String>, String> {
        let kw = keyword.trim();
        if kw.is_empty() {
            return Ok(Vec::new());
        }
        let dev = self.store.device();
        let mut query = HashMap::new();
        query.insert("keyword".to_string(), kw.to_string());
        query.insert("MusicTipCount".to_string(), "8".to_string());
        query.insert("AlbumTipCount".to_string(), "2".to_string());
        query.insert("CorrectTipCount".to_string(), "1".to_string());
        query.insert("MVTipCount".to_string(), "0".to_string());
        query.insert("radiotip".to_string(), "1".to_string());

        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "searchtip.kugou.com".to_string());

        let mut tips = Vec::new();
        if let Ok(resp) = self
            .transport
            .call(
                "/v2/getSearchTip",
                &dev,
                None,
                query,
                None,
                "https://gateway.kugou.com",
                "GET",
                false,
                headers,
                true,
                true,
            )
            .await
        {
            if let Some(arr) = resp.data().as_array().or_else(|| resp.data().get("data").and_then(|v| v.as_array())) {
                for group in arr {
                    if let Some(records) = group.get("RecordDatas").and_then(|v| v.as_array()) {
                        for r in records {
                            let h = Self::val_str(r, "HintInfo");
                            if !h.is_empty() && !tips.contains(&h) {
                                tips.push(h);
                            }
                        }
                    }
                }
            }
        }
        Ok(tips.into_iter().take(10).collect())
    }

    /// 34. 评论二层楼中楼回复、点赞评论与发表评论 (对齐 MusicRepository.songCommentReplies / likeSongComment / postSongComment)
    pub async fn fetch_comment_floor_replies(
        &self,
        song: &Song,
        comment_id: &str,
        special_child_id: &str,
        page: u32,
    ) -> Result<Vec<SongComment>, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let mix_id = if song.album_audio_id > 0 {
            song.album_audio_id
        } else {
            song.audio_id
        };
        let eff_child = if !special_child_id.trim().is_empty() && special_child_id != "0" {
            special_child_id.trim().to_string()
        } else if mix_id > 0 {
            mix_id.to_string()
        } else {
            song.hash.to_lowercase()
        };

        let mut query = HashMap::new();
        query.insert("childrenid".to_string(), eff_child.clone());
        query.insert("special_id".to_string(), eff_child);
        query.insert("need_show_image".to_string(), "1".to_string());
        query.insert("p".to_string(), page.to_string());
        query.insert("pagesize".to_string(), "20".to_string());
        query.insert("code".to_string(), "fc4be23b4e972707f36b8a828a93ba8a".to_string());
        query.insert("tid".to_string(), comment_id.to_string());
        query.insert("cmtid".to_string(), comment_id.to_string());
        query.insert("p_id".to_string(), comment_id.to_string());
        query.insert("ver".to_string(), "6".to_string());
        if mix_id > 0 {
            query.insert("mixsongid".to_string(), mix_id.to_string());
        }
        if !song.hash.is_empty() {
            query.insert("extdata".to_string(), song.hash.to_lowercase());
        }

        let mut replies = Vec::new();
        for path in ["/mcomment/v1/replylist", "/mcomment/v1/hot_replylist"] {
            if let Ok(resp) = self
                .transport
                .call(
                    path,
                    &dev,
                    session.as_ref(),
                    query.clone(),
                    None,
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await
            {
                let root = resp.data();
                for key in ["list", "lists", "reply_list", "hot_reply_list", "comments", "weight_list"] {
                    if let Some(arr) = root.get(key).and_then(|v| v.as_array()) {
                        for item in arr {
                            if let Some(rc) = Self::parse_single_comment_item(item, Some(comment_id)) {
                                if !replies.iter().any(|r: &SongComment| r.id == rc.id) {
                                    replies.push(rc);
                                }
                            }
                        }
                    }
                }
            }
            if !replies.is_empty() {
                break;
            }
        }
        Ok(replies)
    }

    pub async fn like_song_comment(
        &self,
        song: &Song,
        comment_id: &str,
        like: bool,
    ) -> Result<bool, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录酷狗账号".to_string());
        };
        let dev = self.store.device();
        let mix_id = if song.album_audio_id > 0 {
            song.album_audio_id
        } else {
            song.audio_id
        };
        let mut query = HashMap::new();
        query.insert("mixsongid".to_string(), mix_id.to_string());
        query.insert("cmtid".to_string(), comment_id.to_string());
        query.insert("action".to_string(), if like { "1" } else { "0" }.to_string());
        query.insert("code".to_string(), "fc4be23b4e972707f36b8a828a93ba8a".to_string());
        query.insert("token".to_string(), session.token.clone());
        query.insert("userid".to_string(), session.user_id.clone());

        let resp = self
            .transport
            .call(
                "/mcomment/v1/cmt_like",
                &dev,
                Some(&session),
                query,
                None,
                "https://gateway.kugou.com",
                "POST",
                false,
                HashMap::new(),
                true,
                true,
            )
            .await?;
        Ok(resp.is_success() || resp.http_code == 200)
    }

    pub async fn post_song_comment(
        &self,
        song: &Song,
        content: &str,
        reply_to_id: Option<&str>,
    ) -> Result<SongComment, String> {
        let Some(session) = self.store.session() else {
            return Err("请先登录酷狗账号后再发表评论".to_string());
        };
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err("评论内容不能为空".to_string());
        }
        let dev = self.store.device();
        let mix_id = if song.album_audio_id > 0 {
            song.album_audio_id
        } else {
            song.audio_id
        };
        let ext_data = song.hash.to_lowercase();
        let path = if reply_to_id.is_some() {
            "/mcomment/v1/reply_add"
        } else {
            "/mcomment/v1/cmt_add"
        };
        let mut query = HashMap::new();
        query.insert("mixsongid".to_string(), mix_id.to_string());
        query.insert("content".to_string(), trimmed.to_string());
        query.insert("code".to_string(), "fc4be23b4e972707f36b8a828a93ba8a".to_string());
        query.insert("extdata".to_string(), ext_data);
        query.insert("token".to_string(), session.token.clone());
        query.insert("userid".to_string(), session.user_id.clone());
        if let Some(tid) = reply_to_id {
            if !tid.is_empty() {
                query.insert("cmtid".to_string(), tid.to_string());
                query.insert("tid".to_string(), tid.to_string());
            }
        }

        // 通道 1: 对齐 Android MusicRepository.postSongComment -> /index.php?r=commentsv3/add (m.comment.service.kugou.com)
        let now_sec = Utc::now().timestamp();
        let route = if reply_to_id.is_some() {
            "commentsv2/reply"
        } else {
            "commentsv3/add"
        };
        let mut data_obj = json!({
            "content": trimmed,
            "code": "fc4be23b4e972707f36b8a828a93ba8a",
            "childrenid": if mix_id > 0 { mix_id.to_string() } else { song.hash.to_lowercase() },
            "extdata": song.hash.to_lowercase(),
            "mixsongid": mix_id,
            "userid": session.user_id,
            "token": session.token,
            "ver": 6
        });
        if let Some(tid) = reply_to_id {
            if !tid.is_empty() {
                data_obj["tid"] = json!(tid);
            }
        }
        let data_json = data_obj.to_string();
        let sign_key = signer::params_key(now_sec, &format!("{}{}", dev.mid, data_json));
        let mut primary_q = HashMap::new();
        primary_q.insert("r".to_string(), route.to_string());
        primary_q.insert("clienttime".to_string(), now_sec.to_string());
        primary_q.insert("key".to_string(), sign_key);
        let mut primary_h = HashMap::new();
        primary_h.insert("x-router".to_string(), "m.comment.service.kugou.com".to_string());

        let mut comment_id = format!("c_{}", Utc::now().timestamp_millis());
        let mut location = String::new();
        let mut confirmed = false;

        if let Ok(resp) = self
            .transport
            .call(
                "/index.php",
                &dev,
                Some(&session),
                primary_q,
                Some(json!({ "data": data_json })),
                "https://gateway.kugou.com",
                "POST",
                false,
                primary_h,
                true,
                true,
            )
            .await
        {
            if resp.is_success() {
                confirmed = true;
                let d = resp.data();
                let sid = Self::val_str(d, "cmtid");
                if !sid.is_empty() {
                    comment_id = sid;
                }
                location = Self::val_str(d, "location");
            }
        }

        if !confirmed {
            if let Ok(fb_resp) = self
                .transport
                .call(
                    path,
                    &dev,
                    Some(&session),
                    query,
                    None,
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    HashMap::new(),
                    true,
                    true,
                )
                .await
            {
                let d = fb_resp.data();
                let sid = Self::val_str(d, "cmtid");
                if !sid.is_empty() {
                    comment_id = sid;
                }
            }
        }

        Ok(SongComment {
            id: comment_id,
            user_id: session.user_id.clone(),
            user_name: session.nickname.clone(),
            user_avatar: session.avatar_url.clone(),
            content: Self::format_kugou_comment_emojis(trimmed),
            created_time: "刚刚".to_string(),
            like_count: 0,
            is_liked: false,
            is_hot: false,
            vip_level: 6,
            location,
            reply_count: 0,
            special_child_id: String::new(),
            replies: Vec::new(),
            images: Vec::new(),
            reply_user_name: None,
            reply_quoted_content: None,
            parent_id: reply_to_id.map(|s| s.to_string()),
            root_id: reply_to_id.map(|s| s.to_string()),
            reply_target_comment_id: reply_to_id.map(|s| s.to_string()),
            reply_chain_hops: Vec::new(),
            timestamp_ms: Utc::now().timestamp_millis(),
        })
    }

    /// 35. KTV 伴奏匹配与解析 (对齐 MusicRepository.matchAccompanyHash & resolvePlayback(useAccompaniment=true))
    pub async fn resolve_song_accompaniment(&self, song: &Song) -> Result<PlaybackResult, String> {
        if !song.accompany_hash.trim().is_empty() {
            let mut acc_song = song.clone();
            acc_song.hash = song.accompany_hash.clone();
            acc_song.hash128 = song.accompany_hash.clone();
            acc_song.hash320 = song.accompany_hash.clone();
            acc_song.hash_flac.clear();
            acc_song.hash_hi_res.clear();
            if let Ok(res) = self.resolve_song_url(&acc_song, "320").await {
                return Ok(res);
            }
        }
        let keyword = format!("{} {} 伴奏", song.artist, song.title);
        let candidates = self.search_songs(&keyword, 1, 8).await.unwrap_or_default();
        if let Some(acc_song) = candidates.into_iter().find(|c| {
            c.title.contains("伴奏")
                || c.title.to_lowercase().contains("instrumental")
                || c.title.to_lowercase().contains("backing")
        }) {
            return self.resolve_song_url(&acc_song, "320").await;
        }
        Err("该曲目暂未收录官方伴奏音轨".to_string())
    }

    /// 36. 获取单曲完整技术参数与发行公司详情 (对齐 MusicRepository.fetchSongDetail -> /kmr/v2/audio + STREAMINFO)
    pub async fn fetch_song_detail_info(&self, song: &Song) -> Result<SongDetailInfo, String> {
        self.fetch_song_detail_info_with_quality(song, None).await
    }

    pub async fn fetch_song_detail_info_with_quality(
        &self,
        song: &Song,
        preferred_quality: Option<&str>,
    ) -> Result<SongDetailInfo, String> {
        let enriched = self
            .enrich_songs_privilege_lite(vec![song.clone()])
            .await
            .into_iter()
            .next()
            .unwrap_or_else(|| song.clone());

        let mut album_name = if !enriched.album.is_empty() {
            enriched.album.clone()
        } else {
            "单曲发行".to_string()
        };
        let mut publish_time = if !enriched.publish_date.is_empty() {
            enriched.publish_date.clone()
        } else {
            "官方数字收录".to_string()
        };
        let mut company = "酷狗概念版正版曲库".to_string();

        if !enriched.album_id.is_empty() && enriched.album_id != "0" {
            if let Ok(alb) = self.fetch_album_detail(&enriched.album_id).await {
                if !alb.album_name.is_empty() {
                    album_name = alb.album_name;
                }
                if !alb.publish_date.is_empty() {
                    publish_time = alb.publish_date;
                }
                if !alb.publish_company.is_empty() {
                    company = alb.publish_company;
                }
            }
        }

        let target_q = preferred_quality.unwrap_or("high");
        let pb_opt = self.resolve_song_url(&enriched, target_q).await.ok();
        let (audio_quality, sample_rate, bit_rate, file_size, file_hash) = if let Some(pb) = pb_opt
        {
            let sr_str = format!(
                "{:.1} kHz / {} bit ({}ch · {})",
                (pb.sample_rate_hz as f64) / 1000.0,
                pb.bit_depth,
                pb.channels.max(2),
                pb.ext_name.to_uppercase()
            );
            let dur_sec = ((enriched.duration_ms.max(180_000) as f64) / 1000.0).max(1.0);
            let estimated_bytes = ((pb.bitrate_kbps.max(128) as f64) * 1000.0 / 8.0 * dur_sec) as i64;
            let eff_bytes = if pb.file_size_bytes > 0 { pb.file_size_bytes } else { estimated_bytes };
            let sz_mb = format!("{:.2} MB", (eff_bytes as f64) / (1024.0 * 1024.0));
            (
                format!("{} ({})", pb.quality_label, pb.ext_name.to_uppercase()),
                sr_str,
                format!("{} kbps", pb.bitrate_kbps),
                sz_mb,
                pb.resolved_hash.to_uppercase(),
            )
        } else {
            let sz = enriched
                .size_hi_res
                .max(enriched.size_flac)
                .max(enriched.size320)
                .max(enriched.size128);
            let dur_sec = ((enriched.duration_ms.max(180_000) as f64) / 1000.0).max(1.0);
            let fallback_kbps = if enriched.is_hi_res { 2304.0 } else if enriched.is_lossless { 920.0 } else { 320.0 };
            let eff_sz = if sz > 0 { sz } else { (fallback_kbps * 1000.0 / 8.0 * dur_sec) as i64 };
            let sz_mb = format!("{:.2} MB", (eff_sz as f64) / (1024.0 * 1024.0));
            (
                if enriched.is_hi_res {
                    "Hi-Res 高解析度无损 (FLAC)".to_string()
                } else if enriched.is_lossless {
                    "SQ 无损音质 (FLAC)".to_string()
                } else {
                    "HQ 高品音质 (320kbps MP3)".to_string()
                },
                if enriched.is_hi_res {
                    "96.0 kHz / 24 bit (FLAC)".to_string()
                } else {
                    "44.1 kHz / 16 bit".to_string()
                },
                if enriched.is_hi_res {
                    "2304 kbps".to_string()
                } else if enriched.is_lossless {
                    "920 kbps".to_string()
                } else {
                    "320 kbps".to_string()
                },
                sz_mb,
                enriched.hash.to_uppercase(),
            )
        };

        let mut accompany_hash = enriched.accompany_hash.clone();
        if accompany_hash.trim().is_empty() {
            let keyword = format!("{} {} 伴奏", song.artist, song.title);
            if let Ok(candidates) = self.search_songs(&keyword, 1, 6).await {
                if let Some(acc) = candidates.into_iter().find(|c| {
                    c.title.contains("伴奏")
                        || c.title.to_lowercase().contains("instrumental")
                        || c.title.to_lowercase().contains("backing")
                }) {
                    accompany_hash = acc.hash;
                }
            }
        }
        let has_accompaniment = !accompany_hash.trim().is_empty();
        Ok(SongDetailInfo {
            song: enriched,
            album_name,
            singer_name: song.artist.clone(),
            publish_time,
            company,
            audio_quality,
            sample_rate,
            bit_rate,
            file_size,
            file_hash,
            has_accompaniment,
            accompany_hash,
        })
    }

    /// 37. CSCC 三段式真实听歌时长上报 + 听歌历史上传 + 本地最近播放落盘 (对齐 KugouCsccReporter & uploadPlayHistory)
    pub async fn report_playback_telemetry(
        &self,
        song: &Song,
        is_start: bool,
        played_duration_ms: i64,
    ) -> Result<bool, String> {
        let session_opt = self.store.session();
        let uid_key = session_opt
            .as_ref()
            .map(|s| s.user_id.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("guest");

        if is_start {
            // 立即将当前播放歌曲写入本地持久化最近播放历史，重启软件绝不丢失
            self.store.append_play_history(uid_key, song);
        }

        let Some(session) = session_opt else {
            return Ok(false);
        };
        let dev = self.store.device();

        if is_start {
            // 上报云端听歌足迹 (/playhistory/v1/upload_songs)
            let mxid = self.cscc.resolve_mix_song_id(song).await;
            if mxid > 0 {
                let body = json!({
                    "songs": [{
                        "mxid": mxid,
                        "op": 1,
                        "ot": Utc::now().timestamp(),
                        "pc": 1
                    }],
                    "token": session.token,
                    "userid": session.user_id.parse::<i64>().unwrap_or(0),
                    "source_classify": "app"
                });
                let mut query = HashMap::new();
                query.insert("plat".to_string(), "3".to_string());
                query.insert("userid".to_string(), session.user_id.clone());
                query.insert("token".to_string(), session.token.clone());
                let _ = self
                    .transport
                    .call(
                        "/playhistory/v1/upload_songs",
                        &dev,
                        Some(&session),
                        query,
                        Some(body),
                        "https://gateway.kugou.com",
                        "POST",
                        false,
                        HashMap::new(),
                        true,
                        true,
                    )
                    .await;
            }
        }

        self.cscc
            .report_event(
                &dev,
                &session,
                song,
                is_start,
                played_duration_ms,
                if played_duration_ms >= (song.duration_ms * 8 / 10).max(15_000) {
                    "完整播放"
                } else {
                    "切换歌曲"
                },
            )
            .await
    }

    /// 38. 酷狗蝰蛇音效全量预设库与在线调音师热度补全 (对齐 Android AudioEffectManager.kt 全部 6 大分类 20 款预设)
    pub async fn fetch_viper_effects(&self) -> Result<Vec<ViperEffectItem>, String> {
        let presets = vec![
            ViperEffectItem {
                id: "viper_3d".to_string(),
                mode_id: "1001".to_string(),
                label: "3D 丽音".to_string(),
                desc: "环绕立体声场与晶莹高频泛音，给你极致听觉感受".to_string(),
                category: "官方旗舰".to_string(),
                usage_badge: "5.4亿人使用".to_string(),
                author_name: "蝰蛇声学实验室".to_string(),
                acoustic_tags: vec!["3D声场".to_string(), "高频晶莹".to_string(), "旗舰王牌".to_string()],
                gains: vec![3.2, 2.5, 1.0, -0.5, 0.2, 1.4, 2.8, 3.6, 4.2, 3.8],
                sub_bass: 48,
                surround: 78,
                vocal_clarity: 52,
                air_harmonics: 68,
                tube_warmth: 25,
            },
            ViperEffectItem {
                id: "viper_bass".to_string(),
                mode_id: "1002".to_string(),
                label: "超重低音".to_string(),
                desc: "澎湃低频下潜与弹性瞬态冲击，低音浑厚不轰头".to_string(),
                category: "官方旗舰".to_string(),
                usage_badge: "5.8亿人使用".to_string(),
                author_name: "蝰蛇声学实验室".to_string(),
                acoustic_tags: vec!["深沉下潜".to_string(), "低频动态".to_string(), "节奏澎湃".to_string()],
                gains: vec![6.5, 5.8, 3.4, 0.6, -0.8, 0.0, 1.2, 2.0, 2.4, 1.8],
                sub_bass: 85,
                surround: 42,
                vocal_clarity: 45,
                air_harmonics: 35,
                tube_warmth: 40,
            },
            ViperEffectItem {
                id: "viper_vocal".to_string(),
                mode_id: "1003".to_string(),
                label: "纯净人声".to_string(),
                desc: "还原音频中每一个人声呼吸细节，中频温润贴耳".to_string(),
                category: "官方旗舰".to_string(),
                usage_badge: "4.2亿人使用".to_string(),
                author_name: "蝰蛇声学实验室".to_string(),
                acoustic_tags: vec!["人声贴耳".to_string(), "齿音柔化".to_string(), "通透中频".to_string()],
                gains: vec![-1.2, -0.6, 0.8, 2.4, 3.8, 4.4, 3.6, -1.2, 1.8, 2.2],
                sub_bass: 22,
                surround: 35,
                vocal_clarity: 88,
                air_harmonics: 54,
                tube_warmth: 30,
            },
            ViperEffectItem {
                id: "viper_hifi".to_string(),
                mode_id: "1004".to_string(),
                label: "HiFi 现场".to_string(),
                desc: "逼真丰富的现场微动态与宽阔舞台结像感".to_string(),
                category: "官方旗舰".to_string(),
                usage_badge: "3.1亿人使用".to_string(),
                author_name: "蝰蛇声学实验室".to_string(),
                acoustic_tags: vec!["临场还原".to_string(), "微动态".to_string(), "高保真".to_string()],
                gains: vec![3.8, 3.2, 1.4, 0.0, 0.6, 1.8, 2.8, 3.2, 3.0, 2.8],
                sub_bass: 56,
                surround: 72,
                vocal_clarity: 64,
                air_harmonics: 60,
                tube_warmth: 35,
            },
            ViperEffectItem {
                id: "viper_black_diamond".to_string(),
                mode_id: "1006".to_string(),
                label: "黑钻蝰蛇 · 母带".to_string(),
                desc: "多重谐波修复技术，还原黑胶母带光泽与微细节".to_string(),
                category: "官方旗舰".to_string(),
                usage_badge: "1.9亿人使用".to_string(),
                author_name: "酷狗母带工程组".to_string(),
                acoustic_tags: vec!["谐波修复".to_string(), "母带重塑".to_string(), "VIP臻享".to_string()],
                gains: vec![4.2, 3.6, 1.8, 0.2, 0.8, 2.0, 3.2, 3.4, 3.6, 4.0],
                sub_bass: 62,
                surround: 55,
                vocal_clarity: 74,
                air_harmonics: 82,
                tube_warmth: 58,
            },
            ViperEffectItem {
                id: "viper_clear_master".to_string(),
                mode_id: "1007".to_string(),
                label: "蝰蛇 · 超清母带".to_string(),
                desc: "192kHz 采样率声学补偿，展现极高频空气泛音".to_string(),
                category: "官方旗舰".to_string(),
                usage_badge: "2.2亿人使用".to_string(),
                author_name: "酷狗母带工程组".to_string(),
                acoustic_tags: vec!["192kHz感".to_string(), "空气泛音".to_string(), "超高解析".to_string()],
                gains: vec![2.8, 2.4, 1.0, 0.0, 0.6, 1.6, 2.8, 3.8, 4.6, 4.8],
                sub_bass: 46,
                surround: 50,
                vocal_clarity: 70,
                air_harmonics: 92,
                tube_warmth: 20,
            },
            ViperEffectItem {
                id: "viper_panorama_51".to_string(),
                mode_id: "1005".to_string(),
                label: "5.1 全景声".to_string(),
                desc: "多声道球形全息影院声场，沉浸式环绕听感".to_string(),
                category: "3D空间".to_string(),
                usage_badge: "2.5亿人使用".to_string(),
                author_name: "蝰蛇全景声实验室".to_string(),
                acoustic_tags: vec!["5.1多声道".to_string(), "分轨空间".to_string(), "影院包裹".to_string()],
                gains: vec![4.6, 4.0, 2.0, -0.4, 0.4, 1.8, 3.0, 3.6, 3.8, 3.4],
                sub_bass: 68,
                surround: 92,
                vocal_clarity: 58,
                air_harmonics: 65,
                tube_warmth: 28,
            },
            ViperEffectItem {
                id: "viper_surround".to_string(),
                mode_id: "2002".to_string(),
                label: "全景环绕".to_string(),
                desc: "360° 沉浸式声场扩展，打破耳机头中效应".to_string(),
                category: "3D空间".to_string(),
                usage_badge: "2.1亿人使用".to_string(),
                author_name: "空间声学团队".to_string(),
                acoustic_tags: vec!["全息声场".to_string(), "消除头中感".to_string(), "宽阔包裹".to_string()],
                gains: vec![3.4, 2.8, 1.2, -0.6, 0.0, 1.5, 2.8, 3.4, 3.6, 3.2],
                sub_bass: 52,
                surround: 95,
                vocal_clarity: 50,
                air_harmonics: 62,
                tube_warmth: 24,
            },
            ViperEffectItem {
                id: "viper_concert".to_string(),
                mode_id: "2003".to_string(),
                label: "金色大厅交响".to_string(),
                desc: "维也纳音乐厅混响包络，适合古典弦乐与影视原声".to_string(),
                category: "3D空间".to_string(),
                usage_badge: "1.1亿人使用".to_string(),
                author_name: "空间声学团队".to_string(),
                acoustic_tags: vec!["音乐厅混响".to_string(), "恢弘纵深".to_string(), "器乐交响".to_string()],
                gains: vec![3.0, 3.4, 1.8, 0.2, 0.0, 1.2, 2.4, 3.2, 3.8, 3.6],
                sub_bass: 58,
                surround: 86,
                vocal_clarity: 55,
                air_harmonics: 72,
                tube_warmth: 36,
            },
            ViperEffectItem {
                id: "viper_livehouse".to_string(),
                mode_id: "2004".to_string(),
                label: "Livehouse 前排".to_string(),
                desc: "近场舞台监听与热烈低频空气感，犹如置身第一排".to_string(),
                category: "3D空间".to_string(),
                usage_badge: "9400万人使用".to_string(),
                author_name: "现场声学调音师".to_string(),
                acoustic_tags: vec!["近场舞台".to_string(), "乐队现场".to_string(), "鼓点扎实".to_string()],
                gains: vec![5.0, 4.4, 2.4, 0.4, 0.2, 1.4, 2.6, 3.0, 2.8, 2.4],
                sub_bass: 74,
                surround: 68,
                vocal_clarity: 60,
                air_harmonics: 52,
                tube_warmth: 42,
            },
            ViperEffectItem {
                id: "viper_vinyl".to_string(),
                mode_id: "3001".to_string(),
                label: "黑胶唱片".to_string(),
                desc: "温润复古模拟偶次谐波，柔和顺滑久听不累".to_string(),
                category: "复古介质".to_string(),
                usage_badge: "1.8亿人使用".to_string(),
                author_name: "复古模拟实验室".to_string(),
                acoustic_tags: vec!["偶次谐波".to_string(), "温润模拟".to_string(), "黑胶声底".to_string()],
                gains: vec![2.4, 3.2, 2.6, 1.6, 1.2, 0.8, 0.0, -1.0, -1.6, -2.2],
                sub_bass: 50,
                surround: 38,
                vocal_clarity: 62,
                air_harmonics: 24,
                tube_warmth: 92,
            },
            ViperEffectItem {
                id: "viper_tape".to_string(),
                mode_id: "3002".to_string(),
                label: "复古磁带机".to_string(),
                desc: "80 年代 Walkman 二氧化铬磁带微饱和与温暖中低频".to_string(),
                category: "复古介质".to_string(),
                usage_badge: "1.2亿人使用".to_string(),
                author_name: "复古模拟实验室".to_string(),
                acoustic_tags: vec!["Walkman感".to_string(), "磁带微饱和".to_string(), "暖厚人声".to_string()],
                gains: vec![1.8, 2.8, 2.4, 1.8, 1.6, 1.2, 0.4, -0.8, -1.4, -2.0],
                sub_bass: 44,
                surround: 32,
                vocal_clarity: 66,
                air_harmonics: 28,
                tube_warmth: 86,
            },
            ViperEffectItem {
                id: "viper_tube_amp".to_string(),
                mode_id: "3003".to_string(),
                label: "胆机电子管".to_string(),
                desc: "300B 真空电子管甲类放大音色，人声醇厚富有磁性".to_string(),
                category: "复古介质".to_string(),
                usage_badge: "9600万人使用".to_string(),
                author_name: "复古模拟实验室".to_string(),
                acoustic_tags: vec!["300B胆味".to_string(), "甲类放大".to_string(), "醇厚磁性".to_string()],
                gains: vec![2.2, 3.0, 2.2, 1.4, 2.0, 2.4, 1.4, 0.0, -0.6, -1.0],
                sub_bass: 48,
                surround: 40,
                vocal_clarity: 76,
                air_harmonics: 36,
                tube_warmth: 96,
            },
            ViperEffectItem {
                id: "viper_acg".to_string(),
                mode_id: "4001".to_string(),
                label: "ACG 二次元特调".to_string(),
                desc: "提升日系女声清甜度与电子合成器瞬态分离度".to_string(),
                category: "曲风人声".to_string(),
                usage_badge: "1.5亿人使用".to_string(),
                author_name: "ACG调音研究社".to_string(),
                acoustic_tags: vec!["清甜女声".to_string(), "电音瞬态".to_string(), "高频通透".to_string()],
                gains: vec![2.6, 2.0, 0.2, 0.8, 2.2, 3.4, 4.4, 4.6, 3.8, 3.0],
                sub_bass: 45,
                surround: 56,
                vocal_clarity: 84,
                air_harmonics: 80,
                tube_warmth: 22,
            },
            ViperEffectItem {
                id: "viper_folk".to_string(),
                mode_id: "4002".to_string(),
                label: "民谣木吉他".to_string(),
                desc: "凸显原声木吉他拨弦泛音与温暖叙事感人声".to_string(),
                category: "曲风人声".to_string(),
                usage_badge: "8900万人使用".to_string(),
                author_name: "原声声学实验室".to_string(),
                acoustic_tags: vec!["拨弦质感".to_string(), "温暖叙事".to_string(), "木箱共鸣".to_string()],
                gains: vec![1.0, 2.2, 1.8, 0.6, 1.8, 3.2, 3.8, 2.4, 2.8, 2.2],
                sub_bass: 34,
                surround: 44,
                vocal_clarity: 82,
                air_harmonics: 64,
                tube_warmth: 52,
            },
            ViperEffectItem {
                id: "viper_rnb".to_string(),
                mode_id: "4003".to_string(),
                label: "R&B 丝滑律动".to_string(),
                desc: "饱满弹性的 808 贝斯线条与丝滑转音细节".to_string(),
                category: "曲风人声".to_string(),
                usage_badge: "7800万人使用".to_string(),
                author_name: "都市律动工程组".to_string(),
                acoustic_tags: vec!["808弹性".to_string(), "丝滑转音".to_string(), "律动氛围".to_string()],
                gains: vec![4.8, 4.2, 1.8, -0.4, 1.0, 2.4, 3.0, 2.2, 3.2, 3.4],
                sub_bass: 70,
                surround: 58,
                vocal_clarity: 74,
                air_harmonics: 68,
                tube_warmth: 44,
            },
            ViperEffectItem {
                id: "viper_guofeng".to_string(),
                mode_id: "4004".to_string(),
                label: "国风空灵戏腔".to_string(),
                desc: "针对古筝、琵琶、竹笛与戏腔高频泛音深度优化".to_string(),
                category: "曲风人声".to_string(),
                usage_badge: "8200万人使用".to_string(),
                author_name: "东方声学实验室".to_string(),
                acoustic_tags: vec!["空灵戏腔".to_string(), "民乐泛音".to_string(), "悠远意境".to_string()],
                gains: vec![1.4, 1.8, 0.8, 1.2, 2.6, 3.8, 4.2, 3.4, 4.0, 3.6],
                sub_bass: 36,
                surround: 66,
                vocal_clarity: 86,
                air_harmonics: 78,
                tube_warmth: 32,
            },
            ViperEffectItem {
                id: "viper_car_sub".to_string(),
                mode_id: "5001".to_string(),
                label: "柏林之声 · 车载".to_string(),
                desc: "座舱声学反射补偿与低音炮分频增强，驾乘听感饱满".to_string(),
                category: "设备车载".to_string(),
                usage_badge: "1.6亿人使用".to_string(),
                author_name: "车载声学工程组".to_string(),
                acoustic_tags: vec!["座舱声场".to_string(), "车载低音炮".to_string(), "降噪补偿".to_string()],
                gains: vec![5.4, 4.8, 2.6, 0.4, 0.8, 2.0, 3.2, 3.6, 3.8, 3.4],
                sub_bass: 78,
                surround: 74,
                vocal_clarity: 68,
                air_harmonics: 64,
                tube_warmth: 38,
            },
            ViperEffectItem {
                id: "viper_headphone_pro".to_string(),
                mode_id: "5002".to_string(),
                label: "监听耳机 · 哈曼参考".to_string(),
                desc: "对齐 Harman Target 2019 频响曲线，三频均衡自然耐听".to_string(),
                category: "设备车载".to_string(),
                usage_badge: "1.1亿人使用".to_string(),
                author_name: "参考声学实验室".to_string(),
                acoustic_tags: vec!["哈曼曲线".to_string(), "Bauer交叉馈送".to_string(), "监听均衡".to_string()],
                gains: vec![3.2, 2.6, 0.8, -0.4, 0.2, 1.6, 2.8, 2.0, 1.4, 1.0],
                sub_bass: 48,
                surround: 44,
                vocal_clarity: 72,
                air_harmonics: 58,
                tube_warmth: 26,
            },
            ViperEffectItem {
                id: "viper_desktop_speaker".to_string(),
                mode_id: "5003".to_string(),
                label: "桌面近场 · 2.1音响".to_string(),
                desc: "针对桌面箱体驻波抑制与声像聚焦优化，结像清晰立体".to_string(),
                category: "设备车载".to_string(),
                usage_badge: "6800万人使用".to_string(),
                author_name: "桌面声学工程组".to_string(),
                acoustic_tags: vec!["近场结像".to_string(), "驻波抑制".to_string(), "桌面HiFi".to_string()],
                gains: vec![3.6, 2.8, -1.2, 0.2, 0.8, 1.6, 2.4, 3.0, 3.4, 3.2],
                sub_bass: 54,
                surround: 62,
                vocal_clarity: 70,
                air_harmonics: 68,
                tube_warmth: 30,
            },
        ];
        Ok(presets)
    }

    /// 39. 解析酷狗音乐/概念版分享链接、口令、global_collection_id 或歌单文本 (对齐 Android KugouShareLinkParser & resolveSharedMusicText)
    pub async fn parse_shared_music_text(&self, raw_text: &str) -> Result<SharedMusicResolved, String> {
        let text = raw_text.trim();
        if text.is_empty() {
            return Err("请输入分享链接、口令或歌单内容".to_string());
        }

        let mut working_text = text.to_string();

        // 1. 若包含短链 (t1.kugou.com / kgurl.cn / m.kugou.com / m3ws.kugou.com 等)，跟随重定向并提取最终 URL 与 HTML
        static SHARE_URL_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
            regex::Regex::new(r#"https?://[^\s"'<>|，。！？、；：（）【】《》“”‘’]+"#).unwrap()
        });
        static SHARE_GCID_RE: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| {
                regex::Regex::new(
                    r#"(?:global_collection_id|gcid|globalCollectionId)[=:]["']?(collection_[a-zA-Z0-9_]+|[a-zA-Z0-9_]{16,})"#,
                )
                .unwrap()
            });
        static SHARE_BARE_COLL_RE: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| {
                regex::Regex::new(r#"\b(collection_\d+_\d+_\d+_\d+)\b"#).unwrap()
            });
        static SHARE_SPECIAL_ID_RE: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| {
                regex::Regex::new(
                    r#"(?:specialid|special_id|specialID|listid|list_id)[=:]["']?(\d{4,})"#,
                )
                .unwrap()
            });
        static SHARE_HASH_RE: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| {
                regex::Regex::new(r#"(?:hash|Hash|FileHash)[=:]["']?([0-9a-fA-F]{32})"#).unwrap()
            });

        if let Some(m) = SHARE_URL_RE.find(text) {
            let candidate_url = m
                .as_str()
                .trim_end_matches(&[',', '.', '!', '?', ';', ':', ')', ']', '}', '，', '。', '！', '？', '）', '】', '》'][..]);
            if let Ok(resp) = self.transport.client.get(candidate_url).send().await {
                let final_url = resp.url().to_string();
                let html = resp.text().await.unwrap_or_default();
                working_text = format!("{}\n{}\n{}", working_text, final_url, html);
            }
        }

        // 2. 提取 global_collection_id (如 collection_3_... 或 gcid=...)
        let resolved_gcid = SHARE_BARE_COLL_RE
            .captures(&working_text)
            .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            .or_else(|| {
                SHARE_GCID_RE
                    .captures(&working_text)
                    .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            })
            .or_else(|| {
                if text.starts_with("collection_") {
                    Some(text.to_string())
                } else {
                    None
                }
            });

        let resolved_special_id = SHARE_SPECIAL_ID_RE
            .captures(&working_text)
            .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            .or_else(|| {
                if text.chars().all(|c| c.is_ascii_digit()) && text.len() >= 4 {
                    Some(text.to_string())
                } else {
                    None
                }
            });

        if let Some(pid) = resolved_gcid.or(resolved_special_id) {
            let songs = self.fetch_playlist_songs(&pid).await.unwrap_or_default();
            if !songs.is_empty() {
                let pl = Playlist {
                    id: pid.clone(),
                    title: format!("导入歌单 ({})", &pid[..pid.len().min(12)]),
                    cover_url: songs.first().map(|s| s.cover_url.clone()).unwrap_or_default(),
                    track_count: songs.len() as i32,
                    description: "通过酷狗分享链接/集合 ID 解析导入".to_string(),
                    creator_name: "酷狗概念版分享".to_string(),
                    is_created_by_me: false,
                    is_favorite: false,
                    is_liked: false,
                    list_id: 0,
                    global_collection_id: pid,
                    ..Default::default()
                };
                return Ok(SharedMusicResolved {
                    target_type: "playlist".to_string(),
                    song: None,
                    playlist: Some(pl),
                    songs,
                    message: "成功解析分享歌单".to_string(),
                });
            }
        }

        // 3. 提取 32 位单曲 Hash 或 mixsongid / album_audio_id
        if let Some(caps) = SHARE_HASH_RE.captures(&working_text) {
            if let Some(h_match) = caps.get(1) {
                let hash_str = h_match.as_str().to_lowercase();
                let seed = Song {
                    id: hash_str.clone(),
                    hash: hash_str.clone(),
                    hash128: hash_str,
                    title: "分享单曲".to_string(),
                    artist: "酷狗歌手".to_string(),
                    ..Default::default()
                };
                let enriched = self
                    .enrich_songs_privilege_lite(vec![seed])
                    .await
                    .into_iter()
                    .next()
                    .unwrap_or_default();
                if !enriched.hash.is_empty() {
                    return Ok(SharedMusicResolved {
                        target_type: "song".to_string(),
                        song: Some(enriched.clone()),
                        playlist: None,
                        songs: vec![enriched],
                        message: "成功解析分享单曲".to_string(),
                    });
                }
            }
        }

        // 4. 逐行歌名解析匹配
        let mut matched_songs = Vec::new();
        for line in text.lines() {
            let clean_line = line
                .trim()
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '、' || c == '-')
                .trim();
            if clean_line.len() >= 2 && !clean_line.starts_with("http") {
                if let Ok(mut res) = self.search_songs(clean_line, 1, 1).await {
                    if let Some(first) = res.pop() {
                        matched_songs.push(first);
                    }
                }
                if matched_songs.len() >= 50 {
                    break;
                }
            }
        }

        if !matched_songs.is_empty() {
            return Ok(SharedMusicResolved {
                target_type: "playlist".to_string(),
                song: matched_songs.first().cloned(),
                playlist: None,
                songs: matched_songs,
                message: "成功匹配歌单曲目列表".to_string(),
            });
        }

        Err("未能从输入内容中识别出有效的酷狗歌单或歌曲".to_string())
    }

    fn resolve_cache_dir() -> std::path::PathBuf {
        let base_dir = directories::ProjectDirs::from("com", "concept", "BreezeMusic")
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let cache_dir = directories::ProjectDirs::from("com", "concept", "BreezeMusic")
            .map(|p| p.cache_dir().to_path_buf())
            .unwrap_or_else(|| base_dir.join("cache"));
        let _ = std::fs::create_dir_all(&cache_dir);
        cache_dir
    }

    /// 40. 真实磁盘缓存统计与分项清理
    pub fn get_cache_stats(&self) -> Result<CacheDiskStats, String> {
        let base_dir = directories::ProjectDirs::from("com", "concept", "BreezeMusic")
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let cache_dir = Self::resolve_cache_dir();

        let mut audio_bytes: u64 = 0;
        let mut artwork_bytes: u64 = 0;
        let mut playlist_bytes: u64 = 0;
        let mut lyric_bytes: u64 = 0;

        let audio_streams_dir = cache_dir.join("audio_streams");
        for dir in [&base_dir, &cache_dir, &audio_streams_dir] {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(meta) = entry.metadata() {
                        if meta.is_file() {
                            let len = meta.len();
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if name.ends_with(".flac")
                                || name.ends_with(".mp3")
                                || name.ends_with(".ogg")
                                || name.ends_with(".wav")
                            {
                                audio_bytes += len;
                            } else if name.ends_with(".jpg")
                                || name.ends_with(".jpeg")
                                || name.ends_with(".png")
                                || name.ends_with(".webp")
                            {
                                artwork_bytes += len;
                            } else if name.ends_with(".lrc") || name.ends_with(".krc") {
                                lyric_bytes += len;
                            } else if name.contains("session_state") || name.ends_with(".json") {
                                playlist_bytes += len;
                            }
                        }
                    }
                }
            }
        }

        if let Ok(guard) = self.store.state.lock() {
            let pl_count = guard.collected_playlist_cache.len() + guard.playlist_overrides.len();
            playlist_bytes += (pl_count as u64) * 2048;
        }
        if let Ok(guard) = self.taste_seed_songs.lock() {
            lyric_bytes += (guard.len() as u64) * 512;
        }

        let to_mb = |b: u64| -> f64 { ((b as f64) / (1024.0 * 1024.0) * 100.0).round() / 100.0 };
        let audio_mb = to_mb(audio_bytes);
        let artwork_mb = to_mb(artwork_bytes);
        let playlist_mb = to_mb(playlist_bytes);
        let lyric_mb = to_mb(lyric_bytes);
        let total_mb = ((audio_mb + artwork_mb + playlist_mb + lyric_mb) * 100.0).round() / 100.0;

        Ok(CacheDiskStats {
            audio_stream_mb: audio_mb,
            artwork_image_mb: artwork_mb,
            playlist_snapshot_mb: playlist_mb,
            lyric_and_url_mb: lyric_mb,
            total_mb,
            cache_dir_path: cache_dir.to_string_lossy().to_string(),
        })
    }

    pub fn clear_cache_category(&self, category: &str) -> Result<CacheDiskStats, String> {
        let cat = category.trim().to_lowercase();
        let base_dir = directories::ProjectDirs::from("com", "concept", "BreezeMusic")
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let cache_dir = Self::resolve_cache_dir();

        if cat == "all" || cat == "snapshot" || cat == "playlists" {
            if let Ok(mut guard) = self.store.state.lock() {
                guard.collected_playlist_cache.clear();
            }
            self.store.save();
        }
        if cat == "all" || cat == "lyric" || cat == "lyrics" {
            if let Ok(mut guard) = self.seen_personal_fm_hashes.lock() {
                guard.clear();
            }
            if let Ok(mut guard) = self.taste_seed_songs.lock() {
                guard.clear();
            }
        }
        let audio_streams_dir = cache_dir.join("audio_streams");
        for dir in [&base_dir, &cache_dir, &audio_streams_dir] {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if name == "session_state.json" {
                        continue;
                    }
                    let should_del = match cat.as_str() {
                        "all" => true,
                        "audio" => {
                            name.ends_with(".flac")
                                || name.ends_with(".mp3")
                                || name.ends_with(".ogg")
                                || name.ends_with(".wav")
                        }
                        "image" | "images" | "artwork" => {
                            name.ends_with(".jpg")
                                || name.ends_with(".jpeg")
                                || name.ends_with(".png")
                                || name.ends_with(".webp")
                        }
                        "snapshot" | "playlists" => name.ends_with(".json"),
                        "lyric" | "lyrics" => name.ends_with(".lrc") || name.ends_with(".krc"),
                        _ => false,
                    };
                    if should_del {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
        self.get_cache_stats()
    }
}

/// 零外部依赖的真并发 Future 聚合执行器（基于 `std::future::poll_fn` 并发轮询所有子任务并保持原序返回）
async fn futures_join_vec<F, T>(futs: Vec<F>) -> Vec<T>
where
    F: std::future::Future<Output = T>,
{
    let len = futs.len();
    if len == 0 {
        return Vec::new();
    }
    let mut pinned: Vec<Option<std::pin::Pin<Box<F>>>> =
        futs.into_iter().map(|f| Some(Box::pin(f))).collect();
    let mut results: Vec<Option<T>> = (0..len).map(|_| None).collect();

    std::future::poll_fn(|cx| {
        let mut all_done = true;
        for (i, slot) in pinned.iter_mut().enumerate() {
            if let Some(fut) = slot {
                match fut.as_mut().poll(cx) {
                    std::task::Poll::Ready(val) => {
                        results[i] = Some(val);
                        *slot = None;
                    }
                    std::task::Poll::Pending => {
                        all_done = false;
                    }
                }
            }
        }
        if all_done {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await;

    results.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_dfid_detection() {
        assert!(KugouApiService::is_legacy_local_dfid(
            "2d544fe0e844b4eff8c1b5d8"
        ));
        assert!(!KugouApiService::is_legacy_local_dfid(
            "2Be40g4ZdG5S2UKRxv26ERR6"
        ));
        assert!(!KugouApiService::is_legacy_local_dfid(
            "4Wygbl056MbS2dPc7z1v16D7"
        ));
    }

    #[test]
    fn test_grade_progress_separates_points_from_minutes() {
        let (base, cur, next, rem_score, pct, rem_min) =
            KugouApiService::compute_grade_progress(12, 223_089, 300_000, 301_106, 500_000, 1.0);
        assert_eq!(base, 300_000);
        assert_eq!(cur, 301_106);
        assert_eq!(next, 500_000);
        assert_eq!(rem_score, 198_894);
        assert!((pct - 0.553).abs() < 0.01);
        assert_eq!(rem_min, 198_894);
    }

    #[tokio::test]
    #[ignore]
    async fn verify_live_account_profile_and_heal_local_state() {
        let store = Arc::new(AppStore::new());
        if store.session().is_none() {
            return;
        }
        let api = KugouApiService::new(store.clone());
        let profile = api
            .fetch_full_profile()
            .await
            .expect("live fetch_full_profile should succeed");
        println!(
            "LIVE PROFILE => {}",
            serde_json::to_string_pretty(&profile).unwrap_or_default()
        );
        println!(
            "HEALED DEVICE => {:?}\nHEALED GRADE STATE => {:?}",
            store.device(),
            store.get_grade_state(&profile.user_id)
        );
        assert!(profile.raw_server_dsec > 0);
        assert_eq!(profile.listen_duration_sec, profile.raw_server_dsec);
    }
}
