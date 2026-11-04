<template>
  <footer
    class="fixed-bottom-player-bar"
    :class="{ 'player-bar-hidden': store.isNowPlayingOpen }"
  >
    <!-- 左侧：专辑封面 + 歌名/歌手 + 喜欢按钮 + 高对比音质角标 -->
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
          <span
            class="pb-song-artist"
            style="cursor: pointer"
            title="点击查看歌手详情（多歌手支持选择）"
            @click="store.currentSong && store.navigateToSongArtist(store.currentSong)"
          >
            {{ store.currentSong?.artist || "双击顶部「发现」开启概念版畅听之旅" }}
          </span>
          <span
            v-if="store.currentSong?.album"
            class="pb-song-artist"
            style="cursor: pointer; opacity: 0.78"
            title="点击跳转到专辑详情页"
            @click.stop="store.navigateToSongAlbum(store.currentSong)"
          >
            · {{ store.currentSong.album }}
          </span>
          <span
            v-if="store.currentSong"
            class="pb-quality-pill"
            title="点击切换流媒体音质档位"
            @click.stop="cycleQualityQuick"
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

    <!-- 中间：上层【当前时间 + 进度条 + 总时长】，下层【模式 + 上一首 + 大圆形播放暂停 + 下一首 + 高潮】 -->
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
          <PlumpIcon :name="store.isPlaying ? 'pause' : 'play'" :size="20" color="#ffffff" />
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

    <!-- 右侧：音质/调音台胶囊 + 自定义无原生丑滑块的音量条 + 桌面歌词 + 播放队列 -->
    <div class="pb-right-zone">
      <button
        class="pb-feature-pill"
        :class="{ active: store.activeViperEffectId !== 'off' && store.activeViperEffectId !== 'none' }"
        title="前往概念版调音台"
        @click="openTunerDesk"
      >
        <PlumpIcon name="tuning" :size="15" />
        <span>{{ store.activeViperEffectId !== "off" && store.activeViperEffectId !== "none" ? "蝰蛇母带" : qualityBadgeText }}</span>
      </button>

      <div class="pb-volume-control">
        <button
          class="pb-tool-icon-btn"
          :title="`音量 ${Math.round(store.volume * 100)}%`"
          @click="toggleMute"
        >
          <PlumpIcon :name="store.volume === 0 ? 'volume-mute' : 'volume'" :size="18" />
        </button>
        <div
          class="pb-custom-volume-track"
          :title="`音量 ${Math.round(store.volume * 100)}%`"
          @mousedown="handleVolumeMouseDown"
        >
          <div class="pb-custom-volume-bg"></div>
          <div
            class="pb-custom-volume-fill"
            :style="{ width: `${Math.round(store.volume * 100)}%` }"
          >
            <span class="pb-custom-volume-thumb"></span>
          </div>
        </div>
      </div>

      <button
        class="pb-tool-icon-btn"
        title="查看当前歌曲评论弹窗"
        @click="store.openCommentsModal(store.currentSong)"
      >
        <PlumpIcon name="comment" :size="18" />
      </button>

      <button
        class="pb-tool-icon-btn"
        :class="{ active: store.desktopLyricEnabled }"
        title="桌面悬浮逐字歌词"
        @click="toggleDesktopLyric"
      >
        <PlumpIcon name="lyric" :size="18" />
      </button>

      <button
        ref="queueToggleBtnRef"
        class="pb-tool-icon-btn pb-queue-badge-btn"
        :class="{ active: store.isGlobalQueueOpen }"
        title="当前播放队列与最近歌单栈"
        @click.stop="store.isGlobalQueueOpen = !store.isGlobalQueueOpen"
      >
        <PlumpIcon name="queue" :size="18" />
        <span v-if="store.playQueue.length > 0" class="pb-queue-count-badge">
          {{ store.playQueue.length }}
        </span>
      </button>
    </div>

    <!-- 底部固定栏右上角弹出的播放队列抽屉（对齐 Android PlayQueueBottomSheet：当前队列 / 最近队列栈 + 情绪重排 + 拖拽置顶） -->
    <transition name="fade-pop">
      <div
        v-if="store.isGlobalQueueOpen && !store.isNowPlayingOpen"
        ref="queueDrawerRef"
        class="pb-global-queue-drawer glass-popover"
        @click.stop
      >
        <div class="pb-queue-header">
          <div class="pill-tabs" style="padding: 3px">
            <button
              class="pill-tab-btn"
              :class="{ active: queueTab === 'current' }"
              style="padding: 5px 12px !important; font-size: 13.5px !important"
              @click="queueTab = 'current'"
            >
              当前队列 ({{ store.playQueue.length }})
            </button>
            <button
              class="pill-tab-btn"
              :class="{ active: queueTab === 'recent' }"
              style="padding: 5px 12px !important; font-size: 13.5px !important"
              @click="queueTab = 'recent'"
            >
              最近队列栈 ({{ store.recentPlayQueues.length }})
            </button>
          </div>
          <div style="display: flex; align-items: center; gap: 6px">
            <button
              v-if="queueTab === 'current' && store.playQueue.length > 0"
              class="pb-tool-icon-btn"
              title="清空播放队列"
              @click="store.clearPlayQueue()"
            >
              <PlumpIcon name="trash" :size="15" />
            </button>
            <button class="pb-tool-icon-btn" @click="store.isGlobalQueueOpen = false">
              <PlumpIcon name="close" :size="16" />
            </button>
          </div>
        </div>

        <!-- AI 情绪智能重排与 30s 副歌速刷工具条 -->
        <div v-if="queueTab === 'current' && store.playQueue.length > 1" class="pb-queue-mood-bar">
          <button
            class="pb-mood-pill"
            title="按节奏感与高能曲目优先重排队列"
            @click="store.reorderQueueByMood('energetic')"
          >
            ⚡ 律动高能
          </button>
          <button
            class="pb-mood-pill"
            title="按舒缓松弛曲目优先重排队列"
            @click="store.reorderQueueByMood('chill')"
          >
            🌙 舒缓松弛
          </button>
          <button
            class="pb-mood-pill"
            title="按抒情人声叙事重排队列"
            @click="store.reorderQueueByMood('vocal')"
          >
            🎙️ 人声叙事
          </button>
          <button
            class="pb-mood-pill"
            :class="{ active: store.climaxModeEnabled }"
            title="30秒副歌高潮速刷"
            @click="toggleClimaxQuick"
          >
            ✨ 副歌30s
          </button>
        </div>

        <!-- 当前播放队列列表（支持拖拽排序、置顶下一首与单曲移除） -->
        <div v-if="queueTab === 'current'" class="pb-queue-list">
          <div
            v-if="store.playQueue.length === 0"
            class="lib-right-empty"
            style="padding: 36px 16px"
          >
            队列为空，去发现页挑选喜欢的音乐吧
          </div>
          <div
            v-for="(s, idx) in store.playQueue"
            :key="s.hash + '_' + idx"
            class="pb-queue-item"
            :class="{
              active: store.currentSong?.hash === s.hash,
              'drag-over': dragOverIndex === idx,
            }"
            draggable="true"
            @dragstart="onDragStart(idx)"
            @dragover.prevent="onDragOver(idx)"
            @dragleave="onDragLeave"
            @drop="onDrop(idx)"
            @click="store.playSongFromList(s, store.playQueue)"
            @contextmenu.prevent="store.openSongContextMenu($event, s, store.playQueue)"
          >
            <span class="pb-queue-idx">{{ idx + 1 }}</span>
            <img :src="s.coverUrl" class="pb-queue-cover" alt="" />
            <div class="pb-queue-meta">
              <div class="pb-queue-song-name">{{ s.title }}</div>
              <div class="pb-queue-song-artist">{{ s.artist }}</div>
            </div>
            <span class="pb-queue-dur">{{ formatDuration(s) }}</span>
            <div class="pb-queue-row-actions" @click.stop>
              <button
                v-if="store.currentSong?.hash !== s.hash"
                class="pb-q-act-btn"
                title="移至下一首播放"
                @click="store.insertPlayNext(s)"
              >
                <PlumpIcon name="top" :size="13" />
              </button>
              <button
                class="pb-q-act-btn danger"
                title="从队列中移除"
                @click="store.removeSongFromQueue(idx)"
              >
                <PlumpIcon name="close" :size="13" />
              </button>
            </div>
          </div>
        </div>

        <!-- 最近播放过的 5 个历史歌单栈 -->
        <div v-else class="pb-queue-list">
          <div
            v-if="store.recentPlayQueues.length === 0"
            class="lib-right-empty"
            style="padding: 36px 16px"
          >
            暂无历史歌单栈记录
          </div>
          <div
            v-for="(rq, rIdx) in store.recentPlayQueues"
            :key="rq.id || rIdx"
            class="pb-recent-queue-card"
            @click="
              store.switchToRecentQueue(rq.id);
              queueTab = 'current';
            "
          >
            <div class="prq-covers">
              <img
                v-for="(sc, cIdx) in rq.songs.slice(0, 3)"
                :key="cIdx"
                :src="sc.coverUrl"
                class="prq-cover-img"
                :class="`layer-${cIdx}`"
              />
            </div>
            <div class="pb-queue-meta">
              <div class="pb-queue-song-name">{{ rq.title }}</div>
              <div class="pb-queue-song-artist">
                共 {{ rq.songs.length }} 首 · 点击一键恢复此队列
              </div>
            </div>
            <PlumpIcon name="play" :size="16" color="var(--cyan)" />
          </div>
        </div>
      </div>
    </transition>
  </footer>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();
