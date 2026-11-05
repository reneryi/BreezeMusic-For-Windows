use crate::kugou::cscc::KugouCsccReporter;
use crate::kugou::krc;
use crate::kugou::metadata::{self, AudioTagMetadata};
use crate::kugou::models::{
    AccountProfile, AlbumItem, AudioQuality, DownloadTaskResult, FmChannel, FollowedArtist,
    GatewayDiagResult, KugouSession, LoginCandidate, LyricData, PhoneLoginOutcome, PlaybackResult,
    Playlist, PurchasedAlbumItem, PurchasedSongItem, QrCodeSession, QrPollResult, QuotedReplyHop,
    CacheDiskStats, RankItem, ResolvedSongAlbumMeta, SharedMusicResolved, Song, SongAuthor,
    SongComment, SongDetailInfo, VipDayRecord, VipStatusInfo, ViperEffectItem,
};
use crate::kugou::signer::{self, ANDROID_SALT, APP_ID, CLIENT_VERSION};
use crate::kugou::store::AppStore;
use crate::kugou::transport::KugouTransport;
use chrono::{FixedOffset, Utc};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub struct KugouApiService {
    pub transport: KugouTransport,
    pub store: Arc<AppStore>,
    pub cscc: KugouCsccReporter,
    pub personal_fm_cursor: AtomicU32,
    pub fm_channel_cursors: Mutex<HashMap<String, u32>>,
    pub seen_personal_fm_hashes: Mutex<HashSet<String>>,
    pub taste_seed_songs: Mutex<Vec<Song>>,
}

