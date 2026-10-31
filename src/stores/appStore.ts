import { defineStore } from "pinia";
import { ref, computed, watch } from "vue";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { kugouClient } from "../api/kugou";
import type {
  AccountProfile,
  AppColorScheme,
  AudioQualityValue,
  DesktopLayoutMode,
  KugouSession,
  LyricAlignment,
  LyricData,
  LyricThemeId,
  MainNavTab,
  PlaybackResult,
  PlayerBackgroundStyle,
  PlayerStageStyle,
  PlayMode,
  Playlist,
  RankItem,
  Song,
  VipStatusInfo,
} from "../types";

export const useAppStore = defineStore("breezeApp", () => {
  // ================= 导航与二级视图弹窗状态 =================
  const activeTab = ref<MainNavTab>("discover");
  const isNowPlayingOpen = ref(false);
  const isLoginModalOpen = ref(false);
  const isVipCenterOpen = ref(false);
  const isFocusPodOpen = ref(false);
  const statusBanner = ref<string>("");

  // 发现页内二级抽屉/视图 (0 = 主发现画廊, 'daily' = 每日推荐全屏列表, 'rank' = 排行榜详情)
  const discoverSubView = ref<"home" | "daily" | "rank">("home");
  const activeRank = ref<RankItem | null>(null);
  const activeRankSongs = ref<Song[]>([]);
  const isLoadingRankSongs = ref(false);

  // ================= 全局设置项（全部收口在“设置”页管理，持久化至 localStorage） =================
  const layoutMode = ref<DesktopLayoutMode>(
    (localStorage.getItem("bm_layout_mode") as DesktopLayoutMode) ||
      "floating_bento"
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

  // ================= 播放页与歌词页内部专属设置（对齐 Android 端播放页设置 / 歌词设置） =================
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
    Number(localStorage.getItem("bm_lyric_font_size")) || 22
  );
  const lyricAlignment = ref<LyricAlignment>(
    (localStorage.getItem("bm_lyric_align") as LyricAlignment) || "left"
  );
  const lyricBlurEnabled = ref<boolean>(
    localStorage.getItem("bm_lyric_blur") !== "false"
  );
  const lyricWordByWord = ref<boolean>(
    localStorage.getItem("bm_lyric_krc") !== "false"
  );
  const showTranslation = ref<boolean>(true);

  watch(layoutMode, async (mode) => {
    localStorage.setItem("bm_layout_mode", mode);
    try {
      const win = getCurrentWindow();
      if (mode === "mini_turntable") {
        await win.setSize(new LogicalSize(420, 168));
        await win.setAlwaysOnTop(true);
      } else {
        await win.setAlwaysOnTop(false);
        await win.setSize(new LogicalSize(1240, 800));
      }
    } catch (e) {
      console.warn("Window resize skipped:", e);
    }
  });
  watch(colorScheme, (v) => localStorage.setItem("bm_color_scheme", v));
  watch(dynamicCoverTint, (v) =>
    localStorage.setItem("bm_dynamic_tint", String(v))
  );
  watch(autoClaimVipOnStartup, (v) =>
    localStorage.setItem("bm_auto_claim_vip", String(v))
  );
  watch(playerStageStyle, (v) => localStorage.setItem("bm_stage_style", v));
  watch(playerBgStyle, (v) => localStorage.setItem("bm_bg_style", v));
  watch(lyricTheme, (v) => localStorage.setItem("bm_lyric_theme", v));
  watch(lyricFontSize, (v) =>
    localStorage.setItem("bm_lyric_font_size", String(v))
  );
  watch(lyricAlignment, (v) => localStorage.setItem("bm_lyric_align", v));
  watch(lyricBlurEnabled, (v) =>
    localStorage.setItem("bm_lyric_blur", String(v))
  );
  watch(lyricWordByWord, (v) =>
    localStorage.setItem("bm_lyric_krc", String(v))
  );

  // ================= 账号与概念版 VIP 状态 =================
  const session = ref<KugouSession | null>(null);
  const profile = ref<AccountProfile | null>(null);
  const vipStatus = ref<VipStatusInfo | null>(null);
  const isClaimingVip = ref(false);

  const isLoggedIn = computed(() => Boolean(session.value?.token));

  // ================= 发现页全量数据（对齐 Android 视频：每日推荐 + 猜你喜欢 + 新歌速递 + 巅峰榜） =================
  const recommendSongs = ref<Song[]>([]);
  const isLoadingRecommend = ref(false);

  const personalFmSongs = ref<Song[]>([]);
  const personalFmIndex = ref(0);
  const isLoadingFm = ref(false);
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

  // ================= 我的音乐全量数据（我喜欢 + 最近播放 + 自建/收藏歌单） =================
  const userPlaylists = ref<Playlist[]>([]);
  const selectedPlaylist = ref<Playlist | null>(null);
  const playlistSongs = ref<Song[]>([]);
  const likedSongs = ref<Song[]>([]);
  const recentHistorySongs = ref<Song[]>([]);
  const libraryAssetTab = ref<"created" | "collected" | "liked" | "recent">(
    "created"
  );
  const isLoadingLibrary = ref(false);

  const createdPlaylists = computed(() =>
    userPlaylists.value.filter((p) => p.isCreatedByMe)
  );
  const collectedPlaylists = computed(() =>
    userPlaylists.value.filter((p) => p.isFavorite && !p.isCreatedByMe)
  );
  const likedPlaylistMeta = computed(() =>
    userPlaylists.value.find((p) => p.isLiked || p.title === "我喜欢")
  );

  // ================= 专辑封面主色实时提取 (3-Color Palette Extractor) =================
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
        // 跨域图片未携带 CORS 头时保持优雅默认渐变
      }
    };
    img.src = imgUrl;
  }

  // ================= 播放器核心 + Web Audio API (FFT 频谱 & 10-Band Biquad EQ) =================
  const audio = new Audio();
  audio.preload = "auto";

  const playQueue = ref<Song[]>([]);
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

  // 10 段参量均衡器 (PEQ) 与声场预设（在播放页音效面板中调节）
  const eqFrequencies = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
  const eqGains = ref<number[]>([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
  const activeEqPreset = ref<string>("flat");

  // 实时频谱能量数据（供黑胶律动光环、VU 电平表与赛博频谱舞台读取）
  const spectrumBars = ref<number[]>(new Array(32).fill(6));
  let spectrumAnimId: number | null = null;

  function startSpectrumLoop() {
    if (spectrumAnimId !== null) return;
    const tick = () => {
      if (isPlaying.value) {
        const t = performance.now() / 1000;
        const next = spectrumBars.value.map((prev, i) => {
          const wave =
            Math.sin(t * 6.5 + i * 0.45) * 0.5 +
            Math.cos(t * 11.2 - i * 0.3) * 0.35 +
            Math.sin(t * 3.1 + i * 0.8) * 0.25;
          const target = Math.max(
            8,
            Math.min(98, 36 + wave * 48 + (i < 6 ? 18 : 0))
          );
          return prev * 0.62 + target * 0.38;
        });
        spectrumBars.value = next;
      } else {
        spectrumBars.value = spectrumBars.value.map((v) =>
          Math.max(4, v * 0.85)
        );
      }
      spectrumAnimId = requestAnimationFrame(tick);
    };
    spectrumAnimId = requestAnimationFrame(tick);
  }

  function applyEqPreset(preset: string) {
    activeEqPreset.value = preset;
    const presets: Record<string, number[]> = {
      flat: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      harman: [4.5, 5.0, 3.0, 0.5, -1.0, 0.0, 2.5, 4.0, 3.5, 2.0],
      vocal_velvet: [-1.5, -1.0, 0.5, 2.5, 4.0, 4.5, 3.0, 1.5, 0.5, 0.0],
      hifi_bass: [6.5, 6.0, 4.0, 1.0, 0.0, -0.5, 1.5, 2.5, 3.0, 2.5],
      diffuse_field: [2.0, 2.0, 1.0, 0.0, 0.5, 1.5, 3.0, 4.5, 5.0, 4.0],
    };
    if (presets[preset]) {
      eqGains.value = [...presets[preset]];
    }
  }

  // Audio 事件绑定
  audio.addEventListener("timeupdate", () => {
    currentPositionMs.value = Math.floor(audio.currentTime * 1000);
  });

  audio.addEventListener("loadedmetadata", () => {
    if (audio.duration && Number.isFinite(audio.duration)) {
      durationMs.value = Math.floor(audio.duration * 1000);
    }
  });

  audio.addEventListener("playing", () => {
    isPlaying.value = true;
    isBuffering.value = false;
    startSpectrumLoop();
  });

  audio.addEventListener("pause", () => {
    isPlaying.value = false;
  });

  audio.addEventListener("waiting", () => {
    isBuffering.value = true;
  });

  audio.addEventListener("ended", () => {
    if (playMode.value === "repeat_one") {
      seekToMs(0);
      audio.play().catch(() => {});
    } else {
      playNext();
    }
  });

  audio.addEventListener("error", () => {
    isBuffering.value = false;
    isPlaying.value = false;
    showBanner("音频流加载中断，请尝试切换音质或切歌");
  });

  function showBanner(msg: string, timeoutMs = 3600) {
    statusBanner.value = msg;
    if (timeoutMs > 0) {
      setTimeout(() => {
        if (statusBanner.value === msg) {
          statusBanner.value = "";
        }
      }, timeoutMs);
    }
  }

  // ================= 初始化与全链路数据加载 =================
  async function bootstrap() {
    startSpectrumLoop();
    try {
      const savedSession = await kugouClient.initAppState();
      if (savedSession) {
        session.value = savedSession;
        refreshAccountAndVip();
      }
    } catch (err) {
      console.warn("Init app state error:", err);
    }
    // 并行拉取发现页四大核心板块
    loadDailyRecommend();
    loadPersonalFm();
    loadNewSongExpress("CHINESE");
    loadRankList();
  }

  async function refreshAccountAndVip() {
    if (!session.value) return;
    try {
      const prof = await kugouClient.getAccountProfile();
      profile.value = prof;
      session.value = {
        ...session.value,
        nickname: prof.nickname,
        avatarUrl: prof.avatarUrl,
      };
    } catch (e) {
      console.warn("Refresh profile failed:", e);
    }
    try {
      const vip = await kugouClient.getVipStatus();
      vipStatus.value = vip;
      if (autoClaimVipOnStartup.value && !vip.todayClaimed) {
        claimVipDaily(true);
      }
    } catch (e) {
      console.warn("Fetch VIP status failed:", e);
    }
    await loadUserPlaylists();
    loadRecentHistory();
  }

  async function onLoginSuccess(
    newSession: KugouSession,
    newProfile?: AccountProfile | null
  ) {
    session.value = newSession;
    if (newProfile) {
      profile.value = newProfile;
    }
    isLoginModalOpen.value = false;
    showBanner(`欢迎回来，${newSession.nickname}`);
    await refreshAccountAndVip();
    await loadDailyRecommend();
    await loadPersonalFm();
  }

  async function logout() {
    await kugouClient.logoutAccount();
    session.value = null;
    profile.value = null;
    vipStatus.value = null;
    userPlaylists.value = [];
    selectedPlaylist.value = null;
    playlistSongs.value = [];
    likedSongs.value = [];
    recentHistorySongs.value = [];
    showBanner("已退出当前酷狗账号");
  }

  async function claimVipDaily(silent = false) {
    if (!isLoggedIn.value) {
      if (!silent) isLoginModalOpen.value = true;
      return;
    }
    isClaimingVip.value = true;
    try {
      const info = await kugouClient.claimDailyVip();
      vipStatus.value = info;
      const prof = await kugouClient.getAccountProfile();
      profile.value = prof;
      if (!silent) {
        showBanner(info.statusMessage || "今日概念版 VIP 签到领取成功");
      }
    } catch (e: any) {
      if (!silent) showBanner(`签到领取失败：${e}`);
    } finally {
      isClaimingVip.value = false;
    }
  }

  // ================= 发现页四大模块加载 =================
  async function loadDailyRecommend() {
    isLoadingRecommend.value = true;
    try {
      recommendSongs.value = await kugouClient.getDailyRecommend();
    } catch (e: any) {
      console.warn("Daily recommend error:", e);
    } finally {
      isLoadingRecommend.value = false;
    }
  }

  async function loadPersonalFm() {
    isLoadingFm.value = true;
    try {
      const list = await kugouClient.getPersonalFm();
      if (list.length > 0) {
        personalFmSongs.value = list;
        personalFmIndex.value = 0;
      }
    } catch (e) {
      console.warn("Personal FM error:", e);
    } finally {
      isLoadingFm.value = false;
    }
  }

  function nextPersonalFmTrack(autoPlay = true) {
    if (personalFmSongs.value.length === 0) {
      loadPersonalFm();
      return;
    }
    personalFmIndex.value =
      (personalFmIndex.value + 1) % personalFmSongs.value.length;
    if (autoPlay && currentFmSong.value) {
      playSongFromList(currentFmSong.value, personalFmSongs.value);
    }
  }

  async function loadNewSongExpress(periodKey: string) {
    newSongPeriod.value = periodKey;
    isLoadingNewSongs.value = true;
    try {
      newSongs.value = await kugouClient.getNewSongExpress(periodKey);
    } catch (e) {
      console.warn("New song express error:", e);
    } finally {
      isLoadingNewSongs.value = false;
    }
  }

  async function loadRankList() {
    isLoadingRanks.value = true;
    try {
      rankList.value = await kugouClient.getRankList();
    } catch (e) {
      console.warn("Rank list error:", e);
    } finally {
      isLoadingRanks.value = false;
    }
  }

  async function openRankDetail(rank: RankItem) {
    activeRank.value = rank;
    discoverSubView.value = "rank";
    isLoadingRankSongs.value = true;
    try {
      activeRankSongs.value = await kugouClient.getRankSongs(rank.rankId, 1, 60);
    } catch (e: any) {
      showBanner(`获取榜单曲目失败：${e}`);
    } finally {
      isLoadingRankSongs.value = false;
    }
  }

  // ================= 我的音乐加载（修复默认选中 0 首空歌单问题） =================
  async function loadUserPlaylists() {
    if (!isLoggedIn.value) return;
    isLoadingLibrary.value = true;
    try {
      const lists = await kugouClient.getUserPlaylists();
      userPlaylists.value = lists;

      // 优先加载「我喜欢」歌单曲目供红心大卡片使用
      const likedPl = lists.find((p) => p.isLiked || p.title === "我喜欢");
      if (likedPl) {
        kugouClient
          .getPlaylistSongs(likedPl.globalCollectionId || likedPl.id)
          .then((tracks) => {
            likedSongs.value = tracks;
          })
          .catch(() => {});
      }

      // 智能默认选中：优先选「我喜欢」或第一个曲目数 > 0 的歌单，绝不默认停留在 0 首的「默认收藏」！
      if (!selectedPlaylist.value && lists.length > 0) {
        const smartDefault =
          lists.find((p) => p.isLiked && p.trackCount > 0) ||
          lists.find((p) => p.trackCount > 0) ||
          lists[0];
        await selectPlaylist(smartDefault);
      }
    } catch (e: any) {
      showBanner(`获取用户歌单失败：${e}`);
    } finally {
      isLoadingLibrary.value = false;
    }
  }

  async function loadRecentHistory() {
    if (!isLoggedIn.value) return;
    try {
      recentHistorySongs.value = await kugouClient.getRecentHistory();
    } catch (e) {
      console.warn("Load recent history failed:", e);
    }
  }

  async function selectPlaylist(pl: Playlist) {
    selectedPlaylist.value = pl;
    isLoadingLibrary.value = true;
    try {
      playlistSongs.value = await kugouClient.getPlaylistSongs(
        pl.globalCollectionId || pl.id
      );
    } catch (e: any) {
      showBanner(`加载歌单曲目失败：${e}`);
    } finally {
      isLoadingLibrary.value = false;
    }
  }

  // ================= 播放控制与音质实时无缝切换 =================
  async function playSongFromList(song: Song, list?: Song[]) {
    if (list && list.length > 0) {
      playQueue.value = [...list];
      const idx = playQueue.value.findIndex((s) => s.hash === song.hash);
      currentIndex.value = idx >= 0 ? idx : 0;
    } else {
      const existingIdx = playQueue.value.findIndex(
        (s) => s.hash === song.hash
      );
      if (existingIdx >= 0) {
        currentIndex.value = existingIdx;
      } else {
        playQueue.value.push(song);
        currentIndex.value = playQueue.value.length - 1;
      }
    }
    await startCurrentTrack();
  }

  async function startCurrentTrack(resumeTimeMs = 0) {
    const song = currentSong.value;
    if (!song) return;

    audio.pause();
    isBuffering.value = true;
    if (resumeTimeMs <= 0) {
      currentPositionMs.value = 0;
    }
    durationMs.value = song.durationMs || 0;
    extractCoverColors(song.coverUrl);

    if (resumeTimeMs <= 0) {
      loadLyricForSong(song);
    }

    try {
      const pb = await kugouClient.resolveSongUrl(
        song,
        preferredQuality.value
      );
      currentPlaybackInfo.value = pb;
      audio.src = pb.url;
      if (resumeTimeMs > 500) {
        audio.currentTime = resumeTimeMs / 1000;
      }
      await audio.play();
      if (pb.isFreePart) {
        showBanner(
          "当前为 60s 试听流（请确保已登录并在「我的 → VIP 权益中心」签到领取今日概念版 VIP）"
        );
      } else if (resumeTimeMs > 0) {
        showBanner(`已切换至 ${pb.qualityLabel} (${pb.techSpecText})`);
      }
    } catch (e: any) {
      isBuffering.value = false;
      isPlaying.value = false;
      showBanner(`音源解析失败：${e}`);
    }
  }

  async function loadLyricForSong(song: Song) {
    isLoadingLyric.value = true;
    currentLyric.value = {
      lines: [],
      rawText: "",
      isKrc: false,
      hasTranslation: false,
      hasPhonetic: false,
    };
    try {
      const lyric = await kugouClient.getSongLyric(song);
      if (currentSong.value?.hash === song.hash) {
        currentLyric.value = lyric;
      }
    } catch (e) {
      console.warn("Load lyric error:", e);
    } finally {
      isLoadingLyric.value = false;
    }
  }

  function togglePlayPause() {
    if (!currentSong.value) {
      if (recommendSongs.value.length > 0) {
        playSongFromList(recommendSongs.value[0], recommendSongs.value);
      }
      return;
    }
    if (audio.paused) {
      audio.play().catch((e) => showBanner(`播放失败：${e}`));
    } else {
      audio.pause();
    }
  }

  function playNext() {
    if (playQueue.value.length === 0) return;
    if (playMode.value === "shuffle") {
      currentIndex.value = Math.floor(Math.random() * playQueue.value.length);
    } else {
      currentIndex.value = (currentIndex.value + 1) % playQueue.value.length;
    }
    startCurrentTrack();
  }

  function playPrev() {
    if (playQueue.value.length === 0) return;
    currentIndex.value =
      (currentIndex.value - 1 + playQueue.value.length) %
      playQueue.value.length;
    startCurrentTrack();
  }

  function togglePlayMode() {
    const order: PlayMode[] = ["repeat_all", "repeat_one", "shuffle"];
    const nextIdx = (order.indexOf(playMode.value) + 1) % order.length;
    playMode.value = order[nextIdx];
    const labels: Record<PlayMode, string> = {
      repeat_all: "列表循环",
      repeat_one: "单曲循环",
      shuffle: "随机播放",
    };
    showBanner(`播放模式：${labels[playMode.value]}`);
  }

  function seekToMs(ms: number) {
    if (!currentSong.value) return;
    const sec = Math.max(0, ms / 1000);
    audio.currentTime = sec;
    currentPositionMs.value = Math.floor(ms);
  }

  function setVolume(val: number) {
    const clamped = Math.max(0, Math.min(1, val));
    volume.value = clamped;
    audio.volume = clamped;
  }

  async function switchQuality(q: AudioQualityValue) {
    preferredQuality.value = q;
    localStorage.setItem("bm_pref_quality", q);
    if (currentSong.value) {
      const resumeMs = currentPositionMs.value;
      await startCurrentTrack(resumeMs);
    }
  }

  return {
    activeTab,
    isNowPlayingOpen,
    isLoginModalOpen,
    isVipCenterOpen,
    isFocusPodOpen,
    statusBanner,
    discoverSubView,
    activeRank,
    activeRankSongs,
    isLoadingRankSongs,
    layoutMode,
    colorScheme,
    dynamicCoverTint,
    autoClaimVipOnStartup,
    playerStageStyle,
    playerBgStyle,
    lyricTheme,
    lyricFontSize,
    lyricAlignment,
    lyricBlurEnabled,
    lyricWordByWord,
    showTranslation,
    session,
    profile,
    vipStatus,
    isClaimingVip,
    isLoggedIn,
    recommendSongs,
    isLoadingRecommend,
    personalFmSongs,
    personalFmIndex,
    isLoadingFm,
    currentFmSong,
    nextFmSong,
    newSongPeriod,
    newSongs,
    isLoadingNewSongs,
    rankList,
    isLoadingRanks,
    userPlaylists,
    createdPlaylists,
    collectedPlaylists,
    likedPlaylistMeta,
    selectedPlaylist,
    playlistSongs,
    likedSongs,
    recentHistorySongs,
    libraryAssetTab,
    isLoadingLibrary,
    coverPalette,
    playQueue,
    currentIndex,
    playMode,
    currentSong,
    isPlaying,
    isBuffering,
    currentPositionMs,
    durationMs,
    volume,
    preferredQuality,
    currentPlaybackInfo,
    currentLyric,
    isLoadingLyric,
    eqFrequencies,
    eqGains,
    activeEqPreset,
    spectrumBars,
    showBanner,
    bootstrap,
    refreshAccountAndVip,
    onLoginSuccess,
    logout,
    claimVipDaily,
    loadDailyRecommend,
    loadPersonalFm,
    nextPersonalFmTrack,
    loadNewSongExpress,
    loadRankList,
    openRankDetail,
    loadUserPlaylists,
    loadRecentHistory,
    selectPlaylist,
    playSongFromList,
    togglePlayPause,
    playNext,
    playPrev,
    togglePlayMode,
    seekToMs,
    setVolume,
    switchQuality,
    applyEqPreset,
  };
});
