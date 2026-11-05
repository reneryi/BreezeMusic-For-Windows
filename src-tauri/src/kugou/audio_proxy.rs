use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const KUGOU_UA: &str = "Android15-1070-11440-46-0-DiscoveryDRADProtocol-wifi";
const KUGOU_REFERER: &str = "https://www.kugou.com/";

#[derive(Debug, Clone)]
pub struct StreamEntry {
    pub cache_key: String,
    pub primary_url: String,
    pub backup_urls: Vec<String>,
    pub ext_name: String,
    pub expected_size: i64,
}

#[derive(Clone)]
pub struct AudioStreamProxy {
    port: Arc<Mutex<u16>>,
    entries: Arc<Mutex<HashMap<String, StreamEntry>>>,
    client: Client,
    cache_dir: PathBuf,
}

impl AudioStreamProxy {
    pub fn new(cache_dir: PathBuf) -> Self {
        let streams_dir = cache_dir.join("audio_streams");
        let _ = std::fs::create_dir_all(&streams_dir);

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(20))
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_nodelay(true)
            .build()
            .unwrap_or_else(|_| Client::new());

        // 使用 std::net::TcpListener 同步绑定端口，确保 new() 返回时端口立即可用
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").ok();
        let bound_port = std_listener
            .as_ref()
            .and_then(|l| l.local_addr().ok())
            .map(|a| a.port())
            .unwrap_or(0);

        let port_arc = Arc::new(Mutex::new(bound_port));
        let entries_arc: Arc<Mutex<HashMap<String, StreamEntry>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let proxy = Self {
            port: port_arc,
            entries: entries_arc,
            client,
            cache_dir: streams_dir,
        };

