<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from "vue";
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

interface FocusPresetItem {
  minutes: number;
  title: string;
  desc: string;
  channelId: string;
  channelName: string;
}

const focusPresets: FocusPresetItem[] = [
  { minutes: 25, title: "标准番茄钟", desc: "25 分钟高效深度专注", channelId: "306", channelName: "学习自习室" },
  { minutes: 45, title: "深度沉浸", desc: "45 分钟长时编码/阅读", channelId: "305", channelName: "轻音乐纯音" },
  { minutes: 60, title: "心流马拉松", desc: "60 分钟完整心流周期", channelId: "307", channelName: "咖啡馆 Bossa" },
  { minutes: 15, title: "小憩冥想", desc: "15 分钟闭目正念放空", channelId: "303", channelName: "深夜助眠" },
];

const selectedMinutes = ref(25);
const remainSeconds = ref(25 * 60);
const isRunning = ref(false);
const autoPlaySceneMusic = ref(true);
let timerId: number | null = null;

const formattedTimer = computed(() => {
  const m = Math.floor(remainSeconds.value / 60);
  const s = remainSeconds.value % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
});

const progressPercent = computed(() => {
  const total = Math.max(1, selectedMinutes.value * 60);
  return Math.min(100, Math.max(0, Math.round(((total - remainSeconds.value) / total) * 100)));
});

const activePresetObj = computed(
  () => focusPresets.find((p) => p.minutes === selectedMinutes.value) || focusPresets[0]
);

function selectPreset(preset: FocusPresetItem) {
  selectedMinutes.value = preset.minutes;
  remainSeconds.value = preset.minutes * 60;
  stopTimer();
}

