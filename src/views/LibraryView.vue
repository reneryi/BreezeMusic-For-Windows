<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../stores/appStore";
import {
  Crown,
  Heart,
  History,
  Timer,
  Play,
  RefreshCw,
  LogIn,
  ChevronRight,
  Sparkles,
} from "lucide-vue-next";

const store = useAppStore();

const activeListSongs = computed(() => {
  if (store.libraryAssetTab === "liked") {
    return store.likedSongs;
  }
  if (store.libraryAssetTab === "recent") {
    return store.recentHistorySongs;
  }
  return store.playlistSongs;
});

const activeListTitle = computed(() => {
  if (store.libraryAssetTab === "liked") {
    return "我喜欢的音乐";
  }
  if (store.libraryAssetTab === "recent") {
    return "最近播放记录";
  }
  return store.selectedPlaylist?.title || "选择歌单";
});

function openLikedSongs() {
  store.libraryAssetTab = "liked";
  if (store.likedSongs.length === 0 && store.likedPlaylistMeta) {
    store.selectPlaylist(store.likedPlaylistMeta);
  }
}

function openRecentSongs() {
  store.libraryAssetTab = "recent";
  if (store.recentHistorySongs.length === 0) {
    store.loadRecentHistory();
  }
}

function formatDuration(ms: number): string {
  if (!ms || ms <= 0) return "03:30";
  const sec = Math.floor(ms / 1000);
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}
</script>

