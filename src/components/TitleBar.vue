<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore } from "../stores/appStore";
import type { MainNavTab } from "../types";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();
const appWindow = getCurrentWindow();
const searchInputRef = ref<HTMLInputElement | null>(null);

const skylineTabs: { id: MainNavTab; label: string; icon: string }[] = [
  { id: "discover", label: "发现", icon: "compass" },
  { id: "library", label: "我的", icon: "library" },
  { id: "radio", label: "调频台", icon: "radio" },
  { id: "settings", label: "设置", icon: "settings" },
];

function selectSkylineTab(tab: MainNavTab) {
  store.activeTab = tab;
  store.isNowPlayingOpen = false;
  store.isSearchPaletteOpen = false;
  if (tab === "discover") {
    store.discoverSubView = "home";
  }
  if (tab === "library" && store.libraryDetailMode !== "playlist") {
    store.libraryDetailMode = "playlist";
  }
}

function handleQuickBack() {
  if (store.isNowPlayingOpen) {
    store.isNowPlayingOpen = false;
    return;
  }
  if (store.activeTab === "discover" && store.discoverSubView !== "home") {
    store.discoverSubView = "home";
    return;
  }
  if (
    store.activeTab === "library" &&
    store.libraryDetailMode !== "playlist"
  ) {
    store.libraryDetailMode = "playlist";
    return;
  }
  if (store.activeTab === "search") {
    store.activeTab = "discover";
  }
}

function onSearchSubmit() {
  if (store.searchKeyword.trim()) {
    store.executeSearch(store.searchKeyword);
  }
}

function onGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
    e.preventDefault();
    store.isSearchPaletteOpen = !store.isSearchPaletteOpen;
    if (store.isSearchPaletteOpen) {
      setTimeout(() => searchInputRef.value?.focus(), 60);
    }
  } else if (e.key === "Escape") {
    store.isSearchPaletteOpen = false;
    store.closeSongContextMenu();
  }
}

onMounted(() => {
  window.addEventListener("keydown", onGlobalKeydown);
});
onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
});

async function minimizeWindow() {
  try {
    await appWindow.minimize();
  } catch {}
}

async function toggleMaximizeWindow() {
  try {
    await appWindow.toggleMaximize();
  } catch {}
}

async function closeWindow() {
  try {
    await appWindow.close();
  } catch {}
}
</script>

