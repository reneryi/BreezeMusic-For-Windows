<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

const selectedLineIndices = ref<number[]>([0, 1, 2]);
const activeTemplate = ref<"aurora" | "vinyl" | "daylight" | "obsidian">(
  "aurora"
);

const templates = [
  { id: "aurora", name: "极光流彩", bg: "linear-gradient(145deg, #0d1b3a, #103c52, #1b1438)" },
  { id: "vinyl", name: "黑胶留声", bg: "linear-gradient(145deg, #18141c, #2d1f2d, #12131a)" },
  { id: "daylight", name: "水彩白昼", bg: "linear-gradient(145deg, #eef5ff, #dcebfa, #f5f0ff)" },
  { id: "obsidian", name: "深空曜石", bg: "linear-gradient(145deg, #05070b, #0f1522, #070a10)" },
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

function export2KPosterPng() {
  const canvas = document.createElement("canvas");
  canvas.width = 1440;
  canvas.height = 2160;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const isLight = activeTemplate.value === "daylight";
  const grad = ctx.createLinearGradient(0, 0, 1440, 2160);
  if (activeTemplate.value === "aurora") {
    grad.addColorStop(0, "#0d1b3a");
    grad.addColorStop(0.5, "#0f384c");
    grad.addColorStop(1, "#161233");
  } else if (activeTemplate.value === "vinyl") {
    grad.addColorStop(0, "#1c1524");
    grad.addColorStop(1, "#0d0f17");
  } else if (isLight) {
    grad.addColorStop(0, "#f0f6ff");
    grad.addColorStop(1, "#e2edfc");
  } else {
    grad.addColorStop(0, "#05070c");
    grad.addColorStop(1, "#101726");
  }
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, 1440, 2160);

  // 装饰柔光圆
  ctx.fillStyle = isLight ? "rgba(43,140,255,0.12)" : "rgba(0,229,255,0.12)";
  ctx.beginPath();
  ctx.arc(1180, 360, 420, 0, Math.PI * 2);
  ctx.fill();

  // 标题与歌手
  ctx.fillStyle = isLight ? "#0f1d33" : "#f8fafc";
  ctx.font = "bold 64px 'Segoe UI', 'PingFang SC', sans-serif";
  ctx.fillText(store.currentSong?.title || "微风音乐", 140, 260);

  ctx.fillStyle = isLight ? "#4b5e78" : "#94a3b8";
  ctx.font = "500 38px 'Segoe UI', 'PingFang SC', sans-serif";
  ctx.fillText(store.currentSong?.artist || "BreezeMusic", 140, 335);

  // 歌词正文
  let y = 560;
  for (const line of selectedLyrics.value) {
    ctx.fillStyle = isLight ? "#0f1d33" : "#ffffff";
    ctx.font = "bold 56px 'Segoe UI', 'PingFang SC', sans-serif";
    ctx.fillText(line.text, 140, y);
    y += 78;
    if (line.translation) {
      ctx.fillStyle = isLight ? "#5a6e8c" : "rgba(255,255,255,0.68)";
      ctx.font = "500 36px 'Segoe UI', 'PingFang SC', sans-serif";
      ctx.fillText(line.translation, 140, y);
      y += 66;
    }
    y += 42;
  }

  // 底部品牌印记
  ctx.fillStyle = isLight ? "#64748b" : "rgba(255,255,255,0.5)";
  ctx.font = "600 30px 'Segoe UI', 'PingFang SC', sans-serif";
  ctx.fillText("BreezeMusic · 微风音乐 (酷狗概念版 2K 歌词海报)", 140, 1980);

  const dataUrl = canvas.toDataURL("image/png");
  const a = document.createElement("a");
  a.href = dataUrl;
  a.download = `BreezeMusic_${store.currentSong?.title || "Poster"}_2K.png`;
  a.click();
  store.notify("已生成并导出 2K (1440×2160) 超清歌词海报 PNG！");
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
                勾选心仪歌词段落 · 切换高定视觉模板并一键导出 2K 分辨率 PNG
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
              :class="[`tpl-${activeTemplate}`]"
              :style="{
                background: templates.find((t) => t.id === activeTemplate)?.bg,
              }"
            >
              <div class="pcc-header">
                <img
                  v-if="store.currentSong?.coverUrl"
                  :src="store.currentSong.coverUrl"
                  class="pcc-cover"
                />
                <div class="pcc-meta">
                  <div class="pcc-title">
                    {{ store.currentSong?.title || "微风音乐" }}
                  </div>
                  <div class="pcc-artist">
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
                  <div v-if="line.translation" class="pcc-line-trans">
                    {{ line.translation }}
                  </div>
                </div>
              </div>

              <div class="pcc-footer">
                <span>BreezeMusic · 帧享真无损</span>
                <span class="pcc-badge">2K UHD POSTER</span>
              </div>
            </div>
          </div>

          <!-- 右栏：模板切换 + 歌词多选列表 -->
          <div class="poster-controls-col">
            <div class="p-ctrl-block">
              <div class="p-ctrl-label">高定视觉排版模板</div>
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
