<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

const selectedLineIndices = ref<number[]>([0, 1, 2]);
const activeTemplate = ref<
  "vinyl" | "vanguard" | "oriental" | "polaroid" | "cyber"
>("vinyl");

// 严格对齐 Android 端五大高定 2K 海报艺术模板
const templates = [
  {
    id: "vinyl",
    name: "黑胶典藏",
    bg: "linear-gradient(145deg, #16121e, #24192b, #0e1017)",
    accent: "#ffd54f",
    isLight: false,
  },
  {
    id: "vanguard",
    name: "先锋画报",
    bg: "linear-gradient(145deg, #0b1b36, #0d3b4e, #161233)",
    accent: "#00e5ff",
    isLight: false,
  },
  {
    id: "oriental",
    name: "东方雅韵",
    bg: "linear-gradient(145deg, #f6f3eb, #ebe4d5, #e2d9c5)",
    accent: "#9e2a2b",
    isLight: true,
  },
  {
    id: "polaroid",
    name: "复古拍立得",
    bg: "linear-gradient(145deg, #eef5ff, #dcebfa, #f5f0ff)",
    accent: "#155eef",
    isLight: true,
  },
  {
    id: "cyber",
    name: "赛博流光",
    bg: "linear-gradient(145deg, #05070b, #140d2b, #071928)",
    accent: "#ff4d6d",
    isLight: false,
  },
] as const;

watch(
  () => store.isLyricPosterOpen,
  (open) => {
    if (open && store.currentLyric.lines.length > 0) {
      const mid = Math.max(0, Math.floor(store.currentLyric.lines.length / 3));
      selectedLineIndices.value = [mid, mid + 1, mid + 2].filter(
        (i) => i < store.currentLyric.lines.length
      );
    }
  }
);

function toggleLineSelection(idx: number) {
  const pos = selectedLineIndices.value.indexOf(idx);
  if (pos >= 0) {
    if (selectedLineIndices.value.length > 1) {
      selectedLineIndices.value.splice(pos, 1);
    }
  } else {
    if (selectedLineIndices.value.length >= 6) {
      store.notify("单张海报最多选择 6 行歌词以保持最佳排版美感");
      return;
    }
    selectedLineIndices.value.push(idx);
    selectedLineIndices.value.sort((a, b) => a - b);
  }
}

const selectedLyrics = computed(() => {
  if (store.currentLyric.lines.length === 0) {
    return [
      {
        text: "耳畔微风起 · 帧享真无损",
        translation: "BreezeMusic Concept Edition",
      },
    ];
  }
  return selectedLineIndices.value
    .map((i) => store.currentLyric.lines[i])
    .filter(Boolean);
});

function wrapCanvasText(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  startY: number,
  maxWidth: number,
  lineHeight: number
): number {
  const chars = Array.from(text || "");
  let line = "";
  let y = startY;
  for (let i = 0; i < chars.length; i++) {
    const testLine = line + chars[i];
    const metrics = ctx.measureText(testLine);
    if (metrics.width > maxWidth && line.length > 0) {
      ctx.fillText(line, x, y);
      line = chars[i];
      y += lineHeight;
    } else {
      line = testLine;
    }
  }
  if (line) {
    ctx.fillText(line, x, y);
    y += lineHeight;
  }
  return y;
}

