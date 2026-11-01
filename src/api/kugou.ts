import { invoke } from "@tauri-apps/api/core";
import type {
  AccountProfile,
  AlbumItem,
  DownloadTaskResult,
  FmChannel,
  FollowedArtist,
  GatewayDiagResult,
  KugouSession,
  LyricData,
  PhoneLoginOutcome,
  PlaybackResult,
  Playlist,
  PurchasedAlbumItem,
  PurchasedSongItem,
  QrCodeSession,
  QrPollResult,
  RankItem,
  Song,
  SongComment,
  VipStatusInfo,
} from "../types";

export const kugouClient = {
  /** 初始化设备 dfid 注册并读取本地已保存的登录会话 */
  initAppState(): Promise<KugouSession | null> {
    return invoke<KugouSession | null>("init_app_state");
  },

  /** 发送手机短信验证码 */
  sendSmsCode(mobile: string): Promise<string> {
    return invoke<string>("send_sms_code", { mobile });
  },

  /** 手机短信验证码登录（支持多账号候选指定 userId） */
  loginWithSms(
    mobile: string,
    code: string,
    userId?: string
  ): Promise<PhoneLoginOutcome> {
    return invoke<PhoneLoginOutcome>("login_with_sms", {
      mobile,
      code,
      userId: userId ?? null,
    });
  },

  /** 获取扫码登录二维码 */
  createQrLogin(): Promise<QrCodeSession> {
    return invoke<QrCodeSession>("create_qr_login");
  },

  /** 轮询扫码登录状态 */
  checkQrLogin(qrcodeKey: string): Promise<QrPollResult> {
    return invoke<QrPollResult>("check_qr_login", { qrcodeKey });
  },

  /** 退出当前登录账号 */
  logoutAccount(): Promise<void> {
    return invoke<void>("logout_account");
  },

  /** 获取当前账号详细资料、听歌等级与权威累计听歌时长 (d_sec) */
  getAccountProfile(): Promise<AccountProfile> {
    return invoke<AccountProfile>("get_account_profile");
  },

  /** 查询酷狗概念版 VIP 状态与当月签到日历记录 */
  getVipStatus(): Promise<VipStatusInfo> {
    return invoke<VipStatusInfo>("get_vip_status");
  },

  /** 一键每日签到领取并升级概念版 VIP */
  claimDailyVip(): Promise<VipStatusInfo> {
    return invoke<VipStatusInfo>("claim_daily_vip");
  },

  /** 获取概念版每日推荐歌曲列表 */
  getDailyRecommend(): Promise<Song[]> {
    return invoke<Song[]>("get_daily_recommend");
  },

  /** 获取概念版「猜你喜欢 · 随心听」私人 FM 推荐曲目 */
  getPersonalFm(): Promise<Song[]> {
    return invoke<Song[]>("get_personal_fm");
  },

  /** 获取新歌速递（CHINESE / WESTERN / JPKR / CANTON / TODAY） */
  getNewSongExpress(periodKey = "CHINESE"): Promise<Song[]> {
    return invoke<Song[]>("get_new_song_express", { periodKey });
  },

  /** 获取巅峰排行榜列表 */
  getRankList(): Promise<RankItem[]> {
    return invoke<RankItem[]>("get_rank_list");
  },

  /** 获取指定排行榜曲目列表 */
  getRankSongs(rankId: number, page = 1, pageSize = 50): Promise<Song[]> {
    return invoke<Song[]>("get_rank_songs", { rankId, page, pageSize });
  },

  /** 获取精选场景电台频道列表 */
  getFmChannels(): Promise<FmChannel[]> {
    return invoke<FmChannel[]>("get_fm_channels");
  },

  /** 获取指定电台频道的漫游曲目流 */
  getFmSongs(channelId: string): Promise<Song[]> {
    return invoke<Song[]>("get_fm_songs", { channelId });
  },

  /** 获取云端最近播放记录 */
  getRecentHistory(): Promise<Song[]> {
    return invoke<Song[]>("get_recent_history");
  },

  /** 搜索单曲 */
  searchSongs(keyword: string, page = 1, pageSize = 30): Promise<Song[]> {
    return invoke<Song[]>("search_songs", { keyword, page, pageSize });
  },

  /** 获取酷狗实时热搜榜关键词 */
  getSearchHot(): Promise<string[]> {
    return invoke<string[]>("get_search_hot");
  },

  /** 搜索歌单 */
  searchPlaylists(keyword: string): Promise<Playlist[]> {
    return invoke<Playlist[]>("search_playlists_cmd", { keyword });
  },

  /** 搜索专辑 */
  searchAlbums(keyword: string): Promise<AlbumItem[]> {
    return invoke<AlbumItem[]>("search_albums_cmd", { keyword });
  },

  /** 搜索歌手 */
  searchArtists(keyword: string): Promise<FollowedArtist[]> {
    return invoke<FollowedArtist[]>("search_artists_cmd", { keyword });
  },

  /** 获取用户自建与收藏歌单列表 */
  getUserPlaylists(): Promise<Playlist[]> {
    return invoke<Playlist[]>("get_user_playlists");
  },

  /** 获取指定歌单内的全部歌曲 */
  getPlaylistSongs(collectionId: string): Promise<Song[]> {
    return invoke<Song[]>("get_playlist_songs", { collectionId });
  },

  /** 新建云端自定义歌单 */
  createUserPlaylist(name: string): Promise<boolean> {
    return invoke<boolean>("create_user_playlist_cmd", { name });
  },

  /** 删除云端歌单 */
  deleteUserPlaylist(listId: number): Promise<boolean> {
    return invoke<boolean>("delete_user_playlist_cmd", { listId });
  },

  /** 添加歌曲到指定歌单 */
  addSongToPlaylist(listId: number, song: Song): Promise<boolean> {
    return invoke<boolean>("add_song_to_playlist_cmd", { listId, song });
  },

  /** 获取已关注歌手列表 */
  getFollowedArtists(): Promise<FollowedArtist[]> {
    return invoke<FollowedArtist[]>("get_followed_artists");
  },

  /** 关注或取消关注歌手 */
  toggleFollowArtist(artistId: string, follow: boolean): Promise<boolean> {
    return invoke<boolean>("toggle_follow_artist_cmd", { artistId, follow });
  },

  /** 获取已购数字专辑列表 */
  getPurchasedAlbums(): Promise<PurchasedAlbumItem[]> {
    return invoke<PurchasedAlbumItem[]>("get_purchased_albums");
  },

  /** 获取已购单曲列表 */
  getPurchasedSongs(): Promise<PurchasedSongItem[]> {
    return invoke<PurchasedSongItem[]>("get_purchased_songs");
  },

  /** 获取歌手详情 */
  getArtistDetail(artistId: string): Promise<FollowedArtist> {
    return invoke<FollowedArtist>("get_artist_detail_cmd", { artistId });
  },

  /** 获取歌手热门或最新单曲 */
  getArtistSongs(
    artistId: string,
    sort: "hot" | "new" = "hot",
    page = 1
  ): Promise<Song[]> {
    return invoke<Song[]>("get_artist_songs_cmd", { artistId, sort, page });
  },

  /** 获取专辑内全部曲目 */
  getAlbumSongs(albumId: string): Promise<Song[]> {
    return invoke<Song[]>("get_album_songs_cmd", { albumId });
  },

  /** 获取歌曲评论列表（hot / new） */
  getSongComments(
    song: Song,
    sortType: "hot" | "new" = "hot",
    page = 1
  ): Promise<SongComment[]> {
    return invoke<SongComment[]>("get_song_comments_cmd", {
      song,
      sortType,
      page,
    });
  },

  /** 重新签发概念版 24 位 DFID 设备标识 */
  resignDeviceIdentity(): Promise<GatewayDiagResult> {
    return invoke<GatewayDiagResult>("resign_device_cmd");
  },

  /** 音源网关链路延迟诊断 */
  diagnoseGatewayLatency(): Promise<GatewayDiagResult> {
    return invoke<GatewayDiagResult>("diagnose_gateway_cmd");
  },

  /** 下载歌曲与内嵌元数据/导出独立 .lrc 文件 */
  downloadSong(
    song: Song,
    preferredQuality = "high",
    musicDir = "",
    lrcDir = "",
    exportLrc = true
  ): Promise<DownloadTaskResult> {
    return invoke<DownloadTaskResult>("download_song_cmd", {
      song,
      preferredQuality,
      musicDir,
      lrcDir,
      exportLrc,
    });
  },

  /** 解析歌曲真实播放音源 URL + 硬件级 STREAMINFO 采样率/位深探测 */
  resolveSongUrl(
    song: Song,
    preferredQuality = "flac"
  ): Promise<PlaybackResult> {
    return invoke<PlaybackResult>("resolve_song_url", {
      song,
      preferredQuality,
    });
  },

  /** 获取并解密 KRC/LRC 歌词 */
  getSongLyric(song: Song): Promise<LyricData> {
    return invoke<LyricData>("get_song_lyric", { song });
  },
};
