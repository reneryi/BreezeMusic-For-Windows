pub mod kugou;

use kugou::api::KugouApiService;
use kugou::models::{
    AccountProfile, AlbumItem, DownloadTaskResult, FmChannel, FollowedArtist, GatewayDiagResult,
    KugouSession, LyricData, PhoneLoginOutcome, PlaybackResult, Playlist, PurchasedAlbumItem,
    PurchasedSongItem, QrCodeSession, QrPollResult, RankItem, Song, SongComment, VipStatusInfo,
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
async fn get_fm_channels(state: State<'_, AppState>) -> Result<Vec<FmChannel>, String> {
    state.api.fetch_fm_channels().await
}

#[tauri::command]
async fn get_fm_songs(
    state: State<'_, AppState>,
    channel_id: String,
) -> Result<Vec<Song>, String> {
    state.api.fetch_fm_songs(&channel_id).await
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
async fn get_search_hot(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.api.fetch_search_hot().await
}

#[tauri::command]
async fn search_playlists_cmd(
    state: State<'_, AppState>,
    keyword: String,
) -> Result<Vec<Playlist>, String> {
    state.api.search_playlists(&keyword).await
}

#[tauri::command]
async fn search_albums_cmd(
    state: State<'_, AppState>,
    keyword: String,
) -> Result<Vec<AlbumItem>, String> {
    state.api.search_albums(&keyword).await
}

#[tauri::command]
async fn search_artists_cmd(
    state: State<'_, AppState>,
    keyword: String,
) -> Result<Vec<FollowedArtist>, String> {
    state.api.search_artists(&keyword).await
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
async fn create_user_playlist_cmd(
    state: State<'_, AppState>,
    name: String,
) -> Result<bool, String> {
    state.api.create_user_playlist(&name).await
}

#[tauri::command]
async fn delete_user_playlist_cmd(
    state: State<'_, AppState>,
    list_id: i64,
) -> Result<bool, String> {
    state.api.delete_user_playlist(list_id).await
}

#[tauri::command]
async fn add_song_to_playlist_cmd(
    state: State<'_, AppState>,
    list_id: i64,
    song: Song,
) -> Result<bool, String> {
    state.api.add_song_to_user_playlist(list_id, &song).await
}

#[tauri::command]
async fn get_followed_artists(state: State<'_, AppState>) -> Result<Vec<FollowedArtist>, String> {
    state.api.fetch_followed_artists().await
}

#[tauri::command]
async fn toggle_follow_artist_cmd(
    state: State<'_, AppState>,
    artist_id: String,
    follow: bool,
) -> Result<bool, String> {
    state.api.toggle_follow_artist(&artist_id, follow).await
}

#[tauri::command]
async fn get_purchased_albums(
    state: State<'_, AppState>,
) -> Result<Vec<PurchasedAlbumItem>, String> {
    state.api.fetch_purchased_albums().await
}

#[tauri::command]
async fn get_purchased_songs(
    state: State<'_, AppState>,
) -> Result<Vec<PurchasedSongItem>, String> {
    state.api.fetch_purchased_songs().await
}

#[tauri::command]
async fn get_artist_detail_cmd(
    state: State<'_, AppState>,
    artist_id: String,
) -> Result<FollowedArtist, String> {
    state.api.fetch_artist_detail(&artist_id).await
}

#[tauri::command]
async fn get_artist_songs_cmd(
    state: State<'_, AppState>,
    artist_id: String,
    sort: Option<String>,
    page: Option<u32>,
) -> Result<Vec<Song>, String> {
    state
        .api
        .fetch_artist_songs(&artist_id, &sort.unwrap_or_else(|| "hot".to_string()), page.unwrap_or(1))
        .await
}

#[tauri::command]
async fn get_album_songs_cmd(
    state: State<'_, AppState>,
    album_id: String,
) -> Result<Vec<Song>, String> {
    state.api.fetch_album_songs(&album_id).await
}

#[tauri::command]
async fn get_song_comments_cmd(
    state: State<'_, AppState>,
    song: Song,
    sort_type: Option<String>,
    page: Option<u32>,
) -> Result<Vec<SongComment>, String> {
    state
        .api
        .fetch_song_comments(&song, &sort_type.unwrap_or_else(|| "hot".to_string()), page.unwrap_or(1))
        .await
}

#[tauri::command]
async fn resign_device_cmd(state: State<'_, AppState>) -> Result<GatewayDiagResult, String> {
    state.api.resign_device_identity().await
}

#[tauri::command]
async fn diagnose_gateway_cmd(state: State<'_, AppState>) -> Result<GatewayDiagResult, String> {
    state.api.diagnose_gateway_latency().await
}

#[tauri::command]
async fn download_song_cmd(
    state: State<'_, AppState>,
    song: Song,
    preferred_quality: Option<String>,
    music_dir: Option<String>,
    lrc_dir: Option<String>,
    export_lrc: Option<bool>,
) -> Result<DownloadTaskResult, String> {
    state
        .api
        .download_song_to_local(
            &song,
            &preferred_quality.unwrap_or_else(|| "high".to_string()),
            &music_dir.unwrap_or_default(),
            &lrc_dir.unwrap_or_default(),
            export_lrc.unwrap_or(true),
        )
        .await
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
            get_fm_channels,
            get_fm_songs,
            get_recent_history,
            search_songs,
            get_search_hot,
            search_playlists_cmd,
            search_albums_cmd,
            search_artists_cmd,
            get_user_playlists,
            get_playlist_songs,
            create_user_playlist_cmd,
            delete_user_playlist_cmd,
            add_song_to_playlist_cmd,
            get_followed_artists,
            toggle_follow_artist_cmd,
            get_purchased_albums,
            get_purchased_songs,
            get_artist_detail_cmd,
            get_artist_songs_cmd,
            get_album_songs_cmd,
            get_song_comments_cmd,
            resign_device_cmd,
            diagnose_gateway_cmd,
            download_song_cmd,
            resolve_song_url,
            get_song_lyric,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BreezeMusic Windows application");
}
