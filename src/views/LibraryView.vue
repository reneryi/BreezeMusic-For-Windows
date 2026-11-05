<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import type { FollowedArtist, Playlist, PurchasedAlbumItem, Song } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const galleryFilterKw = ref("");
const isRefreshingAssets = ref(false);

// 新建歌单、编辑歌单与外部导入弹窗状态
const showCreateModal = ref(false);
const newPlaylistName = ref("");

const showImportModal = ref(false);
const importInputText = ref("");

const showEditModal = ref(false);
const editingPlaylist = ref<Playlist | null>(null);
const editTitle = ref("");
const editIntro = ref("");

const levelNum = computed(() => {
  return store.profile?.userGrade || store.profile?.userLevel || 1;
});

const listenHoursNumber = computed(() => {
  const sec = store.profile?.listenDurationSec || 0;
  const totalMin = Math.max(store.profile?.listenDurationMin || 0, Math.floor(sec / 60));
  return Math.floor(totalMin / 60);
});

const listenRemainderMins = computed(() => {
  const sec = store.profile?.listenDurationSec || 0;
  const totalMin = Math.max(store.profile?.listenDurationMin || 0, Math.floor(sec / 60));
  return totalMin % 60;
});

const levelProgressPercent = computed(() => {
  const p = store.profile?.levelProgressPct;
  if (typeof p === "number" && p > 0) return Math.min(100, Math.max(6, p));
  return 68;
});

const nextLevelHintText = computed(() => {
  const diff = store.profile?.nextLevelRemainMin || 0;
  if (diff > 0) {
    const h = Math.floor(diff / 60);
    const m = diff % 60;
    return h > 0 ? `距 Lv.${levelNum.value + 1} 需 ${h}h ${m}m` : `距 Lv.${levelNum.value + 1} 需 ${m} 分钟`;
  }
  return `今日听满 30 分钟稳定升级`;
});

const likedTotalCount = computed(() => {
  return Math.max(
    store.likedSongs.length,
    store.likedPlaylistMeta?.trackCount || 0
  );
});

const likedHiResCount = computed(() => {
  return store.likedSongs.filter(
    (s) => s.hasHiRes || s.hasSq || s.isLossless || s.isHiRes
  ).length;
});

const likedDurationText = computed(() => {
  return formatSongsTotalHours(store.likedSongs);
});

function formatSongsTotalHours(songs: Song[]): string {
  if (!songs.length) return "双端云同步";
  const totalSec = songs.reduce((acc, s) => {
    const sec = s.durationSeconds || Math.floor((s.durationMs || 0) / 1000);
    return acc + (Number.isFinite(sec) ? sec : 0);
  }, 0);
  const hours = Math.floor(totalSec / 3600);
  const mins = Math.max(1, Math.floor((totalSec % 3600) / 60));
  return hours > 0 ? `约 ${hours} 小时 ${mins} 分` : `约 ${mins} 分钟`;
}

const assetTabs = computed(() => [
  {
    id: "created" as const,
    label: "自建歌单",
    count: store.createdPlaylists.length,
    color: "var(--primary)",
  },
  {
    id: "collected" as const,
    label: "收藏歌单",
    count: store.collectedPlaylists.length,
    color: "var(--accent-coral)",
  },
  {
    id: "artists" as const,
    label: "关注歌手",
    count: store.followedArtists.length,
    color: "var(--accent-mint)",
  },
  {
    id: "purchased" as const,
    label: "已购内容",
    count:
      store.purchasedAlbums.length + (store.purchasedSongs.length > 0 ? 1 : 0),
    color: "var(--accent-gold)",
  },
]);

const filteredPlaylists = computed<Playlist[]>(() => {
  const base =
    store.libraryAssetTab === "collected"
      ? store.collectedPlaylists
      : store.createdPlaylists;
  const kw = galleryFilterKw.value.trim().toLowerCase();
  if (!kw) return base;
  return base.filter(
    (p) =>
      (p.title || p.name || "").toLowerCase().includes(kw) ||
      (p.description || p.intro || "").toLowerCase().includes(kw) ||
      (p.creator || "").toLowerCase().includes(kw)
  );
});

const filteredArtists = computed<FollowedArtist[]>(() => {
  const kw = galleryFilterKw.value.trim().toLowerCase();
  if (!kw) return store.followedArtists;
  return store.followedArtists.filter((a) =>
    (a.name || "").toLowerCase().includes(kw)
  );
});

const filteredPurchasedAlbums = computed<PurchasedAlbumItem[]>(() => {
  const kw = galleryFilterKw.value.trim().toLowerCase();
  if (!kw) return store.purchasedAlbums;
  return store.purchasedAlbums.filter(
    (a) =>
      (a.albumName || "").toLowerCase().includes(kw) ||
      (a.singerName || a.artistName || "").toLowerCase().includes(kw)
  );
});

function handleSelectAssetTab(
  tabId: "created" | "collected" | "artists" | "purchased"
) {
  store.libraryAssetTab = tabId;
  if (tabId === "artists" && store.followedArtists.length === 0) {
    store.loadFollowedArtists();
  } else if (
    tabId === "purchased" &&
    store.purchasedAlbums.length === 0 &&
    store.purchasedSongs.length === 0
  ) {
    store.loadPurchasedAssets();
  }
}

async function refreshLibraryAssets() {
  if (!store.isLoggedIn) {
    store.isLoginModalOpen = true;
    return;
  }
  if (isRefreshingAssets.value) return;
  isRefreshingAssets.value = true;
  try {
    await Promise.all([
      store.loadUserPlaylists(),
      store.loadRecentHistory(),
      store.loadFollowedArtists(),
      store.loadPurchasedAssets(),
    ]);
    store.notify("已全面刷新我的音乐库与已购资产 ✨");
  } finally {
    isRefreshingAssets.value = false;
  }
}

function playAllLiked() {
  if (store.likedSongs.length > 0) {
    store.playSongFromList(
      store.likedSongs[0],
      store.likedSongs,
      "我喜欢的音乐"
    );
  } else {
    store.openLikedSongsDetail();
  }
}

function shufflePlayLiked() {
  if (store.likedSongs.length > 0) {
    store.playMode = "shuffle";
    const randIdx = Math.floor(Math.random() * store.likedSongs.length);
    store.playSongFromList(
      store.likedSongs[randIdx],
      store.likedSongs,
      "我喜欢的音乐 · 随机漫游"
    );
  } else {
    store.openLikedSongsDetail();
  }
}

function playAllRecent() {
  if (store.recentHistorySongs.length > 0) {
    store.playSongFromList(
      store.recentHistorySongs[0],
      store.recentHistorySongs,
      "最近播放"
    );
  } else {
    store.openRecentHistoryDetail();
  }
}

function playAllPurchasedSingles() {
  const songs = store.purchasedSongs.map((p) => p.song);
  if (songs.length > 0) {
    store.playSongFromList(songs[0], songs, "已购单曲合集");
  } else {
    store.openPurchasedSongsDetail();
  }
}

