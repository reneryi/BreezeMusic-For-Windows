<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed, ref, watch, nextTick } from "vue";
import { useAppStore } from "./stores/appStore";
import TitleBar from "./components/TitleBar.vue";
import PlayerBar from "./components/PlayerBar.vue";
import LoginModal from "./components/LoginModal.vue";
import FocusPodModal from "./components/FocusPodModal.vue";
import LyricPosterModal from "./components/LyricPosterModal.vue";
import SongContextMenu from "./components/SongContextMenu.vue";
import CommentsModal from "./components/CommentsModal.vue";
import MultiArtistModal from "./components/MultiArtistModal.vue";
import PlumpIcon from "./components/PlumpIcon.vue";
import DiscoverView from "./views/DiscoverView.vue";
import LibraryView from "./views/LibraryView.vue";
import RadioView from "./views/RadioView.vue";
import SearchPlaceholderView from "./views/SearchPlaceholderView.vue";
import SettingsPlaceholderView from "./views/SettingsPlaceholderView.vue";
import NowPlayingView from "./views/NowPlayingView.vue";
import PlaylistDetailView from "./views/PlaylistDetailView.vue";
import ArtistDetailView from "./views/ArtistDetailView.vue";
import AlbumDetailView from "./views/AlbumDetailView.vue";

const store = useAppStore();
const stageViewportRef = ref<HTMLElement | null>(null);
const desktopLyricRef = ref<HTMLElement | null>(null);
const isDraggingDesktopLyric = ref(false);
let dragOffsetX = 0;
let dragOffsetY = 0;

function onDesktopLyricMouseDown(e: MouseEvent) {
  if (store.desktopLyricLocked) return;
  const target = e.target as HTMLElement | null;
  if (target && (target.closest("button") || target.closest("input"))) return;
  const el = desktopLyricRef.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  isDraggingDesktopLyric.value = true;
  dragOffsetX = e.clientX - rect.left;
  dragOffsetY = e.clientY - rect.top;

  const onMove = (ev: MouseEvent) => {
    if (!isDraggingDesktopLyric.value) return;
    const width = store.desktopLyricWidth || rect.width || 760;
    const height = rect.height || 100;
    const maxX = Math.max(12, window.innerWidth - width - 12);
    const maxY = Math.max(12, window.innerHeight - height - 12);
    store.desktopLyricX = Math.min(maxX, Math.max(12, Math.round(ev.clientX - dragOffsetX)));
    store.desktopLyricY = Math.min(maxY, Math.max(12, Math.round(ev.clientY - dragOffsetY)));
  };

  const onUp = () => {
    isDraggingDesktopLyric.value = false;
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  };

  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}

const desktopLyricPositionStyle = computed(() => {
  const w = store.desktopLyricWidth || 760;
  const base: Record<string, string> = {
    "--fdl-color": store.desktopLyricColor || "#00e5ff",
    "--fdl-size": `${store.desktopLyricFontSize || 24}px`,
    width: `${w}px`,
    maxWidth: "calc(100vw - 24px)",
  };
  if (store.desktopLyricX !== null && store.desktopLyricY !== null) {
    base.left = `${store.desktopLyricX}px`;
    base.top = `${store.desktopLyricY}px`;
    base.bottom = "auto";
    base.transform = "none";
  }
  return base;
});


const calibratedPosMs = computed(() => {
  return (
    store.currentPositionMs + store.lyricOffsetMs - store.bluetoothDelayMs
  );
});

