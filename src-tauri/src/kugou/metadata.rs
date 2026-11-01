use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub struct AudioTagMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub publish_date: String,
    pub song_hash: String,
    pub quality_label: String,
    pub cover_bytes: Option<Vec<u8>>,
    pub lrc_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioContainerFormat {
    Flac,
    Mp3,
    Ogg,
    Other,
}

pub fn detect_format(path: &Path, fallback_ext: &str) -> AudioContainerFormat {
    if let Ok(bytes) = fs::read(path) {
        if bytes.len() >= 4 {
            if &bytes[0..4] == b"fLaC" {
                return AudioContainerFormat::Flac;
            }
            if &bytes[0..4] == b"OggS" {
                return AudioContainerFormat::Ogg;
            }
            if &bytes[0..3] == b"ID3" {
                return AudioContainerFormat::Mp3;
            }
            if bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
                return AudioContainerFormat::Mp3;
            }
        }
    }
    let ext = if !fallback_ext.trim().is_empty() {
        fallback_ext.trim().trim_start_matches('.').to_lowercase()
    } else {
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase()
    };
    match ext.as_str() {
        "flac" => AudioContainerFormat::Flac,
        "mp3" => AudioContainerFormat::Mp3,
        "ogg" => AudioContainerFormat::Ogg,
        _ => AudioContainerFormat::Other,
    }
}

pub fn detect_image_mime_type(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 4 && bytes[0] == 0x89 && bytes[1] == 0x50 && bytes[2] == 0x4E && bytes[3] == 0x47 {
        "image/png"
    } else {
        "image/jpeg"
    }
}

pub fn embed_metadata(path: &Path, meta: &AudioTagMetadata, fallback_ext: &str) -> bool {
    if !path.exists() {
        return false;
    }
    match detect_format(path, fallback_ext) {
        AudioContainerFormat::Flac => embed_flac_metadata(path, meta).unwrap_or(false),
        AudioContainerFormat::Mp3 => embed_mp3_id3v23_metadata(path, meta).unwrap_or(false),
        AudioContainerFormat::Ogg => true,
        AudioContainerFormat::Other => false,
    }
}

struct FlacBlock {
    block_type: u8,
    data: Vec<u8>,
}

fn embed_flac_metadata(path: &Path, meta: &AudioTagMetadata) -> Result<bool, String> {
    let raw_bytes = fs::read(path).map_err(|e| e.to_string())?;
    if raw_bytes.len() < 8 || &raw_bytes[0..4] != b"fLaC" {
        return Ok(false);
    }

    let mut kept_blocks: Vec<FlacBlock> = Vec::new();
    let mut existing_comments: BTreeMap<String, String> = BTreeMap::new();
    let mut offset = 4usize;
    let mut is_last_block = false;

    while !is_last_block && offset + 4 <= raw_bytes.len() {
        let header_byte = raw_bytes[offset];
        is_last_block = (header_byte & 0x80) != 0;
        let block_type = header_byte & 0x7F;
        let length = ((raw_bytes[offset + 1] as usize) << 16)
            | ((raw_bytes[offset + 2] as usize) << 8)
            | (raw_bytes[offset + 3] as usize);
        offset += 4;
        if offset + length > raw_bytes.len() {
            return Ok(false);
        }
        let block_data = raw_bytes[offset..offset + length].to_vec();
        offset += length;

        match block_type {
            4 => {
                for (k, v) in parse_vorbis_comment_block(&block_data) {
                    existing_comments.insert(k, v);
                }
            }
            6 => {
                if meta.cover_bytes.as_ref().map(|b| b.is_empty()).unwrap_or(true) {
                    kept_blocks.push(FlacBlock {
                        block_type,
                        data: block_data,
                    });
                }
            }
            1 => {
                // Skip old padding block
            }
            _ => {
                kept_blocks.push(FlacBlock {
                    block_type,
                    data: block_data,
                });
            }
        }
    }

    let audio_frames_start = offset;
    if audio_frames_start > raw_bytes.len() {
        return Ok(false);
    }

    if !meta.title.trim().is_empty() {
        existing_comments.insert("TITLE".to_string(), meta.title.trim().to_string());
    }
    if !meta.artist.trim().is_empty() {
        existing_comments.insert("ARTIST".to_string(), meta.artist.trim().to_string());
        existing_comments.insert("ALBUMARTIST".to_string(), meta.artist.trim().to_string());
    }
    if !meta.album.trim().is_empty() {
        existing_comments.insert("ALBUM".to_string(), meta.album.trim().to_string());
    }
    if !meta.publish_date.trim().is_empty() {
        let dt = meta.publish_date.trim();
        existing_comments.insert("DATE".to_string(), dt.to_string());
        existing_comments.insert("YEAR".to_string(), dt.chars().take(4).collect());
    }

    let mut comment_parts = Vec::new();
    if !meta.quality_label.trim().is_empty() {
        comment_parts.push(meta.quality_label.trim().to_string());
    }
    if !meta.song_hash.trim().is_empty() {
        comment_parts.push(format!("Hash:{}", meta.song_hash.trim().to_uppercase()));
    }
    if !comment_parts.is_empty() {
        existing_comments.insert("COMMENT".to_string(), comment_parts.join(" | "));
    }
    if !meta.lrc_text.trim().is_empty() {
        existing_comments.insert("LYRICS".to_string(), meta.lrc_text.clone());
        existing_comments.insert("UNSYNCEDLYRICS".to_string(), meta.lrc_text.clone());
    }

    let vorbis_data = build_vorbis_comment_block("BreezeMusic Kugou Concept", &existing_comments);
    kept_blocks.push(FlacBlock {
        block_type: 4,
        data: vorbis_data,
    });

    if let Some(ref cover) = meta.cover_bytes {
        if !cover.is_empty() {
            let mime = detect_image_mime_type(cover);
            let pic_data = build_flac_picture_block(cover, mime, "Cover");
            if pic_data.len() <= 0xFFFFFF {
                kept_blocks.push(FlacBlock {
                    block_type: 6,
                    data: pic_data,
                });
            }
        }
    }

    let mut out = Vec::with_capacity(raw_bytes.len() + 65536);
    out.extend_from_slice(b"fLaC");
    let last_idx = kept_blocks.len().saturating_sub(1);
    for (i, blk) in kept_blocks.iter().enumerate() {
        let is_last = i == last_idx;
        let first_byte = (if is_last { 0x80 } else { 0x00 }) | (blk.block_type & 0x7F);
        let len = blk.data.len();
        out.push(first_byte);
        out.push(((len >> 16) & 0xFF) as u8);
        out.push(((len >> 8) & 0xFF) as u8);
        out.push((len & 0xFF) as u8);
        out.extend_from_slice(&blk.data);
    }
    out.extend_from_slice(&raw_bytes[audio_frames_start..]);
    fs::write(path, out).map_err(|e| e.to_string())?;
    Ok(true)
}

