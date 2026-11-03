<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

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
        store.notify("心流专注时段已完成，休息一下吧！");
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

interface AmbientChannel {
  id: string;
  label: string;
  sub: string;
  icon: string;
  active: boolean;
  volume: number;
  filterFreq: number;
}

const channels = ref<AmbientChannel[]>([
  {
    id: "rain",
    label: "微风细雨",
    sub: "柔和粉红噪点雨声",
    icon: "wave",
    active: false,
    volume: 0.35,
    filterFreq: 1200,
  },
  {
    id: "ocean",
    label: "深海潮汐",
    sub: "低频呼吸海浪涌动",
    icon: "compass",
    active: false,
    volume: 0.3,
    filterFreq: 420,
  },
  {
    id: "fire",
    label: "壁炉篝火",
    sub: "温暖中低频木柴声",
    icon: "sparkles",
    active: false,
    volume: 0.28,
    filterFreq: 780,
  },
  {
    id: "vinyl",
    label: "黑胶底噪",
    sub: "复古模拟唱片暖感",
    icon: "disc",
    active: false,
    volume: 0.22,
    filterFreq: 2400,
  },
]);

const sceneMixPresets = [
  {
    id: "rainy_study",
    name: "🌧️ 雨夜书房",
    weights: { rain: 0.45, ocean: 0, fire: 0.25, vinyl: 0.18 } as Record<string, number>,
  },
  {
    id: "coastal_breeze",
    name: "🌊 海岸微风",
    weights: { rain: 0.15, ocean: 0.48, fire: 0, vinyl: 0 } as Record<string, number>,
  },
  {
    id: "campfire_vinyl",
    name: "🔥 篝火黑胶",
    weights: { rain: 0, ocean: 0, fire: 0.42, vinyl: 0.28 } as Record<string, number>,
  },
  {
    id: "mute_all",
    name: "🔇 关闭环境音",
    weights: { rain: 0, ocean: 0, fire: 0, vinyl: 0 } as Record<string, number>,
  },
];

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

function setChannelState(ch: AmbientChannel, shouldActive: boolean, targetVol?: number) {
  if (targetVol !== undefined && targetVol > 0) {
    ch.volume = targetVol;
  }
  if (shouldActive && !ch.active) {
    toggleAmbientChannel(ch);
  } else if (!shouldActive && ch.active) {
    toggleAmbientChannel(ch);
  } else if (shouldActive && ch.active) {
    updateChannelVolume(ch, ch.volume);
  }
}

function applySceneMix(scene: { name: string; weights: Record<string, number> }) {
  for (const ch of channels.value) {
    const w = scene.weights[ch.id] || 0;
    setChannelState(ch, w > 0, w > 0 ? w : ch.volume);
  }
  store.notify(`已切换心流环境声景：${scene.name}`);
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
      // 针对不同通道塑造独特声学纹理（海浪呼吸包络 / 篝火微爆裂 / 雨声粉噪）
      let env = 1.0;
      if (ch.id === "ocean") {
        env = 0.55 + 0.45 * Math.sin((i / ctx.sampleRate) * Math.PI * 0.66);
      } else if (ch.id === "fire" && Math.random() < 0.0015) {
        env = 2.4;
      }
      output[i] = (b0 + b1 + b2 + white * 0.015) * 0.35 * env;
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
  <transition name="fade">
    <div
      v-if="store.isFocusPodOpen"
      class="modal-backdrop"
      @click.self="store.isFocusPodOpen = false"
    >
      <div class="compact-glass-modal" style="width: 580px">
        <div class="modal-header">
          <div class="vip-title-row">
            <div class="vip-icon-orb cyan">
              <PlumpIcon name="timer" :size="20" color="#b388ff" />
            </div>
            <div>
              <h3>概念胶囊 · Focus Pod 心流舱</h3>
              <p class="vip-sub-caption">
                番茄心流专注计时器 × 4 路实时程序化环境白噪音混音引擎
              </p>
            </div>
          </div>
          <button
            class="icon-close-btn"
            @click="store.isFocusPodOpen = false"
          >
            <PlumpIcon name="close" :size="15" />
          </button>
        </div>

        <!-- 番茄钟倒计时卡片 -->
        <div
          class="vip-calendar-card mb-14"
          style="display: flex; flex-direction: column; align-items: center; gap: 12px"
        >
          <div style="font-size: 42px; font-weight: 900; color: var(--cyan)">
            {{ formattedTimer }}
          </div>
          <div class="pill-tabs">
            <button
              v-for="p in presets"
              :key="p"
              class="pill-tab-btn"
              :class="{ active: selectedMinutes === p }"
              @click="selectPreset(p)"
            >
              {{ p }} 分钟
            </button>
          </div>
          <div style="display: flex; gap: 10px">
            <button class="primary-btn" @click="toggleTimer">
              <PlumpIcon
                :name="isRunning ? 'pause' : 'play'"
                :size="16"
                color="#ffffff"
              />
              <span>{{ isRunning ? "暂停专注" : "开启心流专注" }}</span>
            </button>
            <button class="tonal-pill-btn" @click="resetTimer">
              <PlumpIcon name="refresh" :size="15" />
              <span>重置</span>
            </button>
          </div>
        </div>

        <!-- 一键场景混音预设 -->
        <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 10px; flex-wrap: wrap; gap: 8px">
          <span class="cfg-label" style="margin-bottom: 0">环境伴音混音台（可与当前播放音乐叠加共鸣）</span>
          <div class="bg-choice-pills">
            <button
              v-for="sc in sceneMixPresets"
              :key="sc.id"
              class="bg-pill-btn"
              style="padding: 4px 10px; font-size: 12.5px"
              @click="applySceneMix(sc)"
            >
              {{ sc.name }}
            </button>
          </div>
        </div>

        <!-- 4 路环境白噪音混音矩阵 -->
        <div class="cache-grid">
          <div
            v-for="ch in channels"
            :key="ch.id"
            class="cache-item-card"
            style="flex-direction: column; align-items: stretch; gap: 8px; cursor: pointer"
            @click="toggleAmbientChannel(ch)"
          >
            <div style="display: flex; justify-content: space-between; align-items: center">
              <div style="display: flex; align-items: center; gap: 8px">
                <PlumpIcon :name="ch.icon" :size="17" color="var(--cyan)" />
                <div>
                  <div style="font-weight: 800; font-size: 15.5px">{{ ch.label }}</div>
                  <div style="font-size: 13.5px; color: var(--text-secondary)">
                    {{ ch.sub }}
                  </div>
                </div>
              </div>
              <span
                class="mini-badge"
                :class="ch.active ? 'sq' : 'vip'"
              >
                {{ ch.active ? "混音中" : "已关" }}
              </span>
            </div>
            <input
              v-if="ch.active"
              type="range"
              min="0.05"
              max="1"
              step="0.05"
              :value="ch.volume"
              @click.stop
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
  </transition>
</template>
