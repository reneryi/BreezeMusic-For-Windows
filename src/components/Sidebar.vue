<script setup lang="ts">
import { useAppStore } from "../stores/appStore";
import type { MainNavTab } from "../types";
import {
  Compass,
  Library,
  Search,
  Settings,
  Disc3,
  Crown,
  LogIn,
} from "lucide-vue-next";

const store = useAppStore();

const navItems: { id: MainNavTab; label: string; sub: string; icon: any }[] = [
  { id: "discover", label: "发现", sub: "概念推荐 · 随心听", icon: Compass },
  { id: "library", label: "我的", sub: "曲库 · 心流舱", icon: Library },
  { id: "search", label: "搜索", sub: "全曲库检索", icon: Search },
  { id: "settings", label: "设置", sub: "视觉与音频定制", icon: Settings },
];

function selectTab(tab: MainNavTab) {
  store.activeTab = tab;
  store.isNowPlayingOpen = false;
  if (tab === "discover") {
    store.discoverSubView = "home";
  }
}
</script>

<template>
  <aside class="sidebar-island">
    <!-- 顶部账号胶囊卡片（点击已登录区域可直达“我的”或打开 VIP 权益中心） -->
    <div class="sidebar-user-pod">
      <template v-if="store.isLoggedIn && store.session">
        <div
          class="user-pod-inner"
          title="点击查看 VIP 权益中心"
          @click="store.isVipCenterOpen = true"
        >
          <div class="avatar-ring">
            <img
              v-if="store.session.avatarUrl"
              :src="store.session.avatarUrl"
              alt="avatar"
              class="pod-avatar"
            />
            <div v-else class="pod-avatar-fallback">
              {{ store.session.nickname.slice(0, 1) }}
            </div>
          </div>
          <div class="pod-user-info">
            <div class="pod-name-row">
              <span class="pod-nickname">{{ store.session.nickname }}</span>
              <span v-if="store.vipStatus?.isVip" class="pod-vip-tag">VIP</span>
            </div>
            <div class="pod-sub-row">
              <Crown :size="12" class="gold-icon" />
              <span>
                {{
                  store.vipStatus?.todayClaimed
                    ? "今日 VIP 已签到"
                    : "点击领取今日 VIP"
                }}
              </span>
            </div>
          </div>
        </div>
      </template>

      <template v-else>
        <button class="guest-login-pod" @click="store.isLoginModalOpen = true">
          <div class="guest-icon-circle">
            <LogIn :size="16" />
          </div>
          <div class="guest-pod-text">
            <div class="guest-pod-title">登录酷狗概念版</div>
            <div class="guest-pod-sub">同步云歌单 · 每日领无损 VIP</div>
          </div>
        </button>
      </template>
    </div>

    <!-- 核心导航胶囊列表 -->
    <nav class="island-nav-list">
      <button
        v-for="item in navItems"
        :key="item.id"
        class="island-nav-item"
        :class="{
          active: store.activeTab === item.id && !store.isNowPlayingOpen,
        }"
        @click="selectTab(item.id)"
      >
        <div class="nav-icon-box">
          <component :is="item.icon" :size="19" />
        </div>
        <div class="nav-text-col">
          <span class="nav-main-label">{{ item.label }}</span>
          <span class="nav-sub-label">{{ item.sub }}</span>
        </div>
        <div class="nav-active-glow"></div>
      </button>
    </nav>

    <!-- 底部当前播放唱片舞台快捷切换卡 -->
    <div class="sidebar-stage-card">
      <button
        class="stage-portal-btn"
        :class="{ active: store.isNowPlayingOpen }"
        @click="store.isNowPlayingOpen = !store.isNowPlayingOpen"
      >
        <div class="mini-vinyl-disc" :class="{ spinning: store.isPlaying }">
          <img
            v-if="store.currentSong?.coverUrl"
            :src="store.currentSong.coverUrl"
            class="mini-vinyl-cover"
          />
          <Disc3 v-else :size="18" />
        </div>
        <div class="stage-portal-meta">
          <div class="stage-portal-title">
            {{ store.isNowPlayingOpen ? "收起歌词舞台" : "沉浸歌词舞台" }}
          </div>
          <div class="stage-portal-sub">
            {{ store.currentSong?.title || "左封面 · 右逐字流光歌词" }}
          </div>
        </div>
      </button>
    </div>
  </aside>
</template>
