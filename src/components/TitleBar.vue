<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore } from "../stores/appStore";
import {
  Disc3,
  Minus,
  Square,
  X,
  Search,
  ChevronLeft,
} from "lucide-vue-next";

const store = useAppStore();

function handleBack() {
  if (store.isNowPlayingOpen) {
    store.isNowPlayingOpen = false;
    return;
  }
  if (store.activeTab === "discover" && store.discoverSubView !== "home") {
    store.discoverSubView = "home";
  }
}

async function minWin() {
  try {
    await getCurrentWindow().minimize();
  } catch {}
}

async function maxWin() {
  try {
    await getCurrentWindow().toggleMaximize();
  } catch {}
}

async function closeWin() {
  try {
    await getCurrentWindow().close();
  } catch {}
}

function jumpToSearch() {
  store.isNowPlayingOpen = false;
  store.activeTab = "search";
}
</script>

<template>
  <header class="custom-titlebar" data-tauri-drag-region>
    <div class="titlebar-left" data-tauri-drag-region>
      <div class="titlebar-brand" data-tauri-drag-region>
        <div class="titlebar-logo">
          <Disc3 :size="16" :class="{ spinning: store.isPlaying }" />
        </div>
        <span class="titlebar-appname">BreezeMusic</span>
        <span class="titlebar-tag">CONCEPT DESKTOP</span>
      </div>

      <button
        v-if="
          store.isNowPlayingOpen ||
          (store.activeTab === 'discover' && store.discoverSubView !== 'home')
        "
        class="titlebar-back-pill"
        @click="handleBack"
      >
        <ChevronLeft :size="15" />
        <span>返回</span>
      </button>
    </div>

    <!-- 中部快捷搜索胶囊入口（不在主页面堆砌杂乱设置按钮） -->
    <div class="titlebar-center" data-tauri-drag-region>
      <button class="titlebar-search-trigger" @click="jumpToSearch">
        <Search :size="14" />
        <span>搜索歌曲、歌手或概念版无损音源...</span>
      </button>
    </div>

    <!-- 右侧纯净系统窗口控制键 -->
    <div class="titlebar-window-controls">
      <button class="win-btn" title="最小化" @click="minWin">
        <Minus :size="15" />
      </button>
      <button class="win-btn" title="最大化 / 还原" @click="maxWin">
        <Square :size="13" />
      </button>
      <button class="win-btn close" title="关闭" @click="closeWin">
        <X :size="16" />
      </button>
    </div>
  </header>
</template>
