use crate::kugou::models::{DeviceIdentity, KugouSession};
use crate::kugou::signer;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedState {
    pub device: DeviceIdentity,
    pub session: Option<KugouSession>,
    pub preferred_quality: String,
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
            preferred_quality: "flac".to_string(),
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
            guard.session = session;
        }
        self.save();
    }
}