function export2KPosterPng() {
  const canvas = document.createElement("canvas");
  canvas.width = 1440;
  canvas.height = 2160;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const tpl =
    templates.find((t) => t.id === activeTemplate.value) || templates[0];
  const isLight = tpl.isLight;
  const grad = ctx.createLinearGradient(0, 0, 1440, 2160);

  if (tpl.id === "vinyl") {
    grad.addColorStop(0, "#181322");
    grad.addColorStop(0.5, "#231829");
    grad.addColorStop(1, "#0b0d14");
  } else if (tpl.id === "vanguard") {
    grad.addColorStop(0, "#0b1b36");
    grad.addColorStop(0.5, "#0d3b4e");
    grad.addColorStop(1, "#161233");
  } else if (tpl.id === "oriental") {
    grad.addColorStop(0, "#f7f4ec");
    grad.addColorStop(1, "#e5dcc8");
  } else if (tpl.id === "polaroid") {
    grad.addColorStop(0, "#f0f6ff");
    grad.addColorStop(1, "#e2edfc");
  } else {
    grad.addColorStop(0, "#05070c");
    grad.addColorStop(0.5, "#150d2a");
    grad.addColorStop(1, "#071a29");
  }
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, 1440, 2160);

  // 装饰图形：根据模板绘制黑胶唱片纹、东方朱印或赛博网格光环
  if (tpl.id === "vinyl") {
    ctx.strokeStyle = "rgba(255, 213, 79, 0.14)";
    ctx.lineWidth = 4;
    for (let r = 180; r <= 420; r += 40) {
      ctx.beginPath();
      ctx.arc(1160, 340, r, 0, Math.PI * 2);
      ctx.stroke();
    }
  } else if (tpl.id === "oriental") {
    // 东方朱砂印章
    ctx.fillStyle = "#9e2a2b";
    ctx.fillRect(1180, 1840, 110, 110);
    ctx.fillStyle = "#f7f4ec";
    ctx.font = "bold 36px 'PingFang SC', serif";
    ctx.fillText("微风", 1198, 1890);
    ctx.fillText("雅韵", 1198, 1934);
  } else {
    ctx.fillStyle = isLight
      ? "rgba(21, 94, 239, 0.1)"
      : "rgba(0, 229, 255, 0.12)";
    ctx.beginPath();
    ctx.arc(1180, 360, 420, 0, Math.PI * 2);
    ctx.fill();
  }

  // 顶部横眉
  ctx.fillStyle = tpl.accent;
  ctx.font = "bold 28px 'Segoe UI', 'PingFang SC', sans-serif";
  ctx.fillText(`BREEZEMUSIC · ${tpl.name.toUpperCase()} EDITION`, 140, 175);

  // 标题与歌手（支持超长自动换行）
  ctx.fillStyle = isLight ? "#0f1d33" : "#f8fafc";
  ctx.font = "bold 64px 'Segoe UI', 'PingFang SC', sans-serif";
  const afterTitleY = wrapCanvasText(
    ctx,
    store.currentSong?.title || "微风音乐",
    140,
    265,
    1140,
    80
  );

  ctx.fillStyle = isLight ? "#4b5e78" : "#94a3b8";
  ctx.font = "600 38px 'Segoe UI', 'PingFang SC', sans-serif";
  ctx.fillText(
    `${store.currentSong?.artist || "BreezeMusic"}${
      store.currentSong?.album ? ` · 《${store.currentSong.album}》` : ""
    }`,
    140,
    afterTitleY + 10
  );

  // 分隔装饰线
  ctx.fillStyle = tpl.accent;
  ctx.fillRect(140, afterTitleY + 55, 96, 6);

  // 歌词正文（支持多行自动折行，绝不超出画布右侧）
  let y = afterTitleY + 150;
  for (const line of selectedLyrics.value) {
    ctx.fillStyle = isLight ? "#0f1d33" : "#ffffff";
    ctx.font =
      tpl.id === "oriental"
        ? "bold 54px 'Songti SC', 'SimSun', 'PingFang SC', serif"
        : "bold 54px 'Segoe UI', 'PingFang SC', sans-serif";
    y = wrapCanvasText(ctx, line.text, 140, y, 1140, 74);

    if (line.translation) {
      ctx.fillStyle = isLight ? "#5a6e8c" : "rgba(255,255,255,0.68)";
      ctx.font = "500 35px 'Segoe UI', 'PingFang SC', sans-serif";
      y = wrapCanvasText(ctx, line.translation, 140, y + 4, 1140, 52);
    }
    y += 44;
  }

  // 底部品牌印记
  ctx.fillStyle = isLight ? "#64748b" : "rgba(255,255,255,0.52)";
  ctx.font = "600 30px 'Segoe UI', 'PingFang SC', sans-serif";
  ctx.fillText(
    `BreezeMusic · 微风音乐 (${tpl.name} · 2K 超清歌词海报)`,
    140,
    1980
  );

  const dataUrl = canvas.toDataURL("image/png");
  const a = document.createElement("a");
  a.href = dataUrl;
  a.download = `BreezeMusic_${store.currentSong?.title || "Poster"}_${tpl.name}_2K.png`;
  a.click();
  store.notify(`已生成并导出「${tpl.name}」2K (1440×2160) 超清歌词海报 PNG！`);
}
</script>