const lastVolume = ref(0.8);
const queueToggleBtnRef = ref<HTMLElement | null>(null);
const queueDrawerRef = ref<HTMLElement | null>(null);
const queueTab = ref<"current" | "recent">("current");
const dragSourceIndex = ref<number | null>(null);
const dragOverIndex = ref<number | null>(null);

function onDragStart(idx: number) {
  dragSourceIndex.value = idx;
}

function onDragOver(idx: number) {
  dragOverIndex.value = idx;
}

function onDragLeave() {
  dragOverIndex.value = null;
}

function onDrop(targetIdx: number) {
  if (dragSourceIndex.value !== null && dragSourceIndex.value !== targetIdx) {
    store.moveQueueItem(dragSourceIndex.value, targetIdx);
  }
  dragSourceIndex.value = null;
  dragOverIndex.value = null;
}

function handleOutsideQueueMouseDown(e: MouseEvent) {
  if (!store.isGlobalQueueOpen) return;
  const target = e.target as Node | null;
  if (!target) return;
  if (queueDrawerRef.value?.contains(target)) return;
  if (queueToggleBtnRef.value?.contains(target)) return;
  store.isGlobalQueueOpen = false;
}

onMounted(() => {
  window.addEventListener("mousedown", handleOutsideQueueMouseDown, true);
});

