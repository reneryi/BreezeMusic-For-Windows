<template>
  <header class="skyline-bar" data-tauri-drag-region>
    <!-- 左侧：返回键 + 四个页面大标题 + 随时段固定在标题右侧的问候语 -->
    <div class="skyline-left" data-tauri-drag-region>
      <button
        v-if="canGoBack"
        class="skyline-back-embedded"
        title="返回上一视图"
        @click="goBack"
      >
        <PlumpIcon name="back" :size="18" />
      </button>

      <div class="skyline-page-header" data-tauri-drag-region>
        <h1 class="skyline-page-title" data-tauri-drag-region>{{ currentPageTitle }}</h1>
        <div class="skyline-greeting-inline" data-tauri-drag-region>
          <span class="skyline-greeting-dot"></span>
          <span class="skyline-greeting-text">{{ greetingSubtitle }}</span>
        </div>
      </div>
    </div>

    <!-- 中间：唯一保留的悬浮导航胶囊 + 悬浮搜索框 -->
    <div class="skyline-center" data-tauri-drag-region>
      <nav class="skyline-nav-capsule" aria-label="主导航">
        <button
          v-for="item in navItems"
          :key="item.id"
          class="skyline-nav-btn"
          :class="{ active: store.activeTab === item.id && !store.isNowPlayingOpen }"
          :title="item.hint"
          @click="handleNavClick(item.id)"
          @dblclick.stop="handleNavDblClick(item.id)"
        >
          <PlumpIcon :name="item.icon" :size="17" />
          <span>{{ item.label }}</span>
        </button>
      </nav>

      <!-- 悬浮快搜输入丸 -->
      <div class="skyline-search-wrap" ref="searchBoxRef">
        <div
          class="skyline-search-pill"
          :class="{ focused: searchFocused }"
        >
          <PlumpIcon name="search" :size="16" class="search-pill-icon" />
          <input
            v-model="store.searchKeyword"
            type="text"
            class="skyline-search-input"
            placeholder="搜索歌曲、歌手、专辑、歌单..."
            @focus="onSearchFocus"
            @input="onSearchInput"
            @keydown.enter="triggerSearch(store.searchKeyword)"
          />
          <kbd class="search-kbd" @click="store.isSearchPaletteOpen = true">Ctrl K</kbd>
        </div>

        <!-- 实时联想词下拉面板 -->
        <transition name="fade-pop">
          <div
            v-if="searchFocused && (store.searchSuggestions.length > 0 || store.searchHistory.length > 0)"
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
                <PlumpIcon name="search" :size="14" />
                <span>{{ sug }}</span>
              </button>
            </template>
            <template v-else-if="store.searchHistory.length > 0">
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
          </div>
        </transition>
      </div>
    </div>

    <!-- 右侧：无框嵌入背景的头像/账号区 + 三个嵌入窗口控制按钮 -->
    <div class="skyline-right" data-tauri-drag-region>
      <div class="skyline-identity-embedded">
        <button
          class="identity-user-embedded"
          :title="store.isLoggedIn ? '点击查看账号或同步云端资产' : '点击登录酷狗概念版账号'"
          @click="handleUserClick"
        >
          <img
            v-if="store.profile?.avatarUrl"
            :src="store.profile.avatarUrl"
            class="identity-avatar-img"
            alt="avatar"
          />
          <div v-else class="identity-avatar-fallback">
            <PlumpIcon name="user" :size="15" />
          </div>
          <span class="identity-nickname">
            {{ store.isLoggedIn ? store.profile?.nickname : "登录概念版" }}
          </span>
          <span v-if="store.isLoggedIn" class="identity-lv-pill">
            Lv.{{ store.profile?.userLevel || 1 }}
          </span>
          <span
            v-if="store.isLoggedIn && (store.profile?.listenDurationMinutes || 0) > 0"
            class="identity-listen-pill"
            title="累计听歌时长"
          >
            {{ Math.round((store.profile?.listenDurationMinutes || 0) / 60) }}h
          </span>
        </button>

        <button
          class="identity-vip-embedded"
          :class="{ claimed: store.vipStatus?.dayVipClaimed }"
          title="概念版每日 VIP 畅听特权中心"
          @click="store.isVipCenterOpen = true"
        >
          <PlumpIcon name="crown" :size="15" />
          <span>{{ store.vipStatus?.dayVipClaimed ? "VIP畅听中" : "领1天VIP" }}</span>
        </button>

        <button
          class="identity-icon-embedded"
          title="切换明亮 / 深邃外观模式"
          @click="toggleColorScheme"
        >
          <PlumpIcon :name="store.colorScheme === 'watercolor_light' || store.colorScheme === 'light' ? 'moon' : 'sun'" :size="16" />
        </button>
      </div>

      <!-- 三个窗口控制按钮：无框嵌入背景 -->
      <div class="skyline-win-controls-embedded">
        <button class="win-embedded-btn" title="最小化" @click="minimizeWin">
          <PlumpIcon name="minimize" :size="14" />
        </button>
        <button class="win-embedded-btn" title="最大化 / 还原" @click="toggleMaximizeWin">
          <PlumpIcon name="maximize" :size="14" />
        </button>
        <button class="win-embedded-btn close" title="关闭窗口" @click="closeWin">
          <PlumpIcon name="close" :size="14" />
        </button>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
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
  { id: "radio", label: "调音台", hint: "概念版母带声学调音台与蝰蛇音效工作站", icon: "tuning" },
  { id: "settings", label: "设置", hint: "外观材质、播放偏好与下载设置", icon: "settings" },
];

