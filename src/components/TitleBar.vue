<template>
  <!-- 顶层全屏无框悬浮交互层：不切割背景，下滑浏览时悬浮胶囊自然遮挡下方内容 -->
  <div class="skyline-floating-overlay">
    <!-- 顶部边缘拖拽热区 -->
    <div class="skyline-top-drag-strip" data-tauri-drag-region></div>

    <!-- 左上角悬浮返回/收起按钮：播放页显示「收起舞台」，独立详情页/二级页显示「返回上一页」 -->
    <button
      v-if="store.isNowPlayingOpen"
      class="skyline-np-floating-back"
      title="收起播放页 (Esc)"
      @click="store.isNowPlayingOpen = false"
    >
      <PlumpIcon name="chevron-down" :size="18" />
      <span>收起舞台</span>
    </button>
    <button
      v-else-if="store.canNavigateBack"
      class="skyline-np-floating-back"
      title="返回上一页"
      @click="store.navigateBack()"
    >
      <PlumpIcon name="arrow-left" :size="17" />
      <span>返回</span>
    </button>

    <!-- 居中悬浮岛：顶部导航胶囊 + 响应式搜索框（小窗口自动收缩为圆形按钮，无快捷键标签） -->
    <div class="skyline-floating-center">
      <nav class="skyline-nav-capsule" aria-label="主导航">
        <button
          v-for="item in navItems"
          :key="item.id"
          class="skyline-nav-btn"
          :class="{ active: (store.activeTab === item.id || (item.id === 'tuner' && store.activeTab === 'radio')) && !store.isNowPlayingOpen }"
          :title="item.hint"
          @click="handleNavClick(item.id)"
          @dblclick.stop="handleNavDblClick(item.id)"
        >
          <PlumpIcon :name="item.icon" :size="17" />
          <span>{{ item.label }}</span>
        </button>
      </nav>

      <!-- 响应式悬浮搜索组件：大窗为无快捷键胶囊输入条，小窗收缩为圆形悬浮按钮 -->
      <div class="skyline-search-wrap" ref="searchBoxRef">
        <!-- 小窗口且未展开时：显示为精致圆形悬浮按钮 -->
        <button
          v-if="isCompactWindow && !compactSearchExpanded"
          class="skyline-search-orb-btn"
          title="展开搜索 (Ctrl + K)"
          @click="expandCompactSearch"
        >
          <PlumpIcon name="search" :size="18" />
        </button>

        <!-- 宽窗口常驻 或 小窗口点击展开后：显示悬浮搜索输入胶囊 -->
        <div
          v-else
          class="skyline-search-pill"
          :class="{
            focused: searchFocused,
            'compact-expanded-pill': isCompactWindow && compactSearchExpanded,
          }"
        >
          <PlumpIcon name="search" :size="17" class="search-pill-icon" />
          <input
            ref="searchInputRef"
            v-model="store.searchKeyword"
            type="text"
            class="skyline-search-input"
            placeholder="搜索歌曲、歌手、专辑、歌单..."
            @focus="onSearchFocus"
            @input="onSearchInput"
            @keydown.enter="triggerSearch(store.searchKeyword)"
            @keydown.esc="collapseCompactSearch"
          />
          <button
            v-if="store.searchKeyword || (isCompactWindow && compactSearchExpanded)"
            class="search-clear-mini"
            title="清空或收起"
            @click.stop="handleClearOrCollapseSearch"
          >
            <PlumpIcon name="close" :size="14" />
          </button>
        </div>

        <!-- 实时联想词、历史搜索与热搜榜下拉面板 -->
        <transition name="fade-pop">
          <div
            v-if="searchFocused && (store.searchSuggestions.length > 0 || store.searchHistory.length > 0 || store.hotKeywords.length > 0)"
            class="search-suggest-dropdown glass-popover"
          >
            <template v-if="store.searchKeyword.trim() && store.searchSuggestions.length > 0">
              <div class="suggest-group-title">实时搜索联想</div>
              <button
                v-for="(sug, i) in store.searchSuggestions"
                :key="i"
                class="suggest-row"
                @click="triggerSearch(sug)"
              >
                <PlumpIcon name="search" :size="15" />
                <span>{{ sug }}</span>
              </button>
            </template>
            <template v-else>
              <template v-if="store.searchHistory.length > 0">
                <div class="suggest-group-title">
                  <span>最近搜索</span>
                  <button class="text-link-btn" @click.stop="store.clearSearchHistory()">清空</button>
                </div>
                <div class="suggest-chips">
                  <button
                    v-for="(h, idx) in store.searchHistory.slice(0, 8)"
                    :key="idx"
                    class="suggest-chip"
                    @click="triggerSearch(h)"
                  >
                    {{ h }}
                  </button>
                </div>
              </template>

              <template v-if="store.hotKeywords.length > 0">
                <div class="suggest-group-title" style="margin-top: 4px">
                  <span>全网热搜</span>
                </div>
                <div class="sd-hot-grid">
                  <button
                    v-for="(hk, hIdx) in store.hotKeywords.slice(0, 8)"
                    :key="hIdx"
                    class="sd-hot-item"
                    @click="triggerSearch(hk)"
                  >
                    <span class="sd-hot-rank" :class="{ top: hIdx < 3 }">{{ hIdx + 1 }}</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{{ hk }}</span>
                  </button>
                </div>
              </template>
            </template>
          </div>
        </transition>
      </div>
    </div>

    <!-- 右上角悬浮区：明暗切换 + 三个窗口控制按钮统一长形悬浮胶囊（个人身份与 VIP 中心统一收归「我的」页首屏） -->
    <div class="skyline-floating-right">
      <div class="skyline-win-floating-capsule">
        <button
          class="win-capsule-btn theme-btn"
          :title="themeBtnTitle"
          @click="toggleColorScheme"
        >
          <PlumpIcon
            :name="store.resolvedColorScheme === 'watercolor_light' ? 'moon' : 'sun'"
            :size="16"
          />
        </button>

        <span class="win-capsule-divider"></span>

        <button class="win-capsule-btn" title="最小化" @click="minimizeWin">
          <PlumpIcon name="minimize" :size="14" />
        </button>
        <button class="win-capsule-btn" title="最大化 / 还原" @click="toggleMaximizeWin">
          <PlumpIcon name="maximize" :size="14" />
        </button>
        <button class="win-capsule-btn close" title="关闭窗口" @click="closeWin">
          <PlumpIcon name="close" :size="14" />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore, type MainTab } from "../stores/appStore";
