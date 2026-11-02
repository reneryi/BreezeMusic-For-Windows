<template>
  <div class="tuner-studio-page">
    <!-- 顶部母带声学状态横幅（无大胶囊框，嵌入背景层） -->
    <section class="tuner-hero-banner">
      <div class="tuner-hero-left">
        <div class="tuner-eyebrow">CONCEPT MASTERING DESK · 64-BIT LTI DSP ENGINE</div>
        <h2 class="tuner-hero-title">概念版声学调音台</h2>
        <p class="tuner-hero-sub">
          集成酷狗概念版蝰蛇声学矩阵、10 段参量均衡器 (Parametric EQ) 与三维空间声场工作站 · 后续还将持续扩展专业调音模块
        </p>
      </div>
      <div class="tuner-hero-right">
        <button
          class="frameless-hover-btn"
          @click="goToRadioPlaza"
        >
          <PlumpIcon name="radio" :size="16" />
          <span>前往发现页 · 电台广场</span>
        </button>
        <button
          class="frameless-hover-btn primary-tint"
          @click="resetAllTuning"
        >
          <PlumpIcon name="refresh" :size="16" />
          <span>恢复直通母带原音</span>
        </button>
      </div>
    </section>

    <!-- 1. 蝰蛇声学矩阵 (Viper Acoustic Matrix) -->
    <section class="discover-block">
      <div class="block-header">
        <div class="block-title-with-tabs">
          <h3 class="block-title">蝰蛇声学矩阵</h3>
          <span class="block-sub-date">/ 概念版官方声学渲染预设</span>
        </div>
      </div>

      <div class="tuner-viper-grid">
        <div
          v-for="vf in store.viperEffects"
          :key="vf.id"
          class="tuner-viper-card"
          :class="{ active: store.activeViperEffectId === vf.id }"
          @click="store.applyViperEffect(vf.id)"
        >
          <div class="tv-top">
            <span class="tv-badge">{{ vf.badge }}</span>
            <span class="tv-status-dot" :class="{ on: store.activeViperEffectId === vf.id }"></span>
          </div>
          <h4 class="tv-name">{{ vf.name }}</h4>
          <p class="tv-desc">{{ vf.desc }}</p>
        </div>
      </div>
    </section>

    <!-- 2. 10 段参量均衡器 (10-Band Parametric EQ) + 空间声场控制 -->
    <section class="tuner-workbench-grid">
      <div class="tuner-eq-panel">
        <div class="tuner-panel-header">
          <div>
            <h3 class="block-title">10 段参量均衡器 (PEQ)</h3>
            <span class="block-sub-date">Q 值: {{ store.eqQFactor.toFixed(2) }} · 实时双二阶滤波器链路</span>
          </div>
          <div class="tuner-preset-pills">
            <button
              v-for="p in eqPresets"
              :key="p.id"
              class="frameless-hover-btn"
              :class="{ 'primary-tint': store.activeEqPreset === p.id }"
              @click="store.setEqPreset(p.id)"
            >
              <span>{{ p.label }}</span>
            </button>
          </div>
        </div>

        <div class="tuner-eq-sliders">
          <div
            v-for="(freq, idx) in store.eqFrequencies"
            :key="freq"
            class="tuner-eq-column"
          >
            <span class="eq-gain-val">
              {{ store.eqGains[idx] > 0 ? `+${store.eqGains[idx].toFixed(1)}` : store.eqGains[idx].toFixed(1) }}dB
            </span>
            <input
              type="range"
              min="-12"
              max="12"
              step="0.5"
              :value="store.eqGains[idx]"
              class="tuner-vertical-slider"
              @input="onEqBandInput(idx, $event)"
            />
            <span class="eq-freq-label">{{ freq >= 1000 ? `${freq / 1000}k` : freq }}Hz</span>
          </div>
        </div>
      </div>

      <!-- 右侧：三维空间声场与倍速变调舱 -->
      <div class="tuner-spatial-panel">
        <div class="tuner-panel-header">
          <div>
            <h3 class="block-title">空间声场与倍速变调</h3>
            <span class="block-sub-date">环绕宽度 · 低频下潜 · 播放速率</span>
          </div>
        </div>

        <div class="tuner-knob-rows">
          <div class="tuner-slider-row">
            <div class="tsr-label">
              <span>三维声场宽度 (Stereo Width)</span>
              <strong>{{ Math.round(store.acousticField.stereoWidth * 100) }}%</strong>
            </div>
            <input
              v-model.number="store.acousticField.stereoWidth"
              type="range"
              min="0"
              max="2"
              step="0.05"
              class="pb-volume-slider full"
              @input="enableAcousticAndSync"
            />
          </div>

          <div class="tuner-slider-row">
            <div class="tsr-label">
              <span>空间混响纵深 (Reverb Mix)</span>
              <strong>{{ Math.round(store.acousticField.reverbMix * 100) }}%</strong>
            </div>
            <input
              v-model.number="store.acousticField.reverbMix"
              type="range"
              min="0"
              max="1"
              step="0.02"
              class="pb-volume-slider full"
              @input="enableAcousticAndSync"
            />
          </div>

          <div class="tuner-slider-row">
            <div class="tsr-label">
              <span>超重低音谐波激励 (Sub-Bass Punch)</span>
              <strong>+{{ store.acousticField.bassBoostDb.toFixed(1) }} dB</strong>
            </div>
            <input
              v-model.number="store.acousticField.bassBoostDb"
              type="range"
              min="0"
              max="12"
              step="0.5"
              class="pb-volume-slider full"
              @input="enableAcousticAndSync"
            />
          </div>

          <div class="tuner-slider-row">
            <div class="tsr-label">
              <span>无损变速速率 (Playback Rate)</span>
              <strong>{{ store.playbackRate.toFixed(2) }}x</strong>
            </div>
            <input
              :value="store.playbackRate"
              type="range"
              min="0.5"
              max="2.0"
              step="0.05"
              class="pb-volume-slider full"
              @input="onRateChange"
            />
          </div>

          <div class="tuner-slider-row">
            <div class="tsr-label">
              <span>半音升降调 (Pitch Semitones)</span>
              <strong>{{ store.pitchSemitones > 0 ? `+${store.pitchSemitones}` : store.pitchSemitones }} st</strong>
            </div>
            <input
              :value="store.pitchSemitones"
              type="range"
              min="-6"
              max="6"
              step="1"
              class="pb-volume-slider full"
              @input="onPitchChange"
            />
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const eqPresets = [
  { id: "flat", label: "平直原音" },
  { id: "pop", label: "清澈流行" },
  { id: "rock", label: "现场摇滚" },
  { id: "classical", label: "古典交响" },
  { id: "jazz", label: "微醺爵士" },
  { id: "vocal", label: "人声特写" },
  { id: "bass", label: "深海低音" },
];

function onEqBandInput(idx: number, e: Event) {
  const val = parseFloat((e.target as HTMLInputElement).value);
  store.updateSingleEqBand(idx, val);
}

function enableAcousticAndSync() {
  store.acousticField.enabled = true;
  store.syncDspChain();
}

function onRateChange(e: Event) {
  const val = parseFloat((e.target as HTMLInputElement).value);
  store.setPlaybackSpeed(val);
}

function onPitchChange(e: Event) {
  const val = parseInt((e.target as HTMLInputElement).value, 10);
  store.setPitchSemitones(val);
}

function resetAllTuning() {
  store.applyViperEffect("off");
  store.setEqPreset("flat");
  store.acousticField.enabled = false;
  store.acousticField.stereoWidth = 1.0;
  store.acousticField.reverbMix = 0;
  store.acousticField.bassBoostDb = 0;
  store.setPlaybackSpeed(1.0);
  store.setPitchSemitones(0);
  store.syncDspChain();
  store.notify("已恢复 64-bit 母带直通原音模式");
}

function goToRadioPlaza() {
  store.activeTab = "discover";
  store.discoverSubView = "radio";
}
</script>