<template>
  <header class="skyline-header" data-tauri-drag-region>
    <!-- 左区：品牌圆润徽标 + 智能返回胶囊 -->
    <div class="skyline-left" data-tauri-drag-region>
      <div
        class="skyline-brand"
        data-tauri-drag-region
        @click="selectSkylineTab('discover')"
      >
        <div class="brand-orb">
          <PlumpIcon name="disc" :size="20" color="#ffffff" accent="#00e5ff" />
        </div>
        <div class="brand-titles" data-tauri-drag-region>
          <span class="brand-main">微风音乐</span>
          <span class="brand-sub">Concept · 3116</span>
        </div>
      </div>

      <button
        v-if="
          store.isNowPlayingOpen ||
          (store.activeTab === 'discover' && store.discoverSubView !== 'home') ||
          (store.activeTab === 'library' &&
            store.libraryDetailMode !== 'playlist') ||
          store.activeTab === 'search'
        "
        class="skyline-back-pill"
        title="返回上一视图"
        @click="handleQuickBack"
      >
        <PlumpIcon name="back" :size="16" />
        <span>返回</span>
      </button>
    </div>

    <!-- 中区：方案二核心 —— 顶部天际线导航胶囊 + Ctrl+K 灵动搜索舱 -->
    <div class="skyline-center">
      <nav class="skyline-nav-capsule">
        <button
          v-for="item in skylineTabs"
          :key="item.id"
          class="skyline-nav-btn"
          :class="{
            active: store.activeTab === item.id && !store.isNowPlayingOpen,
          }"
          @click="selectSkylineTab(item.id)"
        >
          <PlumpIcon
            :name="item.icon"
            :size="17"
            :color="
              store.activeTab === item.id && !store.isNowPlayingOpen
                ? '#ffffff'
                : 'currentColor'
            "
            :accent="
              store.activeTab === item.id && !store.isNowPlayingOpen
                ? '#00e5ff'
                : 'var(--primary)'
            "
          />
          <span>{{ item.label }}</span>
        </button>
      </nav>

      <!-- 天际线搜索触发器与内嵌搜索框 -->
      <div class="skyline-search-wrap">
        <div
          class="skyline-search-pill"
          :class="{ focused: store.isSearchPaletteOpen }"
          @click="store.isSearchPaletteOpen = true"
        >
          <PlumpIcon
            name="search"
            :size="16"
            color="var(--text-secondary)"
            accent="var(--cyan)"
          />
          <input
            ref="searchInputRef"
            v-model="store.searchKeyword"
            type="text"
            class="skyline-search-input"
            placeholder="搜索歌曲、歌手、歌单、专辑..."
            @focus="store.isSearchPaletteOpen = true"
            @keydown.enter="onSearchSubmit"
          />
          <span class="search-kbd-tag">Ctrl K</span>
        </div>

        <!-- 下拉式热搜榜与历史记录指令浮层（无边框色阶浮层） -->
        <transition name="pop-spring">
          <div
            v-if="store.isSearchPaletteOpen"
            class="skyline-search-dropdown"
            @click.stop
          >
            <div v-if="store.searchHistory.length > 0" class="sd-section">
              <div class="sd-head">
                <span>最近搜索</span>
                <button class="sd-clear-btn" @click="store.clearSearchHistory()">
                  清空
                </button>
              </div>
              <div class="sd-chips">
                <button
                  v-for="h in store.searchHistory"
                  :key="h"
                  class="sd-chip"
                  @click="store.executeSearch(h)"
                >
                  {{ h }}
                </button>
              </div>
            </div>

            <div class="sd-section">
              <div class="sd-head">
                <span>酷狗概念版 · 实时热搜榜</span>
              </div>
              <div class="sd-hot-grid">
                <button
                  v-for="(kw, idx) in store.hotSearchKeywords.slice(0, 12)"
                  :key="kw"
                  class="sd-hot-item"
                  @click="store.executeSearch(kw)"
                >
                  <span class="sd-hot-rank" :class="{ top: idx < 3 }">
                    {{ idx + 1 }}
                  </span>
                  <span class="sd-hot-text">{{ kw }}</span>
                </button>
              </div>
            </div>
          </div>
        </transition>
      </div>
    </div>

    <!-- 右区：流光数字身份舱（头像 + 等级 + 听歌时长 + VIP 权益中心入口）+ 窗口控制 -->
    <div class="skyline-right">
      <template v-if="store.isLoggedIn && store.session">
        <div class="skyline-identity-capsule">
          <!-- 点击左半部进入我的音乐/切换账号，悬停查看听歌时长同步详情 -->
          <div
            class="identity-user-part"
            :title="`UID: ${store.session.userId} · 点击前往我的音乐资产 / 同步云端等级`"
            @click="
              store.activeTab = 'library';
              store.isNowPlayingOpen = false;
              store.refreshAccountAndVip(false);
            "
          >
            <div class="identity-avatar-wrap">
              <img
                v-if="store.session.avatarUrl"
                :src="store.session.avatarUrl"
                class="identity-avatar"
                alt="avatar"
              />
              <div v-else class="identity-avatar-fallback">
                {{ store.session.nickname.slice(0, 1) }}
              </div>
              <span class="cscc-live-dot" title="CSCC 听歌时长实时心跳已连接"></span>
            </div>

            <div class="identity-meta">
              <div class="identity-top-row">
                <span class="identity-name">{{ store.session.nickname }}</span>
                <span class="identity-lv-pill">
                  ✦ Lv.{{ store.profile?.userGrade || 11 }}
                </span>
              </div>
              <div class="identity-duration-row">
                {{ store.profile?.listenDurationFormatted || "累计时长同步中" }}
              </div>
            </div>
          </div>

          <!-- 右半部：鎏金 VIP 权益中心直达入口 -->
          <button
            class="identity-vip-trigger"
            :class="{ claimed: store.vipStatus?.todayClaimed }"
            title="打开概念版 VIP 权益与签到日历中心"
            @click="store.isVipCenterOpen = true"
          >
            <PlumpIcon name="crown" :size="15" color="#ffd54f" />
            <span>
              {{ store.vipStatus?.todayClaimed ? "VIP 已签" : "领今日 VIP" }}
            </span>
          </button>
        </div>
      </template>

      <template v-else>
        <button
          class="skyline-login-pill"
          @click="store.isLoginModalOpen = true"
        >
          <PlumpIcon name="crown" :size="15" color="#ffd54f" />
          <span>登录概念版 · 领无损 VIP</span>
        </button>
      </template>

      <!-- 软润无框窗口控制按钮 -->
      <div class="skyline-win-controls">
        <button class="win-orb-btn" title="最小化" @click="minimizeWindow">
          <span class="win-glyph-line"></span>
        </button>
        <button
          class="win-orb-btn"
          title="最大化 / 还原"
          @click="toggleMaximizeWindow"
        >
          <span class="win-glyph-box"></span>
        </button>
        <button
          class="win-orb-btn close"
          title="关闭窗口"
          @click="closeWindow"
        >
          <PlumpIcon name="close" :size="14" />
        </button>
      </div>
    </div>
  </header>
</template>