<template>
  <transition name="fade">
    <div
      v-if="store.isLyricPosterOpen"
      class="modal-backdrop"
      @click.self="store.isLyricPosterOpen = false"
    >
      <div class="poster-modal-card">
        <div class="modal-header">
          <div class="vip-title-row">
            <div class="vip-icon-orb cyan">
              <PlumpIcon name="poster" :size="19" color="#00e5ff" />
            </div>
            <div>
              <h3>2K 超清歌词海报工坊</h3>
              <p class="vip-sub-caption">
                五大高定艺术模板（黑胶典藏 / 先锋画报 / 东方雅韵 / 复古拍立得 / 赛博流光）· 智能多行折行排版
              </p>
            </div>
          </div>
          <button
            class="icon-close-btn"
            @click="store.isLyricPosterOpen = false"
          >
            <PlumpIcon name="close" :size="16" />
          </button>
        </div>

        <div class="poster-workspace">
          <!-- 左栏：实时海报画布预览 -->
          <div class="poster-preview-col">
            <div
              class="poster-canvas-card"
              :class="[
                `tpl-${activeTemplate}`,
                {
                  'tpl-daylight':
                    activeTemplate === 'oriental' ||
                    activeTemplate === 'polaroid',
                },
              ]"
              :style="{
                background: templates.find((t) => t.id === activeTemplate)?.bg,
                color: templates.find((t) => t.id === activeTemplate)?.isLight
                  ? '#0f1d33'
                  : '#ffffff',
              }"
            >
              <div class="pcc-header">
                <img
                  v-if="store.currentSong?.coverUrl"
                  :src="store.currentSong.coverUrl"
                  class="pcc-cover"
                  :style="{
                    borderRadius:
                      activeTemplate === 'vinyl' ? '50%' : '14px',
                  }"
                />
                <div class="pcc-meta">
                  <div class="pcc-title">
                    {{ store.currentSong?.title || "微风音乐" }}
                  </div>
                  <div
                    class="pcc-artist"
                    :style="{
                      color: templates.find((t) => t.id === activeTemplate)
                        ?.isLight
                        ? '#475569'
                        : 'rgba(255,255,255,0.72)',
                    }"
                  >
                    {{ store.currentSong?.artist || "BreezeMusic" }}
                  </div>
                </div>
              </div>

              <div class="pcc-lyrics">
                <div
                  v-for="(line, idx) in selectedLyrics"
                  :key="idx"
                  class="pcc-lyric-block"
                >
                  <div class="pcc-line-main">{{ line.text }}</div>
                  <div
                    v-if="line.translation"
                    class="pcc-line-trans"
                    :style="{
                      color: templates.find((t) => t.id === activeTemplate)
                        ?.isLight
                        ? '#64748b'
                        : 'rgba(255,255,255,0.68)',
                    }"
                  >
                    {{ line.translation }}
                  </div>
                </div>
              </div>

              <div class="pcc-footer">
                <span>
                  BreezeMusic ·
                  {{ templates.find((t) => t.id === activeTemplate)?.name }}
                </span>
                <span class="pcc-badge">2K UHD POSTER</span>
              </div>
            </div>
          </div>

          <!-- 右栏：模板切换 + 歌词多选列表 -->
          <div class="poster-controls-col">
            <div class="p-ctrl-block">
              <div class="p-ctrl-label">五大高定视觉排版模板</div>
              <div class="p-tpl-pills">
                <button
                  v-for="t in templates"
                  :key="t.id"
                  class="p-tpl-btn"
                  :class="{ active: activeTemplate === t.id }"
                  @click="activeTemplate = t.id"
                >
                  {{ t.name }}
                </button>
              </div>
            </div>

            <div class="p-ctrl-block flex-fill">
              <div class="p-ctrl-label">
                点击勾选歌词行（已选 {{ selectedLineIndices.length }} / 6 行）
              </div>
              <div class="p-lyric-picker">
                <button
                  v-for="(l, idx) in store.currentLyric.lines"
                  :key="idx"
                  class="p-lyric-pick-row"
                  :class="{ selected: selectedLineIndices.includes(idx) }"
                  @click="toggleLineSelection(idx)"
                >
                  <PlumpIcon
                    name="check"
                    :size="15"
                    :accent="
                      selectedLineIndices.includes(idx)
                        ? '#00e5ff'
                        : 'rgba(255,255,255,0.2)'
                    "
                  />
                  <div class="p-l-texts">
                    <div class="p-l-main">{{ l.text }}</div>
                    <div v-if="l.translation" class="p-l-sub">
                      {{ l.translation }}
                    </div>
                  </div>
                </button>
              </div>
            </div>

            <button class="primary-btn full-w" @click="export2KPosterPng">
              <PlumpIcon name="download" :size="17" color="#ffffff" />
              <span>一键渲染并保存 2K 超清海报 (.PNG)</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>