fn parse_vorbis_comment_block(data: &[u8]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if data.len() < 8 {
        return map;
    }
    let vendor_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut pos = 4 + vendor_len;
    if pos + 4 > data.len() {
        return map;
    }
    let count = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
    pos += 4;
    for _ in 0..count.min(2000) {
        if pos + 4 > data.len() {
            break;
        }
        let item_len = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;
        if pos + item_len > data.len() {
            break;
        }
        if let Ok(s) = std::str::from_utf8(&data[pos..pos + item_len]) {
            if let Some(eq_idx) = s.find('=') {
                if eq_idx > 0 {
                    map.insert(s[..eq_idx].to_uppercase(), s[eq_idx + 1..].to_string());
                }
            }
        }
        pos += item_len;
    }
    map
}

fn build_vorbis_comment_block(vendor: &str, comments: &BTreeMap<String, String>) -> Vec<u8> {
    let mut out = Vec::new();
    let vendor_bytes = vendor.as_bytes();
    out.extend_from_slice(&(vendor_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(vendor_bytes);

    let entries: Vec<Vec<u8>> = comments
        .iter()
        .filter(|(k, v)| !k.trim().is_empty() && !v.trim().is_empty())
        .map(|(k, v)| format!("{}={}", k.to_uppercase(), v).into_bytes())
        .collect();

    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        out.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        out.extend_from_slice(&entry);
    }
    out
}

fn build_flac_picture_block(image_bytes: &[u8], mime_type: &str, description: &str) -> Vec<u8> {
    let mime_bytes = mime_type.as_bytes();
    let desc_bytes = description.as_bytes();
    let mut out = Vec::with_capacity(32 + mime_bytes.len() + desc_bytes.len() + image_bytes.len());
    out.extend_from_slice(&3u32.to_be_bytes()); // Picture type 3: Front Cover
    out.extend_from_slice(&(mime_bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(mime_bytes);
    out.extend_from_slice(&(desc_bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(desc_bytes);
    out.extend_from_slice(&0u32.to_be_bytes()); // width
    out.extend_from_slice(&0u32.to_be_bytes()); // height
    out.extend_from_slice(&24u32.to_be_bytes()); // color depth
    out.extend_from_slice(&0u32.to_be_bytes()); // indexed colors
    out.extend_from_slice(&(image_bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(image_bytes);
    out
}

fn embed_mp3_id3v23_metadata(path: &Path, meta: &AudioTagMetadata) -> Result<bool, String> {
    let raw_bytes = fs::read(path).map_err(|e| e.to_string())?;
    if raw_bytes.is_empty() {
        return Ok(false);
    }

    let mut audio_start_offset = 0usize;
    if raw_bytes.len() >= 10 && &raw_bytes[0..3] == b"ID3" {
        let flags = raw_bytes[5];
        let synchsafe_size = (((raw_bytes[6] & 0x7F) as usize) << 21)
            | (((raw_bytes[7] & 0x7F) as usize) << 14)
            | (((raw_bytes[8] & 0x7F) as usize) << 7)
            | ((raw_bytes[9] & 0x7F) as usize);
        let footer_size = if (flags & 0x10) != 0 { 10 } else { 0 };
        let total_old_id3 = 10 + synchsafe_size + footer_size;
        if (10..=raw_bytes.len()).contains(&total_old_id3) {
            audio_start_offset = total_old_id3;
        }
    }

    let mut frames_out: Vec<u8> = Vec::new();
    let mut write_frame = |frame_id: &[u8; 4], payload: &[u8]| {
        if payload.is_empty() {
            return;
        }
        frames_out.extend_from_slice(frame_id);
        frames_out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frames_out.extend_from_slice(&[0u8, 0u8]);
        frames_out.extend_from_slice(payload);
    };

    let encode_utf16le_with_bom = |text: &str| -> Vec<u8> {
        let mut v = vec![0xFF, 0xFE];
        for code_unit in text.encode_utf16() {
            v.extend_from_slice(&code_unit.to_le_bytes());
        }
        v
    };

    let build_text_payload = |text: &str| -> Vec<u8> {
        let mut v = vec![1u8]; // UTF-16 with BOM
        v.extend_from_slice(&encode_utf16le_with_bom(text));
        v
    };

    if !meta.title.trim().is_empty() {
        write_frame(b"TIT2", &build_text_payload(meta.title.trim()));
    }
    if !meta.artist.trim().is_empty() {
        write_frame(b"TPE1", &build_text_payload(meta.artist.trim()));
        write_frame(b"TPE2", &build_text_payload(meta.artist.trim()));
    }
    if !meta.album.trim().is_empty() {
        write_frame(b"TALB", &build_text_payload(meta.album.trim()));
    }
    if !meta.publish_date.trim().is_empty() {
        let year: String = meta.publish_date.trim().chars().take(4).collect();
        if year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()) {
            write_frame(b"TYER", &build_text_payload(&year));
        }
        write_frame(b"TDRC", &build_text_payload(meta.publish_date.trim()));
    }

    let mut comment_parts = Vec::new();
    if !meta.quality_label.trim().is_empty() {
        comment_parts.push(meta.quality_label.trim().to_string());
    }
    if !meta.song_hash.trim().is_empty() {
        comment_parts.push(format!("Hash:{}", meta.song_hash.trim().to_uppercase()));
    }
    if !comment_parts.is_empty() {
        let comment_text = comment_parts.join(" | ");
        let mut comm_payload = vec![1u8]; // UTF-16 with BOM
        comm_payload.extend_from_slice(b"eng");
        comm_payload.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x00]); // Empty UTF-16 desc
        comm_payload.extend_from_slice(&encode_utf16le_with_bom(&comment_text));
        write_frame(b"COMM", &comm_payload);
    }

    if !meta.lrc_text.trim().is_empty() {
        let mut uslt_payload = vec![1u8];
        uslt_payload.extend_from_slice(b"chi");
        uslt_payload.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x00]);
        uslt_payload.extend_from_slice(&encode_utf16le_with_bom(&meta.lrc_text));
        write_frame(b"USLT", &uslt_payload);
    }

    if let Some(ref cover) = meta.cover_bytes {
        if !cover.is_empty() {
            let mime = detect_image_mime_type(cover);
            let mut apic_payload = vec![0u8]; // ISO-8859-1
            apic_payload.extend_from_slice(mime.as_bytes());
            apic_payload.push(0);
            apic_payload.push(3); // Front Cover
            apic_payload.extend_from_slice(b"Cover");
            apic_payload.push(0);
            apic_payload.extend_from_slice(cover);
            write_frame(b"APIC", &apic_payload);
        }
    }

    let tag_size = frames_out.len();
    let mut out = Vec::with_capacity(10 + tag_size + (raw_bytes.len() - audio_start_offset));
    out.extend_from_slice(b"ID3");
    out.push(3); // v2.3
    out.push(0);
    out.push(0);
    out.push(((tag_size >> 21) & 0x7F) as u8);
    out.push(((tag_size >> 14) & 0x7F) as u8);
    out.push(((tag_size >> 7) & 0x7F) as u8);
    out.push((tag_size & 0x7F) as u8);
    out.extend_from_slice(&frames_out);
    out.extend_from_slice(&raw_bytes[audio_start_offset..]);
    fs::write(path, out).map_err(|e| e.to_string())?;
    Ok(true)
}