const currentLyricPair = computed(() => {
  const lines = store.currentLyric.lines;
  if (!lines.length) {
    return {
      main: store.currentSong?.title || "微风音乐 · 概念版无损直连",
      sub: store.currentSong?.artist || "耳畔微风起 · 帧享真无损",
      words: [],
      lineStartMs: 0,
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
  return {
    main: active?.text || "",
    sub:
      (store.showTranslation && active?.translation) ||
      (store.showPhonetic && active?.phonetic) ||
      nextLine?.text ||
      "",
    words: active?.words || [],
    lineStartMs: active?.startTimeMs || 0,
  };
});

function desktopWordFillPercent(
  lineStartMs: number,
  wordStartMs: number,
  wordDurMs: number
): number {
  const pos = calibratedPosMs.value;
  const absStart =
    wordStartMs >= lineStartMs ? wordStartMs : lineStartMs + wordStartMs;
  if (pos <= absStart) return 0;
  if (pos >= absStart + Math.max(80, wordDurMs)) return 100;
  return Math.min(100, ((pos - absStart) / Math.max(80, wordDurMs)) * 100);
}

const greetingSubtitle = computed(() => {
  const h = new Date().getHours();
  if (h >= 5 && h < 11) return "晨光微风 · 用清澈旋律唤醒耳朵";
  if (h >= 11 && h < 14) return "午间小憩 · 漫游属于你的声学绿洲";
  if (h >= 14 && h < 18) return "午后灵感 · 无损音质伴你专注时光";
  if (h >= 18 && h < 23) return "夜幕微蓝 · 卸下喧嚣沉浸私人旋律";
  return "深夜静谧 · 让温柔乐音陪你入眠";
});

function resetStageViewportScroll() {
  if (stageViewportRef.value) {
    stageViewportRef.value.scrollTop = 0;
  }
  const el = document.querySelector(".skyline-stage-viewport") as HTMLElement | null;
  if (el) {
    el.scrollTop = 0;
  }
}

// 切换任何主标签、发现页独立二级页面、我的页详情或全屏舞台时，自动重置滚动条至最顶部
watch(
  [
    () => store.activeTab,
    () => store.discoverSubView,
    () => store.libraryDetailMode,
    () => store.isNowPlayingOpen,
    () => store.activeRank?.rankId,
  ],
  () => {
    resetStageViewportScroll();
    nextTick(() => {
      resetStageViewportScroll();
      requestAnimationFrame(resetStageViewportScroll);
    });
  }
);

function handleGlobalKeyDown(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null;
  const isInput =
    target &&
    (target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.isContentEditable);

  // Ctrl + K：快速聚焦搜索框
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
    e.preventDefault();
    const input = document.querySelector(".skyline-search-input") as HTMLInputElement | null;
    if (input) {
      input.focus();
      input.select();
    } else {
      store.isNowPlayingOpen = false;
      store.activeTab = "search";
    }
    return;
  }

  // Esc：收起弹窗或全屏播放舞台
  if (e.key === "Escape") {
    if (store.isMultiArtistModalOpen) {
      store.isMultiArtistModalOpen = false;
      return;
    }
    if (store.isCommentsModalOpen) {
      store.closeCommentsModal();
      return;
    }
    if (store.hdArtworkPreviewUrl) {
      store.hdArtworkPreviewUrl = null;
      return;
    }
    if (store.shareModalSong) {
      store.shareModalSong = null;
      return;
    }
    if (store.songDetailModalSong) {
      store.songDetailModalSong = null;
      return;
    }
    if (store.downloadModalSong) {
      store.downloadModalSong = null;
      return;
    }
    if (store.addToPlaylistModalSong) {
      store.addToPlaylistModalSong = null;
      return;
    }
    if (store.isLyricPosterOpen) {
      store.isLyricPosterOpen = false;
      return;
    }
    if (store.isFocusPodOpen) {
      store.isFocusPodOpen = false;
      return;
    }
    if (store.isVipCenterOpen) {
      store.isVipCenterOpen = false;
      return;
    }
    if (store.isGlobalQueueOpen) {
      store.isGlobalQueueOpen = false;
      return;
    }
    if (store.isNowPlayingOpen && !isInput) {
      store.isNowPlayingOpen = false;
      return;
    }
  }

  if (isInput) return;

  // Space：播放/暂停
  if (e.code === "Space") {
    e.preventDefault();
    store.togglePlayPause();
    return;
  }

  // Ctrl + Left / Right：上一曲 / 下一曲
  if ((e.ctrlKey || e.metaKey) && e.key === "ArrowLeft") {
    e.preventDefault();
    store.playPrev();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key === "ArrowRight") {
    e.preventDefault();
    store.playNext(false);
    return;
  }
}

async function copyShareCommand(song: any) {
  if (!song) return;
  const text = `🎵 正在微风音乐概念版收听：${song.title} - ${song.artist}${song.album ? `《${song.album}》` : ""} #KGHash:${song.hash}#`;
  try {
    await navigator.clipboard.writeText(text);
    store.notify("已复制微风音乐分享口令，可在「导入歌单」中直接解析播放 ✨");
  } catch {
    store.notify(text);
  }
}

onMounted(() => {
  store.bootstrap();
  window.addEventListener("keydown", handleGlobalKeyDown);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleGlobalKeyDown);
});
</script>

