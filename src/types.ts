export type MainNavTab = "discover" | "library" | "search" | "settings";

export type AudioQualityValue = "128" | "320" | "flac" | "high";

/** 全局桌面布局风格（仅在“设置”页中切换，不在主页面散落按钮） */
export type DesktopLayoutMode = "floating_bento" | "zen_minimal" | "mini_turntable";

/** 全局色彩外观主题（仅在“设置”页中切换） */
export type AppColorScheme = "abyssal_dark" | "oled_black" | "watercolor_light";

/** 播放页四款高定舞台样式（在播放页内部专属设置面板切换） */
export type PlayerStageStyle =
  | "vinyl_turntable"
  | "retro_cassette"
  | "cyber_spectrum"
  | "gallery_poster";

/** 播放页五款高定背景（对齐 Android 端 PlayerBackgroundStyle） */
export type PlayerBackgroundStyle =
  | "cover_blur"
  | "fluid_aurora"
  | "rhythm_halo"
  | "starlight_nebula"
  | "deep_oled";

/** 歌词页六款四字流光主题（对齐 Android 端 LyricHighlightTheme 2x3 网格） */
export type LyricThemeId =
  | "ice_blue"
  | "aurora_purple"
  | "amber_gold"
  | "neon_coral"
  | "emerald_mint"
  | "dynamic_cover";

export type LyricAlignment = "left" | "center" | "right";

export type PlayMode = "repeat_all" | "repeat_one" | "shuffle";

export interface KugouSession {
  userId: string;
  token: string;
  t1?: string | null;
  vipToken?: string | null;
  nickname: string;
  avatarUrl: string;
}

export interface AccountProfile {
  userId: string;
  nickname: string;
  avatarUrl: string;
  isVip: boolean;
  svipActive: boolean;
  tvipActive: boolean;
  vipExpireText: string;
  listenDurationMin: number;
  listenDurationSec: number;
  listenDurationFormatted: string;
  userGrade: number;
}

export interface QrCodeSession {
  qrcodeKey: string;
  qrcodeImg: string;
  expiresAtMs: number;
}

export interface QrPollResult {
  status: number;
  message: string;
  session?: KugouSession | null;
  profile?: AccountProfile | null;
}

export interface LoginCandidate {
  userId: string;
  nickname: string;
  avatarUrl: string;
}

export interface PhoneLoginOutcome {
  status: "success" | "choose_account";
  session?: KugouSession | null;
  profile?: AccountProfile | null;
  candidates: LoginCandidate[];
  message: string;
}

export interface VipDayRecord {
  day: string;
  received: boolean;
  vipType: string;
}

export interface VipStatusInfo {
  isVip: boolean;
  svipActive: boolean;
  tvipActive: boolean;
  svipExpiry: string;
  tvipExpiry: string;
  todayClaimed: boolean;
  monthClaimedDays: number;
  records: VipDayRecord[];
  statusMessage: string;
  dfid: string;
  guid: string;
}

export interface Song {
  id: string;
  hash: string;
  hash128: string;
  hash320: string;
  hashFlac: string;
  hashHiRes: string;
  size128: number;
  size320: number;
  sizeFlac: number;
  sizeHiRes: number;
  title: string;
  artist: string;
  artistId: string;
  album: string;
  albumId: string;
  coverUrl: string;
  durationMs: number;
  albumAudioId: number;
  audioId: number;
  mixSongId: string;
  isVip: boolean;
  isLossless: boolean;
  isHiRes: boolean;
  highestQuality: string;
  publishDate: string;
}

export interface PlaybackResult {
  url: string;
  actualQuality: string;
  qualityLabel: string;
  qualityBadge: string;
  bitrateKbps: number;
  fileSizeBytes: number;
  sampleRateHz: number;
  bitDepth: number;
  channels: number;
  techSpecText: string;
  climaxStartMs: number;
  climaxEndMs: number;
  isFreePart: boolean;
  resolvedHash: string;
  extName: string;
}

export interface Playlist {
  id: string;
  title: string;
  coverUrl: string;
  trackCount: number;
  description: string;
  creatorName: string;
  isCreatedByMe: boolean;
  isFavorite: boolean;
  isLiked: boolean;
  listId: number;
  globalCollectionId: string;
}

export interface RankItem {
  rankId: number;
  rankName: string;
  coverUrl: string;
  updateFrequency: string;
  topSongsPreview: string[];
  category: string;
  totalSongs: number;
}

export interface LyricWord {
  startTimeMs: number;
  durationMs: number;
  text: string;
}

export interface LyricLine {
  startTimeMs: number;
  durationMs: number;
  text: string;
  words: LyricWord[];
  translation?: string | null;
  phonetic?: string | null;
}

export interface LyricData {
  lines: LyricLine[];
  rawText: string;
  isKrc: boolean;
  hasTranslation: boolean;
  hasPhonetic: boolean;
}
