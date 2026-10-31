<script setup lang="ts">
import { computed, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import type { AudioQualityValue } from "../types";
import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Volume2,
  Repeat,
  Repeat1,
  Shuffle,
  ChevronUp,
  ChevronDown,
  Loader2,
  ListMusic,
  X,
} from "lucide-vue-next";

const store = useAppStore();
const isQueueOpen = ref(false);

function formatTime(ms: number): string {
  if (!ms || ms < 0) return "00:00";
  const totalSec = Math.floor(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${String(min).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}

const progressPercent = computed(() => {
  if (!store.durationMs || store.durationMs <= 0) return 0;
  return Math.min(100, (store.currentPositionMs / store.durationMs) * 100);
});

function onSeekInput(e: Event) {
  const target = e.target as HTMLInputElement;
  const pct = Number(target.value);
  if (store.durationMs > 0) {
    store.seekToMs((pct / 100) * store.durationMs);
  }
}

function onVolumeInput(e: Event) {
  const target = e.target as HTMLInputElement;
  store.setVolume(Number(target.value));
}

function onQualityChange(e: Event) {
  const target = e.target as HTMLSelectElement;
  store.switchQuality(target.value as AudioQualityValue);
}
</script>

<template>
  <footer class="floating-player-dock">
    <!-- 顶部光学彗星流光进度轨 -->
    <div class="dock-progress-track">
      <input
        type="range"
        min="0"
        max="100"
        step="0.1"
        :value="progressPercent"
        class="dock-progress-input"
        @input="onSeekInput"
      />
      <div
        class="dock-progress-fill"
        :style="{ width: `${progressPercent}%` }"
      >
        <span class="comet-head"></span>
      </div>
    </div>

    <div class="dock-main-row">
      <!-- 左侧：旋转黑胶/流光封面 + 曲名歌手 + 真实采样率/位深/码率胶囊 -->
      <div
        class="dock-track-zone"
        title="点击展开/收起沉浸歌词舞台"
        @click="store.isNowPlayingOpen = !store.isNowPlayingOpen"
      >
        <div class="dock-cover-wrapper" :class="{ spinning: store.isPlaying }">
          <img
            v-if="store.currentSong?.coverUrl"
            :src="store.currentSong.coverUrl"
            alt="cover"
            class="dock-cover-img"
          />
          <div v-else class="dock-cover-placeholder">♪</div>
          <div class="dock-cover-hover">
            <ChevronDown v-if="store.isNowPlayingOpen" :size="18" />
            <ChevronUp v-else :size="18" />
          </div>
        </div>

        <div class="dock-track-texts">
          <div class="dock-song-title">
            {{ store.currentSong?.title || "耳畔微风起 · 帧享真无损" }}
          </div>
          <div class="dock-song-sub">
            <span class="dock-artist-name">
              {{ store.currentSong?.artist || "点击任意曲目即刻开启高解析播放" }}
            </span>
            <span
              v-if="store.currentPlaybackInfo"
              class="dock-spec-pill"
              :class="{
                trial: store.currentPlaybackInfo.isFreePart,
                hires: store.currentPlaybackInfo.actualQuality === 'high',
                sq: store.currentPlaybackInfo.actualQuality === 'flac',
              }"
            >
              {{
                store.currentPlaybackInfo.isFreePart
                  ? "试听片段"
                  : store.currentPlaybackInfo.techSpecText ||
                    `${store.currentPlaybackInfo.qualityBadge} · ${store.currentPlaybackInfo.bitrateKbps} kbps`
              }}
            </span>
          </div>
        </div>
      </div>

      <!-- 中部：主控播按钮坞 -->
      <div class="dock-controls-zone">
        <div class="dock-btn-group">
          <button
            class="dock-icon-btn"
            title="切换播放模式"
            @click="store.togglePlayMode()"
          >
            <Repeat1 v-if="store.playMode === 'repeat_one'" :size="17" />
            <Shuffle v-else-if="store.playMode === 'shuffle'" :size="17" />
            <Repeat v-else :size="17" />
          </button>

          <button class="dock-icon-btn" title="上一首" @click="store.playPrev()">
            <SkipBack :size="19" />
          </button>

          <button
            class="dock-play-fab"
            :title="store.isPlaying ? '暂停' : '播放'"
            @click="store.togglePlayPause()"
          >
            <Loader2 v-if="store.isBuffering" :size="22" class="spin-icon" />
            <Pause v-else-if="store.isPlaying" :size="22" />
            <Play v-else :size="22" />
          </button>

          <button class="dock-icon-btn" title="下一首" @click="store.playNext()">
            <SkipForward :size="19" />
          </button>

          <button
            class="dock-icon-btn"
            :class="{ active: isQueueOpen }"
            title="当前播放队列"
            @click="isQueueOpen = !isQueueOpen"
          >
            <ListMusic :size="18" />
          </button>
        </div>

        <div class="dock-time-row">
          <span>{{ formatTime(store.currentPositionMs) }}</span>
          <span class="time-divider">/</span>
          <span>{{ formatTime(store.durationMs) }}</span>
        </div>
      </div>

      <!-- 右侧：实时音质切换胶囊 + 音量调节 -->
      <div class="dock-right-zone">
        <select
          :value="store.preferredQuality"
          class="dock-quality-pill"
          title="切换音源解析优先音质"
          @change="onQualityChange"
        >
          <option value="128">标准 128K</option>
          <option value="320">高品 320K</option>
          <option value="flac">无损 SQ</option>
          <option value="high">Hi-Res 帧享</option>
        </select>

        <div class="dock-volume-wrap">
          <Volume2 :size="16" class="vol-ico" />
          <input
            type="range"
            min="0"
            max="1"
            step="0.02"
            :value="store.volume"
            class="dock-vol-slider"
            @input="onVolumeInput"
          />
        </div>
      </div>
    </div>

    <!-- 悬浮玻璃拟态播放队列抽屉 -->
    <transition name="pop-up">
      <div v-if="isQueueOpen" class="floating-queue-popover">
        <div class="queue-pop-header">
          <div>
            <h4>当前播放队列</h4>
            <span>共 {{ store.playQueue.length }} 首歌曲</span>
          </div>
          <button class="icon-btn" @click="isQueueOpen = false">
            <X :size="16" />
          </button>
        </div>
        <div class="queue-pop-list">
          <div
            v-for="(item, idx) in store.playQueue"
            :key="item.hash || idx"
            class="queue-pop-item"
            :class="{ active: idx === store.currentIndex }"
            @click="store.playSongFromList(item, store.playQueue)"
          >
            <span class="q-idx">{{ idx + 1 }}</span>
            <div class="q-meta">
              <div class="q-title">{{ item.title }}</div>
              <div class="q-artist">{{ item.artist }}</div>
            </div>
            <span class="q-badge">{{ item.highestQuality }}</span>
          </div>
        </div>
      </div>
    </transition>
  </footer>
</template>
