export type MainNavTab = "discover" | "library" | "radio" | "search" | "settings";

export type DiscoverSubView = "home" | "daily" | "new_songs" | "rank" | "radio";

export type LibraryAssetTab =
  | "created"
  | "collected"
  | "followed"
  | "purchased"
  | "liked"
  | "recent";

export type LibraryDetailMode = "playlist" | "artist" | "album";

export type SearchCategoryTab = "songs" | "playlists" | "albums" | "artists";

export type AudioQualityValue = "128" | "320" | "flac" | "high";

/** 全局桌面布局风格（方案二默认：skyline_bento 顶部天际线中枢画廊） */
export type DesktopLayoutMode =
  | "floating_bento"
  | "zen_minimal"
  | "mini_turntable";

/** Windows 窗口背景材质 */
export type WindowMaterialStyle = "mica_glow" | "acrylic_frost" | "pure_solid";

/** 全局色彩外观主题 */
export type AppColorScheme = "abyssal_dark" | "oled_black" | "watercolor_light";

/** 播放页四款高定舞台样式 */
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

export interface FmChannel {
  id: string;
  name: string;
  coverUrl: string;
  description: string;
  category: string;
  fmType: number;
  listenerCountText: string;
  freqLabel: string;
}

export interface FollowedArtist {
  artistId: string;
  targetUserId: string;
  name: string;
  avatarUrl: string;
  songCount: number;
  albumCount: number;
  fansCount: number;
  isFollowed: boolean;
  intro: string;
}

export interface AlbumItem {
  albumId: string;
  albumName: string;
  artistName: string;
  artistId: string;
  coverUrl: string;
  publishDate: string;
  publishCompany: string;
  songCount: number;
  intro: string;
}

export interface PurchasedAlbumItem {
  albumId: string;
  albumName: string;
  artistName: string;
  coverUrl: string;
  songCount: number;
  buyTime: string;
  publishDate: string;
}

export interface PurchasedSongItem {
  song: Song;
  buyTime: string;
}

export interface SongComment {
  id: string;
  userName: string;
  userAvatar: string;
  content: string;
  createdTime: string;
  likeCount: number;
  isLiked: boolean;
  location: string;
  replyCount: number;
  specialChildId: string;
  replies: SongComment[];
}

export interface GatewayDiagResult {
  latencyMs: number;
  dfid: string;
  mid: string;
  guid: string;
  statusText: string;
  ok: boolean;
}

export interface DownloadTaskResult {
  success: boolean;
  savedAudioPath: string;
  savedLrcPath: string;
  fileSizeBytes: number;
  qualityLabel: string;
  message: string;
}

export interface DownloadHistoryRecord {
  id: string;
  title: string;
  artist: string;
  qualityLabel: string;
  savedPath: string;
  fileSizeText: string;
  timestampText: string;
}

/** 母带级声场调音工作台参数（对齐 Android AcousticFieldTuningTab） */
export interface AcousticFieldSettings {
  enabled: boolean;
  subBassDepth: number; // 0 ~ 100 超重低音下潜
  bassCrossoverHz: number; // 40 / 55 / 68 / 80 / 95 / 110 Hz
  transientPunch: number; // 0 ~ 100 瞬态微动态与鼓点冲击
  surroundWidth: number; // 0 ~ 100 3D 环绕声场宽度
  stageDepth: number; // 0 ~ 100 舞台纵深距离感
  bauerCrossfeed: boolean; // Bauer 耳机交叉馈送（消除头中效应）
  reverbPreDelayMs: number; // 0 ~ 80 ms 空间混响预延迟
  vocalClarity: number; // 0 ~ 100 人声通透与 6.2kHz 齿音柔化
  airHarmonics: number; // 0 ~ 100 12kHz+ 极高频空气泛音
  vinylTubeWarmth: number; // 0 ~ 100 黑胶胆机温暖度
}

/** 10段参量均衡器自定义记忆槽 A / B / C */
export interface PeqMemorySlot {
  slotId: "A" | "B" | "C";
  name: string;
  saved: boolean;
  gains: number[];
  qFactor: number;
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