function openCreateModal() {
  if (!store.isLoggedIn) {
    store.isLoginModalOpen = true;
    return;
  }
  newPlaylistName.value = "";
  showCreateModal.value = true;
}

async function submitCreatePlaylist() {
  const name = newPlaylistName.value.trim();
  if (!name) return;
  showCreateModal.value = false;
  await store.createNewPlaylist(name);
}

function openImportModal() {
  if (!store.isLoggedIn) {
    store.isLoginModalOpen = true;
    return;
  }
  importInputText.value = "";
  showImportModal.value = true;
}

async function submitImportPlaylist() {
  const raw = importInputText.value.trim();
  if (!raw) return;
  showImportModal.value = false;
  await store.importExternalPlaylist(raw);
}

function openEditPlaylist(pl: Playlist) {
  editingPlaylist.value = pl;
  editTitle.value = pl.title || pl.name || "";
  editIntro.value = pl.description || pl.intro || "";
  showEditModal.value = true;
}

async function submitEditPlaylist() {
  if (!editingPlaylist.value || !editingPlaylist.value.listId) return;
  const title = editTitle.value.trim();
  if (!title) return;
  showEditModal.value = false;
  await store.updatePlaylistMeta(
    editingPlaylist.value.listId,
    title,
    editIntro.value.trim()
  );
}

function confirmDeletePlaylist(pl: Playlist) {
  if (window.confirm(`确定要删除自建歌单「${pl.title || pl.name}」吗？`)) {
    store.deletePlaylist(pl);
  }
}

function formatFansCount(cnt: number): string {
  if (!cnt || cnt <= 0) return "概念版音乐人";
  if (cnt >= 10000) return `${(cnt / 10000).toFixed(1)}w 粉丝`;
  return `${cnt} 粉丝`;
}

onMounted(() => {
  if (store.isLoggedIn) {
    if (store.userPlaylists.length === 0) {
      store.loadUserPlaylists();
    }
    if (store.followedArtists.length === 0) {
      store.loadFollowedArtists();
    }
    if (
      store.purchasedAlbums.length === 0 &&
      store.purchasedSongs.length === 0
    ) {
      store.loadPurchasedAssets();
    }
  }
  if (store.recentHistorySongs.length === 0) {
    store.loadRecentHistory();
  }
});
</script>

