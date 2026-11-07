use crate::kugou::models::{
    DeviceIdentity, KugouSession, Playlist, Song, UserListeningGradeState,
};
use crate::kugou::signer;
use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

/// 统一将软件运行产生的目录（cache、config、webview_data、downloads）建立在软件安装目录下，避免占用 C 盘 AppData 空间
static INSTALL_ROOT_DIR: Lazy<PathBuf> = Lazy::new(|| {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let raw = parent.to_string_lossy().to_string();
            let clean = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
            let candidate = PathBuf::from(clean);
            let test_cache = candidate.join("cache");
            if fs::create_dir_all(&test_cache).is_ok() {
                return candidate;
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let test_cache = cwd.join("cache");
        if fs::create_dir_all(&test_cache).is_ok() {
            return cwd;
        }
    }
    PathBuf::from(".")
});

/// 获取软件安装根目录（即可执行文件所在目录）
pub fn resolve_install_root_dir() -> PathBuf {
    INSTALL_ROOT_DIR.clone()
}

/// 获取软件安装目录下的核心缓存目录 (`<安装目录>/cache`)
pub fn resolve_cache_dir() -> PathBuf {
    let dir = resolve_install_root_dir().join("cache");
    let _ = fs::create_dir_all(dir.join("audio_streams"));
    dir
}

