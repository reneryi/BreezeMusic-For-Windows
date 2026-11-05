<script setup lang="ts">
import {
  onMounted,
  onBeforeUnmount,
  computed,
  ref,
  watch,
  nextTick,
  defineAsyncComponent,
} from "vue";
import { useAppStore } from "./stores/appStore";
import TitleBar from "./components/TitleBar.vue";
import PlayerBar from "./components/PlayerBar.vue";
import LoginModal from "./components/LoginModal.vue";
import SongContextMenu from "./components/SongContextMenu.vue";
import MultiArtistModal from "./components/MultiArtistModal.vue";
import DesktopLyricWindow from "./components/DesktopLyricWindow.vue";
import PlumpIcon from "./components/PlumpIcon.vue";
import DiscoverView from "./views/DiscoverView.vue";

const FocusPodModal = defineAsyncComponent(
  () => import("./components/FocusPodModal.vue")
);
const VipCenterModal = defineAsyncComponent(
  () => import("./components/VipCenterModal.vue")
);
const AccountManagerModal = defineAsyncComponent(
  () => import("./components/AccountManagerModal.vue")
);
const LyricPosterModal = defineAsyncComponent(
  () => import("./components/LyricPosterModal.vue")
);
const CommentsModal = defineAsyncComponent(
  () => import("./components/CommentsModal.vue")
);
const LibraryView = defineAsyncComponent(
  () => import("./views/LibraryView.vue")
);
const RadioView = defineAsyncComponent(
  () => import("./views/RadioView.vue")
);
const SearchPlaceholderView = defineAsyncComponent(
  () => import("./views/SearchPlaceholderView.vue")
);
const SettingsPlaceholderView = defineAsyncComponent(
  () => import("./views/SettingsPlaceholderView.vue")
);
const NowPlayingView = defineAsyncComponent(
  () => import("./views/NowPlayingView.vue")
);
const PlaylistDetailView = defineAsyncComponent(
  () => import("./views/PlaylistDetailView.vue")
);
const ArtistDetailView = defineAsyncComponent(
  () => import("./views/ArtistDetailView.vue")
);
const AlbumDetailView = defineAsyncComponent(
  () => import("./views/AlbumDetailView.vue")
);

const isStandaloneDesktopLyricWindow =
  typeof window !== "undefined" &&
  window.location.search.includes("window=desktop_lyric");

const isTauriRuntime =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

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
    const width = store.desktopLyricWidth || rect.width || 440;
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
  const w = store.desktopLyricWidth || 440;
  const base: Record<string, string> = {
    "--fdl-color": store.desktopLyricColor || "#00e5ff",
    "--fdl-size": `${store.desktopLyricFontSize || 22}px`,
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
    if (store.isAccountManagerOpen) {
      store.isAccountManagerOpen = false;
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

  // Ctrl + Shift + L：一键锁定 / 解锁桌面歌词（解决鼠标穿透锁定后快速解锁需求）
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "l") {
    e.preventDefault();
    store.desktopLyricLocked = !store.desktopLyricLocked;
    store.notify(
      store.desktopLyricLocked
        ? "已锁定桌面歌词（开启鼠标穿透，按 Ctrl+Shift+L 可随时解锁）"
        : "已解锁桌面歌词，现在可自由拖动与调节"
    );
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
  if (!isStandaloneDesktopLyricWindow) {
    store.bootstrap();
    window.addEventListener("keydown", handleGlobalKeyDown);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleGlobalKeyDown);
});
</script>

<template>
  <DesktopLyricWindow v-if="isStandaloneDesktopLyricWindow" />
  <div
    v-else
    class="app-root"
    :class="[`scheme-${store.resolvedColorScheme}`]"
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

    <!-- 全屏沉浸式背景渲染 + 顶层悬浮胶囊导航与主画布 -->
    <TitleBar />

    <div class="skyline-workspace">
      <main
        ref="stageViewportRef"
        class="skyline-stage-viewport"
        :class="{
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

    <!-- 浏览器预览回退桌面歌词舱（Tauri 桌面端由独立透明置顶窗口 desktop_lyric 渲染，避免双实例重叠） -->
    <transition name="pop-spring">
      <div
        v-if="store.desktopLyricEnabled && !isTauriRuntime"
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
              title="缩短桌面歌词规定长度 (-30px)"
              @click.stop="store.desktopLyricWidth = Math.max(340, (store.desktopLyricWidth || 440) - 30)"
            >
              W-
            </button>
            <button
              class="fdl-icon-btn"
              title="加长桌面歌词规定长度 (+30px)"
              @click.stop="store.desktopLyricWidth = Math.min(720, (store.desktopLyricWidth || 440) + 30)"
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
    <VipCenterModal />
    <AccountManagerModal />
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
  </div>
</template>
