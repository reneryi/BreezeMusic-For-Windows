pub mod kugou;

use kugou::api::KugouApiService;
use kugou::models::{
    AccountProfile, KugouSession, LyricData, PhoneLoginOutcome, PlaybackResult, Playlist,
    QrCodeSession, QrPollResult, RankItem, Song, VipStatusInfo,
};
use kugou::store::AppStore;
use std::sync::Arc;
use tauri::State;

pub struct AppState {
    pub api: Arc<KugouApiService>,
    pub store: Arc<AppStore>,
}

#[tauri::command]
async fn init_app_state(state: State<'_, AppState>) -> Result<Option<KugouSession>, String> {
    let _ = state.api.register_device(false).await;
    Ok(state.store.session())
}

#[tauri::command]
async fn send_sms_code(state: State<'_, AppState>, mobile: String) -> Result<String, String> {
    state.api.send_mobile_code(&mobile).await
}

#[tauri::command]
async fn login_with_sms(
    state: State<'_, AppState>,
    mobile: String,
    code: String,
    user_id: Option<String>,
) -> Result<PhoneLoginOutcome, String> {
    state.api.login_by_phone(&mobile, &code, user_id).await
}

#[tauri::command]
async fn create_qr_login(state: State<'_, AppState>) -> Result<QrCodeSession, String> {
    state.api.create_qr_code().await
}

#[tauri::command]
async fn check_qr_login(
    state: State<'_, AppState>,
    qrcode_key: String,
) -> Result<QrPollResult, String> {
    state.api.check_qr_code(&qrcode_key).await
}

#[tauri::command]
async fn logout_account(state: State<'_, AppState>) -> Result<(), String> {
    state.store.set_session(None);
    Ok(())
}

#[tauri::command]
async fn get_account_profile(state: State<'_, AppState>) -> Result<AccountProfile, String> {
    state.api.fetch_full_profile().await
}

#[tauri::command]
async fn get_vip_status(state: State<'_, AppState>) -> Result<VipStatusInfo, String> {
    state.api.fetch_vip_status().await
}

#[tauri::command]
async fn claim_daily_vip(state: State<'_, AppState>) -> Result<VipStatusInfo, String> {
    state.api.claim_daily_vip().await
}

#[tauri::command]
async fn get_daily_recommend(state: State<'_, AppState>) -> Result<Vec<Song>, String> {
    state.api.fetch_daily_recommend().await
}

#[tauri::command]
async fn get_personal_fm(state: State<'_, AppState>) -> Result<Vec<Song>, String> {
    state.api.fetch_personal_fm().await
}

#[tauri::command]
async fn get_new_song_express(
    state: State<'_, AppState>,
    period_key: Option<String>,
) -> Result<Vec<Song>, String> {
    let key = period_key.unwrap_or_else(|| "CHINESE".to_string());
    state.api.fetch_new_song_express(&key).await
}

#[tauri::command]
async fn get_rank_list(state: State<'_, AppState>) -> Result<Vec<RankItem>, String> {
    state.api.fetch_rank_list().await
}

#[tauri::command]
async fn get_rank_songs(
    state: State<'_, AppState>,
    rank_id: i32,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Vec<Song>, String> {
    state
        .api
        .fetch_rank_songs(rank_id, page.unwrap_or(1), page_size.unwrap_or(50))
        .await
}

#[tauri::command]
async fn get_recent_history(state: State<'_, AppState>) -> Result<Vec<Song>, String> {
    state.api.fetch_recent_history().await
}

#[tauri::command]
async fn search_songs(
    state: State<'_, AppState>,
    keyword: String,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Vec<Song>, String> {
    state
        .api
        .search_songs(&keyword, page.unwrap_or(1), page_size.unwrap_or(30))
        .await
}

#[tauri::command]
async fn get_user_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    state.api.fetch_user_playlists().await
}

#[tauri::command]
async fn get_playlist_songs(
    state: State<'_, AppState>,
    collection_id: String,
) -> Result<Vec<Song>, String> {
    state.api.fetch_playlist_songs(&collection_id).await
}

#[tauri::command]
async fn resolve_song_url(
    state: State<'_, AppState>,
    song: Song,
    preferred_quality: Option<String>,
) -> Result<PlaybackResult, String> {
    let pref = preferred_quality.unwrap_or_else(|| "flac".to_string());
    state.api.resolve_song_url(&song, &pref).await
}

#[tauri::command]
async fn get_song_lyric(state: State<'_, AppState>, song: Song) -> Result<LyricData, String> {
    state.api.fetch_song_lyric(&song).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Arc::new(AppStore::new());
    let api = Arc::new(KugouApiService::new(store.clone()));

    tauri::Builder::default()
        .manage(AppState { api, store })
        .invoke_handler(tauri::generate_handler![
            init_app_state,
            send_sms_code,
            login_with_sms,
            create_qr_login,
            check_qr_login,
            logout_account,
            get_account_profile,
            get_vip_status,
            claim_daily_vip,
            get_daily_recommend,
            get_personal_fm,
            get_new_song_express,
            get_rank_list,
            get_rank_songs,
            get_recent_history,
            search_songs,
            get_user_playlists,
            get_playlist_songs,
            resolve_song_url,
            get_song_lyric,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BreezeMusic Windows application");
}
