use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_i64_flexible<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntOrStr {
        Int(i64),
        Float(f64),
        Str(String),
        Null,
    }
    match IntOrStr::deserialize(deserializer)? {
        IntOrStr::Int(v) => Ok(v),
        IntOrStr::Float(v) => Ok(v as i64),
        IntOrStr::Str(s) => Ok(s.trim().parse::<i64>().unwrap_or(0)),
        IntOrStr::Null => Ok(0),
    }
}

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
            "high" | "hires" | "hi-res" | "res" | "super" | "master" | "atmos" => {
                AudioQuality::HiRes
            }
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountProfile {
    pub user_id: String,
    pub nickname: String,
    pub avatar_url: String,
    pub bg_pic_url: String,
    pub signature: String,
    pub gender: i32,
    pub birthday: String,
    pub province: String,
    pub city: String,
    pub ip_location: String,
    pub occupation: String,
    pub reg_time: i64,
    pub music_age_years: f32,
    pub follows: i64,
    pub fans: i64,
    pub visitors: i64,
    pub is_vip: bool,
    pub svip_active: bool,
    pub tvip_active: bool,
    pub vip_expire_text: String,
    pub listen_duration_min: i64,
    pub listen_duration_sec: i64,
    pub listen_duration_formatted: String,
    pub pending_diff_sec: i64,
    pub user_grade: i32,
    pub next_grade: i32,
    pub raw_server_dsec: i64,
    pub lite_server_dsec: i64,
    pub grade_base_score: i64,
    pub user_score: i64,
    pub next_level_score: i64,
    pub next_level_remain_score: i64,
    pub grade_speed: f32,
    pub level_progress_pct: f32,
    pub next_level_remain_min: i64,
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
#[serde(rename_all = "camelCase", default)]
pub struct SongAuthor {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Song {
    pub id: String,
    pub hash: String,
    pub hash128: String,
    pub hash320: String,
    pub hash_flac: String,
    pub hash_hi_res: String,
    pub accompany_hash: String,
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub size128: i64,
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub size320: i64,
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub size_flac: i64,
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub size_hi_res: i64,
    pub title: String,
    pub artist: String,
    pub artist_id: String,
    #[serde(default)]
    pub authors: Vec<SongAuthor>,
    pub album: String,
    pub album_id: String,
    pub cover_url: String,
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub duration_ms: i64,
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub album_audio_id: i64,
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub audio_id: i64,
    pub mix_song_id: String,
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub file_id: i64,
    pub is_vip: bool,
    pub is_lossless: bool,
    pub is_hi_res: bool,
    pub highest_quality: String,
    pub publish_date: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folded_versions: Vec<Song>,
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
    #[serde(default)]
    pub backup_urls: Vec<String>,
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
    #[serde(default)]
    pub proxy_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
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
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub list_id: i64,
    pub global_collection_id: String,
    #[serde(default)]
    pub play_count: String,
    #[serde(default)]
    pub list_create_userid: String,
    #[serde(default)]
    pub list_create_listid: String,
    #[serde(default)]
    pub list_create_gid: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FmChannel {
    pub id: String,
    pub name: String,
    pub cover_url: String,
    pub description: String,
    pub category: String,
    pub fm_type: i32,
    pub listener_count_text: String,
    pub freq_label: String,
    pub banner_url: String,
    pub is_recommended: bool,
    pub sample_song: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FollowedArtist {
    pub artist_id: String,
    pub target_user_id: String,
    pub name: String,
    pub avatar_url: String,
    pub song_count: i32,
    pub album_count: i32,
    pub fans_count: i32,
    pub is_followed: bool,
    pub intro: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AlbumItem {
    pub album_id: String,
    pub album_name: String,
    pub artist_name: String,
    pub artist_id: String,
    pub cover_url: String,
    pub publish_date: String,
    pub publish_company: String,
    pub song_count: i32,
    pub intro: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasedAlbumItem {
    pub album_id: String,
    pub album_name: String,
    pub artist_name: String,
    pub cover_url: String,
    pub song_count: i32,
    pub buy_time: String,
    pub publish_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasedSongItem {
    pub song: Song,
    pub buy_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct QuotedReplyHop {
    pub user_name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SongComment {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub user_name: String,
    pub user_avatar: String,
    pub content: String,
    pub created_time: String,
    pub like_count: i32,
    pub is_liked: bool,
    #[serde(default)]
    pub is_hot: bool,
    #[serde(default)]
    pub vip_level: i32,
    pub location: String,
    pub reply_count: i32,
    pub special_child_id: String,
    pub replies: Vec<SongComment>,
    pub images: Vec<String>,
    pub reply_user_name: Option<String>,
    pub reply_quoted_content: Option<String>,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub root_id: Option<String>,
    #[serde(default)]
    pub reply_target_comment_id: Option<String>,
    #[serde(default)]
    pub reply_chain_hops: Vec<QuotedReplyHop>,
    #[serde(default)]
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayDiagResult {
    pub latency_ms: i64,
    pub dfid: String,
    pub mid: String,
    pub guid: String,
    pub status_text: String,
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskResult {
    pub success: bool,
    pub saved_audio_path: String,
    pub saved_lrc_path: String,
    pub file_size_bytes: i64,
    pub quality_label: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViperEffectItem {
    pub id: String,
    pub mode_id: String,
    pub label: String,
    pub desc: String,
    pub category: String,
    pub usage_badge: String,
    pub author_name: String,
    pub acoustic_tags: Vec<String>,
    pub gains: Vec<f32>,
    pub sub_bass: i32,
    pub surround: i32,
    pub vocal_clarity: i32,
    pub air_harmonics: i32,
    pub tube_warmth: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongDetailInfo {
    pub song: Song,
    pub album_name: String,
    pub singer_name: String,
    pub publish_time: String,
    pub company: String,
    pub audio_quality: String,
    pub sample_rate: String,
    pub bit_rate: String,
    pub file_size: String,
    pub file_hash: String,
    pub has_accompaniment: bool,
    pub accompany_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SharedMusicResolved {
    pub target_type: String, // "song" | "playlist" | "unknown"
    pub song: Option<Song>,
    pub playlist: Option<Playlist>,
    pub songs: Vec<Song>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ResolvedSongAlbumMeta {
    pub album_name: String,
    pub album_id: String,
    pub artist_id: String,
    pub artist_name: String,
    pub cover_url: String,
    pub publish_date: String,
    pub company: String,
}


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CacheDiskStats {
    pub audio_stream_mb: f64,
    pub artwork_image_mb: f64,
    pub playlist_snapshot_mb: f64,
    pub lyric_and_url_mb: f64,
    pub total_mb: f64,
    pub cache_dir_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct UserListeningGradeState {
    pub schema_ver: u32,
    pub synced_official_listen_sec: i64,
    pub synced_raw_server_dsec: i64,
    pub pending_diff_sec: i64,
    pub inflight_diff_sec: i64,
    pub inflight_at_sec: i64,
    pub last_known_grade: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ListeningGradeSyncResult {
    pub profile: AccountProfile,
    pub reported_diff_sec: i64,
    pub cloud_confirmed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PagedSearchResult<T> {
    pub items: Vec<T>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HotSearchEntry {
    pub keyword: String,
    pub reason: String,
    pub icon_type: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HotSearchBoard {
    pub name: String,
    pub entries: Vec<HotSearchEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchDiscoveryData {
    pub hot_songs: Vec<Song>,
    pub hot_boards: Vec<HotSearchBoard>,
    pub hot_artists: Vec<FollowedArtist>,
}


