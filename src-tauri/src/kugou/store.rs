use crate::kugou::models::{DeviceIdentity, KugouSession, Playlist};
use crate::kugou::signer;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

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
        }
    }
}

pub struct AppStore {
    file_path: PathBuf,
    pub state: Mutex<PersistedState>,
}

impl AppStore {
    pub fn new() -> Self {
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
        if let Some(proj_dirs) = ProjectDirs::from("com", "concept", "BreezeMusic") {
            proj_dirs.config_dir().join("session_state.json")
        } else {
            PathBuf::from("breezemusic_session_state.json")
        }
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

    pub fn set_playlist_override(&self, key: &str, title: &str, description: &str, cover_url: &str) {
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
