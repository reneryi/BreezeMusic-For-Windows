<script setup lang="ts">
import { onMounted, computed } from "vue";
import { useAppStore } from "./stores/appStore";
import TitleBar from "./components/TitleBar.vue";
import PlayerBar from "./components/PlayerBar.vue";
import LoginModal from "./components/LoginModal.vue";
import FocusPodModal from "./components/FocusPodModal.vue";
import LyricPosterModal from "./components/LyricPosterModal.vue";
import SongContextMenu from "./components/SongContextMenu.vue";
import PlumpIcon from "./components/PlumpIcon.vue";
import DiscoverView from "./views/DiscoverView.vue";
import LibraryView from "./views/LibraryView.vue";
import RadioView from "./views/RadioView.vue";
import SearchPlaceholderView from "./views/SearchPlaceholderView.vue";
import SettingsPlaceholderView from "./views/SettingsPlaceholderView.vue";
import NowPlayingView from "./views/NowPlayingView.vue";

const store = useAppStore();

const currentLyricPair = computed(() => {
  const lines = store.currentLyric.lines;
  if (!lines.length) {
    return {
      main: store.currentSong?.title || "微风音乐 · 概念版无损直连",
      sub: store.currentSong?.artist || "耳畔微风起 · 帧享真无损",
    };
  }
  const pos =
    store.currentPositionMs + store.lyricOffsetMs - store.bluetoothDelayMs;
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
    sub: active?.translation || nextLine?.text || "",
  };
});

const greetingSubtitle = computed(() => {
  const h = new Date().getHours();
  if (h >= 5 && h < 11) return "晨光微风 · 用清澈旋律唤醒耳朵";
  if (h >= 11 && h < 14) return "午间小憩 · 漫游属于你的声学绿洲";
  if (h >= 14 && h < 18) return "午后灵感 · 无损音质伴你专注时光";
  if (h >= 18 && h < 23) return "夜幕微蓝 · 卸下喧嚣沉浸私人旋律";
  return "深夜静谧 · 让温柔乐音陪你入眠";
});

const canGoBack = computed(
  () =>
    store.isNowPlayingOpen ||
    (store.activeTab === "discover" && store.discoverSubView !== "home") ||
    (store.activeTab === "library" && store.libraryDetailMode !== "playlists")
);

function goBack() {
  if (store.isNowPlayingOpen) {
    store.isNowPlayingOpen = false;
    return;
  }
  if (store.activeTab === "discover" && store.discoverSubView !== "home") {
    store.discoverSubView = "home";
    return;
  }
  if (store.activeTab === "library" && store.libraryDetailMode !== "playlists") {
    store.libraryDetailMode = "playlists";
    return;
  }
  store.activeTab = "discover";
}

onMounted(() => {
  store.bootstrap();
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
      store.closeSongContextMenu();
    "
  >
    <!-- 全局浮动提示胶囊 -->
    <transition name="pop-spring">
      <div v-if="store.statusBanner" class="toast-banner">
        <PlumpIcon name="sparkles" :size="15" color="#00e5ff" />
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
          class="skyline-stage-viewport"
          :class="{
            'viewport-locked-library': store.activeTab === 'library' && !store.isNowPlayingOpen,
            'viewport-locked-nowplaying': store.isNowPlayingOpen,
          }"
        >
          <!-- 左上角时段语句：置于页面内容流顶部，无标题名称，放大字号，下滑浏览时随页面自然向上消失 -->
          <div
            v-if="!store.isNowPlayingOpen"
            class="inflow-top-greeting-row"
            data-tauri-drag-region
          >
            <button
              v-if="canGoBack"
              class="inflow-back-btn"
              title="返回上一视图"
              @click="goBack"
            >
              <PlumpIcon name="back" :size="18" />
            </button>
            <div class="inflow-greeting-statement" data-tauri-drag-region>
              <span class="inflow-greeting-dot"></span>
              <span class="inflow-greeting-text" data-tauri-drag-region>{{ greetingSubtitle }}</span>
            </div>
          </div>

          <transition name="view-morph" mode="out-in">
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
              v-else-if="store.activeTab === 'radio'"
              key="radio"
            />
            <SearchPlaceholderView
              v-else-if="store.activeTab === 'search'"
              key="search"
            />
            <SettingsPlaceholderView v-else key="settings" />
          </transition>
        </main>
      </div>

      <!-- 系统级悬浮桌面歌词舱 -->
      <transition name="pop-spring">
        <div
          v-if="store.desktopLyricEnabled"
          class="floating-desktop-lyric-bar"
          :class="{ locked: store.desktopLyricLocked }"
        >
          <div class="fdl-lines">
            <div class="fdl-main">{{ currentLyricPair.main }}</div>
            <div v-if="currentLyricPair.sub" class="fdl-sub">
              {{ currentLyricPair.sub }}
            </div>
          </div>
          <div class="fdl-controls">
            <button
              class="fdl-btn"
              :title="
                store.desktopLyricLocked
                  ? '点击解锁桌面歌词'
                  : '锁定桌面歌词 (物理点击穿透)'
              "
              @click="store.desktopLyricLocked = !store.desktopLyricLocked"
            >
              {{ store.desktopLyricLocked ? "已锁" : "锁定" }}
            </button>
            <button
              v-if="!store.desktopLyricLocked"
              class="fdl-btn"
              title="关闭桌面歌词"
              @click="store.desktopLyricEnabled = false"
            >
              <PlumpIcon name="close" :size="12" />
            </button>
          </div>
        </div>
      </transition>

      <PlayerBar />

      <!-- 全局模态层（VIP 中心已融合至右上角头像悬浮展开卡） -->
      <LoginModal />
      <FocusPodModal />
      <LyricPosterModal />
      <SongContextMenu />
    </template>
  </div>
</template>