/// 获取软件安装目录下的配置与会话存储目录 (`<安装目录>/config`)
pub fn resolve_config_dir() -> PathBuf {
    let dir = resolve_install_root_dir().join("config");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取软件安装目录下的 WebView2 独立用户数据与浏览器缓存目录 (`<安装目录>/webview_data`)
pub fn resolve_webview_data_dir() -> PathBuf {
    let dir = resolve_install_root_dir().join("webview_data");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取软件安装目录下的默认音乐下载目录 (`<安装目录>/downloads`)
pub fn resolve_default_download_dir() -> PathBuf {
    let dir = resolve_install_root_dir().join("downloads");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 自动将旧版 C 盘 `AppData\Roaming\concept\BreezeMusic\config` 下的登录态与播放历史平滑迁移至安装目录，并清理旧版 C 盘 AppData 缓存目录
pub fn migrate_legacy_appdata_if_needed() {
    let new_config_dir = resolve_config_dir();
    let new_session_file = new_config_dir.join("session_state.json");

    if let Some(legacy_proj) = ProjectDirs::from("com", "concept", "BreezeMusic") {
        let legacy_config_dir = legacy_proj.config_dir();
        if legacy_config_dir != new_config_dir.as_path() && legacy_config_dir.exists() {
            if !new_session_file.exists() {
                if let Ok(entries) = fs::read_dir(legacy_config_dir) {
                    for entry in entries.flatten() {
                        let src_path = entry.path();
                        if src_path.is_file() {
                            if let Some(fname) = src_path.file_name() {
                                let dst_path = new_config_dir.join(fname);
                                if !dst_path.exists() {
                                    let _ = fs::copy(&src_path, &dst_path);
                                }
                            }
                        }
                    }
                }
            }
            // 迁移完成后清理旧版 AppData\Roaming\concept 目录
            if new_session_file.exists() {
                if let Some(breeze_dir) = legacy_config_dir.parent() {
                    if let Some(concept_dir) = breeze_dir.parent() {
                        if concept_dir
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.eq_ignore_ascii_case("concept"))
                            .unwrap_or(false)
                        {
                            let _ = fs::remove_dir_all(concept_dir);
                        }
                    }
                }
            }
        }

        // 自动清理旧版遗留在 C:\Users\<User>\AppData\Local\concept 的缓存目录，释放 C 盘空间
        let legacy_cache_dir = legacy_proj.cache_dir();
        if legacy_cache_dir != resolve_cache_dir().as_path() && legacy_cache_dir.exists() {
            if let Some(breeze_dir) = legacy_cache_dir.parent() {
                if let Some(concept_dir) = breeze_dir.parent() {
                    if concept_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.eq_ignore_ascii_case("concept"))
                        .unwrap_or(false)
                    {
                        let _ = fs::remove_dir_all(concept_dir);
                    } else {
                        let _ = fs::remove_dir_all(legacy_cache_dir);
                    }
                }
            }
        }
    }

    // 自动清理旧版 WebView2 遗留在 C:\Users\<User>\AppData\Local\com.concept.breezemusic.desktop 的缓存目录
    if let Some(base_dirs) = directories::BaseDirs::new() {
        let legacy_webview = base_dirs
            .data_local_dir()
            .join("com.concept.breezemusic.desktop");
        if legacy_webview != resolve_webview_data_dir() && legacy_webview.exists() {
            let _ = fs::remove_dir_all(&legacy_webview);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PlaylistCustomMeta {
    pub title: String,
    pub description: String,
    pub cover_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedState {
    pub device: DeviceIdentity,
    pub session: Option<KugouSession>,
    #[serde(default)]
    pub saved_accounts: Vec<KugouSession>,
    pub preferred_quality: String,
    #[serde(default)]
    pub playlist_overrides: BTreeMap<String, PlaylistCustomMeta>,
    #[serde(default)]
    pub collected_playlist_cache: BTreeMap<String, Playlist>,
    #[serde(default)]
    pub grade_states: BTreeMap<String, UserListeningGradeState>,
}

impl Default for PersistedState {
    fn default() -> Self {
        let guid = signer::md5_hex(&Uuid::new_v4().to_string());
        let dev = signer::random_alphanumeric(10).to_uppercase();
        let mid = signer::calc_mid(&guid);
        let fallback_dfid = format!(
            "2{}",
            &signer::md5_hex(&format!("kugou_lite_dfid_{}_{}", guid, dev))[..23]
        );
        Self {
            device: DeviceIdentity {
                guid,
                mid,
                dev,
                mac: "02:00:00:00:00:00".to_string(),
                dfid: fallback_dfid,
                dfid_registered: false,
            },
            session: None,
            saved_accounts: Vec::new(),
            preferred_quality: "flac".to_string(),
            playlist_overrides: BTreeMap::new(),
            collected_playlist_cache: BTreeMap::new(),
            grade_states: BTreeMap::new(),
        }
    }
}

pub fn is_same_music_track(a: &Song, b: &Song) -> bool {
    let collect_hashes = |s: &Song| -> Vec<String> {
        [
            &s.hash,
            &s.hash128,
            &s.hash320,
            &s.hash_flac,
            &s.hash_hi_res,
        ]
        .into_iter()
        .map(|h| h.trim().to_lowercase())
        .filter(|h| h.len() == 32 && !h.chars().all(|c| c == '0'))
        .collect()
    };
    let ha = collect_hashes(a);
    let hb = collect_hashes(b);
    if !ha.is_empty() && !hb.is_empty() && ha.iter().any(|x| hb.contains(x)) {
        return true;
    }
    if a.audio_id > 0 && b.audio_id > 0 && a.audio_id == b.audio_id {
        return true;
    }
    if a.album_audio_id > 0 && b.album_audio_id > 0 && a.album_audio_id == b.album_audio_id {
        return true;
    }
    if !a.mix_song_id.is_empty()
        && a.mix_song_id != "0"
        && a.mix_song_id == b.mix_song_id
    {
        return true;
    }
    let norm = |s: &str| -> String {
        s.trim()
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect()
    };
    let ta = norm(&a.title);
    let tb = norm(&b.title);
    let aa = norm(&a.artist);
    let ab = norm(&b.artist);
    !ta.is_empty()
        && ta != "未知歌曲"
        && ta == tb
        && !aa.is_empty()
        && aa != "未知歌手"
        && aa == ab
}

pub fn merge_song_entries(preferred: &Song, fallback: &Song) -> Song {
    let mut out = preferred.clone();
    if out.hash.is_empty() && !fallback.hash.is_empty() {
        out.hash = fallback.hash.clone();
    }
    if out.hash128.is_empty() && !fallback.hash128.is_empty() {
        out.hash128 = fallback.hash128.clone();
    }
    if out.hash320.is_empty() && !fallback.hash320.is_empty() {
        out.hash320 = fallback.hash320.clone();
    }
    if out.hash_flac.is_empty() && !fallback.hash_flac.is_empty() {
        out.hash_flac = fallback.hash_flac.clone();
    }
    if out.hash_hi_res.is_empty() && !fallback.hash_hi_res.is_empty() {
        out.hash_hi_res = fallback.hash_hi_res.clone();
    }
    if out.cover_url.is_empty() && !fallback.cover_url.is_empty() {
        out.cover_url = fallback.cover_url.clone();
    }
    if out.album.is_empty() && !fallback.album.is_empty() {
        out.album = fallback.album.clone();
    }
    if (out.album_id.is_empty() || out.album_id == "0")
        && !fallback.album_id.is_empty()
        && fallback.album_id != "0"
    {
        out.album_id = fallback.album_id.clone();
    }
    if out.duration_ms <= 0 && fallback.duration_ms > 0 {
        out.duration_ms = fallback.duration_ms;
    }
    if out.album_audio_id <= 0 && fallback.album_audio_id > 0 {
        out.album_audio_id = fallback.album_audio_id;
    }
    if out.audio_id <= 0 && fallback.audio_id > 0 {
        out.audio_id = fallback.audio_id;
    }
    if !out.is_hi_res && fallback.is_hi_res {
        out.is_hi_res = true;
        out.is_lossless = true;
        out.highest_quality = "Hi-Res".to_string();
    } else if !out.is_lossless && fallback.is_lossless {
        out.is_lossless = true;
        if out.highest_quality != "Hi-Res" {
            out.highest_quality = "SQ".to_string();
        }
    }
    out
}

pub struct AppStore {
    file_path: PathBuf,
    pub state: Mutex<PersistedState>,
}

impl AppStore {
    pub fn new() -> Self {
        migrate_legacy_appdata_if_needed();
        let file_path = Self::resolve_store_path();
        let initial_state = if file_path.exists() {
            fs::read_to_string(&file_path)
                .ok()
                .and_then(|raw| serde_json::from_str::<PersistedState>(&raw).ok())
                .unwrap_or_default()
        } else {
            let default_state = PersistedState::default();
            if let Some(parent) = file_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&default_state) {
                let _ = fs::write(&file_path, json);
            }
            default_state
        };

        Self {
            file_path,
            state: Mutex::new(initial_state),
        }
    }

    fn resolve_store_path() -> PathBuf {
        resolve_config_dir().join("session_state.json")
    }

    fn resolve_play_history_path(&self, user_id: &str) -> PathBuf {
        let clean_uid: String = user_id
            .trim()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let uid_key = if clean_uid.is_empty() {
            "guest"
        } else {
            &clean_uid
        };
        if let Some(parent) = self.file_path.parent() {
            parent.join(format!("play_history_{}.json", uid_key))
        } else {
            PathBuf::from(format!("play_history_{}.json", uid_key))
        }
    }

    pub fn load_play_history(&self, user_id: &str) -> Vec<Song> {
        let path = self.resolve_play_history_path(user_id);
        if !path.exists() {
            return Vec::new();
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Vec<Song>>(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save_play_history(&self, user_id: &str, songs: &[Song]) {
        let path = self.resolve_play_history_path(user_id);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let capped: Vec<&Song> = songs.iter().take(200).collect();
        if let Ok(json) = serde_json::to_string(&capped) {
            let _ = fs::write(&path, json);
        }
    }

    pub fn append_play_history(&self, user_id: &str, song: &Song) -> Vec<Song> {
        if song.hash.trim().is_empty() && song.id.trim().is_empty() {
            return self.load_play_history(user_id);
        }
        let existing = self.load_play_history(user_id);
        let mut merged = Vec::with_capacity(existing.len().min(200) + 1);
        let mut head = song.clone();
        for old in existing {
            if is_same_music_track(&head, &old) {
                head = merge_song_entries(&head, &old);
            } else if merged.len() < 199 {
                merged.push(old);
            }
        }
        let mut final_list = Vec::with_capacity(merged.len() + 1);
        final_list.push(head);
        final_list.extend(merged);
        self.save_play_history(user_id, &final_list);
        final_list
    }

    pub fn get_grade_state(&self, user_id: &str) -> UserListeningGradeState {
        let uid = user_id.trim();
        if uid.is_empty() {
            return UserListeningGradeState::default();
        }
        let guard = self.state.lock().unwrap();
        guard.grade_states.get(uid).cloned().unwrap_or_default()
    }

    pub fn set_grade_state(&self, user_id: &str, state: UserListeningGradeState) {
        let uid = user_id.trim();
        if uid.is_empty() {
            return;
        }
        {
            let mut guard = self.state.lock().unwrap();
            guard.grade_states.insert(uid.to_string(), state);
        }
        self.save();
    }

    pub fn add_pending_listen_sec(&self, user_id: &str, delta_sec: i64) -> i64 {
        let uid = user_id.trim();
        if uid.is_empty() || delta_sec <= 0 {
            return 0;
        }
        let clamped = delta_sec.clamp(1, 600);
        let new_pending = {
            let mut guard = self.state.lock().unwrap();
            let entry = guard.grade_states.entry(uid.to_string()).or_default();
            entry.pending_diff_sec = (entry.pending_diff_sec + clamped).clamp(0, 86_400);
            entry.pending_diff_sec
        };
        self.save();
        new_pending
    }

    pub fn save(&self) {
        if let Ok(guard) = self.state.lock() {
            if let Some(parent) = self.file_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&*guard) {
                let _ = fs::write(&self.file_path, json);
            }
        }
    }

    pub fn device(&self) -> DeviceIdentity {
        self.state.lock().unwrap().device.clone()
    }

    pub fn save_dfid(&self, dfid: &str) {
        if dfid.trim().is_empty() || dfid == "-" {
            return;
        }
        {
            let mut guard = self.state.lock().unwrap();
            guard.device.dfid = dfid.trim().to_string();
            guard.device.dfid_registered = true;
        }
        self.save();
    }

    pub fn session(&self) -> Option<KugouSession> {
        self.state.lock().unwrap().session.clone()
    }

    pub fn set_session(&self, session: Option<KugouSession>) {
        {
            let mut guard = self.state.lock().unwrap();
            if let Some(ref s) = session {
                if !s.user_id.trim().is_empty() && !s.token.trim().is_empty() {
                    guard.saved_accounts.retain(|a| a.user_id != s.user_id);
                    guard.saved_accounts.insert(0, s.clone());
                    if guard.saved_accounts.len() > 8 {
                        guard.saved_accounts.truncate(8);
                    }
                }
            }
            guard.session = session;
        }
        self.save();
    }

    pub fn saved_accounts(&self) -> Vec<KugouSession> {
        let guard = self.state.lock().unwrap();
        let mut list = guard.saved_accounts.clone();
        if let Some(ref cur) = guard.session {
            if !list.iter().any(|a| a.user_id == cur.user_id) {
                list.insert(0, cur.clone());
            }
        }
        list
    }

    pub fn switch_account(&self, user_id: &str) -> Option<KugouSession> {
        let target = {
            let guard = self.state.lock().unwrap();
            guard
                .saved_accounts
                .iter()
                .find(|a| a.user_id == user_id)
                .cloned()
        };
        if let Some(ref s) = target {
            self.set_session(Some(s.clone()));
        }
        target
    }

    pub fn remove_saved_account(&self, user_id: &str) -> Vec<KugouSession> {
        {
            let mut guard = self.state.lock().unwrap();
            guard.saved_accounts.retain(|a| a.user_id != user_id);
            if guard
                .session
                .as_ref()
                .map(|s| s.user_id == user_id)
                .unwrap_or(false)
            {
                guard.session = guard.saved_accounts.first().cloned();
            }
        }
        self.save();
        self.saved_accounts()
    }

    pub fn set_playlist_override(
        &self,
        key: &str,
        title: &str,
        description: &str,
        cover_url: &str,
    ) {
        if key.trim().is_empty() {
            return;
        }
        {
            let mut guard = self.state.lock().unwrap();
            guard.playlist_overrides.insert(
                key.trim().to_string(),
                PlaylistCustomMeta {
                    title: title.trim().to_string(),
                    description: description.trim().to_string(),
                    cover_url: cover_url.trim().to_string(),
                },
            );
        }
        self.save();
    }

    pub fn get_playlist_override(&self, key: &str) -> Option<PlaylistCustomMeta> {
        let guard = self.state.lock().unwrap();
        guard.playlist_overrides.get(key.trim()).cloned()
    }

    pub fn cache_collected_playlist(&self, playlist: &Playlist) {
        {
            let mut guard = self.state.lock().unwrap();
            for key in [
                playlist.id.trim(),
                playlist.global_collection_id.trim(),
                playlist.list_create_gid.trim(),
                playlist.list_create_listid.trim(),
            ] {
                if !key.is_empty() && key != "0" {
                    guard
                        .collected_playlist_cache
                        .insert(key.to_string(), playlist.clone());
                }
            }
        }
        self.save();
    }

    pub fn get_cached_collected_playlist(&self, playlist: &Playlist) -> Option<Playlist> {
        let guard = self.state.lock().unwrap();
        for key in [
            playlist.global_collection_id.trim(),
            playlist.list_create_gid.trim(),
            playlist.id.trim(),
            playlist.list_create_listid.trim(),
        ] {
            if !key.is_empty() && key != "0" {
                if let Some(found) = guard.collected_playlist_cache.get(key) {
                    return Some(found.clone());
                }
            }
        }
        None
    }
}