import { kugouClient } from "../api/kugou";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

const navItems: Array<{
  id: MainTab;
  label: string;
  hint: string;
  icon: "compass" | "tuning" | "library" | "settings";
}> = [
  { id: "discover", label: "发现", hint: "单击进入发现页 · 双击整体刷新发现页内容", icon: "compass" },
  { id: "library", label: "我的", hint: "单击进入我的音乐 · 双击同步云端音乐资产", icon: "library" },
  { id: "tuner", label: "调音台", hint: "概念版母带声学调音台与蝰蛇音效工作站（与电台完全独立）", icon: "tuning" },
  { id: "settings", label: "设置", hint: "外观主题、播放偏好、设备凭证与下载设置", icon: "settings" },
];

// 窗口宽度监听：小窗口/非全屏窗口（<=1400px）时搜索框自动收缩为圆形按钮
const windowWidth = ref(typeof window !== "undefined" ? window.innerWidth : 1240);
const isCompactWindow = computed(() => windowWidth.value <= 1400);
const compactSearchExpanded = ref(false);

const searchFocused = ref(false);
const searchBoxRef = ref<HTMLElement | null>(null);
const searchInputRef = ref<HTMLInputElement | null>(null);
let suggestTimer: number | null = null;

function onWindowResize() {
  windowWidth.value = window.innerWidth;
  if (!isCompactWindow.value) {
    compactSearchExpanded.value = false;
  }
}

function expandCompactSearch() {
  compactSearchExpanded.value = true;
  nextTick(() => {
    searchInputRef.value?.focus();
  });
}

