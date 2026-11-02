<template>
  <footer
    class="fixed-bottom-player-bar"
    :class="{ 'player-bar-hidden': store.isNowPlayingOpen }"
  >
    <!-- 左侧：专辑封面 + 歌名/歌手 + 喜欢按钮 + 音质角标 -->
    <div class="pb-left-zone">
      <div
        class="pb-cover-frame"
        :class="{ spinning: store.isPlaying }"
        title="点击展开全屏写真与逐字歌词舞台"
        @click="store.isNowPlayingOpen = !store.isNowPlayingOpen"
      >
        <img
          v-if="store.currentSong?.coverUrl"
          :src="store.currentSong.coverUrl"
          class="pb-cover-img"
          alt="cover"
        />
        <div v-else class="pb-cover-placeholder">
          <PlumpIcon name="disc" :size="24" />
        </div>
        <div class="pb-cover-expand-mask">
          <PlumpIcon name="maximize" :size="16" />
        </div>
      </div>

      <div class="pb-song-meta">
        <div class="pb-title-line">
          <span
            class="pb-song-title"
            @click="store.isNowPlayingOpen = !store.isNowPlayingOpen"
          >
            {{ store.currentSong?.title || "微风音乐 · 概念声学" }}
          </span>
          <button
            v-if="store.currentSong"
            class="pb-inline-like-btn"
            :class="{ liked: store.isSongLiked(store.currentSong.hash) }"
            title="添加到我喜欢"
            @click.stop="store.toggleFavoriteSong(store.currentSong)"
          >
            <PlumpIcon
              :name="store.isSongLiked(store.currentSong.hash) ? 'heart-filled' : 'heart'"
              :size="17"
            />
          </button>
        </div>

        <div class="pb-artist-line">
          <span class="pb-song-artist">
            {{ store.currentSong?.artist || "双击顶部「发现」开启概念版畅听之旅" }}
          </span>
          <span
            v-if="store.currentSong"
            class="pb-quality-pill"
            @click.stop="cycleQualityQuick"
            title="点击切换流媒体音质档位"
          >
            {{ qualityBadgeText }}
          </span>
          <span
            v-if="store.currentSong?.isVip"
            class="pb-vip-pill"
          >
            VIP
          </span>
        </div>
      </div>
    </div>

    <!-- 中间：参考图5双层结构 —— 上层【当前时间 + 进度条 + 总时长】，下层【随机/模式 + 上一首 + 大圆形播放暂停 + 下一首 + 高潮/伴奏】 -->
    <div class="pb-center-zone">
      <!-- 上层：时间与可拖拽进度条 -->
      <div class="pb-progress-timeline-row">
        <span class="pb-time-label">{{ formatDuration(store.currentPositionMs / 1000) }}</span>
        <div class="pb-timeline-track" @mousedown="handleSeekMouseDown">
          <div class="pb-timeline-bg"></div>
          <!-- 副歌高潮区间标记 -->
          <div
            v-if="climaxSegmentStyle"
            class="pb-timeline-climax"
            :style="climaxSegmentStyle"
            title="概念版副歌高潮精华段落"
          ></div>
          <div
            class="pb-timeline-fill"
            :style="{ width: `${progressPercent}%` }"
          >
            <span class="pb-timeline-thumb"></span>
          </div>
        </div>
        <span class="pb-time-label">{{ formatDuration(store.durationMs / 1000) }}</span>
      </div>

      <!-- 下层：居中核心播放控制键 -->
      <div class="pb-transport-controls-row">
        <button
          class="pb-ctrl-btn"
          :class="{ active: store.playMode !== 'sequence' }"
          :title="playModeTitle"
          @click="store.cyclePlayMode()"
        >
          <PlumpIcon :name="playModeIcon" :size="18" />
        </button>

        <button class="pb-ctrl-btn" title="上一曲 (Ctrl + Left)" @click="store.playPrev()">
          <PlumpIcon name="prev" :size="20" />
        </button>

        <button
          class="pb-play-hero-circle"
          :title="store.isPlaying ? '暂停 (Space)' : '播放 (Space)'"
          @click="store.togglePlayPause()"
        >
          <PlumpIcon :name="store.isPlaying ? 'pause' : 'play'" :size="22" />
        </button>

        <button class="pb-ctrl-btn" title="下一曲 (Ctrl + Right)" @click="store.playNext()">
          <PlumpIcon name="next" :size="20" />
        </button>

        <button
          class="pb-ctrl-btn"
          :class="{ active: store.climaxModeEnabled }"
          title="30秒副歌高潮精听模式"
          @click="toggleClimaxQuick"
        >
          <PlumpIcon name="sparkle" :size="18" />
        </button>
      </div>
    </div>

    <!-- 右侧：音质/调音台胶囊 + 音量图标与滑块 + 桌面歌词 + 播放队列（带曲目数角标，参考图5右侧） -->
    <div class="pb-right-zone">
      <button
        class="pb-feature-pill"
        :class="{ active: store.activeViperEffectId !== 'none' || !store.peqBypass }"
        title="前往概念版调音台 / 切换无损音质"
        @click="openTunerDesk"
      >
        <PlumpIcon name="tuning" :size="15" />
        <span>{{ store.activeViperEffectId !== "none" ? "蝰蛇母带" : qualityBadgeText }}</span>
      </button>

      <div class="pb-volume-control">
        <button
          class="pb-tool-icon-btn"
          :title="`音量 ${Math.round(store.volume * 100)}%`"
          @click="toggleMute"
        >
          <PlumpIcon :name="store.volume === 0 ? 'volume-mute' : 'volume'" :size="18" />
        </button>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          :value="store.volume"
          class="pb-volume-slider"
          @input="onVolumeInput"
        />
      </div>

      <button
        class="pb-tool-icon-btn"
        :class="{ active: store.desktopLyricEnabled }"
        title="桌面悬浮逐字歌词"
        @click="toggleDesktopLyric"
      >
        <PlumpIcon name="lyric" :size="18" />
      </button>

      <button
        class="pb-tool-icon-btn pb-queue-badge-btn"
        :class="{ active: store.isGlobalQueueOpen }"
        title="当前播放队列"
        @click="store.isGlobalQueueOpen = !store.isGlobalQueueOpen"
      >
        <PlumpIcon name="queue" :size="18" />
        <span v-if="store.playQueue.length > 0" class="pb-queue-count-badge">
          {{ store.playQueue.length }}
        </span>
      </button>
    </div>

    <!-- 底部固定栏右上角弹出的播放队列抽屉 -->
    <transition name="fade-pop">
      <div
        v-if="store.isGlobalQueueOpen && !store.isNowPlayingOpen"
        class="pb-global-queue-drawer glass-popover"
        @click.stop
      >
        <div class="pb-queue-header">
          <div>
            <div class="pb-queue-title">当前播放队列 ({{ store.playQueue.length }})</div>
            <div class="pb-queue-sub">支持一键切换曲目或展开全屏舞台</div>
          </div>
          <button class="pb-tool-icon-btn" @click="store.isGlobalQueueOpen = false">
            <PlumpIcon name="close" :size="15" />
          </button>
        </div>
        <div class="pb-queue-list">
          <div
            v-for="(s, idx) in store.playQueue"
            :key="s.hash + '_' + idx"
            class="pb-queue-item"
            :class="{ active: store.currentSong?.hash === s.hash }"
            @click="store.playSongFromList(s, store.playQueue)"
          >
            <span class="pb-queue-idx">{{ idx + 1 }}</span>
            <img :src="s.coverUrl" class="pb-queue-cover" alt="" />
            <div class="pb-queue-meta">
              <div class="pb-queue-song-name">{{ s.title }}</div>
              <div class="pb-queue-song-artist">{{ s.artist }}</div>
            </div>
            <span class="pb-queue-dur">{{ formatDuration(s.durationSeconds) }}</span>
          </div>
        </div>
      </div>
    </transition>
  </footer>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();