<template>
  <!-- 方案 B：12 栅格 5:4:3 杂志感不对称 Bento 首屏 + 100% 全宽无界陈列馆 -->
  <div class="library-bento-page" @contextmenu.prevent>
    <!-- ==================== 第一层：12 栅格 5:4:3 杂志感 Bento 首屏 ==================== -->
    <section class="library-bento-hero-grid">
      <!-- 左 5 栅格：个人身份、双水位线听歌等级与内嵌概念版 VIP 权益中心 -->
      <article class="bento-card bento-identity-card">
        <div class="bento-id-top">
          <div
            class="bento-id-user"
            @click="
              store.isLoggedIn
                ? (store.isAccountManagerOpen = true)
                : (store.isLoginModalOpen = true)
            "
          >
            <div class="bento-avatar-wrap">
              <img
                v-if="store.profile?.avatarUrl || store.session?.avatarUrl"
                :src="store.profile?.avatarUrl || store.session?.avatarUrl"
                class="bento-avatar-img"
                draggable="false"
                alt="avatar"
              />
              <div v-else class="bento-avatar-fallback">
                {{
                  (
                    store.profile?.nickname ||
                    store.session?.nickname ||
                    "微"
                  ).slice(0, 1)
                }}
              </div>
              <span class="bento-online-dot"></span>
            </div>

            <div class="bento-user-meta">
              <div class="bento-user-name-row">
                <h2 class="bento-user-name">
                  {{
                    store.isLoggedIn
                      ? store.profile?.nickname ||
                        store.session?.nickname ||
                        "微风乐迷"
                      : "登录酷狗概念版"
                  }}
                </h2>
                <span v-if="store.isLoggedIn" class="bento-lv-badge">
                  Lv.{{ levelNum }}
                </span>
                <span
                  v-if="store.isLoggedIn"
                  class="bento-vip-badge"
                  :class="{
                    active:
                      store.vipStatus?.svipActive ||
                      store.vipStatus?.tvipActive ||
                      store.profile?.isVip,
                  }"
                >
                  {{
                    store.vipStatus?.svipActive
                      ? "概念 SVIP"
                      : store.vipStatus?.tvipActive || store.profile?.isVip
                      ? "畅听 VIP"
                      : "概念版"
                  }}
                </span>
              </div>
              <div class="bento-user-sub">
                <template v-if="store.isLoggedIn">
                  UID: {{ store.session?.userId || store.profile?.userId }}
                  <span v-if="store.savedAccounts.length > 1">
                    · 已绑定 {{ store.savedAccounts.length }} 个账号
                  </span>
                </template>
                <template v-else>
                  扫码或验证码一键登录，同步云端红心曲库与无损 VIP 权益
                </template>
              </div>
            </div>
          </div>

          <div class="bento-id-actions">
            <button
              class="bento-pill-btn"
              title="切换账号或管理会话"
              @click="
                store.isLoggedIn
                  ? (store.isAccountManagerOpen = true)
                  : (store.isLoginModalOpen = true)
              "
            >
              <span>{{ store.isLoggedIn ? "账号管理 ›" : "立即登录 ›" }}</span>
            </button>
            <button
              v-if="store.isLoggedIn"
              class="bento-pill-btn sync-btn"
              :disabled="store.isSyncingListeningGrade"
              title="双水位线上报并校准云端听歌时长与等级"
              @click="store.syncListeningGradeCloud(true)"
            >
              <PlumpIcon
                name="refresh"
                :size="13"
                :class="{ 'spin-anim': store.isSyncingListeningGrade }"
              />
              <span>{{
                store.isSyncingListeningGrade ? "同步中" : "同步云端"
              }}</span>
            </button>
          </div>
        </div>

        <!-- 中段：听歌时长大数字与等级进度条 -->
        <div class="bento-listen-box">
          <div class="bento-listen-left">
            <span class="bento-listen-label">累计听歌时长 · 权威双水位线</span>
            <div class="bento-listen-value">
              <strong>{{ listenHoursNumber }}</strong>
              <span class="unit">小时</span>
              <strong>{{ listenRemainderMins }}</strong>
              <span class="unit">分钟</span>
            </div>
          </div>

          <div class="bento-listen-right">
            <div class="bento-level-meta">
              <span>Lv.{{ levelNum }} → Lv.{{ levelNum + 1 }}</span>
              <span class="hint">{{ nextLevelHintText }}</span>
            </div>
            <div class="bento-level-track">
              <div
                class="bento-level-fill"
                :style="{ width: `${levelProgressPercent}%` }"
              ></div>
            </div>
          </div>
        </div>

        <!-- 底部内嵌：香槟流金 / 黑金 VIP 权益与每日签到横幅 -->
        <div
          class="bento-vip-strip"
          @click="store.isVipCenterOpen = true"
        >
          <div class="bento-vip-left">
            <div class="bento-vip-orb">
              <PlumpIcon name="crown" :size="16" color="#FFD54F" />
            </div>
            <div class="bento-vip-copy">
              <div class="bento-vip-title">
                概念版 VIP 权益中心 · 每日签到免费领畅听 VIP
              </div>
              <div class="bento-vip-sub">
                {{
                  store.vipStatus?.svipExpiry
                    ? `有效期至 ${store.vipStatus.svipExpiry.slice(0, 10)} · 点击查看月度签到日历与网关诊断`
                    : store.vipStatus?.tvipExpiry
                    ? `有效期至 ${store.vipStatus.tvipExpiry.slice(0, 10)} · 点击查看月度签到日历与网关诊断`
                    : "每日签到自动续期概念版无损与 Hi-Res 畅听特权"
                }}
              </div>
            </div>
          </div>

          <button
            class="bento-vip-cta"
            :class="{ claimed: store.vipStatus?.todayClaimed }"
            :disabled="store.isClaimingVip"
            @click.stop="
              store.vipStatus?.todayClaimed
                ? (store.isVipCenterOpen = true)
                : store.claimVipDaily(false)
            "
          >
            {{
              store.isClaimingVip
                ? "签到中..."
                : store.vipStatus?.todayClaimed
                ? "今日已签 · 详情 ›"
                : "立即签到领 VIP"
            }}
          </button>
        </div>
      </article>

      <!-- 中 4 栅格：我喜欢的音乐（珊瑚粉水彩 / 暗夜瑰红渐变主视觉卡） -->
      <article
        class="bento-card bento-liked-card"
        @click="store.openLikedSongsDetail()"
      >
        <div class="bento-liked-top">
          <div class="bento-liked-main">
            <div class="bento-cover-stack">
              <img
                v-if="store.likedSongs[2]?.coverUrl"
                :src="store.likedSongs[2].coverUrl"
                class="stack-img layer-3"
                draggable="false"
                alt=""
              />
              <img
                v-if="store.likedSongs[1]?.coverUrl"
                :src="store.likedSongs[1].coverUrl"
                class="stack-img layer-2"
                draggable="false"
                alt=""
              />
              <div class="stack-img layer-1">
                <img
                  v-if="store.likedSongs[0]?.coverUrl"
                  :src="store.likedSongs[0].coverUrl"
                  draggable="false"
                  alt=""
                />
                <div v-else class="stack-placeholder">
                  <PlumpIcon name="heart-filled" :size="28" color="#fff" />
                </div>
                <span class="stack-heart-badge">
                  <PlumpIcon name="heart-filled" :size="11" color="#fff" />
                </span>
              </div>
            </div>

            <div class="bento-liked-copy">
              <div class="bento-eyebrow coral">FAVORITES · 红心曲库</div>
              <div class="bento-title-row">
                <h3>我喜欢的音乐</h3>
                <span class="bento-count-badge coral">
                  {{ likedTotalCount }} 首
                </span>
              </div>
              <div class="bento-liked-stats">
                <span v-if="likedHiResCount > 0">
                  {{ likedHiResCount }} 首无损 / Hi-Res ·
                </span>
                <span>{{ likedDurationText }}</span>
              </div>
            </div>
          </div>

          <div class="bento-liked-actions" @click.stop>
            <button
              class="bento-icon-circle"
              title="随机漫游我喜欢的音乐"
              @click.stop="shufflePlayLiked"
            >
              <PlumpIcon name="shuffle" :size="15" />
            </button>
            <button
              class="bento-play-fab coral"
              title="立即播放我喜欢的音乐"
              @click.stop="playAllLiked"
            >
              <PlumpIcon name="play" :size="19" color="#fff" />
            </button>
          </div>
        </div>

        <!-- 底部双曲预览胶囊 -->
        <div class="bento-liked-tracks" @click.stop>
          <template v-if="store.likedSongs.length > 0">
            <div
              v-for="song in store.likedSongs.slice(0, 2)"
              :key="song.hash"
              class="bento-mini-track"
              :class="{ playing: store.currentSong?.hash === song.hash }"
              @click.stop="
                store.playSongFromList(song, store.likedSongs, '我喜欢的音乐')
              "
            >
              <img
                v-if="song.coverUrl"
                :src="song.coverUrl"
                class="mini-track-cover"
                draggable="false"
                alt=""
              />
              <div class="mini-track-text">
                <span class="mini-track-title">{{ song.title }}</span>
                <span class="mini-track-artist">{{ song.artist }}</span>
              </div>
              <span
                v-if="song.isHiRes || song.hasHiRes"
                class="mini-track-tag hires"
              >
                Hi-Res
              </span>
              <span
                v-else-if="song.isLossless || song.hasSq"
                class="mini-track-tag sq"
              >
                SQ
              </span>
            </div>
          </template>
          <div
            v-else
            class="bento-liked-empty"
            @click="store.openLikedSongsDetail()"
          >
            <span>点击进入红心曲库详情，全量多页同步你的全部收藏单曲 ›</span>
          </div>
        </div>
      </article>

      <!-- 右 3 栅格：双层叠放（上：最近播放 / 下：概念心流舱 Focus Pod） -->
      <div class="bento-right-stack">
        <!-- 上卡：最近播放（晴空蔚蓝渐变） -->
        <article
          class="bento-card bento-recent-card"
          @click="store.openRecentHistoryDetail()"
        >
          <div class="bento-compact-main">
            <div class="bento-compact-cover blue">
              <img
                v-if="store.recentHistorySongs[0]?.coverUrl"
                :src="store.recentHistorySongs[0].coverUrl"
                draggable="false"
                alt=""
              />
              <PlumpIcon v-else name="timer" :size="22" color="#fff" />
            </div>

            <div class="bento-compact-copy">
              <div class="bento-eyebrow blue">RECENT · 听歌足迹</div>
              <div class="bento-title-row">
                <h4>最近播放</h4>
                <span class="bento-count-badge blue">
                  {{ store.recentHistorySongs.length }}
                </span>
              </div>
              <div class="bento-compact-sub">
                <template v-if="store.recentHistorySongs[0]">
                  {{ store.recentHistorySongs[0].title }} —
                  {{ store.recentHistorySongs[0].artist }}
                </template>
                <template v-else>
                  本机与云端双轨持久化记录
                </template>
              </div>
            </div>
          </div>

          <button
            class="bento-play-fab blue mini"
            title="播放最近听过"
            @click.stop="playAllRecent"
          >
            <PlumpIcon name="play" :size="16" color="#fff" />
          </button>
        </article>

        <!-- 下卡：概念心流舱 · Focus Pod（极光紫罗兰渐变） -->
        <article
          class="bento-card bento-focus-card"
          @click="store.isFocusPodOpen = true"
        >
          <div class="bento-compact-main">
            <div class="bento-compact-cover violet">
              <PlumpIcon name="sparkles" :size="22" color="#fff" />
            </div>

            <div class="bento-compact-copy">
              <div class="bento-eyebrow violet">FOCUS POD · 概念胶囊</div>
              <div class="bento-title-row">
                <h4>心流专注舱</h4>
                <span class="bento-count-badge violet">白噪音 × 番茄钟</span>
              </div>
              <div class="bento-compact-sub">
                <template v-if="store.focusTodayTotalMinutes > 0">
                  今日已专注 {{ store.focusTodayTotalMinutes }} 分钟 · 雨声 / 潮汐 / 篝火
                </template>
                <template v-else>
                  雨声 · 潮汐 · 篝火 · 四轨环境音混音台
                </template>
              </div>
            </div>
          </div>

          <button
            class="bento-play-fab violet mini"
            title="开启概念心流舱"
            @click.stop="store.isFocusPodOpen = true"
          >
            <PlumpIcon name="compass" :size="16" color="#fff" />
          </button>
        </article>
      </div>
    </section>

    <!-- ==================== 第二层：100% 全宽无界歌单与资产陈列馆 ==================== -->
    <section class="library-fullwidth-gallery">
      <!-- 顶部无框四色游标导航栏 + 工具按钮 -->
      <header class="lib-gallery-toolbar">
        <div class="lib-toolbar-left">
          <div class="lib-sliding-tabs">
            <button
              v-for="tab in assetTabs"
              :key="tab.id"
              class="lib-tab-btn"
              :class="{ active: store.libraryAssetTab === tab.id }"
              @click="handleSelectAssetTab(tab.id)"
            >
              <span class="tab-name">{{ tab.label }}</span>
              <span class="tab-num">{{ tab.count }}</span>
              <span
                class="tab-indicator"
                :style="{
                  backgroundColor: tab.color,
                  opacity: store.libraryAssetTab === tab.id ? 1 : 0,
                  transform:
                    store.libraryAssetTab === tab.id
                      ? 'scaleX(1)'
                      : 'scaleX(0.2)',
                }"
              ></span>
            </button>
          </div>

          <div class="lib-filter-box">
            <PlumpIcon name="search" :size="14" />
            <input
              v-model="galleryFilterKw"
              type="text"
              placeholder="筛选歌单、歌手或已购专辑..."
            />
            <button
              v-if="galleryFilterKw"
              class="filter-clear-btn"
              @click="galleryFilterKw = ''"
            >
              ×
            </button>
          </div>
        </div>

        <div class="lib-toolbar-right">
          <button class="lib-tool-btn primary" @click="openCreateModal">
            <PlumpIcon name="plus" :size="15" />
            <span>新建歌单</span>
          </button>
          <button class="lib-tool-btn" @click="openImportModal">
            <PlumpIcon name="sparkles" :size="15" />
            <span>导入歌单 / 口令</span>
          </button>
          <button
            class="lib-tool-icon-btn"
            :disabled="isRefreshingAssets"
            title="刷新云端资产"
            @click="refreshLibraryAssets"
          >
            <PlumpIcon
              name="refresh"
              :size="16"
              :class="{ 'spin-anim': isRefreshingAssets }"
            />
          </button>
          <button
            class="lib-tool-icon-btn"
            :title="
              store.isLibraryGridView ? '切换为双列宽幅列表' : '切换为海报画廊宫格'
            "
            @click="store.isLibraryGridView = !store.isLibraryGridView"
          >
            <PlumpIcon
              :name="store.isLibraryGridView ? 'list-view' : 'grid-view'"
              :size="16"
            />
          </button>
        </div>
      </header>

      <!-- 1. 自建歌单 / 收藏歌单 画廊 -->
      <div
        v-if="
          store.libraryAssetTab === 'created' ||
          store.libraryAssetTab === 'collected'
        "
        class="lib-gallery-body"
      >
        <div
          v-if="filteredPlaylists.length > 0"
          :class="
            store.isLibraryGridView
              ? 'lib-playlist-poster-grid'
              : 'lib-playlist-wide-list'
          "
        >
          <div
            v-for="pl in filteredPlaylists"
            :key="pl.id || pl.globalCollectionId || pl.title"
            class="lib-pl-card"
            @click="store.openPlaylistDetail(pl)"
          >
            <div class="lib-pl-cover-wrap">
              <img
                v-if="pl.coverUrl"
                :src="pl.coverUrl"
                class="lib-pl-cover"
                loading="lazy"
                draggable="false"
                alt=""
              />
              <div v-else class="lib-pl-cover-fallback">
                <PlumpIcon name="music-note" :size="30" />
              </div>
              <span class="lib-pl-count-tag">{{ pl.trackCount || 0 }} 首</span>

              <div class="lib-pl-hover-mask">
                <button
                  class="lib-pl-play-btn"
                  title="查看并播放歌单"
                  @click.stop="store.openPlaylistDetail(pl)"
                >
                  <PlumpIcon name="play" :size="18" color="#fff" />
                </button>
              </div>
            </div>

            <div class="lib-pl-info">
              <div class="lib-pl-title" :title="pl.title || pl.name">
                {{ pl.title || pl.name }}
              </div>
              <div class="lib-pl-sub">
                <span>{{ pl.trackCount || 0 }} 首曲目</span>
                <span v-if="pl.creator"> · {{ pl.creator }}</span>
              </div>
            </div>

            <!-- 自建歌单支持快捷编辑/删除；收藏歌单支持取消收藏 -->
            <div class="lib-pl-item-actions" @click.stop>
              <template v-if="store.libraryAssetTab === 'created' && pl.listId">
                <button
                  class="pl-mini-act-btn"
                  title="编辑歌单标题与简介"
                  @click.stop="openEditPlaylist(pl)"
                >
                  <PlumpIcon name="sliders" :size="13" />
                </button>
                <button
                  class="pl-mini-act-btn danger"
                  title="删除歌单"
                  @click.stop="confirmDeletePlaylist(pl)"
                >
                  <PlumpIcon name="trash" :size="13" />
                </button>
              </template>
              <template v-else-if="store.libraryAssetTab === 'collected'">
                <button
                  class="pl-mini-act-btn danger"
                  title="取消收藏此歌单"
                  @click.stop="store.toggleCollectPlaylist(pl)"
                >
                  <PlumpIcon name="trash" :size="13" />
                </button>
              </template>
            </div>
          </div>
        </div>

        <div v-else class="lib-empty-gallery">
          <PlumpIcon name="music-note" :size="36" />
          <p v-if="!store.isLoggedIn">
            登录酷狗概念版账号后即可查看并管理你的云端歌单
          </p>
          <p v-else-if="store.libraryAssetTab === 'created'">
            暂无自建歌单，点击右上方「+ 新建歌单」或「导入歌单」创建你的专属曲库
          </p>
          <p v-else>
            暂无收藏的外部歌单，可在发现页或搜索页点击红心收藏喜欢的歌单
          </p>
        </div>
      </div>

      <!-- 2. 关注歌手陈列架 -->
      <div
        v-else-if="store.libraryAssetTab === 'artists'"
        class="lib-gallery-body"
      >
        <div
          v-if="filteredArtists.length > 0"
          :class="
            store.isLibraryGridView
              ? 'lib-artist-poster-grid'
              : 'lib-playlist-wide-list'
          "
        >
          <div
            v-for="art in filteredArtists"
            :key="art.artistId || art.singerId || art.name"
            class="lib-artist-card"
            @click="
              store.openArtistDetail(
                String(art.singerId || art.artistId || ''),
                art.name,
                art.avatarUrl
              )
            "
          >
            <div class="lib-artist-avatar-wrap">
              <img
                v-if="art.avatarUrl"
                :src="art.avatarUrl"
                class="lib-artist-avatar"
                loading="lazy"
                draggable="false"
                alt=""
              />
              <div v-else class="lib-artist-avatar fallback">
                {{ (art.name || "歌").slice(0, 1) }}
              </div>
            </div>

            <div class="lib-artist-info">
              <div class="lib-artist-name">{{ art.name }}</div>
              <div class="lib-artist-sub">
                <span v-if="art.songCount">{{ art.songCount }} 首单曲</span>
                <span v-else>{{ formatFansCount(art.fansCount) }}</span>
              </div>
            </div>

            <button
              class="lib-artist-enter-btn"
              @click.stop="
                store.openArtistDetail(
                  String(art.singerId || art.artistId || ''),
                  art.name,
                  art.avatarUrl
                )
              "
            >
              歌手主页 ›
            </button>
          </div>
        </div>

        <div v-else class="lib-empty-gallery">
          <PlumpIcon name="user" :size="36" />
          <p>暂无已关注的歌手，在搜索页或播放页点击歌手即可关注</p>
        </div>
      </div>

      <!-- 3. 已购内容（已购单曲合集 + 已购数字专辑） -->
      <div
        v-else-if="store.libraryAssetTab === 'purchased'"
        class="lib-gallery-body"
      >
        <div
          v-if="
            store.purchasedSongs.length > 0 ||
            filteredPurchasedAlbums.length > 0
          "
          :class="
            store.isLibraryGridView
              ? 'lib-playlist-poster-grid'
              : 'lib-playlist-wide-list'
          "
        >
          <!-- 已购单曲合集入口卡 -->
          <div
            v-if="store.purchasedSongs.length > 0"
            class="lib-pl-card purchased-singles-card"
            @click="store.openPurchasedSongsDetail()"
          >
            <div class="lib-pl-cover-wrap gold-wrap">
              <img
                v-if="store.purchasedSongs[0]?.song?.coverUrl"
                :src="store.purchasedSongs[0].song.coverUrl"
                class="lib-pl-cover"
                draggable="false"
                alt=""
              />
              <div v-else class="lib-pl-cover-fallback gold">
                <PlumpIcon name="crown" :size="30" color="#FFD54F" />
              </div>
              <span class="lib-pl-count-tag gold">
                {{ store.purchasedSongs.length }} 首已购
              </span>

              <div class="lib-pl-hover-mask">
                <button
                  class="lib-pl-play-btn gold"
                  title="播放全部已购单曲"
                  @click.stop="playAllPurchasedSingles"
                >
                  <PlumpIcon name="play" :size="18" color="#1a1300" />
                </button>
              </div>
            </div>

            <div class="lib-pl-info">
              <div class="lib-pl-title">已购单曲合集</div>
              <div class="lib-pl-sub">
                共 {{ store.purchasedSongs.length }} 首已付费数字单曲 · 无损畅听
              </div>
            </div>
          </div>

          <!-- 已购数字专辑卡片 -->
          <div
            v-for="alb in filteredPurchasedAlbums"
            :key="alb.id || alb.albumId || alb.albumName"
            class="lib-pl-card purchased-album-card"
            @click="
              store.openAlbumDetail(
                String(alb.albumId || alb.id || ''),
                alb.albumName,
                alb.singerName || alb.artistName || '',
                alb.coverUrl
              )
            "
          >
            <div class="lib-pl-cover-wrap">
              <img
                v-if="alb.coverUrl"
                :src="alb.coverUrl"
                class="lib-pl-cover"
                loading="lazy"
                draggable="false"
                alt=""
              />
              <div v-else class="lib-pl-cover-fallback gold">
                <PlumpIcon name="disc" :size="30" color="#FFD54F" />
              </div>
              <span class="lib-pl-count-tag gold">
                {{ alb.songCount > 0 ? `${alb.songCount} 首` : "数字专辑" }}
              </span>

              <div class="lib-pl-hover-mask">
                <button
                  class="lib-pl-play-btn gold"
                  title="进入已购数字专辑"
                  @click.stop="
                    store.openAlbumDetail(
                      String(alb.albumId || alb.id || ''),
                      alb.albumName,
                      alb.singerName || alb.artistName || '',
                      alb.coverUrl
                    )
                  "
                >
                  <PlumpIcon name="play" :size="18" color="#1a1300" />
                </button>
              </div>
            </div>

            <div class="lib-pl-info">
              <div class="lib-pl-title" :title="alb.albumName">
                {{ alb.albumName }}
              </div>
              <div class="lib-pl-sub">
                <span>{{
                  alb.singerName || alb.artistName || "数字专辑"
                }}</span>
                <span v-if="alb.buyTime">
                  · {{ alb.buyTime.slice(0, 10) }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <div v-else class="lib-empty-gallery">
          <PlumpIcon name="crown" :size="36" color="#E5A93B" />
          <p>
            当前账号暂无已购数字专辑或单曲记录，点击右上角刷新按钮可重新同步云端已购资产
          </p>
        </div>
      </div>
    </section>

    <!-- 新建歌单弹窗 -->
    <transition name="fade">
      <div
        v-if="showCreateModal"
        class="modal-backdrop"
        @click.self="showCreateModal = false"
      >
        <div class="compact-dialog glass-popover">
          <h3>新建云端自建歌单</h3>
          <p class="dialog-sub">
            歌单将实时同步至你的酷狗概念版云端账号：
          </p>
          <input
            v-model="newPlaylistName"
            type="text"
            class="dialog-input"
            placeholder="请输入歌单名称..."
            @keyup.enter="submitCreatePlaylist"
          />
          <div class="dialog-actions">
            <button class="frameless-hover-btn" @click="showCreateModal = false">
              取消
            </button>
            <button
              class="frameless-hover-btn primary-tint"
              @click="submitCreatePlaylist"
            >
              立即创建
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 编辑歌单弹窗 -->
    <transition name="fade">
      <div
        v-if="showEditModal"
        class="modal-backdrop"
        @click.self="showEditModal = false"
      >
        <div class="compact-dialog glass-popover">
          <h3>编辑歌单信息</h3>
          <p class="dialog-sub">修改歌单标题与简介并同步至酷狗云端：</p>
          <input
            v-model="editTitle"
            type="text"
            class="dialog-input"
            placeholder="歌单名称"
          />
          <textarea
            v-model="editIntro"
            class="dialog-input"
            style="height: 86px; margin-top: 10px; padding: 10px 12px; resize: none"
            placeholder="歌单简介（选填）"
          ></textarea>
          <div class="dialog-actions">
            <button class="frameless-hover-btn" @click="showEditModal = false">
              取消
            </button>
            <button
              class="frameless-hover-btn primary-tint"
              @click="submitEditPlaylist"
            >
              保存修改
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 导入外部歌单弹窗 -->
    <transition name="fade">
      <div
        v-if="showImportModal"
        class="modal-backdrop"
        @click.self="showImportModal = false"
      >
        <div
          class="compact-dialog glass-popover"
          style="width: min(92vw, 520px)"
        >
          <h3>导入外部歌单与分享口令</h3>
          <p class="dialog-sub">
            支持粘贴酷狗分享短链 (t1.kugou.com)、歌单码、global_collection_id、微风口令或「歌名 - 歌手」文本列表：
          </p>
          <textarea
            v-model="importInputText"
            class="dialog-input"
            style="height: 110px; padding: 12px 14px; resize: none; line-height: 1.5"
            placeholder="粘贴酷狗分享链接、口令或多行「歌名 - 歌手」列表..."
          ></textarea>
          <div class="dialog-actions">
            <button class="frameless-hover-btn" @click="showImportModal = false">
              取消
            </button>
            <button
              class="frameless-hover-btn primary-tint"
              @click="submitImportPlaylist"
            >
              智能解析并导入
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.library-bento-page {
  display: flex;
  flex-direction: column;
  gap: 24px;
  padding: 6px 6px 36px;
  min-height: 100%;
}

/* ==================== 第一层：12 栅格 5:4:3 杂志感 Bento 首屏 ==================== */
.library-bento-hero-grid {
  display: grid;
  grid-template-columns: repeat(12, minmax(0, 1fr));
  gap: 16px;
  align-items: stretch;
}

.bento-card {
  position: relative;
  border-radius: 22px;
  padding: 20px 22px;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  box-shadow: var(--shadow-card);
  transition: transform 0.24s cubic-bezier(0.22, 1, 0.36, 1),
    box-shadow 0.24s ease, border-color 0.24s ease;
  overflow: hidden;
}

/* 左 5 栅格：个人身份 + 听歌等级 + 内嵌 VIP 权益条 */
.bento-identity-card {
  grid-column: span 5;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 14px;
  background: linear-gradient(
    145deg,
    rgba(43, 140, 255, 0.12) 0%,
    rgba(0, 196, 167, 0.06) 55%,
    var(--bg-card) 100%
  );
}

.bento-id-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.bento-id-user {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
  cursor: pointer;
}

.bento-avatar-wrap {
  position: relative;
  width: 54px;
  height: 54px;
  flex-shrink: 0;
}

.bento-avatar-img,
.bento-avatar-fallback {
  width: 54px;
  height: 54px;
  border-radius: 50%;
  object-fit: cover;
  border: 2px solid rgba(43, 140, 255, 0.55);
  box-shadow: 0 6px 16px rgba(26, 115, 232, 0.22);
}

.bento-avatar-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--primary-soft);
  color: var(--primary);
  font-size: 20px;
  font-weight: 800;
}

