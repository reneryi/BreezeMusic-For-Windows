<script setup lang="ts">
import { computed, ref, watch, nextTick } from "vue";
import { useAppStore } from "../stores/appStore";
import type {
  LyricAlignment,
  LyricThemeId,
  PlayerBackgroundStyle,
  PlayerStageStyle,
} from "../types";
import {
  Languages,
  Disc3,
  Sparkles,
  Sliders,
  Type,
  Palette,
  X,
} from "lucide-vue-next";

const store = useAppStore();
const lyricListRef = ref<HTMLElement | null>(null);

// 播放页内部专属设置面板开关（严格遵守设计要求：播放页设置与歌词设置放在播放页内部）
const activeOverlayPanel = ref<"none" | "stage_bg" | "lyric_cfg" | "dsp_eq">(
  "none"
);

const stageStyles: { id: PlayerStageStyle; label: string; desc: string }[] = [
  {
    id: "vinyl_turntable",
    label: "流光黑胶唱盘",
    desc: "S型金属唱臂 + 实时声波律动外环",
  },
  {
    id: "retro_cassette",
    label: "复古磁带与电平",
    desc: "机械双轮磁带窗 + 暖光 VU 电平表",
  },
  {
    id: "cyber_spectrum",
    label: "赛博频谱舞台",
    desc: "32 段对称动态频谱 + 悬浮封面",
  },
  {
    id: "gallery_poster",
    label: "极简画廊方封",
    desc: "高定圆角大画幅专辑封面",
  },
];

const bgStyles: { id: PlayerBackgroundStyle; label: string }[] = [
  { id: "fluid_aurora", label: "流体极光" },
  { id: "rhythm_halo", label: "律动光晕" },
  { id: "cover_blur", label: "封面虚化" },
  { id: "starlight_nebula", label: "暗夜星尘" },
  { id: "deep_oled", label: "深邃纯黑" },
];

// 对齐 Android 端 2x3 双行三列六大四字主题名
const lyricThemes: {
  id: LyricThemeId;
  label: string;
  c1: string;
  c2: string;
}[] = [
  { id: "ice_blue", label: "冰晶青蓝", c1: "#00e5ff", c2: "#2b8cff" },
  { id: "aurora_purple", label: "极光幻紫", c1: "#b388ff", c2: "#6c5ce7" },
  { id: "amber_gold", label: "鎏金琥珀", c1: "#ffd54f", c2: "#f59e0b" },
  { id: "neon_coral", label: "霓虹珊瑚", c1: "#ff6b8b", c2: "#ff6b5b" },
  { id: "emerald_mint", label: "翡翠薄荷", c1: "#00e5a3", c2: "#00b894" },
  { id: "dynamic_cover", label: "封面随动", c1: "#3898ff", c2: "#00c4a7" },
];

const activeLyricColors = computed(() => {
  if (store.lyricTheme === "dynamic_cover") {
    return {
      c1: store.coverPalette[0],
      c2: store.coverPalette[1],
    };
  }
  const found = lyricThemes.find((t) => t.id === store.lyricTheme);
  return found
    ? { c1: found.c1, c2: found.c2 }
    : { c1: "#00e5ff", c2: "#2b8cff" };
});

// 计算当前播放毫秒对应的歌词行索引
const activeLineIndex = computed(() => {
  const lines = store.currentLyric.lines;
  if (!lines.length) return -1;
  const pos = store.currentPositionMs;
  let candidate = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].startTimeMs <= pos) {
      candidate = i;
    } else {
      break;
    }
  }
  return candidate;
});

// 当歌词行变化时，自动平滑滚动到舞台视觉中心
watch(activeLineIndex, async (idx) => {
  if (idx < 0 || !lyricListRef.value) return;
  await nextTick();
  const container = lyricListRef.value;
  const lineEl = container.querySelector(
    `[data-line-idx="${idx}"]`
  ) as HTMLElement | null;
  if (lineEl) {
    const targetTop =
      lineEl.offsetTop -
      container.clientHeight / 2 +
      lineEl.clientHeight / 2;
    container.scrollTo({
      top: Math.max(0, targetTop),
      behavior: "smooth",
    });
  }
});

