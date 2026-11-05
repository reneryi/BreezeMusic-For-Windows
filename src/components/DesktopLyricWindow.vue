<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import PlumpIcon from "./PlumpIcon.vue";

interface LyricWordItem {
  text: string;
  startTimeMs: number;
  durationMs: number;
}

interface LyricLineItem {
  startTimeMs: number;
  durationMs: number;
  text: string;
  translation: string;
  phonetic: string;
  words: LyricWordItem[];
}

const isLocked = ref<boolean>(localStorage.getItem("bm_desktop_lyric_locked") === "true");
const isHovering = ref<boolean>(false);
const color = ref<string>(localStorage.getItem("bm_desktop_lyric_color") || "#00e5ff");
const fontSize = ref<number>(Number(localStorage.getItem("bm_desktop_lyric_size")) || 22);
const widthPx = ref<number>(Number(localStorage.getItem("bm_desktop_lyric_width")) || 440);
const showTranslation = ref<boolean>(localStorage.getItem("bm_desktop_lyric_trans") !== "false");
const isPlaying = ref<boolean>(false);
const positionMs = ref<number>(0);
const lyricOffsetMs = ref<number>(Number(localStorage.getItem("bm_lyric_offset_ms")) || 0);
const bluetoothDelayMs = ref<number>(Number(localStorage.getItem("bm_bt_delay_ms")) || 0);

const currentSong = ref<{
  hash: string;
  title: string;
  artist: string;
  coverUrl: string;
} | null>(null);

const lyricLines = ref<LyricLineItem[]>([]);
const isKrc = ref<boolean>(false);

const colorPresets = [
  "#00e5ff", // 极光青
  "#ffd54f", // 流金珀
  "#ff6b8b", // 樱花粉
  "#00e5a3", // 翡翠绿
  "#b388ff", // 星云紫
  "#ffffff", // 纯净白
];

let channel: BroadcastChannel | null = null;

function sendCmd(type: string, extra: Record<string, any> = {}) {
  if (!channel) return;
  channel.postMessage({ type, ...extra });
}

const calibratedPosMs = computed(
  () => positionMs.value + lyricOffsetMs.value - bluetoothDelayMs.value
);

const activePair = computed(() => {
  const lines = lyricLines.value;
  if (!lines || lines.length === 0) {
    return {
      main: currentSong.value?.title || "微风音乐 · 概念版桌面歌词",
      sub: currentSong.value?.artist || "耳畔微风起 · 帧享真无损",
      words: [] as LyricWordItem[],
      lineStartMs: 0,
      isLongLine: false,
    };
  }
  const pos = calibratedPosMs.value;
  let active = lines[0];
  let nextLine = lines[1];
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].startTimeMs <= pos) {
      active = lines[i];
      nextLine = lines[i + 1];
    } else {
      break;
    }
  }
  const mainText = active?.text || "";
  const subText =
    (showTranslation.value && active?.translation) ||
    nextLine?.text ||
    "";
  return {
    main: mainText,
    sub: subText,
    words: active?.words || [],
    lineStartMs: active?.startTimeMs || 0,
    isLongLine: mainText.length > 22,
  };
});

function wordFillPct(lineStartMs: number, wStart: number, wDur: number): number {
  const pos = calibratedPosMs.value;
  const absStart = wStart >= lineStartMs ? wStart : lineStartMs + wStart;
  const dur = Math.max(80, wDur);
  if (pos <= absStart) return 0;
  if (pos >= absStart + dur) return 100;
  return Math.min(100, ((pos - absStart) / dur) * 100);
}

async function handleStartDrag(e: MouseEvent) {
  if (isLocked.value) return;
  const target = e.target as HTMLElement | null;
  if (target && (target.closest("button") || target.closest("input"))) return;
  try {
    await getCurrentWindow().startDragging();
  } catch {
    // ignore in browser preview
  }
}

function cycleColor() {
  const idx = colorPresets.indexOf(color.value);
  const next = colorPresets[(idx + 1) % colorPresets.length];
  color.value = next;
  sendCmd("CMD_SET_COLOR", { color: next });
}

function adjustWidth(delta: number) {
  const next = Math.max(340, Math.min(720, widthPx.value + delta));
  widthPx.value = next;
  sendCmd("CMD_SET_WIDTH", { width: next });
}

onMounted(() => {
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";

  if (typeof BroadcastChannel !== "undefined") {
    channel = new BroadcastChannel("bm_desktop_lyric_channel");
    channel.onmessage = (ev) => {
      const msg = ev.data;
      if (!msg || msg.type !== "STATE_SYNC") return;
      isLocked.value = Boolean(msg.locked);
      if (msg.color) color.value = msg.color;
      if (msg.fontSize) fontSize.value = Number(msg.fontSize);
      if (msg.width) widthPx.value = Number(msg.width);
      showTranslation.value = Boolean(msg.showTranslation);
      isPlaying.value = Boolean(msg.isPlaying);
      positionMs.value = Number(msg.positionMs || 0);
      lyricOffsetMs.value = Number(msg.lyricOffsetMs || 0);
      bluetoothDelayMs.value = Number(msg.bluetoothDelayMs || 0);
      currentSong.value = msg.song || null;
      if (msg.lyric) {
        isKrc.value = Boolean(msg.lyric.isKrc);
        lyricLines.value = msg.lyric.lines || [];
      }
    };
    sendCmd("CMD_REQUEST_SYNC");
  }
});

