<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import { useAppStore } from "../stores/appStore";
import {
  X,
  Timer,
  CloudRain,
  Waves,
  Flame,
  Wind,
  Disc,
  Play,
  Pause,
  RotateCcw,
} from "lucide-vue-next";

const store = useAppStore();

// 番茄钟专注计时器状态
const presets = [15, 25, 45, 60];
const selectedMinutes = ref(25);
const remainSeconds = ref(25 * 60);
const isRunning = ref(false);
let timerId: number | null = null;

const formattedTimer = computed(() => {
  const m = Math.floor(remainSeconds.value / 60);
  const s = remainSeconds.value % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
});

function selectPreset(min: number) {
  selectedMinutes.value = min;
  remainSeconds.value = min * 60;
  stopTimer();
}

function toggleTimer() {
  if (isRunning.value) {
    stopTimer();
  } else {
    isRunning.value = true;
    timerId = window.setInterval(() => {
      if (remainSeconds.value > 0) {
        remainSeconds.value -= 1;
      } else {
        stopTimer();
        store.showBanner("心流专注时段已完成，休息一下吧！");
      }
    }, 1000);
  }
}

function stopTimer() {
  isRunning.value = false;
  if (timerId !== null) {
    clearInterval(timerId);
    timerId = null;
  }
}

function resetTimer() {
  stopTimer();
  remainSeconds.value = selectedMinutes.value * 60;
}

// Web Audio API 程序化环境白噪音合成器（对齐 Android 端 AmbientSoundManager.kt，零外部音频文件依赖）
interface AmbientChannel {
  id: string;
  label: string;
  sub: string;
  icon: any;
  active: boolean;
  volume: number;
  filterFreq: number;
}

const channels = ref<AmbientChannel[]>([
  {
    id: "rain",
    label: "微风细雨",
    sub: "柔和粉红噪点雨声",
    icon: CloudRain,
    active: false,
    volume: 0.35,
    filterFreq: 1200,
  },
  {
    id: "ocean",
    label: "深海潮汐",
    sub: "低频呼吸海浪涌动",
    icon: Waves,
    active: false,
    volume: 0.3,
    filterFreq: 420,
  },
  {
    id: "fire",
    label: "壁炉篝火",
    sub: "温暖中低频木柴声",
    icon: Flame,
    active: false,
    volume: 0.28,
    filterFreq: 780,
  },
  {
    id: "wind",
    label: "山谷林风",
    sub: "空灵带通微风声",
    icon: Wind,
    active: false,
    volume: 0.25,
    filterFreq: 600,
  },
  {
    id: "vinyl",
    label: "黑胶底噪",
    sub: "复古模拟唱片暖感",
    icon: Disc,
    active: false,
    volume: 0.2,
    filterFreq: 2400,
  },
]);

let audioCtx: AudioContext | null = null;
const activeNodes = new Map<
  string,
  { source: AudioBufferSourceNode; gain: GainNode }
>();

function ensureAudioContext(): AudioContext {
  if (!audioCtx) {
    audioCtx = new AudioContext();
  }
  if (audioCtx.state === "suspended") {
    audioCtx.resume();
  }
  return audioCtx;
}

function toggleAmbientChannel(ch: AmbientChannel) {
  ch.active = !ch.active;
  if (ch.active) {
    const ctx = ensureAudioContext();
    const bufferSize = ctx.sampleRate * 3;
    const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
    const output = buffer.getChannelData(0);
    let b0 = 0,
      b1 = 0,
      b2 = 0;
    for (let i = 0; i < bufferSize; i++) {
      const white = Math.random() * 2 - 1;
      b0 = 0.997 * b0 + white * 0.029;
      b1 = 0.985 * b1 + white * 0.032;
      b2 = 0.95 * b2 + white * 0.048;
      output[i] = (b0 + b1 + b2 + white * 0.015) * 0.35;
    }

    const noise = ctx.createBufferSource();
    noise.buffer = buffer;
    noise.loop = true;

    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = ch.filterFreq;

    const gain = ctx.createGain();
    gain.gain.value = ch.volume * 0.35;

    noise.connect(filter);
    filter.connect(gain);
    gain.connect(ctx.destination);
    noise.start();

    activeNodes.set(ch.id, { source: noise, gain });
  } else {
    const node = activeNodes.get(ch.id);
    if (node) {
      try {
        node.source.stop();
        node.source.disconnect();
      } catch {}
      activeNodes.delete(ch.id);
    }
  }
}

function updateChannelVolume(ch: AmbientChannel, val: number) {
  ch.volume = val;
  const node = activeNodes.get(ch.id);
  if (node) {
    node.gain.gain.value = val * 0.35;
  }
}

onUnmounted(() => {
  stopTimer();
  for (const [, node] of activeNodes) {
    try {
      node.source.stop();
      node.source.disconnect();
    } catch {}
  }
  activeNodes.clear();
});
</script>

<template>
  <div class="modal-backdrop" @click.self="store.isFocusPodOpen = false">
    <div class="glass-modal focus-pod-modal">
      <div class="modal-header">
        <div class="vip-modal-title-row">
          <div class="focus-violet-circle">
            <Timer :size="18" />
          </div>
          <div>
            <h3>概念投喂 · Focus Pod 心流舱</h3>
            <p class="modal-subtitle">
              番茄心流专注计时器 × 实时程序化环境白噪音混音引擎
            </p>
          </div>
        </div>
        <button class="icon-btn" @click="store.isFocusPodOpen = false">
          <X :size="20" />
        </button>
      </div>

      <!-- 番茄钟倒计时卡片 -->
      <div class="focus-timer-card">
        <div class="timer-display">{{ formattedTimer }}</div>
        <div class="timer-presets">
          <button
            v-for="p in presets"
            :key="p"
            class="preset-chip"
            :class="{ active: selectedMinutes === p }"
            @click="selectPreset(p)"
          >
            {{ p }} 分钟
          </button>
        </div>
        <div class="timer-controls">
          <button class="primary-btn" @click="toggleTimer">
            <Pause v-if="isRunning" :size="16" />
            <Play v-else :size="16" />
            <span>{{ isRunning ? "暂停专注" : "开启心流专注" }}</span>
          </button>
          <button class="secondary-btn" @click="resetTimer">
            <RotateCcw :size="15" />
            <span>重置</span>
          </button>
        </div>
      </div>

      <!-- 环境白噪音混音矩阵 -->
      <div class="ambient-section">
        <div class="ambient-head">
          <span>环境伴音混音台（可与当前播放音乐叠加共鸣）</span>
        </div>
        <div class="ambient-grid">
          <div
            v-for="ch in channels"
            :key="ch.id"
            class="ambient-item-card"
            :class="{ active: ch.active }"
          >
            <div class="ambient-top" @click="toggleAmbientChannel(ch)">
              <component :is="ch.icon" :size="18" class="amb-ico" />
              <div class="amb-texts">
                <div class="amb-title">{{ ch.label }}</div>
                <div class="amb-sub">{{ ch.sub }}</div>
              </div>
              <span class="amb-status">{{ ch.active ? "开启" : "关闭" }}</span>
            </div>
            <input
              v-if="ch.active"
              type="range"
              min="0.05"
              max="1"
              step="0.05"
              :value="ch.volume"
              class="ambient-slider"
              @input="
                updateChannelVolume(
                  ch,
                  Number(($event.target as HTMLInputElement).value)
                )
              "
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