// 计算距离当前激活行的行数差（用于连续光学景深高斯模糊）
function lineBlurPx(idx: number): number {
  if (!store.lyricBlurEnabled || activeLineIndex.value < 0) return 0;
  const dist = Math.abs(idx - activeLineIndex.value);
  if (dist === 0) return 0;
  if (dist === 1) return 0.6;
  return Math.min(4.2, (dist - 1) * 1.1);
}

// 计算 KRC 逐字高亮进度 (0 ~ 100%)
function wordProgressPercent(
  wordStartMs: number,
  wordDurationMs: number
): number {
  const pos = store.currentPositionMs;
  if (pos <= wordStartMs) return 0;
  if (wordDurationMs <= 0 || pos >= wordStartMs + wordDurationMs) return 100;
  return Math.min(100, ((pos - wordStartMs) / wordDurationMs) * 100);
}

function togglePanel(p: "stage_bg" | "lyric_cfg" | "dsp_eq") {
  activeOverlayPanel.value = activeOverlayPanel.value === p ? "none" : p;
}
</script>

<template>
  <section
    class="immersive-stage"
    :class="[`bg-mode-${store.playerBgStyle}`]"
    :style="{
      '--cover-c1': store.coverPalette[0],
      '--cover-c2': store.coverPalette[1],
      '--cover-c3': store.coverPalette[2],
      '--lyric-c1': activeLyricColors.c1,
      '--lyric-c2': activeLyricColors.c2,
      '--lyric-size': `${store.lyricFontSize}px`,
    }"
  >
    <!-- 1. 五款高定动态背景渲染层 -->
    <div
      v-if="store.playerBgStyle === 'cover_blur'"
      class="bg-layer-cover-blur"
      :style="
        store.currentSong?.coverUrl
          ? { backgroundImage: `url(${store.currentSong.coverUrl})` }
          : {}
      "
    ></div>

    <div
      v-else-if="store.playerBgStyle === 'fluid_aurora'"
      class="bg-layer-fluid-aurora"
      :class="{ paused: !store.isPlaying }"
    >
      <div class="aurora-blob blob-1"></div>
      <div class="aurora-blob blob-2"></div>
      <div class="aurora-blob blob-3"></div>
    </div>

    <div
      v-else-if="store.playerBgStyle === 'rhythm_halo'"
      class="bg-layer-rhythm-halo"
    >
      <div
        class="halo-ring ring-1"
        :style="{
          transform: `scale(${1 + (store.spectrumBars[2] || 10) / 140})`,
        }"
      ></div>
      <div
        class="halo-ring ring-2"
        :style="{
          transform: `scale(${1 + (store.spectrumBars[8] || 10) / 120})`,
        }"
      ></div>
      <div
        class="halo-ring ring-3"
        :style="{
          transform: `scale(${1 + (store.spectrumBars[16] || 10) / 110})`,
        }"
      ></div>
    </div>

    <div
      v-else-if="store.playerBgStyle === 'starlight_nebula'"
      class="bg-layer-starlight"
    >
      <div class="nebula-cloud c1"></div>
      <div class="nebula-cloud c2"></div>
    </div>

    <!-- 2. 左封面舞台 · 右逐字歌词双栏主体 -->
    <div class="stage-main-grid">
      <!-- ==================== 左栏：四款可切换视觉舞台 ==================== -->
      <div class="stage-left-zone">
        <!-- 样式 1：流光黑胶唱盘 (Vinyl Turntable + S-Tonearm + Audio Halo) -->
        <div
          v-if="store.playerStageStyle === 'vinyl_turntable'"
          class="turntable-assembly"
        >
          <!-- 实时声波呼吸外光圈 -->
          <div
            class="turntable-audio-halo"
            :style="{
              transform: `scale(${1 + (store.spectrumBars[1] || 10) / 260})`,
            }"
          ></div>

          <!-- S 型金属唱臂 -->
          <div
            class="vinyl-tonearm"
            :class="{ engaged: store.isPlaying }"
          ></div>

          <!-- 拟真黑胶纹路转盘 -->
          <div class="vinyl-platter" :class="{ spinning: store.isPlaying }">
            <div class="vinyl-grooves"></div>
            <div class="vinyl-label-art">
              <img
                v-if="store.currentSong?.coverUrl"
                :src="store.currentSong.coverUrl"
                class="vinyl-center-img"
              />
              <Disc3 v-else :size="48" />
            </div>
            <div class="vinyl-spindle"></div>
          </div>
        </div>

        <!-- 样式 2：复古磁带机与双 VU 电平表 (Retro Cassette & VU Meter) -->
        <div
          v-else-if="store.playerStageStyle === 'retro_cassette'"
          class="cassette-deck-box"
        >
          <div class="cassette-shell">
            <div class="cassette-label">
              <div class="cassette-title">
                {{ store.currentSong?.title || "BREEZE CONCEPT TAPE" }}
              </div>
              <div class="cassette-window">
                <div
                  class="tape-spool"
                  :class="{ spinning: store.isPlaying }"
                ></div>
                <div class="tape-bridge">
                  <span>TYPE IV · METAL MASTER</span>
                </div>
                <div
                  class="tape-spool"
                  :class="{ spinning: store.isPlaying }"
                ></div>
              </div>
            </div>
          </div>

          <!-- 实时跳动电平条 -->
          <div class="vu-meter-strip">
            <div
              v-for="(val, i) in store.spectrumBars.slice(0, 20)"
              :key="i"
              class="vu-bar"
              :style="{ height: `${Math.max(12, val * 0.55)}px` }"
            ></div>
          </div>
        </div>

        <!-- 样式 3：赛博霓虹频谱舞台 (Cyber Spectrum Visualizer) -->
        <div
          v-else-if="store.playerStageStyle === 'cyber_spectrum'"
          class="cyber-stage-box"
        >
          <div class="cyber-cover-hex">
            <img
              v-if="store.currentSong?.coverUrl"
              :src="store.currentSong.coverUrl"
              class="cyber-cover-img"
            />
          </div>
          <div class="cyber-spectrum-bars">
            <div
              v-for="(val, i) in store.spectrumBars"
              :key="i"
              class="cyber-bar"
              :style="{ height: `${Math.max(8, val * 0.9)}px` }"
            ></div>
          </div>
        </div>

        <!-- 样式 4：极简画廊方封面 (Gallery Poster) -->
        <div v-else class="gallery-square-card" :class="{ playing: store.isPlaying }">
          <img
            v-if="store.currentSong?.coverUrl"
            :src="store.currentSong.coverUrl"
            class="gallery-square-img"
          />
          <div v-else class="gallery-square-placeholder">
            <Disc3 :size="68" />
          </div>
        </div>

        <!-- 左下方歌曲标题、歌手与真实采样率/位深/码率胶囊 -->
        <div class="stage-track-meta">
          <h1 class="stage-main-song-title">
            {{ store.currentSong?.title || "暂未播放歌曲" }}
          </h1>
          <p class="stage-main-song-artist">
            {{ store.currentSong?.artist || "微风音乐 · 帧享真无损" }}
            <span v-if="store.currentSong?.album" class="stage-album-dot">
              · {{ store.currentSong.album }}
            </span>
          </p>

          <!-- 硬件级 STREAMINFO 真实音质规格胶囊 -->
          <div v-if="store.currentPlaybackInfo" class="stage-spec-badges">
            <span class="spec-pill gold">
              <Sparkles :size="12" />
              {{ store.currentPlaybackInfo.qualityLabel }}
            </span>
            <span class="spec-pill">
              {{ store.currentPlaybackInfo.techSpecText }}
            </span>
          </div>
        </div>

        <!-- 播放页专属内嵌工具坞（对齐 Android 演示视频 00:08 封面下方 4 枚精致胶囊按钮） -->
        <div class="stage-internal-dock">
          <button
            class="stage-tool-pill"
            :class="{ active: activeOverlayPanel === 'stage_bg' }"
            title="播放页舞台与背景设置"
            @click="togglePanel('stage_bg')"
          >
            <Palette :size="15" />
            <span>舞台背景</span>
          </button>

          <button
            class="stage-tool-pill"
            :class="{ active: activeOverlayPanel === 'lyric_cfg' }"
            title="歌词显示设置"
            @click="togglePanel('lyric_cfg')"
          >
            <Type :size="15" />
            <span>歌词设置</span>
          </button>

          <button
            class="stage-tool-pill"
            :class="{ active: activeOverlayPanel === 'dsp_eq' }"
            title="10 段母带参量均衡器与音效"
            @click="togglePanel('dsp_eq')"
          >
            <Sliders :size="15" />
            <span>母带音效</span>
          </button>

          <button
            v-if="
              store.currentLyric.hasTranslation ||
              store.currentLyric.hasPhonetic
            "
            class="stage-tool-pill"
            :class="{ active: store.showTranslation }"
            title="切换双语翻译/音译"
            @click="store.showTranslation = !store.showTranslation"
          >
            <Languages :size="15" />
            <span>译/音</span>
          </button>
        </div>
      </div>

      <!-- ==================== 右栏：无边框边缘渐隐 KRC 逐字流光歌词舞台 ==================== -->
      <div class="stage-right-lyric-zone">
        <div
          ref="lyricListRef"
          class="amll-lyric-scroll"
          :class="[`align-${store.lyricAlignment}`]"
        >
          <div v-if="store.isLoadingLyric" class="lyric-status-placeholder">
            正在解密 KRC 逐字流光歌词...
          </div>

          <div
            v-else-if="store.currentLyric.lines.length === 0"
            class="lyric-status-placeholder"
          >
            纯音乐或暂未匹配到歌词，请欣赏
          </div>

          <template v-else>
            <div class="lyric-top-bottom-pad"></div>
            <div
              v-for="(line, idx) in store.currentLyric.lines"
              :key="idx"
              :data-line-idx="idx"
              class="amll-lyric-line"
              :class="{
                active: idx === activeLineIndex,
                passed: idx < activeLineIndex,
              }"
              :style="{
                filter:
                  lineBlurPx(idx) > 0 ? `blur(${lineBlurPx(idx)}px)` : 'none',
              }"
              @click="store.seekToMs(line.startTimeMs)"
            >
              <!-- KRC 逐字卡拉 OK 流光扫光 + 逐字微上浮 -->
              <div
                v-if="
                  store.lyricWordByWord &&
                  idx === activeLineIndex &&
                  line.words.length > 0
                "
                class="amll-krc-row"
              >
                <span
                  v-for="(w, wIdx) in line.words"
                  :key="wIdx"
                  class="amll-krc-word"
                  :class="{
                    sung: wordProgressPercent(w.startTimeMs, w.durationMs) > 5,
                  }"
                  :style="{
                    '--w-pct': `${wordProgressPercent(
                      w.startTimeMs,
                      w.durationMs
                    )}%`,
                  }"
                >
                  {{ w.text }}
                </span>
              </div>

              <!-- 普通行文本 -->
              <div v-else class="amll-plain-row">
                {{ line.text }}
              </div>

              <!-- 译文或罗马音 -->
              <div
                v-if="
                  store.showTranslation &&
                  (line.translation || line.phonetic)
                "
                class="amll-trans-row"
              >
                {{ line.translation || line.phonetic }}
              </div>
            </div>
            <div class="lyric-top-bottom-pad"></div>
          </template>
        </div>
      </div>
    </div>

    <!-- ==================== 3. 播放页内部悬浮设置面板（舞台背景 / 歌词设置 / 10段PEQ） ==================== -->
    <transition name="pop-up">
      <div
        v-if="activeOverlayPanel !== 'none'"
        class="stage-floating-drawer"
      >
        <div class="drawer-head">
          <h4>
            {{
              activeOverlayPanel === "stage_bg"
                ? "播放页舞台与动态背景定制"
                : activeOverlayPanel === "lyric_cfg"
                ? "歌词显示设置（对齐 Android 2×3 主题网格）"
                : "母带级 10 段参量均衡器 (PEQ) 与目标曲线"
            }}
          </h4>
          <button class="icon-btn" @click="activeOverlayPanel = 'none'">
            <X :size="16" />
          </button>
        </div>

        <!-- 面板 A：舞台样式与 5 款高定动态背景 -->
        <div v-if="activeOverlayPanel === 'stage_bg'" class="drawer-body">
          <div class="cfg-label">左侧视觉舞台形态（4 款）</div>
          <div class="stage-style-grid">
            <button
              v-for="st in stageStyles"
              :key="st.id"
              class="stage-opt-card"
              :class="{ active: store.playerStageStyle === st.id }"
              @click="store.playerStageStyle = st.id"
            >
              <div class="st-title">{{ st.label }}</div>
              <div class="st-desc">{{ st.desc }}</div>
            </button>
          </div>

          <div class="cfg-label">动态光学背景引擎（5 款）</div>
          <div class="bg-pill-row">
            <button
              v-for="bg in bgStyles"
              :key="bg.id"
              class="bg-opt-pill"
              :class="{ active: store.playerBgStyle === bg.id }"
              @click="store.playerBgStyle = bg.id"
            >
              {{ bg.label }}
            </button>
          </div>
        </div>

        <!-- 面板 B：歌词显示设置（2x3 六大主题 + 字号 + 对齐 + 景深虚化） -->
        <div v-else-if="activeOverlayPanel === 'lyric_cfg'" class="drawer-body">
          <div class="cfg-label">流光高亮配色（2×3 主题网格）</div>
          <div class="lyric-theme-2x3-grid">
            <button
              v-for="th in lyricThemes"
              :key="th.id"
              class="lyric-theme-chip"
              :class="{ active: store.lyricTheme === th.id }"
              @click="store.lyricTheme = th.id"
            >
              <span
                class="theme-swatch"
                :style="{
                  background: `linear-gradient(135deg, ${th.c1}, ${th.c2})`,
                }"
              ></span>
              <span>{{ th.label }}</span>
            </button>
          </div>

          <div class="lyric-controls-row">
            <div class="lyric-ctrl-col">
              <span class="cfg-label">
                歌词字号 ({{ store.lyricFontSize }}px)
              </span>
              <input
                type="range"
                min="16"
                max="32"
                step="1"
                :value="store.lyricFontSize"
                @input="
                  store.lyricFontSize = Number(
                    ($event.target as HTMLInputElement).value
                  )
                "
              />
            </div>

            <div class="lyric-ctrl-col">
              <span class="cfg-label">对齐方式</span>
              <div class="align-btn-group">
                <button
                  v-for="al in [
                    { id: 'left', label: '居左' },
                    { id: 'center', label: '居中' },
                    { id: 'right', label: '居右' },
                  ]"
                  :key="al.id"
                  class="mini-opt-btn"
                  :class="{ active: store.lyricAlignment === al.id }"
                  @click="store.lyricAlignment = al.id as LyricAlignment"
                >
                  {{ al.label }}
                </button>
              </div>
            </div>

            <div class="lyric-ctrl-col">
              <span class="cfg-label">光学景深虚化</span>
              <button
                class="mini-opt-btn"
                :class="{ active: store.lyricBlurEnabled }"
                @click="store.lyricBlurEnabled = !store.lyricBlurEnabled"
              >
                {{ store.lyricBlurEnabled ? "已开启景深模糊" : "已关闭" }}
              </button>
            </div>
          </div>
        </div>

        <!-- 面板 C：10 段参量均衡器与 5 大发烧目标参考曲线 -->
        <div v-else-if="activeOverlayPanel === 'dsp_eq'" class="drawer-body">
          <div class="cfg-label">发烧参考目标曲线预设</div>
          <div class="bg-pill-row">
            <button
              v-for="pr in [
                { id: 'flat', label: '监听平直 (0dB)' },
                { id: 'harman', label: '哈曼曲线 (Harman)' },
                { id: 'vocal_velvet', label: '人声丝绒' },
                { id: 'hifi_bass', label: 'HiFi 深潜低频' },
                { id: 'diffuse_field', label: '弥散场参考' },
              ]"
              :key="pr.id"
              class="bg-opt-pill"
              :class="{ active: store.activeEqPreset === pr.id }"
              @click="store.applyEqPreset(pr.id)"
            >
              {{ pr.label }}
            </button>
          </div>

          <div class="eq-sliders-grid">
            <div
              v-for="(freq, fIdx) in store.eqFrequencies"
              :key="freq"
              class="eq-band-col"
            >
              <span class="eq-db-val">
                {{ store.eqGains[fIdx] > 0 ? `+${store.eqGains[fIdx]}` : store.eqGains[fIdx] }}dB
              </span>
              <input
                type="range"
                min="-10"
                max="10"
                step="0.5"
                :value="store.eqGains[fIdx]"
                class="eq-vert-slider"
                @input="
                  store.eqGains[fIdx] = Number(
                    ($event.target as HTMLInputElement).value
                  )
                "
              />
              <span class="eq-freq-label">
                {{ freq >= 1000 ? `${freq / 1000}k` : freq }}Hz
              </span>
            </div>
          </div>
        </div>
      </div>
    </transition>
  </section>
</template>
