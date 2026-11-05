import { defineStore } from "pinia";
import { ref, computed, watch, reactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { kugouClient, splitSongAuthors } from "../api/kugou";
import type {
  AccountProfile,
  AcousticFieldSettings,
  NavigationHistoryEntry,
  SongAuthor,
  AlbumItem,
  AppColorScheme,
  AudioQualityValue,
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
} from "../types";

export type MainTab = MainNavTab;

function clampVal(n: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, n));
}

export const useAppStore = defineStore("breezeApp", () => {
  // ================= 1. 顶部天际线中枢导航与全局浮层状态 =================
  const activeTab = ref<MainNavTab>("discover");
  const isSearchPaletteOpen = ref(false);
  const isNowPlayingOpen = ref(false);
  const isLoginModalOpen = ref(false);
  const isVipCenterOpen = ref(false);
  const isAccountManagerOpen = ref(false);
  const isFocusPodOpen = ref(false);
  const isLyricPosterOpen = ref(false);
  const isCommentsModalOpen = ref(false);
  const commentsModalSong = ref<Song | null>(null);
  const isMultiArtistModalOpen = ref(false);
  const multiArtistCandidates = ref<SongAuthor[]>([]);
  const multiArtistSongTitle = ref<string>("");
  const statusBanner = ref<string>("");

  // 独立页面返回栈与历史记录
  const navHistoryStack = ref<NavigationHistoryEntry[]>([]);
  const canNavigateBack = computed(() =>
    isNowPlayingOpen.value ||
    navHistoryStack.value.length > 0 ||
    (activeTab.value === "discover" && discoverSubView.value !== "home") ||
    activeTab.value === "artist_detail" ||
    activeTab.value === "album_detail" ||
    activeTab.value === "playlist_detail" ||
    activeTab.value === "search"
  );

  // 心流舱专注统计与环境伴音协同
  const todayDateKey = new Date().toISOString().slice(0, 10);
  const savedFocusDate = localStorage.getItem("bm_focus_date") || "";
  const focusTodayCompletedCount = ref<number>(
    savedFocusDate === todayDateKey ? Number(localStorage.getItem("bm_focus_count") || 0) : 0
  );
  const focusTodayTotalMinutes = ref<number>(
    savedFocusDate === todayDateKey ? Number(localStorage.getItem("bm_focus_mins") || 0) : 0
  );
  const ambientSyncWithMusic = ref<boolean>(
    localStorage.getItem("bm_ambient_sync_music") === "true"
  );
  function recordCompletedFocusSession(minutes: number) {
    const m = Math.max(1, Math.round(minutes));
    focusTodayCompletedCount.value += 1;
    focusTodayTotalMinutes.value += m;
    const key = new Date().toISOString().slice(0, 10);
    localStorage.setItem("bm_focus_date", key);
    localStorage.setItem("bm_focus_count", String(focusTodayCompletedCount.value));
    localStorage.setItem("bm_focus_mins", String(focusTodayTotalMinutes.value));
  }

  // 桌面悬浮穿透歌词栏（默认宽度略大于屏幕宽度的 1/5 ≈ 23%，刚好长于一句较长的歌词且便于全屏自由拖动）
  const defaultCompactLyricWidth = clampVal(
    Math.round((typeof window !== "undefined" && window.screen?.availWidth ? window.screen.availWidth : 1920) * 0.23),
    400,
    520
  );
  const rawSavedLyricWidth = Number(localStorage.getItem("bm_desktop_lyric_width"));
  // 若旧版缓存了 >= 680px 的半屏宽度，自动迁移回紧凑宽度 defaultCompactLyricWidth
  const initialDesktopLyricWidth =
    rawSavedLyricWidth >= 340 && rawSavedLyricWidth <= 650
      ? rawSavedLyricWidth
      : defaultCompactLyricWidth;

  const desktopLyricEnabled = ref<boolean>(
    localStorage.getItem("bm_desktop_lyric") === "true"
  );
  const desktopLyricLocked = ref<boolean>(
    localStorage.getItem("bm_desktop_lyric_locked") === "true"
  );
  const desktopLyricColor = ref<string>(localStorage.getItem("bm_desktop_lyric_color") || "#00e5ff");
  const desktopLyricFontSize = ref<number>(Number(localStorage.getItem("bm_desktop_lyric_size")) || 22);
  const desktopLyricWidth = ref<number>(
    clampVal(initialDesktopLyricWidth, 340, 720)
  );
  const desktopLyricX = ref<number | null>(
    localStorage.getItem("bm_desktop_lyric_x") !== null
      ? Number(localStorage.getItem("bm_desktop_lyric_x"))
      : null
  );
  const desktopLyricY = ref<number | null>(
    localStorage.getItem("bm_desktop_lyric_y") !== null
      ? Number(localStorage.getItem("bm_desktop_lyric_y"))
      : null
  );
  const desktopLyricShowTranslation = ref<boolean>(
    localStorage.getItem("bm_desktop_lyric_trans") !== "false"
  );

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
  const shareModalSong = ref<Song | null>(null);
  const hdArtworkPreviewUrl = ref<string | null>(null);

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
  const libraryDetailMode = ref<LibraryDetailMode>("liked");
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

  const createdOrderIds = ref<string[]>(
    JSON.parse(localStorage.getItem("bm_order_created") || "[]")
  );
  const collectedOrderIds = ref<string[]>(
    JSON.parse(localStorage.getItem("bm_order_collected") || "[]")
  );
  const artistsOrderIds = ref<string[]>(
    JSON.parse(localStorage.getItem("bm_order_artists") || "[]")
  );
  const purchasedOrderIds = ref<string[]>(
    JSON.parse(localStorage.getItem("bm_order_purchased") || "[]")
  );

  function isDefaultCollectPlaylist(p: Playlist): boolean {
    const t = (p.title || p.name || "").trim();
    return t === "默认收藏";
  }

  function applySavedOrder<T>(
    items: T[],
    orderIds: string[],
    getKey: (item: T) => string
  ): T[] {
    if (!orderIds || orderIds.length === 0) return items;
    const orderMap = new Map<string, number>();
    orderIds.forEach((id, idx) => orderMap.set(id, idx));
    return [...items].sort((a, b) => {
      const ia = orderMap.get(getKey(a));
      const ib = orderMap.get(getKey(b));
      if (ia !== undefined && ib !== undefined) return ia - ib;
      if (ia !== undefined) return -1;
      if (ib !== undefined) return 1;
      return 0;
    });
  }

  const createdPlaylists = computed(() => {
    const raw = userPlaylists.value.filter(
      (p) => p.isCreatedByMe && !p.isLiked && !isDefaultCollectPlaylist(p)
    );
    return applySavedOrder(raw, createdOrderIds.value, (p) =>
      String(p.id || p.listId || p.title)
    );
  });
  const collectedPlaylists = computed(() => {
    const cloud = userPlaylists.value.filter(
      (p) => p.isFavorite && !p.isCreatedByMe && !isDefaultCollectPlaylist(p)
    );
    const merged = [...cloud];
    for (const rp of collectedRemotePlaylists.value) {
      if (
        !isDefaultCollectPlaylist(rp) &&
        !merged.some((m) => m.id === rp.id || m.title === rp.title)
      ) {
        merged.push(rp);
      }
    }
    return applySavedOrder(merged, collectedOrderIds.value, (p) =>
      String(p.id || p.globalCollectionId || p.title)
    );
  });
  const likedPlaylistMeta = computed(() =>
    userPlaylists.value.find((p) => p.isLiked || p.title === "我喜欢")
  );

  // ================= 4. 搜索与天际线指令舱状态 =================
  const searchKeyword = ref<string>("");
  const searchSuggestions = ref<string[]>([]);
  const searchCategoryTab = ref<SearchCategoryTab>("all");
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
  // 明暗双主题（幽邃微蓝 abyssal_dark / 水彩浸润 watercolor_light / 跟随系统 system）
  const rawSavedScheme = localStorage.getItem("bm_color_scheme") || "abyssal_dark";
  const initialColorScheme: AppColorScheme =
    rawSavedScheme === "watercolor_light" || rawSavedScheme === "light"
      ? "watercolor_light"
      : rawSavedScheme === "system"
      ? "system"
      : "abyssal_dark";
  const colorScheme = ref<AppColorScheme>(initialColorScheme);

  const systemPrefersDark = ref<boolean>(
    typeof window !== "undefined" && window.matchMedia
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
      : true
  );
  if (typeof window !== "undefined" && window.matchMedia) {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener?.("change", (ev) => {
      systemPrefersDark.value = ev.matches;
    });
  }

  const resolvedColorScheme = computed<"abyssal_dark" | "watercolor_light">(() => {
    if (colorScheme.value === "system") {
      return systemPrefersDark.value ? "abyssal_dark" : "watercolor_light";
    }
    return colorScheme.value === "watercolor_light" ? "watercolor_light" : "abyssal_dark";
  });

  const isDarkTheme = computed(() => resolvedColorScheme.value === "abyssal_dark");

  function syncDocumentThemeAttr() {
    if (typeof document === "undefined") return;
    const isLight = resolvedColorScheme.value === "watercolor_light";
    document.documentElement.dataset.theme = isLight ? "watercolor_light" : "abyssal_dark";
    document.documentElement.style.colorScheme = isLight ? "light" : "dark";
    document.documentElement.style.backgroundColor = isLight ? "#f2f6fc" : "#0a0e17";
  }
  syncDocumentThemeAttr();

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
  const rawSavedBgStyle = localStorage.getItem("bm_bg_style") || "fluid_aurora";
  const playerBgStyle = ref<PlayerBackgroundStyle>(
    rawSavedBgStyle === "deep_oled"
      ? "fluid_aurora"
      : (rawSavedBgStyle as PlayerBackgroundStyle)
  );
  const rawSavedLyricTheme = localStorage.getItem("bm_lyric_theme") || "ice_blue";
  const lyricTheme = ref<LyricThemeId>(
    rawSavedLyricTheme === "dynamic_cover"
      ? "ice_blue"
      : (rawSavedLyricTheme as LyricThemeId)
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
  const keepScreenAwake = ref<boolean>(
    localStorage.getItem("bm_keep_awake") !== "false"
  );

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

  // 四维存储缓存明细统计（对齐 SettingsDownloadAndPermissions.kt，从 Rust 后端实时读取真实磁盘字节）
  const cacheStats = reactive({
    audioStreamMb: 0,
    artworkImageMb: 0,
    playlistSnapshotMb: 0,
    lyricAndUrlMb: 0,
    maxQuotaGb: 4,
  });

  async function refreshCacheStats() {
    try {
      const real = await kugouClient.getCacheStats();
      cacheStats.audioStreamMb = Number((real.audioStreamMb || 0).toFixed(2));
      cacheStats.artworkImageMb = Number((real.artworkImageMb || 0).toFixed(2));
      cacheStats.playlistSnapshotMb = Number((real.playlistSnapshotMb || 0).toFixed(2));
      cacheStats.lyricAndUrlMb = Number((real.lyricAndUrlMb || 0).toFixed(2));
    } catch {
      // ignore in browser preview
    }
  }

  watch(colorScheme, (v) => {
    localStorage.setItem("bm_color_scheme", v);
    syncDocumentThemeAttr();
  });
  watch(resolvedColorScheme, () => {
    syncDocumentThemeAttr();
  });
  watch(autoClaimVipOnStartup, (v) =>
    localStorage.setItem("bm_auto_claim_vip", String(v))
  );
  watch(autoResumeOnStartup, (v) =>
    localStorage.setItem("bm_auto_resume", String(v))
  );
  watch(closeToTray, (v) => {
    localStorage.setItem("bm_close_to_tray", String(v));
    kugouClient.setCloseToTray(v).catch(() => {});
  });
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
  watch(lyricOffsetMs, (v) => {
    localStorage.setItem("bm_lyric_offset_ms", String(v));
    broadcastDesktopLyricState();
  });
  watch(bluetoothDelayMs, (v) => {
    localStorage.setItem("bm_bt_delay_ms", String(v));
    broadcastDesktopLyricState();
  });
  // 跨进程/跨窗口 BroadcastChannel 桌面歌词实时同步通道
  const desktopLyricChannel =
    typeof BroadcastChannel !== "undefined"
      ? new BroadcastChannel("bm_desktop_lyric_channel")
      : null;

  function broadcastDesktopLyricState() {
    if (!desktopLyricChannel) return;
    try {
      desktopLyricChannel.postMessage({
        type: "STATE_SYNC",
        enabled: desktopLyricEnabled.value,
        locked: desktopLyricLocked.value,
        color: desktopLyricColor.value,
        fontSize: desktopLyricFontSize.value,
        width: desktopLyricWidth.value,
        showTranslation: desktopLyricShowTranslation.value,
        isPlaying: isPlaying.value,
        positionMs: currentPositionMs.value,
        durationMs: durationMs.value,
        lyricOffsetMs: lyricOffsetMs.value,
        bluetoothDelayMs: bluetoothDelayMs.value,
        song: currentSong.value
          ? {
              hash: currentSong.value.hash,
              title: currentSong.value.title,
              artist: currentSong.value.artist,
              coverUrl: currentSong.value.coverUrl,
            }
          : null,
        lyric: currentLyric.value
          ? {
              isKrc: currentLyric.value.isKrc,
              hasTranslation: currentLyric.value.hasTranslation,
              lines: currentLyric.value.lines,
            }
          : null,
      });
    } catch {
      // ignore serialization error
    }
  }

  if (desktopLyricChannel) {
    desktopLyricChannel.onmessage = (ev) => {
      const msg = ev.data;
      if (!msg || typeof msg !== "object") return;
      if (msg.type === "CMD_TOGGLE_PLAY") togglePlayPause();
      else if (msg.type === "CMD_PREV") playPrev();
      else if (msg.type === "CMD_NEXT") playNext(false);
      else if (msg.type === "CMD_UNLOCK") {
        desktopLyricLocked.value = false;
      } else if (msg.type === "CMD_LOCK") {
        desktopLyricLocked.value = true;
      } else if (msg.type === "CMD_CLOSE") {
        desktopLyricEnabled.value = false;
      } else if (msg.type === "CMD_SET_COLOR" && msg.color) {
        desktopLyricColor.value = msg.color;
      } else if (msg.type === "CMD_SET_WIDTH" && msg.width) {
        desktopLyricWidth.value = clampVal(Number(msg.width), 340, 720);
      } else if (msg.type === "CMD_REQUEST_SYNC") {
        broadcastDesktopLyricState();
      }
    };
  }

  watch(desktopLyricEnabled, (v) => {
    localStorage.setItem("bm_desktop_lyric", String(v));
    kugouClient
      .setDesktopLyricWindowState(v, desktopLyricLocked.value, desktopLyricWidth.value, 118)
      .catch(() => {});
    broadcastDesktopLyricState();
  });
  watch(desktopLyricLocked, (v) => {
    localStorage.setItem("bm_desktop_lyric_locked", String(v));
    if (desktopLyricEnabled.value) {
      kugouClient
        .setDesktopLyricWindowState(true, v, desktopLyricWidth.value, 118)
        .catch(() => {});
    }
    broadcastDesktopLyricState();
  });
  watch(desktopLyricColor, (v) => {
    localStorage.setItem("bm_desktop_lyric_color", v);
    broadcastDesktopLyricState();
  });
  watch(desktopLyricFontSize, (v) => {
    localStorage.setItem("bm_desktop_lyric_size", String(v));
    broadcastDesktopLyricState();
  });
  watch(desktopLyricWidth, (v) => {
    const clamped = clampVal(v, 340, 720);
    localStorage.setItem("bm_desktop_lyric_width", String(clamped));
    if (desktopLyricEnabled.value) {
      kugouClient
        .setDesktopLyricWindowState(true, desktopLyricLocked.value, clamped, 118)
        .catch(() => {});
    }
    broadcastDesktopLyricState();
  });
  watch(desktopLyricX, (v) => {
    if (v === null) localStorage.removeItem("bm_desktop_lyric_x");
    else localStorage.setItem("bm_desktop_lyric_x", String(Math.round(v)));
  });
  watch(desktopLyricY, (v) => {
    if (v === null) localStorage.removeItem("bm_desktop_lyric_y");
    else localStorage.setItem("bm_desktop_lyric_y", String(Math.round(v)));
  });
  watch(desktopLyricShowTranslation, (v) => {
    localStorage.setItem("bm_desktop_lyric_trans", String(v));
    broadcastDesktopLyricState();
  });
  watch(ambientSyncWithMusic, (v) =>
    localStorage.setItem("bm_ambient_sync_music", String(v))
  );
  watch(climaxModeEnabled, (v) =>
    localStorage.setItem("bm_climax_mode", String(v))
  );
  watch(replayGainEnabled, (v) => {
    localStorage.setItem("bm_replay_gain", String(v));
    if (v && !audioCtx && audio.crossOrigin) {
      ensureWebAudioGraph();
    }
    syncDspChain();
  });
  watch(nightLoudnessEnabled, (v) => {
    localStorage.setItem("bm_night_loudness", String(v));
    if (v && !audioCtx && audio.crossOrigin) {
      ensureWebAudioGraph();
    }
    syncDspChain();
  });
  watch(crossfadeSeconds, (v) => {
    localStorage.setItem("bm_crossfade_sec", String(v));
    if (v <= 0) {
      audio.volume = volume.value;
    }
  });
  watch(playPauseFadeEnabled, (v) =>
    localStorage.setItem("bm_play_fade", String(v))
  );
  watch(keepScreenAwake, (v) => {
    localStorage.setItem("bm_keep_awake", String(v));
    syncScreenWakeLock();
  });
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
  const isSyncingListeningGrade = ref(false);

  // ================= 8. 64-bit 风格母带级 WebAudio DSP 引擎 + 播放连续动量 =================
  const audio = new Audio();
  audio.preload = "auto";
  audio.crossOrigin = "anonymous";

  const playQueue = ref<Song[]>([]);
  const recentPlayQueues = ref<{ id: string; title: string; songs: Song[] }[]>([]);
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
  const savedVolRaw = localStorage.getItem("bm_volume");
  const volume = ref(
    savedVolRaw !== null && Number.isFinite(Number(savedVolRaw))
      ? Math.max(0, Math.min(1, Number(savedVolRaw)))
      : 0.85
  );
  audio.volume = volume.value;

  // Web Screen Wake Lock 屏幕常亮防休眠控制器（对齐 keepScreenAwake 配置）
  let screenWakeLockSentinel: any = null;
  async function syncScreenWakeLock() {
    if (typeof navigator === "undefined" || !("wakeLock" in navigator)) return;
    if (
      keepScreenAwake.value &&
      isPlaying.value &&
      typeof document !== "undefined" &&
      document.visibilityState === "visible"
    ) {
      if (!screenWakeLockSentinel) {
        try {
          screenWakeLockSentinel = await (navigator as any).wakeLock.request("screen");
          screenWakeLockSentinel?.addEventListener?.("release", () => {
            screenWakeLockSentinel = null;
          });
        } catch {
          screenWakeLockSentinel = null;
        }
      }
    } else if (screenWakeLockSentinel) {
      try {
        await screenWakeLockSentinel.release();
      } catch {
        // ignore release error
      }
      screenWakeLockSentinel = null;
    }
  }
  if (typeof document !== "undefined") {
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "visible") {
        syncScreenWakeLock();
      }
    });
  }

  // 跨曲交叉淡入淡出与 HTML5 Audio 包络平滑控制器
  let isCrossfadeTransitioning = false;
  let volumeRampRafId: number | null = null;
  function cancelVolumeRamp() {
    if (volumeRampRafId !== null) {
      cancelAnimationFrame(volumeRampRafId);
      volumeRampRafId = null;
    }
  }
  function rampHtmlAudioVolume(
    fromVol: number,
    toVol: number,
    durationMsVal: number,
    onDone?: () => void
  ) {
    cancelVolumeRamp();
    const startVal = Math.max(0, Math.min(1, fromVol));
    const endVal = Math.max(0, Math.min(1, toVol));
    const totalMs = Math.max(30, durationMsVal);
    const startWall = performance.now();
    audio.volume = startVal;
    const tick = (now: number) => {
      const progress = Math.min(1, (now - startWall) / totalMs);
      audio.volume = startVal + (endVal - startVal) * progress;
      if (progress < 1) {
        volumeRampRafId = requestAnimationFrame(tick);
      } else {
        volumeRampRafId = null;
        onDone?.();
      }
    };
    volumeRampRafId = requestAnimationFrame(tick);
  }

  // 记录单曲播放开始时间戳，供 CSCC 三段式真实听歌时长上报使用
  let currentSongPlayStartWallMs = 0;
  // 播放请求单调递增令牌与多 CDN 备用直链容灾队列（对齐 Android KGM 播放器自动切源机制）
  let activePlayRequestId = 0;
  let currentCandidateUrls: string[] = [];
  let currentCandidateIdx = 0;
  let hasRetriedWithoutCors = false;

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
  watch(isPlaying, (playing) => {
    animatePlayMomentum(playing ? 1 : 0);
    syncScreenWakeLock();
  });

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
  const masterPreampDb = ref<number>(0);
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
    stageDistance: 28,
    bauerCrossfeed: true,
    reverbPreset: "none",
    reverbPreDelayMs: 24,
    reverbWetPercent: 25,
    vocalClarity: 40,
    airHarmonics: 32,
    vinylTubeWarmth: 20,
    stereoWidth: 1.0,
    reverbMix: 0,
    bassBoostDb: 0,
  });

  const spectrumBars = ref<number[]>(new Array(32).fill(6));

  // 实时音频三频律动能量遥测（驱动律动光晕 RHYTHM_HALO 背景与频谱）
  const rhythmTelemetry = reactive({
    lowEnergy: 0,
    midEnergy: 0,
    highEnergy: 0,
    bassEnergy: 0,
    midVocalEnergy: 0,
    highAirEnergy: 0,
    beatPulse: 0,
  });

  let audioCtx: AudioContext | null = null;
  let analyser: AnalyserNode | null = null;
  let highPassFilter: BiquadFilterNode | null = null;
  let lowPassFilter: BiquadFilterNode | null = null;
  let eqFilters: BiquadFilterNode[] = [];
  let subBassFilter: BiquadFilterNode | null = null;
  let transientPunchFilter: BiquadFilterNode | null = null;
  let airShelfFilter: BiquadFilterNode | null = null;
  let vocalPeakFilter: BiquadFilterNode | null = null;
  let deEsserNotchFilter: BiquadFilterNode | null = null;
  let stagePreDelayNode: DelayNode | null = null;
  let reverbConvolverNode: ConvolverNode | null = null;
  let reverbDampingFilter: BiquadFilterNode | null = null;
  let reverbWetGain: GainNode | null = null;
  let stereoSplitter: ChannelSplitterNode | null = null;
  let stereoMerger: ChannelMergerNode | null = null;
  let msLLGain: GainNode | null = null;
  let msLRGain: GainNode | null = null;
  let msRLGain: GainNode | null = null;
  let msRRGain: GainNode | null = null;
  let masterPreampGain: GainNode | null = null;
  let dynamicsCompressor: DynamicsCompressorNode | null = null;
  let lastBuiltReverbSignature = "";
  let rafId: number | null = null;

  function buildReverbImpulseResponse(
    ctx: AudioContext,
    preset: string,
    stageDepthVal: number
  ): AudioBuffer {
    const presetConfig: Record<string, { duration: number; decay: number; earlyBoost: number }> = {
      none: { duration: 0.15, decay: 4.5, earlyBoost: 0.1 },
      small_room: { duration: 0.55, decay: 3.6, earlyBoost: 0.55 },
      medium_room: { duration: 0.95, decay: 3.0, earlyBoost: 0.48 },
      large_room: { duration: 1.45, decay: 2.5, earlyBoost: 0.42 },
      medium_hall: { duration: 1.95, decay: 2.1, earlyBoost: 0.36 },
      large_hall: { duration: 2.75, decay: 1.75, earlyBoost: 0.32 },
      plate: { duration: 1.35, decay: 2.4, earlyBoost: 0.65 },
    };
    const cfg = presetConfig[preset] || presetConfig.small_room;
    const depthScale = 0.8 + (clampVal(stageDepthVal, 0, 100) / 100) * 0.55;
    const durationSec = clampVal(cfg.duration * depthScale, 0.15, 3.2);
    const sampleRate = ctx.sampleRate || 48000;
    const length = Math.max(256, Math.floor(sampleRate * durationSec));
    const impulse = ctx.createBuffer(2, length, sampleRate);
    const left = impulse.getChannelData(0);
    const right = impulse.getChannelData(1);

    // 早期反射抽头 (Early Reflections) + 指数衰减扩散尾音
    const earlyTaps = [0.007, 0.013, 0.019, 0.029, 0.041, 0.053];
    for (const tapSec of earlyTaps) {
      const idxL = Math.floor(tapSec * sampleRate);
      const idxR = Math.floor((tapSec + 0.0023) * sampleRate);
      if (idxL < length) left[idxL] += cfg.earlyBoost * 0.45;
      if (idxR < length) right[idxR] += cfg.earlyBoost * 0.45;
    }

    for (let i = 0; i < length; i++) {
      const t = i / length;
      const env = Math.pow(1 - t, cfg.decay);
      left[i] += (Math.random() * 2 - 1) * env * 0.38;
      right[i] += (Math.random() * 2 - 1) * env * 0.38;
    }
    return impulse;
  }

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

      transientPunchFilter = audioCtx.createBiquadFilter();
      transientPunchFilter.type = "peaking";
      transientPunchFilter.frequency.value = 95;
      transientPunchFilter.Q.value = 1.8;
      transientPunchFilter.gain.value = 0;

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

      // 立体声 Mid-Side 宽度与 Bauer 双耳交叉馈送矩阵
      stereoSplitter = audioCtx.createChannelSplitter(2);
      stereoMerger = audioCtx.createChannelMerger(2);
      msLLGain = audioCtx.createGain();
      msLRGain = audioCtx.createGain();
      msRLGain = audioCtx.createGain();
      msRRGain = audioCtx.createGain();
      msLLGain.gain.value = 1.0;
      msLRGain.gain.value = 0.0;
      msRLGain.gain.value = 0.0;
      msRRGain.gain.value = 1.0;

      // 空间混响卷积支路 (Pre-Delay -> Convolver -> Damping -> WetGain)
      stagePreDelayNode = audioCtx.createDelay(0.25);
      stagePreDelayNode.delayTime.value = 0.024;
      reverbConvolverNode = audioCtx.createConvolver();
      reverbConvolverNode.buffer = buildReverbImpulseResponse(
        audioCtx,
        acousticField.reverbPreset || "small_room",
        acousticField.stageDepth
      );
      lastBuiltReverbSignature = `${acousticField.reverbPreset}_${Math.round(acousticField.stageDepth / 10)}`;
      reverbDampingFilter = audioCtx.createBiquadFilter();
      reverbDampingFilter.type = "lowpass";
      reverbDampingFilter.frequency.value = 7200;
      reverbWetGain = audioCtx.createGain();
      reverbWetGain.gain.value = 0;

      masterPreampGain = audioCtx.createGain();
      masterPreampGain.gain.value = 1.0;

      dynamicsCompressor = audioCtx.createDynamicsCompressor();
      dynamicsCompressor.threshold.value = -14;
      dynamicsCompressor.knee.value = 18;
      dynamicsCompressor.ratio.value = 2.5;
      dynamicsCompressor.attack.value = 0.008;
      dynamicsCompressor.release.value = 0.18;

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
      subBassFilter.connect(transientPunchFilter);
      transientPunchFilter.connect(vocalPeakFilter);
      vocalPeakFilter.connect(deEsserNotchFilter);
      deEsserNotchFilter.connect(airShelfFilter);

      // 接入 M/S 立体声宽度与交叉馈送矩阵
      airShelfFilter.connect(stereoSplitter);
      stereoSplitter.connect(msLLGain, 0);
      stereoSplitter.connect(msLRGain, 0);
      stereoSplitter.connect(msRLGain, 1);
      stereoSplitter.connect(msRRGain, 1);
      msLLGain.connect(stereoMerger, 0, 0);
      msRLGain.connect(stereoMerger, 0, 0);
      msLRGain.connect(stereoMerger, 0, 1);
      msRRGain.connect(stereoMerger, 0, 1);

      // 主干信号 + 卷积空间混响支路
      stereoMerger.connect(masterPreampGain);
      stereoMerger.connect(stagePreDelayNode);
      stagePreDelayNode.connect(reverbConvolverNode);
      reverbConvolverNode.connect(reverbDampingFilter);
      reverbDampingFilter.connect(reverbWetGain);
      reverbWetGain.connect(masterPreampGain);

      masterPreampGain.connect(dynamicsCompressor);
      dynamicsCompressor.connect(analyser);
      analyser.connect(audioCtx.destination);
      syncDspChain();
      startSpectrumLoop();
    } catch (e) {
      console.warn("WebAudio graph fallback:", e);
    }
  }

  function syncDspChain() {
    const bypass = peqBypass.value;
    // 防重叠增益协同 (Overlap Scaling): 当同时开启 EQ 与声场调音时自动按峰值校准防削波余量
    const maxEqBoost = Math.max(0, ...eqGains.value);
    const scale = !bypass && acousticField.enabled ? 0.84 : 1.0;

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
      transientPunchFilter &&
      vocalPeakFilter &&
      deEsserNotchFilter &&
      airShelfFilter &&
      stagePreDelayNode &&
      reverbWetGain &&
      masterPreampGain &&
      dynamicsCompressor
    ) {
      // Fletcher-Munson 夜间小音量等响度曲线动态补偿系数：低音量聆听时自动补偿超低频下潜与极高频空气细节
      const loudnessFactor =
        !bypass && nightLoudnessEnabled.value && volume.value < 0.78
          ? Math.pow(Math.max(0, 1 - volume.value / 0.78), 1.2)
          : 0;
      const nightBassCompDb = loudnessFactor * 6.5;
      const nightAirCompDb = loudnessFactor * 3.8;

      subBassFilter.frequency.value =
        !bypass && nightLoudnessEnabled.value
          ? Math.max(acousticField.bassCrossoverHz, 105)
          : acousticField.bassCrossoverHz;
      const acousticBassDb =
        !bypass && acousticField.enabled
          ? (acousticField.subBassDepth / 100) * 5.8 + (acousticField.bassBoostDb || 0) * 0.65
          : 0;
      subBassFilter.gain.value = bypass
        ? 0
        : Math.min(10.5, acousticBassDb + nightBassCompDb);

      transientPunchFilter.gain.value =
        !bypass && acousticField.enabled
          ? (acousticField.transientPunch / 100) * 3.2
          : 0;

      vocalPeakFilter.gain.value =
        !bypass && acousticField.enabled
          ? (acousticField.vocalClarity / 100) * 4.2 +
            ((acousticField.surroundWidth - 40) / 100) * 1.2
          : 0;

      deEsserNotchFilter.gain.value =
        !bypass && acousticField.enabled
          ? -(acousticField.vocalClarity / 100) * 2.4
          : 0;

      airShelfFilter.frequency.value =
        !bypass && nightLoudnessEnabled.value ? 10000 : 12000;
      const acousticAirDb =
        !bypass && acousticField.enabled
          ? ((acousticField.airHarmonics - acousticField.vinylTubeWarmth * 0.35) /
              100) *
            5.0
          : 0;
      airShelfFilter.gain.value = bypass
        ? 0
        : Math.min(8.0, acousticAirDb + nightAirCompDb);

      const preDelaySec = Math.max(
        0.005,
        Math.min(0.12, (acousticField.reverbPreDelayMs || 24) / 1000 + (acousticField.stageDepth / 100) * 0.025)
      );
      stagePreDelayNode.delayTime.value = preDelaySec;

      // M/S 立体声宽度与 Bauer 双耳交叉馈送增益矩阵计算
      if (msLLGain && msLRGain && msRLGain && msRRGain) {
        if (!bypass && acousticField.enabled) {
          const widthFromSurround = 0.75 + (acousticField.surroundWidth / 100) * 0.75;
          const userStereoWidth = acousticField.stereoWidth ?? 1.0;
          const effectiveW = clampVal(widthFromSurround * userStereoWidth, 0.35, 1.85);
          // L' = 0.5*(1+W)*L + 0.5*(1-W)*R
          let sameCh = 0.5 * (1 + effectiveW);
          let crossCh = 0.5 * (1 - effectiveW);
          if (acousticField.bauerCrossfeed) {
            crossCh += 0.12;
            sameCh *= 0.94;
          }
          msLLGain.gain.value = sameCh;
          msRRGain.gain.value = sameCh;
          msLRGain.gain.value = crossCh;
          msRLGain.gain.value = crossCh;
        } else {
          msLLGain.gain.value = 1.0;
          msRRGain.gain.value = 1.0;
          msLRGain.gain.value = 0.0;
          msRLGain.gain.value = 0.0;
        }
      }

      // 动态更新混响卷积核与高频阻尼 (Stage Distance)
      if (audioCtx && reverbConvolverNode) {
        const sig = `${acousticField.reverbPreset}_${Math.round(acousticField.stageDepth / 10)}`;
        if (sig !== lastBuiltReverbSignature) {
          lastBuiltReverbSignature = sig;
          reverbConvolverNode.buffer = buildReverbImpulseResponse(
            audioCtx,
            acousticField.reverbPreset || "small_room",
            acousticField.stageDepth
          );
        }
      }
      if (reverbDampingFilter) {
        const distVal = clampVal(acousticField.stageDistance ?? 28, 0, 100);
        reverbDampingFilter.frequency.value = Math.max(2400, 11000 - distVal * 72);
      }

      const presetWetMap: Record<string, number> = {
        none: 0,
        small_room: 0.16,
        medium_room: 0.24,
        large_room: 0.32,
        medium_hall: 0.38,
        large_hall: 0.46,
        plate: 0.28,
      };
      const presetBase = presetWetMap[acousticField.reverbPreset] ?? 0;
      const wetSliderScale = clampVal((acousticField.reverbWetPercent ?? 25) / 35, 0.2, 2.2);
      const customMix = (acousticField.reverbMix || 0) / 160;
      const baseWet = Math.max(presetBase * wetSliderScale, customMix);
      reverbWetGain.gain.value =
        !bypass && acousticField.enabled ? Math.min(0.65, baseWet) : 0;

      // Headroom 自动补偿 + ReplayGain / 夜间响度柔化
      const maxTotalBoost = Math.max(
        maxEqBoost,
        subBassFilter.gain.value,
        airShelfFilter.gain.value
      );
      const headroomDb = maxTotalBoost > 3 ? -(maxTotalBoost - 3) * 0.35 : 0;
      const rgDb = replayGainEnabled.value ? -1.2 : 0;
      const totalGainLinear = Math.pow(10, (headroomDb + rgDb + (masterPreampDb.value || 0)) / 20);
      masterPreampGain.gain.value = bypass ? 1.0 : totalGainLinear;

      if (nightLoudnessEnabled.value) {
        dynamicsCompressor.threshold.value = -22;
        dynamicsCompressor.ratio.value = 3.6;
      } else {
        dynamicsCompressor.threshold.value = -12;
        dynamicsCompressor.ratio.value = 2.0;
      }
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
        let lowSum = 0,
          midSum = 0,
          highSum = 0;
        for (let i = 0; i < 32; i++) {
          const val = freqData[i] || 0;
          bars.push(Math.max(6, Math.round((val / 255) * 100)));
          if (i < 6) lowSum += val;
          else if (i < 18) midSum += val;
          else highSum += val;
        }
        spectrumBars.value = bars;
        rhythmTelemetry.lowEnergy = Number((lowSum / (6 * 255)).toFixed(3));
        rhythmTelemetry.midEnergy = Number((midSum / (12 * 255)).toFixed(3));
        rhythmTelemetry.highEnergy = Number((highSum / (14 * 255)).toFixed(3));
        rhythmTelemetry.bassEnergy = rhythmTelemetry.lowEnergy;
        rhythmTelemetry.midVocalEnergy = rhythmTelemetry.midEnergy;
        rhythmTelemetry.highAirEnergy = rhythmTelemetry.highEnergy;
        rhythmTelemetry.beatPulse = Number(
          Math.min(1, rhythmTelemetry.lowEnergy * 1.25).toFixed(3)
        );
      } else if (playMomentum.value > 0.02) {
        spectrumBars.value = spectrumBars.value.map((v) =>
          Math.max(6, Math.round(v * 0.88))
        );
        rhythmTelemetry.lowEnergy *= 0.9;
        rhythmTelemetry.midEnergy *= 0.9;
        rhythmTelemetry.highEnergy *= 0.9;
        rhythmTelemetry.bassEnergy = rhythmTelemetry.lowEnergy;
        rhythmTelemetry.midVocalEnergy = rhythmTelemetry.midEnergy;
        rhythmTelemetry.highAirEnergy = rhythmTelemetry.highEnergy;
        rhythmTelemetry.beatPulse *= 0.88;
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
      // 兼容传统音乐风格预设名称
      pop: [2.4, 3.2, 1.6, -0.6, -1.0, 1.2, 2.8, 3.4, 2.6, 2.0],
      rock: [4.8, 3.8, 1.4, -1.2, -1.8, 0.8, 2.8, 4.2, 4.6, 3.8],
      classical: [0.8, 0.6, 0.2, 0.0, 0.4, 1.2, 2.4, 3.2, 3.6, 4.0],
      jazz: [3.2, 2.4, 1.2, 1.8, -1.2, -1.0, 0.8, 2.2, 3.0, 3.2],
      vocal: [-1.4, -0.6, 0.8, 2.2, 3.4, 4.4, 3.8, -1.8, 1.6, 2.4],
      bass: [6.2, 5.4, 2.6, -1.6, -0.6, 0.6, 1.8, 2.4, 2.8, 3.2],
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

  // 真实播放秒数累加器与播放卡死看门狗（对齐 MakcRe/KuGouMusicApi user_listen_report.js & user_grade_info.js）
  let activeTelemetrySong: Song | null = null;
  let currentTrackPlayedAccumMs = 0;
  let unPersistedListenSec = 0;
  let unSyncedGradeSec = 0;
  let lastWatchdogPosSec = -1;
  let watchdogStallTicks = 0;
  let lastWatchdogRecoverWallMs = 0;

  function settleActiveTrackTelemetry() {
    const prevSong = activeTelemetrySong;
    const playedMs = currentTrackPlayedAccumMs;
    const remSec = unPersistedListenSec;
    activeTelemetrySong = null;
    currentTrackPlayedAccumMs = 0;
    unPersistedListenSec = 0;
    unSyncedGradeSec = 0;

    if (!isLoggedIn.value) return;

    const flushPromise =
      remSec > 0
        ? kugouClient.recordListeningSeconds(remSec).catch(() => {})
        : Promise.resolve();

    if (prevSong && playedMs >= 10000) {
      flushPromise.finally(() => {
        kugouClient
          .reportPlaybackTelemetry(prevSong, false, playedMs)
          .then(() => {
            setTimeout(() => {
              kugouClient
                .syncListeningGrade(false)
                .then((res) => {
                  if (res?.profile) profile.value = res.profile;
                })
                .catch(() => {});
            }, 2000);
          })
          .catch(() => {});
      });
    }
  }

  window.setInterval(() => {
    if (isPlaying.value && currentSong.value && !audio.paused && !audio.ended) {
      currentTrackPlayedAccumMs += 1000;
      unPersistedListenSec += 1;
      unSyncedGradeSec += 1;
      if (profile.value) {
        profile.value.listenDurationSec = (profile.value.listenDurationSec || 0) + 1;
        profile.value.listenDurationMin = Math.floor(profile.value.listenDurationSec / 60);
        const h = Math.floor(profile.value.listenDurationMin / 60);
        const m = profile.value.listenDurationMin % 60;
        profile.value.listenDurationFormatted =
          h > 0 ? `${h} 小时 ${m} 分` : `${m} 分钟`;
      }
      // 每满 10 秒持久化增量秒数至本地缓冲，杜绝重复叠加与关闭丢失
      if (unPersistedListenSec >= 10 && isLoggedIn.value) {
        const secToSave = unPersistedListenSec;
        unPersistedListenSec = 0;
        kugouClient.recordListeningSeconds(secToSave).catch(() => {});
      }
      // 每满 60 秒真实播放仅触发一次 /v2/get_grade_info 增量时长同步（绝不在曲中伪造 CSCC 完播事件）
      if (unSyncedGradeSec >= 60 && isLoggedIn.value) {
        unSyncedGradeSec = 0;
        kugouClient
          .syncListeningGrade(false)
          .then((res) => {
            if (res?.profile) profile.value = res.profile;
          })
          .catch(() => {});
      }

      // 播放中途停滞看门狗 (PlaybackStallWatchdog)：检测 isPlaying=true 但 currentTime 连续 4 秒停滞
      const curSec = audio.currentTime || 0;
      const realDurMs =
        audio.duration && isFinite(audio.duration) && audio.duration > 5
          ? audio.duration * 1000
          : durationMs.value;
      if (curSec > 1 && realDurMs > 10000 && curSec * 1000 < realDurMs - 2500) {
        if (Math.abs(curSec - lastWatchdogPosSec) < 0.04) {
          watchdogStallTicks += 1;
          if (
            watchdogStallTicks >= 4 &&
            Date.now() - lastWatchdogRecoverWallMs > 8000
          ) {
            watchdogStallTicks = 0;
            lastWatchdogRecoverWallMs = Date.now();
            const resumeMs = Math.floor(curSec * 1000);
            console.warn(
              `[PlaybackStallWatchdog] 检测到播放中途停滞于 ${curSec.toFixed(1)}s，正在无缝重连恢复...`
            );
            if (currentCandidateIdx + 1 < currentCandidateUrls.length) {
              currentCandidateIdx += 1;
              audio.src = currentCandidateUrls[currentCandidateIdx];
              audio.currentTime = curSec;
              audio.play().catch(() => {});
            } else {
              loadAndPlayCurrentSong(resumeMs);
            }
          }
        } else {
          lastWatchdogPosSec = curSec;
          watchdogStallTicks = 0;
        }
      }
    } else {
      watchdogStallTicks = 0;
    }
  }, 1000);

  // HTML5 Audio 事件绑定
  audio.addEventListener("timeupdate", () => {
    const posMs = Math.floor(audio.currentTime * 1000);
    currentPositionMs.value = posMs;
    if (desktopLyricEnabled.value) {
      broadcastDesktopLyricState();
    }

    // 副歌速刷模式 (Climax 30s 精华连播)：到达 climaxEndMs 自动切下一首
    if (climaxModeEnabled.value && currentPlaybackInfo.value) {
      const cEnd = currentPlaybackInfo.value.climaxEndMs || 75000;
      if (cEnd > 10000 && posMs >= cEnd) {
        playNext(true);
        return;
      }
    }

    // 跨曲平滑淡出与无缝切歌过渡 (crossfadeSeconds: 1 ~ 8s)
    // 严格要求 audio.duration 已解析且有效，避免因元数据时长偏差导致提前静音或跳歌
    if (
      crossfadeSeconds.value > 0 &&
      !climaxModeEnabled.value &&
      isPlaying.value &&
      audio.duration &&
      isFinite(audio.duration) &&
      audio.duration > 30 &&
      playQueue.value.length > 0
    ) {
      const realDurMs = Math.floor(audio.duration * 1000);
      const fadeWinMs = crossfadeSeconds.value * 1000;
      const remainMs = realDurMs - posMs;
      if (remainMs > 0 && remainMs <= fadeWinMs && !isCrossfadeTransitioning) {
        const envelope = Math.max(0.03, remainMs / fadeWinMs);
        audio.volume = Math.max(0, Math.min(1, volume.value * envelope));
        if (remainMs <= 220) {
          isCrossfadeTransitioning = true;
          playNext(true);
        }
      }
    }
  });
  audio.addEventListener("loadedmetadata", () => {
    if (audio.duration && isFinite(audio.duration)) {
      durationMs.value = Math.floor(audio.duration * 1000);
    }
    applySpeedAndPitch();
    if (desktopLyricEnabled.value) broadcastDesktopLyricState();
  });
  audio.addEventListener("play", () => {
    isPlaying.value = true;
    isBuffering.value = false;
    if (desktopLyricEnabled.value) broadcastDesktopLyricState();
  });
  audio.addEventListener("pause", () => {
    isPlaying.value = false;
    if (desktopLyricEnabled.value) broadcastDesktopLyricState();
  });
  audio.addEventListener("waiting", () => {
    isBuffering.value = true;
  });
  audio.addEventListener("canplay", () => {
    isBuffering.value = false;
  });
  let hasRetriedFallbackQuality = false;
  audio.addEventListener("error", () => {
    const reqId = activePlayRequestId;
    if (!currentSong.value || currentCandidateUrls.length === 0) {
      isBuffering.value = false;
      return;
    }
    const resumePosSec = audio.currentTime > 0 ? audio.currentTime : 0;
    if (currentCandidateIdx + 1 < currentCandidateUrls.length) {
      currentCandidateIdx += 1;
      const nextUrl = currentCandidateUrls[currentCandidateIdx];
      console.warn(
        `[Audio Failover] 主节点加载失败，自动切换备用节点 (${currentCandidateIdx + 1}/${currentCandidateUrls.length}):`,
        nextUrl
      );
      audio.src = nextUrl;
      if (resumePosSec > 0) {
        audio.currentTime = resumePosSec;
      }
      audio.play().catch((err) => {
        if (reqId === activePlayRequestId && err?.name !== "AbortError") {
          console.warn("[Audio Failover] 备用节点 play() 异常:", err);
        }
      });
      return;
    }
    // 若所有节点在 crossOrigin="anonymous" 下均因缺少 CORS 头失败，则自动降级移除 crossOrigin 重试一次
    if (!hasRetriedWithoutCors) {
      hasRetriedWithoutCors = true;
      currentCandidateIdx = 0;
      audio.removeAttribute("crossorigin");
      audio.src = currentCandidateUrls[0];
      if (resumePosSec > 0) {
        audio.currentTime = resumePosSec;
      }
      audio.play().catch(() => {});
      return;
    }
    // 若当前高码率音源流损坏，自动平滑降级一档音质并在原进度续播
    if (!hasRetriedFallbackQuality && currentSong.value) {
      hasRetriedFallbackQuality = true;
      const fallbackOrder: AudioQualityValue[] = ["high", "flac", "320", "128"];
      const curIdx = fallbackOrder.indexOf(preferredQuality.value);
      const fallbackQ = fallbackOrder[Math.min(fallbackOrder.length - 1, Math.max(0, curIdx) + 1)] || "320";
      const songSnapshot = currentSong.value;
      const resumeMs = Math.floor(resumePosSec * 1000);
      kugouClient
        .resolveSongUrl(songSnapshot, fallbackQ)
        .then((pb) => {
          if (reqId !== activePlayRequestId) return;
          currentPlaybackInfo.value = pb;
          const nextCandidates = [pb.proxyUrl, pb.url, ...(pb.backupUrls || [])].filter(
            (u, idx, arr): u is string => Boolean(u) && arr.indexOf(u) === idx
          );
          if (nextCandidates.length > 0) {
            currentCandidateUrls = nextCandidates;
            currentCandidateIdx = 0;
            audio.src = nextCandidates[0];
            if (resumeMs > 0) {
              audio.currentTime = resumeMs / 1000;
            }
            audio.play().catch(() => {});
          }
        })
        .catch(() => {
          isBuffering.value = false;
          notify("当前音源直链节点连接失败，请尝试切换其他曲目");
        });
      return;
    }
    isBuffering.value = false;
    notify("当前音源直链节点连接失败，正在尝试重新加载或切换音质");
  });
  audio.addEventListener("ended", () => {
    settleActiveTrackTelemetry();

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

    // 同步系统托盘关闭策略、真实磁盘缓存统计与独立桌面歌词窗口状态
    kugouClient.setCloseToTray(closeToTray.value).catch(() => {});
    refreshCacheStats();
    if (desktopLyricEnabled.value) {
      kugouClient
        .setDesktopLyricWindowState(
          true,
          desktopLyricLocked.value,
          desktopLyricWidth.value,
          118
        )
        .catch(() => {});
    }
    listen<string>("tray-action", (event) => {
      const cmd = event.payload;
      if (cmd === "toggle_play" || cmd === "toggle") {
        togglePlayPause();
      } else if (cmd === "prev_song" || cmd === "prev") {
        playPrev();
      } else if (cmd === "next_song" || cmd === "next") {
        playNext(false);
      } else if (cmd === "toggle_desktop_lyric" || cmd === "desktop_lyric") {
        desktopLyricEnabled.value = !desktopLyricEnabled.value;
        notify(
          desktopLyricEnabled.value
            ? "已开启系统级桌面悬浮歌词"
            : "已关闭桌面悬浮歌词"
        );
      } else if (cmd === "toggle_lyric_lock") {
        desktopLyricLocked.value = !desktopLyricLocked.value;
        notify(
          desktopLyricLocked.value
            ? "已锁定桌面歌词（鼠标穿透生效，可通过托盘或 Ctrl+Shift+L 解锁）"
            : "已解锁桌面歌词，现在可自由拖动与调节"
        );
      }
    }).catch(() => {});

    // 启动时恢复上次播放队列与进度 (autoResumeOnStartup)
    try {
      const savedQueueRaw = localStorage.getItem("bm_saved_queue");
      const savedIdx = Number(localStorage.getItem("bm_saved_index") ?? -1);
      const savedPosMs = Number(localStorage.getItem("bm_saved_pos_ms") ?? 0);
      if (savedQueueRaw) {
        const parsedQueue = JSON.parse(savedQueueRaw) as Song[];
        if (Array.isArray(parsedQueue) && parsedQueue.length > 0) {
          playQueue.value = parsedQueue;
          currentIndex.value = clampVal(savedIdx >= 0 ? savedIdx : 0, 0, parsedQueue.length - 1);
          const restoredSong = playQueue.value[currentIndex.value];
          if (restoredSong) {
            durationMs.value = restoredSong.durationMs || 210000;
            currentPositionMs.value = Math.max(0, savedPosMs);
            loadLyricForSong(restoredSong);
            if (autoResumeOnStartup.value) {
              setTimeout(() => {
                loadAndPlayCurrentSong(Math.max(0, savedPosMs));
              }, 600);
            }
          }
        }
      }
    } catch {
      // ignore corrupted queue
    }

    loadDailyRecommend();
    loadPersonalFm();
    loadNewSongExpress("CHINESE");
    loadRankList();
    loadFmChannels();
    loadRecentHistory();
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

  async function syncListeningGradeCloud(forceReport = true) {
    if (!isLoggedIn.value) {
      isLoginModalOpen.value = true;
      return;
    }
    if (isSyncingListeningGrade.value) return;
    isSyncingListeningGrade.value = true;
    try {
      if (unPersistedListenSec > 0) {
        const remSec = unPersistedListenSec;
        unPersistedListenSec = 0;
        unSyncedGradeSec = 0;
        await kugouClient.recordListeningSeconds(remSec).catch(() => {});
      }
      const [syncRes, vip] = await Promise.all([
        kugouClient.syncListeningGrade(forceReport),
        kugouClient.getVipStatus().catch(() => null),
      ]);
      if (syncRes?.profile) {
        profile.value = syncRes.profile;
      }
      if (vip) {
        vipStatus.value = vip;
      }
      notify(syncRes?.message || "已校准云端听歌等级与累计时长");
    } catch (e: any) {
      notify(`同步听歌等级失败：${String(e)}`);
    } finally {
      isSyncingListeningGrade.value = false;
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
      }
    } catch (e) {
      console.warn("loadDailyRecommend error:", e);
    } finally {
      isLoadingRecommend.value = false;
    }
  }

  const fmHistoryHashes = ref<string[]>(
    JSON.parse(localStorage.getItem("bm_fm_history_hashes") || "[]")
  );
  function recordFmHistoryHashes(songs: Song[]) {
    const incoming = songs
      .map((s) => (s.hash || "").toUpperCase())
      .filter((h) => h.length > 0);
    if (incoming.length === 0) return;
    const merged = Array.from(new Set([...incoming, ...fmHistoryHashes.value])).slice(0, 240);
    fmHistoryHashes.value = merged;
    localStorage.setItem("bm_fm_history_hashes", JSON.stringify(merged));
  }

  async function loadPersonalFm(append = false) {
    isLoadingFm.value = true;
    try {
      const currentExclude = Array.from(
        new Set([
          ...personalFmSongs.value.map((s) => (s.hash || "").toUpperCase()),
          ...fmHistoryHashes.value.slice(0, 120),
        ])
      ).filter(Boolean);
      const songs = await kugouClient.getPersonalFm(currentExclude);
      if (songs.length > 0) {
        recordFmHistoryHashes(songs);
        if (append) {
          const seen = new Set(personalFmSongs.value.map((s) => (s.hash || "").toUpperCase()));
          const unique = songs.filter((s) => !seen.has((s.hash || "").toUpperCase()));
          personalFmSongs.value = [...personalFmSongs.value, ...(unique.length > 0 ? unique : songs)];
          notify(`已为猜你喜欢随心听无缝焕新 ${unique.length || songs.length} 首歌曲`);
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

  function scrollStageToTop() {
    if (typeof document === "undefined") return;
    const reset = () => {
      const stage = document.querySelector(".skyline-stage-viewport") as HTMLElement | null;
      if (stage) stage.scrollTop = 0;
    };
    reset();
    if (typeof requestAnimationFrame !== "undefined") {
      requestAnimationFrame(reset);
    }
    setTimeout(reset, 24);
  }

  async function openRankDetail(rank: RankItem) {
    activeRank.value = rank;
    rankSearchKeyword.value = "";
    discoverSubView.value = "rank";
    activeTab.value = "discover";
    isNowPlayingOpen.value = false;
    isRankGridModalOpen.value = false;
    scrollStageToTop();
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
    discoverHeroExpanding.value = "none";
    discoverSubView.value = "daily";
    scrollStageToTop();
  }

  function openGuessYouLikePage() {
    if (discoverSubView.value === "fm_detail") return;
    if (personalFmSongs.value.length === 0) {
      loadPersonalFm(false);
    }
    discoverHeroExpanding.value = "none";
    discoverSubView.value = "fm_detail";
    scrollStageToTop();
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
    try {
      const list = await kugouClient.getRecentHistory();
      if (list.length > 0 || recentHistorySongs.value.length === 0) {
        recentHistorySongs.value = list;
      }
    } catch (e) {
      console.warn("loadRecentHistory error:", e);
    }
  }

  async function loadUserPlaylists() {
    if (!isLoggedIn.value) return;
    isLoadingLibrary.value = true;
    try {
      const rawList = await kugouClient.getUserPlaylists();
      const list = rawList.filter((p) => !isDefaultCollectPlaylist(p));
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
        await selectPlaylist(preferred, false);
      }
    } catch (e: any) {
      notify(`获取歌单失败：${String(e)}`);
    } finally {
      isLoadingLibrary.value = false;
    }
  }

  // 独立详情页 SWR 内存秒开缓存 (Playlist / Artist / Album)
  const playlistSwrCache = new Map<string, { playlist: Playlist; songs: Song[] }>();
  const artistSwrCache = new Map<
    string,
    { detail: FollowedArtist; songs: Song[]; albums: AlbumItem[] }
  >();
  const albumSwrCache = new Map<string, { detail: AlbumItem; songs: Song[] }>();

  function getStageScrollTop(): number {
    if (typeof document === "undefined") return 0;
    const stage = document.querySelector(".skyline-stage-viewport") as HTMLElement | null;
    return stage ? stage.scrollTop : 0;
  }

  function restoreStageScrollTop(scrollTop = 0) {
    if (typeof document === "undefined") return;
    const apply = () => {
      const stage = document.querySelector(".skyline-stage-viewport") as HTMLElement | null;
      if (stage) stage.scrollTop = scrollTop;
    };
    apply();
    if (typeof requestAnimationFrame !== "undefined") {
      requestAnimationFrame(apply);
    }
    setTimeout(apply, 28);
  }

  function pushNavHistory() {
    const entry: NavigationHistoryEntry = {
      tab: activeTab.value,
      discoverSubView: discoverSubView.value,
      libraryAssetTab: libraryAssetTab.value,
      libraryDetailMode: libraryDetailMode.value,
      artistId: activeArtistDetail.value?.artistId,
      albumId: activeAlbumDetail.value?.albumId,
      playlistId: selectedPlaylist.value?.id || selectedPlaylist.value?.globalCollectionId,
      returnToNowPlaying: isNowPlayingOpen.value,
      scrollTop: getStageScrollTop(),
    };
    const top = navHistoryStack.value[navHistoryStack.value.length - 1];
    if (
      !top ||
      top.tab !== entry.tab ||
      top.discoverSubView !== entry.discoverSubView ||
      top.artistId !== entry.artistId ||
      top.albumId !== entry.albumId ||
      top.playlistId !== entry.playlistId ||
      top.returnToNowPlaying !== entry.returnToNowPlaying
    ) {
      navHistoryStack.value.push(entry);
      if (navHistoryStack.value.length > 24) {
        navHistoryStack.value.shift();
      }
    }
  }

  function navigateBack() {
    if (isNowPlayingOpen.value) {
      isNowPlayingOpen.value = false;
      return;
    }
    const popped = navHistoryStack.value.pop();
    if (popped) {
      if (popped.returnToNowPlaying) {
        activeTab.value = popped.tab;
        if (popped.discoverSubView) discoverSubView.value = popped.discoverSubView;
        isNowPlayingOpen.value = true;
        return;
      }
      activeTab.value = popped.tab;
      if (popped.discoverSubView) discoverSubView.value = popped.discoverSubView;
      if (popped.libraryAssetTab) libraryAssetTab.value = popped.libraryAssetTab;
      if (popped.libraryDetailMode) libraryDetailMode.value = popped.libraryDetailMode;
      if (popped.tab === "artist_detail" && popped.artistId) {
        const cachedArtist = artistSwrCache.get(popped.artistId);
        if (cachedArtist) {
          activeArtistDetail.value = cachedArtist.detail;
          activeArtistSongs.value = cachedArtist.songs;
          activeArtistAlbums.value = cachedArtist.albums;
        }
      } else if (popped.tab === "album_detail" && popped.albumId) {
        const cachedAlbum = albumSwrCache.get(popped.albumId);
        if (cachedAlbum) {
          activeAlbumDetail.value = cachedAlbum.detail;
          activeAlbumSongs.value = cachedAlbum.songs;
        }
      } else if (popped.tab === "playlist_detail" && popped.playlistId) {
        const cachedPl = playlistSwrCache.get(popped.playlistId);
        if (cachedPl) {
          selectedPlaylist.value = cachedPl.playlist;
          playlistSongs.value = cachedPl.songs;
        }
      }
      restoreStageScrollTop(popped.scrollTop || 0);
      return;
    }
    if (activeTab.value === "discover" && discoverSubView.value !== "home") {
      discoverSubView.value = "home";
      scrollStageToTop();
      return;
    }
    if (
      activeTab.value === "artist_detail" ||
      activeTab.value === "album_detail" ||
      activeTab.value === "playlist_detail" ||
      activeTab.value === "search"
    ) {
      activeTab.value = "discover";
      discoverSubView.value = "home";
      scrollStageToTop();
    }
  }

  function openSoundMixerFromPlayer() {
    pushNavHistory();
    isNowPlayingOpen.value = false;
    activeTab.value = "tuner";
    scrollStageToTop();
  }

  async function selectPlaylist(pl: Playlist, openStandalonePage = true) {
    if (openStandalonePage) {
      pushNavHistory();
      activeTab.value = "playlist_detail";
      isNowPlayingOpen.value = false;
      isSearchPaletteOpen.value = false;
      scrollStageToTop();
    }
    const gid = pl.globalCollectionId || pl.id;
    const cached = gid ? playlistSwrCache.get(gid) : undefined;
    if (cached) {
      selectedPlaylist.value = { ...cached.playlist, ...pl };
      playlistSongs.value = cached.songs;
      isLoadingLibrary.value = false;
    } else {
      selectedPlaylist.value = pl;
      playlistSongs.value = [];
      isLoadingLibrary.value = true;
    }
    libraryDetailMode.value = "playlist";
    try {
      const [songs, metaInfo] = await Promise.all([
        kugouClient.getPlaylistSongs(gid),
        !pl.isCreatedByMe && gid
          ? kugouClient.getPlaylistInfo(gid).catch(() => null)
          : Promise.resolve(null),
      ]);
      playlistSongs.value = songs;
      if (metaInfo && selectedPlaylist.value) {
        selectedPlaylist.value = {
          ...selectedPlaylist.value,
          title: metaInfo.title || selectedPlaylist.value.title,
          name: metaInfo.title || selectedPlaylist.value.name,
          description: metaInfo.description || selectedPlaylist.value.description,
          intro: metaInfo.intro || metaInfo.description || selectedPlaylist.value.intro,
          coverUrl: metaInfo.coverUrl || selectedPlaylist.value.coverUrl || songs[0]?.coverUrl || "",
          creator: metaInfo.creator || selectedPlaylist.value.creator,
          playCount: metaInfo.playCount || selectedPlaylist.value.playCount,
          trackCount: Math.max(songs.length, metaInfo.trackCount || 0, selectedPlaylist.value.trackCount || 0),
        };
      } else if (selectedPlaylist.value) {
        if (selectedPlaylist.value.trackCount < songs.length) {
          selectedPlaylist.value.trackCount = songs.length;
        }
        if (!selectedPlaylist.value.coverUrl && songs[0]?.coverUrl) {
          selectedPlaylist.value.coverUrl = songs[0].coverUrl;
        }
      }
      if (pl.isLiked || pl.title === "我喜欢") {
        likedSongs.value = songs;
      }
      if (gid && selectedPlaylist.value) {
        playlistSwrCache.set(gid, {
          playlist: { ...selectedPlaylist.value },
          songs: [...songs],
        });
      }
    } catch (e: any) {
      if (!cached) notify(`加载歌单曲目失败：${String(e)}`);
    } finally {
      isLoadingLibrary.value = false;
    }
  }

  async function openPlaylistDetail(pl: Playlist) {
    await selectPlaylist(pl, true);
  }

  async function openLikedSongsDetail() {
    if (!isLoggedIn.value) {
      isLoginModalOpen.value = true;
      return;
    }
    const likedPl = likedPlaylistMeta.value;
    if (likedPl && (likedPl.globalCollectionId || likedPl.id)) {
      if (likedSongs.value.length > 0) {
        playlistSwrCache.set(likedPl.globalCollectionId || likedPl.id, {
          playlist: {
            ...likedPl,
            title: "我喜欢的音乐",
            name: "我喜欢的音乐",
            isLiked: true,
            trackCount: Math.max(likedSongs.value.length, likedPl.trackCount || 0),
            coverUrl: likedSongs.value[0]?.coverUrl || likedPl.coverUrl || "",
          },
          songs: [...likedSongs.value],
        });
      }
      await selectPlaylist(
        {
          ...likedPl,
          title: "我喜欢的音乐",
          name: "我喜欢的音乐",
          isLiked: true,
          trackCount: Math.max(likedSongs.value.length, likedPl.trackCount || 0),
          coverUrl: likedSongs.value[0]?.coverUrl || likedPl.coverUrl || "",
        },
        true
      );
      return;
    }
    pushNavHistory();
    selectedPlaylist.value = {
      id: "__liked_music__",
      listId: 0,
      globalCollectionId: "",
      title: "我喜欢的音乐",
      name: "我喜欢的音乐",
      description: "你在酷狗概念版标记红心的全部珍藏单曲，支持云端实时双向同步与无损畅听。",
      intro: "你在酷狗概念版标记红心的全部珍藏单曲，支持云端实时双向同步与无损畅听。",
      coverUrl: likedSongs.value[0]?.coverUrl || "",
      trackCount: likedSongs.value.length,
      playCount: 0,
      creatorName: profile.value?.nickname || session.value?.nickname || "我",
      creator: profile.value?.nickname || session.value?.nickname || "我",
      isLiked: true,
      isCreatedByMe: true,
      isFavorite: false,
    };
    playlistSongs.value = [...likedSongs.value];
    activeTab.value = "playlist_detail";
    isNowPlayingOpen.value = false;
    isSearchPaletteOpen.value = false;
    scrollStageToTop();
  }

  async function openRecentHistoryDetail() {
    pushNavHistory();
    selectedPlaylist.value = {
      id: "__recent_history__",
      listId: 0,
      globalCollectionId: "__recent_history__",
      title: "最近播放",
      name: "最近播放",
      description: "自动持久化保存本地与酷狗概念版云端双端最近聆听足迹，重启软件不丢失。",
      intro: "自动持久化保存本地与酷狗概念版云端双端最近聆听足迹，重启软件不丢失。",
      coverUrl: recentHistorySongs.value[0]?.coverUrl || "",
      trackCount: recentHistorySongs.value.length,
      playCount: 0,
      creatorName: profile.value?.nickname || session.value?.nickname || "微风乐迷",
      creator: profile.value?.nickname || session.value?.nickname || "微风乐迷",
      isLiked: false,
      isCreatedByMe: false,
      isFavorite: false,
    };
    playlistSongs.value = [...recentHistorySongs.value];
    activeTab.value = "playlist_detail";
    isNowPlayingOpen.value = false;
    isSearchPaletteOpen.value = false;
    scrollStageToTop();
    await loadRecentHistory();
    if (selectedPlaylist.value?.id === "__recent_history__") {
      playlistSongs.value = [...recentHistorySongs.value];
      selectedPlaylist.value.trackCount = recentHistorySongs.value.length;
      if (!selectedPlaylist.value.coverUrl && recentHistorySongs.value[0]?.coverUrl) {
        selectedPlaylist.value.coverUrl = recentHistorySongs.value[0].coverUrl;
      }
    }
  }

  async function openPurchasedSongsDetail() {
    pushNavHistory();
    const songs = purchasedSongs.value.map((p) => p.song);
    selectedPlaylist.value = {
      id: "__purchased_songs__",
      listId: 0,
      globalCollectionId: "__purchased_songs__",
      title: "已购单曲合集",
      name: "已购单曲合集",
      description: "你在酷狗音乐已付费购买的数字单曲与无损珍藏曲目。",
      intro: "你在酷狗音乐已付费购买的数字单曲与无损珍藏曲目。",
      coverUrl: songs[0]?.coverUrl || purchasedAlbums.value[0]?.coverUrl || "",
      trackCount: songs.length,
      playCount: 0,
      creatorName: profile.value?.nickname || session.value?.nickname || "我",
      creator: profile.value?.nickname || session.value?.nickname || "我",
      isLiked: false,
      isCreatedByMe: false,
      isFavorite: false,
    };
    playlistSongs.value = [...songs];
    activeTab.value = "playlist_detail";
    isNowPlayingOpen.value = false;
    isSearchPaletteOpen.value = false;
    scrollStageToTop();
    await loadPurchasedAssets();
    if (selectedPlaylist.value?.id === "__purchased_songs__") {
      const updated = purchasedSongs.value.map((p) => p.song);
      playlistSongs.value = updated;
      selectedPlaylist.value.trackCount = updated.length;
      if (!selectedPlaylist.value.coverUrl && updated[0]?.coverUrl) {
        selectedPlaylist.value.coverUrl = updated[0].coverUrl;
      }
    }
  }

  async function refreshCurrentPlaylistOnly() {
    const pl = selectedPlaylist.value;
    if (!pl) return;
    isLoadingLibrary.value = true;
    try {
      if (pl.id === "__recent_history__") {
        await loadRecentHistory();
        playlistSongs.value = [...recentHistorySongs.value];
        if (selectedPlaylist.value) {
          selectedPlaylist.value.trackCount = recentHistorySongs.value.length;
          selectedPlaylist.value.coverUrl =
            recentHistorySongs.value[0]?.coverUrl || selectedPlaylist.value.coverUrl;
        }
        notify(`已刷新「最近播放」（共 ${recentHistorySongs.value.length} 首）`);
        return;
      }
      if (pl.id === "__purchased_songs__") {
        await loadPurchasedAssets();
        const songs = purchasedSongs.value.map((p) => p.song);
        playlistSongs.value = songs;
        if (selectedPlaylist.value) {
          selectedPlaylist.value.trackCount = songs.length;
          selectedPlaylist.value.coverUrl =
            songs[0]?.coverUrl || selectedPlaylist.value.coverUrl;
        }
        notify(`已刷新「已购单曲合集」（共 ${songs.length} 首）`);
        return;
      }
      const gid = pl.globalCollectionId || pl.id;
      const [songs, metaInfo] = await Promise.all([
        kugouClient.getPlaylistSongs(gid),
        !pl.isCreatedByMe && gid
          ? kugouClient.getPlaylistInfo(gid).catch(() => null)
          : Promise.resolve(null),
      ]);
      playlistSongs.value = songs;
      if (metaInfo && selectedPlaylist.value) {
        selectedPlaylist.value = {
          ...selectedPlaylist.value,
          title: metaInfo.title || selectedPlaylist.value.title,
          name: metaInfo.title || selectedPlaylist.value.name,
          description: metaInfo.description || selectedPlaylist.value.description,
          intro: metaInfo.intro || metaInfo.description || selectedPlaylist.value.intro,
          coverUrl: metaInfo.coverUrl || selectedPlaylist.value.coverUrl || songs[0]?.coverUrl || "",
          trackCount: Math.max(songs.length, metaInfo.trackCount || 0),
        };
      } else if (selectedPlaylist.value) {
        selectedPlaylist.value.trackCount = Math.max(songs.length, selectedPlaylist.value.trackCount || 0);
        if (!selectedPlaylist.value.coverUrl && songs[0]?.coverUrl) {
          selectedPlaylist.value.coverUrl = songs[0].coverUrl;
        }
      }
      if (pl.isLiked || pl.title === "我喜欢" || pl.title === "我喜欢的音乐") {
        likedSongs.value = songs;
      }
      notify(`已刷新歌单「${selectedPlaylist.value?.title || pl.title}」（共 ${songs.length} 首）`);
    } catch (e: any) {
      notify(`刷新歌单失败：${String(e)}`);
    } finally {
      isLoadingLibrary.value = false;
    }
  }

  async function updatePlaylistMeta(listId: number, name: string, intro: string, coverUrl = "") {
    try {
      const target =
        userPlaylists.value.find((p) => p.listId === listId) ||
        (selectedPlaylist.value?.listId === listId ? selectedPlaylist.value : null);
      const globalId = target?.globalCollectionId || target?.id || "";
      await kugouClient.updateUserPlaylist(listId, globalId, name, intro);
      if (selectedPlaylist.value && selectedPlaylist.value.listId === listId) {
        selectedPlaylist.value.title = name;
        selectedPlaylist.value.name = name;
        selectedPlaylist.value.description = intro;
        selectedPlaylist.value.intro = intro;
        if (coverUrl) selectedPlaylist.value.coverUrl = coverUrl;
      }
      if (target) {
        target.title = name;
        target.name = name;
        target.description = intro;
        target.intro = intro;
        if (coverUrl) target.coverUrl = coverUrl;
      }
      notify(`已保存歌单「${name}」编辑信息并同步`);
      return true;
    } catch (e: any) {
      notify(`编辑歌单失败：${String(e)}`);
      return false;
    }
  }

  async function batchAddSongsToPlaylist(listId: number, songs: Song[]) {
    if (!songs.length) return false;
    try {
      await kugouClient.addSongsToPlaylistBatch(listId, songs);
      notify(`已将 ${songs.length} 首歌曲批量添加至目标歌单`);
      await loadUserPlaylists();
      return true;
    } catch (e: any) {
      notify(`批量添加失败：${String(e)}`);
      return false;
    }
  }

  async function batchRemoveSongsFromPlaylist(songsToRemove: Song[]) {
    const pl = selectedPlaylist.value;
    if (!pl || !pl.listId || songsToRemove.length === 0) {
      notify("仅自建云歌单支持批量移除曲目");
      return false;
    }
    let removedCount = 0;
    const removeHashes = new Set(songsToRemove.map((s) => s.hash.toLowerCase()));
    for (const s of songsToRemove) {
      try {
        await kugouClient.removeSongFromPlaylist(pl.listId, s);
        removedCount += 1;
      } catch {
        // continue
      }
    }
    playlistSongs.value = playlistSongs.value.filter(
      (s) => !removeHashes.has((s.hash || "").toLowerCase())
    );
    if (pl.isLiked) {
      likedSongs.value = likedSongs.value.filter(
        (s) => !removeHashes.has((s.hash || "").toLowerCase())
      );
    }
    pl.trackCount = playlistSongs.value.length;
    notify(`已从「${pl.title}」批量移除 ${removedCount} 首歌曲`);
    return true;
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
    const customName = rawTextArg !== undefined ? nameOrRawText.trim() : "";
    const rawText = (rawTextArg !== undefined ? rawTextArg : nameOrRawText).trim();
    if (!rawText) return 0;

    notify("正在解析分享链接 / 口令 / 歌单曲目...");
    let songsToImport: Song[] = [];
    let resolvedTitle = customName || `导入歌单 ${new Date().toLocaleDateString()}`;

    try {
      const parsed = await kugouClient.parseSharedMusic(rawText);
      if (parsed.songs && parsed.songs.length > 0) {
        songsToImport = parsed.songs;
        if (!customName && parsed.playlist?.title) {
          resolvedTitle = parsed.playlist.title;
        }
      }
    } catch {
      // 回退至逐行搜索匹配
    }

    if (songsToImport.length === 0) {
      const lines = rawText
        .split(/\r?\n/)
        .map((l) => l.trim())
        .filter((l) => l.length > 1);
      for (const line of lines.slice(0, 35)) {
        try {
          const res = await kugouClient.searchSongs(line, 1, 2);
          if (res.length > 0) {
            songsToImport.push(res[0]);
          }
        } catch {
          // ignore
        }
      }
    }

    if (songsToImport.length === 0) {
      notify("未能从输入内容中解析到可导入的歌曲");
      return 0;
    }

    await kugouClient.createUserPlaylist(resolvedTitle);
    await loadUserPlaylists();
    const target = userPlaylists.value.find((p) => p.title === resolvedTitle);
    let matchedCount = 0;

    if (target?.listId) {
      for (const song of songsToImport.slice(0, 60)) {
        try {
          await kugouClient.addSongToPlaylist(target.listId, song);
          matchedCount += 1;
        } catch {
          // ignore single failure
        }
      }
    }
    await loadUserPlaylists();
    if (target) {
      await selectPlaylist(target);
    }
    notify(`歌单「${resolvedTitle}」导入完成，成功同步 ${matchedCount} 首曲目`);
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
      const list = await kugouClient.getFollowedArtists();
      followedArtists.value = applySavedOrder(
        list,
        artistsOrderIds.value,
        (a) => String(a.singerId || a.artistId || a.name)
      );
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
      purchasedAlbums.value = applySavedOrder(
        albums,
        purchasedOrderIds.value,
        (a) => String(a.id || a.albumId || a.albumName)
      );
      purchasedSongs.value = songs;
    } catch (e) {
      console.warn("loadPurchasedAssets error:", e);
    }
  }

  function reorderLibraryAssetList(
    category: "created" | "collected" | "artists" | "purchased",
    fromIndex: number,
    toIndex: number,
    customKeys?: string[]
  ) {
    if (fromIndex === toIndex && !customKeys) return;
    if (category === "created") {
      const current = [...createdPlaylists.value];
      if (
        fromIndex < 0 ||
        fromIndex >= current.length ||
        toIndex < 0 ||
        toIndex >= current.length
      )
        return;
      const [moved] = current.splice(fromIndex, 1);
      current.splice(toIndex, 0, moved);
      createdOrderIds.value = current.map((p) =>
        String(p.id || p.listId || p.title)
      );
      localStorage.setItem(
        "bm_order_created",
        JSON.stringify(createdOrderIds.value)
      );
    } else if (category === "collected") {
      const current = [...collectedPlaylists.value];
      if (
        fromIndex < 0 ||
        fromIndex >= current.length ||
        toIndex < 0 ||
        toIndex >= current.length
      )
        return;
      const [moved] = current.splice(fromIndex, 1);
      current.splice(toIndex, 0, moved);
      collectedOrderIds.value = current.map((p) =>
        String(p.id || p.globalCollectionId || p.title)
      );
      localStorage.setItem(
        "bm_order_collected",
        JSON.stringify(collectedOrderIds.value)
      );
    } else if (category === "artists") {
      const current = [...followedArtists.value];
      if (
        fromIndex < 0 ||
        fromIndex >= current.length ||
        toIndex < 0 ||
        toIndex >= current.length
      )
        return;
      const [moved] = current.splice(fromIndex, 1);
      current.splice(toIndex, 0, moved);
      followedArtists.value = current;
      artistsOrderIds.value = current.map((a) =>
        String(a.singerId || a.artistId || a.name)
      );
      localStorage.setItem(
        "bm_order_artists",
        JSON.stringify(artistsOrderIds.value)
      );
    } else if (category === "purchased") {
      if (customKeys) {
        purchasedOrderIds.value = [...customKeys];
        localStorage.setItem(
          "bm_order_purchased",
          JSON.stringify(purchasedOrderIds.value)
        );
        purchasedAlbums.value = applySavedOrder(
          purchasedAlbums.value,
          purchasedOrderIds.value,
          (a) => String(a.id || a.albumId || a.albumName)
        );
      } else {
        const current = [...purchasedAlbums.value];
        if (
          fromIndex < 0 ||
          fromIndex >= current.length ||
          toIndex < 0 ||
          toIndex >= current.length
        )
          return;
        const [moved] = current.splice(fromIndex, 1);
        current.splice(toIndex, 0, moved);
        purchasedAlbums.value = current;
        purchasedOrderIds.value = current.map((a) =>
          String(a.id || a.albumId || a.albumName)
        );
        localStorage.setItem(
          "bm_order_purchased",
          JSON.stringify(purchasedOrderIds.value)
        );
      }
    }
  }

  const artistSongsPage = ref<number>(1);
  const artistSongsHasMore = ref<boolean>(true);
  const isLoadingMoreArtistSongs = ref<boolean>(false);

  async function openArtistDetail(
    artistId: string,
    fallbackName = "",
    fallbackAvatar = ""
  ) {
    pushNavHistory();
    activeTab.value = "artist_detail";
    libraryDetailMode.value = "artist";
    artistDetailSubTab.value = "songs";
    artistSongsPage.value = 1;
    artistSongsHasMore.value = true;
    isNowPlayingOpen.value = false;
    isSearchPaletteOpen.value = false;
    isMultiArtistModalOpen.value = false;
    scrollStageToTop();
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
      let resolvedId = (artistId || "").trim();
      if ((!resolvedId || resolvedId === "0") && fallbackName) {
        const matchedArtists = await kugouClient.searchArtists(fallbackName).catch(() => []);
        const exact =
          matchedArtists.find((a) => a.name.toLowerCase() === fallbackName.trim().toLowerCase()) ||
          matchedArtists[0];
        if (exact && exact.artistId && exact.artistId !== "0") {
          resolvedId = exact.artistId;
          if (!fallbackAvatar && exact.avatarUrl) {
            fallbackAvatar = exact.avatarUrl;
          }
        }
      }

      if (resolvedId && resolvedId !== "0") {
        const [detail, songs, albums] = await Promise.all([
          kugouClient.getArtistDetail(resolvedId).catch(() => null),
          kugouClient.getArtistSongs(resolvedId, artistSongsSort.value, 1),
          kugouClient.getArtistAlbums(resolvedId, 1, "new").catch(() => []),
        ]);
        if (detail) {
          detail.isFollowed = followedArtists.value.some(
            (a) => a.artistId === detail.artistId || a.name === detail.name
          );
          if (!detail.avatarUrl) detail.avatarUrl = fallbackAvatar || songs[0]?.coverUrl || "";
          if (!detail.songCount || detail.songCount < songs.length) detail.songCount = songs.length;
          if (!detail.albumCount || detail.albumCount < albums.length) detail.albumCount = albums.length;
          activeArtistDetail.value = detail;
        } else if (activeArtistDetail.value) {
          activeArtistDetail.value.artistId = resolvedId;
          activeArtistDetail.value.songCount = songs.length;
          activeArtistDetail.value.albumCount = albums.length;
          if (!activeArtistDetail.value.avatarUrl && songs[0]?.coverUrl) {
            activeArtistDetail.value.avatarUrl = songs[0].coverUrl;
          }
        }
        activeArtistSongs.value = songs;
        activeArtistAlbums.value = albums;
        artistSongsHasMore.value = songs.length >= 25;
        if (activeArtistDetail.value) {
          artistSwrCache.set(resolvedId, {
            detail: { ...activeArtistDetail.value },
            songs: [...songs],
            albums: [...albums],
          });
        }
      } else if (fallbackName) {
        const [songs, albums] = await Promise.all([
          kugouClient.searchSongs(fallbackName, 1, 50),
          kugouClient.searchAlbums(fallbackName).catch(() => []),
        ]);
        activeArtistSongs.value = songs;
        activeArtistAlbums.value = albums;
        artistSongsHasMore.value = false;
        if (activeArtistDetail.value) {
          if (!activeArtistDetail.value.avatarUrl && songs[0]?.coverUrl) {
            activeArtistDetail.value.avatarUrl = songs[0].coverUrl;
          }
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

  async function loadMoreArtistSongs() {
    const aid = activeArtistDetail.value?.artistId || activeArtistDetail.value?.singerId;
    if (!aid || aid === "0" || isLoadingMoreArtistSongs.value || !artistSongsHasMore.value) return;
    isLoadingMoreArtistSongs.value = true;
    try {
      const nextPage = artistSongsPage.value + 1;
      const more = await kugouClient.getArtistSongs(String(aid), artistSongsSort.value, nextPage);
      if (more.length === 0) {
        artistSongsHasMore.value = false;
      } else {
        const seen = new Set(activeArtistSongs.value.map((s) => s.hash));
        const unique = more.filter((s) => !seen.has(s.hash));
        activeArtistSongs.value = [...activeArtistSongs.value, ...unique];
        artistSongsPage.value = nextPage;
        if (more.length < 25 || unique.length === 0) {
          artistSongsHasMore.value = false;
        }
      }
    } catch {
      artistSongsHasMore.value = false;
    } finally {
      isLoadingMoreArtistSongs.value = false;
    }
  }

  async function openAlbumDetail(
    albumId: string,
    fallbackName = "",
    fallbackArtist = "",
    fallbackCover = ""
  ) {
    pushNavHistory();
    activeTab.value = "album_detail";
    libraryDetailMode.value = "album";
    isNowPlayingOpen.value = false;
    isSearchPaletteOpen.value = false;
    scrollStageToTop();
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
        if (activeAlbumDetail.value) {
          albumSwrCache.set(albumId, {
            detail: { ...activeAlbumDetail.value },
            songs: [...songs],
          });
        }
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

  async function navigateToSongArtist(song: Song | null | undefined, specificAuthor?: SongAuthor) {
    if (!song && !specificAuthor) return;
    if (specificAuthor) {
      isMultiArtistModalOpen.value = false;
      await openArtistDetail(specificAuthor.id || "0", specificAuthor.name, specificAuthor.avatar || song?.coverUrl || "");
      return;
    }
    if (!song) return;
    const authors = song.authors && song.authors.length > 0 ? song.authors : splitSongAuthors(song.artist, song.artistId, song.coverUrl);
    if (authors.length > 1) {
      multiArtistCandidates.value = authors;
      multiArtistSongTitle.value = song.title;
      isMultiArtistModalOpen.value = true;
      return;
    }
    const target = authors[0] || { id: song.artistId || "0", name: song.artist || "未知歌手", avatar: song.coverUrl };
    await openArtistDetail(target.id || song.artistId || "0", target.name, target.avatar || song.coverUrl || "");
  }

  async function navigateToSongAlbum(song: Song | null | undefined) {
    if (!song) return;
    let albumId = (song.albumId || "").trim();
    let albumName = (song.album || "").trim();
    let artistId = (song.artistId || "").trim();
    let artistName = (song.artist || "").trim();
    let coverUrl = song.coverUrl || "";

    if ((!albumId || albumId === "0") && song.hash) {
      try {
        const resolved = await kugouClient.resolveSongAlbumMeta(song.hash, song.albumAudioId);
        if (resolved.albumId && resolved.albumId !== "0") {
          albumId = resolved.albumId;
          song.albumId = resolved.albumId;
        }
        if (resolved.albumName && !albumName) {
          albumName = resolved.albumName;
          song.album = resolved.albumName;
        }
        if (resolved.artistId && (!artistId || artistId === "0")) {
          artistId = resolved.artistId;
          song.artistId = resolved.artistId;
        }
        if (resolved.artistName && !artistName) {
          artistName = resolved.artistName;
        }
        if (resolved.coverUrl && !coverUrl) {
          coverUrl = resolved.coverUrl;
        }
      } catch {
        // fallback to search inside openAlbumDetail
      }
    }

    if ((!albumId || albumId === "0") && !albumName) {
      notify("该曲目暂无关联专辑信息");
      return;
    }
    await openAlbumDetail(albumId || "0", albumName || song.title, artistName, coverUrl);
  }

  async function openCommentsModal(song?: Song | null) {
    const target = song || currentSong.value;
    if (!target) {
      notify("请先选择或播放一首歌曲以查看评论");
      return;
    }
    commentsModalSong.value = target;
    isCommentsModalOpen.value = true;
    await loadCommentsForSong(target, commentSortType.value);
  }

  function closeCommentsModal() {
    isCommentsModalOpen.value = false;
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

    // 切歌前结算上一首曲目的真实播放时长与 CSCC 完播/切歌事件
    settleActiveTrackTelemetry();

    if (list && list.length > 0) {
      playQueue.value = [...list];
      if (!recentPlayQueues.value.some((q) => q.title === queueTitle)) {
        recentPlayQueues.value.unshift({
          id: `${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
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

    const reqId = ++activePlayRequestId;
    isBuffering.value = true;
    currentPlaybackInfo.value = null;
    currentTrackPlayedAccumMs = 0;
    durationMs.value = song.durationMs || 0;
    syncWindowsMediaSession(song);

    loadLyricForSong(song);
    loadCommentsForSong(song, commentSortType.value);

    try {
      const pb = ktvAccompanimentMode.value
        ? await kugouClient
            .resolveSongAccompaniment(song)
            .catch(() => kugouClient.resolveSongUrl(song, preferredQuality.value))
        : await kugouClient.resolveSongUrl(song, preferredQuality.value);

      // 若在异步解析直链期间用户已切到其他歌曲，直接丢弃过期结果，杜绝切歌竞态覆盖
      if (reqId !== activePlayRequestId) return;

      currentPlaybackInfo.value = pb;
      // 优先使用 Rust 本地 HTTP Range 边听边存流媒体代理 (proxyUrl)，彻底杜绝酷狗 CDN 长连接静默断流
      const candidates = [pb.proxyUrl, pb.url, ...(pb.backupUrls || [])].filter(
        (u, idx, arr): u is string => Boolean(u) && arr.indexOf(u) === idx
      );
      currentCandidateUrls = candidates;
      currentCandidateIdx = 0;
      hasRetriedWithoutCors = false;
      hasRetriedFallbackQuality = false;
      lastWatchdogPosSec = -1;
      watchdogStallTicks = 0;
      if (!audio.crossOrigin) {
        audio.crossOrigin = "anonymous";
      }
      audio.src = candidates[0] || pb.url;
      currentSongPlayStartWallMs = Date.now();

      // 触发 CSCC start 与听歌足迹上报
      activeTelemetrySong = song;
      currentTrackPlayedAccumMs = 0;
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
      cancelVolumeRamp();
      const shouldFadeIn = isCrossfadeTransitioning && crossfadeSeconds.value > 0;
      isCrossfadeTransitioning = false;
      audio.volume = shouldFadeIn
        ? Math.min(0.04, volume.value)
        : volume.value;

      if (
        (nightLoudnessEnabled.value || replayGainEnabled.value) &&
        !audioCtx &&
        audio.crossOrigin
      ) {
        ensureWebAudioGraph();
      }

      try {
        await audio.play();
      } catch (playErr: any) {
        if (reqId !== activePlayRequestId) return;
        if (playErr?.name === "AbortError") {
          return;
        }
        // 主直链 play() 失败时立即尝试备用 CDN 节点
        let recovered = false;
        while (currentCandidateIdx + 1 < currentCandidateUrls.length) {
          if (reqId !== activePlayRequestId) return;
          currentCandidateIdx += 1;
          audio.src = currentCandidateUrls[currentCandidateIdx];
          if (startAtMs > 0) {
            audio.currentTime = startAtMs / 1000;
          }
          try {
            await audio.play();
            recovered = true;
            break;
          } catch (retryErr: any) {
            if (retryErr?.name === "AbortError") {
              return;
            }
          }
        }
        if (!recovered) {
          audio.volume = volume.value;
          throw playErr;
        }
      }

      if (shouldFadeIn && volume.value > 0.04) {
        rampHtmlAudioVolume(
          Math.min(0.04, volume.value),
          volume.value,
          Math.min(2600, crossfadeSeconds.value * 450)
        );
      }
      if (pb.isFreePart) {
        notify("当前为试听片段（登录并签到概念版 VIP 即可解锁整首无损）");
      }
      refreshCacheStats();
    } catch (e: any) {
      if (reqId !== activePlayRequestId || e?.name === "AbortError") return;
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
      const targetQuality =
        currentSong.value?.hash === song.hash && currentPlaybackInfo.value?.actualQuality
          ? currentPlaybackInfo.value.actualQuality
          : preferredQuality.value;
      activeSongDetailInfo.value = await kugouClient.getSongDetailInfo(song, targetQuality);
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
      broadcastDesktopLyricState();
    } catch {
      currentLyric.value = {
        lines: [],
        rawText: "",
        isKrc: false,
        hasTranslation: false,
        hasPhonetic: false,
      };
      broadcastDesktopLyricState();
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
    const targetSong = commentsModalSong.value || currentSong.value;
    if (floorRepliesMap[cmt.id]?.length || !targetSong) return;
    isLoadingFloorReplies.value = true;
    try {
      floorRepliesMap[cmt.id] = await kugouClient.getCommentFloorReplies(
        targetSong,
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
    const targetSong = commentsModalSong.value || currentSong.value;
    if (!targetSong) return;
    const nextLike = !cmt.isLiked;
    cmt.isLiked = nextLike;
    cmt.likeCount = Math.max(0, cmt.likeCount + (nextLike ? 1 : -1));
    await kugouClient
      .likeSongComment(targetSong, cmt.id, nextLike)
      .catch(() => {});
  }

  async function submitSongComment(content: string, replyToId?: string) {
    const targetSong = commentsModalSong.value || currentSong.value;
    if (!targetSong) return false;
    if (!isLoggedIn.value) {
      isLoginModalOpen.value = true;
      return false;
    }
    try {
      const created = await kugouClient.postSongComment(
        targetSong,
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
      if (playPauseFadeEnabled.value && audioCtx && masterPreampGain) {
        const now = audioCtx.currentTime;
        const targetGain = masterPreampGain.gain.value || 1.0;
        masterPreampGain.gain.cancelScheduledValues(now);
        masterPreampGain.gain.setValueAtTime(0.05, now);
        audio.volume = volume.value;
        await audio.play();
        masterPreampGain.gain.linearRampToValueAtTime(targetGain, now + 0.16);
      } else if (playPauseFadeEnabled.value && volume.value > 0.05) {
        audio.volume = 0.04;
        await audio.play();
        rampHtmlAudioVolume(0.04, volume.value, 160);
      } else {
        audio.volume = volume.value;
        await audio.play();
      }
    } else {
      if (playPauseFadeEnabled.value && audioCtx && masterPreampGain) {
        const now = audioCtx.currentTime;
        const savedGain = masterPreampGain.gain.value || 1.0;
        masterPreampGain.gain.cancelScheduledValues(now);
        masterPreampGain.gain.setValueAtTime(savedGain, now);
        masterPreampGain.gain.linearRampToValueAtTime(0.02, now + 0.15);
        setTimeout(() => {
          audio.pause();
          if (masterPreampGain && audioCtx) {
            masterPreampGain.gain.cancelScheduledValues(audioCtx.currentTime);
            masterPreampGain.gain.setValueAtTime(savedGain, audioCtx.currentTime);
          }
        }, 155);
      } else if (playPauseFadeEnabled.value && audio.volume > 0.05) {
        const startVol = audio.volume;
        rampHtmlAudioVolume(startVol, 0.02, 150, () => {
          audio.pause();
          audio.volume = volume.value;
        });
      } else {
        audio.pause();
      }
    }
  }

  async function playNext(fromAutoEnded = false) {
    const len = playQueue.value.length;
    if (len === 0) return;

    if (!fromAutoEnded) {
      settleActiveTrackTelemetry();
    }

    if (fromAutoEnded && playMode.value === "repeat_one") {
      if (currentSong.value) {
        activeTelemetrySong = currentSong.value;
        currentTrackPlayedAccumMs = 0;
        kugouClient.reportPlaybackTelemetry(currentSong.value, true, 0).catch(() => {});
      }
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
    settleActiveTrackTelemetry();
    currentIndex.value = (currentIndex.value - 1 + len) % len;
    await loadAndPlayCurrentSong(0);
  }

  function insertPlayNext(song: Song) {
    if (playQueue.value.length === 0) {
      playSongFromList(song, [song], "当前播放队列");
      return;
    }
    const existingIdx = playQueue.value.findIndex(
      (s) => (s.hash && s.hash === song.hash) || (s.id && s.id === song.id)
    );
    if (existingIdx >= 0 && existingIdx !== currentIndex.value) {
      playQueue.value.splice(existingIdx, 1);
      if (existingIdx < currentIndex.value) {
        currentIndex.value -= 1;
      }
    }
    const insertIdx = Math.min(playQueue.value.length, currentIndex.value + 1);
    playQueue.value.splice(insertIdx, 0, song);
    notify(`已安排下一首播放：「${song.title}」`);
  }

  function removeSongFromQueue(index: number) {
    if (index < 0 || index >= playQueue.value.length) return;
    const removed = playQueue.value[index];
    playQueue.value.splice(index, 1);
    if (playQueue.value.length === 0) {
      settleActiveTrackTelemetry();
      audio.pause();
      currentIndex.value = -1;
      return;
    }
    if (index < currentIndex.value) {
      currentIndex.value -= 1;
    } else if (index === currentIndex.value) {
      settleActiveTrackTelemetry();
      currentIndex.value = currentIndex.value % playQueue.value.length;
      loadAndPlayCurrentSong(0);
    }
    if (removed) {
      notify(`已从播放队列移除「${removed.title}」`);
    }
  }

  function clearPlayQueue() {
    settleActiveTrackTelemetry();
    audio.pause();
    playQueue.value = [];
    currentIndex.value = -1;
    notify("已清空当前播放队列");
  }

  function moveQueueItem(fromIndex: number, toIndex: number) {
    if (
      fromIndex === toIndex ||
      fromIndex < 0 ||
      toIndex < 0 ||
      fromIndex >= playQueue.value.length ||
      toIndex >= playQueue.value.length
    ) {
      return;
    }
    const currentHash = currentSong.value?.hash;
    const list = [...playQueue.value];
    const [moved] = list.splice(fromIndex, 1);
    list.splice(toIndex, 0, moved);
    playQueue.value = list;
    if (currentHash) {
      const newIdx = list.findIndex((s) => s.hash === currentHash);
      if (newIdx >= 0) currentIndex.value = newIdx;
    }
  }

  function reorderQueueByMood(mood: "energetic" | "chill" | "vocal") {
    if (playQueue.value.length <= 2) return;
    const current = currentSong.value;
    const rest = playQueue.value.filter((_, idx) => idx !== currentIndex.value);
    rest.sort((a, b) => {
      if (mood === "energetic") {
        return (a.durationMs || 200000) - (b.durationMs || 200000);
      } else if (mood === "chill") {
        return (b.durationMs || 200000) - (a.durationMs || 200000);
      } else {
        return a.artist.localeCompare(b.artist, "zh-CN");
      }
    });
    playQueue.value = current ? [current, ...rest] : rest;
    currentIndex.value = 0;
    const moodLabels = {
      energetic: "律动高能",
      chill: "舒缓松弛",
      vocal: "人声叙事",
    };
    notify(`AI 智能编排已按「${moodLabels[mood]}」重塑队列顺序`);
  }

  function switchToRecentQueue(idOrIdx: string | number) {
    const target =
      typeof idOrIdx === "number"
        ? recentPlayQueues.value[idOrIdx]
        : recentPlayQueues.value.find((q) => q.id === idOrIdx);
    if (!target || target.songs.length === 0) return;
    playSongFromList(target.songs[0], target.songs, target.title);
    notify(`已切换至队列「${target.title}」`);
  }

  function removeRecentQueue(idx: number) {
    if (idx >= 0 && idx < recentPlayQueues.value.length) {
      recentPlayQueues.value.splice(idx, 1);
    }
  }

  function syncWindowsMediaSession(song: Song) {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator)) return;
    try {
      navigator.mediaSession.metadata = new MediaMetadata({
        title: song.title,
        artist: song.artist,
        album: song.album || "微风音乐 · 酷狗概念版",
        artwork: song.coverUrl
          ? [{ src: song.coverUrl, sizes: "400x400", type: "image/jpeg" }]
          : [],
      });
      navigator.mediaSession.setActionHandler("play", () => togglePlayPause());
      navigator.mediaSession.setActionHandler("pause", () => togglePlayPause());
      navigator.mediaSession.setActionHandler("previoustrack", () => playPrev());
      navigator.mediaSession.setActionHandler("nexttrack", () => playNext(false));
    } catch {
      // ignore unsupported mediaSession environment
    }
  }

  function seekToMs(ms: number) {
    if (!audio.src) return;
    const clamped = Math.max(0, Math.min(ms, durationMs.value || ms));
    isCrossfadeTransitioning = false;
    cancelVolumeRamp();
    audio.volume = volume.value;
    audio.currentTime = clamped / 1000;
    currentPositionMs.value = clamped;
  }

  function setVolume(val: number) {
    const v = Math.max(0, Math.min(1, val));
    cancelVolumeRamp();
    volume.value = v;
    audio.volume = v;
    localStorage.setItem("bm_volume", String(v));
    if (nightLoudnessEnabled.value && audioCtx) {
      syncDspChain();
    }
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
        bilingualLrcExport.value,
        skipExistingFiles.value
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

  async function clearCacheCategory(
    category: "audio" | "image" | "snapshot" | "lyric" | "all"
  ) {
    try {
      const updated = await kugouClient.clearCacheCategory(category);
      cacheStats.audioStreamMb = Number((updated.audioStreamMb || 0).toFixed(2));
      cacheStats.artworkImageMb = Number((updated.artworkImageMb || 0).toFixed(2));
      cacheStats.playlistSnapshotMb = Number((updated.playlistSnapshotMb || 0).toFixed(2));
      cacheStats.lyricAndUrlMb = Number((updated.lyricAndUrlMb || 0).toFixed(2));
    } catch {
      if (category === "audio" || category === "all") cacheStats.audioStreamMb = 0;
      if (category === "image" || category === "all") cacheStats.artworkImageMb = 0;
      if (category === "snapshot" || category === "all") cacheStats.playlistSnapshotMb = 0;
      if (category === "lyric" || category === "all") cacheStats.lyricAndUrlMb = 0;
    }
    notify("已完成指定磁盘缓存分项清理");
  }

  // 自动持久化播放队列与进度，供下次启动恢复 (autoResumeOnStartup)
  watch(
    [playQueue, currentIndex],
    () => {
      try {
        localStorage.setItem(
          "bm_saved_queue",
          JSON.stringify(playQueue.value.slice(0, 60))
        );
        localStorage.setItem("bm_saved_index", String(currentIndex.value));
      } catch {
        // ignore quota error
      }
    },
    { deep: false }
  );
  watch(currentPositionMs, (pos) => {
    if (pos > 0 && pos % 3000 < 400) {
      localStorage.setItem("bm_saved_pos_ms", String(pos));
    }
  });

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

  function setBandFilterTopology(index: number, topology: any) {
    if (index < 0 || index >= 10) return;
    eqBandTypes.value[index] = topology;
    syncDspChain();
  }

  async function switchArtistSongSort(sort: "hot" | "new") {
    artistSongsSort.value = sort;
    const aid = activeArtistDetail.value?.artistId || activeArtistDetail.value?.singerId;
    if (aid && aid !== "0") {
      try {
        activeArtistSongs.value = await kugouClient.getArtistSongs(String(aid), sort, 1);
        notify(sort === "new" ? "已切换为最新发行单曲排序" : "已切换为最热代表作排序");
      } catch {}
    }
  }

  return {
    desktopLyricColor,
    desktopLyricFontSize,
    desktopLyricWidth,
    desktopLyricX,
    desktopLyricY,
    desktopLyricShowTranslation,
    navHistoryStack,
    canNavigateBack,
    pushNavHistory,
    navigateBack,
    openSoundMixerFromPlayer,
    isCommentsModalOpen,
    commentsModalSong,
    openCommentsModal,
    closeCommentsModal,
    isMultiArtistModalOpen,
    multiArtistCandidates,
    multiArtistSongTitle,
    navigateToSongArtist,
    navigateToSongAlbum,
    openPlaylistDetail,
    openLikedSongsDetail,
    openRecentHistoryDetail,
    openPurchasedSongsDetail,
    syncListeningGradeCloud,
    refreshCurrentPlaylistOnly,
    updatePlaylistMeta,
    batchAddSongsToPlaylist,
    batchRemoveSongsFromPlaylist,
    artistSongsPage,
    artistSongsHasMore,
    isLoadingMoreArtistSongs,
    loadMoreArtistSongs,
    focusTodayCompletedCount,
    focusTodayTotalMinutes,
    ambientSyncWithMusic,
    recordCompletedFocusSession,
    skipExistingDownloaded: skipExistingFiles,
    hotKeywords: hotSearchKeywords,
    artistSongSort: artistSongsSort,
    switchArtistSongSort,
    eqBandTopologies: eqBandTypes,
    masterPreampDb,
    setBandFilterTopology,
    activeTab,
    isSearchPaletteOpen,
    isNowPlayingOpen,
    isLoginModalOpen,
    isVipCenterOpen,
    isAccountManagerOpen,
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
    shareModalSong,
    hdArtworkPreviewUrl,
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
    colorScheme,
    resolvedColorScheme,
    isDarkTheme,
    isSyncingListeningGrade,
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
    createdOrderIds,
    collectedOrderIds,
    artistsOrderIds,
    purchasedOrderIds,
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
    rhythmTelemetry,
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
    reorderLibraryAssetList,
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
    insertPlayNext,
    removeSongFromQueue,
    clearPlayQueue,
    moveQueueItem,
    reorderQueueByMood,
    switchToRecentQueue,
    removeRecentQueue,
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
    refreshCacheStats,
    clearCacheCategory,
    broadcastDesktopLyricState,
    handleLoginSuccess,
    logout,
  };
});