<template>
  <div
    class="app-root"
    :class="[
      `layout-${store.layoutMode}`,
      `scheme-${store.colorScheme}`,
      `material-${store.windowMaterial}`,
    ]"
    :style="{
      '--dyn-c1': store.dynamicCoverTint ? store.coverPalette[0] : '#2b8cff',
      '--dyn-c2': store.dynamicCoverTint ? store.coverPalette[1] : '#00c4a7',
      '--dyn-c3': store.dynamicCoverTint ? store.coverPalette[2] : '#6c5ce7',
    }"
    @click="
      store.isSearchPaletteOpen = false;
      store.isGlobalQueueOpen = false;
      store.closeSongContextMenu();
    "
  >
    <!-- 全局浮动提示胶囊 -->
    <transition name="pop-spring">
      <div v-if="store.statusBanner" class="toast-banner">
        <PlumpIcon name="sparkles" :size="16" color="#00e5ff" />
        <span>{{ store.statusBanner }}</span>
      </div>
    </transition>

    <!-- 形态 C：Windows 桌面置顶迷你黑胶挂件模式 -->
    <template v-if="store.layoutMode === 'mini_turntable'">
      <div class="mini-widget-shell" data-tauri-drag-region>
        <div
          class="mini-widget-vinyl"
          :class="{ spinning: store.isPlaying }"
          data-tauri-drag-region
        >
          <img
            v-if="store.currentSong?.coverUrl"
            :src="store.currentSong.coverUrl"
            class="mini-widget-cover"
          />
          <PlumpIcon v-else name="disc" :size="28" color="#00e5ff" />
        </div>

        <div class="mini-widget-body" data-tauri-drag-region>
          <div class="mini-widget-title" data-tauri-drag-region>
            {{ store.currentSong?.title || "BreezeMusic 迷你挂件" }}
          </div>
          <div class="mini-widget-lyric" data-tauri-drag-region>
            {{ currentLyricPair.main }}
          </div>

          <div class="mini-widget-controls">
            <button class="dock-icon-btn" @click="store.playPrev()">
              <PlumpIcon name="prev" :size="16" />
            </button>
            <button
              class="dock-play-main-btn mini"
              @click="store.togglePlayPause()"
            >
              <PlumpIcon
                :name="store.isPlaying ? 'pause' : 'play'"
                :size="16"
                color="#ffffff"
              />
            </button>
            <button class="dock-icon-btn" @click="store.playNext(false)">
              <PlumpIcon name="next" :size="16" />
            </button>
            <button
              class="mini-expand-btn"
              title="还原顶部天际线大窗口模式"
              @click="store.layoutMode = 'floating_bento'"
            >
              <span>展开主窗口</span>
            </button>
          </div>
        </div>
      </div>
    </template>

    <!-- 形态 A & B：全屏沉浸式背景渲染 + 顶层悬浮胶囊导航与右上角融合卡片 -->
    <template v-else>
      <TitleBar />

      <div class="skyline-workspace">
        <main
          ref="stageViewportRef"
          class="skyline-stage-viewport"
          :class="{
            'viewport-locked-library': store.activeTab === 'library' && !store.isNowPlayingOpen,
            'viewport-locked-nowplaying': store.isNowPlayingOpen,
          }"
        >
          <!-- 左上角时段语句：置于页面内容流顶部，无多余左上角返回按钮，下滑浏览时随页面自然向上消失 -->
          <div
            v-if="!store.isNowPlayingOpen"
            class="inflow-top-greeting-row"
            data-tauri-drag-region
          >
            <div class="inflow-greeting-statement" data-tauri-drag-region>
              <span class="inflow-greeting-dot"></span>
              <span class="inflow-greeting-text" data-tauri-drag-region>{{ greetingSubtitle }}</span>
            </div>
          </div>

          <transition name="view-morph" mode="out-in" @before-enter="resetStageViewportScroll">
            <NowPlayingView
              v-if="store.isNowPlayingOpen"
              key="now-playing"
            />
            <DiscoverView
              v-else-if="store.activeTab === 'discover'"
              key="discover"
            />
            <LibraryView
              v-else-if="store.activeTab === 'library'"
              key="library"
            />
            <RadioView
              v-else-if="store.activeTab === 'radio' || store.activeTab === 'tuner'"
              key="tuner"
            />
            <PlaylistDetailView
              v-else-if="store.activeTab === 'playlist_detail'"
              key="playlist-detail"
            />
            <ArtistDetailView
              v-else-if="store.activeTab === 'artist_detail'"
              key="artist-detail"
            />
            <AlbumDetailView
              v-else-if="store.activeTab === 'album_detail'"
              key="album-detail"
            />
            <SearchPlaceholderView
              v-else-if="store.activeTab === 'search'"
              key="search"
            />
            <SettingsPlaceholderView v-else key="settings" />
          </transition>
        </main>
      </div>

      <!-- 系统级悬浮桌面歌词舱（规定长度 + 屏幕内任意 2D 自由拖动 + KRC 逐字流光扫掠） -->
      <transition name="pop-spring">
        <div
          v-if="store.desktopLyricEnabled"
          ref="desktopLyricRef"
          class="floating-desktop-lyric-bar"
          :class="{
            locked: store.desktopLyricLocked,
            dragging: isDraggingDesktopLyric,
          }"
          :style="desktopLyricPositionStyle"
          @mousedown="onDesktopLyricMouseDown"
        >
          <div v-if="!store.desktopLyricLocked" class="fdl-top-toolbar">
            <span class="fdl-song-chip" title="按住歌词栏任意空白处即可在屏幕内自由拖动">
              ✥ {{ store.currentSong?.title || "微风桌面歌词" }} ({{ store.desktopLyricWidth }}px)
            </span>
            <div class="fdl-btn-cluster">
              <button class="fdl-icon-btn" title="上一曲" @click.stop="store.playPrev()">
                <PlumpIcon name="prev" :size="13" />
              </button>
              <button class="fdl-icon-btn" title="播放 / 暂停" @click.stop="store.togglePlayPause()">
                <PlumpIcon :name="store.isPlaying ? 'pause' : 'play'" :size="13" />
              </button>
              <button class="fdl-icon-btn" title="下一曲" @click.stop="store.playNext(false)">
                <PlumpIcon name="next" :size="13" />
              </button>
              <button
                class="fdl-icon-btn"
                title="缩短桌面歌词规定长度 (-60px)"
                @click.stop="store.desktopLyricWidth = Math.max(480, (store.desktopLyricWidth || 760) - 60)"
              >
                W-
              </button>
              <button
                class="fdl-icon-btn"
                title="加长桌面歌词规定长度 (+60px)"
                @click.stop="store.desktopLyricWidth = Math.min(1280, (store.desktopLyricWidth || 760) + 60)"
              >
                W+
              </button>
              <button
                class="fdl-icon-btn"
                title="字号缩小"
                @click.stop="store.desktopLyricFontSize = Math.max(18, (store.desktopLyricFontSize || 24) - 2)"
              >
                A-
              </button>
              <button
                class="fdl-icon-btn"
                title="字号放大"
                @click.stop="store.desktopLyricFontSize = Math.min(36, (store.desktopLyricFontSize || 24) + 2)"
              >
                A+
              </button>
              <button
                class="fdl-icon-btn"
                title="切换桌面歌词流光配色"
                @click.stop="
                  store.desktopLyricColor = ['#00e5ff', '#ffd54f', '#ff6b8b', '#00e5a3', '#b388ff'][
                    (['#00e5ff', '#ffd54f', '#ff6b8b', '#00e5a3', '#b388ff'].indexOf(store.desktopLyricColor) + 1) % 5
                  ]
                "
              >
                🎨
              </button>
              <button
                class="fdl-btn"
                title="锁定桌面歌词防误触"
                @click.stop="store.desktopLyricLocked = true"
              >
                <PlumpIcon name="lock" :size="13" />
                <span>锁定</span>
              </button>
              <button
                class="fdl-btn"
                title="关闭桌面歌词"
                @click.stop="store.desktopLyricEnabled = false"
              >
                <PlumpIcon name="close" :size="13" />
              </button>
            </div>
          </div>

          <div v-else class="fdl-locked-unlock-pill">
            <button
              class="fdl-btn"
              title="点击解锁桌面歌词"
              @click.stop="store.desktopLyricLocked = false"
            >
              <PlumpIcon name="unlock" :size="12" />
              <span>已锁 · 点击解锁</span>
            </button>
          </div>

          <div class="fdl-lines">
            <div
              v-if="store.lyricWordByWord && currentLyricPair.words.length > 0"
              class="fdl-main krc-words-row"
            >
              <span
                v-for="(w, wIdx) in currentLyricPair.words"
                :key="wIdx"
                class="fdl-krc-word"
                :style="{
                  '--fill-pct': `${desktopWordFillPercent(
                    currentLyricPair.lineStartMs,
                    w.startTimeMs,
                    w.durationMs
                  )}%`,
                }"
              >
                {{ w.text }}
              </span>
            </div>
            <div v-else class="fdl-main">{{ currentLyricPair.main }}</div>
            <div v-if="currentLyricPair.sub" class="fdl-sub">
              {{ currentLyricPair.sub }}
            </div>
          </div>
        </div>
      </transition>

      <PlayerBar />

      <!-- 全局模态层 -->
      <LoginModal />
      <FocusPodModal />
      <LyricPosterModal />
      <SongContextMenu />
      <CommentsModal />
      <MultiArtistModal />

      <!-- 全局 1000P 超清专辑封面预览与保存弹窗 -->
      <transition name="fade">
        <div
          v-if="store.hdArtworkPreviewUrl"
          class="modal-backdrop"
          @click.self="store.hdArtworkPreviewUrl = null"
        >
          <div class="hd-cover-modal glass-popover" @click.stop>
            <img :src="store.hdArtworkPreviewUrl" class="hd-cover-large" alt="HD Cover" />
            <div class="hd-cover-actions">
              <a
                :href="store.hdArtworkPreviewUrl"
                target="_blank"
                download
                class="primary-btn"
              >
                <PlumpIcon name="download" :size="16" color="#ffffff" />
                <span>打开 / 保存 1000×1000 超清原图</span>
              </a>
              <button
                class="tonal-pill-btn"
                @click="store.hdArtworkPreviewUrl = null"
              >
                关闭预览
              </button>
            </div>
          </div>
        </div>
      </transition>

      <!-- 全局歌曲分享与口令生成弹窗 -->
      <transition name="fade">
        <div
          v-if="store.shareModalSong"
          class="modal-backdrop"
          @click.self="store.shareModalSong = null"
        >
          <div class="compact-dialog glass-popover" @click.stop>
            <div class="modal-top-bar">
              <h3>分享歌曲与微风口令</h3>
              <button class="icon-close-btn" @click="store.shareModalSong = null">
                <PlumpIcon name="close" :size="15" />
              </button>
            </div>
            <div class="share-song-card-preview">
              <img
                v-if="store.shareModalSong.coverUrl"
                :src="store.shareModalSong.coverUrl"
                class="share-song-cover"
              />
              <div class="share-song-meta">
                <div class="share-song-title">{{ store.shareModalSong.title }}</div>
                <div class="share-song-artist">{{ store.shareModalSong.artist }}</div>
                <div class="share-song-hash">Hash: {{ store.shareModalSong.hash }}</div>
              </div>
            </div>
            <p class="dialog-sub">
              复制微风音乐口令给好友，对方在「我的 → 导入歌单」粘贴即可直连播放；或一键生成 2K 艺术歌词海报：
            </p>
            <div class="dialog-actions">
              <button
                class="tonal-pill-btn"
                @click="
                  if (store.shareModalSong) {
                    store.playSongFromList(store.shareModalSong);
                  }
                  store.shareModalSong = null;
                  store.isLyricPosterOpen = true;
                "
              >
                <PlumpIcon name="poster" :size="15" />
                <span>生成 2K 歌词海报</span>
              </button>
              <button
                class="primary-btn"
                @click="
                  copyShareCommand(store.shareModalSong);
                  store.shareModalSong = null;
                "
              >
                <PlumpIcon name="copy" :size="15" color="#ffffff" />
                <span>复制分享口令</span>
              </button>
            </div>
          </div>
        </div>
      </transition>
    </template>
  </div>
</template>