const lastVolume = ref(0.8);

const progressPercent = computed(() => {
  if (!store.durationMs || store.durationMs <= 0) return 0;
  return Math.min(100, Math.max(0, (store.currentPositionMs / store.durationMs) * 100));
});

const climaxSegmentStyle = computed(() => {
  const song = store.currentSong;
  if (!song || !store.durationMs || song.climaxEndMs <= song.climaxStartMs) return null;
  const startPct = Math.max(0, Math.min(100, (song.climaxStartMs / store.durationMs) * 100));
  const widthPct = Math.max(
    2,
    Math.min(100 - startPct, ((song.climaxEndMs - song.climaxStartMs) / store.durationMs) * 100)
  );
  return {
    left: `${startPct}%`,
    width: `${widthPct}%`,
  };
});

const qualityBadgeText = computed(() => {
  const q = store.currentPlaybackInfo?.matchedQuality || store.preferredQuality;
  switch (q) {
    case "master":
      return "母带 Master";
    case "hires":
      return "Hi-Res";
    case "flac":
      return "FLAC 无损";
    case "320":
      return "320k HQ";
    default:
      return "标准 128k";
  }
});

const playModeIcon = computed(() => {
  switch (store.playMode) {
    case "shuffle":
      return "shuffle";
    case "loop_one":
      return "repeat-one";
    default:
      return "repeat";
  }
});

const playModeTitle = computed(() => {
  switch (store.playMode) {
    case "sequence":
      return "顺序播放";
    case "loop_all":
      return "列表循环";
    case "loop_one":
      return "单曲循环";
    case "shuffle":
      return "随机漫游";
  }
});

function handleSeekMouseDown(e: MouseEvent) {
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

function onVolumeInput(e: Event) {
  const val = parseFloat((e.target as HTMLInputElement).value);
  store.setVolume(val);
}

function toggleMute() {
  if (store.volume > 0) {
    lastVolume.value = store.volume;
    store.setVolume(0);
  } else {
    store.setVolume(lastVolume.value || 0.8);
  }
}

function toggleClimaxQuick() {
  store.climaxModeEnabled = !store.climaxModeEnabled;
  store.notify(
    store.climaxModeEnabled
      ? "已开启 30s 副歌高潮精听模式"
      : "已恢复完整曲目播放模式"
  );
}

function toggleDesktopLyric() {
  store.desktopLyricEnabled = !store.desktopLyricEnabled;
  store.notify(store.desktopLyricEnabled ? "已开启桌面悬浮逐字歌词" : "已关闭桌面悬浮歌词");
}

function cycleQualityQuick() {
  const order: Array<"128" | "320" | "flac" | "hires" | "master"> = [
    "320",
    "flac",
    "hires",
    "master",
    "128",
  ];
  const idx = order.indexOf(store.preferredQuality);
  const next = order[(idx + 1) % order.length];
  store.switchAudioQuality(next);
}

function openTunerDesk() {
  store.isNowPlayingOpen = false;
  store.activeTab = "radio";
}
</script>
