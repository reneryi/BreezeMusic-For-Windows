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
static LRC_MULTI_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]").unwrap());

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

/// 解密 Base64 编码的 KRC1 歌词字节流（4 字节魔数 "krc1" + 16 字节循环 XOR + Zlib 解压 + 2MB 防炸弹保护）
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

    let decoder = ZlibDecoder::new(&decrypted[..]);
    let mut limited = decoder.take(2 * 1024 * 1024);
    let mut output = String::new();
    if limited.read_to_string(&mut output).is_ok() {
        output
    } else {
        String::new()
    }
}

fn contains_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

fn contains_kana_or_hangul(s: &str) -> bool {
    s.chars().any(|c| {
        ('\u{3040}'..='\u{30FF}').contains(&c)
            || ('\u{AC00}'..='\u{D7AF}').contains(&c)
            || ('\u{1100}'..='\u{11FF}').contains(&c)
    })
}

pub fn is_predominantly_chinese(lines: &[LyricLine]) -> bool {
    if lines.is_empty() {
        return true;
    }
    let mut cjk_lines = 0usize;
    let mut foreign_lines = 0usize;
    for l in lines {
        if contains_kana_or_hangul(&l.text) {
            foreign_lines += 1;
        } else if contains_cjk(&l.text) {
            cjk_lines += 1;
        }
    }
    if foreign_lines > lines.len() / 5 {
        return false;
    }
    cjk_lines * 10 >= lines.len() * 6
}

/// 解析 KRC 内嵌的 [language:base64] 双语翻译与音译注音层 (对齐 KrcDecoder.kt 启发式语种识别与双轨对齐)
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
        let mut multi_token_rows = 0usize;
        let mut non_empty_rows = 0usize;
        let mut chinese_rows = 0usize;

        for row in lyric_content {
            if let Some(arr) = row.as_array() {
                let tokens: Vec<String> = arr
                    .iter()
                    .filter_map(|t| t.as_str())
                    .map(|s| unescape_entities(s.trim()))
                    .filter(|s| !s.is_empty() && s != "//" && s != "/")
                    .collect();
                if tokens.len() > 1 {
                    multi_token_rows += 1;
                }
                let joined = tokens.join(" ");
                if !joined.is_empty() {
                    non_empty_rows += 1;
                    if contains_cjk(&joined) && !contains_kana_or_hangul(&joined) {
                        chinese_rows += 1;
                    }
                }
                parsed_lines.push(joined);
            } else if let Some(s) = row.as_str() {
                let clean = unescape_entities(s.trim());
                let final_s = if clean == "//" || clean == "/" {
                    String::new()
                } else {
                    clean
                };
                if !final_s.is_empty() {
                    non_empty_rows += 1;
                    if contains_cjk(&final_s) && !contains_kana_or_hangul(&final_s) {
                        chinese_rows += 1;
                    }
                }
                parsed_lines.push(final_s);
            } else {
                parsed_lines.push(String::new());
            }
        }

        if non_empty_rows == 0 {
            continue;
        }

        // 对齐 Android KrcDecoder.classifyLanguageLayer 启发式分类
        let is_likely_phonetic =
            multi_token_rows * 2 > non_empty_rows && chinese_rows * 3 < non_empty_rows;
        let is_likely_translation =
            chinese_rows * 10 >= non_empty_rows * 4 && multi_token_rows * 2 <= non_empty_rows;

        if type_val == 0 || (is_likely_phonetic && type_val != 1) {
            if phonetics.is_empty() {
                phonetics = parsed_lines;
            } else if translations.is_empty() && is_likely_translation {
                translations = parsed_lines;
            }
        } else if type_val == 1 || is_likely_translation {
            if translations.is_empty() {
                translations = parsed_lines;
            } else if phonetics.is_empty() && is_likely_phonetic {
                phonetics = parsed_lines;
            }
        } else if translations.is_empty() {
            translations = parsed_lines;
        } else if phonetics.is_empty() {
            phonetics = parsed_lines;
        }
    }

    (translations, phonetics)
}