onBeforeUnmount(() => {
  window.removeEventListener("mousedown", handleOutsideQueueMouseDown, true);
});

const progressPercent = computed(() => {
  if (!store.durationMs || store.durationMs <= 0) return 0;
  return Math.min(100, Math.max(0, (store.currentPositionMs / store.durationMs) * 100));
});

const climaxSegmentStyle = computed(() => {
  const song = store.currentSong;
  const cStart = song?.climaxStartMs || store.currentPlaybackInfo?.climaxStartMs || 0;
  const cEnd = song?.climaxEndMs || store.currentPlaybackInfo?.climaxEndMs || 0;
  if (!song || !store.durationMs || cEnd <= cStart) return null;
  const startPct = Math.max(0, Math.min(100, (cStart / store.durationMs) * 100));
  const widthPct = Math.max(
    2,
    Math.min(100 - startPct, ((cEnd - cStart) / store.durationMs) * 100)
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
    case "high":
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
    case "repeat_one":
      return "repeat-one";
    case "sequence":
      return "sequence";
    default:
      return "repeat";
  }
});

const playModeTitle = computed(() => {
  switch (store.playMode) {
    case "sequence":
      return "顺序播放";
    case "loop_all":
    case "repeat_all":
      return "列表循环";
    case "loop_one":
    case "repeat_one":
      return "单曲循环";
    case "shuffle":
      return "随机漫游";
    default:
      return "列表循环";
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

function handleVolumeMouseDown(e: MouseEvent) {
  const bar = e.currentTarget as HTMLElement;
  const updateVol = (ev: MouseEvent) => {
    const rect = bar.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (ev.clientX - rect.left) / rect.width));
    store.setVolume(Number(ratio.toFixed(2)));
  };
  updateVol(e);
  const onMove = (ev: MouseEvent) => updateVol(ev);
  const onUp = () => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  };
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
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
  const idx = order.indexOf(store.preferredQuality as any);
  const next = order[(idx + 1) % order.length];
  store.switchAudioQuality(next);
}

function openTunerDesk() {
  store.openSoundMixerFromPlayer();
}
</script>
