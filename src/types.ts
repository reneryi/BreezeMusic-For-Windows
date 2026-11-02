export type NavTab = "discover" | "radio" | "library" | "settings" | "search";

export type DiscoverSubView = "home" | "daily" | "fm_detail" | "new_songs" | "rank" | "radio";

export type PlayMode = "sequence" | "loop_all" | "loop_one" | "shuffle";

export type AudioQuality = "128" | "320" | "flac" | "hires" | "master";

export type ThemePresetId =
  | "breeze_light"
  | "sakura_mist"
  | "matcha_latte"
  | "sunset_amber"
  | "ocean_glass"
  | "obsidian_dark"
  | "aurora_cyber";

export type SoundEffectPreset =
  | "none"
  | "viper_clear"
  | "viper_bass"
  | "viper_3d"
  | "viper_vinyl"
  | "concert_hall"
  | "vocal_warm";

export interface Song {
  id: string;
  hash: string;
  title: string;
  artist: string;
  album: string;
  albumId: string;
  albumAudioId: string;
  durationSeconds: number;
  coverUrl: string;
  mvHash: string;
  hasHighQuality: boolean;
  hasSq: boolean;
  hasHiRes: boolean;
  isVip: boolean;
  privilege: number;
  payType: number;
  failProcess: number;
  climaxStartMs: number;
  climaxEndMs: number;
  publishDate: string;
}

export interface PlaylistItem {
  id: string;
  globalCollectionId: string;
  listId: number;
  listCreateUserid: number;
  listCreateGid: string;
  name: string;
  coverUrl: string;
  trackCount: number;
  creatorName: string;
  description: string;
  playCount: number;
  category: string;
  tags: string;
  customOrder: number;
  source: number;
}

export interface RankItem {
  rankId: number;
  name: string;
  coverUrl: string;
  updateFrequency: string;
  category: string;
  songPreviews: string[];
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

export interface FollowedArtistItem {
  singerId: number;
  name: string;
  avatarUrl: string;
  albumCount: number;
  mvCount: number;
  fansCount: number;
}

export interface ArtistDetailInfo {
  singerId: number;
  name: string;
  avatarUrl: string;
  intro: string;
  songCount: number;
  albumCount: number;
  mvCount: number;
  fansCount: number;
}

export interface PurchasedAlbumItem {
  id: string;
  albumName: string;
  singerName: string;
  coverUrl: string;
  buyTime: string;
  songCount: number;
}

export interface LyricWordToken {
  startMs: number;
  durationMs: number;
  text: string;
}

export interface LyricLine {
  timeMs: number;
  durationMs: number;
  text: string;
  translation: string;
  romanization: string;
  words: LyricWordToken[];
}

export interface LyricResult {
  rawText: string;
  isVerbatimKrc: boolean;
  sourceCandidateId: string;
  lines: LyricLine[];
}

export interface LyricCandidate {
  id: string;
  accessKey: string;
  songName: string;
  singer: string;
  durationMs: number;
  score: number;
  fmt: string;
}

export interface PlaybackResolution {
  songHash: string;
  url: string;
  backupUrls: string[];
  extName: string;
  bitrate: number;
  fileSize: number;
  matchedQuality: string;
  climaxStartMs: number;
  climaxEndMs: number;
  diagnosticNote: string;
}

export interface AccountSession {
  userId: string;
  token: string;
  nickname: string;
  avatarUrl: string;
  isVip: boolean;
  vipType: number;
  vipBeginTime: string;
  vipEndTime: string;
  svipLevel: number;
  svipScore: number;
  listenDurationMinutes: number;
  userLevel: number;
  loginTime: number;
}

export interface VipClaimStatus {
  dayVipReceived: boolean;
  dayVipClaimable: boolean;
  upgradeVipReceived: boolean;
  monthDoneDays: number;
  monthTargetDays: number;
  monthRemainingDays: number;
  vipEndTime: string;
  svipLevel: number;
  svipScore: number;
  statusNote: string;
}

export interface BootstrapPayload {
  deviceMid: string;
  deviceUuid: string;
  deviceDfid: string;
  session: AccountSession | null;
  vipStatus: VipClaimStatus;
  dailySongs: Song[];
  personalFmSongs: Song[];
}

export interface PlayHistoryEntry {
  song: Song;
  playedAtMs: number;
  playCount: number;
}