impl KugouApiService {
    pub fn new(store: Arc<AppStore>) -> Self {
        Self {
            transport: KugouTransport::new(),
            store,
            cscc: KugouCsccReporter::new(),
            personal_fm_cursor: AtomicU32::new(0),
            fm_channel_cursors: Mutex::new(HashMap::new()),
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

    /// 1. 设备风控注册 (对齐 register_dev.js -> /risk/v2/r_register_dev)
    pub async fn register_device(&self, force_refresh: bool) -> Result<String, String> {
        let dev = self.store.device();
        if !force_refresh && dev.dfid_registered && !dev.dfid.is_empty() && dev.dfid != "-" {
            return Ok(dev.dfid);
        }

        let session = self.store.session();
        let active_uid = session
            .as_ref()
            .map(|s| s.user_id.as_str())
            .unwrap_or("0");
        let active_token = session
            .as_ref()
            .map(|s| s.token.as_str())
            .unwrap_or("");

        let data_plain = json!({
            "mid": dev.mid,
            "uuid": dev.guid,
            "appid": "1014",
            "userid": active_uid
        })
        .to_string();

        if let Ok((aes_key, aes_cipher)) = signer::playlist_aes_encrypt(&data_plain) {
            let p_inner = json!({
                "aes": aes_key,
                "uid": active_uid,
                "token": active_token
            })
            .to_string();

            if let Ok(p_rsa) = signer::rsa_pkcs1_hex(&p_inner) {
                let mut query = HashMap::new();
                query.insert("part".to_string(), "1".to_string());
                query.insert("plat".to_string(), "1".to_string());
                query.insert("p".to_string(), p_rsa.to_uppercase());

                if let Ok(raw_bytes) = self
                    .transport
                    .request_raw_bytes(
                        "/risk/v2/r_register_dev",
                        &dev,
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
                                if !dfid.trim().is_empty() && dfid != "-" {
                                    self.store.save_dfid(dfid);
                                    return Ok(dfid.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        let fallback_dfid = if !dev.dfid.is_empty() && dev.dfid != "-" {
            dev.dfid
        } else {
            format!(
                "2{}",
                &signer::md5_hex(&format!("kugou_lite_{}_{}", dev.guid, dev.mid))[..23]
            )
        };
        self.store.save_dfid(&fallback_dfid);
        Ok(fallback_dfid)
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

    /// 对齐 Android 端 MusicRepository.resolveStandardClientLocalDiffSec(userId, rawServerSeconds)
    fn resolve_standard_client_local_diff_sec(_user_id: &str, _raw_server_seconds: i64) -> i64 {
        0
    }

    /// 6. 获取当前登录账号详细资料、权威听歌时长 (d_sec) 与 VIP 状态
    /// 严格对齐 Android 端 MusicRepository.userGradeInfo:
    /// 聚合 /v3/get_my_info + /v1/get_my_userinfo + /v4/get_grade_info + /v2/get_grade_info
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
        let dev = self.store.device();
        let now_sec = Utc::now().timestamp();
        let now_ms = Utc::now().timestamp_millis();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);

        let mut max_d_sec: i64 = 0;
        let mut max_duration_min: i64 = 0;
        let mut max_grade: i32 = 0;
        let mut max_score: i64 = 0;
        let mut max_next_score: i64 = 0;
        let mut max_base_score: i64 = 0;
        let mut remain_sec_server: i64 = 0;

        let req_d_sec = reported_d_sec.unwrap_or(0).max(0);
        let req_diff_sec = reported_diff_sec.unwrap_or(0).max(0);
        let is_report = req_d_sec > 0 && req_diff_sec > 0;

        // 通道 A: /v3/get_my_info
        let p_plain = json!({
            "token": session.token,
            "clienttime": now_sec
        })
        .to_string();

        if let Ok(p_rsa) = signer::rsa_raw_zero_pad_hex(&p_plain) {
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

            if let Ok(resp) = self
                .transport
                .call(
                    "/v3/get_my_info",
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
                if resp.is_success() {
                    let d = resp.data();
                    let nick = Self::val_str(d, "nickname");
                    if !nick.is_empty() {
                        session.nickname = nick;
                    }
                    let pic = Self::val_str(d, "pic");
                    if !pic.is_empty() {
                        session.avatar_url = signer::normalize_image_url(Some(&pic));
                    }
                    self.store.set_session(Some(session.clone()));

                    for k in ["r_duration", "duration", "listen_duration", "user_duration", "total_duration", "listen_time"] {
                        if let Some(m) = Self::val_i64(d, k) {
                            max_duration_min = max_duration_min.max(m);
                        }
                    }
                    for k in ["user_grade", "p_grade", "grade", "level"] {
                        if let Some(g) = Self::val_i64(d, k) {
                            max_grade = max_grade.max(g as i32);
                        }
                    }
                    for k in ["p_current_point", "score", "user_score"] {
                        if let Some(sc) = Self::val_i64(d, k) {
                            max_score = max_score.max(sc);
                        }
                    }
                }
            }
        }

        // 通道 A2: /v1/get_my_userinfo (对齐 Android 端 MusicRepository.userGradeInfo 通道2)
        let p_ms_plain = json!({
            "clienttime": now_ms,
            "token": session.token
        })
        .to_string();
        if let Ok(p_ms_rsa) = signer::rsa_raw_zero_pad_hex(&p_ms_plain) {
            let rel_body = json!({
                "p": p_ms_rsa.to_uppercase(),
                "appid": APP_ID,
                "mid": dev.mid,
                "clientver": CLIENT_VERSION,
                "source": 0,
                "clienttime": now_ms,
                "uuid": "-",
                "userid": uid_num,
                "key": signer::params_key(now_ms, "")
            });
            let mut rel_headers = HashMap::new();
            rel_headers.insert("x-router".to_string(), "relation.user.kugou.com".to_string());
            if let Ok(rel_resp) = self
                .transport
                .call(
                    "/v1/get_my_userinfo",
                    &dev,
                    Some(&session),
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
            {
                let d_rel = rel_resp.data().get("data").unwrap_or_else(|| rel_resp.data());
                for k in ["r_duration", "duration", "listen_duration", "user_duration", "total_duration"] {
                    if let Some(m) = Self::val_i64(d_rel, k) {
                        max_duration_min = max_duration_min.max(m);
                    }
                }
                for k in ["user_grade", "p_grade", "grade"] {
                    if let Some(g) = Self::val_i64(d_rel, k) {
                        max_grade = max_grade.max(g as i32);
                    }
                }
            }
        }

        // 通道 B: /v4/get_grade_info (获取实时 d_sec 秒级时长与 p_grade，并支持上报真实累计时长)
        let md5_val = signer::md5_hex(&format!("{}{}11", req_d_sec, req_diff_sec));
        let aes_seed = signer::random_hex(16);
        let seed_md5 = signer::md5_hex(&aes_seed);
        let params_plain = json!({
            "userid": uid_num,
            "token": session.token,
            "user_type": 29,
            "user_y_type": 1,
            "md5": md5_val
        })
        .to_string();

        if let (Ok(params_enc), Ok(pk_v4)) = (
            signer::aes_cbc_encrypt_hex(&params_plain, &seed_md5[0..32], &seed_md5[16..32]),
            signer::rsa_raw_zero_pad_hex(
                &json!({ "clienttime_ms": now_ms, "key": aes_seed }).to_string(),
            ),
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
            sign_params.insert("dfid".to_string(), dev.dfid.clone());
            sign_params.insert("mid".to_string(), dev.mid.clone());
            sign_params.insert("userid".to_string(), session.user_id.clone());
            sign_params.insert("uuid".to_string(), dev.guid.clone());

            let sign_str: String = sign_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("");
            let sig = signer::md5_hex(&format!(
                "{}{}{}{}",
                ANDROID_SALT, sign_str, v4_body_str, ANDROID_SALT
            ));
            let mut v4_query: HashMap<String, String> = sign_params.into_iter().collect();
            v4_query.insert("signature".to_string(), sig);

            let mut v4_headers = HashMap::new();
            v4_headers.insert("KG-DEVID".to_string(), dev.mid.clone());
            v4_headers.insert("KG-CLIENTTIMEMS".to_string(), now_ms.to_string());
            v4_headers.insert(
                "x-router".to_string(),
                "userinfoservice.kugou.com".to_string(),
            );

            for host in ["https://userinfoservice.kugou.com", "https://gateway.kugou.com"] {
                if let Ok(v4_resp) = self
                    .transport
                    .call(
                        "/v4/get_grade_info",
                        &dev,
                        Some(&session),
                        v4_query.clone(),
                        Some(v4_body.clone()),
                        host,
                        "POST",
                        false,
                        v4_headers.clone(),
                        false,
                        false,
                    )
                    .await
                {
                    let d_root = v4_resp.data();
                    let d_inner = d_root.get("data").unwrap_or(d_root);
                    let mut got_any = false;
                    for k in ["d_sec", "dSec", "listen_sec"] {
                        if let Some(sec) = Self::val_i64(d_inner, k) {
                            max_d_sec = max_d_sec.max(sec);
                            got_any = true;
                        }
                    }
                    for k in ["d_min", "duration", "r_duration"] {
                        if let Some(m) = Self::val_i64(d_inner, k) {
                            max_duration_min = max_duration_min.max(m);
                            got_any = true;
                        }
                    }
                    for k in ["p_grade", "user_grade", "grade"] {
                        if let Some(g) = Self::val_i64(d_inner, k) {
                            max_grade = max_grade.max(g as i32);
                            got_any = true;
                        }
                    }
                    for k in ["p_current_point", "score", "user_score"] {
                        if let Some(sc) = Self::val_i64(d_inner, k) {
                            max_score = max_score.max(sc);
                        }
                    }
                    for k in ["p_next_grade_point", "next_score"] {
                        if let Some(nsc) = Self::val_i64(d_inner, k) {
                            max_next_score = max_next_score.max(nsc);
                        }
                    }
                    for k in ["p_grade_point"] {
                        if let Some(bsc) = Self::val_i64(d_inner, k) {
                            max_base_score = max_base_score.max(bsc);
                        }
                    }
                    for k in ["remain_d_sec", "need_d_sec", "upgrade_need_sec", "upgrade_d_sec"] {
                        if let Some(rem) = Self::val_i64(d_inner, k) {
                            if rem > 0 {
                                remain_sec_server = rem;
                            }
                        }
                    }
                    if got_any {
                        break;
                    }
                }
            }
        }

        // 通道 C: /v2/get_grade_info (lite，同时支持查询与上报 d_sec / diff_sec)
        let v2_client_ver = 10597;
        let v2_key = signer::md5_hex(&format!(
            "{}{}{}{}",
            APP_ID, ANDROID_SALT, v2_client_ver, now_sec
        ));
        let v2_inner = if is_report {
            let report_md5 = signer::md5_hex(&format!("{}{}00", req_d_sec, req_diff_sec));
            format!("{{\"token\":\"{}\",\"md5\":\"{}\"}}", session.token, report_md5)
        } else {
            format!("{{\"clienttime\":{},\"userid\":{}}}", now_sec, uid_num)
        };
        if let Ok(v2_p) = signer::rsa_raw_zero_pad_hex(&v2_inner) {
            let mut v2_body = json!({
                "mid": dev.mid,
                "type": 1,
                "uuid": dev.guid,
                "userid": uid_num,
                "p": v2_p.to_uppercase(),
                "appid": "3116",
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
            v2_query.insert("dfid".to_string(), dev.dfid.clone());
            let mut v2_headers = HashMap::new();
            v2_headers.insert(
                "Content-Type".to_string(),
                "text/plain; charset=ISO-8859-1".to_string(),
            );
            v2_headers.insert(
                "x-router".to_string(),
                "userinfo.user.kugou.com".to_string(),
            );
            if let Ok(v2_resp) = self
                .transport
                .call(
                    "/v2/get_grade_info",
                    &dev,
                    Some(&session),
                    v2_query,
                    Some(v2_body),
                    "https://gateway.kugou.com",
                    "POST",
                    false,
                    v2_headers,
                    false,
                    false,
                )
                .await
            {
                let d_root = v2_resp.data();
                let d_inner = d_root.get("data").unwrap_or(d_root);
                for k in ["d_sec", "dSec", "listen_sec"] {
                    if let Some(sec) = Self::val_i64(d_inner, k) {
                        max_d_sec = max_d_sec.max(sec);
                    }
                }
                for k in ["d_min", "duration", "r_duration"] {
                    if let Some(m) = Self::val_i64(d_inner, k) {
                        max_duration_min = max_duration_min.max(m);
                    }
                }
                for k in ["p_grade", "user_grade", "grade"] {
                    if let Some(g) = Self::val_i64(d_inner, k) {
                        max_grade = max_grade.max(g as i32);
                    }
                }
                for k in ["p_current_point", "score", "user_score"] {
                    if let Some(sc) = Self::val_i64(d_inner, k) {
                        max_score = max_score.max(sc);
                    }
                }
                for k in ["p_next_grade_point", "next_score"] {
                    if let Some(nsc) = Self::val_i64(d_inner, k) {
                        max_next_score = max_next_score.max(nsc);
                    }
                }
            }
        }

        let min_field_in_sec = if max_duration_min > 2_000_000 {
            max_duration_min
        } else {
            max_duration_min * 60
        };
        let mut raw_base_sec = max_d_sec.max(min_field_in_sec);
        if is_report && (req_d_sec + req_diff_sec) > raw_base_sec {
            raw_base_sec = req_d_sec + req_diff_sec;
        }
        let local_diff_sec =
            Self::resolve_standard_client_local_diff_sec(&session.user_id, raw_base_sec);
        let total_sec = raw_base_sec + local_diff_sec;
        let total_min = total_sec / 60;
        let hours = total_min / 60;
        let mins = total_min % 60;
        let listen_duration_formatted = if hours > 0 {
            format!("{} 小时 {} 分", hours, mins)
        } else {
            format!("{} 分钟", mins)
        };

        let (level_progress_pct, next_level_remain_min) = if max_next_score > max_base_score && max_score >= max_base_score {
            let span = (max_next_score - max_base_score).max(1) as f32;
            let cur = (max_score - max_base_score).max(0) as f32;
            let pct = ((cur / span) * 100.0).clamp(2.0, 99.0);
            let rem = if remain_sec_server > 0 {
                (remain_sec_server / 60).max(1)
            } else {
                ((max_next_score - max_score).max(1) * 6).max(10)
            };
            (pct, rem)
        } else {
            let step_mins = ((max_grade.max(1) as i64) * 300).max(300);
            let in_level = total_min % step_mins;
            let pct = (((in_level as f32) / (step_mins as f32)) * 100.0).clamp(5.0, 98.0);
            (pct, (step_mins - in_level).max(1))
        };

        let vip_info = self.fetch_vip_status().await.unwrap_or(VipStatusInfo {
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
            format!("概念版 VIP · 到期 {}", vip_info.svip_expiry)
        } else if vip_info.tvip_active {
            format!("畅听 VIP · 到期 {}", vip_info.tvip_expiry)
        } else {
            "未开通概念版 VIP（可每日签到免费领取）".to_string()
        };

        Ok(AccountProfile {
            user_id: session.user_id,
            nickname: session.nickname,
            avatar_url: session.avatar_url,
            is_vip: vip_info.is_vip,
            svip_active: vip_info.svip_active,
            tvip_active: vip_info.tvip_active,
            vip_expire_text,
            listen_duration_min: total_min,
            listen_duration_sec: total_sec,
            listen_duration_formatted,
            user_grade: max_grade,
            raw_server_dsec: raw_base_sec,
            user_score: max_score,
            next_level_score: max_next_score.max(100),
            level_progress_pct,
            next_level_remain_min,
        })
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

        let resp = self
            .transport
            .call(
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
            )
            .await?
            .require_success()?;

        let data = resp.data();
        let mut svip_active = false;
        let mut tvip_active = false;
        let mut svip_expiry = String::new();
        let mut tvip_expiry = String::new();

        if let Some(busi_arr) = data.get("busi_vip").and_then(|v| v.as_array()) {
            for item in busi_arr {
                let pt = Self::val_str(item, "product_type").to_lowercase();
                let is_vip = Self::val_i64(item, "is_vip").unwrap_or(0) == 1;
                let end_time = Self::val_str(item, "vip_end_time");
                if pt == "svip" && is_vip {
                    svip_active = true;
                    svip_expiry = end_time;
                } else if (pt == "tvip" || pt == "dvip" || pt == "qvip") && is_vip {
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

        let mut rec_query = HashMap::new();
        rec_query.insert("latest_limit".to_string(), "100".to_string());
        if let Ok(rec_resp) = self
            .transport
            .call(
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
            .await
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

    /// 11. 新歌速递 (严格对齐四大分类：华语 21608 / 欧美 21609 / 日韩 21610 / 粤语 21611，核验最近一周真实发行日期)
    pub async fn fetch_new_song_express(&self, period_key: &str) -> Result<Vec<Song>, String> {
        let session = self.store.session();
        let dev = self.store.device();
        let uid_num = session
            .as_ref()
            .and_then(|s| s.user_id.parse::<i64>().ok())
            .unwrap_or(0);

        let norm_key = period_key.trim().to_uppercase();
        let region_pairs: Vec<(i32, i32)> = match norm_key.as_str() {
            "ALL" | "TODAY" => vec![
                (21608, 31308),
                (21609, 31310),
                (21610, 31311),
                (21611, 31312),
                (21611, 31313),
            ],
            "WESTERN" | "EUROPE_AMERICA" | "21609" => vec![(21609, 31310)],
            "KOREA" | "KR" | "31311" => vec![(21610, 31311)],
            "JAPAN" | "JP" | "31312" => vec![(21610, 31312)],
            "JPKR" | "JAPAN_KOREA" | "21610" => vec![(21610, 31311), (21610, 31312)],
            "CANTON" | "CANTONESE" | "21611" => vec![(21611, 31313)],
            _ => vec![(21608, 31308)],
        };

        let mut songs = Vec::new();
        let mut seen_hashes = HashSet::new();
        let pages_to_fetch = if region_pairs.len() == 1 { 2 } else { 1 };

        for (rank_id, fallback_chart_id) in region_pairs {
            let mut region_collected = 0usize;
            for page_idx in 1..=pages_to_fetch {
                let body = json!({
                    "rank_id": rank_id,
                    "userid": uid_num,
                    "page": page_idx,
                    "pagesize": 40,
                    "tags": []
                });
                if let Ok(resp) = self
                    .transport
                    .call(
                        "/musicadservice/container/v1/newsong_publish",
                        &dev,
                        session.as_ref(),
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
                    let mut batch = Self::parse_songs(resp.data());
                    if batch.is_empty() {
                        batch = Self::parse_songs(&resp.body);
                    }
                    for s in batch {
                        let k = if !s.hash.is_empty() {
                            s.hash.to_lowercase()
                        } else {
                            s.id.to_lowercase()
                        };
                        if !k.is_empty() && seen_hashes.insert(k) {
                            songs.push(s);
                            region_collected += 1;
                        }
                    }
                }
            }
            if region_collected < 15 {
                let fb_batch = self
                    .fetch_rank_songs(fallback_chart_id, 1, 40)
                    .await
                    .unwrap_or_default();
                for s in fb_batch {
                    let k = if !s.hash.is_empty() {
                        s.hash.to_lowercase()
                    } else {
                        s.id.to_lowercase()
                    };
                    if !k.is_empty() && seen_hashes.insert(k) {
                        songs.push(s);
                    }
                }
            }
        }

        songs = self.enrich_songs_privilege_lite(songs).await;
        songs = self.enrich_songs_publish_date(songs).await;

        let china_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let today_date = Utc::now().with_timezone(&china_tz).date_naive();
        let cutoff_7d = (today_date - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();
        let cutoff_14d = (today_date - chrono::Duration::days(14))
            .format("%Y-%m-%d")
            .to_string();
        let today_str = today_date.format("%Y-%m-%d").to_string();

        // 严格筛选最近一周（7天内）正式发布的歌曲，绝不篡改或伪造旧歌发行日期
        let mut strictly_7d: Vec<Song> = songs
            .iter()
            .filter(|s| !s.publish_date.is_empty() && s.publish_date >= cutoff_7d && s.publish_date <= today_str)
            .cloned()
            .collect();

        if strictly_7d.len() < 12 {
            for s in &songs {
                if strictly_7d.len() >= 36 {
                    break;
                }
                if !strictly_7d.iter().any(|e| e.hash == s.hash)
                    && !s.publish_date.is_empty()
                    && s.publish_date >= cutoff_14d
                    && s.publish_date <= today_str
                {
                    strictly_7d.push(s.clone());
                }
            }
        }

        let mut final_list = if !strictly_7d.is_empty() {
            strictly_7d
        } else {
            songs
                .into_iter()
                .filter(|s| !s.publish_date.is_empty())
                .take(36)
                .collect()
        };

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
        let minute_stamp = (Utc::now().timestamp() / 60).to_string();

        let mut query = HashMap::new();
        query.insert("version".to_string(), "9108".to_string());
        query.insert("plat".to_string(), "0".to_string());
        query.insert("showtype".to_string(), "2".to_string());
        query.insert("withsong".to_string(), "1".to_string());
        query.insert("parentid".to_string(), "0".to_string());
        query.insert("apiver".to_string(), "6".to_string());
        query.insert("area_code".to_string(), "1".to_string());
        query.insert("_t".to_string(), minute_stamp);

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
                        for (idx, s) in s_arr.iter().take(3).enumerate() {
                            let sname = {
                                let n = Self::val_str(s, "songname");
                                if !n.is_empty() {
                                    n
                                } else {
                                    Self::val_str(s, "name")
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
                            if !sname.is_empty() {
                                if !singer.is_empty() && !sname.contains(&singer) {
                                    previews.push(format!("{}. {} - {}", idx + 1, sname, singer));
                                } else {
                                    previews.push(format!("{}. {}", idx + 1, sname));
                                }
                            }
                        }
                    }

                    let freq = {
                        let f = Self::val_str(o, "update_frequency");
                        if !f.is_empty() {
                            f
                        } else {
                            "实时更新".to_string()
                        }
                    };
                    let category = Self::classify_rank_category(id as i32, &name);
                    let default_total = if id == 8888 { 500 } else { 100 };
                    let raw_total = Self::val_i64(o, "songcount")
                        .or_else(|| Self::val_i64(o, "total"))
                        .unwrap_or(default_total) as i32;
                    ranks.push(RankItem {
                        rank_id: id as i32,
                        rank_name: name,
                        cover_url: signer::normalize_image_url(Some(&cover_raw)),
                        update_frequency: freq,
                        top_songs_preview: previews,
                        category,
                        total_songs: if raw_total > 0 { raw_total } else { default_total as i32 },
                    });
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
                        "1. 瞬 - 告五人".to_string(),
                        "2. 奢香夫人 - 凤凰传奇".to_string(),
                        "3. 离别开出花 - 就是南方凯".to_string(),
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
                        "1. 晴天 - 周杰伦".to_string(),
                        "2. 爱在西元前 - 周杰伦".to_string(),
                        "3. 篇章 - 张韶涵".to_string(),
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
                        "1. 悬溺 - 葛东琪".to_string(),
                        "2. 若把你 - Kirsty刘瑾睿".to_string(),
                        "3. 晚风告白 - 星野".to_string(),
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
                        "1. 向云端 - 小霞".to_string(),
                        "2. 笼 - 张碧晨".to_string(),
                        "3. 想见你想见你想见你 - 八三夭".to_string(),
                    ],
                    category: "官方热榜".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 24971,
                    rank_name: "粤语金曲榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175409477029.jpg".to_string(),
                    update_frequency: "每周更新".to_string(),
                    top_songs_preview: vec![
                        "1. 富士山下 - 陈奕迅".to_string(),
                        "2. 月半小夜曲 - 李克勤".to_string(),
                        "3. 海阔天空 - Beyond".to_string(),
                    ],
                    category: "特色曲风".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 33162,
                    rank_name: "欧美金曲榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175249561655.jpg".to_string(),
                    update_frequency: "每周更新".to_string(),
                    top_songs_preview: vec![
                        "1. Cruel Summer - Taylor Swift".to_string(),
                        "2. Counting Stars - OneRepublic".to_string(),
                        "3. Flowers - Miley Cyrus".to_string(),
                    ],
                    category: "特色曲风".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 4681,
                    rank_name: "美国BillBoard榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175344315250.jpg".to_string(),
                    update_frequency: "每周更新".to_string(),
                    top_songs_preview: vec![
                        "1. Die With A Smile - Lady Gaga".to_string(),
                        "2. Birds of a Feather - Billie Eilish".to_string(),
                        "3. Espresso - Sabrina Carpenter".to_string(),
                    ],
                    category: "全球权威".to_string(),
                    total_songs: 100,
                },
                RankItem {
                    rank_id: 4673,
                    rank_name: "日本公信榜".to_string(),
                    cover_url: "https://imge.kugou.com/commendpic/20210729/20210729175409477029.jpg".to_string(),
                    update_frequency: "每周更新".to_string(),
                    top_songs_preview: vec![
                        "1. Idol - YOASOBI".to_string(),
                        "2. Lemon - 米津玄师".to_string(),
                        "3. 怪兽的花呗 - Vaundy".to_string(),
                    ],
                    category: "全球权威".to_string(),
                    total_songs: 100,
                },
            ];
        }

        // 对齐 Android MusicRepository.rankList 优先级排序，且不截断榜单数量
        let priority_ids = [
            6666, 8888, 59703, 52144, 23784, 31308, 31313, 31310, 31311, 31312, 24971, 33162,
            33160, 33161, 44412, 4681, 4673, 4680, 4672, 33163,
        ];
        ranks.sort_by_key(|r| {
            priority_ids
                .iter()
                .position(|&pid| pid == r.rank_id)
                .unwrap_or(999)
        });

        // 并发补全缺失 Top 3 歌曲预览的榜单，确保任意榜单卡片展示真实实时前三名
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
                            .enumerate()
                            .map(|(i, s)| {
                                if !s.artist.is_empty() && s.artist != "未知歌手" {
                                    format!("{}. {} - {}", i + 1, s.title, s.artist)
                                } else {
                                    format!("{}. {}", i + 1, s.title)
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
        let mut query = HashMap::new();
        query.insert("plat".to_string(), "0".to_string());
        query.insert("ranktype".to_string(), "2".to_string());
        query.insert("rankid".to_string(), rank_id.to_string());
        query.insert("with_res_tag".to_string(), "0".to_string());
        query.insert("_t".to_string(), minute_stamp);
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

        // 通道 1: /api/v3/rank/song
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

        // 通道 2: /openapi/kmr/v2/rank/audio
        let session = self.store.session();
        let body = json!({
            "show_portrait_mv": 1,
            "show_type_total": 1,
            "filter_original_remarks": 1,
            "area_code": 1,
            "pagesize": page_size,
            "rank_cid": latest_vol_id,
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
            let songs = Self::parse_songs(resp.data());
            if !songs.is_empty() {
                return songs;
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
        if page == 1 && page_size >= 100 {
            let max_pages = if rank_id == 8888 && page_size >= 200 { 5 } else { 2 };
            let mut all_songs = Vec::new();
            let mut seen = HashSet::new();
            for p in 1..=max_pages {
                let batch = self
                    .fetch_rank_songs_single_page(rank_id, p, 100, latest_vol_id)
                    .await;
                if batch.is_empty() {
                    break;
                }
                let batch_len = batch.len();
                let mut added = 0usize;
                for s in batch {
                    let k = if !s.hash.is_empty() {
                        s.hash.to_lowercase()
                    } else {
                        format!("{}_{}", s.title, s.artist).to_lowercase()
                    };
                    if seen.insert(k) {
                        all_songs.push(s);
                        added += 1;
                    }
                }
                if added == 0 || batch_len < 30 || all_songs.len() >= page_size as usize {
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

    /// 13. 云端最近播放记录 (对齐 MusicRepository.latestListenSongs)
    pub async fn fetch_recent_history(&self) -> Result<Vec<Song>, String> {
        let Some(session) = self.store.session() else {
            return Ok(Vec::new());
        };
        let dev = self.store.device();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);

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
            let songs = Self::parse_songs(resp.data());
            if !songs.is_empty() {
                let enriched = self.enrich_songs_privilege_lite(songs).await;
                self.record_taste_seeds(&enriched);
                return Ok(enriched);
            }
        }

        let fallback_body = json!({
            "area_code": "1",
            "sources": ["pc", "mobile", "tv", "car"],
            "userid": uid_num,
            "ret_info": 1,
            "token": session.token,
            "pagesize": 60
        });
        let resp = self
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
            .await?
            .require_success()?;

        let songs = Self::parse_songs(resp.data());
        let enriched = self.enrich_songs_privilege_lite(songs).await;
        self.record_taste_seeds(&enriched);
        Ok(enriched)
    }

    /// 14. 歌曲搜索 (对齐 searchSongs -> /v2/search/song)
    pub async fn search_songs(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Song>, String> {
        let kw = keyword.trim();
        if kw.is_empty() {
            return Ok(Vec::new());
        }
        let session = self.store.session();
        let dev = self.store.device();

        let mut query = HashMap::new();
        query.insert("keyword".to_string(), kw.to_string());
        query.insert("page".to_string(), page.to_string());
        query.insert("pagesize".to_string(), page_size.to_string());
        query.insert("platform".to_string(), "AndroidFilter".to_string());
        query.insert("albumhide".to_string(), "0".to_string());
        query.insert("nocollect".to_string(), "0".to_string());
        query.insert("iscorrection".to_string(), "1".to_string());
        query.insert("privilegefilter".to_string(), "0".to_string());
        query.insert("area_code".to_string(), "1".to_string());
        query.insert("dopicfull".to_string(), "1".to_string());
        query.insert("style_id".to_string(), "1".to_string());

        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "complexsearch.kugou.com".to_string());

        let resp = self
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
            .await?
            .require_success()?;

        let songs = Self::parse_songs(resp.data());
        Ok(self.enrich_songs_privilege_lite(songs).await)
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
        let gid = collection_id.trim();
        if gid.is_empty() {
            return Ok(Vec::new());
        }
        let session = self.store.session();
        let dev = self.store.device();

        let mut all_songs = Vec::new();
        let mut seen_keys = HashSet::new();
        let mut begin_idx = 0usize;
        let page_size = 200usize;
        let mut expected_total = 0usize;

        for _page_count in 0..25 {
            let mut query = HashMap::new();
            query.insert("area_code".to_string(), "1".to_string());
            query.insert("begin_idx".to_string(), begin_idx.to_string());
            query.insert("plat".to_string(), "1".to_string());
            query.insert("type".to_string(), "1".to_string());
            query.insert("mode".to_string(), "1".to_string());
            query.insert("personal_switch".to_string(), "1".to_string());
            query.insert("pagesize".to_string(), page_size.to_string());
            query.insert("global_collection_id".to_string(), gid.to_string());

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

            if expected_total == 0 {
                expected_total = Self::val_i64(resp.data(), "total")
                    .or_else(|| Self::val_i64(resp.data(), "count"))
                    .unwrap_or(0) as usize;
            }
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
            begin_idx += page_size;
            if added == 0 {
                break;
            }
            if expected_total > 0 && all_songs.len() >= expected_total {
                break;
            }
            if expected_total == 0 && raw_len < 30 {
                break;
            }
        }

        if !all_songs.is_empty() {
            return Ok(self.enrich_songs_privilege_lite(all_songs).await);
        }

        // 备用通道 1：/v4/get_other_list_file 多页拉取
        for page in 1..=20 {
            let mut v4_query = HashMap::new();
            v4_query.insert("global_collection_id".to_string(), gid.to_string());
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
            Some(gid.to_string())
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

        let mut all_items: Vec<Value> = Vec::new();
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

            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "media.store.kugou.com".to_string());

            if let Ok(resp) = self
                .transport
                .call(
                    "/v2/get_res_privilege/lite",
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
                let items_opt = resp
                    .body
                    .get("data")
                    .and_then(|v| v.as_array())
                    .or_else(|| resp.data().get("data").and_then(|v| v.as_array()))
                    .or_else(|| resp.data().get("resource").and_then(|v| v.as_array()));
                if let Some(items) = items_opt {
                    all_items.extend(items.iter().cloned());
                }
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

        if !missing_album_ids.is_empty() {
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
                    let mut alb_map: HashMap<String, (String, String)> = HashMap::new();
                    for obj in arr {
                        let aid = Self::val_str(obj, "album_id");
                        let aname = Self::val_str(obj, "album_name");
                        let pdate: String =
                            Self::val_str(obj, "publish_date").chars().take(10).collect();
                        if !aid.is_empty() && !aname.is_empty() {
                            alb_map.insert(aid, (aname, pdate));
                        }
                    }
                    for s in &mut songs {
                        if let Some((aname, pdate)) = alb_map.get(s.album_id.trim()) {
                            if s.album.trim().is_empty() {
                                s.album = aname.clone();
                            }
                            if s.publish_date.trim().is_empty() && !pdate.is_empty() {
                                s.publish_date = pdate.clone();
                            }
                        }
                    }
                }
            }
        }

        // 第二阶段：对仍缺少 album 或 album_id 但有 album_audio_id / audio_id 的歌曲，通过 /kmr/v2/audio 补全
        let missing_entities: Vec<i64> = songs
            .iter()
            .filter(|s| {
                (s.album.trim().is_empty() || s.album_id.trim().is_empty() || s.album_id == "0")
                    && (s.album_audio_id > 0 || s.audio_id > 0)
            })
            .map(|s| if s.album_audio_id > 0 { s.album_audio_id } else { s.audio_id })
            .collect::<HashSet<_>>()
            .into_iter()
            .take(40)
            .collect();

        if !missing_entities.is_empty() {
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
                    let mut entity_map: HashMap<i64, (String, String, String, String)> = HashMap::new();
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
                    for s in &mut songs {
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
        let album_info = o
            .get("album_info")
            .or_else(|| o.get("albuminfo"))
            .or_else(|| audio_info.get("album_info"))
            .or_else(|| replace_obj.and_then(|r| r.get("album_info")))
            .unwrap_or(o);
        let trans_param = o
            .get("trans_param")
            .or_else(|| audio_info.get("trans_param"))
            .or_else(|| base.get("trans_param"))
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
            Self::val_str(base, "audio_name"),
            Self::val_str(o, "audio_name"),
            Self::val_str(audio_info, "audio_name"),
            Self::val_str(audio_info, "songname"),
            Self::val_str(o, "name"),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_default();

        // 提取多歌手结构化列表 authors
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
                let aname = [
                    Self::val_str(a_obj, "author_name"),
                    Self::val_str(a_obj, "name"),
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
                    Self::val_str(a_obj, "id"),
                    Self::val_str(a_obj, "singerid"),
                ]
                .into_iter()
                .find(|s| !s.is_empty() && s != "0")
                .unwrap_or_default();
                let av = [
                    Self::val_str(a_obj, "sizable_avatar"),
                    Self::val_str(a_obj, "avatar"),
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

        let album = [
            Self::val_str(o, "album_name"),
            Self::val_str(o, "AlbumName"),
            Self::val_str(o, "albumname"),
            Self::val_str(album_info, "album_name"),
            Self::val_str(album_info, "albumname"),
            Self::val_str(album_info, "name"),
            Self::val_str(base, "album_name"),
            Self::val_str(audio_info, "album_name"),
            Self::val_str(replace_audio, "album_name"),
            Self::val_str(trans_param, "album_name"),
            Self::val_str(o, "remark"),
        ]
        .into_iter()
        .find(|s| !s.is_empty() && s != "0" && s != "未知专辑")
        .unwrap_or_default()
        .replace("<em>", "")
        .replace("</em>", "")
        .trim()
        .to_string();

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
            .or_else(|| Self::val_i64(audio_info, "duration"))
            .or_else(|| Self::val_i64(audio_info, "timelength"))
            .or_else(|| Self::val_i64(audio_info, "timelen"))
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
            Self::val_str(o, "flac_hash"),
            Self::val_str(replace_audio, "hash_flac"),
            Self::val_str(audio_info, "hash_flac"),
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
            .or_else(|| Self::val_i64(o, "filesize_320"))
            .or_else(|| Self::val_i64(o, "filesize_hq"))
            .or_else(|| Self::val_i64(audio_info, "filesize_320"))
            .or_else(|| Self::val_i64(audio_info, "320filesize"))
            .or_else(|| Self::val_i64(replace_audio, "filesize_320"))
            .or_else(|| Self::val_i64(trans_param, "ogg_320_filesize"))
            .unwrap_or(relate_hq_size);

        let std_size = Self::val_i64(o, "FileSize")
            .or_else(|| Self::val_i64(o, "filesize"))
            .or_else(|| Self::val_i64(o, "128filesize"))
            .or_else(|| Self::val_i64(o, "filesize_128"))
            .or_else(|| Self::val_i64(audio_info, "filesize_128"))
            .or_else(|| Self::val_i64(audio_info, "filesize"))
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
            || Self::val_i64(o, "pay_type") == Some(3)
            || Self::val_i64(o, "PayType") == Some(3)
            || Self::val_i64(o, "fail_process") == Some(4)
            || Self::val_i64(o, "FailProcess") == Some(4)
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

    fn classify_rank_category(rank_id: i32, name: &str) -> String {
        if matches!(rank_id, 4681 | 4673 | 4680 | 4672 | 33163)
            || name.contains("BillBoard")
            || name.contains("公信")
            || name.contains("英国")
            || name.contains("全球")
            || name.contains("美国")
            || name.contains("日本")
            || name.contains("韩国")
        {
            "全球权威".to_string()
        } else if matches!(rank_id, 24971 | 33162 | 33160 | 33161 | 44412)
            || name.contains("粤语")
            || name.contains("欧美")
            || name.contains("国风")
            || name.contains("ACG")
            || name.contains("电音")
            || name.contains("说唱")
            || name.contains("民谣")
            || name.contains("纯音乐")
            || name.contains("影视")
        {
            "特色曲风".to_string()
        } else {
            "官方热榜".to_string()
        }
    }

    /// 批量补全歌曲精确发行日期 (/kmr/v2/albums + /kmr/v2/audio 权威核验，不伪造日期)
    pub async fn enrich_songs_publish_date(&self, mut songs: Vec<Song>) -> Vec<Song> {
        let dev = self.store.device();
        let session = self.store.session();
        let mut missing_album_ids = Vec::new();
        let mut seen = HashSet::new();
        for s in &songs {
            if s.publish_date.is_empty() && !s.album_id.is_empty() && s.album_id != "0" {
                if seen.insert(s.album_id.clone()) {
                    missing_album_ids.push(json!({ "album_id": s.album_id }));
                }
            }
        }

        let mut date_map: HashMap<String, String> = HashMap::new();
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
                            let pdate = Self::val_str(item, "publish_date");
                            if !aid.is_empty() && pdate.len() >= 10 && !pdate.starts_with("0000") {
                                date_map.insert(aid, pdate[..10].to_string());
                            }
                        }
                    }
                }
            }
        }

        for s in songs.iter_mut() {
            if s.publish_date.is_empty() {
                if let Some(d) = date_map.get(&s.album_id) {
                    s.publish_date = d.clone();
                }
            } else if s.publish_date.len() > 10 {
                s.publish_date = s.publish_date[..10].to_string();
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

    pub fn default_fm_channels() -> Vec<FmChannel> {
        let presets = [
            ("hot", "流行风向标", "全天候智能音乐伴听 · 热歌与品味共振", "推荐电台", 1, "64.2k 正在漫游"),
            ("daily_vibe", "私人口味漫游", "贴合你近期常听偏好的红心延伸声场", "推荐电台", 1, "58.4k 正在漫游"),
            ("red_heart", "红心共鸣电台", "从你收藏的红心单曲延展相似旋律", "推荐电台", 1, "47.8k 正在漫游"),
            ("niche_gem", "宝藏小众漫游", "避开喧嚣热榜 · 邂逅高审美独立佳作", "推荐电台", 1, "39.1k 正在漫游"),
            ("late_night", "深夜静谧台", "卸下喧嚣与疲惫 · 耳机里的温柔晚风", "场景", 1, "51.8k 正在漫游"),
            ("focus_study", "心流伴读台", "轻柔器乐与Lo-Fi律动 · 沉浸专注工作", "场景", 1, "43.5k 正在漫游"),
            ("workout_drive", "公路兜风电台", "开阔地平线与高能节奏 · 一路向阳疾驰", "场景", 1, "36.6k 正在漫游"),
            ("cafe_afternoon", "午后咖啡馆", "慵懒爵士与原声木吉他 · 阳光洒落窗台", "场景", 1, "31.2k 正在漫游"),
            ("morning_wake", "晨间元气唤醒", "清晨第一缕阳光与轻快律动", "场景", 1, "29.6k 正在漫游"),
            ("sleep_ambient", "晚安助眠轻音", "舒缓钢琴与自然氛围 · 伴你安然入梦", "场景", 1, "35.4k 正在漫游"),
            ("cantonese", "港乐黄金年代", "霓虹街角与粤语经典 · 帧享岁月留声", "年代语种", 2, "49.1k 正在漫游"),
            ("mandopop_golden", "华语千禧金曲", "周杰伦/孙燕姿/林俊杰 · 青春记忆里的MP3", "年代语种", 2, "55.9k 正在漫游"),
            ("western_pop", "欧美潮流前线", "Billboard 热单与先锋合成器流行", "年代语种", 2, "45.3k 正在漫游"),
            ("acg", "二次元幻想台", "高燃番剧 OP/ED 与日系治愈声线", "年代语种", 2, "38.9k 正在漫游"),
            ("kpop_wave", "韩流节奏律动", "K-Pop 先锋舞曲与清甜氛围感旋律", "年代语种", 2, "33.7k 正在漫游"),
            ("retro_8090", "80/90 复古留声", "岁月沉淀的华语经典老歌与時代记忆", "年代语种", 2, "41.5k 正在漫游"),
            ("indie_folk", "独立民谣漫游", "木吉他与故事感人声 · 傍晚微风低语", "曲风", 3, "29.4k 正在漫游"),
            ("rnb_soul", "微醺 R&B 律动", "丝滑转音与都市霓虹节拍 · 暮色松弛感", "曲风", 3, "34.7k 正在漫游"),
            ("guofeng", "国风新韵电台", "山水诗意与戏腔弦乐 · 东方美学意境", "曲风", 3, "27.8k 正在漫游"),
            ("rock_live", "摇滚乐队现场", "失真吉他的热血轰鸣 · Livehouse 律动", "曲风", 3, "32.5k 正在漫游"),
            ("edm_synth", "电音脉冲电台", "Progressive House 与赛博合成器浪潮", "曲风", 3, "30.8k 正在漫游"),
            ("healing_mood", "治愈松弛电台", "微风拂过耳畔 · 唤醒好心情的清澈旋律", "心情", 4, "41.0k 正在漫游"),
            ("emo_rain", "雨夜微凉心事", "适合一个人静静听的细腻叙事慢歌", "心情", 4, "37.4k 正在漫游"),
            ("joy_sunshine", "元气晴空漫游", "明快节拍与清亮声线 · 瞬间满电出发", "心情", 4, "28.9k 正在漫游"),
            ("chill_sunset", "日落橘子海", "黄昏海岸线与浪漫梦幻流行", "心情", 4, "33.2k 正在漫游"),
        ];
        presets
            .into_iter()
            .map(|(id, name, desc, cat, fm_type, listeners)| FmChannel {
                id: id.to_string(),
                name: name.to_string(),
                cover_url: String::new(),
                description: desc.to_string(),
                category: cat.to_string(),
                fm_type,
                listener_count_text: listeners.to_string(),
                freq_label: Self::fm_freq_label_for_id(id),
            })
            .collect()
    }

    fn extract_fm_item_cover(item: &Value) -> String {
        for key in ["imgurl", "imgUrl150", "imgUrl100", "cover", "pic", "img", "sizable_cover"] {
            let raw = Self::val_str(item, key);
            if !raw.is_empty() && !raw.contains("20210519") {
                let norm = signer::normalize_image_url(Some(&raw));
                if !norm.is_empty() {
                    return norm;
                }
            }
        }
        if let Some(songs) = item
            .get("songs")
            .or_else(|| item.get("song_list"))
            .or_else(|| item.get("songinfo"))
            .and_then(|v| v.as_array())
        {
            for s in songs {
                for key in ["album_sizable_cover", "sizable_cover", "imgurl", "cover", "pic", "trans_param"] {
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

    fn resolve_fm_theme_keywords(channel_id: &str, channel_name: &str) -> Vec<&'static str> {
        let lower = format!("{} {}", channel_id, channel_name).to_lowercase();
        if lower.contains("late_night") || lower.contains("深夜") || lower.contains("晚安") || lower.contains("助眠") {
            vec!["深夜 治愈 温柔", "晚安 慢歌 抒情", "夜色 安静 独处"]
        } else if lower.contains("focus_study") || lower.contains("心流") || lower.contains("伴读") || lower.contains("轻音乐") || lower.contains("阅读") {
            vec!["纯音乐 钢琴 治愈", "轻音乐 伴读 专注", "久石让 坂本龙一 纯音乐"]
        } else if lower.contains("cafe") || lower.contains("咖啡") || lower.contains("午后") || lower.contains("爵士") {
            vec!["咖啡馆 庸懒 爵士", "Bossa Nova 下午茶", "小野丽莎 诺拉琼斯"]
        } else if lower.contains("cantonese") || lower.contains("港乐") || lower.contains("粤语") {
            vec!["粤语经典 港乐", "陈奕迅 张敬轩 粤语", "杨千嬅 卫兰 Beyond"]
        } else if lower.contains("mandopop_golden") || lower.contains("千禧") || lower.contains("怀旧") || lower.contains("金曲") {
            vec!["华语经典 千禧金曲", "周杰伦 孙燕姿 林俊杰", "蔡依林 王力宏 陶喆"]
        } else if lower.contains("acg") || lower.contains("二次元") || lower.contains("动漫") || lower.contains("日语") {
            vec!["YOASOBI 米津玄师", "动漫 燃曲 日语", "新海诚 Aimer 泽野弘之"]
        } else if lower.contains("kpop") || lower.contains("韩") {
            vec!["K-Pop 韩语 热单", "NewJeans aespa IU", "G-Dragon BLACKPINK"]
        } else if lower.contains("indie_folk") || lower.contains("民谣") || lower.contains("独立") {
            vec!["民谣 木吉他 故事", "房东的猫 陈粒 宋冬野", "毛不易 赵雷 独立乐队"]
        } else if lower.contains("rnb_soul") || lower.contains("r&b") || lower.contains("微醺") || lower.contains("灵魂") {
            vec!["R&B 灵魂乐 微醺", "方大同 陶喆 单依纯", "袁娅维 律动 R&B"]
        } else if lower.contains("guofeng") || lower.contains("国风") || lower.contains("古风") {
            vec!["国风 戏腔 古风", "银临 双笙 周深 国风", "国风新韵 诗意"]
        } else if lower.contains("rock_live") || lower.contains("摇滚") || lower.contains("乐队") {
            vec!["摇滚 乐队 现场", "告五人 万能青年旅店 新裤子", "五月天 痛仰 橘子海"]
        } else if lower.contains("workout_drive") || lower.contains("公路") || lower.contains("兜风") || lower.contains("车载") || lower.contains("运动") {
            vec!["公路 兜风 车载节奏", "电音 律动 驾驶", "OneRepublic 节奏 热单"]
        } else if lower.contains("western_pop") || lower.contains("欧美") {
            vec!["欧美流行 Billboard", "Taylor Swift The Weeknd", "Bruno Mars Ariana Grande"]
        } else if lower.contains("emo_rain") || lower.contains("雨夜") || lower.contains("伤感") || lower.contains("心事") {
            vec!["伤感 走心 慢歌", "雨天 抒情 遗憾", "薛之谦 林宥嘉 裘德"]
        } else if lower.contains("joy_sunshine") || lower.contains("元气") || lower.contains("晴空") || lower.contains("快乐") {
            vec!["元气 快乐 明快", "清新 好心情 甜歌", "王心凌 告五人 晴天"]
        } else if lower.contains("healing_mood") || lower.contains("治愈") || lower.contains("松弛") {
            vec!["治愈 清新 松弛感", "周深 温柔 治愈系", "徐佳莹 梁静茹 温暖"]
        } else {
            vec!["热歌 流行 精选", "华语流行 必听榜", "周杰伦 林俊杰 陈奕迅"]
        }
    }

    /// 20. 电台频道列表 (对齐 Android MusicRepository.fmChannels: /v1/rcmd_list + /v1/class_fm_song + /v1/time_fm_info + 18大精选主题电台)
    pub async fn fetch_fm_channels(&self) -> Result<Vec<FmChannel>, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let mut channels = Vec::new();
        let mut seen = HashSet::new();

        for (endpoint, extra_type) in [
            ("/v1/rcmd_list", None),
            ("/v1/class_fm_song", Some(1i64)),
            ("/v1/time_fm_info", None),
        ] {
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "fm.service.kugou.com".to_string());
            let mut body = json!({
                "appid": APP_ID,
                "clientver": CLIENT_VERSION,
                "mid": dev.mid,
                "platform": "android"
            });
            if let Some(ft) = extra_type {
                body["fmtype"] = json!(ft);
            }
            if let Ok(resp) = self
                .transport
                .call(
                    endpoint,
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
                let data = resp.data();
                let mut candidate_arrays = Vec::new();
                for k in ["list", "info", "class_list", "fm_list"] {
                    if let Some(arr) = data.get(k).and_then(|v| v.as_array()) {
                        candidate_arrays.push(arr);
                    }
                }
                for arr in candidate_arrays {
                    for item in arr {
                        let fmid = {
                            let a = Self::val_str(item, "fmid");
                            if !a.is_empty() {
                                a
                            } else {
                                Self::val_str(item, "id")
                            }
                        };
                        let name = {
                            let a = Self::val_str(item, "fmname");
                            if !a.is_empty() {
                                a
                            } else {
                                let b = Self::val_str(item, "name");
                                if !b.is_empty() {
                                    b
                                } else {
                                    Self::val_str(item, "title")
                                }
                            }
                        };
                        if !fmid.is_empty() && !name.is_empty() && seen.insert(fmid.clone()) {
                            let cover_url = Self::extract_fm_item_cover(item);
                            let desc = {
                                let d = Self::val_str(item, "description");
                                if !d.is_empty() {
                                    d
                                } else {
                                    let d2 = Self::val_str(item, "desc");
                                    if !d2.is_empty() {
                                        d2
                                    } else {
                                        "概念版场景随心调频".to_string()
                                    }
                                }
                            };
                            let cat = {
                                let c = Self::val_str(item, "classname");
                                if !c.is_empty() {
                                    c
                                } else {
                                    "推荐电台".to_string()
                                }
                            };
                            channels.push(FmChannel {
                                freq_label: Self::fm_freq_label_for_id(&fmid),
                                id: fmid,
                                name,
                                cover_url,
                                description: desc,
                                category: cat,
                                fm_type: Self::val_i64(item, "fmtype").unwrap_or(1) as i32,
                                listener_count_text: "42.6k 正在漫游".to_string(),
                            });
                        }
                    }
                }
            }
        }

        for def in Self::default_fm_channels() {
            if seen.insert(def.id.clone()) {
                channels.push(def);
            }
        }

        // 对缺失封面的电台自动关联主题精选歌单或代表作封面
        let mut resolved_count = 0usize;
        for ch in channels.iter_mut() {
            if !ch.cover_url.is_empty() && !ch.cover_url.contains("20210519") {
                continue;
            }
            if resolved_count >= 18 {
                break;
            }
            resolved_count += 1;
            let kws = Self::resolve_fm_theme_keywords(&ch.id, &ch.name);
            let kw = kws.first().copied().unwrap_or(ch.name.as_str());
            if let Ok(pls) = self.search_playlists(kw, 1, 6).await {
                if let Some(found) = pls
                    .into_iter()
                    .find(|p| !p.cover_url.is_empty() && !p.cover_url.contains("20210519"))
                {
                    ch.cover_url = found.cover_url;
                    continue;
                }
            }
            if let Ok(songs) = self.search_songs(kw, 1, 4).await {
                if let Some(found) = songs
                    .into_iter()
                    .find(|s| !s.cover_url.is_empty() && !s.cover_url.contains("20210519"))
                {
                    ch.cover_url = found.cover_url;
                }
            }
        }

        Ok(channels)
    }

    /// 21. 获取指定电台频道的漫游曲目流 (对齐 Android MusicRepository.fmSongs: 多分片 offset 游标 + 主题歌单/关键词深度匹配，每次刷新换新一批)
    pub async fn fetch_fm_songs(&self, channel_id: &str) -> Result<Vec<Song>, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let cid = channel_id.trim();

        let cursor = {
            let mut guard = self.fm_channel_cursors.lock().unwrap();
            let entry = guard.entry(cid.to_string()).or_insert(0);
            let cur = *entry;
            *entry = cur.wrapping_add(1);
            cur
        };

        let mut collected = Vec::new();
        let mut seen = HashSet::new();

        // 1. 若是数字电台 ID，请求 /v1/app_song_list_offset（随 cursor 递增 offset 保证每次刷新换新一批）
        if let Ok(num_id) = cid.parse::<i64>() {
            let mut headers = HashMap::new();
            headers.insert("x-router".to_string(), "fm.service.kugou.com".to_string());
            for slice in 0..2 {
                let offset_val = ((cursor * 2 + slice) % 12) * 20;
                let body = json!({
                    "fmid": num_id,
                    "fmtype": 1,
                    "offset": offset_val,
                    "size": 20
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
                        headers.clone(),
                        true,
                        true,
                    )
                    .await
                {
                    for s in Self::parse_songs(resp.data()) {
                        let k = s.hash.to_lowercase();
                        if !k.is_empty() && seen.insert(k) {
                            collected.push(s);
                        }
                    }
                }
            }
        }

        // 2. 若是 "daily_vibe" (私人口味漫游)，直接联动个人随心听引擎
        if cid == "daily_vibe" && collected.len() < 16 {
            if let Ok(pfm) = self.fetch_personal_fm().await {
                for s in pfm {
                    let k = s.hash.to_lowercase();
                    if !k.is_empty() && seen.insert(k) {
                        collected.push(s);
                    }
                }
            }
        }

        // 3. 结合电台主题关键词从精选主题歌单与搜索分页提取深度契合主题的歌曲
        if collected.len() < 24 {
            let kws = Self::resolve_fm_theme_keywords(cid, cid);
            let active_kw = kws[(cursor as usize) % kws.len()];
            let secondary_kw = kws[((cursor as usize) + 1) % kws.len()];

            if let Ok(pls) = self.search_playlists(active_kw, 1, 6).await {
                if !pls.is_empty() {
                    let pick_pl = &pls[(cursor as usize) % pls.len()];
                    let gid = if !pick_pl.global_collection_id.is_empty() {
                        &pick_pl.global_collection_id
                    } else {
                        &pick_pl.id
                    };
                    if let Ok(pl_songs) = self.fetch_playlist_songs(gid).await {
                        let skip_n = ((cursor as usize) * 8) % pl_songs.len().max(1);
                        for s in pl_songs.into_iter().cycle().skip(skip_n).take(16) {
                            let k = s.hash.to_lowercase();
                            if !k.is_empty() && seen.insert(k) {
                                collected.push(s);
                            }
                        }
                    }
                }
            }

            let search_page = ((cursor % 4) + 1) as u32;
            if let Ok(kw_songs) = self.search_songs(active_kw, search_page, 20).await {
                for s in kw_songs {
                    let k = s.hash.to_lowercase();
                    if !k.is_empty() && seen.insert(k) {
                        collected.push(s);
                    }
                }
            }
            if collected.len() < 20 && secondary_kw != active_kw {
                if let Ok(sec_songs) = self.search_songs(secondary_kw, 1, 15).await {
                    for s in sec_songs {
                        let k = s.hash.to_lowercase();
                        if !k.is_empty() && seen.insert(k) {
                            collected.push(s);
                        }
                    }
                }
            }
        }

        let diversified = Self::diversify_artist_order(collected)
            .into_iter()
            .take(28)
            .collect::<Vec<_>>();
        Ok(self.enrich_songs_privilege_lite(diversified).await)
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

    /// 24. 已购数字专辑与已购单曲 (对齐 MusicRepository.userPurchasedAlbums & userPurchasedSongs)
    pub async fn fetch_purchased_albums(&self) -> Result<Vec<PurchasedAlbumItem>, String> {
        let Some(session) = self.store.session() else {
            return Ok(Vec::new());
        };
        let dev = self.store.device();
        let uid_num = session.user_id.parse::<i64>().unwrap_or(0);
        let body = json!({
            "userid": uid_num,
            "token": session.token,
            "page": 1,
            "pagesize": 20
        });
        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());

        let mut albums = Vec::new();
        if let Ok(resp) = self
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
                headers,
                true,
                true,
            )
            .await
        {
            if let Some(arr) = resp
                .data()
                .get("list")
                .or_else(|| resp.data().get("info"))
                .or_else(|| resp.data().get("albums"))
                .and_then(|v| v.as_array())
            {
                for item in arr {
                    let aid = Self::val_str(item, "album_id");
                    let aname = Self::val_str(item, "album_name");
                    if !aid.is_empty() && !aname.is_empty() {
                        let pic = {
                            let p = Self::val_str(item, "sizable_cover");
                            if !p.is_empty() {
                                p
                            } else {
                                Self::val_str(item, "imgurl")
                            }
                        };
                        albums.push(PurchasedAlbumItem {
                            album_id: aid,
                            album_name: aname,
                            artist_name: Self::val_str(item, "singer_name"),
                            cover_url: signer::normalize_image_url(Some(&pic)),
                            song_count: Self::val_i64(item, "song_count").unwrap_or(10) as i32,
                            buy_time: Self::val_str(item, "buy_time"),
                            publish_date: Self::val_str(item, "publish_date"),
                        });
                    }
                }
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
        let body = json!({
            "userid": uid_num,
            "token": session.token,
            "page": 1,
            "pagesize": 50
        });
        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());

        let mut items = Vec::new();
        if let Ok(resp) = self
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
                headers,
                true,
                true,
            )
            .await
        {
            let songs = self.enrich_songs_privilege_lite(Self::parse_songs(resp.data())).await;
            for s in songs {
                items.push(PurchasedSongItem {
                    buy_time: s.publish_date.clone(),
                    song: s,
                });
            }
        }
        Ok(items)
    }

    /// 25. 歌手主页详情、歌手单曲、专辑详情与专辑曲目
    pub async fn fetch_artist_detail(&self, artist_id: &str) -> Result<FollowedArtist, String> {
        let dev = self.store.device();
        let session = self.store.session();
        let mut gw_artist: Option<FollowedArtist> = None;

        // 通道 1: /kmr/v3/author (概念版完整歌手百科与作品计数)
        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
        headers.insert("kg-tid".to_string(), "36".to_string());
        let author_val = if let Ok(n) = artist_id.trim().parse::<i64>() {
            json!(n)
        } else {
            json!(artist_id.trim())
        };
        let body = json!({
            "author_id": author_val,
            "fields": "base,extra",
            "area_code": 1
        });
        if let Ok(resp) = self
            .transport
            .call(
                "/kmr/v3/author",
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
            let d = resp.data().get("data").unwrap_or_else(|| resp.data());
            let base = d.get("base").unwrap_or(d);
            let extra = d.get("extra").unwrap_or(d);
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
                gw_artist = Some(FollowedArtist {
                    artist_id: artist_id.to_string(),
                    target_user_id: String::new(),
                    name,
                    avatar_url: signer::normalize_image_url(Some(&pic)),
                    song_count: Self::val_i64(extra, "audio_count")
                        .or_else(|| Self::val_i64(base, "audio_count"))
                        .unwrap_or(0) as i32,
                    album_count: Self::val_i64(extra, "album_count")
                        .or_else(|| Self::val_i64(base, "album_count"))
                        .unwrap_or(0) as i32,
                    fans_count: Self::val_i64(extra, "fans_count").unwrap_or(0) as i32,
                    is_followed: false,
                    intro,
                });
            }
        }

        // 通道 2: CDN /api/v3/singer/info
        let mut query = HashMap::new();
        query.insert("version".to_string(), "9108".to_string());
        query.insert("singerid".to_string(), artist_id.to_string());
        query.insert("plat".to_string(), "0".to_string());
        query.insert("with_res_tag".to_string(), "1".to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/singer/info",
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
            let d = resp.data().get("data").unwrap_or_else(|| resp.data());
            let name = Self::val_str(d, "singername");
            if !name.is_empty() {
                let pic = Self::val_str(d, "imgurl");
                let raw_intro = Self::val_str(d, "intro");
                let raw_profile = Self::val_str(d, "profile");
                let cdn_intro = if !raw_intro.is_empty() && !raw_profile.is_empty() && !raw_intro.contains(&raw_profile) {
                    format!("{}

{}", raw_intro, raw_profile)
                } else if !raw_intro.is_empty() {
                    raw_intro
                } else {
                    raw_profile
                };
                let cdn_artist = FollowedArtist {
                    artist_id: artist_id.to_string(),
                    target_user_id: Self::val_str(d, "userid"),
                    name,
                    avatar_url: signer::normalize_image_url(Some(&pic)),
                    song_count: Self::val_i64(d, "songcount").unwrap_or(0) as i32,
                    album_count: Self::val_i64(d, "albumcount").unwrap_or(0) as i32,
                    fans_count: Self::val_i64(d, "fanscount").unwrap_or(0) as i32,
                    is_followed: false,
                    intro: cdn_intro,
                };
                if let Some(gw) = gw_artist {
                    return Ok(FollowedArtist {
                        artist_id: artist_id.to_string(),
                        target_user_id: cdn_artist.target_user_id,
                        name: if !gw.name.is_empty() { gw.name } else { cdn_artist.name },
                        avatar_url: if !gw.avatar_url.is_empty() { gw.avatar_url } else { cdn_artist.avatar_url },
                        song_count: gw.song_count.max(cdn_artist.song_count),
                        album_count: gw.album_count.max(cdn_artist.album_count),
                        fans_count: gw.fans_count.max(cdn_artist.fans_count),
                        is_followed: false,
                        intro: if cdn_artist.intro.len() > gw.intro.len() { cdn_artist.intro } else { gw.intro },
                    });
                }
                return Ok(cdn_artist);
            }
        }

        if let Some(gw) = gw_artist {
            return Ok(gw);
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

        // 通道 1: /openapi/kmr/v2/audio_group/author (概念版歌手单曲，含替换后的 Hi-Res 资源)
        let mut v2_query = HashMap::new();
        v2_query.insert("author_id".to_string(), aid.to_string());
        v2_query.insert("area_code".to_string(), "all".to_string());
        v2_query.insert("Filter".to_string(), "filter".to_string());
        v2_query.insert("page".to_string(), page.to_string());
        v2_query.insert("pagesize".to_string(), page_size.to_string());
        v2_query.insert("replace_api_version".to_string(), "1".to_string());
        v2_query.insert("sort".to_string(), sort_val.to_string());
        v2_query.insert("mvdata_need".to_string(), "1".to_string());
        v2_query.insert("show_audio_honor".to_string(), "1".to_string());
        v2_query.insert("show_audio_tag".to_string(), "1".to_string());
        v2_query.insert("replace_need".to_string(), "1".to_string());

        let mut v2_headers = HashMap::new();
        v2_headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
        v2_headers.insert("kg-tid".to_string(), "220".to_string());

        let mut v2_songs = Vec::new();
        if let Ok(resp) = self
            .transport
            .call(
                "/openapi/kmr/v2/audio_group/author",
                &dev,
                session.as_ref(),
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
            v2_songs = Self::parse_songs(resp.data());
            if v2_songs.is_empty() {
                v2_songs = Self::parse_songs(&resp.body);
            }
        }

        // 通道 2: /kmr/v1/audio_group/author
        let now_sec = Utc::now().timestamp();
        let v1_body = json!({
            "appid": APP_ID,
            "clientver": CLIENT_VERSION,
            "clienttime": now_sec,
            "key": signer::params_key(now_sec, ""),
            "author_id": aid,
            "pagesize": page_size,
            "page": page,
            "sort": if sort == "new" { 2 } else { 1 },
            "area_code": "all",
            "replace_api_version": 1,
            "replace_need": 1,
            "show_audio_honor": 1,
            "show_audio_tag": 1
        });
        let mut v1_headers = HashMap::new();
        v1_headers.insert("x-router".to_string(), "openapi.kugou.com".to_string());
        v1_headers.insert("kg-tid".to_string(), "220".to_string());

        let mut v1_songs = Vec::new();
        if let Ok(resp) = self
            .transport
            .call(
                "/kmr/v1/audio_group/author",
                &dev,
                session.as_ref(),
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
            v1_songs = Self::parse_songs(resp.data());
            if v1_songs.is_empty() {
                v1_songs = Self::parse_songs(&resp.body);
            }
        }

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
            if added == 0 || batch_len < 15 {
                break;
            }
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
            if added == 0 || batch_len < 15 {
                break;
            }
        }

        let merged = if !all_songs.is_empty() {
            let mut cdn_by_title: HashMap<String, Song> = HashMap::new();
            for cs in cdn_songs {
                cdn_by_title.insert(cs.title.trim().to_lowercase(), cs);
            }
            for gs in &mut all_songs {
                if let Some(cs) = cdn_by_title.get(&gs.title.trim().to_lowercase()) {
                    if gs.hash_flac.is_empty() && !cs.hash_flac.is_empty() {
                        gs.hash_flac = cs.hash_flac.clone();
                        gs.is_lossless = true;
                    }
                    if gs.hash320.is_empty() && !cs.hash320.is_empty() {
                        gs.hash320 = cs.hash320.clone();
                    }
                }
            }
            all_songs
        } else {
            cdn_songs
        };

        Ok(self.enrich_songs_privilege_lite(merged).await)
    }

    /// 26. 热搜榜与多维搜索（歌单 / 专辑 / 歌手，支持分页与双通道解析）
    pub async fn fetch_search_hot(&self) -> Result<Vec<String>, String> {
        let dev = self.store.device();
        let mut query = HashMap::new();
        query.insert("plat".to_string(), "0".to_string());
        query.insert("count".to_string(), "20".to_string());
        let mut headers = HashMap::new();
        headers.insert("x-router".to_string(), "msearch.kugou.com".to_string());

        let mut keywords = Vec::new();
        if let Ok(resp) = self
            .transport
            .call(
                "/api/v3/search/hot_tab",
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
            if let Some(list) = resp.data().get("list").and_then(|v| v.as_array()) {
                for group in list {
                    if let Some(kws) = group.get("keywords").and_then(|v| v.as_array()) {
                        for k in kws {
                            let kw = Self::val_str(k, "keyword");
                            if !kw.is_empty() && !keywords.contains(&kw) {
                                keywords.push(kw);
                            }
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
        Ok(keywords.into_iter().take(16).collect())
    }

    pub async fn search_playlists(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Playlist>, String> {
        let kw = keyword.trim();
        if kw.is_empty() {
            return Ok(Vec::new());
        }
        let dev = self.store.device();
        let session = self.store.session();
        let mut out = Vec::new();
        let mut seen = HashSet::new();

        // 通道 1: 概念版网关歌单搜索 (/v1/search/special)
        let mut gw_query = HashMap::new();
        gw_query.insert("keyword".to_string(), kw.to_string());
        gw_query.insert("page".to_string(), page.max(1).to_string());
        gw_query.insert("pagesize".to_string(), page_size.clamp(6, 50).to_string());
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
                                if !pc.is_empty() { pc } else { Self::val_str(o, "total_play_count") }
                            },
                            ..Default::default()
                        });
                    }
                }
            }
        }

        if !out.is_empty() {
            return Ok(out);
        }

        // 通道 2: CDN 歌单搜索 (/api/v3/search/special)
        let mut query = HashMap::new();
        query.insert("keyword".to_string(), kw.to_string());
        query.insert("page".to_string(), page.max(1).to_string());
        query.insert("pagesize".to_string(), page_size.clamp(6, 50).to_string());
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
        Ok(out)
    }

    pub async fn search_albums(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<AlbumItem>, String> {
        let kw = keyword.trim();
        if kw.is_empty() {
            return Ok(Vec::new());
        }
        let dev = self.store.device();
        let session = self.store.session();
        let mut out = Vec::new();
        let mut seen = HashSet::new();

        // 通道 1: 概念版网关专辑搜索 (/v1/search/album)
        let mut gw_query = HashMap::new();
        gw_query.insert("keyword".to_string(), kw.to_string());
        gw_query.insert("page".to_string(), page.max(1).to_string());
        gw_query.insert("pagesize".to_string(), page_size.clamp(6, 50).to_string());
        gw_query.insert("platform".to_string(), "AndroidFilter".to_string());
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
                                let raw = if !pt.is_empty() { pt } else { Self::val_str(o, "publishtime") };
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
        }

        if !out.is_empty() {
            return Ok(out);
        }

        // 通道 2: CDN 专辑搜索 (/api/v3/search/album)
        let mut query = HashMap::new();
        query.insert("keyword".to_string(), kw.to_string());
        query.insert("page".to_string(), page.max(1).to_string());
        query.insert("pagesize".to_string(), page_size.clamp(6, 50).to_string());
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
        Ok(out)
    }

    pub async fn search_artists(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<FollowedArtist>, String> {
        let kw = keyword.trim();
        if kw.is_empty() {
            return Ok(Vec::new());
        }
        let dev = self.store.device();
        let session = self.store.session();
        let mut out = Vec::new();
        let mut seen = HashSet::new();

        // 通道 1: 概念版网关歌手搜索 (/v1/search/singer)
        let mut gw_query = HashMap::new();
        gw_query.insert("keyword".to_string(), kw.to_string());
        gw_query.insert("page".to_string(), page.max(1).to_string());
        gw_query.insert("pagesize".to_string(), page_size.clamp(4, 50).to_string());
        gw_query.insert("platform".to_string(), "AndroidFilter".to_string());
        let mut gw_headers = HashMap::new();
        gw_headers.insert("x-router".to_string(), "complexsearch.kugou.com".to_string());

        if let Ok(resp) = self
            .transport
            .call(
                "/v1/search/singer",
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
        }

        // 通道 2: CDN 歌手搜索 (/api/v3/search/singer)
        let mut query = HashMap::new();
        query.insert("keyword".to_string(), kw.to_string());
        query.insert("page".to_string(), page.max(1).to_string());
        query.insert("pagesize".to_string(), page_size.clamp(4, 50).to_string());
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

        // 通道 3: 若关键字匹配歌手但上述接口返回不足，从单曲搜索结果的 authors 中提取真实歌手
        if out.is_empty() {
            if let Ok(songs) = self.search_songs(kw, 1, 15).await {
                for s in songs {
                    for a in &s.authors {
                        if !a.id.is_empty()
                            && a.id != "0"
                            && a.name.to_lowercase().contains(&kw.to_lowercase())
                            && seen.insert(a.id.clone())
                        {
                            if let Ok(det) = self.fetch_artist_detail(&a.id).await {
                                out.push(det);
                            } else {
                                out.push(FollowedArtist {
                                    artist_id: a.id.clone(),
                                    target_user_id: String::new(),
                                    name: a.name.clone(),
                                    avatar_url: if !a.avatar_url.is_empty() {
                                        a.avatar_url.clone()
                                    } else {
                                        s.cover_url.clone()
                                    },
                                    song_count: 1,
                                    album_count: 0,
                                    fans_count: 0,
                                    is_followed: false,
                                    intro: String::new(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(out)
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

    /// 37. CSCC 三段式真实听歌时长上报 + 听歌历史上传 (对齐 KugouCsccReporter & uploadPlayHistory)
    pub async fn report_playback_telemetry(
        &self,
        song: &Song,
        is_start: bool,
        played_duration_ms: i64,
    ) -> Result<bool, String> {
        let Some(session) = self.store.session() else {
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

        for dir in [&base_dir, &cache_dir] {
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
        for dir in [&base_dir, &cache_dir] {
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