function collapseCompactSearch() {
  searchFocused.value = false;
  if (isCompactWindow.value && !store.searchKeyword.trim()) {
    compactSearchExpanded.value = false;
  }
}

function handleClearOrCollapseSearch() {
  if (store.searchKeyword) {
    store.searchKeyword = "";
    store.searchSuggestions = [];
    searchInputRef.value?.focus();
  } else {
    compactSearchExpanded.value = false;
    searchFocused.value = false;
  }
}

function handleNavClick(id: MainTab) {
  store.isNowPlayingOpen = false;
  if (id === "discover" && store.activeTab === "discover") {
    store.discoverSubView = "home";
  }
  store.activeTab = id;
}

function handleNavDblClick(id: MainTab) {
  store.isNowPlayingOpen = false;
  store.activeTab = id;
  if (id === "discover") {
    store.discoverSubView = "home";
    store.refreshDiscoverAll();
  } else if (id === "library") {
    store.notify("正在同步云端音乐资产与听歌等级...");
    Promise.all([
      store.refreshAccountAndVip(false),
      store.loadUserPlaylists(),
      store.loadFollowedArtists(),
      store.loadPurchasedAssets(),
    ]).then(() => {
      store.notify("云端音乐资产同步完成 ✨");
    });
  }
}

function onSearchFocus() {
  searchFocused.value = true;
}

function onSearchInput() {
  if (suggestTimer) window.clearTimeout(suggestTimer);
  const kw = store.searchKeyword.trim();
  if (!kw) {
    store.searchSuggestions = [];
    return;
  }
  suggestTimer = window.setTimeout(async () => {
    try {
      store.searchSuggestions = await kugouClient.getSearchSuggest(kw);
    } catch {
      store.searchSuggestions = [];
    }
  }, 180);
}

function triggerSearch(kw: string) {
  const q = kw.trim();
  if (!q) return;
  searchFocused.value = false;
  compactSearchExpanded.value = false;
  store.executeSearch(q);
}

const themeBtnTitle = computed(() => {
  if (store.colorScheme === "system") {
    return `跟随系统自动切换（当前：${store.resolvedColorScheme === "watercolor_light" ? "水彩浸润" : "幽邃微蓝"}）`;
  }
  return store.resolvedColorScheme === "watercolor_light"
    ? "当前：水彩浸润（浅色）· 点击切换为幽邃微蓝（深色）"
    : "当前：幽邃微蓝（深色）· 点击切换为水彩浸润（浅色）";
});

function toggleColorScheme() {
  if (store.resolvedColorScheme === "watercolor_light") {
    store.colorScheme = "abyssal_dark";
    store.notify("已切换为幽邃微蓝（深色）主题");
  } else {
    store.colorScheme = "watercolor_light";
    store.notify("已切换为水彩浸润（浅色）主题");
  }
}

async function minimizeWin() {
  try {
    await getCurrentWindow().minimize();
  } catch {
    store.notify("浏览器预览模式：最小化窗口");
  }
}

async function toggleMaximizeWin() {
  try {
    await getCurrentWindow().toggleMaximize();
  } catch {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen().catch(() => {});
    } else {
      document.exitFullscreen().catch(() => {});
    }
  }
}

async function closeWin() {
  try {
    if (store.closeToTray) {
      await getCurrentWindow().hide();
      store.notify("微风音乐已最小化至系统托盘");
    } else {
      await getCurrentWindow().close();
    }
  } catch {
    store.notify("浏览器预览模式：关闭窗口");
  }
}

function handleOutsideClick(e: MouseEvent) {
  const target = e.target as Node;
  if (searchBoxRef.value && !searchBoxRef.value.contains(target)) {
    searchFocused.value = false;
    if (isCompactWindow.value && !store.searchKeyword.trim()) {
      compactSearchExpanded.value = false;
    }
  }
}

onMounted(() => {
  onWindowResize();
  window.addEventListener("mousedown", handleOutsideClick);
  window.addEventListener("resize", onWindowResize);
});

onBeforeUnmount(() => {
  window.removeEventListener("mousedown", handleOutsideClick);
  window.removeEventListener("resize", onWindowResize);
});
</script>
