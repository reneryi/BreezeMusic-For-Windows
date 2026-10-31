use crate::kugou::models::{LyricData, LyricLine, LyricWord};
use base64::{engine::general_purpose::STANDARD as BASE64_STD, Engine as _};
use flate2::read::ZlibDecoder;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::io::Read;

/// KRC 16 字节循环异或密钥 (对齐 KrcDecoder.kt 与 MakcRe/KuGouMusicApi)
const KRC_XOR_KEY: [u8; 16] = [
    64, 71, 97, 119, 94, 50, 116, 71, 81, 54, 49, 45, 206, 210, 110, 105,
];

const METADATA_PREFIXES: [&str; 12] = [
    "[id:", "[ar:", "[ti:", "[by:", "[hash:", "[al:", "[sign:", "[qq:", "[total:", "[offset:",
    "[language:", "[kana:",
];

static KRC_LINE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\[(\d+),(\d+)\](.*)$").unwrap());
static KRC_WORD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<(-?\d+),(-?\d+)(?:,-?\d+)?>([^<]*)").unwrap());
static KRC_LANG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[language:([^\]]*)\]").unwrap());
static LRC_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\](.*)$").unwrap());

fn unescape_entities(raw: &str) -> String {
    if !raw.contains('&') {
        return raw.to_string();
    }
    raw.replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

/// 解密 Base64 编码的 KRC1 歌词字节流（4 字节魔数 "krc1" + 16 字节循环 XOR + Zlib 解压）
pub fn decode_krc_base64(b64_content: &str) -> String {
    let cleaned: String = b64_content.chars().filter(|c| !c.is_whitespace()).collect();
    let Ok(raw_bytes) = BASE64_STD.decode(&cleaned) else {
        return String::new();
    };
    if raw_bytes.len() <= 4 || &raw_bytes[0..4] != b"krc1" {
        return String::new();
    }

    let decrypted: Vec<u8> = raw_bytes[4..]
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ KRC_XOR_KEY[i % KRC_XOR_KEY.len()])
        .collect();

    let mut decoder = ZlibDecoder::new(&decrypted[..]);
    let mut output = String::new();
    if decoder.read_to_string(&mut output).is_ok() {
        output
    } else {
        String::new()
    }
}

/// 解析 KRC 内嵌的 [language:base64] 双语翻译与音译注音层
fn extract_krc_language_layers(text: &str) -> (Vec<String>, Vec<String>) {
    let mut translations = Vec::new();
    let mut phonetics = Vec::new();

    let Some(caps) = KRC_LANG_RE.captures(text) else {
        return (translations, phonetics);
    };
    let b64 = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
    if b64.is_empty() {
        return (translations, phonetics);
    }

    let pad_len = (4 - b64.len() % 4) % 4;
    let padded = format!("{}{}", b64, "=".repeat(pad_len));
    let Ok(decoded_bytes) = BASE64_STD.decode(padded) else {
        return (translations, phonetics);
    };
    let Ok(json_str) = String::from_utf8(decoded_bytes) else {
        return (translations, phonetics);
    };
    let Ok(root) = serde_json::from_str::<Value>(&json_str) else {
        return (translations, phonetics);
    };
    let Some(content_arr) = root.get("content").and_then(|v| v.as_array()) else {
        return (translations, phonetics);
    };

    for item in content_arr {
        let type_val = item.get("type").and_then(|v| v.as_i64()).unwrap_or(-1);
        let Some(lyric_content) = item.get("lyricContent").and_then(|v| v.as_array()) else {
            continue;
        };
        let mut parsed_lines = Vec::new();
        for row in lyric_content {
            if let Some(arr) = row.as_array() {
                let joined: Vec<String> = arr
                    .iter()
                    .filter_map(|t| t.as_str())
                    .map(|s| unescape_entities(s.trim()))
                    .filter(|s| !s.is_empty() && s != "//" && s != "/")
                    .collect();
                parsed_lines.push(joined.join(" "));
            } else if let Some(s) = row.as_str() {
                let clean = unescape_entities(s.trim());
                parsed_lines.push(if clean == "//" || clean == "/" {
                    String::new()
                } else {
                    clean
                });
            }
        }
        if type_val == 1 && translations.is_empty() {
            translations = parsed_lines;
        } else if type_val == 0 && phonetics.is_empty() {
            phonetics = parsed_lines;
        } else if translations.is_empty() {
            translations = parsed_lines;
        }
    }

    (translations, phonetics)
}