onBeforeUnmount(() => {
  if (channel) {
    channel.close();
    channel = null;
  }
});
</script>

<template>
  <div
    class="desktop-lyric-standalone-shell"
    :class="{
      'is-locked': isLocked,
      'is-hovering': isHovering,
    }"
    :style="{
      '--dl-accent': color,
      '--dl-font-size': `${fontSize}px`,
    }"
    @mouseenter="isHovering = true"
    @mouseleave="isHovering = false"
    @mousedown="handleStartDrag"
  >
    <!-- 顶部迷你控制栏（未锁定且悬停时显示） -->
    <div v-if="!isLocked" class="dl-toolbar">
      <span class="dl-song-tag">
        ✥ {{ currentSong?.title || "微风桌面歌词" }}
      </span>
      <div class="dl-btn-group">
        <button class="dl-icon-btn" title="上一曲" @click.stop="sendCmd('CMD_PREV')">
          <PlumpIcon name="prev" :size="12" />
        </button>
        <button class="dl-icon-btn" title="播放/暂停" @click.stop="sendCmd('CMD_TOGGLE_PLAY')">
          <PlumpIcon :name="isPlaying ? 'pause' : 'play'" :size="12" />
        </button>
        <button class="dl-icon-btn" title="下一曲" @click.stop="sendCmd('CMD_NEXT')">
          <PlumpIcon name="next" :size="12" />
        </button>
        <button class="dl-icon-btn" title="缩短窗口宽度 (-30px)" @click.stop="adjustWidth(-30)">
          W-
        </button>
        <button class="dl-icon-btn" title="加宽窗口宽度 (+30px)" @click.stop="adjustWidth(30)">
          W+
        </button>
        <button class="dl-icon-btn" title="切换歌词配色" @click.stop="cycleColor">
          🎨
        </button>
        <button class="dl-icon-btn" title="锁定鼠标穿透" @click.stop="sendCmd('CMD_LOCK')">
          <PlumpIcon name="lock" :size="12" />
        </button>
        <button class="dl-icon-btn" title="关闭桌面歌词" @click.stop="sendCmd('CMD_CLOSE')">
          <PlumpIcon name="close" :size="12" />
        </button>
      </div>
    </div>

    <!-- 歌词主体（短句两行居中，超长句自动折行） -->
    <div class="dl-lyric-body" :class="{ 'long-wrap': activePair.isLongLine }">
      <div
        v-if="activePair.words.length > 0"
        class="dl-main-line krc-mode"
      >
        <span
          v-for="(w, idx) in activePair.words"
          :key="idx"
          class="dl-krc-word"
          :style="{
            '--w-pct': `${wordFillPct(activePair.lineStartMs, w.startTimeMs, w.durationMs)}%`,
          }"
        >
          {{ w.text }}
        </span>
      </div>
      <div v-else class="dl-main-line">
        {{ activePair.main }}
      </div>

      <div v-if="activePair.sub" class="dl-sub-line">
        {{ activePair.sub }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.desktop-lyric-standalone-shell {
  width: 100vw;
  height: 100vh;
  box-sizing: border-box;
  padding: 6px 14px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  border-radius: 16px;
  background: rgba(10, 14, 24, 0.35);
  border: 1px solid rgba(255, 255, 255, 0.12);
  backdrop-filter: blur(14px);
  cursor: grab;
  user-select: none;
  overflow: hidden;
  transition: background 0.22s ease, border-color 0.22s ease;
}
.desktop-lyric-standalone-shell.is-locked {
  background: transparent;
  border-color: transparent;
  backdrop-filter: none;
  cursor: default;
}
.dl-toolbar {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  margin-bottom: 4px;
  opacity: 0.25;
  transition: opacity 0.2s ease;
}
.desktop-lyric-standalone-shell:hover .dl-toolbar {
  opacity: 1;
}
.dl-song-tag {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.78);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 150px;
}
.dl-btn-group {
  display: flex;
  align-items: center;
  gap: 3px;
}
.dl-icon-btn {
  height: 20px;
  min-width: 20px;
  padding: 0 5px;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
  font-size: 10.5px;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}
.dl-icon-btn:hover {
  background: rgba(0, 229, 255, 0.25);
  border-color: rgba(0, 229, 255, 0.55);
}
.dl-lyric-body {
  width: 100%;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 3px;
}
.dl-main-line {
  font-size: var(--dl-font-size, 22px);
  font-weight: 900;
  line-height: 1.25;
  color: var(--dl-accent, #00e5ff);
  text-shadow: 0 2px 10px rgba(0, 0, 0, 0.85), 0 0 18px rgba(0, 229, 255, 0.28);
  word-break: break-word;
}
.dl-krc-word {
  display: inline-block;
  background: linear-gradient(
    90deg,
    var(--dl-accent, #00e5ff) var(--w-pct, 0%),
    rgba(255, 255, 255, 0.92) var(--w-pct, 0%)
  );
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.82));
}
.dl-sub-line {
  font-size: calc(var(--dl-font-size, 22px) * 0.64);
  font-weight: 600;
  color: rgba(255, 255, 255, 0.85);
  text-shadow: 0 1px 6px rgba(0, 0, 0, 0.85);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 96%;
}
</style>
