pub mod kugou;

use kugou::api::KugouApiService;
use kugou::models::{
    AccountProfile, AlbumItem, CacheDiskStats, DownloadTaskResult, FmChannel, FollowedArtist,
    GatewayDiagResult, KugouSession, LyricData, PhoneLoginOutcome, PlaybackResult, Playlist,
    PurchasedAlbumItem, PurchasedSongItem, QrCodeSession, QrPollResult, RankItem,
    ResolvedSongAlbumMeta, SharedMusicResolved, Song, SongComment, SongDetailInfo, VipStatusInfo,
    ViperEffectItem,
};
use kugou::store::AppStore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalSize, Manager, Size, State, WindowEvent,
};

pub struct AppState {
    pub api: Arc<KugouApiService>,
    pub store: Arc<AppStore>,
    pub close_to_tray: Arc<AtomicBool>,
}

#[tauri::command]
async fn init_app_state(state: State<'_, AppState>) -> Result<Option<KugouSession>, String> {
    let _ = state.api.register_device(false).await;
    Ok(state.store.session())
}

#[tauri::command]
async fn get_saved_accounts_cmd(state: State<'_, AppState>) -> Result<Vec<KugouSession>, String> {
    Ok(state.store.saved_accounts())
}

#[tauri::command]
async fn switch_saved_account_cmd(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Option<KugouSession>, String> {
    Ok(state.store.switch_account(&user_id))
}

#[tauri::command]
async fn remove_saved_account_cmd(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<KugouSession>, String> {
    Ok(state.store.remove_saved_account(&user_id))
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
async fn get_account_profile(
    state: State<'_, AppState>,
    reported_d_sec: Option<i64>,
    reported_diff_sec: Option<i64>,
) -> Result<AccountProfile, String> {
    state
        .api
        .fetch_full_profile_with_report(reported_d_sec, reported_diff_sec)
        .await
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
async fn get_personal_fm(
    state: State<'_, AppState>,
    exclude_hashes: Option<Vec<String>>,
) -> Result<Vec<Song>, String> {
    state
        .api
        .fetch_personal_fm_with_excludes(exclude_hashes)
        .await
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
        .fetch_rank_songs(rank_id, page.unwrap_or(1), page_size.unwrap_or(100))
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
        .search_songs(&keyword, page.unwrap_or(1), page_size.unwrap_or(40))
        .await
}

#[tauri::command]
async fn get_search_hot(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.api.fetch_search_hot().await
}

#[tauri::command]
async fn get_search_suggest_cmd(
    state: State<'_, AppState>,
    keyword: String,
) -> Result<Vec<String>, String> {
    state.api.fetch_search_suggest(&keyword).await
}

#[tauri::command]
async fn search_playlists_cmd(
    state: State<'_, AppState>,
    keyword: String,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Vec<Playlist>, String> {
    state
        .api
        .search_playlists(&keyword, page.unwrap_or(1), page_size.unwrap_or(30))
        .await
}

#[tauri::command]
async fn search_albums_cmd(
    state: State<'_, AppState>,
    keyword: String,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Vec<AlbumItem>, String> {
    state
        .api
        .search_albums(&keyword, page.unwrap_or(1), page_size.unwrap_or(30))
        .await
}

#[tauri::command]
async fn search_artists_cmd(
    state: State<'_, AppState>,
    keyword: String,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Vec<FollowedArtist>, String> {
    state
        .api
        .search_artists(&keyword, page.unwrap_or(1), page_size.unwrap_or(24))
        .await
}

#[tauri::command]
async fn get_user_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    state.api.fetch_user_playlists().await
}

#[tauri::command]
async fn get_playlist_info_cmd(
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<Playlist, String> {
    state.api.fetch_playlist_info(&playlist_id).await
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
async fn update_user_playlist_cmd(
    state: State<'_, AppState>,
    list_id: i64,
    global_id: Option<String>,
    title: String,
    description: Option<String>,
) -> Result<bool, String> {
    state
        .api
        .update_user_playlist(
            list_id,
            &global_id.unwrap_or_default(),
            &title,
            &description.unwrap_or_default(),
        )
        .await
}

#[tauri::command]
async fn delete_user_playlist_cmd(
    state: State<'_, AppState>,
    list_id: i64,
) -> Result<bool, String> {
    state.api.delete_user_playlist(list_id).await
}

#[tauri::command]
async fn collect_playlist_cmd(
    state: State<'_, AppState>,
    playlist: Playlist,
) -> Result<bool, String> {
    state.api.collect_playlist_cloud(&playlist).await
}

#[tauri::command]
async fn uncollect_playlist_cmd(
    state: State<'_, AppState>,
    playlist: Playlist,
) -> Result<bool, String> {
    state.api.uncollect_playlist_cloud(&playlist).await
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
async fn add_songs_to_playlist_batch_cmd(
    state: State<'_, AppState>,
    list_id: i64,
    songs: Vec<Song>,
) -> Result<usize, String> {
    state.api.add_songs_to_playlist_batch(list_id, &songs).await
}

#[tauri::command]
async fn remove_song_from_playlist_cmd(
    state: State<'_, AppState>,
    list_id: i64,
    song: Song,
) -> Result<bool, String> {
    state.api.remove_song_from_user_playlist(list_id, &song).await
}

#[tauri::command]
async fn resolve_song_album_meta_cmd(
    state: State<'_, AppState>,
    song: Song,
) -> Result<ResolvedSongAlbumMeta, String> {
    state.api.resolve_song_album_meta(&song).await
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
        .fetch_artist_songs(
            &artist_id,
            &sort.unwrap_or_else(|| "hot".to_string()),
            page.unwrap_or(1),
        )
        .await
}

#[tauri::command]
async fn get_artist_albums_cmd(
    state: State<'_, AppState>,
    artist_id: String,
    page: Option<u32>,
    sort: Option<String>,
) -> Result<Vec<AlbumItem>, String> {
    state
        .api
        .fetch_artist_albums(
            &artist_id,
            page.unwrap_or(1),
            &sort.unwrap_or_else(|| "new".to_string()),
        )
        .await
}

#[tauri::command]
async fn get_album_detail_cmd(
    state: State<'_, AppState>,
    album_id: String,
) -> Result<AlbumItem, String> {
    state.api.fetch_album_detail(&album_id).await
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
        .fetch_song_comments(
            &song,
            &sort_type.unwrap_or_else(|| "hot".to_string()),
            page.unwrap_or(1),
        )
        .await
}

#[tauri::command]
async fn get_comment_floor_replies_cmd(
    state: State<'_, AppState>,
    song: Song,
    comment_id: String,
    special_child_id: Option<String>,
    page: Option<u32>,
) -> Result<Vec<SongComment>, String> {
    state
        .api
        .fetch_comment_floor_replies(
            &song,
            &comment_id,
            &special_child_id.unwrap_or_default(),
            page.unwrap_or(1),
        )
        .await
}

#[tauri::command]
async fn like_song_comment_cmd(
    state: State<'_, AppState>,
    song: Song,
    comment_id: String,
    like: bool,
) -> Result<bool, String> {
    state.api.like_song_comment(&song, &comment_id, like).await
}

#[tauri::command]
async fn post_song_comment_cmd(
    state: State<'_, AppState>,
    song: Song,
    content: String,
    reply_to_id: Option<String>,
) -> Result<SongComment, String> {
    state
        .api
        .post_song_comment(&song, &content, reply_to_id.as_deref())
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
    embed_cover_and_tags: Option<bool>,
    embed_lyrics_in_audio: Option<bool>,
    bilingual_lrc: Option<bool>,
    skip_existing: Option<bool>,
) -> Result<DownloadTaskResult, String> {
    state
        .api
        .download_song_to_local(
            &song,
            &preferred_quality.unwrap_or_else(|| "high".to_string()),
            &music_dir.unwrap_or_default(),
            &lrc_dir.unwrap_or_default(),
            export_lrc.unwrap_or(true),
            embed_cover_and_tags.unwrap_or(true),
            embed_lyrics_in_audio.unwrap_or(true),
            bilingual_lrc.unwrap_or(true),
            skip_existing.unwrap_or(true),
        )
        .await
}

#[tauri::command]
async fn parse_shared_music_cmd(
    state: State<'_, AppState>,
    raw_text: String,
) -> Result<SharedMusicResolved, String> {
    state.api.parse_shared_music_text(&raw_text).await
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
async fn resolve_song_accompaniment_cmd(
    state: State<'_, AppState>,
    song: Song,
) -> Result<PlaybackResult, String> {
    state.api.resolve_song_accompaniment(&song).await
}

#[tauri::command]
async fn get_song_detail_info_cmd(
    state: State<'_, AppState>,
    song: Song,
    preferred_quality: Option<String>,
) -> Result<SongDetailInfo, String> {
    state
        .api
        .fetch_song_detail_info_with_quality(&song, preferred_quality.as_deref())
        .await
}

#[tauri::command]
async fn get_cache_stats_cmd(state: State<'_, AppState>) -> Result<CacheDiskStats, String> {
    state.api.get_cache_stats()
}

#[tauri::command]
async fn clear_cache_category_cmd(
    state: State<'_, AppState>,
    category: String,
) -> Result<CacheDiskStats, String> {
    state.api.clear_cache_category(&category)
}

#[tauri::command]
async fn set_desktop_lyric_window_state_cmd(
    app: AppHandle,
    visible: bool,
    locked: bool,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<bool, String> {
    if let Some(win) = app.get_webview_window("desktop_lyric") {
        if let (Some(w), Some(h)) = (width, height) {
            let clamped_w = w.clamp(340.0, 720.0);
            let clamped_h = h.clamp(92.0, 220.0);
            let _ = win.set_size(Size::Logical(LogicalSize::new(clamped_w, clamped_h)));
        }
        let _ = win.set_always_on_top(true);
        let _ = win.set_ignore_cursor_events(locked);
        if visible {
            let _ = win.show();
        } else {
            let _ = win.hide();
        }
        return Ok(true);
    }
    Ok(false)
}

#[tauri::command]
async fn set_close_to_tray_cmd(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    state.close_to_tray.store(enabled, Ordering::Relaxed);
    Ok(enabled)
}

#[tauri::command]
async fn report_playback_telemetry_cmd(
    state: State<'_, AppState>,
    song: Song,
    is_start: bool,
    played_duration_ms: Option<i64>,
) -> Result<bool, String> {
    state
        .api
        .report_playback_telemetry(&song, is_start, played_duration_ms.unwrap_or(0))
        .await
}

#[tauri::command]
async fn get_viper_effects_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<ViperEffectItem>, String> {
    state.api.fetch_viper_effects().await
}

#[tauri::command]
async fn get_song_lyric(state: State<'_, AppState>, song: Song) -> Result<LyricData, String> {
    state.api.fetch_song_lyric(&song).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Arc::new(AppStore::new());
    let api = Arc::new(KugouApiService::new(store.clone()));
    let close_to_tray = Arc::new(AtomicBool::new(true));
    let close_to_tray_for_event = close_to_tray.clone();

    tauri::Builder::default()
        .manage(AppState {
            api,
            store,
            close_to_tray,
        })
        .setup(|app| {
            let show_i = MenuItemBuilder::with_id("show", "显示主窗口").build(app)?;
            let toggle_play_i =
                MenuItemBuilder::with_id("toggle_play", "播放 / 暂停").build(app)?;
            let prev_i = MenuItemBuilder::with_id("prev_song", "上一首").build(app)?;
            let next_i = MenuItemBuilder::with_id("next_song", "下一首").build(app)?;
            let desktop_lyric_i =
                MenuItemBuilder::with_id("toggle_desktop_lyric", "开关桌面歌词").build(app)?;
            let lyric_lock_i =
                MenuItemBuilder::with_id("toggle_lyric_lock", "锁定 / 解锁桌面歌词").build(app)?;
            let quit_i = MenuItemBuilder::with_id("quit", "退出微风音乐").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[
                    &show_i,
                    &toggle_play_i,
                    &prev_i,
                    &next_i,
                    &desktop_lyric_i,
                    &lyric_lock_i,
                    &quit_i,
                ])
                .build()?;

            let mut tray_builder = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .tooltip("BreezeMusic 微风音乐 · 酷狗概念版")
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app_handle = tray.app_handle();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(|app_handle, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "toggle_play" => {
                        let _ = app_handle.emit("tray-action", "toggle_play");
                    }
                    "prev_song" => {
                        let _ = app_handle.emit("tray-action", "prev_song");
                    }
                    "next_song" => {
                        let _ = app_handle.emit("tray-action", "next_song");
                    }
                    "toggle_desktop_lyric" => {
                        let _ = app_handle.emit("tray-action", "toggle_desktop_lyric");
                    }
                    "toggle_lyric_lock" => {
                        let _ = app_handle.emit("tray-action", "toggle_lyric_lock");
                    }
                    "quit" => {
                        app_handle.exit(0);
                    }
                    _ => {}
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _ = tray_builder.build(app);
            Ok(())
        })
        .on_window_event(move |window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    if close_to_tray_for_event.load(Ordering::Relaxed) {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        window.app_handle().exit(0);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            init_app_state,
            get_saved_accounts_cmd,
            switch_saved_account_cmd,
            remove_saved_account_cmd,
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
            get_search_suggest_cmd,
            search_playlists_cmd,
            search_albums_cmd,
            search_artists_cmd,
            get_user_playlists,
            get_playlist_info_cmd,
            get_playlist_songs,
            create_user_playlist_cmd,
            update_user_playlist_cmd,
            delete_user_playlist_cmd,
            collect_playlist_cmd,
            uncollect_playlist_cmd,
            add_song_to_playlist_cmd,
            add_songs_to_playlist_batch_cmd,
            remove_song_from_playlist_cmd,
            resolve_song_album_meta_cmd,
            get_followed_artists,
            toggle_follow_artist_cmd,
            get_purchased_albums,
            get_purchased_songs,
            get_artist_detail_cmd,
            get_artist_songs_cmd,
            get_artist_albums_cmd,
            get_album_detail_cmd,
            get_album_songs_cmd,
            get_song_comments_cmd,
            get_comment_floor_replies_cmd,
            like_song_comment_cmd,
            post_song_comment_cmd,
            resign_device_cmd,
            diagnose_gateway_cmd,
            download_song_cmd,
            parse_shared_music_cmd,
            resolve_song_url,
            resolve_song_accompaniment_cmd,
            get_song_detail_info_cmd,
            get_cache_stats_cmd,
            clear_cache_category_cmd,
            set_desktop_lyric_window_state_cmd,
            set_close_to_tray_cmd,
            report_playback_telemetry_cmd,
            get_viper_effects_cmd,
            get_song_lyric,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BreezeMusic Windows application");
}