        if let Some(std_l) = std_listener {
            let _ = std_l.set_nonblocking(true);
            let proxy_clone = proxy.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(listener) = TcpListener::from_std(std_l) {
                    loop {
                        if let Ok((socket, _)) = listener.accept().await {
                            let p = proxy_clone.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = p.handle_connection(socket).await;
                            });
                        }
                    }
                }
            });
        }

        proxy
    }

    pub fn register_stream(
        &self,
        cache_key: &str,
        primary_url: &str,
        backup_urls: Vec<String>,
        ext_name: &str,
        expected_size: i64,
    ) -> String {
        let port = self.port.lock().map(|g| *g).unwrap_or(0);
        if port == 0 || primary_url.trim().is_empty() {
            return String::new();
        }
        let safe_key: String = cache_key
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let clean_ext = if ext_name.trim().is_empty() {
            "mp3".to_string()
        } else {
            ext_name.trim().trim_start_matches('.').to_lowercase()
        };
        let entry = StreamEntry {
            cache_key: safe_key.clone(),
            primary_url: primary_url.trim().to_string(),
            backup_urls,
            ext_name: clean_ext.clone(),
            expected_size,
        };
        if let Ok(mut guard) = self.entries.lock() {
            guard.insert(safe_key.clone(), entry);
        }
        format!("http://127.0.0.1:{}/stream/{}.{}", port, safe_key, clean_ext)
    }

    fn content_type_for_ext(ext: &str) -> &'static str {
        match ext.to_lowercase().as_str() {
            "flac" => "audio/flac",
            "ogg" | "oga" => "audio/ogg",
            "m4a" | "aac" | "mp4" => "audio/mp4",
            "wav" => "audio/wav",
            _ => "audio/mpeg",
        }
    }

    fn parse_range_header(header_block: &str) -> Option<(u64, Option<u64>)> {
        for line in header_block.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("range:") {
                let val = line[6..].trim();
                let val_lower = val.to_ascii_lowercase();
                if let Some(rest) = val_lower.strip_prefix("bytes=") {
                    let first_range = rest.split(',').next()?.trim();
                    let mut parts = first_range.splitn(2, '-');
                    let start_str = parts.next()?.trim();
                    let end_str = parts.next().map(|s| s.trim()).unwrap_or("");
                    if let Ok(start) = start_str.parse::<u64>() {
                        let end_opt = if end_str.is_empty() {
                            None
                        } else {
                            end_str.parse::<u64>().ok()
                        };
                        return Some((start, end_opt));
                    }
                }
            }
        }
        None
    }

    async fn handle_connection(&self, mut socket: TcpStream) -> Result<(), String> {
        let _ = socket.set_nodelay(true);
        let mut req_buf = vec![0u8; 8192];
        let n = socket
            .read(&mut req_buf)
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        let req_str = String::from_utf8_lossy(&req_buf[..n]);
        let first_line = req_str.lines().next().unwrap_or("");
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("GET").to_uppercase();
        let raw_path = parts.next().unwrap_or("/");

        if method == "OPTIONS" {
            let resp = "HTTP/1.1 204 No Content\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
                Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
                Access-Control-Expose-Headers: Content-Length, Content-Range, Accept-Ranges\r\n\
                Connection: close\r\n\r\n";
            let _ = socket.write_all(resp.as_bytes()).await;
            return Ok(());
        }

        let path_no_query = raw_path.split('?').next().unwrap_or("/");
        let token_with_ext = path_no_query.strip_prefix("/stream/").unwrap_or("");
        let cache_key = token_with_ext
            .rsplit_once('.')
            .map(|(k, _)| k)
            .unwrap_or(token_with_ext);

        let entry_opt = self
            .entries
            .lock()
            .ok()
            .and_then(|g| g.get(cache_key).cloned());
        let Some(entry) = entry_opt else {
            let not_found = "HTTP/1.1 404 Not Found\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = socket.write_all(not_found.as_bytes()).await;
            return Ok(());
        };

        let range_req = Self::parse_range_header(&req_str);
        let cache_file = self
            .cache_dir
            .join(format!("{}.{}", entry.cache_key, entry.ext_name));

        // 1. 优先命中已完整缓存的本地磁盘音频文件（零网络抖动，纳秒级 Range 拖动）
        if let Ok(meta) = std::fs::metadata(&cache_file) {
            let file_len = meta.len();
            let min_valid_len = if entry.expected_size > 65_536 {
                (entry.expected_size as u64).saturating_sub(32_768)
            } else {
                32_768
            };
            if file_len >= min_valid_len {
                return self
                    .serve_from_cached_file(&mut socket, &cache_file, file_len, &entry.ext_name, range_req, &method)
                    .await;
            }
        }

        // 2. 在线流式代理 + 断点自动重连 + 多 CDN 备线无缝切换 + 本地边听边存
        self.serve_from_upstream(&mut socket, &entry, &cache_file, range_req, &method)
            .await
    }

    async fn serve_from_cached_file(
        &self,
        socket: &mut TcpStream,
        cache_file: &PathBuf,
        total_len: u64,
        ext_name: &str,
        range_req: Option<(u64, Option<u64>)>,
        method: &str,
    ) -> Result<(), String> {
        let mime = Self::content_type_for_ext(ext_name);
        let (start, end, is_partial) = match range_req {
            Some((s, e_opt)) => {
                if s >= total_len {
                    let hdr = format!(
                        "HTTP/1.1 416 Range Not Satisfiable\r\n\
                        Access-Control-Allow-Origin: *\r\n\
                        Content-Range: bytes */{}\r\n\
                        Connection: close\r\n\r\n",
                        total_len
                    );
                    let _ = socket.write_all(hdr.as_bytes()).await;
                    return Ok(());
                }
                let e = e_opt.unwrap_or(total_len - 1).min(total_len - 1);
                (s, e, true)
            }
            None => (0, total_len.saturating_sub(1), false),
        };

        let send_len = end.saturating_sub(start) + 1;
        let status_line = if is_partial {
            "HTTP/1.1 206 Partial Content"
        } else {
            "HTTP/1.1 200 OK"
        };
        let range_header = if is_partial {
            format!("Content-Range: bytes {}-{}/{}\r\n", start, end, total_len)
        } else {
            String::new()
        };

        let headers = format!(
            "{}\r\n\
            Content-Type: {}\r\n\
            Content-Length: {}\r\n\
            Accept-Ranges: bytes\r\n\
            {}\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Expose-Headers: Content-Length, Content-Range, Accept-Ranges\r\n\
            Cache-Control: no-cache\r\n\
            Connection: close\r\n\r\n",
            status_line, mime, send_len, range_header
        );
        socket
            .write_all(headers.as_bytes())
            .await
            .map_err(|e| e.to_string())?;

        if method == "HEAD" {
            return Ok(());
        }

        let mut file = tokio::fs::File::open(cache_file)
            .await
            .map_err(|e| e.to_string())?;
        if start > 0 {
            file.seek(std::io::SeekFrom::Start(start))
                .await
                .map_err(|e| e.to_string())?;
        }

        let mut remaining = send_len;
        let mut buf = vec![0u8; 65_536];
        while remaining > 0 {
            let to_read = (remaining as usize).min(buf.len());
            let n = file
                .read(&mut buf[..to_read])
                .await
                .map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            if socket.write_all(&buf[..n]).await.is_err() {
                break;
            }
            remaining = remaining.saturating_sub(n as u64);
        }
        let _ = socket.flush().await;
        Ok(())
    }

    fn parse_upstream_total_size(resp: &reqwest::Response, start_offset: u64) -> Option<u64> {
        if let Some(cr) = resp
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
        {
            if let Some((_, total_str)) = cr.rsplit_once('/') {
                if let Ok(total) = total_str.trim().parse::<u64>() {
                    if total > 0 {
                        return Some(total);
                    }
                }
            }
        }
        if let Some(cl) = resp.content_length() {
            if cl > 0 {
                return Some(start_offset + cl);
            }
        }
        None
    }

    async fn open_upstream_at_offset(
        &self,
        urls: &[String],
        start_offset: u64,
        end_offset: Option<u64>,
        prefer_url_idx: usize,
    ) -> Option<(reqwest::Response, usize)> {
        if urls.is_empty() {
            return None;
        }
        let len = urls.len();
        for step in 0..len {
            let idx = (prefer_url_idx + step) % len;
            let url = &urls[idx];
            let mut req = self
                .client
                .get(url)
                .header(reqwest::header::USER_AGENT, KUGOU_UA)
                .header(reqwest::header::REFERER, KUGOU_REFERER)
                .header(reqwest::header::ACCEPT, "*/*")
                .header(reqwest::header::CONNECTION, "keep-alive");

            let range_val = match end_offset {
                Some(end) => format!("bytes={}-{}", start_offset, end),
                None => format!("bytes={}-", start_offset),
            };
            req = req.header(reqwest::header::RANGE, range_val);

            if let Ok(resp) = req.send().await {
                let code = resp.status().as_u16();
                if code == 200 || code == 206 {
                    return Some((resp, idx));
                }
            }
        }
        None
    }

    async fn serve_from_upstream(
        &self,
        socket: &mut TcpStream,
        entry: &StreamEntry,
        final_cache_file: &PathBuf,
        range_req: Option<(u64, Option<u64>)>,
        method: &str,
    ) -> Result<(), String> {
        let mut all_urls = vec![entry.primary_url.clone()];
        for b in &entry.backup_urls {
            if !b.trim().is_empty() && !all_urls.contains(b) {
                all_urls.push(b.clone());
            }
        }

        let (req_start, req_end_opt) = range_req.unwrap_or((0, None));
        let Some((mut resp, mut active_idx)) = self
            .open_upstream_at_offset(&all_urls, req_start, req_end_opt, 0)
            .await
        else {
            let bad_gw = "HTTP/1.1 502 Bad Gateway\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = socket.write_all(bad_gw.as_bytes()).await;
            return Ok(());
        };

        let upstream_status = resp.status().as_u16();
        let total_size = Self::parse_upstream_total_size(&resp, req_start)
            .or_else(|| {
                if entry.expected_size > 0 {
                    Some(entry.expected_size as u64)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let is_partial = range_req.is_some() || upstream_status == 206;
        let effective_end = match req_end_opt {
            Some(e) if total_size > 0 => e.min(total_size - 1),
            Some(e) => e,
            None if total_size > 0 => total_size - 1,
            None => 0,
        };
        let content_len = if total_size > 0 && effective_end >= req_start {
            Some(effective_end - req_start + 1)
        } else {
            resp.content_length()
        };

        let mime = Self::content_type_for_ext(&entry.ext_name);
        let status_line = if is_partial {
            "HTTP/1.1 206 Partial Content"
        } else {
            "HTTP/1.1 200 OK"
        };

        let mut resp_headers = format!(
            "{}\r\n\
            Content-Type: {}\r\n\
            Accept-Ranges: bytes\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Expose-Headers: Content-Length, Content-Range, Accept-Ranges\r\n\
            Cache-Control: no-cache\r\n\
            Connection: close\r\n",
            status_line, mime
        );

        if let Some(cl) = content_len {
            resp_headers.push_str(&format!("Content-Length: {}\r\n", cl));
        }
        if is_partial && total_size > 0 && effective_end >= req_start {
            resp_headers.push_str(&format!(
                "Content-Range: bytes {}-{}/{}\r\n",
                req_start, effective_end, total_size
            ));
        }
        resp_headers.push_str("\r\n");

        if socket.write_all(resp_headers.as_bytes()).await.is_err() {
            return Ok(());
        }
        if method == "HEAD" {
            return Ok(());
        }

        // 若从 0 字节开始拉取全曲，则同步写入临时文件并在完整落盘后原子重命名为正式缓存文件
        let can_cache_full = req_start == 0 && req_end_opt.is_none();
        let part_path = self.cache_dir.join(format!(
            "{}.{}.part.{}",
            entry.cache_key,
            entry.ext_name,
            uuid::Uuid::new_v4().simple()
        ));
        let mut part_file = if can_cache_full {
            tokio::fs::File::create(&part_path).await.ok()
        } else {
            None
        };

        let mut cursor = req_start;
        let target_end_exclusive = if total_size > 0 && effective_end >= req_start {
            Some(effective_end + 1)
        } else {
            content_len.map(|cl| req_start + cl)
        };

        let mut retries = 0u32;
        let max_retries = 5u32;
        let mut client_disconnected = false;

        loop {
            match resp.chunk().await {
                Ok(Some(chunk)) => {
                    if chunk.is_empty() {
                        continue;
                    }
                    retries = 0; // 收到有效数据即重置连续重试计数
                    if let Some(ref mut pf) = part_file {
                        let _ = pf.write_all(&chunk).await;
                    }
                    if socket.write_all(&chunk).await.is_err() {
                        client_disconnected = true;
                        break;
                    }
                    cursor += chunk.len() as u64;
                    if let Some(target_end) = target_end_exclusive {
                        if cursor >= target_end {
                            break;
                        }
                    }
                }
                Ok(None) => {
                    // 上游流 EOF：检查是否真的达到了预期结尾，还是被酷狗 CDN 中途静默掐断！
                    let truncated = match target_end_exclusive {
                        Some(target_end) => cursor < target_end,
                        None => false,
                    };
                    if !truncated {
                        break;
                    }
                    if retries >= max_retries {
                        break;
                    }
                    retries += 1;
                    let next_idx = if retries >= 2 {
                        (active_idx + 1) % all_urls.len()
                    } else {
                        active_idx
                    };
                    let retry_end = target_end_exclusive.map(|te| te.saturating_sub(1));
                    tokio::time::sleep(Duration::from_millis(180 * (retries as u64))).await;
                    if let Some((new_resp, new_idx)) = self
                        .open_upstream_at_offset(&all_urls, cursor, retry_end, next_idx)
                        .await
                    {
                        resp = new_resp;
                        active_idx = new_idx;
                        continue;
                    } else {
                        break;
                    }
                }
                Err(_) => {
                    // 上游网络超时或 RST 断流：从当前精确字节游标 cursor 处无缝重连
                    if retries >= max_retries {
                        break;
                    }
                    retries += 1;
                    let next_idx = (active_idx + 1) % all_urls.len();
                    let retry_end = target_end_exclusive.map(|te| te.saturating_sub(1));
                    tokio::time::sleep(Duration::from_millis(200 * (retries as u64))).await;
                    if let Some((new_resp, new_idx)) = self
                        .open_upstream_at_offset(&all_urls, cursor, retry_end, next_idx)
                        .await
                    {
                        resp = new_resp;
                        active_idx = new_idx;
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }

        let _ = socket.flush().await;

        if let Some(mut pf) = part_file {
            let _ = pf.flush().await;
            drop(pf);
            let completed = match target_end_exclusive {
                Some(target_end) => !client_disconnected && cursor >= target_end && cursor > 32_768,
                None => !client_disconnected && cursor > 128_000,
            };
            if completed {
                let _ = std::fs::rename(&part_path, final_cache_file);
            } else {
                let _ = std::fs::remove_file(&part_path);
            }
        }

        Ok(())
    }
}
