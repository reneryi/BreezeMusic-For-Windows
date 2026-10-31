use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AudioQuality {
    #[serde(rename = "128")]
    Standard,
    #[serde(rename = "320")]
    High,
    #[serde(rename = "flac")]
    Lossless,
    #[serde(rename = "high")]
    HiRes,
}

impl AudioQuality {
    pub fn api_value(&self) -> &'static str {
        match self {
            AudioQuality::Standard => "128",
            AudioQuality::High => "320",
            AudioQuality::Lossless => "flac",
            AudioQuality::HiRes => "high",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AudioQuality::Standard => "标准 128K",
            AudioQuality::High => "高品 320K",
            AudioQuality::Lossless => "无损 SQ",
            AudioQuality::HiRes => "Hi-Res",
        }
    }

    pub fn short_badge(&self) -> &'static str {
        match self {
            AudioQuality::Standard => "128K",
            AudioQuality::High => "320K",
            AudioQuality::Lossless => "SQ",
            AudioQuality::HiRes => "Hi-Res",
        }
    }

    pub fn from_str(val: &str) -> Self {
        match val.trim().to_lowercase().as_str() {
            "high" | "hires" | "hi-res" | "res" | "super" => AudioQuality::HiRes,
            "flac" | "lossless" | "sq" => AudioQuality::Lossless,
            "320" | "hq" | "high_quality" => AudioQuality::High,
            _ => AudioQuality::Standard,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentity {
    pub guid: String,
    pub mid: String,
    pub dev: String,
    pub mac: String,
    pub dfid: String,
    pub dfid_registered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KugouSession {
    pub user_id: String,
    pub token: String,
    pub t1: Option<String>,
    pub vip_token: Option<String>,
    pub nickname: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub user_id: String,
    pub nickname: String,
    pub avatar_url: String,
    pub is_vip: bool,
    pub svip_active: bool,
    pub tvip_active: bool,
    pub vip_expire_text: String,
    pub listen_duration_min: i64,
    pub listen_duration_sec: i64,
    pub listen_duration_formatted: String,
    pub user_grade: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeSession {
    pub qrcode_key: String,
    pub qrcode_img: String,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrPollResult {
    /// 对齐酷狗官方与 Android 端 LibraryViewModel:
    /// 0 = 二维码已过期, 1 = 等待扫码, 2 = 已扫码待在手机端点击确认, 4 = 授权成功
    pub status: i32,
    pub message: String,
    pub session: Option<KugouSession>,
    pub profile: Option<AccountProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginCandidate {
    pub user_id: String,
    pub nickname: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneLoginOutcome {
    pub status: String, // "success" | "choose_account"
    pub session: Option<KugouSession>,
    pub profile: Option<AccountProfile>,
    pub candidates: Vec<LoginCandidate>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VipDayRecord {
    pub day: String,
    pub received: bool,
    pub vip_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VipStatusInfo {
    pub is_vip: bool,
    pub svip_active: bool,
    pub tvip_active: bool,
    pub svip_expiry: String,
    pub tvip_expiry: String,
    pub today_claimed: bool,
    pub month_claimed_days: usize,
    pub records: Vec<VipDayRecord>,
    pub status_message: String,
    pub dfid: String,
    pub guid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    pub hash: String,
    pub hash128: String,
    pub hash320: String,
    pub hash_flac: String,
    pub hash_hi_res: String,
    pub size128: i64,
    pub size320: i64,
    pub size_flac: i64,
    pub size_hi_res: i64,
    pub title: String,
    pub artist: String,
    pub artist_id: String,
    pub album: String,
    pub album_id: String,
    pub cover_url: String,
    pub duration_ms: i64,
    pub album_audio_id: i64,
    pub audio_id: i64,
    pub mix_song_id: String,
    pub is_vip: bool,
    pub is_lossless: bool,
    pub is_hi_res: bool,
    pub highest_quality: String,
    pub publish_date: String,
}

impl Song {
    pub fn hash_for_quality(&self, quality: &AudioQuality) -> String {
        let pick = |candidates: &[&str]| -> String {
            for c in candidates {
                let trimmed = c.trim();
                if !trimmed.is_empty() && trimmed != "0" {
                    return trimmed.to_lowercase();
                }
            }
            String::new()
        };
        match quality {
            AudioQuality::HiRes => pick(&[
                &self.hash_hi_res,
                &self.hash_flac,
                &self.hash320,
                &self.hash128,
                &self.hash,
            ]),
            AudioQuality::Lossless => {
                pick(&[&self.hash_flac, &self.hash320, &self.hash128, &self.hash])
            }
            AudioQuality::High => pick(&[&self.hash320, &self.hash128, &self.hash]),
            AudioQuality::Standard => pick(&[&self.hash128, &self.hash]),
        }
    }

    pub fn known_size_for_quality(&self, quality: &AudioQuality) -> i64 {
        match quality {
            AudioQuality::HiRes => self.size_hi_res,
            AudioQuality::Lossless => self.size_flac,
            AudioQuality::High => self.size320,
            AudioQuality::Standard => self.size128,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackResult {
    pub url: String,
    pub actual_quality: String,
    pub quality_label: String,
    pub quality_badge: String,
    pub bitrate_kbps: i32,
    pub file_size_bytes: i64,
    pub sample_rate_hz: i32,
    pub bit_depth: i32,
    pub channels: i32,
    pub tech_spec_text: String,
    pub climax_start_ms: i64,
    pub climax_end_ms: i64,
    pub is_free_part: bool,
    pub resolved_hash: String,
    pub ext_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub cover_url: String,
    pub track_count: i32,
    pub description: String,
    pub creator_name: String,
    pub is_created_by_me: bool,
    pub is_favorite: bool,
    pub is_liked: bool,
    pub list_id: i64,
    pub global_collection_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankItem {
    pub rank_id: i32,
    pub rank_name: String,
    pub cover_url: String,
    pub update_frequency: String,
    pub top_songs_preview: Vec<String>,
    pub category: String,
    pub total_songs: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricWord {
    pub start_time_ms: i64,
    pub duration_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start_time_ms: i64,
    pub duration_ms: i64,
    pub text: String,
    pub words: Vec<LyricWord>,
    pub translation: Option<String>,
    pub phonetic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LyricData {
    pub lines: Vec<LyricLine>,
    pub raw_text: String,
    pub is_krc: bool,
    pub has_translation: bool,
    pub has_phonetic: bool,
}