const currentPageTitle = computed(() => {
  if (store.isNowPlayingOpen) return "正在播放";
  switch (store.activeTab) {
    case "discover":
      if (store.discoverSubView === "daily") return "每日推荐";
      if (store.discoverSubView === "fm_detail") return "猜你喜欢";
      if (store.discoverSubView === "new_songs") return "新歌速递";
      if (store.discoverSubView === "rank") return store.activeRank?.name || "巅峰排行榜";
      if (store.discoverSubView === "radio") return "电台广场";
      return "发现";
    case "library":
      return "我的";
    case "radio":
      return "调音台";
    case "settings":
      return "设置";
    case "search":
      return "搜索";
    default:
      return "发现";
  }
});

const greetingSubtitle = computed(() => {
  const h = new Date().getHours();
  if (h >= 5 && h < 11) return "晨光微风 · 用清澈旋律唤醒耳朵";
  if (h >= 11 && h < 14) return "午间小憩 · 漫游属于你的声学绿洲";
  if (h >= 14 && h < 18) return "午后灵感 · 无损音质伴你专注时光";
  if (h >= 18 && h < 23) return "夜幕微蓝 · 卸下喧嚣沉浸私人旋律";
  return "深夜静谧 · 让温柔乐音陪你入眠";
});

const searchFocused = ref(false);
const searchBoxRef = ref<HTMLElement | null>(null);
let suggestTimer: number | null = null;

const canGoBack = computed(
  () =>
    store.isNowPlayingOpen ||
    (store.activeTab === "discover" && store.discoverSubView !== "home") ||
    (store.activeTab === "library" && store.libraryDetailMode !== "playlists")
);

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
  store.executeSearch(q);
}

function handleUserClick() {
  if (!store.isLoggedIn) {
    store.isLoginModalOpen = true;
  } else {
    store.isNowPlayingOpen = false;
    store.activeTab = "library";
  }
}

function toggleColorScheme() {
  if (store.colorScheme === "watercolor_light" || store.colorScheme === "light") {
    store.colorScheme = "abyssal_dark";
    store.notify("已切换为深邃夜幕外观");
  } else {
    store.colorScheme = "watercolor_light";
    store.notify("已切换为纯净微风浅色外观");
  }
}

async function minimizeWin() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  } catch {
    store.notify("浏览器预览模式：最小化窗口");
  }
}

async function toggleMaximizeWin() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
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
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
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
  if (searchBoxRef.value && !searchBoxRef.value.contains(e.target as Node)) {
    searchFocused.value = false;
  }
}

onMounted(() => {
  window.addEventListener("mousedown", handleOutsideClick);
});

onBeforeUnmount(() => {
  window.removeEventListener("mousedown", handleOutsideClick);
});
</script>
