import { defineStore } from "pinia";
import { ref, computed, watch, reactive } from "vue";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { kugouClient } from "../api/kugou";
import type {
  AccountProfile,
  AcousticFieldSettings,
  AlbumItem,
  AppColorScheme,
  AudioQualityValue,
  DesktopLayoutMode,
  DiscoverSubView,
  DownloadHistoryRecord,
  FmChannel,
  FollowedArtist,
  GatewayDiagResult,
  KugouSession,
  LibraryAssetTab,
  LibraryDetailMode,
  LyricAlignment,
  LyricData,
  LyricThemeId,
  MainNavTab,
  PeqFilterType,
  PeqMemorySlot,
  PlaybackResult,
  PlayerBackgroundStyle,
  PlayerStageStyle,
  PlayMode,
  Playlist,
  PurchasedAlbumItem,
  PurchasedSongItem,
  RankItem,
  SearchCategoryTab,
  Song,
  SongComment,
  SongDetailInfo,
  VipStatusInfo,
  ViperEffectItem,
  WindowMaterialStyle,
} from "../types";

export type MainTab = MainNavTab;

export const useAppStore = defineStore("breezeApp", () => {
  // ================= 1. 顶部天际线中枢导航与全局浮层状态 =================
  const activeTab = ref<MainNavTab>("discover");
  const isSearchPaletteOpen = ref(false);
  const isNowPlayingOpen = ref(false);
  const isLoginModalOpen = ref(false);
  const isVipCenterOpen = ref(false);
  const isFocusPodOpen = ref(false);
  const isLyricPosterOpen = ref(false);
  const statusBanner = ref<string>("");

  // 桌面悬浮穿透歌词栏
  const desktopLyricEnabled = ref<boolean>(
    localStorage.getItem("bm_desktop_lyric") === "true"
  );
  const desktopLyricLocked = ref<boolean>(false);

  // 全局右键上下文菜单与歌曲详情/加歌单弹窗
  const contextMenu = reactive<{
    visible: boolean;
    x: number;
    y: number;
    song: Song | null;
  }>({
    visible: false,
    x: 0,
    y: 0,
    song: null,
  });
  const songDetailModalSong = ref<Song | null>(null);
  const activeSongDetailInfo = ref<SongDetailInfo | null>(null);
  const isLoadingSongDetail = ref<boolean>(false);
  const addToPlaylistModalSong = ref<Song | null>(null);
  const downloadModalSong = ref<Song | null>(null);

  // ================= 2. 发现页与调频台全量状态 =================
  const discoverSubView = ref<DiscoverSubView>("home");
  const discoverHeroExpanding = ref<"none" | "daily" | "fm">("none");
  const isGlobalQueueOpen = ref(false);

  const recommendSongs = ref<Song[]>([]);
  const isLoadingRecommend = ref(false);

  const personalFmSongs = ref<Song[]>([]);
  const personalFmIndex = ref(0);
  const isLoadingFm = ref(false);
  const isPersonalFmDrawerOpen = ref(false);

  const currentFmSong = computed<Song | null>(
    () => personalFmSongs.value[personalFmIndex.value] || null
  );
  const nextFmSong = computed<Song | null>(
    () =>
      personalFmSongs.value[
        (personalFmIndex.value + 1) % Math.max(1, personalFmSongs.value.length)
      ] || null
  );

  const newSongPeriod = ref<string>("CHINESE");
  const newSongs = ref<Song[]>([]);
  const isLoadingNewSongs = ref(false);

  const rankList = ref<RankItem[]>([]);
  const isLoadingRanks = ref(false);
  const isRankGridModalOpen = ref(false);
  const selectedRankCategory = ref<string>("全部");
  const activeRank = ref<RankItem | null>(null);
  const activeRankSongs = ref<Song[]>([]);
  const isLoadingRankSongs = ref(false);
  const rankSearchKeyword = ref<string>("");

  const fmChannels = ref<FmChannel[]>([]);
  const selectedFmCategory = ref<string>("全部");
  const activeFmChannel = ref<FmChannel | null>(null);
  const fmStationSongs = ref<Song[]>([]);
  const isLoadingFmStation = ref(false);

  // ================= 3. 我的音乐库（我喜欢/最近/自建/收藏/关注/已购） =================
  const userPlaylists = ref<Playlist[]>([]);
  const selectedPlaylist = ref<Playlist | null>(null);
  const playlistSongs = ref<Song[]>([]);
  const likedSongs = ref<Song[]>([]);
  const recentHistorySongs = ref<Song[]>([]);
  const followedArtists = ref<FollowedArtist[]>([]);
  const purchasedAlbums = ref<PurchasedAlbumItem[]>([]);
  const purchasedSongs = ref<PurchasedSongItem[]>([]);
  const collectedRemotePlaylists = ref<Playlist[]>([]);

  const libraryAssetTab = ref<LibraryAssetTab>("created");
  const libraryDetailMode = ref<LibraryDetailMode>("playlist");
  const isLibraryGridView = ref<boolean>(
    localStorage.getItem("bm_lib_grid") !== "false"
  );
  const isLoadingLibrary = ref(false);

  // 歌手详情与专辑详情内嵌工作台状态
  const activeArtistDetail = ref<FollowedArtist | null>(null);
  const activeArtistSongs = ref<Song[]>([]);
  const activeArtistAlbums = ref<AlbumItem[]>([]);
  const artistDetailSubTab = ref<"songs" | "albums" | "about">("songs");
  const artistSongsSort = ref<"hot" | "new">("hot");
  const isLoadingArtist = ref(false);

  const activeAlbumDetail = ref<AlbumItem | null>(null);
  const activeAlbumSongs = ref<Song[]>([]);
  const isLoadingAlbum = ref(false);

  const createdPlaylists = computed(() =>
    userPlaylists.value.filter((p) => p.isCreatedByMe && !p.isLiked)
  );
  const collectedPlaylists = computed(() => {
    const cloud = userPlaylists.value.filter(
      (p) => p.isFavorite && !p.isCreatedByMe
    );
    const merged = [...cloud];
    for (const rp of collectedRemotePlaylists.value) {
      if (!merged.some((m) => m.id === rp.id || m.title === rp.title)) {
        merged.push(rp);
      }
    }
    return merged;
  });
  const likedPlaylistMeta = computed(() =>
    userPlaylists.value.find((p) => p.isLiked || p.title === "我喜欢")
  );

  // ================= 4. 搜索与天际线指令舱状态 =================
  const searchKeyword = ref<string>("");
  const searchSuggestions = ref<string[]>([]);
  const searchCategoryTab = ref<SearchCategoryTab>("songs");
  const hotSearchKeywords = ref<string[]>([
    "周杰伦",
    "林俊杰",
    "陈奕迅",
    "邓紫棋",
    "告五人",
    "陶喆",
    "王菲",
    "张敬轩",
    "方大同",
    "YOASOBI",
  ]);
  const searchHistory = ref<string[]>(
    JSON.parse(localStorage.getItem("bm_search_history") || "[]")
  );
  const searchResultsSongs = ref<Song[]>([]);
  const searchResultsPlaylists = ref<Playlist[]>([]);
  const searchResultsAlbums = ref<AlbumItem[]>([]);
  const searchResultsArtists = ref<FollowedArtist[]>([]);
  const isSearching = ref(false);

  let suggestDebounceTimer: number | null = null;
  watch(searchKeyword, (kw) => {
    if (suggestDebounceTimer) clearTimeout(suggestDebounceTimer);
    const trimmed = kw.trim();
    if (!trimmed) {
      searchSuggestions.value = [];
      return;
    }
    suggestDebounceTimer = window.setTimeout(async () => {
      try {
        searchSuggestions.value = await kugouClient.getSearchSuggest(trimmed);
      } catch {
        searchSuggestions.value = [];
      }
    }, 240);
  });

  // ================= 5. 全局设置与播放/歌词/DSP 配置 =================
  const layoutMode = ref<DesktopLayoutMode>(
    (localStorage.getItem("bm_layout_mode") as DesktopLayoutMode) ||
      "floating_bento"
  );
  const windowMaterial = ref<WindowMaterialStyle>(
    (localStorage.getItem("bm_win_material") as WindowMaterialStyle) ||
      "mica_glow"
  );
  const colorScheme = ref<AppColorScheme>(
    (localStorage.getItem("bm_color_scheme") as AppColorScheme) ||
      "abyssal_dark"
  );
  const dynamicCoverTint = ref<boolean>(
    localStorage.getItem("bm_dynamic_tint") !== "false"
  );
  const autoClaimVipOnStartup = ref<boolean>(
    localStorage.getItem("bm_auto_claim_vip") !== "false"
  );
  const autoResumeOnStartup = ref<boolean>(
    localStorage.getItem("bm_auto_resume") === "true"
  );
  const closeToTray = ref<boolean>(
    localStorage.getItem("bm_close_to_tray") !== "false"
  );

  // 播放页与歌词页内部专属设置（对齐 Android LandscapePlayerPage & NowPlayingDialogs）
  const playerStageStyle = ref<PlayerStageStyle>(
    (localStorage.getItem("bm_stage_style") as PlayerStageStyle) ||
      "vinyl_turntable"
  );
  const playerBgStyle = ref<PlayerBackgroundStyle>(
    (localStorage.getItem("bm_bg_style") as PlayerBackgroundStyle) ||
      "fluid_aurora"
  );
  const lyricTheme = ref<LyricThemeId>(
    (localStorage.getItem("bm_lyric_theme") as LyricThemeId) || "ice_blue"
  );
  const lyricFontSize = ref<number>(
    Number(localStorage.getItem("bm_lyric_font_size")) || 24
  );
  const lyricTranslationScale = ref<number>(
    Number(localStorage.getItem("bm_lyric_trans_scale")) || 0.72
  );
  const lyricLineSpacing = ref<number>(
    Number(localStorage.getItem("bm_lyric_line_spacing")) || 1.45
  );
  const lyricAlignment = ref<LyricAlignment>(
    (localStorage.getItem("bm_lyric_align") as LyricAlignment) || "left"
  );
  const lyricBlurEnabled = ref<boolean>(
    localStorage.getItem("bm_lyric_blur") !== "false"
  );
  const lyricBlurIntensity = ref<number>(
    Number(localStorage.getItem("bm_lyric_blur_int")) || 3.2
  );
  const lyricWordByWord = ref<boolean>(
    localStorage.getItem("bm_lyric_krc") !== "false"
  );
  const lyricOffsetMs = ref<number>(
    Number(localStorage.getItem("bm_lyric_offset_ms")) || 0
  );
  const bluetoothDelayMs = ref<number>(
    Number(localStorage.getItem("bm_bt_delay_ms")) || 0
  );
  const showTranslation = ref<boolean>(true);
  const showPhonetic = ref<boolean>(false);
  const keepScreenAwake = ref<boolean>(true);

  // 进阶播放控制（副歌速刷 Climax 30s、伴奏模式、音量平衡、跨曲淡入淡出、倍速、升降调、睡眠定时）
  const climaxModeEnabled = ref<boolean>(
    localStorage.getItem("bm_climax_mode") === "true"
  );
  const ktvAccompanimentMode = ref<boolean>(false);
  const replayGainEnabled = ref<boolean>(
    localStorage.getItem("bm_replay_gain") !== "false"
  );
  const nightLoudnessEnabled = ref<boolean>(
    localStorage.getItem("bm_night_loudness") === "true"
  );
  const crossfadeSeconds = ref<number>(
    Number(localStorage.getItem("bm_crossfade_sec")) || 0
  );
  const playPauseFadeEnabled = ref<boolean>(
    localStorage.getItem("bm_play_fade") !== "false"
  );
  const playbackRate = ref<number>(1.0);
  const pitchSemitones = ref<number>(0);
  const linkPitchWithSpeed = ref<boolean>(true);

  const sleepTimerMinutes = ref<number>(0);
  const sleepWaitCurrentSong = ref<boolean>(true);
  const sleepRemainingSeconds = ref<number>(0);
  let sleepIntervalId: number | null = null;

  // 合规隐藏式「音乐下载与歌词保存」开关（默认关闭隐藏，在设置->功能与隐私管控中手动开启后解锁）
  const downloadFeatureUnlocked = ref<boolean>(
    localStorage.getItem("bm_download_unlocked") === "true"
  );
  const downloadQuality = ref<AudioQualityValue>(
    (localStorage.getItem("bm_dl_quality") as AudioQualityValue) || "high"
  );
  const downloadMusicDir = ref<string>(
    localStorage.getItem("bm_dl_music_dir") ||
      "C:\\Users\\Public\\Music\\BreezeMusic"
  );
  const downloadLrcDir = ref<string>(
    localStorage.getItem("bm_dl_lrc_dir") ||
      "C:\\Users\\Public\\Music\\BreezeMusic\\Lyrics"
  );
  const embedCoverAndTags = ref<boolean>(
    localStorage.getItem("bm_dl_embed_cover") !== "false"
  );
  const embedLyricsInAudio = ref<boolean>(
    localStorage.getItem("bm_dl_embed_lyric") !== "false"
  );
  const exportStandaloneLrc = ref<boolean>(
    localStorage.getItem("bm_dl_export_lrc") !== "false"
  );
  const bilingualLrcExport = ref<boolean>(
    localStorage.getItem("bm_dl_bilingual_lrc") !== "false"
  );
  const skipExistingFiles = ref<boolean>(
    localStorage.getItem("bm_dl_skip_exist") !== "false"
  );
  const downloadHistory = ref<DownloadHistoryRecord[]>(
    JSON.parse(localStorage.getItem("bm_dl_history") || "[]")
  );
  const isDownloadingSong = ref<boolean>(false);

  // 四维存储缓存明细统计（对齐 SettingsDownloadAndPermissions.kt）
  const cacheStats = reactive({
    audioStreamMb: 128.4,
    artworkImageMb: 34.2,
    playlistSnapshotMb: 6.8,
    lyricAndUrlMb: 4.5,
    maxQuotaGb: 4,
  });

  // 窗口尺寸记忆：解决从迷你黑胶挂件恢复主窗口时破坏最大化状态或用户自定义窗口尺寸的问题
  let savedNormalBounds = { width: 1260, height: 820, wasMaximized: false };

  watch(layoutMode, async (mode, oldMode) => {
    localStorage.setItem("bm_layout_mode", mode);
    try {
      const win = getCurrentWindow();
      if (mode === "mini_turntable") {
        const isMax = await win.isMaximized();
        savedNormalBounds.wasMaximized = isMax;
        if (isMax) {
          await win.unmaximize();
        } else {
          const factor = await win.scaleFactor();
          const pSize = await win.innerSize();
          if (pSize.width / factor >= 700) {
            savedNormalBounds.width = Math.round(pSize.width / factor);
            savedNormalBounds.height = Math.round(pSize.height / factor);
          }
        }
        await win.setSize(new LogicalSize(420, 168));
        await win.setAlwaysOnTop(true);
      } else if (oldMode === "mini_turntable") {
        await win.setAlwaysOnTop(false);
        await win.setSize(
          new LogicalSize(
            Math.max(960, savedNormalBounds.width),
            Math.max(640, savedNormalBounds.height)
          )
        );
        if (savedNormalBounds.wasMaximized) {
          await win.maximize();
        }
      }
    } catch {
      // 浏览器预览环境跳过窗口尺寸调用
    }
  });
  watch(windowMaterial, (v) => localStorage.setItem("bm_win_material", v));
  watch(colorScheme, (v) => localStorage.setItem("bm_color_scheme", v));
  watch(dynamicCoverTint, (v) =>
    localStorage.setItem("bm_dynamic_tint", String(v))
  );
  watch(autoClaimVipOnStartup, (v) =>
    localStorage.setItem("bm_auto_claim_vip", String(v))
  );
  watch(autoResumeOnStartup, (v) =>
    localStorage.setItem("bm_auto_resume", String(v))
  );
  watch(closeToTray, (v) =>
    localStorage.setItem("bm_close_to_tray", String(v))
  );
  watch(playerStageStyle, (v) => localStorage.setItem("bm_stage_style", v));
  watch(playerBgStyle, (v) => localStorage.setItem("bm_bg_style", v));
  watch(lyricTheme, (v) => localStorage.setItem("bm_lyric_theme", v));
  watch(lyricFontSize, (v) =>
    localStorage.setItem("bm_lyric_font_size", String(v))
  );
  watch(lyricTranslationScale, (v) =>
    localStorage.setItem("bm_lyric_trans_scale", String(v))
  );
  watch(lyricLineSpacing, (v) =>
    localStorage.setItem("bm_lyric_line_spacing", String(v))
  );
  watch(lyricAlignment, (v) => localStorage.setItem("bm_lyric_align", v));
  watch(lyricBlurEnabled, (v) =>
    localStorage.setItem("bm_lyric_blur", String(v))
  );
  watch(lyricBlurIntensity, (v) =>
    localStorage.setItem("bm_lyric_blur_int", String(v))
  );
  watch(lyricWordByWord, (v) =>
    localStorage.setItem("bm_lyric_krc", String(v))
  );
  watch(lyricOffsetMs, (v) =>
    localStorage.setItem("bm_lyric_offset_ms", String(v))
  );
  watch(bluetoothDelayMs, (v) =>
    localStorage.setItem("bm_bt_delay_ms", String(v))
  );
  watch(desktopLyricEnabled, (v) =>
    localStorage.setItem("bm_desktop_lyric", String(v))
  );
  watch(climaxModeEnabled, (v) =>
    localStorage.setItem("bm_climax_mode", String(v))
  );
  watch(replayGainEnabled, (v) =>
    localStorage.setItem("bm_replay_gain", String(v))
  );
  watch(nightLoudnessEnabled, (v) =>
    localStorage.setItem("bm_night_loudness", String(v))
  );
  watch(crossfadeSeconds, (v) =>
    localStorage.setItem("bm_crossfade_sec", String(v))
  );
  watch(playPauseFadeEnabled, (v) =>
    localStorage.setItem("bm_play_fade", String(v))
  );
  watch(isLibraryGridView, (v) =>
    localStorage.setItem("bm_lib_grid", String(v))
  );
  watch(downloadFeatureUnlocked, (v) =>
    localStorage.setItem("bm_download_unlocked", String(v))
  );
  watch(downloadQuality, (v) => localStorage.setItem("bm_dl_quality", v));
  watch(downloadMusicDir, (v) => localStorage.setItem("bm_dl_music_dir", v));
  watch(downloadLrcDir, (v) => localStorage.setItem("bm_dl_lrc_dir", v));
  watch(embedCoverAndTags, (v) =>
    localStorage.setItem("bm_dl_embed_cover", String(v))
  );
  watch(embedLyricsInAudio, (v) =>
    localStorage.setItem("bm_dl_embed_lyric", String(v))
  );
  watch(exportStandaloneLrc, (v) =>
    localStorage.setItem("bm_dl_export_lrc", String(v))
  );
  watch(bilingualLrcExport, (v) =>
    localStorage.setItem("bm_dl_bilingual_lrc", String(v))
  );
  watch(skipExistingFiles, (v) =>
    localStorage.setItem("bm_dl_skip_exist", String(v))
  );

  // ================= 6. 账号、多账号切换、等级时长与概念版 VIP 状态 =================
  const session = ref<KugouSession | null>(null);
  const savedAccounts = ref<KugouSession[]>([]);
  const profile = ref<AccountProfile | null>(null);
  const vipStatus = ref<VipStatusInfo | null>(null);
  const isClaimingVip = ref(false);
  const gatewayDiag = ref<GatewayDiagResult | null>(null);
  const isDiagnosingGateway = ref(false);

  const isLoggedIn = computed(() => Boolean(session.value?.token));

  // ================= 7. 专辑封面三色流体提取器 =================
  const coverPalette = ref<[string, string, string]>([
    "#2b8cff",
    "#00c4a7",
    "#6c5ce7",
  ]);

  function extractCoverColors(imgUrl: string) {
    if (!imgUrl) {
      coverPalette.value = ["#2b8cff", "#00c4a7", "#6c5ce7"];
      return;
    }
    const img = new Image();
    img.crossOrigin = "Anonymous";
    img.onload = () => {
      try {
        const canvas = document.createElement("canvas");
        canvas.width = 24;
        canvas.height = 24;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;
        ctx.drawImage(img, 0, 0, 24, 24);
        const data = ctx.getImageData(0, 0, 24, 24).data;
        const samples: [number, number, number][] = [];
        for (let i = 0; i < data.length; i += 16) {
          const r = data[i];
          const g = data[i + 1];
          const b = data[i + 2];
          const lum = 0.299 * r + 0.587 * g + 0.114 * b;
          if (lum > 28 && lum < 230) {
            samples.push([r, g, b]);
          }
        }
        if (samples.length >= 3) {
          const c1 = samples[Math.floor(samples.length * 0.15)];
          const c2 = samples[Math.floor(samples.length * 0.5)];
          const c3 = samples[Math.floor(samples.length * 0.85)];
          coverPalette.value = [
            `rgb(${c1[0]}, ${c1[1]}, ${c1[2]})`,
            `rgb(${c2[0]}, ${c2[1]}, ${c2[2]})`,
            `rgb(${c3[0]}, ${c3[1]}, ${c3[2]})`,
          ];
        }
      } catch {
        // 跨域未携带 CORS 时保留默认流体极光色板
      }
    };
    img.src = imgUrl;
  }

  // ================= 8. 64-bit 风格母带级 WebAudio DSP 引擎 + 播放连续动量 =================
  const audio = new Audio();
  audio.preload = "auto";
  audio.crossOrigin = "anonymous";

  const playQueue = ref<Song[]>([]);
  const recentPlayQueues = ref<{ title: string; songs: Song[] }[]>([]);
  const currentIndex = ref<number>(-1);
  const playMode = ref<PlayMode>("repeat_all");
  const currentSong = computed<Song | null>(() =>
    currentIndex.value >= 0 && currentIndex.value < playQueue.value.length
      ? playQueue.value[currentIndex.value]
      : null
  );

  const isPlaying = ref(false);
  const isBuffering = ref(false);
  const currentPositionMs = ref(0);
  const durationMs = ref(0);
  const volume = ref(0.85);
  audio.volume = volume.value;

  // 记录单曲播放开始时间戳，供 CSCC 三段式真实听歌时长上报使用
  let currentSongPlayStartWallMs = 0;

  // 播放 <-> 暂停 880ms 连续动量相位因子 (0.0 静止 ~ 1.0 全速)，消除暂停时背景与唱盘跳变
  const playMomentum = ref<number>(0);
  let momentumRafId: number | null = null;
  function animatePlayMomentum(target: number) {
    if (momentumRafId) cancelAnimationFrame(momentumRafId);
    const startVal = playMomentum.value;
    const startTime = performance.now();
    const duration = 880;
    const step = (now: number) => {
      const t = Math.min(1, (now - startTime) / duration);
      const eased = t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
      playMomentum.value = startVal + (target - startVal) * eased;
      if (t < 1) {
        momentumRafId = requestAnimationFrame(step);
      }
    };
    momentumRafId = requestAnimationFrame(step);
  }
  watch(isPlaying, (playing) => animatePlayMomentum(playing ? 1 : 0));

  const preferredQuality = ref<AudioQualityValue>(
    (localStorage.getItem("bm_pref_quality") as AudioQualityValue) || "high"
  );
  const currentPlaybackInfo = ref<PlaybackResult | null>(null);
  const currentLyric = ref<LyricData>({
    lines: [],
    rawText: "",
    isKrc: false,
    hasTranslation: false,
    hasPhonetic: false,
  });
  const isLoadingLyric = ref(false);

  // 歌曲评论区状态（含楼中楼展开与评论发送）
  const activeSongComments = ref<SongComment[]>([]);
  const commentSortType = ref<"hot" | "new">("hot");
  const isLoadingComments = ref(false);
  const expandedFloorCommentId = ref<string | null>(null);
  const floorRepliesMap = reactive<Record<string, SongComment[]>>({});
  const isLoadingFloorReplies = ref<boolean>(false);

  // 10 段参量均衡器 (PEQ) 与 3 组自定义记忆槽 (A/B/C) + 声场调音台 + 酷狗蝰蛇音效库
  const eqFrequencies = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
  const eqGains = ref<number[]>([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
  const eqBandTypes = ref<PeqFilterType[]>([
    "lowshelf",
    "peaking",
    "peaking",
    "peaking",
    "peaking",
    "peaking",
    "peaking",
    "peaking",
    "peaking",
    "highshelf",
  ]);
  const eqQFactor = ref<number>(1.41);
  const eqHighPassHz = ref<number>(20);
  const eqLowPassHz = ref<number>(20000);
  const activeEqPreset = ref<string>("flat");
  const peqBypass = ref<boolean>(false);

  const viperEffects = ref<ViperEffectItem[]>([]);
  const activeViperEffectId = ref<string>("off");

  const peqMemorySlots = ref<PeqMemorySlot[]>(
    JSON.parse(
      localStorage.getItem("bm_peq_slots") ||
        JSON.stringify([
          {
            slotId: "A",
            name: "记忆槽 A",
            saved: false,
            gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            qFactor: 1.41,
          },
          {
            slotId: "B",
            name: "记忆槽 B",
            saved: false,
            gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            qFactor: 1.41,
          },
          {
            slotId: "C",
            name: "记忆槽 C",
            saved: false,
            gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            qFactor: 1.41,
          },
        ])
    )
  );

  const acousticField = reactive<AcousticFieldSettings>({
    enabled: false,
    subBassDepth: 35,
    bassCrossoverHz: 68,
    transientPunch: 30,
    surroundWidth: 42,
    stageDepth: 28,
    bauerCrossfeed: true,
    reverbPreset: "none",
    reverbPreDelayMs: 24,
    vocalClarity: 40,
    airHarmonics: 32,
    vinylTubeWarmth: 20,
    stereoWidth: 1.0,
    reverbMix: 0,
    bassBoostDb: 0,
  });

  const spectrumBars = ref<number[]>(new Array(32).fill(6));

  let audioCtx: AudioContext | null = null;
  let analyser: AnalyserNode | null = null;
  let highPassFilter: BiquadFilterNode | null = null;
  let lowPassFilter: BiquadFilterNode | null = null;
  let eqFilters: BiquadFilterNode[] = [];
  let subBassFilter: BiquadFilterNode | null = null;
  let airShelfFilter: BiquadFilterNode | null = null;
  let vocalPeakFilter: BiquadFilterNode | null = null;
  let deEsserNotchFilter: BiquadFilterNode | null = null;
  let rafId: number | null = null;

  function ensureWebAudioGraph() {
    if (audioCtx) return;
    try {
      const Ctx = window.AudioContext || (window as any).webkitAudioContext;
      audioCtx = new Ctx();
      const source = audioCtx.createMediaElementSource(audio);
      analyser = audioCtx.createAnalyser();
      analyser.fftSize = 128;
      analyser.smoothingTimeConstant = 0.78;

      highPassFilter = audioCtx.createBiquadFilter();
      highPassFilter.type = "highpass";
      highPassFilter.frequency.value = eqHighPassHz.value;

      lowPassFilter = audioCtx.createBiquadFilter();
      lowPassFilter.type = "lowpass";
      lowPassFilter.frequency.value = eqLowPassHz.value;

      subBassFilter = audioCtx.createBiquadFilter();
      subBassFilter.type = "lowshelf";
      subBassFilter.frequency.value = acousticField.bassCrossoverHz;
      subBassFilter.gain.value = 0;

      vocalPeakFilter = audioCtx.createBiquadFilter();
      vocalPeakFilter.type = "peaking";
      vocalPeakFilter.frequency.value = 2800;
      vocalPeakFilter.Q.value = 1.2;
      vocalPeakFilter.gain.value = 0;

      deEsserNotchFilter = audioCtx.createBiquadFilter();
      deEsserNotchFilter.type = "peaking";
      deEsserNotchFilter.frequency.value = 6200;
      deEsserNotchFilter.Q.value = 2.6;
      deEsserNotchFilter.gain.value = 0;

      airShelfFilter = audioCtx.createBiquadFilter();
      airShelfFilter.type = "highshelf";
      airShelfFilter.frequency.value = 12000;
      airShelfFilter.gain.value = 0;

      eqFilters = eqFrequencies.map((freq, idx) => {
        const filter = audioCtx!.createBiquadFilter();
        filter.type = eqBandTypes.value[idx] || "peaking";
        filter.frequency.value = freq;
        filter.Q.value = eqQFactor.value;
        filter.gain.value = eqGains.value[idx] || 0;
        return filter;
      });

      let prevNode: AudioNode = source;
      prevNode.connect(highPassFilter);
      prevNode = highPassFilter;
      for (const f of eqFilters) {
        prevNode.connect(f);
        prevNode = f;
      }
      prevNode.connect(lowPassFilter);
      lowPassFilter.connect(subBassFilter);
      subBassFilter.connect(vocalPeakFilter);
      vocalPeakFilter.connect(deEsserNotchFilter);
      deEsserNotchFilter.connect(airShelfFilter);
      airShelfFilter.connect(analyser);
      analyser.connect(audioCtx.destination);
      syncDspChain();
      startSpectrumLoop();
    } catch (e) {
      console.warn("WebAudio graph fallback:", e);
    }
  }

  function syncDspChain() {
    const bypass = peqBypass.value;
    const scale = acousticField.enabled ? 0.82 : 1.0;
    if (highPassFilter && lowPassFilter) {
      highPassFilter.frequency.value = bypass ? 10 : Math.max(15, eqHighPassHz.value);
      lowPassFilter.frequency.value = bypass ? 22000 : Math.min(20000, eqLowPassHz.value);
    }
    eqFilters.forEach((f, idx) => {
      f.type = eqBandTypes.value[idx] || "peaking";
      f.Q.value = eqQFactor.value;
      f.gain.value = bypass ? 0 : (eqGains.value[idx] || 0) * scale;
    });
    if (
      subBassFilter &&
      vocalPeakFilter &&
      deEsserNotchFilter &&
      airShelfFilter
    ) {
      subBassFilter.frequency.value = acousticField.bassCrossoverHz;
      subBassFilter.gain.value =
        !bypass && acousticField.enabled
          ? (acousticField.subBassDepth / 100) * 6.5
          : 0;
      vocalPeakFilter.gain.value =
        !bypass && acousticField.enabled
          ? (acousticField.vocalClarity / 100) * 4.2
          : 0;
      deEsserNotchFilter.gain.value =
        !bypass && acousticField.enabled
          ? -(acousticField.vocalClarity / 100) * 2.4
          : 0;
      airShelfFilter.gain.value =
        !bypass && acousticField.enabled
          ? ((acousticField.airHarmonics - acousticField.vinylTubeWarmth * 0.35) /
              100) *
            5.0
          : 0;
    }
  }

  function applyViperEffect(effectOrId: ViperEffectItem | string | null) {
    const effect =
      typeof effectOrId === "string"
        ? effectOrId === "off" || effectOrId === "none"
          ? null
          : viperEffects.value.find((v) => v.id === effectOrId) || null
        : effectOrId;
    if (!effect || activeViperEffectId.value === effect.id) {
      activeViperEffectId.value = "off";
      acousticField.enabled = false;
      setEqPreset("flat");
      notify("已关闭蝰蛇音效，恢复母带直通 (Bit-Perfect)");
      return;
    }
    activeViperEffectId.value = effect.id;
    activeEqPreset.value = effect.id;
    eqGains.value = [...effect.gains];
    acousticField.enabled = true;
    acousticField.subBassDepth = effect.subBass;
    acousticField.surroundWidth = effect.surround;
    acousticField.vocalClarity = effect.vocalClarity;
    acousticField.airHarmonics = effect.airHarmonics;
    acousticField.vinylTubeWarmth = effect.tubeWarmth;
    syncDspChain();
    notify(`已启用酷狗蝰蛇音效：「${effect.label || effect.name}」`);
  }

  function startSpectrumLoop() {
    if (rafId) cancelAnimationFrame(rafId);
    const freqData = new Uint8Array(64);
    const tick = () => {
      if (isPlaying.value && analyser) {
        analyser.getByteFrequencyData(freqData);
        const bars: number[] = [];
        for (let i = 0; i < 32; i++) {
          const val = freqData[i] || 0;
          bars.push(Math.max(6, Math.round((val / 255) * 100)));
        }
        spectrumBars.value = bars;
      } else if (playMomentum.value > 0.02) {
        spectrumBars.value = spectrumBars.value.map((v) =>
          Math.max(6, Math.round(v * 0.88))
        );
      }
      rafId = requestAnimationFrame(tick);
    };
    rafId = requestAnimationFrame(tick);
  }

  function setEqPreset(presetName: string) {
    activeEqPreset.value = presetName;
    if (presetName === "flat") {
      activeViperEffectId.value = "off";
    }
    const presets: Record<string, number[]> = {
      flat: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      harman: [5.2, 4.4, 1.8, -0.8, -0.4, 1.4, 3.8, 4.2, 2.4, 1.8],
      diffuse_field: [1.2, 1.0, 0.6, 0.0, 0.8, 2.2, 3.6, 3.2, 2.8, 3.4],
      vocal_velvet: [-1.4, -0.6, 0.8, 2.2, 3.4, 4.4, 3.8, -1.8, 1.6, 2.4],
      hifi_sub: [6.2, 5.4, 2.6, -1.6, -0.6, 0.6, 1.8, 2.4, 2.8, 3.2],
      acg_bright: [2.5, 1.8, 0.0, 1.0, 2.4, 3.5, 4.5, 4.8, 3.8, 2.5],
    };
    const gains = presets[presetName] || presets.flat;
    eqGains.value = [...gains];
    syncDspChain();
  }

  function updateSingleEqBand(index: number, gainDb: number) {
    activeEqPreset.value = "custom";
    const clamped = Math.max(-10, Math.min(10, Number(gainDb.toFixed(1))));
    eqGains.value[index] = clamped;
    syncDspChain();
  }

  function savePeqToSlot(slotId: "A" | "B" | "C") {
    const slot = peqMemorySlots.value.find((s) => s.slotId === slotId);
    if (!slot) return;
    slot.gains = [...eqGains.value];
    slot.qFactor = eqQFactor.value;
    slot.bandTypes = [...eqBandTypes.value];
    slot.saved = true;
    localStorage.setItem("bm_peq_slots", JSON.stringify(peqMemorySlots.value));
    notify(`已存入母带均衡器记忆槽 ${slotId}`);
  }

  function loadPeqFromSlot(slotId: "A" | "B" | "C") {
    const slot = peqMemorySlots.value.find((s) => s.slotId === slotId);
    if (!slot || !slot.saved) {
      notify(`记忆槽 ${slotId} 尚未存入曲线，点击右侧「存入」即可保存当前参数`);
      return;
    }
    eqGains.value = [...slot.gains];
    eqQFactor.value = slot.qFactor;
    if (slot.bandTypes && slot.bandTypes.length === 10) {
      eqBandTypes.value = [...slot.bandTypes];
    }
    activeEqPreset.value = `slot_${slotId}`;
    syncDspChain();
    notify(`已载入记忆槽 ${slotId} 曲线`);
  }

  function resetSinglePeqSlot(slotId: "A" | "B" | "C") {
    const slot = peqMemorySlots.value.find((s) => s.slotId === slotId);
    if (!slot) return;
    slot.saved = false;
    slot.gains = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    slot.qFactor = 1.41;
    localStorage.setItem("bm_peq_slots", JSON.stringify(peqMemorySlots.value));
    notify(`已清空记忆槽 ${slotId}`);
  }

  function resetAllPeqSlots() {
    peqMemorySlots.value.forEach((s) => {
      s.saved = false;
      s.gains = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
      s.qFactor = 1.41;
    });
    localStorage.setItem("bm_peq_slots", JSON.stringify(peqMemorySlots.value));
    setEqPreset("flat");
    notify("已重置全部记忆槽与均衡器参数");
  }

  // HTML5 Audio 事件绑定
  audio.addEventListener("timeupdate", () => {
    const posMs = Math.floor(audio.currentTime * 1000);
    currentPositionMs.value = posMs;

    // 副歌速刷模式 (Climax 30s 精华连播)：到达 climaxEndMs 自动切下一首
    if (climaxModeEnabled.value && currentPlaybackInfo.value) {
      const cEnd = currentPlaybackInfo.value.climaxEndMs || 75000;
      if (cEnd > 10000 && posMs >= cEnd) {
        playNext(true);
      }
    }
  });
  audio.addEventListener("loadedmetadata", () => {
    if (audio.duration && isFinite(audio.duration)) {
      durationMs.value = Math.floor(audio.duration * 1000);
    }
    applySpeedAndPitch();
  });
  audio.addEventListener("play", () => {
    isPlaying.value = true;
    isBuffering.value = false;
  });
  audio.addEventListener("pause", () => {
    isPlaying.value = false;
  });
  audio.addEventListener("waiting", () => {
    isBuffering.value = true;
  });
  audio.addEventListener("canplay", () => {
    isBuffering.value = false;
  });
  audio.addEventListener("ended", () => {
    // 上报本首完整播放 CSCC 事件
    if (currentSong.value) {
      const played =
        currentSongPlayStartWallMs > 0
          ? Math.max(durationMs.value, Date.now() - currentSongPlayStartWallMs)
          : durationMs.value || 180000;
      kugouClient
        .reportPlaybackTelemetry(currentSong.value, false, played)
        .then((settled) => {
          if (settled && isLoggedIn.value) {
            setTimeout(() => {
              kugouClient
                .getAccountProfile()
                .then((p) => {
                  if (p) profile.value = p;
                })
                .catch(() => {});
            }, 3500);
          }
        })
        .catch(() => {});
    }

    if (
      sleepTimerMinutes.value === -1 ||
      (sleepRemainingSeconds.value <= 0 && sleepTimerMinutes.value > 0)
    ) {
      sleepTimerMinutes.value = 0;
      audio.pause();
      notify("睡眠定时已到点，已为您自然播完本曲并暂停");
      return;
    }
    playNext(true);
  });

  function notify(msg: string) {
    statusBanner.value = msg;
    setTimeout(() => {
      if (statusBanner.value === msg) {
        statusBanner.value = "";
      }
    }, 3400);
  }

  function applySpeedAndPitch() {
    const pitchFactor = Math.pow(2, pitchSemitones.value / 12);
    if (linkPitchWithSpeed.value) {
      audio.playbackRate = Number((playbackRate.value * pitchFactor).toFixed(3));
      (audio as any).preservesPitch = false;
    } else {
      audio.playbackRate = playbackRate.value;
      (audio as any).preservesPitch = true;
    }
  }

  function setPlaybackSpeed(rate: number) {
    playbackRate.value = Number(rate.toFixed(2));
    applySpeedAndPitch();
  }

  function setPitchSemitones(semitones: number) {
    pitchSemitones.value = Math.max(-6, Math.min(6, semitones));
    applySpeedAndPitch();
  }

  function startSleepTimer(mins: number) {
    if (sleepIntervalId) {
      clearInterval(sleepIntervalId);
      sleepIntervalId = null;
    }
    sleepTimerMinutes.value = mins;
    if (mins <= 0) {
      sleepRemainingSeconds.value = 0;
      if (mins === -1) {
        notify("将在当前曲目播放完毕后自动停止");
      } else {
        notify("已关闭睡眠定时");
      }
      return;
    }
    sleepRemainingSeconds.value = mins * 60;
    notify(`已开启睡眠定时：${mins} 分钟后停止播放`);
    sleepIntervalId = window.setInterval(() => {
      if (sleepRemainingSeconds.value > 0) {
        sleepRemainingSeconds.value -= 1;
        if (sleepRemainingSeconds.value <= 0) {
          if (sleepIntervalId) clearInterval(sleepIntervalId);
          sleepIntervalId = null;
          if (!sleepWaitCurrentSong.value) {
            audio.pause();
            sleepTimerMinutes.value = 0;
            notify("睡眠定时已结束，音乐已暂停");
          } else {
            notify("睡眠定时已到点，将在当前歌曲播完后停止");
          }
        }
      }
    }, 1000);
  }

  // ================= 9. 核心启动与全模块数据加载 =================
  async function bootstrap() {
    try {
      const [savedSession, accounts, vipers] = await Promise.all([
        kugouClient.initAppState().catch(() => null),
        kugouClient.getSavedAccounts().catch(() => []),
        kugouClient.getViperEffects().catch(() => []),
      ]);
      savedAccounts.value = accounts;
      viperEffects.value = vipers;
      if (savedSession && savedSession.token) {
        session.value = savedSession;
        refreshAccountAndVip(true);
      }
    } catch (e) {
      console.warn("initAppState skipped in browser preview:", e);
    }

    loadDailyRecommend();
    loadPersonalFm();
    loadNewSongExpress("CHINESE");
    loadRankList();
    loadFmChannels();
    kugouClient
      .getSearchHot()
      .then((kws) => {
        if (kws.length > 0) hotSearchKeywords.value = kws;
      })
      .catch(() => {});
  }

  async function refreshAccountAndVip(isStartup = false) {
    if (!isLoggedIn.value) return;
    try {
      const [prof, vip, accounts] = await Promise.all([
        kugouClient.getAccountProfile().catch(() => null),
        kugouClient.getVipStatus().catch(() => null),
        kugouClient.getSavedAccounts().catch(() => []),
      ]);
      if (accounts.length > 0) savedAccounts.value = accounts;
      if (prof) profile.value = prof;
      if (vip) {
        vipStatus.value = vip;
        if (isStartup && autoClaimVipOnStartup.value && !vip.todayClaimed) {
          claimVipDaily(true);
        }
      }
      loadUserPlaylists();
      loadRecentHistory();
      loadFollowedArtists();
      loadPurchasedAssets();
    } catch (e) {
      console.warn("refreshAccountAndVip error:", e);
    }
  }

  async function switchAccount(userId: string) {
    try {
      const switched = await kugouClient.switchSavedAccount(userId);
      if (switched) {
        session.value = switched;
        isLoginModalOpen.value = false;
        notify(`已切换至账号「${switched.nickname}」`);
        await refreshAccountAndVip(false);
        await loadDailyRecommend();
      }
    } catch (e: any) {
      notify(`切换账号失败：${String(e)}`);
    }
  }

  async function removeSavedAccount(userId: string) {
    try {
      savedAccounts.value = await kugouClient.removeSavedAccount(userId);
      if (session.value?.userId === userId) {
        session.value = savedAccounts.value[0] || null;
        if (session.value) {
          await refreshAccountAndVip(false);
        } else {
          await logout();
        }
      }
    } catch (e: any) {
      notify(`移除账号失败：${String(e)}`);
    }
  }

  async function claimVipDaily(silent = false) {
    if (!isLoggedIn.value) {
      isLoginModalOpen.value = true;
      return;
    }
    isClaimingVip.value = true;
    try {
      const updated = await kugouClient.claimDailyVip();
      vipStatus.value = updated;
      const prof = await kugouClient.getAccountProfile().catch(() => null);
      if (prof) profile.value = prof;
      if (!silent) {
        notify("概念版今日 VIP 签到与升级已完成！");
      }
    } catch (e: any) {
      if (!silent) {
        notify(`VIP 签到提示：${String(e)}`);
      }
    } finally {
      isClaimingVip.value = false;
    }
  }

  async function resignDeviceAndRefresh() {
    isDiagnosingGateway.value = true;
    try {
      const res = await kugouClient.resignDeviceIdentity();
      gatewayDiag.value = res;
      if (vipStatus.value) {
        vipStatus.value.dfid = res.dfid;
        vipStatus.value.guid = res.guid;
      }
      notify(res.statusText);
    } catch (e: any) {
      notify(`重签设备标识失败：${String(e)}`);
    } finally {
      isDiagnosingGateway.value = false;
    }
  }

  async function runGatewayDiagnostic() {
    isDiagnosingGateway.value = true;
    try {
      const res = await kugouClient.diagnoseGatewayLatency();
      gatewayDiag.value = res;
      notify(res.statusText);
    } catch (e: any) {
      notify(`网关诊断异常：${String(e)}`);
    } finally {
      isDiagnosingGateway.value = false;
    }
  }

  async function loadDailyRecommend() {
    isLoadingRecommend.value = true;
    try {
      recommendSongs.value = await kugouClient.getDailyRecommend();
      if (playQueue.value.length === 0 && recommendSongs.value.length > 0) {
        playQueue.value = [...recommendSongs.value];
        currentIndex.value = 0;
        const first = recommendSongs.value[0];
        durationMs.value = first.durationMs || 210000;
        extractCoverColors(first.coverUrl);
      }
    } catch (e) {
      console.warn("loadDailyRecommend error:", e);
    } finally {
      isLoadingRecommend.value = false;
    }
  }

  async function loadPersonalFm(append = false) {
    isLoadingFm.value = true;
    try {
      const songs = await kugouClient.getPersonalFm();
      if (songs.length > 0) {
        if (append) {
          const seen = new Set(personalFmSongs.value.map((s) => s.hash));
          const unique = songs.filter((s) => !seen.has(s.hash));
          personalFmSongs.value = [...personalFmSongs.value, ...unique];
          notify(`已为随心听漫游池无缝续杯 ${unique.length} 首歌曲`);
        } else {
          personalFmSongs.value = songs;
          personalFmIndex.value = 0;
        }
      }
    } catch (e) {
      console.warn("loadPersonalFm error:", e);
    } finally {
      isLoadingFm.value = false;
    }
  }

  async function nextPersonalFmTrack(autoPlay = true) {
    if (personalFmSongs.value.length === 0) {
      await loadPersonalFm();
    }
    if (personalFmSongs.value.length === 0) return;

    if (personalFmIndex.value + 1 < personalFmSongs.value.length) {
      personalFmIndex.value += 1;
    } else {
      await loadPersonalFm();
    }
    if (autoPlay && currentFmSong.value) {
      playSongFromList(currentFmSong.value, personalFmSongs.value);
    }
  }

  async function loadNewSongExpress(period = "CHINESE") {
    newSongPeriod.value = period;
    isLoadingNewSongs.value = true;
    try {
      newSongs.value = await kugouClient.getNewSongExpress(period);
      resolveFmChannelCovers();
    } catch (e) {
      console.warn("loadNewSongExpress error:", e);
    } finally {
      isLoadingNewSongs.value = false;
    }
  }

  async function loadRankList() {
    isLoadingRanks.value = true;
    try {
      rankList.value = await kugouClient.getRankList();
      resolveFmChannelCovers();
    } catch (e) {
      console.warn("loadRankList error:", e);
    } finally {
      isLoadingRanks.value = false;
    }
  }

  async function openRankDetail(rank: RankItem) {
    activeRank.value = rank;
    rankSearchKeyword.value = "";
    discoverSubView.value = "rank";
    activeTab.value = "discover";
    isNowPlayingOpen.value = false;
    isRankGridModalOpen.value = false;
    isLoadingRankSongs.value = true;
    try {
      activeRankSongs.value = await kugouClient.getRankSongs(
        rank.rankId,
        1,
        80
      );
    } catch (e: any) {
      notify(`加载榜单曲目失败：${String(e)}`);
    } finally {
      isLoadingRankSongs.value = false;
    }
  }

  function resolveFmChannelCovers() {
    const pool: string[] = [];
    for (const s of recommendSongs.value) {
      if (s.coverUrl && !pool.includes(s.coverUrl)) pool.push(s.coverUrl);
    }
    for (const s of newSongs.value) {
      if (s.coverUrl && !pool.includes(s.coverUrl)) pool.push(s.coverUrl);
    }
    for (const r of rankList.value) {
      if (r.coverUrl && !pool.includes(r.coverUrl)) pool.push(r.coverUrl);
    }
    for (const s of personalFmSongs.value) {
      if (s.coverUrl && !pool.includes(s.coverUrl)) pool.push(s.coverUrl);
    }
    if (pool.length === 0) return;
    fmChannels.value = fmChannels.value.map((ch, idx) => {
      if (ch.coverUrl && !ch.coverUrl.includes("20210519")) return ch;
      return {
        ...ch,
        coverUrl: pool[idx % pool.length] || "",
      };
    });
  }

  async function refreshDiscoverAll() {
    notify("正在全面焕新发现页内容...");
    await Promise.all([
      loadDailyRecommend(),
      loadPersonalFm(false),
      loadNewSongExpress(newSongPeriod.value),
      loadRankList(),
      loadFmChannels(),
    ]);
    resolveFmChannelCovers();
    notify("发现页推荐已全面焕新 ✨");
  }

  function openDailyRecommendPage() {
    if (discoverSubView.value === "daily") return;
    discoverHeroExpanding.value = "daily";
    setTimeout(() => {
      discoverSubView.value = "daily";
      discoverHeroExpanding.value = "none";
    }, 380);
  }

  function openGuessYouLikePage() {
    if (discoverSubView.value === "fm_detail") return;
    if (personalFmSongs.value.length === 0) {
      loadPersonalFm(false);
    }
    discoverHeroExpanding.value = "fm";
    setTimeout(() => {
      discoverSubView.value = "fm_detail";
      discoverHeroExpanding.value = "none";
    }, 380);
  }

  async function playRankDirectly(rank: RankItem) {
    try {
      notify(`正在加载「${rank.name}」曲目...`);
      const songs = await kugouClient.getRankSongs(rank.rankId, 1, 50);
      if (songs.length > 0) {
        activeRank.value = rank;
        activeRankSongs.value = songs;
        playSongFromList(songs[0], songs);
      } else {
        notify(`「${rank.name}」暂无可播曲目`);
      }
    } catch (e: any) {
      notify(`加载榜单失败：${String(e)}`);
    }
  }

  async function loadFmChannels() {
    try {
      const list = await kugouClient.getFmChannels();
      fmChannels.value = list;
      resolveFmChannelCovers();
      if (!activeFmChannel.value && list.length > 0) {
        activeFmChannel.value = list[0];
        loadFmStationSongs(list[0].id, false);
      }
    } catch (e) {
      console.warn("loadFmChannels error:", e);
    }
  }

  async function loadFmStationSongs(
    channelId: string,
    append = false,
    autoPlay = false
  ) {
    const found = fmChannels.value.find((c) => c.id === channelId);
    if (found) activeFmChannel.value = found;
    isLoadingFmStation.value = true;
    try {
      const batch = await kugouClient.getFmSongs(channelId);
      if (append) {
        const seen = new Set(fmStationSongs.value.map((s) => s.hash));
        const extra = batch.filter((s) => !seen.has(s.hash));
        fmStationSongs.value = [...fmStationSongs.value, ...extra];
        notify(
          `已追加 ${extra.length} 首「${activeFmChannel.value?.name || "电台"}」漫游新曲`
        );
      } else {
        fmStationSongs.value = batch;
        if (autoPlay && batch.length > 0) {
          playSongFromList(batch[0], batch);
        }
      }
    } catch (e: any) {
      notify(`调频电台加载失败：${String(e)}`);
    } finally {
      isLoadingFmStation.value = false;
    }
  }

  async function loadRecentHistory() {
    if (!isLoggedIn.value) return;
    try {
      recentHistorySongs.value = await kugouClient.getRecentHistory();
    } catch (e) {
      console.warn("loadRecentHistory error:", e);
    }
  }

  async function loadUserPlaylists() {
    if (!isLoggedIn.value) return;
    isLoadingLibrary.value = true;
    try {
      const list = await kugouClient.getUserPlaylists();
      userPlaylists.value = list;

      const likedPl = list.find((p) => p.isLiked || p.title === "我喜欢");
      if (likedPl) {
        kugouClient
          .getPlaylistSongs(likedPl.globalCollectionId)
          .then((songs) => {
            likedSongs.value = songs;
            if (likedPl.trackCount <= 0 && songs.length > 0) {
              likedPl.trackCount = songs.length;
            }
          })
          .catch(() => {});
      }

      if (list.length > 0 && !selectedPlaylist.value) {
        const preferred =
          list.find((p) => p.isCreatedByMe && !p.isLiked && p.trackCount > 0) ||
          list.find((p) => p.isLiked && p.trackCount > 0) ||
          list.find((p) => p.trackCount > 0) ||
          list[0];
        await selectPlaylist(preferred);
      }
    } catch (e: any) {
      notify(`获取歌单失败：${String(e)}`);
    } finally {
      isLoadingLibrary.value = false;
    }
  }

  async function selectPlaylist(pl: Playlist) {
    selectedPlaylist.value = pl;
    libraryDetailMode.value = "playlist";
    isLoadingLibrary.value = true;
    try {
      const songs = await kugouClient.getPlaylistSongs(
        pl.globalCollectionId || pl.id
      );
      playlistSongs.value = songs;
      if (pl.trackCount < songs.length) {
        pl.trackCount = songs.length;
      }
      if (pl.isLiked || pl.title === "我喜欢") {
        likedSongs.value = songs;
      }
    } catch (e: any) {
      notify(`加载歌单曲目失败：${String(e)}`);
    } finally {
      isLoadingLibrary.value = false;
    }
  }

  async function createNewPlaylist(name: string) {
    if (!isLoggedIn.value) {
      isLoginModalOpen.value = true;
      return false;
    }
    try {
      await kugouClient.createUserPlaylist(name);
      notify(`已创建歌单「${name}」`);
      await loadUserPlaylists();
      return true;
    } catch (e: any) {
      notify(`新建歌单失败：${String(e)}`);
      return false;
    }
  }

  async function deletePlaylist(pl: Playlist) {
    if (!pl.listId) return;
    try {
      await kugouClient.deleteUserPlaylist(pl.listId);
      notify(`已删除歌单「${pl.title}」`);
      await loadUserPlaylists();
    } catch (e: any) {
      notify(`删除歌单失败：${String(e)}`);
    }
  }

  async function removeSongFromCurrentPlaylist(song: Song) {
    const pl =
      libraryAssetTab.value === "liked"
        ? likedPlaylistMeta.value
        : selectedPlaylist.value;
    if (!pl || !pl.listId) {
      notify("仅自建云歌单支持移除曲目");
      return;
    }
    try {
      await kugouClient.removeSongFromPlaylist(pl.listId, song);
      playlistSongs.value = playlistSongs.value.filter(
        (s) => s.hash !== song.hash
      );
      if (pl.isLiked) {
        likedSongs.value = likedSongs.value.filter((s) => s.hash !== song.hash);
      }
      if (pl.trackCount > 0) pl.trackCount -= 1;
      notify(`已从「${pl.title}」移除 ${song.title}`);
    } catch (e: any) {
      notify(`移除曲目失败：${String(e)}`);
    }
  }

  async function importExternalPlaylist(nameOrRawText: string, rawTextArg?: string) {
    if (!isLoggedIn.value) {
      isLoginModalOpen.value = true;
      return 0;
    }
    const name = rawTextArg !== undefined ? nameOrRawText : `导入歌单 ${new Date().toLocaleDateString()}`;
    const rawText = rawTextArg !== undefined ? rawTextArg : nameOrRawText;
    const lines = rawText
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter((l) => l.length > 1);
    if (lines.length === 0) return 0;

    await kugouClient.createUserPlaylist(name);
    await loadUserPlaylists();
    const target = userPlaylists.value.find((p) => p.title === name);
    let matchedCount = 0;

    for (const line of lines.slice(0, 30)) {
      try {
        const res = await kugouClient.searchSongs(line, 1, 3);
        if (res.length > 0 && target?.listId) {
          await kugouClient.addSongToPlaylist(target.listId, res[0]);
          matchedCount += 1;
        }
      } catch {
        // 忽略单首匹配异常
      }
    }
    await loadUserPlaylists();
    notify(`歌单「${name}」导入完成，成功匹配 ${matchedCount} 首曲目`);
    return matchedCount;
  }

  function isPlaylistCollected(pl: Playlist): boolean {
    return collectedPlaylists.value.some(
      (p) => p.id === pl.id || p.title === pl.title
    );
  }

  async function toggleCollectPlaylist(pl: Playlist) {
    const already = isPlaylistCollected(pl);
    if (already) {
      collectedRemotePlaylists.value = collectedRemotePlaylists.value.filter(
        (p) => p.id !== pl.id && p.title !== pl.title
      );
      if (isLoggedIn.value) {
        await kugouClient.uncollectPlaylist(pl).catch(() => {});
        loadUserPlaylists();
      }
      notify(`已取消收藏歌单「${pl.title}」`);
    } else {
      collectedRemotePlaylists.value.unshift({
        ...pl,
        isFavorite: true,
        isCreatedByMe: false,
      });
      if (isLoggedIn.value) {
        await kugouClient.collectPlaylist(pl).catch(() => {});
        loadUserPlaylists();
      }
      notify(`已收藏歌单「${pl.title}」并同步至云端`);
    }
  }

  async function addSongToUserPlaylist(listId: number, song: Song) {
    try {
      await kugouClient.addSongToPlaylist(listId, song);
      notify(`已将「${song.title}」添加到歌单`);
      addToPlaylistModalSong.value = null;
      loadUserPlaylists();
    } catch (e: any) {
      notify(`添加歌曲失败：${String(e)}`);
    }
  }

  function isSongLiked(songOrHash: Song | string | null | undefined): boolean {
    if (!songOrHash) return false;
    if (typeof songOrHash === "string") {
      const targetHash = songOrHash.toLowerCase();
      return likedSongs.value.some(
        (s) => (s.hash && s.hash.toLowerCase() === targetHash) || s.id === songOrHash
      );
    }
    return likedSongs.value.some(
      (s) =>
        (s.hash && s.hash.toLowerCase() === songOrHash.hash.toLowerCase()) ||
        (s.id && s.id === songOrHash.id)
    );
  }

  async function toggleFavoriteSong(song: Song | null) {
    if (!song) return;
    const idx = likedSongs.value.findIndex(
      (s) =>
        (s.hash && s.hash.toLowerCase() === song.hash.toLowerCase()) ||
        (s.id && s.id === song.id)
    );
    if (idx >= 0) {
      likedSongs.value.splice(idx, 1);
      if (likedPlaylistMeta.value?.listId) {
        kugouClient
          .removeSongFromPlaylist(likedPlaylistMeta.value.listId, song)
          .catch(() => {});
      }
      notify(`已从「我喜欢的音乐」移除 ${song.title}`);
    } else {
      likedSongs.value.unshift(song);
      if (likedPlaylistMeta.value?.listId) {
        kugouClient
          .addSongToPlaylist(likedPlaylistMeta.value.listId, song)
          .catch(() => {});
      }
      notify(`♥ 已将「${song.title}」加入我喜欢的音乐`);
    }
  }

  async function loadFollowedArtists() {
    if (!isLoggedIn.value) return;
    try {
      followedArtists.value = await kugouClient.getFollowedArtists();
    } catch (e) {
      console.warn("loadFollowedArtists error:", e);
    }
  }

  async function toggleFollowArtist(artist: FollowedArtist) {
    const nextState = !artist.isFollowed;
    try {
      await kugouClient.toggleFollowArtist(artist.artistId, nextState);
      artist.isFollowed = nextState;
      if (nextState) {
        if (
          !followedArtists.value.some((a) => a.artistId === artist.artistId)
        ) {
          followedArtists.value.unshift({ ...artist, isFollowed: true });
        }
        notify(`已关注歌手「${artist.name}」`);
      } else {
        followedArtists.value = followedArtists.value.filter(
          (a) => a.artistId !== artist.artistId
        );
        notify(`已取消关注「${artist.name}」`);
      }
    } catch (e: any) {
      notify(`操作失败：${String(e)}`);
    }
  }

  async function loadPurchasedAssets() {
    if (!isLoggedIn.value) return;
    try {
      const [albums, songs] = await Promise.all([
        kugouClient.getPurchasedAlbums().catch(() => []),
        kugouClient.getPurchasedSongs().catch(() => []),
      ]);
      purchasedAlbums.value = albums;
      purchasedSongs.value = songs;
    } catch (e) {
      console.warn("loadPurchasedAssets error:", e);
    }
  }

  async function openArtistDetail(
    artistId: string,
    fallbackName = "",
    fallbackAvatar = ""
  ) {
    activeTab.value = "library";
    libraryDetailMode.value = "artist";
    artistDetailSubTab.value = "songs";
    isNowPlayingOpen.value = false;
    isSearchPaletteOpen.value = false;
    isLoadingArtist.value = true;

    activeArtistDetail.value = {
      artistId: artistId || "0",
      targetUserId: "",
      name: fallbackName || "歌手主页",
      avatarUrl: fallbackAvatar,
      songCount: 0,
      albumCount: 0,
      fansCount: 0,
      isFollowed: followedArtists.value.some(
        (a) => a.artistId === artistId || a.name === fallbackName
      ),
      intro: "",
    };

    try {
      if (artistId && artistId !== "0") {
        const [detail, songs, albums] = await Promise.all([
          kugouClient.getArtistDetail(artistId).catch(() => null),
          kugouClient.getArtistSongs(artistId, artistSongsSort.value, 1),
          kugouClient.getArtistAlbums(artistId, 1, "new").catch(() => []),
        ]);
        if (detail) {
          detail.isFollowed = followedArtists.value.some(
            (a) => a.artistId === detail.artistId || a.name === detail.name
          );
          activeArtistDetail.value = detail;
        }
        activeArtistSongs.value = songs;
        activeArtistAlbums.value = albums;
      } else if (fallbackName) {
        const [songs, albums] = await Promise.all([
          kugouClient.searchSongs(fallbackName, 1, 40),
          kugouClient.searchAlbums(fallbackName).catch(() => []),
        ]);
        activeArtistSongs.value = songs;
        activeArtistAlbums.value = albums;
        if (songs[0]?.coverUrl && activeArtistDetail.value) {
          activeArtistDetail.value.avatarUrl = songs[0].coverUrl;
          activeArtistDetail.value.songCount = songs.length;
          activeArtistDetail.value.albumCount = albums.length;
        }
      }
    } catch (e: any) {
      notify(`加载歌手作品失败：${String(e)}`);
    } finally {
      isLoadingArtist.value = false;
    }
  }

  async function openAlbumDetail(
    albumId: string,
    fallbackName = "",
    fallbackArtist = "",
    fallbackCover = ""
  ) {
    activeTab.value = "library";
    libraryDetailMode.value = "album";
    isNowPlayingOpen.value = false;
    isSearchPaletteOpen.value = false;
    isLoadingAlbum.value = true;

    activeAlbumDetail.value = {
      albumId: albumId || "0",
      albumName: fallbackName || "专辑详情",
      artistName: fallbackArtist,
      artistId: "",
      coverUrl: fallbackCover,
      publishDate: "",
      publishCompany: "酷狗概念版数字发行",
      songCount: 0,
      intro: "",
    };

    try {
      if (albumId && albumId !== "0") {
        const [detail, songs] = await Promise.all([
          kugouClient.getAlbumDetail(albumId).catch(() => null),
          kugouClient.getAlbumSongs(albumId),
        ]);
        if (detail) {
          activeAlbumDetail.value = {
            ...detail,
            coverUrl:
              detail.coverUrl || fallbackCover || songs[0]?.coverUrl || "",
            songCount: songs.length || detail.songCount,
          };
        } else if (activeAlbumDetail.value) {
          activeAlbumDetail.value.songCount = songs.length;
          if (!activeAlbumDetail.value.coverUrl && songs[0]?.coverUrl) {
            activeAlbumDetail.value.coverUrl = songs[0].coverUrl;
          }
        }
        activeAlbumSongs.value = songs;
      } else if (fallbackName) {
        const songs = await kugouClient.searchSongs(
          `${fallbackArtist} ${fallbackName}`.trim(),
          1,
          25
        );
        activeAlbumSongs.value = songs;
      }
    } catch (e: any) {
      notify(`加载专辑曲目失败：${String(e)}`);
    } finally {
      isLoadingAlbum.value = false;
    }
  }

  // ================= 10. 多维搜索执行 =================
  async function executeSearch(kw?: string) {
    const target = (kw !== undefined ? kw : searchKeyword.value).trim();
    if (!target) return;
    searchKeyword.value = target;
    activeTab.value = "search";
    isSearchPaletteOpen.value = false;
    isNowPlayingOpen.value = false;

    searchHistory.value = [
      target,
      ...searchHistory.value.filter((item) => item !== target),
    ].slice(0, 14);
    localStorage.setItem(
      "bm_search_history",
      JSON.stringify(searchHistory.value)
    );

    isSearching.value = true;
    try {
      const [songs, playlists, albums, artists] = await Promise.all([
        kugouClient.searchSongs(target, 1, 45).catch(() => []),
        kugouClient.searchPlaylists(target).catch(() => []),
        kugouClient.searchAlbums(target).catch(() => []),
        kugouClient.searchArtists(target).catch(() => []),
      ]);
      searchResultsSongs.value = songs;
      searchResultsPlaylists.value = playlists;
      searchResultsAlbums.value = albums;
      searchResultsArtists.value = artists;
    } catch (e: any) {
      notify(`搜索失败：${String(e)}`);
    } finally {
      isSearching.value = false;
    }
  }

  function clearSearchHistory() {
    searchHistory.value = [];
    localStorage.removeItem("bm_search_history");
  }

  // ================= 11. 播放调度、伴奏切换、评论楼中楼与本地下载 =================
  async function playSongFromList(
    song: Song,
    list?: Song[],
    queueTitle = "当前播放列表"
  ) {
    ensureWebAudioGraph();
    if (audioCtx && audioCtx.state === "suspended") {
      await audioCtx.resume();
    }

    // 切歌前上报上一首已播放时长 (CSCC end)
    if (currentSong.value && currentSongPlayStartWallMs > 0) {
      const elapsed = Date.now() - currentSongPlayStartWallMs;
      if (elapsed >= 10_000) {
        kugouClient
          .reportPlaybackTelemetry(currentSong.value, false, elapsed)
          .catch(() => {});
      }
    }

    if (list && list.length > 0) {
      playQueue.value = [...list];
      if (!recentPlayQueues.value.some((q) => q.title === queueTitle)) {
        recentPlayQueues.value.unshift({
          title: queueTitle,
          songs: list.slice(0, 30),
        });
        if (recentPlayQueues.value.length > 5) {
          recentPlayQueues.value.pop();
        }
      }
      const idx = list.findIndex(
        (s) =>
          (s.hash && s.hash === song.hash) || (s.id && s.id === song.id)
      );
      currentIndex.value = idx >= 0 ? idx : 0;
    } else {
      const existingIdx = playQueue.value.findIndex(
        (s) =>
          (s.hash && s.hash === song.hash) || (s.id && s.id === song.id)
      );
      if (existingIdx >= 0) {
        currentIndex.value = existingIdx;
      } else {
        playQueue.value.push(song);
        currentIndex.value = playQueue.value.length - 1;
      }
    }

    recentHistorySongs.value = [
      song,
      ...recentHistorySongs.value.filter((s) => s.hash !== song.hash),
    ].slice(0, 100);

    await loadAndPlayCurrentSong(0);
  }

  async function loadAndPlayCurrentSong(seekMs = 0) {
    const song = currentSong.value;
    if (!song) return;

    isBuffering.value = true;
    currentPlaybackInfo.value = null;
    durationMs.value = song.durationMs || 0;
    extractCoverColors(song.coverUrl);

    loadLyricForSong(song);
    loadCommentsForSong(song, commentSortType.value);

    try {
      const pb = ktvAccompanimentMode.value
        ? await kugouClient
            .resolveSongAccompaniment(song)
            .catch(() => kugouClient.resolveSongUrl(song, preferredQuality.value))
        : await kugouClient.resolveSongUrl(song, preferredQuality.value);

      currentPlaybackInfo.value = pb;
      audio.src = pb.url;
      currentSongPlayStartWallMs = Date.now();

      // 触发 CSCC start 与听歌足迹上报
      kugouClient.reportPlaybackTelemetry(song, true, 0).catch(() => {});

      const startAtMs =
        seekMs > 0
          ? seekMs
          : climaxModeEnabled.value && pb.climaxStartMs > 0
          ? pb.climaxStartMs
          : 0;

      if (startAtMs > 0) {
        audio.currentTime = startAtMs / 1000;
      }
      await audio.play();
      if (pb.isFreePart) {
        notify("当前为试听片段（登录并签到概念版 VIP 即可解锁整首无损）");
      }
    } catch (e: any) {
      isBuffering.value = false;
      notify(String(e));
    }
  }

  async function toggleKtvAccompaniment() {
    ktvAccompanimentMode.value = !ktvAccompanimentMode.value;
    if (currentSong.value) {
      const pos = currentPositionMs.value;
      await loadAndPlayCurrentSong(pos);
      notify(
        ktvAccompanimentMode.value
          ? "已切换至 KTV 伴奏音轨模式"
          : "已恢复原唱人声模式"
      );
    }
  }

  async function openSongDetailModal(song: Song) {
    songDetailModalSong.value = song;
    activeSongDetailInfo.value = null;
    isLoadingSongDetail.value = true;
    try {
      activeSongDetailInfo.value = await kugouClient.getSongDetailInfo(song);
    } catch {
      activeSongDetailInfo.value = null;
    } finally {
      isLoadingSongDetail.value = false;
    }
  }

  async function loadLyricForSong(song: Song) {
    isLoadingLyric.value = true;
    try {
      currentLyric.value = await kugouClient.getSongLyric(song);
    } catch {
      currentLyric.value = {
        lines: [],
        rawText: "",
        isKrc: false,
        hasTranslation: false,
        hasPhonetic: false,
      };
    } finally {
      isLoadingLyric.value = false;
    }
  }

  async function loadCommentsForSong(
    song: Song | null = currentSong.value,
    sort: "hot" | "new" = "hot"
  ) {
    if (!song) return;
    commentSortType.value = sort;
    expandedFloorCommentId.value = null;
    isLoadingComments.value = true;
    try {
      activeSongComments.value = await kugouClient.getSongComments(
        song,
        sort,
        1
      );
    } catch {
      activeSongComments.value = [];
    } finally {
      isLoadingComments.value = false;
    }
  }

  async function toggleCommentFloorReplies(cmt: SongComment) {
    if (expandedFloorCommentId.value === cmt.id) {
      expandedFloorCommentId.value = null;
      return;
    }
    expandedFloorCommentId.value = cmt.id;
    if (floorRepliesMap[cmt.id]?.length || !currentSong.value) return;
    isLoadingFloorReplies.value = true;
    try {
      floorRepliesMap[cmt.id] = await kugouClient.getCommentFloorReplies(
        currentSong.value,
        cmt.id,
        cmt.specialChildId,
        1
      );
    } catch {
      floorRepliesMap[cmt.id] = [];
    } finally {
      isLoadingFloorReplies.value = false;
    }
  }

  async function toggleLikeComment(cmt: SongComment) {
    if (!currentSong.value) return;
    const nextLike = !cmt.isLiked;
    cmt.isLiked = nextLike;
    cmt.likeCount = Math.max(0, cmt.likeCount + (nextLike ? 1 : -1));
    await kugouClient
      .likeSongComment(currentSong.value, cmt.id, nextLike)
      .catch(() => {});
  }

  async function submitSongComment(content: string, replyToId?: string) {
    if (!currentSong.value) return false;
    if (!isLoggedIn.value) {
      isLoginModalOpen.value = true;
      return false;
    }
    try {
      const created = await kugouClient.postSongComment(
        currentSong.value,
        content,
        replyToId
      );
      if (replyToId && floorRepliesMap[replyToId]) {
        floorRepliesMap[replyToId].unshift(created);
      } else {
        activeSongComments.value.unshift(created);
      }
      notify("评论已发布");
      return true;
    } catch (e: any) {
      notify(`发表评论失败：${String(e)}`);
      return false;
    }
  }

  async function togglePlayPause() {
    ensureWebAudioGraph();
    if (audioCtx && audioCtx.state === "suspended") {
      await audioCtx.resume();
    }
    if (!currentSong.value) {
      if (recommendSongs.value.length > 0) {
        await playSongFromList(recommendSongs.value[0], recommendSongs.value);
      }
      return;
    }
    if (!audio.src) {
      await loadAndPlayCurrentSong(currentPositionMs.value);
      return;
    }
    if (audio.paused) {
      await audio.play();
    } else {
      audio.pause();
    }
  }

  async function playNext(fromAutoEnded = false) {
    const len = playQueue.value.length;
    if (len === 0) return;

    if (fromAutoEnded && playMode.value === "repeat_one") {
      audio.currentTime = 0;
      await audio.play();
      return;
    }

    if (playMode.value === "shuffle" && len > 1) {
      let nextIdx = currentIndex.value;
      while (nextIdx === currentIndex.value) {
        nextIdx = Math.floor(Math.random() * len);
      }
      currentIndex.value = nextIdx;
    } else {
      currentIndex.value = (currentIndex.value + 1) % len;
    }
    await loadAndPlayCurrentSong(0);
  }

  async function playPrev() {
    const len = playQueue.value.length;
    if (len === 0) return;
    currentIndex.value = (currentIndex.value - 1 + len) % len;
    await loadAndPlayCurrentSong(0);
  }

  function seekToMs(ms: number) {
    if (!audio.src) return;
    const clamped = Math.max(0, Math.min(ms, durationMs.value || ms));
    audio.currentTime = clamped / 1000;
    currentPositionMs.value = clamped;
  }

  function setVolume(val: number) {
    const v = Math.max(0, Math.min(1, val));
    volume.value = v;
    audio.volume = v;
  }

  async function switchAudioQuality(q: AudioQualityValue) {
    preferredQuality.value = q;
    localStorage.setItem("bm_pref_quality", q);
    if (currentSong.value) {
      const resumeMs = currentPositionMs.value;
      await loadAndPlayCurrentSong(resumeMs);
      if (currentPlaybackInfo.value) {
        notify(
          `已切换至 ${currentPlaybackInfo.value.qualityLabel} (${currentPlaybackInfo.value.techSpecText})`
        );
      }
    }
  }

  function cyclePlayMode() {
    const order: PlayMode[] = ["repeat_all", "repeat_one", "shuffle"];
    const currIdx = order.indexOf(playMode.value);
    const next = order[(currIdx >= 0 ? currIdx + 1 : 0) % order.length];
    playMode.value = next;
    const labelMap: Record<PlayMode, string> = {
      repeat_all: "已切换为列表循环",
      repeat_one: "已切换为单曲循环",
      shuffle: "已切换为随机播放",
      sequence: "已切换为顺序播放",
      loop_all: "已切换为列表循环",
      loop_one: "已切换为单曲循环",
    };
    notify(labelMap[next]);
  }

  function openSongContextMenu(e: MouseEvent, song: Song, _sourcePlaylist?: any) {
    e.preventDefault();
    e.stopPropagation();
    contextMenu.x = Math.min(e.clientX, window.innerWidth - 250);
    contextMenu.y = Math.min(e.clientY, window.innerHeight - 330);
    contextMenu.song = song;
    contextMenu.visible = true;
  }

  function closeSongContextMenu() {
    contextMenu.visible = false;
  }

  async function triggerDownloadSong(
    song: Song,
    customQuality?: AudioQualityValue
  ) {
    if (!downloadFeatureUnlocked.value) {
      notify("请先前往「设置 → 存储缓存与功能管控」开启「音乐下载与歌词保存」");
      return;
    }
    const targetQuality = customQuality || downloadQuality.value;
    isDownloadingSong.value = true;
    downloadModalSong.value = null;
    notify(`正在下载「${song.title}」并二进制内嵌 1000P 封面与歌词标签...`);
    try {
      const res = await kugouClient.downloadSong(
        song,
        targetQuality,
        downloadMusicDir.value,
        downloadLrcDir.value,
        exportStandaloneLrc.value,
        embedCoverAndTags.value,
        embedLyricsInAudio.value,
        bilingualLrcExport.value
      );
      const sizeMb = (res.fileSizeBytes / (1024 * 1024)).toFixed(1) + " MB";
      const record: DownloadHistoryRecord = {
        id: `${song.hash}_${Date.now()}`,
        title: song.title,
        artist: song.artist,
        qualityLabel: res.qualityLabel,
        savedPath: res.savedAudioPath,
        fileSizeText: sizeMb,
        timestampText: new Date().toLocaleTimeString(),
      };
      downloadHistory.value.unshift(record);
      localStorage.setItem(
        "bm_dl_history",
        JSON.stringify(downloadHistory.value.slice(0, 50))
      );
      notify(res.message);
    } catch (e: any) {
      notify(`下载失败：${String(e)}`);
    } finally {
      isDownloadingSong.value = false;
    }
  }

  function clearCacheCategory(
    category: "audio" | "image" | "snapshot" | "lyric" | "all"
  ) {
    if (category === "audio" || category === "all")
      cacheStats.audioStreamMb = 0;
    if (category === "image" || category === "all")
      cacheStats.artworkImageMb = 0;
    if (category === "snapshot" || category === "all")
      cacheStats.playlistSnapshotMb = 0;
    if (category === "lyric" || category === "all")
      cacheStats.lyricAndUrlMb = 0;
    notify("已完成指定缓存分项清理");
  }

  async function handleLoginSuccess(
    newSession: KugouSession,
    newProfile?: AccountProfile | null
  ) {
    session.value = newSession;
    if (newProfile) profile.value = newProfile;
    isLoginModalOpen.value = false;
    notify(`欢迎回来，${newSession.nickname}！`);
    await refreshAccountAndVip(false);
    await loadDailyRecommend();
  }

  async function logout() {
    await kugouClient.logoutAccount().catch(() => {});
    session.value = null;
    profile.value = null;
    vipStatus.value = null;
    userPlaylists.value = [];
    playlistSongs.value = [];
    likedSongs.value = [];
    followedArtists.value = [];
    purchasedAlbums.value = [];
    purchasedSongs.value = [];
    notify("已退出酷狗概念版账号");
  }

  return {
    activeTab,
    isSearchPaletteOpen,
    isNowPlayingOpen,
    isLoginModalOpen,
    isVipCenterOpen,
    isFocusPodOpen,
    isLyricPosterOpen,
    desktopLyricEnabled,
    desktopLyricLocked,
    contextMenu,
    songDetailModalSong,
    activeSongDetailInfo,
    isLoadingSongDetail,
    addToPlaylistModalSong,
    downloadModalSong,
    statusBanner,
    discoverSubView,
    discoverHeroExpanding,
    isGlobalQueueOpen,
    activeRank,
    activeRankSongs,
    isLoadingRankSongs,
    rankSearchKeyword,
    isRankGridModalOpen,
    selectedRankCategory,
    fmChannels,
    selectedFmCategory,
    activeFmChannel,
    fmStationSongs,
    isLoadingFmStation,
    layoutMode,
    windowMaterial,
    colorScheme,
    dynamicCoverTint,
    autoClaimVipOnStartup,
    autoResumeOnStartup,
    closeToTray,
    playerStageStyle,
    playerBgStyle,
    lyricTheme,
    lyricFontSize,
    lyricTranslationScale,
    lyricLineSpacing,
    lyricAlignment,
    lyricBlurEnabled,
    lyricBlurIntensity,
    lyricWordByWord,
    lyricOffsetMs,
    bluetoothDelayMs,
    showTranslation,
    showPhonetic,
    keepScreenAwake,
    climaxModeEnabled,
    ktvAccompanimentMode,
    replayGainEnabled,
    nightLoudnessEnabled,
    crossfadeSeconds,
    playPauseFadeEnabled,
    playbackRate,
    pitchSemitones,
    linkPitchWithSpeed,
    sleepTimerMinutes,
    sleepWaitCurrentSong,
    sleepRemainingSeconds,
    downloadFeatureUnlocked,
    downloadQuality,
    downloadMusicDir,
    downloadLrcDir,
    embedCoverAndTags,
    embedLyricsInAudio,
    exportStandaloneLrc,
    bilingualLrcExport,
    skipExistingFiles,
    downloadHistory,
    isDownloadingSong,
    cacheStats,
    session,
    savedAccounts,
    profile,
    vipStatus,
    isClaimingVip,
    gatewayDiag,
    isDiagnosingGateway,
    isLoggedIn,
    recommendSongs,
    isLoadingRecommend,
    personalFmSongs,
    personalFmIndex,
    isLoadingFm,
    isPersonalFmDrawerOpen,
    currentFmSong,
    nextFmSong,
    newSongPeriod,
    newSongs,
    isLoadingNewSongs,
    rankList,
    isLoadingRanks,
    userPlaylists,
    selectedPlaylist,
    playlistSongs,
    likedSongs,
    recentHistorySongs,
    followedArtists,
    purchasedAlbums,
    purchasedSongs,
    libraryAssetTab,
    libraryDetailMode,
    isLibraryGridView,
    isLoadingLibrary,
    activeArtistDetail,
    activeArtistSongs,
    activeArtistAlbums,
    artistDetailSubTab,
    artistSongsSort,
    isLoadingArtist,
    activeAlbumDetail,
    activeAlbumSongs,
    isLoadingAlbum,
    createdPlaylists,
    collectedPlaylists,
    likedPlaylistMeta,
    searchKeyword,
    searchSuggestions,
    searchCategoryTab,
    hotSearchKeywords,
    searchHistory,
    searchResultsSongs,
    searchResultsPlaylists,
    searchResultsAlbums,
    searchResultsArtists,
    isSearching,
    coverPalette,
    playQueue,
    recentPlayQueues,
    currentIndex,
    playMode,
    currentSong,
    isPlaying,
    isBuffering,
    currentPositionMs,
    durationMs,
    volume,
    playMomentum,
    preferredQuality,
    currentPlaybackInfo,
    currentLyric,
    isLoadingLyric,
    activeSongComments,
    commentSortType,
    isLoadingComments,
    expandedFloorCommentId,
    floorRepliesMap,
    isLoadingFloorReplies,
    eqFrequencies,
    eqGains,
    eqBandTypes,
    eqQFactor,
    eqHighPassHz,
    eqLowPassHz,
    activeEqPreset,
    peqBypass,
    peqMemorySlots,
    viperEffects,
    activeViperEffectId,
    acousticField,
    spectrumBars,
    notify,
    bootstrap,
    refreshAccountAndVip,
    switchAccount,
    removeSavedAccount,
    claimVipDaily,
    resignDeviceAndRefresh,
    runGatewayDiagnostic,
    loadDailyRecommend,
    loadPersonalFm,
    nextPersonalFmTrack,
    loadNewSongExpress,
    loadRankList,
    openRankDetail,
    resolveFmChannelCovers,
    refreshDiscoverAll,
    openDailyRecommendPage,
    openGuessYouLikePage,
    playRankDirectly,
    loadFmChannels,
    loadFmStationSongs,
    loadRecentHistory,
    loadUserPlaylists,
    selectPlaylist,
    createNewPlaylist,
    deletePlaylist,
    removeSongFromCurrentPlaylist,
    importExternalPlaylist,
    isPlaylistCollected,
    toggleCollectPlaylist,
    addSongToUserPlaylist,
    isSongLiked,
    toggleFavoriteSong,
    loadFollowedArtists,
    toggleFollowArtist,
    loadPurchasedAssets,
    openArtistDetail,
    openAlbumDetail,
    executeSearch,
    clearSearchHistory,
    playSongFromList,
    togglePlayPause,
    toggleKtvAccompaniment,
    openSongDetailModal,
    playNext,
    playPrev,
    seekToMs,
    setVolume,
    setPlaybackSpeed,
    setPitchSemitones,
    applySpeedAndPitch,
    startSleepTimer,
    switchAudioQuality,
    cyclePlayMode,
    syncDspChain,
    applyViperEffect,
    setEqPreset,
    updateSingleEqBand,
    savePeqToSlot,
    loadPeqFromSlot,
    resetSinglePeqSlot,
    resetAllPeqSlots,
    loadCommentsForSong,
    toggleCommentFloorReplies,
    toggleLikeComment,
    submitSongComment,
    openSongContextMenu,
    closeSongContextMenu,
    triggerDownloadSong,
    clearCacheCategory,
    handleLoginSuccess,
    logout,
  };
});
