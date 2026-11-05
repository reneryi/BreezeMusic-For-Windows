<template>
  <div class="tuner-studio-page">
    <!-- 顶部母带声学状态横幅（无大胶囊框，嵌入背景层） -->
    <section class="tuner-hero-banner">
      <div class="tuner-hero-left">
        <div class="tuner-eyebrow">CONCEPT MASTERING DESK · 64-BIT LTI DSP ENGINE</div>
        <h2 class="tuner-hero-title">概念版声学调音台</h2>
        <p class="tuner-hero-sub">
          集成酷狗概念版蝰蛇声学矩阵、10 段双二阶参量均衡器 (Parametric EQ · 含 96 点对数频响曲线) 与三维空间声场工作站
        </p>
      </div>
      <div class="tuner-hero-right">
        <button
          class="frameless-hover-btn"
          :class="{ 'primary-tint': store.peqBypass }"
          @click="
            store.peqBypass = !store.peqBypass;
            store.syncDspChain();
          "
        >
          <PlumpIcon name="wave" :size="16" />
          <span>{{ store.peqBypass ? "原音直通对比中 (A/B)" : "A/B 原音对比" }}</span>
        </button>
        <button
          v-if="store.currentSong"
          class="frameless-hover-btn"
          title="返回全屏播放页查看实时声效表现"
          @click="store.isNowPlayingOpen = true"
        >
          <PlumpIcon name="disc" :size="16" />
          <span>返回播放舞台</span>
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

          <div class="bg-choice-pills" style="margin-left: 12px">
            <button
              v-for="cat in viperCategories"
              :key="cat"
              class="bg-pill-btn"
              :class="{ active: selectedViperCategory === cat }"
              @click="selectedViperCategory = cat"
            >
              {{ cat }}
            </button>
          </div>
        </div>

        <button
          v-if="store.activeViperEffectId !== 'off'"
          class="frameless-hover-btn"
          style="color: var(--coral)"
          @click="store.applyViperEffect(null)"
        >
          <span>关闭蝰蛇音效</span>
        </button>
      </div>

      <div class="tuner-viper-grid">
        <div
          v-for="vf in filteredViperEffects"
          :key="vf.id"
          class="tuner-viper-card"
          :class="{ active: store.activeViperEffectId === vf.id }"
          @click="store.applyViperEffect(vf)"
        >
          <div class="tv-top">
            <span class="tv-badge">{{ vf.usageBadge || vf.badge }}</span>
            <span class="tv-status-dot" :class="{ on: store.activeViperEffectId === vf.id }"></span>
          </div>
          <h4 class="tv-name">{{ vf.label || vf.name }}</h4>
          <p class="tv-desc">{{ vf.desc }}</p>
          <div style="display: flex; gap: 6px; flex-wrap: wrap; margin-top: 8px">
            <span
              v-for="tag in vf.acousticTags"
              :key="tag"
              class="mini-badge hires"
            >
              {{ tag }}
            </span>
          </div>
        </div>
      </div>
    </section>

    <!-- 2. 10 段参量均衡器 (10-Band Parametric EQ) + 空间声场控制 -->
    <section class="tuner-workbench-grid">
      <div class="tuner-eq-panel">
        <div class="tuner-panel-header">
          <div>
            <h3 class="block-title">10 段参量均衡器 (PEQ)</h3>
            <span class="block-sub-date">
              Q 值: {{ store.eqQFactor.toFixed(2) }} · 前级: {{ store.masterPreampDb > 0 ? `+${store.masterPreampDb}` : store.masterPreampDb }}dB · 实时双二阶滤波器链路
            </span>
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

        <!-- 实时 96 点对数频率响应包络画布 -->
        <div class="peq-biquad-curve-box">
          <svg viewBox="0 0 560 100" preserveAspectRatio="none" class="peq-curve-svg">
            <defs>
              <linearGradient id="radioPeqAreaGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="#00e5ff" stop-opacity="0.38" />
                <stop offset="50%" stop-color="#2b8cff" stop-opacity="0.08" />
                <stop offset="100%" stop-color="#6c5ce7" stop-opacity="0.32" />
              </linearGradient>
            </defs>
            <line x1="0" y1="20" x2="560" y2="20" stroke="rgba(148,163,184,0.14)" stroke-dasharray="3 3" />
            <line x1="0" y1="50" x2="560" y2="50" stroke="rgba(0,229,255,0.32)" stroke-width="1" />
            <line x1="0" y1="80" x2="560" y2="80" stroke="rgba(148,163,184,0.14)" stroke-dasharray="3 3" />
            <path :d="peqResponseCurveSvg.areaPath" fill="url(#radioPeqAreaGrad)" />
            <path :d="peqResponseCurveSvg.linePath" fill="none" stroke="var(--cyan)" stroke-width="2.4" stroke-linecap="round" />
          </svg>
          <div class="peq-curve-overlay-tags">
            <span>+12dB</span>
            <span>0dB · 20Hz ~ 20kHz Logarithmic Biquad</span>
            <span>-12dB</span>
          </div>
        </div>

        <!-- 三槽独立调音记忆槽 -->
        <div class="peq-slots-row">
          <div
            v-for="slot in store.peqMemorySlots"
            :key="slot.slotId"
            class="peq-slot-pill"
            :class="{ active: store.activeEqPreset === `slot_${slot.slotId}` }"
          >
            <button class="slot-load-btn" @click="store.loadPeqFromSlot(slot.slotId)">
              <span>记忆槽 {{ slot.slotId }}</span>
              <small>{{ slot.saved ? "已存" : "空置" }}</small>
            </button>
            <button class="slot-mini-act" @click="store.savePeqToSlot(slot.slotId)">存入</button>
            <button
              v-if="slot.saved"
              class="slot-mini-act danger"
              @click="store.resetSinglePeqSlot(slot.slotId)"
            >
              删
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
              step="0.1"
              :value="store.eqGains[idx]"
              class="tuner-vertical-slider"
              @input="onEqBandInput(idx, $event)"
            />
            <span class="eq-freq-label">{{ freq >= 1000 ? `${freq / 1000}k` : freq }}Hz</span>
          </div>
        </div>

        <div class="lyric-sliders-row">
          <div class="cfg-col">
            <div class="cfg-label">前级增益 (Master Preamp)：{{ store.masterPreampDb > 0 ? `+${store.masterPreampDb}` : store.masterPreampDb }} dB</div>
            <input
              v-model.number="store.masterPreampDb"
              type="range"
              min="-6"
              max="6"
              step="0.5"
              class="pb-volume-slider full"
              @input="store.syncDspChain()"
            />
          </div>
          <div class="cfg-col">
            <div class="cfg-label">品质因数 Q 值：{{ store.eqQFactor.toFixed(2) }}</div>
            <input
              v-model.number="store.eqQFactor"
              type="range"
              min="0.4"
              max="6.0"
              step="0.1"
              class="pb-volume-slider full"
              @input="store.syncDspChain()"
            />
          </div>
          <div class="cfg-col">
            <div class="cfg-label">高通滤波 (HPF)：{{ store.eqHighPassHz }} Hz</div>
            <input
              v-model.number="store.eqHighPassHz"
              type="range"
              min="15"
              max="180"
              step="5"
              class="pb-volume-slider full"
              @input="store.syncDspChain()"
            />
          </div>
          <div class="cfg-col">
            <div class="cfg-label">低通滤波 (LPF)：{{ store.eqLowPassHz }} Hz</div>
            <input
              v-model.number="store.eqLowPassHz"
              type="range"
              min="8000"
              max="20000"
              step="500"
              class="pb-volume-slider full"
              @input="store.syncDspChain()"
            />
          </div>
        </div>
      </div>

      <!-- 右侧：三维空间声场与倍速变调舱 -->
      <div class="tuner-spatial-panel">
        <div class="tuner-panel-header">
          <div>
            <h3 class="block-title">空间声场与倍速变调</h3>
            <span class="block-sub-date">环绕宽度 · 混响模型 · 低频下潜 · 变速升降调</span>
          </div>
          <label class="plump-switch sm">
            <input
              v-model="store.acousticField.enabled"
              type="checkbox"
              @change="store.syncDspChain()"
            />
            <span class="plump-slider"></span>
          </label>
        </div>

        <div class="bg-choice-pills">
          <button
            v-for="rv in reverbPresets"
            :key="rv.id"
            class="bg-pill-btn"
            :class="{ active: store.acousticField.reverbPreset === rv.id }"
            @click="
              store.acousticField.enabled = true;
              store.acousticField.reverbPreset = rv.id;
              store.syncDspChain();
            "
          >
            {{ rv.label }}
          </button>
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
              min="0.5"
              max="2"
              step="0.05"
              class="pb-volume-slider full"
              @input="enableAcousticAndSync"
            />
          </div>

          <div class="tuner-slider-row">
            <div class="tsr-label">
              <span>空间混响湿度与预延迟 ({{ store.acousticField.reverbWetPercent }}% / {{ store.acousticField.reverbPreDelayMs }}ms)</span>
              <label style="display: inline-flex; align-items: center; gap: 6px; font-size: 11px; cursor: pointer">
                <input
                  v-model="store.acousticField.bauerCrossfeed"
                  type="checkbox"
                  @change="enableAcousticAndSync"
                />
                <span>Bauer 双耳交叉馈送</span>
              </label>
            </div>
            <div style="display: flex; gap: 10px">
              <input
                v-model.number="store.acousticField.reverbWetPercent"
                type="range"
                min="0"
                max="100"
                step="1"
                class="pb-volume-slider full"
                @input="onReverbWetChange"
              />
              <input
                v-model.number="store.acousticField.reverbPreDelayMs"
                type="range"
                min="0"
                max="120"
                step="2"
                class="pb-volume-slider full"
                @input="enableAcousticAndSync"
              />
            </div>
          </div>

          <div class="tuner-slider-row">
            <div class="tsr-label">
              <span>超重低音下潜与谐波激励 ({{ store.acousticField.bassCrossoverHz }}Hz)</span>
              <strong>{{ store.acousticField.subBassDepth }}%</strong>
            </div>
            <input
              v-model.number="store.acousticField.subBassDepth"
              type="range"
              min="0"
              max="100"
              step="1"
              class="pb-volume-slider full"
              @input="onSubBassChange"
            />
          </div>

          <div class="tuner-slider-row">
            <div class="tsr-label">
              <span>人声结像通透度 &amp; 12kHz+ 空气泛音</span>
              <strong>{{ store.acousticField.vocalClarity }}% / {{ store.acousticField.airHarmonics }}%</strong>
            </div>
            <div style="display: flex; gap: 10px">
              <input
                v-model.number="store.acousticField.vocalClarity"
                type="range"
                min="0"
                max="100"
                step="1"
                class="pb-volume-slider full"
                @input="enableAcousticAndSync"
              />
              <input
                v-model.number="store.acousticField.airHarmonics"
                type="range"
                min="0"
                max="100"
                step="1"
                class="pb-volume-slider full"
                @input="enableAcousticAndSync"
              />
            </div>
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
import { computed, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import type { ReverbPresetId } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();
const selectedViperCategory = ref<string>("全部");
const viperCategories = ["全部", "官方旗舰", "3D空间", "复古介质", "曲风人声", "器乐发烧", "游戏影院"];

const filteredViperEffects = computed(() => {
  if (selectedViperCategory.value === "全部") return store.viperEffects;
  return store.viperEffects.filter(
    (v) => v.category === selectedViperCategory.value
  );
});

const eqPresets = [
  { id: "flat", label: "监听平直" },
  { id: "harman", label: "哈曼曲线" },
  { id: "diffuse_field", label: "弥散场" },
  { id: "vocal_velvet", label: "人声丝绒" },
  { id: "hifi_sub", label: "HiFi 低音" },
  { id: "acg_bright", label: "ACG 空气" },
  { id: "pop", label: "清澈流行" },
  { id: "rock", label: "现场摇滚" },
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

// 96 点对数频率响应曲线计算
const peqResponseCurveSvg = computed(() => {
  const width = 560;
  const height = 100;
  const midY = height / 2;
  const numPoints = 96;
  const logMin = Math.log10(20);
  const logMax = Math.log10(20000);

  const freqs = store.eqFrequencies;
  const gains = store.eqGains;
  const qBase = Math.max(0.4, store.eqQFactor || 1.41);
  const hpf = store.eqHighPassHz || 20;
  const lpf = store.eqLowPassHz || 20000;
  const bypass = store.peqBypass;

  const pts: Array<{ x: number; y: number }> = [];
  for (let i = 0; i < numPoints; i++) {
    const ratio = i / (numPoints - 1);
    const f = Math.pow(10, logMin + ratio * (logMax - logMin));
    let totalDb = 0;

    if (!bypass) {
      for (let b = 0; b < freqs.length; b++) {
        const fc = freqs[b];
        const g = gains[b] || 0;
        if (Math.abs(g) < 0.01) continue;
        const octDist = Math.log2(f / fc);
        const bw = 1.0 / qBase;
        const bell = Math.exp(-(octDist * octDist) / (2 * bw * bw));
        totalDb += g * bell;
      }
      if (f < hpf) {
        totalDb -= Math.min(14, Math.log2(hpf / f) * 9);
      }
      if (f > lpf) {
        totalDb -= Math.min(14, Math.log2(f / lpf) * 9);
      }
    }

    const clampedDb = Math.max(-13, Math.min(13, totalDb));
    const x = Number((ratio * width).toFixed(1));
    const y = Number((midY - (clampedDb / 13) * (midY - 8)).toFixed(1));
    pts.push({ x, y });
  }

  const linePath = pts
    .map((p, idx) => `${idx === 0 ? "M" : "L"}${p.x},${p.y}`)
    .join(" ");
  const areaPath = `${linePath} L${width},${midY} L0,${midY} Z`;
  return { linePath, areaPath };
});

function onEqBandInput(idx: number, e: Event) {
  const val = parseFloat((e.target as HTMLInputElement).value);
  store.updateSingleEqBand(idx, val);
}

function enableAcousticAndSync() {
  store.acousticField.enabled = true;
  store.syncDspChain();
}

function onReverbWetChange() {
  store.acousticField.enabled = true;
  store.acousticField.reverbMix = Number((store.acousticField.reverbWetPercent / 100).toFixed(2));
  if (store.acousticField.reverbWetPercent > 0 && store.acousticField.reverbPreset === "none") {
    store.acousticField.reverbPreset = "medium_hall";
  }
  store.syncDspChain();
}

function onSubBassChange() {
  store.acousticField.enabled = true;
  store.acousticField.bassBoostDb = Number(((store.acousticField.subBassDepth / 100) * 9).toFixed(1));
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
  store.applyViperEffect(null);
  store.resetAllPeqSlots();
  store.setPlaybackSpeed(1.0);
  store.setPitchSemitones(0);
  store.syncDspChain();
  store.notify("已恢复 64-bit 母带直通原音模式");
}


</script>
