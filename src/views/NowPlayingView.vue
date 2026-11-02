<script setup lang="ts">
import { computed, ref, watch, nextTick } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import type {
  LyricAlignment,
  LyricThemeId,
  PlayerBackgroundStyle,
  PlayerStageStyle,
  ReverbPresetId,
} from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();
const lyricListRef = ref<HTMLElement | null>(null);

// 播放页内部专属抽屉面板开关
const activeOverlayPanel = ref<
  "none" | "stage_bg" | "lyric_cfg" | "dsp_eq" | "comments" | "queue" | "more"
>("none");

// 母带 DSP 面板内子 Tab：'viper' = 酷狗蝰蛇音效库, 'peq' = 10段参量均衡器, 'field' = 声场调音工作台
const dspSubTab = ref<"viper" | "peq" | "field">("viper");
const selectedViperCategory = ref<string>("全部");

// 评论发送框与回复状态
const newCommentText = ref<string>("");
const replyTargetComment = ref<{ id: string; userName: string } | null>(null);
const isSubmittingComment = ref<boolean>(false);

// 播放队列子 Tab：'current' = 当前队列, 'recent' = 最近播放过的队列层叠卡
const queueSubTab = ref<"current" | "recent">("current");

// 高清 1000P 封面大图预览弹窗
const showHdCoverPreview = ref(false);

const stageStyles: { id: PlayerStageStyle; label: string; desc: string }[] = [
  {
    id: "vinyl_turntable",
    label: "流光黑胶唱盘",
    desc: "S型液压平衡唱臂 + 实时声波律动外环",
  },
  {
    id: "gallery_poster",
    label: "流光棱镜方封",
    desc: "3D 呼吸微倾圆角方图 + 色散流光边框",
  },
  {
    id: "retro_cassette",
    label: "复古磁带与电平",
    desc: "机械双轮磁带窗 + 暖光 VU 电平表",
  },
  {
    id: "cyber_spectrum",
    label: "赛博频谱舞台",
    desc: "32 段对称动态频谱 + 悬浮封面",
  },
];

const bgStyles: { id: PlayerBackgroundStyle; label: string }[] = [
  { id: "fluid_aurora", label: "流体极光" },
  { id: "rhythm_halo", label: "律动光晕" },
  { id: "cover_blur", label: "封面虚化" },
  { id: "starlight_nebula", label: "暗夜星尘" },
  { id: "deep_oled", label: "深邃纯黑" },
];

// 对齐 Android 端 2x3 双行三列六大四字主题名
const lyricThemes: {
  id: LyricThemeId;
  label: string;
  c1: string;
  c2: string;
}[] = [
  { id: "ice_blue", label: "冰晶青蓝", c1: "#00e5ff", c2: "#2b8cff" },
  { id: "aurora_purple", label: "极光幻紫", c1: "#b388ff", c2: "#6c5ce7" },
  { id: "amber_gold", label: "鎏金琥珀", c1: "#ffd54f", c2: "#f59e0b" },
  { id: "neon_coral", label: "霓虹珊瑚", c1: "#ff6b8b", c2: "#ff6b5b" },
  { id: "emerald_mint", label: "翡翠薄荷", c1: "#00e5a3", c2: "#00b894" },
  { id: "dynamic_cover", label: "封面随动", c1: "#3898ff", c2: "#00c4a7" },
];

const activeLyricColors = computed(() => {
  if (store.lyricTheme === "dynamic_cover") {
    return {
      c1: store.coverPalette[0],
      c2: store.coverPalette[1],
    };
  }
  const found = lyricThemes.find((t) => t.id === store.lyricTheme);
  return found
    ? { c1: found.c1, c2: found.c2 }
    : { c1: "#00e5ff", c2: "#2b8cff" };
});

// 校准后的歌词毫秒位置
const npProgressPercent = computed(() => {
  if (!store.durationMs || store.durationMs <= 0) return 0;
  return Math.min(100, Math.max(0, (store.currentPositionMs / store.durationMs) * 100));
});

function handleNpSeekMouseDown(e: MouseEvent) {
  const bar = e.currentTarget as HTMLElement;
  const updateSeek = (ev: MouseEvent) => {
    const rect = bar.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (ev.clientX - rect.left) / rect.width));
    store.seekToMs(ratio * (store.durationMs || 0));
  };
  updateSeek(e);
  const onMove = (ev: MouseEvent) => updateSeek(ev);
  const onUp = () => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  };
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}

const calibratedPosMs = computed(() => {
  return (
    store.currentPositionMs + store.lyricOffsetMs - store.bluetoothDelayMs
  );
});

const activeLineIndex = computed(() => {
  const lines = store.currentLyric.lines;
  if (!lines.length) return -1;
  const pos = calibratedPosMs.value;
  let idx = 0;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].startTimeMs <= pos) {
      idx = i;
    } else {
      break;
    }
  }
  return idx;
});

// O(1) 前奏/间奏三点呼吸波点检测 (对齐 Android hasInterludeCandidateAt)
const isInterludeActive = computed(() => {
  const lines = store.currentLyric.lines;
  if (!lines.length || !store.isPlaying) return false;
  const pos = calibratedPosMs.value;
  if (lines[0].startTimeMs > 4000 && pos < lines[0].startTimeMs - 400) {
    return true;
  }
  const idx = activeLineIndex.value;
  if (idx >= 0 && idx + 1 < lines.length) {
    const curEnd =
      lines[idx].startTimeMs + Math.max(1800, lines[idx].durationMs);
    const nextStart = lines[idx + 1].startTimeMs;
    if (nextStart - curEnd >= 4800 && pos > curEnd && pos < nextStart - 400) {
      return true;
    }
  }
  return false;
});

watch(activeLineIndex, async (newIdx) => {
  if (newIdx < 0 || !lyricListRef.value) return;
  await nextTick();
  const container = lyricListRef.value;
  const activeEl = container.querySelector(
    `.krc-line-item[data-idx="${newIdx}"]`
  ) as HTMLElement | null;
  if (activeEl) {
    const targetTop =
      activeEl.offsetTop -
      container.clientHeight / 2 +
      activeEl.clientHeight / 2;
    container.scrollTo({
      top: Math.max(0, targetTop),
      behavior: "smooth",
    });
  }
});

