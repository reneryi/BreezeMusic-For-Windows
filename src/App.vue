<script setup lang="ts">
import { onMounted, computed } from "vue";
import { useAppStore } from "./stores/appStore";
import TitleBar from "./components/TitleBar.vue";
import PlayerBar from "./components/PlayerBar.vue";
import LoginModal from "./components/LoginModal.vue";
import VipCenterModal from "./components/VipCenterModal.vue";
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
    <!-- 全局浮动提示胶囊 (无框柔光胶囊) -->
    <transition name="pop-spring">
      <div v-if="store.statusBanner" class="toast-banner">
        <PlumpIcon name="sparkles" :size="15" color="#00e5ff" />
        <span>{{ store.statusBanner }}</span>
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

    <!-- 形态 A & B：方案二「顶部天际线中枢画廊 (Skyline Command & Bento)」100% 全宽无遮挡视野 -->
    <template v-else>
      <TitleBar />

      <div class="skyline-workspace">
        <main class="skyline-stage-viewport">
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

      <!-- 系统级悬浮桌面歌词舱（支持锁定穿透） -->
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
    </template>

    <!-- 全局弹窗与上下文菜单 -->
    <LoginModal />
    <VipCenterModal />
    <FocusPodModal />
    <LyricPosterModal />
    <SongContextMenu />
  </div>
</template>