.bento-online-dot {
  position: absolute;
  right: 2px;
  bottom: 2px;
  width: 11px;
  height: 11px;
  border-radius: 50%;
  background: var(--accent-mint);
  border: 2px solid var(--bg-base);
}

.bento-user-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.bento-user-name-row {
  display: flex;
  align-items: center;
  gap: 7px;
  flex-wrap: wrap;
}

.bento-user-name {
  margin: 0;
  font-size: 17px;
  font-weight: 800;
  color: var(--text-main);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 165px;
}

.bento-lv-badge {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 800;
  color: var(--accent-mint);
  background: var(--accent-mint-soft);
}

.bento-vip-badge {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 10.5px;
  font-weight: 800;
  color: var(--text-sub);
  background: var(--bg-glass-hover);
}

.bento-vip-badge.active {
  color: var(--accent-gold);
  background: var(--accent-gold-soft);
}

.bento-user-sub {
  font-size: 11.5px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.bento-id-actions {
  display: flex;
  align-items: center;
  gap: 7px;
  flex-shrink: 0;
}

.bento-pill-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 6px 11px;
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 700;
  color: var(--text-sub);
  background: var(--bg-glass);
  border: 1px solid var(--border-subtle);
  cursor: pointer;
  transition: all 0.2s ease;
}