<template>
  <div class="page-scroll-canvas">
    <header class="discover-top-header">
      <div>
        <h1 class="discover-main-title">我的</h1>
        <div class="discover-greeting-line">
          <span class="greeting-dot"></span>
          <span>云端曲库与听歌等级实时同步</span>
        </div>
      </div>
    </header>

    <!-- 未登录引导卡片 -->
    <div v-if="!store.isLoggedIn" class="unauth-bento-card">
      <div class="unauth-bento-inner">
        <h3>登录酷狗概念版账号</h3>
        <p>
          支持酷狗 App 扫码登录与手机验证码登录。登录后自动同步「我喜欢的音乐」、云端歌单、听歌等级时长，并解锁每日免费概念版 VIP。
        </p>
        <button class="primary-btn" @click="store.isLoginModalOpen = true">
          <LogIn :size="16" />
          <span>立即登录账号</span>
        </button>
      </div>
    </div>

    <template v-else>
      <!-- 顶部 Bento 矩阵（复刻 Android 演示视频 00:06 布局） -->
      <section class="library-bento-matrix">
        <!-- 左上：用户信息 + Lv.11 累计听歌 X 小时 Y 分 + VIP 权益中心入口 -->
        <div class="lib-user-bento-col">
          <div class="lib-profile-card">
            <div class="lib-prof-top">
              <img
                v-if="store.session?.avatarUrl"
                :src="store.session.avatarUrl"
                class="lib-prof-avatar"
              />
              <div v-else class="lib-prof-avatar-fallback">
                {{ store.session?.nickname?.slice(0, 1) || "U" }}
              </div>
              <div class="lib-prof-meta">
                <div class="lib-prof-name-row">
                  <span class="lib-prof-name">
                    {{ store.profile?.nickname || store.session?.nickname }}
                  </span>
                  <span v-if="store.vipStatus?.isVip" class="gold-vip-pill">
                    VIP
                  </span>
                </div>
                <div class="lib-prof-sub">云端曲库与听歌等级已同步</div>
              </div>
              <button
                class="ghost-pill-btn"
                @click="store.isLoginModalOpen = true"
              >
                切换账号
              </button>
            </div>

            <!-- 严格对齐 Android 端视频 00:06: Lv.11 · 累计听歌 3689 小时 8 分 -->
            <div class="lib-grade-bar">
              <div class="grade-left">
                <span class="grade-lv-badge">
                  ✦ Lv.{{ store.profile?.userGrade || 11 }}
                </span>
                <span class="grade-duration-text">
                  累计听歌
                  <strong>
                    {{
                      store.profile?.listenDurationFormatted || "正在同步云端..."
                    }}
                  </strong>
                </span>
              </div>
              <button
                class="sync-text-btn"
                title="同步云端等级与歌单"
                @click="store.refreshAccountAndVip()"
              >
                <RefreshCw
                  :size="13"
                  :class="{ 'spin-icon': store.isLoadingLibrary }"
                />
                <span>同步云端</span>
              </button>
            </div>
          </div>

          <!-- 鎏金 VIP 权益中心入口卡 -->
          <div
            class="lib-vip-entry-card"
            @click="store.isVipCenterOpen = true"
          >
            <div class="vip-entry-left">
              <div class="vip-crown-box">
                <Crown :size="20" />
              </div>
              <div>
                <div class="vip-entry-title">VIP 权益中心</div>
                <div class="vip-entry-sub">每日签到 · 畅享无损</div>
              </div>
            </div>
            <div class="vip-entry-right">
              <span>
                {{ store.vipStatus?.todayClaimed ? "今日已领" : "立即领取" }}
              </span>
              <ChevronRight :size="16" />
            </div>
          </div>
        </div>

        <!-- 右侧三张高定特色功能卡：「我喜欢的音乐」+「最近播放」+「概念投喂 · Focus Pod」 -->
        <div class="lib-feature-bento-col">
          <div class="lib-two-cards-row">
            <!-- 红心暗纹卡：我喜欢的音乐 -->
            <div
              class="lib-shortcut-card liked-card"
              :class="{ active: store.libraryAssetTab === 'liked' }"
              @click="openLikedSongs"
            >
              <div class="sc-top">
                <div class="sc-icon-circle red">
                  <Heart :size="18" />
                </div>
                <ChevronRight :size="16" class="sc-arrow" />
              </div>
              <div class="sc-bottom">
                <div class="sc-title">我喜欢的音乐</div>
                <div class="sc-count">
                  {{
                    store.likedPlaylistMeta?.trackCount ||
                    store.likedSongs.length
                  }}
                  首歌曲
                </div>
              </div>
            </div>

            <!-- 青蓝暗纹卡：最近播放 -->
            <div
              class="lib-shortcut-card recent-card"
              :class="{ active: store.libraryAssetTab === 'recent' }"
              @click="openRecentSongs"
            >
              <div class="sc-top">
                <div class="sc-icon-circle cyan">
                  <History :size="18" />
                </div>
                <ChevronRight :size="16" class="sc-arrow" />
              </div>
              <div class="sc-bottom">
                <div class="sc-title">最近播放</div>
                <div class="sc-count">
                  {{ store.recentHistorySongs.length }} 首记录
                </div>
              </div>
            </div>
          </div>

          <!-- 幻紫流光卡：概念投喂 · Focus Pod 心流舱 -->
          <div
            class="lib-focus-pod-card"
            @click="store.isFocusPodOpen = true"
          >
            <div class="fp-left">
              <div class="sc-icon-circle violet">
                <Timer :size="19" />
              </div>
              <div>
                <div class="fp-title">概念投喂 · Focus Pod</div>
                <div class="fp-sub">专注计时与环境伴音舱</div>
              </div>
            </div>
            <div class="fp-action">
              <span>开启心流</span>
              <ChevronRight :size="16" />
            </div>
          </div>
        </div>
      </section>

      <!-- 下方：歌单资产 Tab 与双栏曲目浏览区 -->
      <section class="library-explorer-section">
        <div class="explorer-tabs-bar">
          <button
            class="exp-tab"
            :class="{ active: store.libraryAssetTab === 'created' }"
            @click="store.libraryAssetTab = 'created'"
          >
            自建 <span>{{ store.createdPlaylists.length }}</span>
          </button>
          <button
            class="exp-tab"
            :class="{ active: store.libraryAssetTab === 'collected' }"
            @click="store.libraryAssetTab = 'collected'"
          >
            收藏 <span>{{ store.collectedPlaylists.length }}</span>
          </button>
          <button
            class="exp-tab"
            :class="{ active: store.libraryAssetTab === 'liked' }"
            @click="openLikedSongs"
          >
            我喜欢
            <span>
              {{
                store.likedPlaylistMeta?.trackCount || store.likedSongs.length
              }}
            </span>
          </button>
          <button
            class="exp-tab"
            :class="{ active: store.libraryAssetTab === 'recent' }"
            @click="openRecentSongs"
          >
            最近 <span>{{ store.recentHistorySongs.length }}</span>
          </button>
        </div>

        <div class="explorer-workspace">
          <!-- 左栏：歌单卡片列表（当处于自建或收藏 Tab 时展示） -->
          <div
            v-if="
              store.libraryAssetTab === 'created' ||
              store.libraryAssetTab === 'collected'
            "
            class="explorer-playlists-col"
          >
            <button
              v-for="pl in store.libraryAssetTab === 'created'
                ? store.createdPlaylists
                : store.collectedPlaylists"
              :key="pl.id"
              class="exp-playlist-item"
              :class="{ active: store.selectedPlaylist?.id === pl.id }"
              @click="store.selectPlaylist(pl)"
            >
              <img
                v-if="pl.coverUrl"
                :src="pl.coverUrl"
                class="exp-pl-cover"
                loading="lazy"
              />
              <div v-else class="exp-pl-cover-fallback">♪</div>
              <div class="exp-pl-info">
                <div class="exp-pl-title">{{ pl.title }}</div>
                <div class="exp-pl-sub">{{ pl.trackCount }} 首歌曲</div>
              </div>
            </button>
          </div>

          <!-- 右栏：当前选中歌单 / 我喜欢 / 最近播放的曲目流 -->
          <div class="explorer-tracks-col">
            <div class="exp-tracks-header">
              <div>
                <h3 class="exp-tracks-title">{{ activeListTitle }}</h3>
                <span class="exp-tracks-count">
                  共 {{ activeListSongs.length }} 首曲目
                </span>
              </div>
              <button
                v-if="activeListSongs.length > 0"
                class="primary-btn"
                @click="
                  store.playSongFromList(activeListSongs[0], activeListSongs)
                "
              >
                <Play :size="15" />
                <span>播放全部</span>
              </button>
            </div>

            <div v-if="store.isLoadingLibrary" class="loading-box">
              正在同步云端曲目与概念版无损音源规格...
            </div>

            <div
              v-else-if="activeListSongs.length === 0"
              class="empty-box"
            >
              <Sparkles :size="30" />
              <p>当前列表暂无歌曲，请从左侧选择有曲目的歌单</p>
            </div>

            <div v-else class="modern-track-list">
              <div
                v-for="(song, idx) in activeListSongs"
                :key="song.hash || song.id"
                class="modern-track-item"
                :class="{ playing: store.currentSong?.hash === song.hash }"
                @click="store.playSongFromList(song, activeListSongs)"
              >
                <div class="t-num">
                  <span
                    v-if="store.currentSong?.hash === song.hash"
                    class="t-playing"
                  >
                    ▶
                  </span>
                  <span v-else>{{ String(idx + 1).padStart(2, "0") }}</span>
                </div>
                <img
                  v-if="song.coverUrl"
                  :src="song.coverUrl"
                  class="t-cover"
                  loading="lazy"
                />
                <div v-else class="t-cover-placeholder">♪</div>
                <div class="t-main">
                  <div class="t-title">{{ song.title }}</div>
                  <div class="t-sub">
                    <span v-if="song.isHiRes" class="mini-badge hires">
                      Hi-Res
                    </span>
                    <span v-else-if="song.isLossless" class="mini-badge sq">
                      SQ
                    </span>
                    <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                    <span>{{ song.artist }}</span>
                    <span v-if="song.album" class="t-album">
                      · {{ song.album }}
                    </span>
                  </div>
                </div>
                <div class="t-duration">
                  {{ formatDuration(song.durationMs) }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>
    </template>
  </div>
</template>