pub fn parse_krc_text(text: &str) -> LyricData {
    if text.trim().is_empty() {
        return LyricData::default();
    }
    let (translations, phonetics) = extract_krc_language_layers(text);

    // 统计总时间行与非空时间行，实现双模式对齐
    let mut total_time_lines = 0usize;
    let mut non_blank_time_lines = 0usize;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        if METADATA_PREFIXES.iter().any(|p| lower.starts_with(p)) {
            continue;
        }
        if let Some(caps) = KRC_LINE_RE.captures(line) {
            total_time_lines += 1;
            let rest = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let mut has_text = false;
            for w_cap in KRC_WORD_RE.captures_iter(rest) {
                let w_text = w_cap.get(3).map(|m| m.as_str().trim()).unwrap_or("");
                if !w_text.is_empty() {
                    has_text = true;
                    break;
                }
            }
            if !has_text && !rest.trim().is_empty() {
                has_text = true;
            }
            if has_text {
                non_blank_time_lines += 1;
            }
        }
    }

    let trans_use_total_idx =
        translations.len() == total_time_lines || translations.len() != non_blank_time_lines;
    let phon_use_total_idx =
        phonetics.len() == total_time_lines || phonetics.len() != non_blank_time_lines;

    let mut lines = Vec::new();
    let mut time_line_idx = 0usize;
    let mut non_blank_idx = 0usize;

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
        let cur_total_idx = time_line_idx;
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
            let cur_nb_idx = non_blank_idx;
            non_blank_idx += 1;

            let t_idx = if trans_use_total_idx {
                cur_total_idx
            } else {
                cur_nb_idx
            };
            let p_idx = if phon_use_total_idx {
                cur_total_idx
            } else {
                cur_nb_idx
            };

            let translation = translations
                .get(t_idx)
                .cloned()
                .filter(|s| !s.trim().is_empty());
            let phonetic = phonetics
                .get(p_idx)
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

/// 解析标准/双语 LRC 文本（支持同一行多个时间标签 `[00:15.00][01:30.00]`、行内 `//` 双语分割及同时间戳双行自动合并）
pub fn parse_lrc_text(text: &str) -> LyricData {
    if text.trim().is_empty() {
        return LyricData::default();
    }
    let mut raw_entries: Vec<(i64, usize, String, Option<String>, Option<String>)> = Vec::new();
    let mut order_counter = 0usize;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        if METADATA_PREFIXES.iter().any(|p| lower.starts_with(p)) {
            continue;
        }

        let mut timestamps = Vec::new();
        let mut last_match_end = 0usize;
        for caps in LRC_MULTI_TAG_RE.captures_iter(line) {
            if let Some(m) = caps.get(0) {
                last_match_end = m.end();
            }
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
            timestamps.push(min * 60_000 + sec * 1000 + ms);
        }

        if timestamps.is_empty() {
            continue;
        }

        let content = unescape_entities(line[last_match_end..].trim());
        if content.is_empty() || content == "//" {
            continue;
        }

        let (main_text, inline_trans, inline_phon) = if content.contains(" // ") {
            let parts: Vec<&str> = content.split(" // ").map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                (
                    parts[0].to_string(),
                    Some(parts[1].to_string()).filter(|s| !s.is_empty()),
                    Some(parts[2].to_string()).filter(|s| !s.is_empty()),
                )
            } else if parts.len() == 2 {
                (
                    parts[0].to_string(),
                    Some(parts[1].to_string()).filter(|s| !s.is_empty()),
                    None,
                )
            } else {
                (content, None, None)
            }
        } else {
            (content, None, None)
        };

        for ts in timestamps {
            order_counter += 1;
            raw_entries.push((
                ts,
                order_counter,
                main_text.clone(),
                inline_trans.clone(),
                inline_phon.clone(),
            ));
        }
    }

    raw_entries.sort_by_key(|e| (e.0, e.1));

    let mut lines: Vec<LyricLine> = Vec::new();
    for (ts, _, text_part, tr_part, ph_part) in raw_entries {
        if let Some(last) = lines.last_mut() {
            if (last.start_time_ms - ts).abs() <= 100 && last.text != text_part {
                if last.translation.is_none() {
                    last.translation = Some(text_part);
                    continue;
                } else if last.phonetic.is_none() {
                    last.phonetic = Some(text_part);
                    continue;
                }
            }
        }
        lines.push(LyricLine {
            start_time_ms: ts,
            duration_ms: 4000,
            text: text_part,
            words: Vec::new(),
            translation: tr_part,
            phonetic: ph_part,
        });
    }

    for i in 0..lines.len().saturating_sub(1) {
        let next_start = lines[i + 1].start_time_ms;
        let dur = (next_start - lines[i].start_time_ms).max(500);
        lines[i].duration_ms = dur;
    }

    let has_translation = lines.iter().any(|l| l.translation.is_some());
    let has_phonetic = lines.iter().any(|l| l.phonetic.is_some());

    LyricData {
        lines,
        raw_text: text.to_string(),
        is_krc: false,
        has_translation,
        has_phonetic,
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