pub fn parse_krc_text(text: &str) -> LyricData {
    if text.trim().is_empty() {
        return LyricData::default();
    }
    let (translations, phonetics) = extract_krc_language_layers(text);
    let mut lines = Vec::new();
    let mut time_line_idx = 0usize;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        if METADATA_PREFIXES.iter().any(|p| lower.starts_with(p)) {
            continue;
        }
        let Some(caps) = KRC_LINE_RE.captures(line) else {
            continue;
        };
        let cur_idx = time_line_idx;
        time_line_idx += 1;

        let line_start = caps
            .get(1)
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        let line_duration = caps
            .get(2)
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        let rest = caps.get(3).map(|m| m.as_str()).unwrap_or("");

        let mut words = Vec::new();
        let mut full_text = String::new();
        for w_cap in KRC_WORD_RE.captures_iter(rest) {
            let offset = w_cap
                .get(1)
                .and_then(|m| m.as_str().parse::<i64>().ok())
                .unwrap_or(0);
            let duration = w_cap
                .get(2)
                .and_then(|m| m.as_str().parse::<i64>().ok())
                .unwrap_or(0);
            let w_text = unescape_entities(w_cap.get(3).map(|m| m.as_str()).unwrap_or(""));
            if !w_text.is_empty() {
                full_text.push_str(&w_text);
                words.push(LyricWord {
                    start_time_ms: line_start + offset,
                    duration_ms: duration,
                    text: w_text,
                });
            }
        }

        if full_text.is_empty() {
            full_text = unescape_entities(rest.trim());
        }
        if !full_text.is_empty() {
            let translation = translations
                .get(cur_idx)
                .cloned()
                .filter(|s| !s.trim().is_empty());
            let phonetic = phonetics
                .get(cur_idx)
                .cloned()
                .filter(|s| !s.trim().is_empty());
            lines.push(LyricLine {
                start_time_ms: line_start,
                duration_ms: line_duration,
                text: full_text,
                words,
                translation,
                phonetic,
            });
        }
    }

    lines.sort_by_key(|l| l.start_time_ms);
    let has_translation = lines.iter().any(|l| l.translation.is_some());
    let has_phonetic = lines.iter().any(|l| l.phonetic.is_some());

    LyricData {
        lines,
        raw_text: text.to_string(),
        is_krc: true,
        has_translation,
        has_phonetic,
    }
}

pub fn parse_lrc_text(text: &str) -> LyricData {
    if text.trim().is_empty() {
        return LyricData::default();
    }
    let mut lines: Vec<LyricLine> = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        if METADATA_PREFIXES.iter().any(|p| lower.starts_with(p)) {
            continue;
        }
        if let Some(caps) = LRC_TAG_RE.captures(line) {
            let min = caps
                .get(1)
                .and_then(|m| m.as_str().parse::<i64>().ok())
                .unwrap_or(0);
            let sec = caps
                .get(2)
                .and_then(|m| m.as_str().parse::<i64>().ok())
                .unwrap_or(0);
            let frac_str = caps.get(3).map(|m| m.as_str()).unwrap_or("0");
            let ms = match frac_str.len() {
                1 => frac_str.parse::<i64>().unwrap_or(0) * 100,
                2 => frac_str.parse::<i64>().unwrap_or(0) * 10,
                3 => frac_str.parse::<i64>().unwrap_or(0),
                _ => 0,
            };
            let content = unescape_entities(caps.get(4).map(|m| m.as_str().trim()).unwrap_or(""));
            if content.is_empty() || content == "//" {
                continue;
            }
            let start_ms = min * 60_000 + sec * 1000 + ms;
            lines.push(LyricLine {
                start_time_ms: start_ms,
                duration_ms: 4000,
                text: content,
                words: Vec::new(),
                translation: None,
                phonetic: None,
            });
        }
    }

    lines.sort_by_key(|l| l.start_time_ms);
    for i in 0..lines.len().saturating_sub(1) {
        let next_start = lines[i + 1].start_time_ms;
        let dur = (next_start - lines[i].start_time_ms).max(500);
        lines[i].duration_ms = dur;
    }

    LyricData {
        lines,
        raw_text: text.to_string(),
        is_krc: false,
        has_translation: false,
        has_phonetic: false,
    }
}

pub fn parse_lyric_auto(content: &str, is_base64_krc: bool) -> LyricData {
    if is_base64_krc {
        let decoded = decode_krc_base64(content);
        if !decoded.is_empty() {
            let krc = parse_krc_text(&decoded);
            if !krc.lines.is_empty() {
                return krc;
            }
        }
    }
    let cleaned: String = content.chars().filter(|c| !c.is_whitespace()).collect();
    if let Ok(bytes) = BASE64_STD.decode(&cleaned) {
        if let Ok(utf8) = String::from_utf8(bytes) {
            let lrc = parse_lrc_text(&utf8);
            if !lrc.lines.is_empty() {
                return lrc;
            }
        }
    }
    let krc = parse_krc_text(content);
    if !krc.lines.is_empty() {
        return krc;
    }
    parse_lrc_text(content)
}
