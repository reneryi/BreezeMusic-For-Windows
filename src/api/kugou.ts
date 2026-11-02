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
  SongDetailInfo,
  VipStatusInfo,
  ViperEffectItem,
} from "../types";

function normSong(s: Song): Song {
  return {
    ...s,
    durationSeconds: s.durationSeconds ?? Math.floor((s.durationMs || 0) / 1000),
    hasHiRes: s.hasHiRes ?? Boolean(s.isHiRes || s.hashHiRes),
    hasSq: s.hasSq ?? Boolean(s.isLossless || s.hashFlac),
    hasHighQuality: s.hasHighQuality ?? Boolean(s.hash320 || s.size320 > 0),
    climaxStartMs: s.climaxStartMs ?? 0,
    climaxEndMs: s.climaxEndMs ?? 0,
  };
}

function normSongs(list: Song[]): Song[] {
  return (list || []).map(normSong);
}

function normPlaylist(p: Playlist): Playlist {
  return {
    ...p,
    name: p.name || p.title,
    title: p.title || p.name || "",
  };
}

function normRank(r: RankItem): RankItem {
  return {
    ...r,
    name: r.name || r.rankName,
    rankName: r.rankName || r.name || "",
    songPreviews: r.songPreviews || r.topSongsPreview || [],
    topSongsPreview: r.topSongsPreview || r.songPreviews || [],
  };
}

function normArtist(a: FollowedArtist): FollowedArtist {
  return {
    ...a,
    singerId: a.singerId ?? (Number(a.artistId) || 0),
  };
}

function normPurchasedAlbum(a: PurchasedAlbumItem): PurchasedAlbumItem {
  return {
    ...a,
    id: a.id || a.albumId,
    singerName: a.singerName || a.artistName,
  };
}

function normProfile(p: AccountProfile): AccountProfile {
  return {
    ...p,
    userLevel: p.userLevel ?? p.userGrade,
    listenDurationMinutes: p.listenDurationMinutes ?? p.listenDurationMin,
  };
}

function normVipStatus(v: VipStatusInfo): VipStatusInfo {
  return {
    ...v,
    dayVipClaimed: v.dayVipClaimed ?? v.todayClaimed,
  };
}

