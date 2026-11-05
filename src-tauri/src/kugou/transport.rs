use crate::kugou::models::{DeviceIdentity, KugouSession};
use crate::kugou::signer::{self, APP_ID, CLIENT_VERSION};
use chrono::Utc;
use reqwest::Client;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ApiResponse {
    pub body: Value,
    pub ssa_code: Option<String>,
    pub http_code: u16,
}

impl ApiResponse {
    pub fn data(&self) -> &Value {
        self.body.get("data").unwrap_or(&self.body)
    }

    pub fn code(&self) -> Option<i64> {
        self.body
            .get("error_code")
            .or_else(|| self.body.get("err_code"))
            .or_else(|| self.body.get("errcode"))
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            })
    }

    pub fn status(&self) -> Option<i64> {
        self.body.get("status").and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
    }

    pub fn is_success(&self) -> bool {
        if !(200..=299).contains(&self.http_code) {
            return false;
        }
        if let Some(c) = self.code() {
            if c != 0 {
                return false;
            }
        }
        self.status() == Some(1) || (self.status().is_none() && self.code() == Some(0))
    }

    pub fn require_success(self) -> Result<Self, String> {
        if self.is_success() {
            Ok(self)
        } else {
            let msg = self
                .body
                .get("error_msg")
                .or_else(|| self.body.get("msg"))
                .or_else(|| self.body.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("酷狗接口未确认成功");
            let code_suffix = self.code().map(|c| format!(" ({})", c)).unwrap_or_default();
            Err(format!("{}{}", msg, code_suffix))
        }
    }
}

pub struct KugouTransport {
    pub client: Client,
}