.bento-pill-btn:hover:not(:disabled) {
  color: var(--primary);
  border-color: var(--border-accent);
  background: var(--primary-soft);
}

.bento-pill-btn.sync-btn {
  color: var(--primary);
  background: var(--primary-soft);
  border-color: var(--border-accent);
}

/* 听歌时长与等级进度框 */
.bento-listen-box {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 15px;
  border-radius: 15px;
  background: var(--bg-glass);
  border: 1px solid var(--border-subtle);
}

.bento-listen-left {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.bento-listen-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
}

.bento-listen-value {
  display: flex;
  align-items: baseline;
  gap: 4px;
  color: var(--text-main);
}

.bento-listen-value strong {
  font-size: 21px;
  font-weight: 800;
  letter-spacing: -0.02em;
  color: var(--primary);
}

.bento-listen-value .unit {
  font-size: 11.5px;
  color: var(--text-sub);
  margin-right: 4px;
}

.bento-listen-right {
  flex: 1;
  max-width: 210px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.bento-level-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11px;
  font-weight: 700;
  color: var(--accent-mint);
}

.bento-level-meta .hint {
  font-weight: 600;
  color: var(--text-muted);
}

.bento-level-track {
  height: 6px;
  border-radius: 999px;
  background: rgba(120, 145, 185, 0.18);
  overflow: hidden;
}