function wordFillPercent(
  lineStartMs: number,
  wordStartMs: number,
  wordDurMs: number
): number {
  const pos = calibratedPosMs.value;
  const absStart =
    wordStartMs >= lineStartMs ? wordStartMs : lineStartMs + wordStartMs;
  if (pos <= absStart) return 0;
  if (pos >= absStart + Math.max(80, wordDurMs)) return 100;
  return Math.min(100, ((pos - absStart) / Math.max(80, wordDurMs)) * 100);
}

// 阶梯景深虚化计算 (对齐 Android lyricBlurEnabled & lyricBlurIntensity)
function lineDepthBlurFilter(idx: number): string {
  if (!store.lyricBlurEnabled || idx === activeLineIndex.value) return "none";
  const dist = Math.abs(idx - activeLineIndex.value);
  const px = Math.min(8, dist * 0.85 * (store.lyricBlurIntensity / 3.2));
  return `blur(${px.toFixed(1)}px)`;
}

const avgEnergy = computed(() => {
  const bars = store.spectrumBars;
  const sum = bars.slice(0, 12).reduce((a, b) => a + b, 0);
  return Math.round((sum / 12) * store.playMomentum);
});

const viperCategories = ["全部", "官方旗舰", "3D空间", "复古介质", "曲风人声"];

const filteredViperEffects = computed(() => {
  if (selectedViperCategory.value === "全部") return store.viperEffects;
  return store.viperEffects.filter(
    (v) => v.category === selectedViperCategory.value
  );
});

const eqPresetList = [
  { id: "flat", name: "监听平直 (0dB)" },
  { id: "harman", name: "哈曼目标曲线" },
  { id: "diffuse_field", name: "弥散场参考" },
  { id: "vocal_velvet", name: "人声丝绒" },
  { id: "hifi_sub", name: "HiFi 深潜低频" },
  { id: "acg_bright", name: "ACG 空气泛音" },
];

const crossoverFreqs = [
  { hz: 40, label: "40Hz 极深潜" },
  { hz: 55, label: "55Hz 影院级" },
  { hz: 68, label: "68Hz 黄金点" },
  { hz: 80, label: "80Hz 弹性鼓" },
  { hz: 95, label: "95Hz 丰满度" },
  { hz: 110, label: "110Hz 暖厚感" },
];

const reverbPresets: { id: ReverbPresetId; label: string }[] = [
  { id: "none", label: "无混响" },
  { id: "small_room", label: "小型房间" },
  { id: "medium_room", label: "中型房间" },
  { id: "large_room", label: "大型场馆" },
  { id: "medium_hall", label: "音乐大厅" },
  { id: "large_hall", label: "混响礼堂" },
  { id: "plate", label: "录音室板式" },
];

const hdCoverUrl = computed(() => {
  const raw = store.currentSong?.coverUrl || "";
  return raw.replace("/400/", "/1000/");
});

function togglePanel(
  panel:
    | "stage_bg"
    | "lyric_cfg"
    | "dsp_eq"
    | "comments"
    | "queue"
    | "more"
) {
  activeOverlayPanel.value =
    activeOverlayPanel.value === panel ? "none" : panel;
}

async function handleSendComment() {
  if (!newCommentText.value.trim()) return;
  isSubmittingComment.value = true;
  try {
    const ok = await store.submitSongComment(
      newCommentText.value.trim(),
      replyTargetComment.value?.id
    );
    if (ok) {
      newCommentText.value = "";
      replyTargetComment.value = null;
    }
  } finally {
    isSubmittingComment.value = false;
  }
}
</script>