impl KugouTransport {
    pub fn new() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(12))
            .timeout(Duration::from_secs(20))
            .build()
            .expect("Failed to initialize reqwest HTTP client");
        Self { client }
    }

    /// 通过轻量级 HTTP Range (bytes=0-511 或扩大至 bytes=0-4095) 请求读取远程音频流前 N 字节，
    /// 用于 FLAC STREAMINFO / OGG Vorbis / MP3 硬件级真实采样率 (44.1/48/96/192kHz) 与位深 (16/24-bit) 探测
    pub async fn probe_stream_head_bytes(&self, url: &str) -> Option<Vec<u8>> {
        self.probe_stream_head_bytes_range(url, 512).await
    }

    pub async fn probe_stream_head_bytes_range(&self, url: &str, max_bytes: usize) -> Option<Vec<u8>> {
        let trimmed = url.trim();
        if trimmed.is_empty() || max_bytes < 12 {
            return None;
        }
        let end_byte = max_bytes.saturating_sub(1);
        let resp = self
            .client
            .get(trimmed)
            .header("Range", format!("bytes=0-{}", end_byte))
            .header(
                "User-Agent",
                "Android15-1070-11440-46-0-DiscoveryDRADProtocol-wifi",
            )
            .timeout(Duration::from_millis(2800))
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() && resp.status().as_u16() != 206 {
            return None;
        }
        let bytes = resp.bytes().await.ok()?;
        if bytes.len() >= 12 {
            Some(bytes.iter().take(max_bytes).copied().collect())
        } else {
            None
        }
    }

    /// 针对二级子域 TLS 通配符证书限制及国内多运营商网络，构建候选容灾路由端点
    fn build_candidate_endpoints(
        raw_host: &str,
        base_headers: &HashMap<String, String>,
    ) -> Vec<(String, HashMap<String, String>)> {
        let lower = raw_host.trim().to_lowercase();
        let host_name = lower
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("");

        let mut login_router = base_headers.clone();
        login_router.insert("x-router".to_string(), "login.user.kugou.com".to_string());

        let mut qr_router = base_headers.clone();
        qr_router.insert("x-router".to_string(), "login-user.kugou.com".to_string());

        let mut tracker_router = base_headers.clone();
        tracker_router.insert("x-router".to_string(), "tracker.kugou.com".to_string());

        match host_name {
            "login.user.kugou.com" => vec![
                ("http://login.user.kugou.com".to_string(), base_headers.clone()),
                ("https://gateway.kugou.com".to_string(), login_router),
                ("https://login-user.kugou.com".to_string(), base_headers.clone()),
            ],
            "loginserviceretry.kugou.com" => vec![
                (raw_host.to_string(), base_headers.clone()),
                ("https://gateway.kugou.com".to_string(), login_router),
                ("http://login.user.kugou.com".to_string(), base_headers.clone()),
            ],
            "login-user.kugou.com" => vec![
                (raw_host.to_string(), base_headers.clone()),
                ("https://gateway.kugou.com".to_string(), qr_router),
            ],
            "gateway.kugou.com" => vec![
                (raw_host.to_string(), base_headers.clone()),
                ("https://gatewayretry.kugou.com".to_string(), base_headers.clone()),
            ],
            "tracker.kugou.com" => vec![
                (raw_host.to_string(), base_headers.clone()),
                ("http://tracker.kugou.com".to_string(), base_headers.clone()),
                ("https://gateway.kugou.com".to_string(), tracker_router),
            ],
            _ => vec![(raw_host.to_string(), base_headers.clone())],
        }
    }

    pub async fn call(
        &self,
        path: &str,
        device: &DeviceIdentity,
        session: Option<&KugouSession>,
        query: HashMap<String, String>,
        body: Option<Value>,
        host: &str,
        method: &str,
        web: bool,
        headers: HashMap<String, String>,
        sign: bool,
        include_default_params: bool,
    ) -> Result<ApiResponse, String> {
        let candidates = Self::build_candidate_endpoints(host, &headers);
        let mut last_err = String::from("网络连接失败");

        for (idx, (candidate_host, candidate_headers)) in candidates.iter().enumerate() {
            match self
                .execute_single_call(
                    path,
                    device,
                    session,
                    &query,
                    body.as_ref(),
                    candidate_host,
                    method,
                    web,
                    candidate_headers,
                    sign,
                    include_default_params,
                )
                .await
            {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    last_err = e;
                    if idx + 1 < candidates.len() {
                        tokio::time::sleep(Duration::from_millis(150 * (idx as u64 + 1))).await;
                    }
                }
            }
        }
        Err(last_err)
    }

    async fn execute_single_call(
        &self,
        path: &str,
        device: &DeviceIdentity,
        session: Option<&KugouSession>,
        query: &HashMap<String, String>,
        body: Option<&Value>,
        host: &str,
        method: &str,
        web: bool,
        headers: &HashMap<String, String>,
        sign: bool,
        include_default_params: bool,
    ) -> Result<ApiResponse, String> {
        let now_sec = Utc::now().timestamp().to_string();
        let mut params = BTreeMap::new();

        if include_default_params {
            params.insert("dfid".to_string(), device.dfid.clone());
            params.insert("mid".to_string(), device.mid.clone());
            params.insert("uuid".to_string(), "-".to_string());
            params.insert("appid".to_string(), APP_ID.to_string());
            params.insert("clientver".to_string(), CLIENT_VERSION.to_string());
            params.insert("clienttime".to_string(), now_sec.clone());
            if let Some(s) = session {
                if !s.token.is_empty() {
                    params.insert("token".to_string(), s.token.clone());
                }
                if !s.user_id.is_empty() {
                    params.insert("userid".to_string(), s.user_id.clone());
                }
            }
        }

        for (k, v) in query {
            params.insert(k.clone(), v.clone());
        }

        let payload = body
            .map(|b| serde_json::to_string(b).unwrap_or_default())
            .unwrap_or_default();

        if sign {
            let sig = signer::signature(&params, &payload, web);
            params.insert("signature".to_string(), sig);
        }

        let base = host.trim_end_matches('/');
        let clean_path = path.trim_start_matches('/');
        let full_url = format!("{}/{}", base, clean_path);

        let req_method = method.to_uppercase();
        let mut req_builder = match req_method.as_str() {
            "POST" => self.client.post(&full_url),
            _ => self.client.get(&full_url),
        };

        let query_vec: Vec<(&String, &String)> = params.iter().collect();
        req_builder = req_builder.query(&query_vec);

        req_builder = req_builder
            .header(
                "User-Agent",
                "Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi",
            )
            .header("kg-rc", "1")
            .header("kg-thash", "5d816a0")
            .header("kg-rec", "1")
            .header("kg-rf", "B9EDA08A64250DEFFBCADDEE00F8F25F");

        if include_default_params {
            req_builder = req_builder
                .header("dfid", &device.dfid)
                .header("mid", &device.mid)
                .header(
                    "clienttime",
                    params.get("clienttime").cloned().unwrap_or(now_sec),
                );
        }

        let mut has_content_type = false;
        for (k, v) in headers {
            if k.eq_ignore_ascii_case("content-type") {
                has_content_type = true;
            }
            req_builder = req_builder.header(k, v);
        }

        if req_method == "POST" {
            if !has_content_type {
                req_builder = req_builder.header("content-type", "application/json; charset=utf-8");
            }
            req_builder = req_builder.body(payload);
        }

        let resp = req_builder.send().await.map_err(|e| e.to_string())?;
        let http_code = resp.status().as_u16();
        let ssa_code = resp
            .headers()
            .get("ssa-code")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let text = resp.text().await.map_err(|e| e.to_string())?;
        let parsed: Value = serde_json::from_str(&text)
            .map_err(|_| "酷狗服务器返回了非 JSON 响应".to_string())?;

        let ssa_from_body = parsed
            .get("ssaCode")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(ApiResponse {
            body: parsed,
            ssa_code: ssa_code.or(ssa_from_body),
            http_code,
        })
    }

    pub async fn request_raw_bytes(
        &self,
        path: &str,
        device: &DeviceIdentity,
        session: Option<&KugouSession>,
        query: HashMap<String, String>,
        raw_body: Option<String>,
        host: &str,
        method: &str,
        headers: HashMap<String, String>,
        sign: bool,
        include_default_params: bool,
    ) -> Result<Vec<u8>, String> {
        let candidates = Self::build_candidate_endpoints(host, &headers);
        let mut last_err = String::from("网络连接失败");

        for (idx, (candidate_host, candidate_headers)) in candidates.iter().enumerate() {
            let now_sec = Utc::now().timestamp().to_string();
            let mut params = BTreeMap::new();
            if include_default_params {
                params.insert("dfid".to_string(), device.dfid.clone());
                params.insert("mid".to_string(), device.mid.clone());
                params.insert("uuid".to_string(), "-".to_string());
                params.insert("appid".to_string(), APP_ID.to_string());
                params.insert("clientver".to_string(), CLIENT_VERSION.to_string());
                params.insert("clienttime".to_string(), now_sec.clone());
                if let Some(s) = session {
                    if !s.token.is_empty() {
                        params.insert("token".to_string(), s.token.clone());
                    }
                    if !s.user_id.is_empty() && s.user_id != "0" {
                        params.insert("userid".to_string(), s.user_id.clone());
                    }
                }
            }
            for (k, v) in &query {
                params.insert(k.clone(), v.clone());
            }
            let payload = raw_body.clone().unwrap_or_default();
            if sign {
                let sig = signer::signature(&params, &payload, false);
                params.insert("signature".to_string(), sig);
            }

            let full_url = format!(
                "{}/{}",
                candidate_host.trim_end_matches('/'),
                path.trim_start_matches('/')
            );
            let mut req_builder = match method.to_uppercase().as_str() {
                "POST" => self.client.post(&full_url),
                _ => self.client.get(&full_url),
            };
            let query_vec: Vec<(&String, &String)> = params.iter().collect();
            req_builder = req_builder
                .query(&query_vec)
                .header(
                    "User-Agent",
                    "Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi",
                )
                .header("kg-rc", "1")
                .header("kg-thash", "5d816a0")
                .header("kg-rec", "1")
                .header("kg-rf", "B9EDA08A64250DEFFBCADDEE00F8F25F");

            for (k, v) in candidate_headers {
                req_builder = req_builder.header(k, v);
            }
            if method.eq_ignore_ascii_case("POST") {
                req_builder = req_builder.body(payload);
            }

            match req_builder.send().await {
                Ok(resp) => match resp.bytes().await {
                    Ok(bytes) => return Ok(bytes.to_vec()),
                    Err(e) => last_err = e.to_string(),
                },
                Err(e) => last_err = e.to_string(),
            }
            if idx + 1 < candidates.len() {
                tokio::time::sleep(Duration::from_millis(120 * (idx as u64 + 1))).await;
            }
        }
        Err(last_err)
    }
}