.bento-level-fill {
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--primary), var(--accent-mint));
  transition: width 0.35s ease;
}

/* 内嵌概念版 VIP 黑金/流金权益条 */
.bento-vip-strip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 14px;
  background: linear-gradient(
    135deg,
    rgba(255, 213, 79, 0.16) 0%,
    rgba(229, 169, 59, 0.08) 100%
  );
  border: 1px solid rgba(255, 213, 79, 0.3);
  cursor: pointer;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.bento-vip-strip:hover {
  transform: translateY(-1px);
  border-color: rgba(255, 213, 79, 0.52);
}

.bento-vip-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.bento-vip-orb {
  width: 32px;
  height: 32px;
  border-radius: 10px;
  background: rgba(255, 213, 79, 0.2);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.bento-vip-copy {
  min-width: 0;
}

.bento-vip-title {
  font-size: 12.5px;
  font-weight: 800;
  color: var(--accent-gold);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.bento-vip-sub {
  font-size: 11px;
  color: var(--text-sub);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.bento-vip-cta {
  padding: 6px 12px;
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 800;
  background: linear-gradient(135deg, #ffd54f, #e5a93b);
  color: #1f1600;
  border: none;
  cursor: pointer;
  flex-shrink: 0;
  transition: transform 0.2s ease;
}

.bento-vip-cta.claimed {
  background: var(--accent-gold-soft);
  color: var(--accent-gold);
  border: 1px solid rgba(255, 213, 79, 0.35);
}

/* 中 4 栅格：我喜欢的音乐（珊瑚粉水彩 / 暗夜瑰红渐变主视觉卡） */
.bento-liked-card {
  grid-column: span 4;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 14px;
  cursor: pointer;
  background: linear-gradient(
    145deg,
    rgba(255, 107, 91, 0.16) 0%,
    rgba(255, 126, 157, 0.08) 55%,
    var(--bg-card) 100%
  );
  border-color: rgba(255, 107, 91, 0.28);
}

.bento-liked-card:hover {
  transform: translateY(-2px);
  border-color: rgba(255, 107, 91, 0.48);
}

.bento-liked-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.bento-liked-main {
  display: flex;
  align-items: center;
  gap: 16px;
  min-width: 0;
}

.bento-cover-stack {
  position: relative;
  width: 78px;
  height: 70px;
  flex-shrink: 0;
}

.stack-img {
  position: absolute;
  top: 0;
  left: 0;
  width: 66px;
  height: 66px;
  border-radius: 15px;
  object-fit: cover;
  box-shadow: 0 8px 18px rgba(0, 0, 0, 0.22);
}

.stack-img.layer-3 {
  transform: translate(14px, 6px) scale(0.84);
  opacity: 0.45;
}

.stack-img.layer-2 {
  transform: translate(7px, 3px) scale(0.92);
  opacity: 0.72;
}

.stack-img.layer-1 {
  position: relative;
  overflow: hidden;
  background: linear-gradient(135deg, #ff6b5b, #e84362);
}

.stack-img.layer-1 img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.stack-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.stack-heart-badge {
  position: absolute;
  right: 5px;
  bottom: 5px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #ff5252;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
}

.bento-liked-copy {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.bento-eyebrow {
  font-size: 10.5px;
  font-weight: 800;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.bento-eyebrow.coral {
  color: var(--accent-coral);
}

.bento-eyebrow.blue {
  color: var(--primary);
}

.bento-eyebrow.violet {
  color: var(--accent-violet);
}

.bento-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.bento-title-row h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 800;
  color: var(--text-main);
}

.bento-title-row h4 {
  margin: 0;
  font-size: 15.5px;
  font-weight: 800;
  color: var(--text-main);
}

.bento-count-badge {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 800;
}

.bento-count-badge.coral {
  background: var(--accent-coral-soft);
  color: var(--accent-coral);
}

.bento-count-badge.blue {
  background: var(--primary-soft);
  color: var(--primary);
}

.bento-count-badge.violet {
  background: var(--accent-violet-soft);
  color: var(--accent-violet);
}

.bento-liked-stats {
  font-size: 12px;
  color: var(--text-sub);
}

.bento-liked-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.bento-icon-circle {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  background: var(--bg-glass);
  border: 1px solid var(--border-subtle);
  color: var(--text-main);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s ease;
}

.bento-icon-circle:hover {
  color: var(--accent-coral);
  border-color: var(--accent-coral);
}

.bento-play-fab {
  width: 42px;
  height: 42px;
  border-radius: 50%;
  border: none;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
  flex-shrink: 0;
}

.bento-play-fab:hover {
  transform: scale(1.06);
}

.bento-play-fab.coral {
  background: linear-gradient(135deg, #ff6b5b, #e84362);
  box-shadow: 0 8px 20px rgba(255, 107, 91, 0.35);
}

.bento-play-fab.blue {
  background: linear-gradient(135deg, #2b8cff, #00c4a7);
  box-shadow: 0 6px 16px rgba(43, 140, 255, 0.32);
}

.bento-play-fab.violet {
  background: linear-gradient(135deg, #9688ff, #6c5ce7);
  box-shadow: 0 6px 16px rgba(150, 136, 255, 0.32);
}

.bento-play-fab.mini {
  width: 36px;
  height: 36px;
}

.bento-liked-tracks {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.bento-mini-track {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
  border-radius: 12px;
  background: var(--bg-glass);
  border: 1px solid var(--border-subtle);
  cursor: pointer;
  transition: all 0.2s ease;
}

.bento-mini-track:hover,
.bento-mini-track.playing {
  border-color: rgba(255, 107, 91, 0.42);
  background: var(--accent-coral-soft);
}

.mini-track-cover {
  width: 30px;
  height: 30px;
  border-radius: 7px;
  object-fit: cover;
  flex-shrink: 0;
}

.mini-track-text {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.mini-track-title {
  font-size: 12.5px;
  font-weight: 700;
  color: var(--text-main);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mini-track-artist {
  font-size: 11.5px;
  color: var(--text-sub);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mini-track-tag {
  font-size: 9.5px;
  font-weight: 800;
  padding: 1px 6px;
  border-radius: 5px;
  flex-shrink: 0;
}

.mini-track-tag.hires {
  background: var(--accent-gold-soft);
  color: var(--accent-gold);
}

.mini-track-tag.sq {
  background: var(--accent-mint-soft);
  color: var(--accent-mint);
}

.bento-liked-empty {
  padding: 14px;
  border-radius: 12px;
  background: var(--bg-glass);
  font-size: 12px;
  color: var(--text-sub);
  text-align: center;
}

/* 右 3 栅格：上下叠放双胶囊卡 */
.bento-right-stack {
  grid-column: span 3;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.bento-recent-card,
.bento-focus-card {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 18px;
  cursor: pointer;
}

.bento-recent-card {
  background: linear-gradient(
    145deg,
    rgba(43, 140, 255, 0.15) 0%,
    rgba(0, 196, 167, 0.08) 65%,
    var(--bg-card) 100%
  );
  border-color: rgba(43, 140, 255, 0.26);
}

.bento-recent-card:hover {
  transform: translateY(-2px);
  border-color: rgba(43, 140, 255, 0.48);
}

.bento-focus-card {
  background: linear-gradient(
    145deg,
    rgba(150, 136, 255, 0.16) 0%,
    rgba(108, 92, 231, 0.08) 65%,
    var(--bg-card) 100%
  );
  border-color: rgba(150, 136, 255, 0.28);
}

.bento-focus-card:hover {
  transform: translateY(-2px);
  border-color: rgba(150, 136, 255, 0.48);
}

.bento-compact-main {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.bento-compact-cover {
  width: 48px;
  height: 48px;
  border-radius: 14px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.bento-compact-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.bento-compact-cover.blue {
  background: linear-gradient(135deg, #2b8cff, #00c4a7);
}

.bento-compact-cover.violet {
  background: linear-gradient(135deg, #9688ff, #6c5ce7);
}

.bento-compact-copy {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.bento-compact-sub {
  font-size: 11.5px;
  color: var(--text-sub);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ==================== 第二层：100% 全宽无界陈列馆 ==================== */
.library-fullwidth-gallery {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.lib-gallery-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 6px 4px 10px;
  border-bottom: 1px solid var(--border-subtle);
  flex-wrap: wrap;
}

.lib-toolbar-left {
  display: flex;
  align-items: center;
  gap: 22px;
  flex-wrap: wrap;
}

.lib-sliding-tabs {
  display: flex;
  align-items: center;
  gap: 6px;
}

.lib-tab-btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 8px 14px 11px;
  background: none;
  border: none;
  color: var(--text-sub);
  font-size: 14.5px;
  font-weight: 700;
  cursor: pointer;
  transition: color 0.2s ease;
}

.lib-tab-btn.active {
  color: var(--text-main);
  font-weight: 800;
}

.lib-tab-btn .tab-num {
  font-size: 11.5px;
  font-weight: 800;
  padding: 1px 7px;
  border-radius: 999px;
  background: var(--bg-glass);
  color: var(--text-muted);
}

.lib-tab-btn.active .tab-num {
  color: var(--text-main);
  background: var(--primary-soft);
}

.lib-tab-btn .tab-indicator {
  position: absolute;
  left: 14px;
  right: 14px;
  bottom: 0;
  height: 3px;
  border-radius: 999px;
  transition: transform 0.25s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.2s ease;
}

.lib-filter-box {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-radius: 999px;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  color: var(--text-muted);
  width: 230px;
}

.lib-filter-box input {
  border: none;
  background: none;
  outline: none;
  color: var(--text-main);
  font-size: 12.5px;
  width: 100%;
}

.filter-clear-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 14px;
}

.lib-toolbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.lib-tool-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border-radius: 11px;
  font-size: 12.5px;
  font-weight: 700;
  background: var(--bg-card);
  color: var(--text-main);
  border: 1px solid var(--border-subtle);
  cursor: pointer;
  transition: all 0.2s ease;
}

.lib-tool-btn:hover {
  border-color: var(--border-accent);
  color: var(--primary);
}

.lib-tool-btn.primary {
  background: var(--primary-soft);
  color: var(--primary);
  border-color: var(--border-accent);
}

.lib-tool-btn.primary:hover {
  background: var(--primary);
  color: #fff;
}

.lib-tool-icon-btn {
  width: 34px;
  height: 34px;
  border-radius: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  color: var(--text-sub);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s ease;
}

.lib-tool-icon-btn:hover {
  color: var(--primary);
  border-color: var(--border-accent);
}

/* 海报宫格与双列宽幅列表 */
.lib-playlist-poster-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(186px, 1fr));
  gap: 18px;
}

.lib-playlist-wide-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 14px;
}

.lib-pl-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  border-radius: 18px;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  cursor: pointer;
  transition: transform 0.22s ease, border-color 0.22s ease,
    box-shadow 0.22s ease;
}

.lib-playlist-wide-list .lib-pl-card {
  flex-direction: row;
  align-items: center;
  gap: 14px;
}

.lib-pl-card:hover {
  transform: translateY(-3px);
  border-color: var(--border-accent);
  box-shadow: var(--shadow-card);
}

.lib-pl-cover-wrap {
  position: relative;
  width: 100%;
  aspect-ratio: 1 / 1;
  border-radius: 14px;
  overflow: hidden;
  background: var(--bg-glass);
  flex-shrink: 0;
}

.lib-playlist-wide-list .lib-pl-cover-wrap {
  width: 68px;
  height: 68px;
}

.lib-pl-cover {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: transform 0.35s ease;
}

.lib-pl-card:hover .lib-pl-cover {
  transform: scale(1.05);
}

.lib-pl-cover-fallback {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--primary);
  background: var(--primary-soft);
}

.lib-pl-cover-fallback.gold {
  background: var(--accent-gold-soft);
}

.lib-pl-count-tag {
  position: absolute;
  top: 8px;
  right: 8px;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 10.5px;
  font-weight: 800;
  color: #fff;
  background: rgba(10, 14, 23, 0.68);
  backdrop-filter: blur(8px);
}

.lib-pl-count-tag.gold {
  color: #ffd54f;
  border: 1px solid rgba(255, 213, 79, 0.35);
}

.lib-pl-hover-mask {
  position: absolute;
  inset: 0;
  background: rgba(10, 14, 23, 0.28);
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
  padding: 10px;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.lib-pl-card:hover .lib-pl-hover-mask {
  opacity: 1;
}

.lib-pl-play-btn {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  border: none;
  background: var(--primary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  box-shadow: 0 6px 16px rgba(43, 140, 255, 0.4);
}

.lib-pl-play-btn.gold {
  background: linear-gradient(135deg, #ffd54f, #e5a93b);
}

.lib-pl-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.lib-pl-title {
  font-size: 14px;
  font-weight: 800;
  color: var(--text-main);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lib-pl-sub {
  font-size: 12px;
  color: var(--text-sub);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lib-pl-item-actions {
  position: absolute;
  top: 18px;
  left: 18px;
  display: flex;
  gap: 6px;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.lib-playlist-wide-list .lib-pl-item-actions {
  position: static;
}

.lib-pl-card:hover .lib-pl-item-actions {
  opacity: 1;
}

.pl-mini-act-btn {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(10, 14, 23, 0.72);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.pl-mini-act-btn.danger:hover {
  background: var(--accent-coral);
  border-color: var(--accent-coral);
}

/* 关注歌手卡片 */
.lib-artist-poster-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(175px, 1fr));
  gap: 18px;
}

.lib-artist-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 10px;
  padding: 18px 14px;
  border-radius: 18px;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  cursor: pointer;
  transition: all 0.22s ease;
}

.lib-playlist-wide-list .lib-artist-card {
  flex-direction: row;
  text-align: left;
  padding: 12px 16px;
}

.lib-artist-card:hover {
  transform: translateY(-3px);
  border-color: var(--accent-mint);
}

.lib-artist-avatar-wrap {
  width: 84px;
  height: 84px;
  border-radius: 50%;
  overflow: hidden;
  border: 2px solid var(--accent-mint-soft);
  flex-shrink: 0;
}

.lib-playlist-wide-list .lib-artist-avatar-wrap {
  width: 56px;
  height: 56px;
}

.lib-artist-avatar {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.lib-artist-avatar.fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--accent-mint-soft);
  color: var(--accent-mint);
  font-size: 24px;
  font-weight: 800;
}

.lib-artist-info {
  flex: 1;
  min-width: 0;
}

.lib-artist-name {
  font-size: 14.5px;
  font-weight: 800;
  color: var(--text-main);
}

.lib-artist-sub {
  font-size: 12px;
  color: var(--text-sub);
  margin-top: 2px;
}

.lib-artist-enter-btn {
  padding: 5px 12px;
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 700;
  background: var(--accent-mint-soft);
  color: var(--accent-mint);
  border: none;
  cursor: pointer;
}

.lib-empty-gallery {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 56px 20px;
  border-radius: 20px;
  background: var(--bg-card);
  border: 1px dashed var(--border-subtle);
  color: var(--text-muted);
  font-size: 13.5px;
}

@media (max-width: 1180px) {
  .bento-identity-card {
    grid-column: span 12;
  }
  .bento-liked-card {
    grid-column: span 7;
  }
  .bento-right-stack {
    grid-column: span 5;
  }
}
</style>
