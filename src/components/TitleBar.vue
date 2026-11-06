<template>
  <!-- 顶层全屏无框悬浮交互层：居中精简五合一导航胶囊 + 右上角主题与窗口控制胶囊 -->
  <div class="skyline-floating-overlay">
    <!-- 顶部边缘拖拽热区 -->
    <div class="skyline-top-drag-strip" data-tauri-drag-region></div>

    <!-- 全屏播放舞台专用收起按钮（其余二级/详情页面均使用页内自带返回按钮，绝不遮挡左上角时段文案） -->
    <button
      v-if="store.isNowPlayingOpen"
      class="skyline-np-floating-back"
      title="收起播放页 (Esc)"
      @click="store.isNowPlayingOpen = false"
    >
      <PlumpIcon name="chevron-down" :size="18" />
      <span>收起舞台</span>
    </button>

    <!-- 居中悬浮岛：五合一圆润饱满图标导航胶囊（仅在五个主页面显示，进入二级详情/播放页自然上移收起，返回时自然下移浮现） -->
    <Transition name="skyline-nav-float">
      <div v-if="isMainNavVisible" class="skyline-floating-center">
        <nav class="skyline-nav-capsule" aria-label="主导航">
          <button
            v-for="item in navItems"
            :key="item.id"
            type="button"
            class="skyline-nav-btn"
            :class="{
              active: isNavItemActive(item.id),
              'icon-only': !isNavItemActive(item.id),
            }"
            :aria-label="item.label"
            :aria-current="isNavItemActive(item.id) ? 'page' : undefined"
            :title="item.hint"
            @click="handleNavClick(item.id)"
            @dblclick.stop="handleNavDblClick(item.id)"
          >
            <PlumpIcon
              :name="item.icon"
              :size="18"
              :color="isNavItemActive(item.id) ? '#ffffff' : 'currentColor'"
              :accent="
                isNavItemActive(item.id)
                  ? 'rgba(255, 255, 255, 0.88)'
                  : store.resolvedColorScheme === 'watercolor_light'
                  ? '#2b8cff'
                  : 'var(--cyan, #00e5ff)'
              "
            />
            <span v-if="isNavItemActive(item.id)" class="skyline-nav-label">
              {{ item.label }}
            </span>
          </button>
        </nav>
      </div>
    </Transition>

    <!-- 右上角悬浮区：明暗切换 + 窗口控制按钮胶囊 -->
    <div class="skyline-floating-right">
      <div class="skyline-win-floating-capsule">
        <button
          type="button"
          class="win-capsule-btn theme-btn"
          :title="themeBtnTitle"
          @click="toggleColorScheme"
        >
          <PlumpIcon
            :name="
              store.resolvedColorScheme === 'watercolor_light' ? 'moon' : 'sun'
            "
            :size="16"
          />
        </button>

        <span class="win-capsule-divider"></span>

        <button
          type="button"
          class="win-capsule-btn"
          title="最小化"
          @click="minimizeWin"
        >
          <PlumpIcon name="minimize" :size="14" />
        </button>
        <button
          type="button"
          class="win-capsule-btn"
          title="最大化 / 还原"
          @click="toggleMaximizeWin"
        >
          <PlumpIcon name="maximize" :size="14" />
        </button>
        <button
          type="button"
          class="win-capsule-btn close"
          title="关闭窗口"
          @click="closeWin"
        >
          <PlumpIcon name="close" :size="14" />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore, type MainTab } from "../stores/appStore";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

const isMainNavVisible = computed(() => {
  if (store.isNowPlayingOpen) {
    return false;
  }
  if (store.activeTab === "discover") {
    return (
      store.discoverSubView !== "new_songs" &&
      store.discoverSubView !== "rank" &&
      store.discoverSubView !== "radio"
    );
  }
  return (
    store.activeTab === "search" ||
    store.activeTab === "library" ||
    store.activeTab === "tuner" ||
    store.activeTab === "radio" ||
    store.activeTab === "settings"
  );
});

const navItems: Array<{
  id: MainTab;
  label: string;
  hint: string;
  icon:
    | "nav-discover"
    | "nav-search"
    | "nav-library"
    | "nav-tuner"
    | "nav-settings";
}> = [
  {
    id: "discover",
    label: "发现",
    hint: "发现 · 双击整体刷新推荐内容",
    icon: "nav-discover",
  },
  {
    id: "search",
    label: "搜索",
    hint: "搜索 (Ctrl+K) · 实时热搜与全量曲库检索",
    icon: "nav-search",
  },
  {
    id: "library",
    label: "我的",
    hint: "我的音乐 · 双击同步云端音乐资产",
    icon: "nav-library",
  },
  {
    id: "tuner",
    label: "调音台",
    hint: "调音台 · 母带声学均衡器与蝰蛇音效工作站",
    icon: "nav-tuner",
  },
  {
    id: "settings",
    label: "设置",
    hint: "设置 · 外观主题、播放偏好与下载设置",
    icon: "nav-settings",
  },
];

function isNavItemActive(id: MainTab): boolean {
  if (store.isNowPlayingOpen) return false;
  if (id === "tuner") {
    return store.activeTab === "tuner" || store.activeTab === "radio";
  }
  return store.activeTab === id;
}

function handleNavClick(id: MainTab) {
  store.isNowPlayingOpen = false;
  if (id === "discover" && store.activeTab === "discover") {
    store.discoverSubView = "home";
  } else if (id === "search") {
    if (store.activeTab === "search" && store.searchViewMode === "results") {
      store.openSearchLanding(false);
    } else if (!store.searchKeyword.trim()) {
      store.searchViewMode = "landing";
      store.loadSearchDiscovery(false);
    }
    store.activeTab = "search";
    nextTick(() => {
      const input = document.getElementById(
        "independent-search-input"
      ) as HTMLInputElement | null;
      if (input && store.searchViewMode === "landing") {
        input.focus();
      }
    });
    return;
  }
  store.activeTab = id;
}

function handleNavDblClick(id: MainTab) {
  store.isNowPlayingOpen = false;
  store.activeTab = id;
  if (id === "discover") {
    store.discoverSubView = "home";
    store.refreshDiscoverAll();
  } else if (id === "search") {
    store.openSearchLanding(false);
    store.loadSearchDiscovery(true).then(() => {
      store.notify("实时热搜歌曲与热门歌手已刷新 ✨");
    });
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

const themeBtnTitle = computed(() => {
  if (store.colorScheme === "system") {
    return `跟随系统自动切换（当前：${
      store.resolvedColorScheme === "watercolor_light" ? "水彩浸润" : "幽邃微蓝"
    }）`;
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
</script>
