import { invoke } from "@tauri-apps/api/core";
import type {
  AccountProfile,
  KugouSession,
  LyricData,
  PhoneLoginOutcome,
  PlaybackResult,
  Playlist,
  QrCodeSession,
  QrPollResult,
  RankItem,
  Song,
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

  /** 获取云端最近播放记录 */
  getRecentHistory(): Promise<Song[]> {
    return invoke<Song[]>("get_recent_history");
  },

  /** 搜索歌曲 */
  searchSongs(keyword: string, page = 1, pageSize = 30): Promise<Song[]> {
    return invoke<Song[]>("search_songs", { keyword, page, pageSize });
  },

  /** 获取用户自建与收藏歌单列表 */
  getUserPlaylists(): Promise<Playlist[]> {
    return invoke<Playlist[]>("get_user_playlists");
  },

  /** 获取指定歌单内的全部歌曲 */
  getPlaylistSongs(collectionId: string): Promise<Song[]> {
    return invoke<Song[]>("get_playlist_songs", { collectionId });
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
