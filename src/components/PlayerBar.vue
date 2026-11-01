<script setup lang="ts">
import { computed, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import type { AudioQualityValue } from "../types";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();
const showQualityMenu = ref(false);

const qualityOptions: {
  val: AudioQualityValue;
  label: string;
  desc: string;
}[] = [
  { val: "high", label: "Hi-Res 高解析", desc: "24bit / 96kHz~192kHz 母带无损" },
  { val: "flac", label: "无损 SQ", desc: "16bit / 44.1kHz~48kHz FLAC 无损" },
  { val: "320", label: "高品 320K", desc: "320kbps 饱满立体声" },
  { val: "128", label: "标准 128K", desc: "轻量省流流畅聆听" },
];

const progressPercent = computed(() => {
  if (!store.durationMs || store.durationMs <= 0) return 0;
  return Math.min(
    100,
    Math.max(0, (store.currentPositionMs / store.durationMs) * 100)
  );
});

const climaxRangePercent = computed(() => {
  const pb = store.currentPlaybackInfo;
  const dur = store.durationMs;
  if (!pb || !dur || dur <= 0 || !pb.climaxEndMs) return null;
  const left = Math.max(0, Math.min(92, (pb.climaxStartMs / dur) * 100));
  const width = Math.max(
    4,
    Math.min(100 - left, ((pb.climaxEndMs - pb.climaxStartMs) / dur) * 100)
  );
  return { left: `${left}%`, width: `${width}%` };
});

function formatTime(ms: number): string {
  if (!ms || ms < 0) return "00:00";
  const totalSec = Math.floor(ms / 1000);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

function onProgressInput(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  const targetMs = Math.floor((val / 100) * (store.durationMs || 0));
  store.seekToMs(targetMs);
}

function onVolumeInput(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  store.setVolume(val / 100);
}

function selectQuality(q: AudioQualityValue) {
  showQualityMenu.value = false;
  store.switchAudioQuality(q);
}

const playModeIconName = computed(() => {
  if (store.playMode === "repeat_one") return "repeat-one";
  if (store.playMode === "shuffle") return "shuffle";
  return "repeat-all";
});
</script>

<template>
  <footer class="floating-player-dock">
    <!-- 顶部光学彗星进度条（含副歌高潮区间高亮标记） -->
    <div class="dock-progress-strip">
      <div
        class="dock-progress-fill"
        :style="{ width: `${progressPercent}%` }"
      >
        <span class="comet-head-glow"></span>
      </div>
      <div
        v-if="climaxRangePercent"
        class="dock-climax-marker"
        :style="climaxRangePercent"
        title="歌曲副歌高潮精华段落 (Climax)"
      ></div>
      <input
        type="range"
        min="0"
        max="100"
        step="0.1"
        :value="progressPercent"
        class="dock-progress-slider"
        @input="onProgressInput"
      />
    </div>

    <div class="dock-content">
      <!-- 左侧：旋转圆润封面 + 歌曲信息 + 硬件级 STREAMINFO 规格标签 -->
      <div
        class="dock-song-left"
        title="点击展开平板级双栏沉浸播放舞台"
        @click="store.isNowPlayingOpen = !store.isNowPlayingOpen"
      >
        <div class="dock-cover-wrapper" :class="{ playing: store.isPlaying }">
          <img
            v-if="store.currentSong?.coverUrl"
            :src="store.currentSong.coverUrl"
            alt="cover"
            class="dock-cover-img"
          />
          <div v-else class="dock-cover-placeholder">
            <PlumpIcon name="disc" :size="22" />
          </div>
        </div>

        <div class="dock-song-meta">
          <div class="dock-song-title-row">
            <span class="dock-song-title">
              {{ store.currentSong?.title || "微风音乐 · 概念版无损直连" }}
            </span>
            <button
              v-if="store.currentSong"
              class="dock-like-inline"
              title="收藏至我喜欢的音乐"
              @click.stop="store.toggleFavoriteSong(store.currentSong)"
            >
              <PlumpIcon
                name="heart"
                :size="16"
                :filled="store.isSongLiked(store.currentSong)"
                :color="
                  store.isSongLiked(store.currentSong)
                    ? '#ff4d6d'
                    : 'var(--text-secondary)'
                "
              />
            </button>
          </div>
          <div class="dock-song-sub">
            <span
              class="dock-artist-link"
              @click.stop="
                store.currentSong &&
                  store.openArtistDetail(
                    store.currentSong.artistId,
                    store.currentSong.artist,
                    store.currentSong.coverUrl
                  )
              "
            >
              {{ store.currentSong?.artist || "轻触任意歌曲开启帧享无损之旅" }}
            </span>
            <span
              v-if="store.currentPlaybackInfo?.techSpecText"
              class="stream-spec-badge"
            >
              {{ store.currentPlaybackInfo.techSpecText }}
            </span>
            <span
              v-else-if="store.currentSong?.highestQuality"
              class="stream-spec-badge"
            >
              {{ store.currentSong.highestQuality }}
            </span>
          </div>
        </div>
      </div>

      <!-- 中间：软润饱满播放传输控制区（对齐 Android NowPlayingCanvasControls） -->
      <div class="dock-transport-center">
        <div class="transport-buttons">
          <button
            class="dock-icon-btn"
            :title="
              store.playMode === 'repeat_all'
                ? '列表循环'
                : store.playMode === 'repeat_one'
                ? '单曲循环'
                : '随机播放'
            "
            @click="store.cyclePlayMode()"
          >
            <PlumpIcon :name="playModeIconName" :size="18" />
          </button>

          <button
            class="dock-icon-btn"
            title="上一首"
            @click="store.playPrev()"
          >
            <PlumpIcon name="prev" :size="20" />
          </button>

          <button
            class="dock-play-main-btn"
            :title="store.isPlaying ? '暂停' : '播放'"
            @click="store.togglePlayPause()"
          >
            <PlumpIcon
              :name="store.isPlaying ? 'pause' : 'play'"
              :size="22"
              color="#ffffff"
            />
          </button>

          <button
            class="dock-icon-btn"
            title="下一首"
            @click="store.playNext(false)"
          >
            <PlumpIcon name="next" :size="20" />
          </button>

          <button
            class="dock-icon-btn"
            :class="{ 'active-accent': store.climaxModeEnabled }"
            title="副歌速刷模式 (Climax 30秒高潮连播)"
            @click="
              store.climaxModeEnabled = !store.climaxModeEnabled;
              store.notify(
                store.climaxModeEnabled
                  ? '已开启副歌速刷 (30秒精华连播)'
                  : '已恢复整首完整播放'
              );
            "
          >
            <PlumpIcon
              name="sparkles"
              :size="18"
              :color="
                store.climaxModeEnabled ? 'var(--gold)' : 'currentColor'
              "
            />
          </button>
        </div>

        <div class="dock-time-row">
          <span>{{ formatTime(store.currentPositionMs) }}</span>
          <span class="time-sep">/</span>
          <span>
            {{
              formatTime(store.durationMs || store.currentSong?.durationMs || 0)
            }}
          </span>
        </div>
      </div>

      <!-- 右侧：音质切换、桌面歌词、母带音效、音量与沉浸歌词页开关 -->
      <div class="dock-tools-right">
        <!-- 音质档位切换上拉菜单 -->
        <div class="quality-dropdown-wrapper">
          <button
            class="quality-pill-btn"
            @click="showQualityMenu = !showQualityMenu"
          >
            <PlumpIcon name="sparkles" :size="14" color="var(--cyan)" />
            <span>
              {{
                store.currentPlaybackInfo?.qualityLabel ||
                (store.preferredQuality === "high"
                  ? "Hi-Res"
                  : store.preferredQuality === "flac"
                  ? "无损 SQ"
                  : store.preferredQuality === "320"
                  ? "高品 320K"
                  : "标准 128K")
              }}
            </span>
          </button>

          <transition name="pop-spring">
            <div v-if="showQualityMenu" class="quality-popover-menu">
              <div class="qp-header">在线解析音质档位</div>
              <button
                v-for="opt in qualityOptions"
                :key="opt.val"
                class="qp-item"
                :class="{ active: store.preferredQuality === opt.val }"
                @click="selectQuality(opt.val)"
              >
                <div class="qp-item-top">
                  <span class="qp-label">{{ opt.label }}</span>
                  <PlumpIcon
                    v-if="store.preferredQuality === opt.val"
                    name="check"
                    :size="15"
                  />
                </div>
                <div class="qp-desc">{{ opt.desc }}</div>
              </button>
            </div>
          </transition>
        </div>

        <!-- 桌面悬浮歌词开关 -->
        <button
          class="dock-icon-btn"
          :class="{ 'active-accent': store.desktopLyricEnabled }"
          title="开启/关闭桌面穿透悬浮歌词"
          @click="store.desktopLyricEnabled = !store.desktopLyricEnabled"
        >
          <PlumpIcon
            name="lyric"
            :size="19"
            :color="
              store.desktopLyricEnabled ? 'var(--cyan)' : 'currentColor'
            "
          />
        </button>

        <!-- 音量调节 -->
        <div class="volume-control-group">
          <PlumpIcon
            name="volume"
            :size="17"
            color="var(--text-secondary)"
            accent="var(--cyan)"
          />
          <input
            type="range"
            min="0"
            max="100"
            :value="Math.round(store.volume * 100)"
            class="volume-slider"
            @input="onVolumeInput"
          />
        </div>

        <!-- 展开/收起平板级播放舞台 -->
        <button
          class="dock-icon-btn"
          :class="{ 'active-accent': store.isNowPlayingOpen }"
          title="展开/收起双栏沉浸播放页"
          @click="store.isNowPlayingOpen = !store.isNowPlayingOpen"
        >
          <PlumpIcon
            name="disc"
            :size="19"
            :color="store.isNowPlayingOpen ? 'var(--cyan)' : 'currentColor'"
          />
        </button>
      </div>
    </div>
  </footer>
</template>