function toggleTimer() {
  if (isRunning.value) {
    stopTimer();
  } else {
    isRunning.value = true;
    if (autoPlaySceneMusic.value && !store.isPlaying) {
      store.loadFmStationSongs(activePresetObj.value.channelId, false, true);
    }
    syncAllAmbientGains();
    timerId = window.setInterval(() => {
      if (remainSeconds.value > 0) {
        remainSeconds.value -= 1;
      } else {
        stopTimer();
        store.recordCompletedFocusSession(selectedMinutes.value);
        store.notify(`🎉 心流专注时段 (${selectedMinutes.value} 分钟) 已完成，休息一下吧！`);
        remainSeconds.value = selectedMinutes.value * 60;
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
    label: "窗外细雨",
    sub: "双层屋檐水滴与粉噪细雨",
    icon: "wave",
    active: false,
    volume: 0.36,
    filterFreq: 1250,
  },
  {
    id: "fire",
    label: "温暖篝火",
    sub: "松木爆裂火星与温暖底噪",
    icon: "sparkles",
    active: false,
    volume: 0.30,
    filterFreq: 820,
  },
  {
    id: "ocean",
    label: "深海潮汐",
    sub: "0.12Hz 慢呼吸海浪涌动",
    icon: "compass",
    active: false,
    volume: 0.32,
    filterFreq: 440,
  },
  {
    id: "vinyl",
    label: "黑胶唱针",
    sub: "33.3 RPM 模拟唱盘温暖微尘",
    icon: "disc",
    active: false,
    volume: 0.24,
    filterFreq: 2400,
  },
]);

const sceneMixPresets = [
  {
    id: "rainy_study",
    name: "🌧️ 雨夜书房",
    weights: { rain: 0.45, ocean: 0, fire: 0.26, vinyl: 0.18 } as Record<string, number>,
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
    name: "🔇 关闭伴音",
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

function getEffectiveGain(ch: AmbientChannel): number {
  if (!ch.active) return 0;
  if (store.ambientSyncWithMusic && !store.isPlaying) {
    return 0;
  }
  return ch.volume * 0.35;
}

function syncAllAmbientGains() {
  for (const ch of channels.value) {
    const node = activeNodes.get(ch.id);
    if (node) {
      node.gain.gain.value = getEffectiveGain(ch);
    }
  }
}

watch(
  () => [store.isPlaying, store.ambientSyncWithMusic],
  () => {
    syncAllAmbientGains();
  }
);

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
    const bufferSize = ctx.sampleRate * 4;
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
      let env = 1.0;
      if (ch.id === "ocean") {
        env = 0.52 + 0.48 * Math.sin((i / ctx.sampleRate) * Math.PI * 0.5);
      } else if (ch.id === "fire" && Math.random() < 0.0018) {
        env = 2.6;
      } else if (ch.id === "vinyl" && Math.random() < 0.0009) {
        env = 1.9;
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
    gain.gain.value = getEffectiveGain(ch);

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
    node.gain.gain.value = getEffectiveGain(ch);
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
      <div class="compact-glass-modal" style="width: 640px; max-width: 94vw">
        <div class="modal-header">
          <div class="vip-title-row">
            <div class="vip-icon-orb cyan">
              <PlumpIcon name="timer" :size="20" color="#b388ff" />
            </div>
            <div>
              <h3>概念胶囊 · MindFlow 心流专注与环境伴音舱</h3>
              <p class="vip-sub-caption">
                今日已完成专注 <strong>{{ store.focusTodayCompletedCount }}</strong> 次 · 累计
                <strong>{{ store.focusTodayTotalMinutes }}</strong> 分钟
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

        <!-- 番茄钟倒计时与专注场景联动卡片 -->
        <div
          class="vip-calendar-card mb-14"
          style="display: flex; flex-direction: column; align-items: center; gap: 12px; position: relative; overflow: hidden"
        >
          <div style="display: flex; align-items: baseline; gap: 12px">
            <div style="font-size: 46px; font-weight: 900; color: var(--cyan); font-variant-numeric: tabular-nums; letter-spacing: 1px">
              {{ formattedTimer }}
            </div>
            <span class="mini-badge sq" style="font-size: 12px">
              {{ isRunning ? `专注进行中 ${progressPercent}%` : activePresetObj.title }}
            </span>
          </div>

          <div style="font-size: 13px; color: var(--text-secondary)">
            {{ activePresetObj.desc }} · 推荐联动电台：<strong>{{ activePresetObj.channelName }}</strong>
          </div>

          <div class="pill-tabs" style="flex-wrap: wrap; justify-content: center">
            <button
              v-for="p in focusPresets"
              :key="p.minutes"
              class="pill-tab-btn"
              :class="{ active: selectedMinutes === p.minutes }"
              @click="selectPreset(p)"
            >
              {{ p.title }} ({{ p.minutes }}m)
            </button>
          </div>

          <div style="display: flex; align-items: center; gap: 12px; margin-top: 4px; flex-wrap: wrap; justify-content: center">
            <button class="primary-btn" @click="toggleTimer">
              <PlumpIcon
                :name="isRunning ? 'pause' : 'play'"
                :size="16"
                color="#ffffff"
              />
              <span>{{ isRunning ? "暂停心流专注" : "开启心流专注" }}</span>
            </button>
            <button class="tonal-pill-btn" @click="resetTimer">
              <PlumpIcon name="refresh" :size="15" />
              <span>重置计时</span>
            </button>
            <button
              class="tonal-pill-btn"
              title="立即播放当前专注模式匹配的场景电台"
              @click="store.loadFmStationSongs(activePresetObj.channelId, false, true)"
            >
              <PlumpIcon name="radio" :size="15" />
              <span>播放「{{ activePresetObj.channelName }}」</span>
            </button>
          </div>

          <!-- 进度细条 -->
          <div style="width: 100%; height: 4px; border-radius: 999px; background: rgba(255,255,255,0.08); overflow: hidden; margin-top: 4px">
            <div
              :style="{
                width: `${progressPercent}%`,
                height: '100%',
                background: 'linear-gradient(90deg, var(--cyan), #b388ff)',
                transition: 'width 0.4s ease'
              }"
            ></div>
          </div>
        </div>

        <!-- 环境伴音智能协同开关与一键场景混音预设 -->
        <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 10px; flex-wrap: wrap; gap: 8px">
          <div style="display: flex; align-items: center; gap: 10px">
            <span class="cfg-label" style="margin-bottom: 0">4 路程序化环境白噪音混音矩阵</span>
            <label style="display: inline-flex; align-items: center; gap: 6px; font-size: 12.5px; color: var(--text-secondary); cursor: pointer">
              <input v-model="store.ambientSyncWithMusic" type="checkbox" />
              <span>仅在播放音乐时伴音</span>
            </label>
          </div>
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
            :style="ch.active ? 'border-color: rgba(0, 229, 255, 0.42); background: rgba(0, 229, 255, 0.07)' : ''"
            @click="toggleAmbientChannel(ch)"
          >
            <div style="display: flex; justify-content: space-between; align-items: center">
              <div style="display: flex; align-items: center; gap: 8px">
                <PlumpIcon :name="ch.icon as any" :size="17" color="var(--cyan)" />
                <div>
                  <div style="font-weight: 800; font-size: 15px">{{ ch.label }}</div>
                  <div style="font-size: 12.5px; color: var(--text-secondary)">
                    {{ ch.sub }}
                  </div>
                </div>
              </div>
              <span
                class="mini-badge"
                :class="ch.active ? 'sq' : 'vip'"
              >
                {{ ch.active ? `${Math.round(ch.volume * 100)}%` : "已关" }}
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