export const kugouClient = {
  /** 初始化设备 dfid 注册并读取本地已保存的登录会话 */
  initAppState(): Promise<KugouSession | null> {
    return invoke<KugouSession | null>("init_app_state");
  },

  /** 获取本地已保存的多账号列表 */
  getSavedAccounts(): Promise<KugouSession[]> {
    return invoke<KugouSession[]>("get_saved_accounts_cmd");
  },

  /** 一键切换至已保存的酷狗账号 */
  switchSavedAccount(userId: string): Promise<KugouSession | null> {
    return invoke<KugouSession | null>("switch_saved_account_cmd", { userId });
  },

  /** 移除已保存的账号会话 */
  removeSavedAccount(userId: string): Promise<KugouSession[]> {
    return invoke<KugouSession[]>("remove_saved_account_cmd", { userId });
  },

  /** 发送手机短信验证码 */
  sendSmsCode(mobile: string): Promise<string> {
    return invoke<string>("send_sms_code", { mobile });
  },

  /** 手机短信验证码登录（支持多账号候选指定 userId） */
  async loginWithSms(
    mobile: string,
    code: string,
    userId?: string
  ): Promise<PhoneLoginOutcome> {
    const res = await invoke<PhoneLoginOutcome>("login_with_sms", {
      mobile,
      code,
      userId: userId ?? null,
    });
    if (res.profile) res.profile = normProfile(res.profile);
    return res;
  },

  /** 获取扫码登录二维码 */
  createQrLogin(): Promise<QrCodeSession> {
    return invoke<QrCodeSession>("create_qr_login");
  },

  /** 轮询扫码登录状态 */
  async checkQrLogin(qrcodeKey: string): Promise<QrPollResult> {
    const res = await invoke<QrPollResult>("check_qr_login", { qrcodeKey });
    if (res.profile) res.profile = normProfile(res.profile);
    return res;
  },

  /** 退出当前登录账号 */
  logoutAccount(): Promise<void> {
    return invoke<void>("logout_account");
  },

  /** 获取当前账号详细资料、听歌等级与权威累计听歌时长 (d_sec) */
  async getAccountProfile(): Promise<AccountProfile> {
    const p = await invoke<AccountProfile>("get_account_profile");
    return normProfile(p);
  },

  /** 查询酷狗概念版 VIP 状态与当月签到日历记录 */
  async getVipStatus(): Promise<VipStatusInfo> {
    const v = await invoke<VipStatusInfo>("get_vip_status");
    return normVipStatus(v);
  },

  /** 一键每日签到领取并升级概念版 VIP */
  async claimDailyVip(): Promise<VipStatusInfo> {
    const v = await invoke<VipStatusInfo>("claim_daily_vip");
    return normVipStatus(v);
  },

  /** 获取概念版每日推荐歌曲列表 */
  async getDailyRecommend(): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("get_daily_recommend"));
  },

  /** 获取概念版「猜你喜欢 · 随心听」私人 FM 推荐曲目 */
  async getPersonalFm(): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("get_personal_fm"));
  },

  /** 获取新歌速递（CHINESE / WESTERN / JPKR / CANTON / TODAY） */
  async getNewSongExpress(periodKey = "CHINESE"): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("get_new_song_express", { periodKey }));
  },

  /** 获取巅峰排行榜列表 */
  async getRankList(): Promise<RankItem[]> {
    const list = await invoke<RankItem[]>("get_rank_list");
    return (list || []).map(normRank);
  },

  /** 获取指定排行榜曲目列表 */
  async getRankSongs(rankId: number, page = 1, pageSize = 50): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("get_rank_songs", { rankId, page, pageSize }));
  },

  /** 获取精选场景电台频道列表 */
  getFmChannels(): Promise<FmChannel[]> {
    return invoke<FmChannel[]>("get_fm_channels");
  },

  /** 获取指定电台频道的漫游曲目流 */
  async getFmSongs(channelId: string): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("get_fm_songs", { channelId }));
  },

  /** 获取云端最近播放记录 */
  async getRecentHistory(): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("get_recent_history"));
  },

  /** 搜索单曲 */
  async searchSongs(keyword: string, page = 1, pageSize = 30): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("search_songs", { keyword, page, pageSize }));
  },

  /** 获取酷狗实时热搜榜关键词 */
  getSearchHot(): Promise<string[]> {
    return invoke<string[]>("get_search_hot");
  },

  /** 获取实时搜索联想建议词 (/v2/getSearchTip) */
  getSearchSuggest(keyword: string): Promise<string[]> {
    return invoke<string[]>("get_search_suggest_cmd", { keyword });
  },

  /** 搜索歌单 */
  async searchPlaylists(keyword: string): Promise<Playlist[]> {
    const list = await invoke<Playlist[]>("search_playlists_cmd", { keyword });
    return (list || []).map(normPlaylist);
  },

  /** 搜索专辑 */
  searchAlbums(keyword: string): Promise<AlbumItem[]> {
    return invoke<AlbumItem[]>("search_albums_cmd", { keyword });
  },

  /** 搜索歌手 */
  async searchArtists(keyword: string): Promise<FollowedArtist[]> {
    const list = await invoke<FollowedArtist[]>("search_artists_cmd", { keyword });
    return (list || []).map(normArtist);
  },

  /** 获取用户自建与收藏歌单列表 */
  async getUserPlaylists(): Promise<Playlist[]> {
    const list = await invoke<Playlist[]>("get_user_playlists");
    return (list || []).map(normPlaylist);
  },

  /** 获取指定歌单内的全部歌曲（支持多页自动聚合） */
  async getPlaylistSongs(collectionId: string): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("get_playlist_songs", { collectionId }));
  },

  /** 新建云端自定义歌单 */
  createUserPlaylist(name: string): Promise<boolean> {
    return invoke<boolean>("create_user_playlist_cmd", { name });
  },

  /** 删除云端歌单 */
  deleteUserPlaylist(listId: number): Promise<boolean> {
    return invoke<boolean>("delete_user_playlist_cmd", { listId });
  },

  /** 收藏外部歌单至云端 */
  collectPlaylist(playlist: Playlist): Promise<boolean> {
    return invoke<boolean>("collect_playlist_cmd", { playlist });
  },

  /** 取消收藏云端歌单 */
  uncollectPlaylist(playlist: Playlist): Promise<boolean> {
    return invoke<boolean>("uncollect_playlist_cmd", { playlist });
  },

  /** 添加歌曲到指定歌单 */
  addSongToPlaylist(listId: number, song: Song): Promise<boolean> {
    return invoke<boolean>("add_song_to_playlist_cmd", { listId, song });
  },

  /** 从自建歌单中移除单曲 */
  removeSongFromPlaylist(listId: number, song: Song): Promise<boolean> {
    return invoke<boolean>("remove_song_from_playlist_cmd", { listId, song });
  },

  /** 获取已关注歌手列表 */
  async getFollowedArtists(): Promise<FollowedArtist[]> {
    const list = await invoke<FollowedArtist[]>("get_followed_artists");
    return (list || []).map(normArtist);
  },

  /** 关注或取消关注歌手 */
  toggleFollowArtist(artistId: string, follow: boolean): Promise<boolean> {
    return invoke<boolean>("toggle_follow_artist_cmd", { artistId, follow });
  },

  /** 获取已购数字专辑列表 */
  async getPurchasedAlbums(): Promise<PurchasedAlbumItem[]> {
    const list = await invoke<PurchasedAlbumItem[]>("get_purchased_albums");
    return (list || []).map(normPurchasedAlbum);
  },

  /** 获取已购单曲列表 */
  async getPurchasedSongs(): Promise<PurchasedSongItem[]> {
    const list = await invoke<PurchasedSongItem[]>("get_purchased_songs");
    return (list || []).map((item) => ({
      ...item,
      song: normSong(item.song),
    }));
  },

  /** 获取歌手详情 */
  async getArtistDetail(artistId: string): Promise<FollowedArtist> {
    return normArtist(await invoke<FollowedArtist>("get_artist_detail_cmd", { artistId }));
  },

  /** 获取歌手热门或最新单曲 */
  async getArtistSongs(
    artistId: string,
    sort: "hot" | "new" = "hot",
    page = 1
  ): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("get_artist_songs_cmd", { artistId, sort, page }));
  },

  /** 获取歌手专辑列表 */
  getArtistAlbums(
    artistId: string,
    page = 1,
    sort: "new" | "hot" = "new"
  ): Promise<AlbumItem[]> {
    return invoke<AlbumItem[]>("get_artist_albums_cmd", {
      artistId,
      page,
      sort,
    });
  },

  /** 获取专辑完整详情（含发行公司、发行时间、长简介） */
  getAlbumDetail(albumId: string): Promise<AlbumItem> {
    return invoke<AlbumItem>("get_album_detail_cmd", { albumId });
  },

  /** 获取专辑内全部曲目 */
  async getAlbumSongs(albumId: string): Promise<Song[]> {
    return normSongs(await invoke<Song[]>("get_album_songs_cmd", { albumId }));
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

  /** 获取评论二层楼中楼回复 */
  getCommentFloorReplies(
    song: Song,
    commentId: string,
    specialChildId = "",
    page = 1
  ): Promise<SongComment[]> {
    return invoke<SongComment[]>("get_comment_floor_replies_cmd", {
      song,
      commentId,
      specialChildId,
      page,
    });
  },

  /** 点赞或取消点赞评论 */
  likeSongComment(
    song: Song,
    commentId: string,
    like: boolean
  ): Promise<boolean> {
    return invoke<boolean>("like_song_comment_cmd", {
      song,
      commentId,
      like,
    });
  },

  /** 发表歌曲评论或回复指定楼层 */
  postSongComment(
    song: Song,
    content: string,
    replyToId?: string
  ): Promise<SongComment> {
    return invoke<SongComment>("post_song_comment_cmd", {
      song,
      content,
      replyToId: replyToId ?? null,
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

  /** 下载歌曲与二进制内嵌元数据/1000P封面/导出独立 .lrc 文件 */
  downloadSong(
    song: Song,
    preferredQuality = "high",
    musicDir = "",
    lrcDir = "",
    exportLrc = true,
    embedCoverAndTags = true,
    embedLyricsInAudio = true,
    bilingualLrc = true
  ): Promise<DownloadTaskResult> {
    return invoke<DownloadTaskResult>("download_song_cmd", {
      song,
      preferredQuality,
      musicDir,
      lrcDir,
      exportLrc,
      embedCoverAndTags,
      embedLyricsInAudio,
      bilingualLrc,
    });
  },

  /** 解析歌曲真实播放音源 URL + 硬件级 STREAMINFO 采样率/位深探测 */
  async resolveSongUrl(
    song: Song,
    preferredQuality = "flac"
  ): Promise<PlaybackResult> {
    const res = await invoke<PlaybackResult>("resolve_song_url", {
      song,
      preferredQuality,
    });
    return {
      ...res,
      matchedQuality: res.matchedQuality || res.actualQuality,
    };
  },

  /** 解析歌曲 KTV 伴奏音轨 URL */
  async resolveSongAccompaniment(song: Song): Promise<PlaybackResult> {
    const res = await invoke<PlaybackResult>("resolve_song_accompaniment_cmd", { song });
    return {
      ...res,
      matchedQuality: res.matchedQuality || res.actualQuality,
    };
  },

  /** 获取单曲完整技术参数与唱片公司详情 */
  getSongDetailInfo(song: Song): Promise<SongDetailInfo> {
    return invoke<SongDetailInfo>("get_song_detail_info_cmd", { song });
  },

  /** CSCC 三段式真实听歌时长与云端历史遥测上报 */
  reportPlaybackTelemetry(
    song: Song,
    isStart: boolean,
    playedDurationMs = 0
  ): Promise<boolean> {
    return invoke<boolean>("report_playback_telemetry_cmd", {
      song,
      isStart,
      playedDurationMs,
    });
  },

  /** 获取酷狗蝰蛇音效官方声学预设矩阵 */
  async getViperEffects(): Promise<ViperEffectItem[]> {
    const list = await invoke<ViperEffectItem[]>("get_viper_effects_cmd");
    return (list || []).map((v) => ({
      ...v,
      name: v.name || v.label,
      badge: v.badge || v.usageBadge,
    }));
  },

  /** 获取并解密 KRC/LRC 歌词 */
  getSongLyric(song: Song): Promise<LyricData> {
    return invoke<LyricData>("get_song_lyric", { song });
  },
};

/** 格式化时长（兼容秒数或毫秒数） */
export function formatDuration(val?: number | null): string {
  if (!val || val <= 0 || !Number.isFinite(val)) return "00:00";
  const totalSec = val > 10000 ? Math.floor(val / 1000) : Math.floor(val);
  const mins = Math.floor(totalSec / 60);
  const secs = totalSec % 60;
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}
