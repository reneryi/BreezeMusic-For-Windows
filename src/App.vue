<script setup lang="ts">
import { onMounted, computed } from "vue";
import { useAppStore } from "./stores/appStore";
import TitleBar from "./components/TitleBar.vue";
import Sidebar from "./components/Sidebar.vue";
import PlayerBar from "./components/PlayerBar.vue";
import LoginModal from "./components/LoginModal.vue";
import VipCenterModal from "./components/VipCenterModal.vue";
import FocusPodModal from "./components/FocusPodModal.vue";
import DiscoverView from "./views/DiscoverView.vue";
import LibraryView from "./views/LibraryView.vue";
import SearchPlaceholderView from "./views/SearchPlaceholderView.vue";
import SettingsPlaceholderView from "./views/SettingsPlaceholderView.vue";
import NowPlayingView from "./views/NowPlayingView.vue";
import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Maximize2,
  Disc3,
} from "lucide-vue-next";

const store = useAppStore();

const currentLyricText = computed(() => {
  const lines = store.currentLyric.lines;
  if (!lines.length) return store.currentSong?.artist || "BreezeMusic · 概念版";
  const pos = store.currentPositionMs;
  let active = lines[0]?.text || "";
  for (const l of lines) {
    if (l.startTimeMs <= pos) {
      active = l.text;
    } else {
      break;
    }
  }
  return active;
});

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
    ]"
    :style="{
      '--dyn-c1': store.dynamicCoverTint ? store.coverPalette[0] : '#2b8cff',
      '--dyn-c2': store.dynamicCoverTint ? store.coverPalette[1] : '#00c4a7',
    }"
  >
    <!-- 全局浮动提示胶囊 -->
    <transition name="fade">
      <div v-if="store.statusBanner" class="toast-banner">
        {{ store.statusBanner }}
      </div>
    </transition>

    <!-- 形态 C：Windows 桌面置顶迷你黑胶挂件模式 (Mini Turntable Widget) -->
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
          <Disc3 v-else :size="28" />
        </div>

        <div class="mini-widget-body" data-tauri-drag-region>
          <div class="mini-widget-title" data-tauri-drag-region>
            {{ store.currentSong?.title || "BreezeMusic 迷你挂件" }}
          </div>
          <div class="mini-widget-lyric" data-tauri-drag-region>
            {{ currentLyricText }}
          </div>

          <div class="mini-widget-controls">
            <button class="dock-icon-btn" @click="store.playPrev()">
              <SkipBack :size="16" />
            </button>
            <button class="mini-play-btn" @click="store.togglePlayPause()">
              <Pause v-if="store.isPlaying" :size="16" />
              <Play v-else :size="16" />
            </button>
            <button class="dock-icon-btn" @click="store.playNext()">
              <SkipForward :size="16" />
            </button>
            <button
              class="dock-icon-btn"
              title="还原完整桌面窗口"
              @click="store.layoutMode = 'floating_bento'"
            >
              <Maximize2 :size="15" />
            </button>
          </div>
        </div>
      </div>
    </template>

    <!-- 形态 A / B：灵动悬浮岛画布 (Floating Bento Island) 与 极简纯粹模式 (Zen Minimalist) -->
    <template v-else>
      <TitleBar />

      <div class="desktop-workspace">
        <Sidebar />

        <main class="stage-viewport">
          <transition name="page-morph" mode="out-in">
            <NowPlayingView v-if="store.isNowPlayingOpen" key="nowplaying" />
            <DiscoverView
              v-else-if="store.activeTab === 'discover'"
              key="discover"
            />
            <LibraryView
              v-else-if="store.activeTab === 'library'"
              key="library"
            />
            <SearchPlaceholderView
              v-else-if="store.activeTab === 'search'"
              key="search"
            />
            <SettingsPlaceholderView
              v-else-if="store.activeTab === 'settings'"
              key="settings"
            />
          </transition>
        </main>
      </div>

      <PlayerBar />
    </template>

    <!-- 独立模态弹窗 -->
    <LoginModal v-if="store.isLoginModalOpen" />
    <VipCenterModal v-if="store.isVipCenterOpen" />
    <FocusPodModal v-if="store.isFocusPodOpen" />
  </div>
</template>