<template>
  <div
    class="now-playing-stage"
    :class="[`bg-${store.playerBgStyle}`]"
    :style="{
      '--lyric-c1': activeLyricColors.c1,
      '--lyric-c2': activeLyricColors.c2,
      '--halo-scale': 1 + (avgEnergy / 100) * 0.24,
      '--momentum': store.playMomentum,
    }"
  >
    <!-- ==================== 1. 五款高定动态光学背景层（含 playMomentum 连续相位过渡） ==================== -->
    <div
      v-if="store.playerBgStyle === 'cover_blur' && store.currentSong?.coverUrl"
      class="np-bg-cover-blur"
      :style="{ backgroundImage: `url(${store.currentSong.coverUrl})` }"
    ></div>

    <div
      v-else-if="store.playerBgStyle === 'fluid_aurora'"
      class="np-bg-fluid-aurora"
    >
      <div
        class="aurora-blob b1"
        :style="{ background: store.coverPalette[0] }"
      ></div>
      <div
        class="aurora-blob b2"
        :style="{ background: store.coverPalette[1] }"
      ></div>
      <div
        class="aurora-blob b3"
        :style="{ background: store.coverPalette[2] }"
      ></div>
    </div>

    <div
      v-else-if="store.playerBgStyle === 'rhythm_halo'"
      class="np-bg-rhythm-halo"
    >
      <div class="halo-ring r1"></div>
      <div class="halo-ring r2"></div>
      <div class="halo-ring r3"></div>
    </div>

    <div
      v-else-if="store.playerBgStyle === 'starlight_nebula'"
      class="np-bg-starlight"
    >
      <div class="nebula-cloud c1"></div>
      <div class="nebula-cloud c2"></div>
      <div class="star-dots"></div>
    </div>

    <!-- ==================== 2. 平板级左右黄金分割主舞台 (左 46% 封面控播 : 右 54% 无框 KRC 歌词) ==================== -->
    <div class="np-main-columns">
      <!-- 左栏 (46%)：大尺寸主角舞台 + 右下角竖向三联软润锚点 + 底部沉降式统一控播坞 -->
      <div class="np-left-stage">
        <div
          class="np-visual-stage-container"
          @dblclick="store.toggleFavoriteSong(store.currentSong)"
          @contextmenu.prevent="showHdCoverPreview = true"
        >
          <!-- 形态 1：流光黑胶唱盘 (MasterVinylTurntableView) -->
          <div
            v-if="store.playerStageStyle === 'vinyl_turntable'"
            class="stage-vinyl-box"
          >
            <div class="vinyl-outer-halo"></div>
            <div
              class="tonearm-assembly"
              :class="{ engaged: store.isPlaying }"
            >
              <div class="tonearm-pivot"></div>
              <div class="tonearm-rod"></div>
            </div>
            <div
              class="vinyl-platter"
              :class="{ spinning: store.isPlaying }"
            >
              <div class="vinyl-grooves"></div>
              <img
                v-if="store.currentSong?.coverUrl"
                :src="store.currentSong.coverUrl"
                class="vinyl-center-label"
              />
              <div v-else class="vinyl-center-fallback">
                <PlumpIcon name="disc" :size="36" color="#00e5ff" />
              </div>
              <div class="vinyl-spindle-hole"></div>
            </div>
          </div>

          <!-- 形态 2：3D 呼吸微倾棱镜方封 -->
          <div
            v-else-if="store.playerStageStyle === 'gallery_poster'"
            class="stage-gallery-box"
          >
            <div
              class="gallery-art-frame"
              :class="{ breathing: store.isPlaying }"
            >
              <img
                v-if="store.currentSong?.coverUrl"
                :src="store.currentSong.coverUrl"
                class="gallery-art-img"
              />
              <div v-else class="gallery-art-fallback">
                <PlumpIcon name="disc" :size="48" />
              </div>
            </div>
          </div>

          <!-- 形态 3：复古磁带机 + 双通道暖光 VU 电平表 -->
          <div
            v-else-if="store.playerStageStyle === 'retro_cassette'"
            class="stage-cassette-box"
          >
            <div class="cassette-shell">
              <div class="cassette-top-label">
                <span class="cassette-badge">TYPE IV · METAL MASTER</span>
                <span class="cassette-song-title">
                  {{ store.currentSong?.title || "BreezeMusic" }}
                </span>
              </div>
              <div class="cassette-window">
                <div
                  class="cassette-reel"
                  :class="{ spinning: store.isPlaying }"
                ></div>
                <div class="cassette-tape-bridge">
                  <img
                    v-if="store.currentSong?.coverUrl"
                    :src="store.currentSong.coverUrl"
                    class="cassette-mini-cover"
                  />
                </div>
                <div
                  class="cassette-reel"
                  :class="{ spinning: store.isPlaying }"
                ></div>
              </div>
              <div class="vu-meters-row">
                <div class="vu-channel">
                  <span>L</span>
                  <div class="vu-bar-track">
                    <div
                      class="vu-bar-fill"
                      :style="{ width: `${Math.min(100, avgEnergy + 12)}%` }"
                    ></div>
                  </div>
                </div>
                <div class="vu-channel">
                  <span>R</span>
                  <div class="vu-bar-track">
                    <div
                      class="vu-bar-fill"
                      :style="{ width: `${Math.min(100, avgEnergy + 6)}%` }"
                    ></div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 形态 4：赛博 32 段对称频谱舞台 -->
          <div
            v-else-if="store.playerStageStyle === 'cyber_spectrum'"
            class="stage-spectrum-box"
          >
            <div class="cyber-cover-card">
              <img
                v-if="store.currentSong?.coverUrl"
                :src="store.currentSong.coverUrl"
                class="cyber-cover-img"
              />
            </div>
            <div class="cyber-bars-row">
              <div
                v-for="(val, i) in store.spectrumBars"
                :key="i"
                class="cyber-bar"
                :style="{ height: `${val}%` }"
              ></div>
            </div>
          </div>

          <!-- 平板横屏专属：舞台右下角竖向三联软润按钮 -->
          <div class="stage-right-vertical-orbs" @click.stop>
            <button
              class="stage-orb-btn"
              title="红心收藏 (双击封面亦可触发)"
              @click="store.toggleFavoriteSong(store.currentSong)"
            >
              <PlumpIcon
                name="heart"
                :size="20"
                :filled="store.isSongLiked(store.currentSong)"
                :color="
                  store.isSongLiked(store.currentSong) ? '#ff4d6d' : '#ffffff'
                "
              />
            </button>

            <button
              class="stage-orb-btn"
              title="添加到自建歌单"
              @click="
                store.currentSong &&
                  (store.addToPlaylistModalSong = store.currentSong)
              "
            >
              <PlumpIcon
                name="playlist-add"
                :size="20"
                color="#ffffff"
                accent="var(--cyan)"
              />
            </button>

            <button
              class="stage-orb-btn"
              title="生成 2K 超清歌词海报"
              @click="store.isLyricPosterOpen = true"
            >
              <PlumpIcon
                name="poster"
                :size="20"
                color="#ffffff"
                accent="var(--cyan)"
              />
            </button>
          </div>
        </div>

        <!-- 左下沉降式统一控播与信息坞 -->
        <div class="np-bottom-control-dock">
          <div class="np-song-info-block">
            <h2 class="np-song-title" @click="showHdCoverPreview = true">
              {{ store.currentSong?.title || "未播放歌曲" }}
            </h2>
            <p class="np-song-sub">
              <span
                class="np-clickable-link"
                title="点击前往歌手主页"
                @click="
                  store.currentSong &&
                    store.openArtistDetail(
                      store.currentSong.artistId,
                      store.currentSong.artist,
                      store.currentSong.coverUrl
                    )
                "
              >
                {{ store.currentSong?.artist || "微风音乐" }}
              </span>
              <span
                v-if="store.currentSong?.album"
                class="np-clickable-link"
                title="点击前往专辑详情"
                @click="
                  store.currentSong &&
                    store.openAlbumDetail(
                      store.currentSong.albumId,
                      store.currentSong.album,
                      store.currentSong.artist,
                      store.currentSong.coverUrl
                    )
                "
              >
                · {{ store.currentSong.album }}
              </span>
            </p>

            <div class="np-tech-badges">
              <span
                class="tech-pill hires"
                style="cursor: pointer"
                title="点击查看音频流硬件百科详情 (STREAMINFO)"
                @click="
                  store.currentSong &&
                    store.openSongDetailModal(store.currentSong)
                "
              >
                <PlumpIcon name="sparkles" :size="13" color="var(--cyan)" />
                <span>
                  {{
                    store.currentPlaybackInfo?.qualityLabel ||
                    store.currentSong?.highestQuality ||
                    "Hi-Res"
                  }}
                </span>
              </span>
              <span
                v-if="store.currentPlaybackInfo?.techSpecText"
                class="tech-pill spec"
                style="cursor: pointer"
                title="RFC 9639 FLAC STREAMINFO 硬件流头实测采样率"
                @click="
                  store.currentSong &&
                    store.openSongDetailModal(store.currentSong)
                "
              >
                {{ store.currentPlaybackInfo.techSpecText }}
              </span>
              <span v-if="store.climaxModeEnabled" class="tech-pill gold">
                副歌速刷 30s
              </span>
              <span v-if="store.ktvAccompanimentMode" class="tech-pill gold">
                KTV 纯伴奏中
              </span>
            </div>
          </div>

          <!-- 播放页内嵌主控台（因底部控制栏进入播放页后自然下移隐藏，此处提供完整进度与播控） -->
          <div class="np-inline-transport-box">
            <div class="pb-progress-timeline-row np-timeline-row">
              <span class="pb-time-label">{{ formatDuration(store.currentPositionMs / 1000) }}</span>
              <div class="pb-timeline-track" @mousedown="handleNpSeekMouseDown">
                <div class="pb-timeline-bg"></div>
                <div
                  class="pb-timeline-fill"
                  :style="{ width: `${npProgressPercent}%` }"
                >
                  <span class="pb-timeline-thumb"></span>
                </div>
              </div>
              <span class="pb-time-label">{{ formatDuration(store.durationMs / 1000) }}</span>
            </div>

            <div class="pb-transport-controls-row np-transport-row">
              <button
                class="pb-ctrl-btn"
                :class="{ active: store.playMode !== 'sequence' }"
                title="切换播放模式"
                @click="store.cyclePlayMode()"
              >
                <PlumpIcon :name="store.playMode === 'shuffle' ? 'shuffle' : store.playMode === 'loop_one' ? 'repeat-one' : 'repeat'" :size="18" />
              </button>
              <button class="pb-ctrl-btn" title="上一曲" @click="store.playPrev()">
                <PlumpIcon name="prev" :size="20" />
              </button>
              <button class="pb-play-hero-circle" title="播放 / 暂停" @click="store.togglePlayPause()">
                <PlumpIcon :name="store.isPlaying ? 'pause' : 'play'" :size="22" />
              </button>
              <button class="pb-ctrl-btn" title="下一曲" @click="store.playNext()">
                <PlumpIcon name="next" :size="20" />
              </button>
              <button
                class="pb-ctrl-btn"
                :class="{ active: store.climaxModeEnabled }"
                title="30秒副歌高潮精听"
                @click="store.climaxModeEnabled = !store.climaxModeEnabled"
              >
                <PlumpIcon name="sparkle" :size="18" />
              </button>
            </div>
          </div>

          <!-- 播放页左下角专属六维软润工具栏 -->
          <div class="np-stage-toolbar">
            <button
              class="np-tool-pill"
              :class="{ active: activeOverlayPanel === 'stage_bg' }"
              @click="togglePanel('stage_bg')"
            >
              <PlumpIcon name="disc" :size="15" />
              <span>舞台与背景</span>
            </button>

            <button
              class="np-tool-pill"
              :class="{ active: activeOverlayPanel === 'lyric_cfg' }"
              @click="togglePanel('lyric_cfg')"
            >
              <PlumpIcon name="lyric" :size="15" />
              <span>歌词设置</span>
            </button>

            <button
              class="np-tool-pill"
              :class="{
                active:
                  activeOverlayPanel === 'dsp_eq' ||
                  store.acousticField.enabled ||
                  store.activeEqPreset !== 'flat' ||
                  store.activeViperEffectId !== 'off',
              }"
              @click="togglePanel('dsp_eq')"
            >
              <PlumpIcon name="wave" :size="15" />
              <span>蝰蛇 / 母带 DSP</span>
            </button>

            <button
              class="np-tool-pill"
              :class="{ active: activeOverlayPanel === 'comments' }"
              @click="togglePanel('comments')"
            >
              <PlumpIcon name="comment" :size="15" />
              <span>评论 ({{ store.activeSongComments.length }})</span>
            </button>

            <button
              class="np-tool-pill"
              :class="{ active: activeOverlayPanel === 'queue' }"
              @click="togglePanel('queue')"
            >
              <PlumpIcon name="queue" :size="15" />
              <span>队列 ({{ store.playQueue.length }})</span>
            </button>

            <button
              class="np-tool-pill"
              :class="{ active: activeOverlayPanel === 'more' }"
              title="倍速、升降调、KTV伴奏、睡眠定时与高清封面"
              @click="togglePanel('more')"
            >
              <PlumpIcon name="more" :size="15" />
            </button>
          </div>
        </div>
      </div>

      <!-- 右栏 (54%)：全高无边框、无滚动条、上下渐隐 KRC 逐字流光歌词舞台 -->
      <div class="np-right-lyrics">
        <div class="krc-stage-header">
          <div class="krc-status-left">
            <span class="krc-status-tag">
              {{
                store.currentLyric.isKrc
                  ? "KRC 逐字流光引擎 · 64-bit Pure LTI DSP"
                  : "沉浸同步歌词"
              }}
            </span>
            <div v-if="isInterludeActive" class="interlude-breathing-dots">
              <span></span><span></span><span></span>
            </div>
          </div>

          <div class="krc-header-pills">
            <button
              class="trans-toggle-pill"
              :class="{ active: store.ktvAccompanimentMode }"
              title="一键切换 KTV 伴奏 / 原唱"
              @click="store.toggleKtvAccompaniment()"
            >
              <span>伴</span>
            </button>
            <button
              v-if="store.currentLyric.hasTranslation"
              class="trans-toggle-pill"
              :class="{ active: store.showTranslation }"
              @click="store.showTranslation = !store.showTranslation"
            >
              <span>译</span>
            </button>
            <button
              v-if="store.currentLyric.hasPhonetic"
              class="trans-toggle-pill"
              :class="{ active: store.showPhonetic }"
              @click="store.showPhonetic = !store.showPhonetic"
            >
              <span>音</span>
            </button>
          </div>
        </div>

        <div
          ref="lyricListRef"
          class="krc-lyrics-viewport"
          :class="[`align-${store.lyricAlignment}`]"
        >
          <div
            v-if="store.currentLyric.lines.length === 0"
            class="krc-empty-state"
          >
            {{
              store.isLoadingLyric
                ? "正在解密 KRC 逐字流光时间轴..."
                : "纯音乐或暂未收录歌词，尽享纯粹无损旋律"
            }}
          </div>

          <template v-else>
            <div class="krc-spacer"></div>
            <div
              v-for="(line, idx) in store.currentLyric.lines"
              :key="idx"
              :data-idx="idx"
              class="krc-line-item"
              :class="{ active: idx === activeLineIndex }"
              :style="{
                fontSize:
                  idx === activeLineIndex
                    ? `${store.lyricFontSize + 5}px`
                    : `${store.lyricFontSize}px`,
                lineHeight: store.lyricLineSpacing,
                filter: lineDepthBlurFilter(idx),
              }"
              @click="store.seekToMs(line.startTimeMs)"
            >
              <div
                v-if="store.lyricWordByWord && line.words.length > 0"
                class="krc-words-row"
              >
                <span
                  v-for="(w, wIdx) in line.words"
                  :key="wIdx"
                  class="krc-word"
                  :style="
                    idx === activeLineIndex
                      ? {
                          '--fill-pct': `${wordFillPercent(
                            line.startTimeMs,
                            w.startTimeMs,
                            w.durationMs
                          )}%`,
                        }
                      : {}
                  "
                >
                  {{ w.text }}
                </span>
              </div>

              <div v-else class="krc-plain-row">
                {{ line.text }}
              </div>

              <div
                v-if="store.showPhonetic && line.phonetic"
                class="krc-trans-row"
                :style="{
                  fontSize: `${Math.round(
                    store.lyricFontSize * store.lyricTranslationScale
                  )}px`,
                }"
              >
                {{ line.phonetic }}
              </div>

              <div
                v-if="store.showTranslation && line.translation"
                class="krc-trans-row"
                :style="{
                  fontSize: `${Math.round(
                    store.lyricFontSize * store.lyricTranslationScale
                  )}px`,
                }"
              >
                {{ line.translation }}
              </div>
            </div>
            <div class="krc-spacer"></div>
          </template>
        </div>
      </div>
    </div>

    <!-- ==================== 3. 播放页内部专属六大浮层抽屉 ==================== -->
    <transition name="pop-spring">
      <div v-if="activeOverlayPanel !== 'none'" class="np-Config-drawer">
        <div class="drawer-head">
          <h4>
            {{
              activeOverlayPanel === "stage_bg"
                ? "播放页舞台形态与动态光学背景"
                : activeOverlayPanel === "lyric_cfg"
                ? "歌词显示设置（六大流光主题 · 排版 · 校准）"
                : activeOverlayPanel === "dsp_eq"
                ? "酷狗蝰蛇声学矩阵 · 64-bit 纯线性母带 DSP & 10段参量均衡器"
                : activeOverlayPanel === "comments"
                ? `歌曲热评与多级楼中楼评论区 (${store.activeSongComments.length})`
                : activeOverlayPanel === "queue"
                ? `播放队列与最近歌单栈 (${store.playQueue.length})`
                : "进阶声学与播放控制工具"
            }}
          </h4>
          <button class="icon-close-btn" @click="activeOverlayPanel = 'none'">
            <PlumpIcon name="close" :size="15" />
          </button>
        </div>

        <!-- 面板 A：舞台形态 × 五款动态背景 -->
        <div v-if="activeOverlayPanel === 'stage_bg'" class="drawer-body">
          <div class="cfg-label">左侧视觉舞台形态（4 选 1）</div>
          <div class="stage-choice-grid">
            <button
              v-for="s in stageStyles"
              :key="s.id"
              class="stage-choice-card"
              :class="{ active: store.playerStageStyle === s.id }"
              @click="store.playerStageStyle = s.id"
            >
              <div class="scc-title">{{ s.label }}</div>
              <div class="scc-desc">{{ s.desc }}</div>
            </button>
          </div>

          <div class="cfg-label mt-14">五款高定动态背景（含暂停连续动量）</div>
          <div class="bg-choice-pills">
            <button
              v-for="b in bgStyles"
              :key="b.id"
              class="bg-pill-btn"
              :class="{ active: store.playerBgStyle === b.id }"
              @click="store.playerBgStyle = b.id"
            >
              {{ b.label }}
            </button>
          </div>
        </div>

        <!-- 面板 B：歌词显示设置（2x3 六大四字主题网格 + 景深 + 时长校准） -->
        <div v-else-if="activeOverlayPanel === 'lyric_cfg'" class="drawer-body">
          <div class="cfg-label">逐字流光配色（2×3 双行三列六大四字主题）</div>
          <div class="lyric-theme-2x3-grid">
            <button
              v-for="t in lyricThemes"
              :key="t.id"
              class="lyric-theme-cell"
              :class="{ active: store.lyricTheme === t.id }"
              @click="store.lyricTheme = t.id"
            >
              <span
                class="theme-color-dot"
                :style="{
                  background: `linear-gradient(135deg, ${t.c1}, ${t.c2})`,
                }"
              ></span>
              <span>{{ t.label }}</span>
            </button>
          </div>

          <div class="lyric-sliders-row mt-14">
            <div class="cfg-col">
              <div class="cfg-label">主词字号：{{ store.lyricFontSize }}px</div>
              <input
                v-model.number="store.lyricFontSize"
                type="range"
                min="18"
                max="34"
                step="1"
              />
            </div>

            <div class="cfg-col">
              <div class="cfg-label">
                时长校准：{{ (store.lyricOffsetMs / 1000).toFixed(1) }}s
              </div>
              <input
                v-model.number="store.lyricOffsetMs"
                type="range"
                min="-2000"
                max="2000"
                step="100"
              />
            </div>

            <div class="cfg-col">
              <div class="cfg-label">对齐与光学景深</div>
              <div class="bg-choice-pills">
                <button
                  v-for="al in (['left', 'center'] as LyricAlignment[])"
                  :key="al"
                  class="bg-pill-btn"
                  :class="{ active: store.lyricAlignment === al }"
                  @click="store.lyricAlignment = al"
                >
                  {{ al === "left" ? "左对齐" : "居中" }}
                </button>
                <button
                  class="bg-pill-btn"
                  :class="{ active: store.lyricBlurEnabled }"
                  @click="store.lyricBlurEnabled = !store.lyricBlurEnabled"
                >
                  阶梯景深虚化
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 面板 C：母带级 DSP 调音台（酷狗蝰蛇音效库 + 10段PEQ + 声场调音工作台） -->
        <div v-else-if="activeOverlayPanel === 'dsp_eq'" class="drawer-body">
          <div class="dsp-top-bar">
            <div class="pill-tabs">
              <button
                class="pill-tab-btn"
                :class="{ active: dspSubTab === 'viper' }"
                @click="dspSubTab = 'viper'"
              >
                酷狗蝰蛇音效库
              </button>
              <button
                class="pill-tab-btn"
                :class="{ active: dspSubTab === 'peq' }"
                @click="dspSubTab = 'peq'"
              >
                10 段参量均衡器 (PEQ)
              </button>
              <button
                class="pill-tab-btn"
                :class="{ active: dspSubTab === 'field' }"
                @click="dspSubTab = 'field'"
              >
                声场调音与混响
              </button>
            </div>

            <div class="dsp-right-btns">
              <button
                class="bg-pill-btn"
                :class="{ active: store.peqBypass }"
                @click="
                  store.peqBypass = !store.peqBypass;
                  store.syncDspChain();
                "
              >
                {{ store.peqBypass ? "原音直通中 (A/B)" : "A/B 原音对比" }}
              </button>
              <button class="bg-pill-btn" @click="store.resetAllPeqSlots()">
                一键重置全部
              </button>
            </div>
          </div>

          <!-- 子 Tab 1：酷狗蝰蛇官方音效矩阵 (对齐 AudioEffectDialog.kt) -->
          <template v-if="dspSubTab === 'viper'">
            <div class="bg-choice-pills mb-10">
              <button
                v-for="cat in viperCategories"
                :key="cat"
                class="bg-pill-btn"
                :class="{ active: selectedViperCategory === cat }"
                @click="selectedViperCategory = cat"
              >
                {{ cat }}
              </button>
              <button
                v-if="store.activeViperEffectId !== 'off'"
                class="bg-pill-btn"
                style="color: var(--coral)"
                @click="store.applyViperEffect(null)"
              >
                关闭蝰蛇音效
              </button>
            </div>

            <div class="stage-choice-grid" style="grid-template-columns: repeat(3, 1fr)">
              <button
                v-for="vf in filteredViperEffects"
                :key="vf.id"
                class="stage-choice-card"
                :class="{ active: store.activeViperEffectId === vf.id }"
                @click="store.applyViperEffect(vf)"
              >
                <div
                  style="
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                  "
                >
                  <span class="scc-title">{{ vf.label }}</span>
                  <span class="mini-badge sq">{{ vf.usageBadge }}</span>
                </div>
                <div class="scc-desc">{{ vf.desc }}</div>
                <div
                  style="
                    display: flex;
                    gap: 4px;
                    flex-wrap: wrap;
                    margin-top: 6px;
                  "
                >
                  <span
                    v-for="tag in vf.acousticTags"
                    :key="tag"
                    class="mini-badge hires"
                  >
                    {{ tag }}
                  </span>
                </div>
              </button>
            </div>
          </template>

          <!-- 子 Tab 2：10 段参量均衡器 (PEQ) -->
          <template v-else-if="dspSubTab === 'peq'">
            <div class="bg-choice-pills mb-10">
              <button
                v-for="p in eqPresetList"
                :key="p.id"
                class="bg-pill-btn"
                :class="{ active: store.activeEqPreset === p.id }"
                @click="store.setEqPreset(p.id)"
              >
                {{ p.name }}
              </button>
            </div>

            <div class="peq-slots-row mb-10">
              <div
                v-for="slot in store.peqMemorySlots"
                :key="slot.slotId"
                class="peq-slot-pill"
                :class="{
                  active: store.activeEqPreset === `slot_${slot.slotId}`,
                }"
              >
                <button
                  class="slot-load-btn"
                  @click="store.loadPeqFromSlot(slot.slotId)"
                >
                  <span>记忆槽 {{ slot.slotId }}</span>
                  <small>{{ slot.saved ? "已存" : "空置" }}</small>
                </button>
                <button
                  class="slot-mini-act"
                  title="将当前增益存入此槽"
                  @click="store.savePeqToSlot(slot.slotId)"
                >
                  存入
                </button>
                <button
                  v-if="slot.saved"
                  class="slot-mini-act danger"
                  title="清空此记忆槽"
                  @click="store.resetSinglePeqSlot(slot.slotId)"
                >
                  删
                </button>
              </div>
            </div>

            <div class="eq-bands-grid">
              <div
                v-for="(freq, idx) in store.eqFrequencies"
                :key="freq"
                class="eq-band-col"
              >
                <span class="eq-gain-val">
                  {{
                    store.eqGains[idx] > 0
                      ? `+${store.eqGains[idx]}`
                      : store.eqGains[idx]
                  }}dB
                </span>
                <input
                  type="range"
                  min="-10"
                  max="10"
                  step="0.5"
                  :value="store.eqGains[idx]"
                  class="eq-vertical-slider"
                  @input="
                    store.updateSingleEqBand(
                      idx,
                      Number(($event.target as HTMLInputElement).value)
                    )
                  "
                />
                <span class="eq-freq-label">
                  {{ freq >= 1000 ? `${freq / 1000}k` : freq }}
                </span>
              </div>
            </div>

            <!-- 品质因数 Q 值与 HPF / LPF 边界滤波控制 (对齐 ProfessionalEqualizerPage) -->
            <div class="lyric-sliders-row mt-14">
              <div class="cfg-col">
                <div class="cfg-label">
                  品质因数 Q 值：{{ store.eqQFactor.toFixed(2) }}
                </div>
                <input
                  v-model.number="store.eqQFactor"
                  type="range"
                  min="0.4"
                  max="6.0"
                  step="0.1"
                  @input="store.syncDspChain()"
                />
              </div>
              <div class="cfg-col">
                <div class="cfg-label">
                  高通边界滤波 (HPF)：{{ store.eqHighPassHz }} Hz
                </div>
                <input
                  v-model.number="store.eqHighPassHz"
                  type="range"
                  min="15"
                  max="180"
                  step="5"
                  @input="store.syncDspChain()"
                />
              </div>
              <div class="cfg-col">
                <div class="cfg-label">
                  低通边界滤波 (LPF)：{{ store.eqLowPassHz }} Hz
                </div>
                <input
                  v-model.number="store.eqLowPassHz"
                  type="range"
                  min="8000"
                  max="20000"
                  step="500"
                  @input="store.syncDspChain()"
                />
              </div>
            </div>
          </template>

          <!-- 子 Tab 3：声场调音工作台 (AcousticFieldTuningTab) -->
          <template v-else>
            <div class="acoustic-field-grid">
              <div class="af-toggle-head">
                <span>启用 64-bit 母带声场调音矩阵（智能防重叠增益协同）</span>
                <label class="plump-switch">
                  <input
                    v-model="store.acousticField.enabled"
                    type="checkbox"
                    @change="store.syncDspChain()"
                  />
                  <span class="plump-slider"></span>
                </label>
              </div>

              <div class="cfg-label">低频分频谐振中心频点 & 空间声学混响模型</div>
              <div class="bg-choice-pills mb-10">
                <button
                  v-for="cf in crossoverFreqs"
                  :key="cf.hz"
                  class="bg-pill-btn"
                  :class="{
                    active: store.acousticField.bassCrossoverHz === cf.hz,
                  }"
                  @click="
                    store.acousticField.bassCrossoverHz = cf.hz;
                    store.syncDspChain();
                  "
                >
                  {{ cf.label }}
                </button>
              </div>

              <div class="bg-choice-pills mb-10">
                <button
                  v-for="rv in reverbPresets"
                  :key="rv.id"
                  class="bg-pill-btn"
                  :class="{
                    active: store.acousticField.reverbPreset === rv.id,
                  }"
                  @click="store.acousticField.reverbPreset = rv.id"
                >
                  {{ rv.label }}
                </button>
              </div>

              <div class="lyric-sliders-row">
                <div class="cfg-col">
                  <div class="cfg-label">
                    超重低音下潜：{{ store.acousticField.subBassDepth }}%
                  </div>
                  <input
                    v-model.number="store.acousticField.subBassDepth"
                    type="range"
                    min="0"
                    max="100"
                    @input="store.syncDspChain()"
                  />
                </div>
                <div class="cfg-col">
                  <div class="cfg-label">
                    人声通透与 6.2kHz 齿音抑制：{{
                      store.acousticField.vocalClarity
                    }}%
                  </div>
                  <input
                    v-model.number="store.acousticField.vocalClarity"
                    type="range"
                    min="0"
                    max="100"
                    @input="store.syncDspChain()"
                  />
                </div>
                <div class="cfg-col">
                  <div class="cfg-label">
                    12kHz+ 空气泛音 / 胆机温暖度：{{
                      store.acousticField.airHarmonics
                    }}%
                  </div>
                  <input
                    v-model.number="store.acousticField.airHarmonics"
                    type="range"
                    min="0"
                    max="100"
                    @input="store.syncDspChain()"
                  />
                </div>
              </div>
            </div>
          </template>
        </div>

        <!-- 面板 D：歌曲评论区（最热/最新切换 + 多级楼中楼展开 + 点赞 + 发表评论） -->
        <div
          v-else-if="activeOverlayPanel === 'comments'"
          class="drawer-body comments-drawer-body"
        >
          <div
            style="
              display: flex;
              align-items: center;
              justify-content: space-between;
              gap: 12px;
              margin-bottom: 10px;
              flex-wrap: wrap;
            "
          >
            <div class="pill-tabs">
              <button
                class="pill-tab-btn"
                :class="{ active: store.commentSortType === 'hot' }"
                @click="store.loadCommentsForSong(store.currentSong, 'hot')"
              >
                精彩热评
              </button>
              <button
                class="pill-tab-btn"
                :class="{ active: store.commentSortType === 'new' }"
                @click="store.loadCommentsForSong(store.currentSong, 'new')"
              >
                最新评论
              </button>
            </div>

            <!-- 发表评论或回复楼层输入框 -->
            <div style="display: flex; align-items: center; gap: 8px; flex: 1; max-width: 480px">
              <span
                v-if="replyTargetComment"
                class="mini-badge sq"
                style="cursor: pointer"
                title="点击取消回复楼层"
                @click="replyTargetComment = null"
              >
                @{{ replyTargetComment.userName }} ×
              </span>
              <input
                v-model="newCommentText"
                type="text"
                class="plump-text-input"
                style="padding: 7px 14px; font-size: 12.5px"
                :placeholder="
                  replyTargetComment
                    ? `回复 @${replyTargetComment.userName}...`
                    : '写下你的听歌感受，与概念版乐迷共鸣...'
                "
                @keydown.enter="handleSendComment"
              />
              <button
                class="tonal-pill-btn sm primary"
                :disabled="isSubmittingComment"
                @click="handleSendComment"
              >
                发送
              </button>
            </div>
          </div>

          <div
            v-if="store.activeSongComments.length === 0"
            class="empty-box compact"
          >
            {{
              store.isLoadingComments
                ? "正在同步歌曲评论..."
                : "暂无评论，静享纯粹音乐时刻"
            }}
          </div>

          <div v-else class="comments-scroll-list">
            <div
              v-for="cmt in store.activeSongComments"
              :key="cmt.id"
              class="comment-card-item"
              style="flex-direction: column; gap: 8px"
            >
              <div style="display: flex; gap: 12px; align-items: flex-start">
                <img
                  v-if="cmt.userAvatar"
                  :src="cmt.userAvatar"
                  class="cmt-avatar"
                />
                <div v-else class="cmt-avatar fallback">
                  {{ cmt.userName.slice(0, 1) }}
                </div>
                <div class="cmt-main">
                  <div class="cmt-top">
                    <span class="cmt-user">{{ cmt.userName }}</span>
                    <span class="cmt-time">
                      {{ cmt.createdTime }} {{ cmt.location }}
                    </span>
                  </div>
                  <div class="cmt-text">{{ cmt.content }}</div>

                  <div
                    style="
                      display: flex;
                      align-items: center;
                      gap: 12px;
                      margin-top: 6px;
                      font-size: 11.5px;
                    "
                  >
                    <button
                      v-if="cmt.replyCount > 0"
                      style="color: var(--cyan); font-weight: 700"
                      @click="store.toggleCommentFloorReplies(cmt)"
                    >
                      {{
                        store.expandedFloorCommentId === cmt.id
                          ? `收起 ${cmt.replyCount} 条回复`
                          : `查看 ${cmt.replyCount} 条楼中楼回复 >`
                      }}
                    </button>
                    <button
                      style="color: var(--text-muted); font-weight: 700"
                      @click="
                        replyTargetComment = {
                          id: cmt.id,
                          userName: cmt.userName,
                        }
                      "
                    >
                      回复
                    </button>
                  </div>
                </div>
                <button
                  class="cmt-likes"
                  style="cursor: pointer"
                  @click="store.toggleLikeComment(cmt)"
                >
                  <PlumpIcon
                    name="heart"
                    :size="13"
                    :filled="cmt.isLiked"
                    :color="cmt.isLiked ? '#ff4d6d' : 'currentColor'"
                  />
                  <span>{{ cmt.likeCount }}</span>
                </button>
              </div>

              <!-- 二层楼中楼回复折叠区 -->
              <div
                v-if="store.expandedFloorCommentId === cmt.id"
                style="
                  margin-left: 48px;
                  padding: 10px 14px;
                  border-radius: 14px;
                  background: var(--bg-surface);
                  display: flex;
                  flex-direction: column;
                  gap: 8px;
                "
              >
                <div
                  v-if="
                    store.isLoadingFloorReplies &&
                    !store.floorRepliesMap[cmt.id]?.length
                  "
                  style="font-size: 12px; color: var(--text-secondary)"
                >
                  正在拉取楼层回复...
                </div>
                <div
                  v-for="rep in store.floorRepliesMap[cmt.id] || []"
                  :key="rep.id"
                  style="font-size: 12.5px; line-height: 1.5"
                >
                  <strong style="color: var(--cyan)">{{ rep.userName }}：</strong>
                  <span>{{ rep.content }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 面板 E：当前播放队列与最近队列层叠卡片抽屉 (对齐 PlayQueueBottomSheet.kt) -->
        <div
          v-else-if="activeOverlayPanel === 'queue'"
          class="drawer-body comments-drawer-body"
        >
          <div class="pill-tabs mb-10">
            <button
              class="pill-tab-btn"
              :class="{ active: queueSubTab === 'current' }"
              @click="queueSubTab = 'current'"
            >
              当前队列 ({{ store.playQueue.length }})
            </button>
            <button
              class="pill-tab-btn"
              :class="{ active: queueSubTab === 'recent' }"
              @click="queueSubTab = 'recent'"
            >
              最近播放过的歌单栈 ({{ store.recentPlayQueues.length }})
            </button>
          </div>

          <div v-if="queueSubTab === 'current'" class="comments-scroll-list">
            <div
              v-for="(s, idx) in store.playQueue"
              :key="s.hash || idx"
              class="plump-song-card compact"
              :class="{
                'track-card-playing': store.currentSong?.hash === s.hash,
              }"
              @click="store.playSongFromList(s, store.playQueue)"
            >
              <div class="psc-index">
                {{ String(idx + 1).padStart(2, "0") }}
              </div>
              <div class="psc-main">
                <div class="psc-title">{{ s.title }}</div>
                <div class="psc-sub">
                  <span>{{ s.artist }}</span>
                </div>
              </div>
              <button
                class="psc-act-btn"
                title="从播放队列中移除"
                @click.stop="store.playQueue.splice(idx, 1)"
              >
                <PlumpIcon name="close" :size="13" />
              </button>
            </div>
          </div>

          <div v-else class="comments-scroll-list">
            <div
              v-for="(rq, rIdx) in store.recentPlayQueues"
              :key="rIdx"
              class="plump-song-card compact"
              @click="
                rq.songs.length > 0 &&
                  store.playSongFromList(rq.songs[0], rq.songs, rq.title);
                queueSubTab = 'current';
              "
            >
              <div class="psc-main">
                <div class="psc-title">{{ rq.title }}</div>
                <div class="psc-sub">
                  <span>共 {{ rq.songs.length }} 首曲目 · 点击一键恢复此队列</span>
                </div>
              </div>
              <PlumpIcon name="play" :size="15" color="var(--cyan)" />
            </div>
          </div>
        </div>

        <!-- 面板 F：更多声学与播放控制（倍速、半音升降调、KTV伴奏、睡眠定时、高清封面、下载） -->
        <div v-else-if="activeOverlayPanel === 'more'" class="drawer-body">
          <div class="lyric-sliders-row">
            <div class="cfg-col">
              <div class="cfg-label">
                播放倍速 ({{ store.playbackRate }}x) &amp; 升降调 ({{
                  store.pitchSemitones > 0
                    ? `+${store.pitchSemitones}`
                    : store.pitchSemitones
                }}
                半音)
              </div>
              <div class="bg-choice-pills mb-10">
                <button
                  v-for="spd in [0.75, 1.0, 1.25, 1.5, 2.0]"
                  :key="spd"
                  class="bg-pill-btn"
                  :class="{ active: store.playbackRate === spd }"
                  @click="store.setPlaybackSpeed(spd)"
                >
                  {{ spd }}x
                </button>
              </div>
              <div class="bg-choice-pills">
                <button
                  v-for="semi in [-4, -2, 0, 2, 4]"
                  :key="semi"
                  class="bg-pill-btn"
                  :class="{ active: store.pitchSemitones === semi }"
                  @click="store.setPitchSemitones(semi)"
                >
                  {{ semi === 0 ? "原调" : semi > 0 ? `+${semi}音` : `${semi}音` }}
                </button>
              </div>
            </div>

            <div class="cfg-col">
              <div class="cfg-label">睡眠定时关闭</div>
              <div class="bg-choice-pills">
                <button
                  v-for="m in [0, 15, 30, 60, -1]"
                  :key="m"
                  class="bg-pill-btn"
                  :class="{ active: store.sleepTimerMinutes === m }"
                  @click="store.startSleepTimer(m)"
                >
                  {{
                    m === 0 ? "关闭" : m === -1 ? "播完本首" : `${m}分钟`
                  }}
                </button>
              </div>
            </div>

            <div class="cfg-col">
              <div class="cfg-label">特色声学工具</div>
              <div class="bg-choice-pills">
                <button
                  class="bg-pill-btn"
                  :class="{ active: store.ktvAccompanimentMode }"
                  @click="store.toggleKtvAccompaniment()"
                >
                  KTV 伴奏切换
                </button>
                <button
                  class="bg-pill-btn"
                  :class="{ active: store.climaxModeEnabled }"
                  @click="store.climaxModeEnabled = !store.climaxModeEnabled"
                >
                  副歌速刷 30s
                </button>
                <button
                  class="bg-pill-btn"
                  @click="showHdCoverPreview = true"
                >
                  1000P 高清封面
                </button>
                <button
                  v-if="store.downloadFeatureUnlocked && store.currentSong"
                  class="bg-pill-btn active"
                  @click="store.downloadModalSong = store.currentSong"
                >
                  多音质下载本曲
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </transition>

    <!-- 1000P 高清专辑封面预览与保存弹窗 -->
    <transition name="fade">
      <div
        v-if="showHdCoverPreview && store.currentSong?.coverUrl"
        class="modal-backdrop"
        @click.self="showHdCoverPreview = false"
      >
        <div class="hd-cover-modal" @click.stop>
          <img :src="hdCoverUrl" class="hd-cover-large" />
          <div class="hd-cover-actions">
            <a
              :href="hdCoverUrl"
              target="_blank"
              download
              class="primary-btn"
            >
              <PlumpIcon name="download" :size="16" color="#ffffff" />
              <span>打开 / 保存 1000×1000 原图</span>
            </a>
            <button
              class="tonal-pill-btn"
              @click="showHdCoverPreview = false"
            >
              关闭预览
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>
