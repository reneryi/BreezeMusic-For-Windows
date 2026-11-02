<template>
  <div class="library-page">
    <!-- ==================== 顶部三张 3D 立体核心快捷卡片（已上移，无冗余顶栏，内部无框沉浸排版） ==================== -->
    <section class="library-hero-trio">
      <!-- 1. 我喜欢的音乐 -->
      <div
        class="lib-stereo-card fav-stereo-card"
        :class="{ active: rightStageMode === 'liked' }"
        @click="activateLikedMode"
      >
        <div class="lib-card-glow fav-glow"></div>
        <div class="lib-card-visual">
          <div v-if="store.likedSongs[1]?.coverUrl" class="lib-visual-back">
            <img :src="store.likedSongs[1].coverUrl" alt="" />
          </div>
          <div class="lib-visual-front">
            <img
              v-if="store.likedSongs[0]?.coverUrl"
              :src="store.likedSongs[0].coverUrl"
              alt=""
            />
            <div class="lib-visual-badge coral">
              <PlumpIcon name="heart-filled" :size="20" color="#fff" />
            </div>
          </div>
        </div>

        <div class="lib-card-copy">
          <span class="lib-card-eyebrow">MY FAVORITES · 红心珍藏</span>
          <div class="lib-card-title-row">
            <h3 class="lib-card-title">我喜欢的音乐</h3>
            <span class="lib-card-count">{{ store.likedSongs.length }} 首</span>
          </div>
          <p class="lib-card-preview">
            <template v-if="store.likedSongs[0]">
              ♪ {{ store.likedSongs[0].title }} - {{ store.likedSongs[0].artist }}
            </template>
            <template v-else>
              双端实时同步红心曲库 · 珍藏每一份心动旋律
            </template>
          </p>
        </div>

        <button
          class="lib-card-play-fab coral"
          title="立即播放我喜欢的音乐"
          @click.stop="playAllLiked"
        >
          <PlumpIcon name="play" :size="19" color="#fff" />
        </button>
      </div>

      <!-- 2. 最近播放 -->
      <div
        class="lib-stereo-card recent-stereo-card"
        :class="{ active: rightStageMode === 'recent' }"
        @click="activateRecentMode"
      >
        <div class="lib-card-glow recent-glow"></div>
        <div class="lib-card-visual">
          <div v-if="store.recentHistorySongs[1]?.coverUrl" class="lib-visual-back">
            <img :src="store.recentHistorySongs[1].coverUrl" alt="" />
          </div>
          <div class="lib-visual-front">
            <img
              v-if="store.recentHistorySongs[0]?.coverUrl"
              :src="store.recentHistorySongs[0].coverUrl"
              alt=""
            />
            <div class="lib-visual-badge blue">
              <PlumpIcon name="clock" :size="20" color="#fff" />
            </div>
          </div>
        </div>

        <div class="lib-card-copy">
          <span class="lib-card-eyebrow">RECENT FOOTPRINTS · 听歌足迹</span>
          <div class="lib-card-title-row">
            <h3 class="lib-card-title">最近播放</h3>
            <span class="lib-card-count">{{ store.recentHistorySongs.length }} 首</span>
          </div>
          <p class="lib-card-preview">
            <template v-if="store.recentHistorySongs[0]">
              ♪ {{ store.recentHistorySongs[0].title }} - {{ store.recentHistorySongs[0].artist }}
            </template>
            <template v-else>
              自动记录本机与云端最近播放曲目与频次
            </template>
          </p>
        </div>

        <button
          class="lib-card-play-fab blue"
          title="立即播放最近听过"
          @click.stop="playAllRecent"
        >
          <PlumpIcon name="play" :size="19" color="#fff" />
        </button>
      </div>

      <!-- 3. 心流舱 · Focus Pod -->
      <div
        class="lib-stereo-card focus-stereo-card"
        @click="store.isFocusPodOpen = true"
      >
        <div class="lib-card-glow focus-glow"></div>
        <div class="lib-card-visual">
          <div class="lib-visual-front focus-orb-visual">
            <div class="focus-orb-ring"></div>
            <PlumpIcon name="sparkle" :size="26" color="#fff" />
          </div>
        </div>

        <div class="lib-card-copy">
          <span class="lib-card-eyebrow">FOCUS POD · 概念心流舱</span>
          <div class="lib-card-title-row">
            <h3 class="lib-card-title">心流舱</h3>
            <span class="lib-card-count">番茄钟 × 白噪音</span>
          </div>
          <p class="lib-card-preview">
            雨声 / 篝火 / 咖啡馆环境音混音 · 打造沉浸式专注声场
          </p>
        </div>

        <button
          class="lib-card-play-fab mint"
          title="开启概念心流舱"
          @click.stop="store.isFocusPodOpen = true"
        >
          <PlumpIcon name="compass" :size="19" color="#fff" />
        </button>
      </div>
    </section>

    <!-- ==================== 下方左右分栏：左侧【自建/收藏/关注/已购 标题+内容整体面板】 vs 右侧【无框歌曲列表】 ==================== -->
    <section class="library-split-workspace">
      <!-- 左侧整体面板：左上角分类标题严格沿用 App 端设计（含底部滑动横条），右上角新建/导入用纯 UI 图标显示 -->
      <aside class="library-left-unified-panel">
        <div class="lib-left-header">
          <div class="app-sliding-tabs lib-asset-sliding-tabs">
            <button
              v-for="tab in assetTabs"
              :key="tab.id"
              class="app-sliding-tab-btn"
              :class="{ active: store.libraryAssetTab === tab.id }"
              @click="handleSelectAssetTab(tab.id)"
            >
              <span class="tab-label">{{ tab.label }}</span>
              <span class="tab-count">{{ tab.count }}</span>
              <span
                class="tab-bottom-indicator"
                :style="{
                  backgroundColor: tab.color,
                  opacity: store.libraryAssetTab === tab.id ? 1 : 0,
                  transform: store.libraryAssetTab === tab.id ? 'scaleX(1)' : 'scaleX(0.2)',
                }"
              ></span>
            </button>
          </div>

          <!-- 右上角：新建歌单、导入外部歌单、视图切换 纯 UI 图标按钮 -->
          <div class="lib-left-icon-tools">
            <button
              class="frameless-icon-square-btn"
              title="新建自建歌单"
              @click="openCreateModal"
            >
              <PlumpIcon name="plus" :size="17" />
            </button>
            <button
              class="frameless-icon-square-btn"
              title="导入外部歌单链接或酷狗歌单码"
              @click="openImportModal"
            >
              <PlumpIcon name="upload" :size="17" />
            </button>
            <button
              v-if="store.libraryAssetTab === 'created' || store.libraryAssetTab === 'collected'"
              class="frameless-icon-square-btn"
              :title="store.isLibraryGridView ? '切换为紧凑列表' : '切换为封面宫格'"
              @click="store.isLibraryGridView = !store.isLibraryGridView"
            >
              <PlumpIcon :name="store.isLibraryGridView ? 'list' : 'grid'" :size="17" />
            </button>
          </div>
        </div>

        <!-- 左面板下方内容区（与上方四个分类标题融为一个整体） -->
        <div class="lib-left-body">
          <!-- A. 自建歌单 / 收藏歌单 -->
          <template v-if="store.libraryAssetTab === 'created' || store.libraryAssetTab === 'collected'">
            <div v-if="activePlaylistGroup.length === 0" class="lib-left-empty">
              <PlumpIcon name="folder" :size="28" />
              <p>{{ store.libraryAssetTab === "created" ? "暂无自建歌单，点击右上角 + 新建" : "暂无收藏歌单，点击右上角图标可导入" }}</p>
            </div>

            <!-- 宫格模式 -->
            <div v-else-if="store.isLibraryGridView" class="lib-playlist-grid">
              <div
                v-for="pl in activePlaylistGroup"
                :key="pl.id"
                class="lib-pl-grid-item"
                :class="{ active: rightStageMode === 'playlist' && store.selectedPlaylist?.id === pl.id }"
                @click="selectPlaylistToRight(pl)"
              >
                <div class="lib-pl-cover-wrap">
                  <img :src="pl.coverUrl" class="lib-pl-cover" loading="lazy" alt="" />
                  <span class="lib-pl-count-tag">{{ pl.trackCount }}首</span>
                </div>
                <div class="lib-pl-name">{{ pl.name }}</div>
              </div>
            </div>

            <!-- 列表模式 -->
            <div v-else class="lib-playlist-list">
              <div
                v-for="pl in activePlaylistGroup"
                :key="pl.id"
                class="lib-pl-row-item"
                :class="{ active: rightStageMode === 'playlist' && store.selectedPlaylist?.id === pl.id }"
                @click="selectPlaylistToRight(pl)"
              >
                <img :src="pl.coverUrl" class="lib-pl-row-cover" loading="lazy" alt="" />
                <div class="lib-pl-row-meta">
                  <div class="lib-pl-row-name">{{ pl.name }}</div>
                  <div class="lib-pl-row-sub">
                    {{ pl.trackCount }} 首 · {{ pl.creatorName || "概念版歌单" }}
                  </div>
                </div>
                <button
                  v-if="store.libraryAssetTab === 'created' && pl.listId > 0"
                  class="frameless-icon-square-btn danger-hover"
                  title="删除歌单"
                  @click.stop="confirmDeletePlaylist(pl)"
                >
                  <PlumpIcon name="trash" :size="14" />
                </button>
              </div>
            </div>
          </template>

          <!-- B. 关注歌手列表 -->
          <template v-else-if="store.libraryAssetTab === 'artists'">
            <div v-if="store.followedArtists.length === 0" class="lib-left-empty">
              <PlumpIcon name="user" :size="28" />
              <p>暂无关注的歌手</p>
            </div>
            <div v-else class="lib-playlist-list">
              <div
                v-for="art in store.followedArtists"
                :key="art.singerId"
                class="lib-pl-row-item"
                :class="{ active: rightStageMode === 'artist' && store.activeArtistDetail?.singerId === art.singerId }"
                @click="selectArtistToRight(art)"
              >
                <img :src="art.avatarUrl" class="lib-pl-row-cover round" loading="lazy" alt="" />
                <div class="lib-pl-row-meta">
                  <div class="lib-pl-row-name">{{ art.name }}</div>
                  <div class="lib-pl-row-sub">
                    {{ art.albumCount ? `${art.albumCount} 张专辑` : "已关注音乐人" }}
                  </div>
                </div>
                <PlumpIcon name="chevron-right" :size="14" />
              </div>
            </div>
          </template>

          <!-- C. 已购数字专辑与单曲 -->
          <template v-else-if="store.libraryAssetTab === 'purchased'">
            <div
              v-if="store.purchasedAlbums.length === 0 && store.purchasedSongs.length === 0"
              class="lib-left-empty"
            >
              <PlumpIcon name="disc" :size="28" />
              <p>暂无已购数字专辑或单曲</p>
            </div>
            <div v-else class="lib-playlist-list">
              <div
                v-if="store.purchasedSongs.length > 0"
                class="lib-pl-row-item"
                :class="{ active: rightStageMode === 'purchased_songs' }"
                @click="rightStageMode = 'purchased_songs'"
              >
                <div class="lib-purchased-icon-cover">
                  <PlumpIcon name="sparkle" :size="20" color="#fff" />
                </div>
                <div class="lib-pl-row-meta">
                  <div class="lib-pl-row-name">已购尊享单曲合集</div>
                  <div class="lib-pl-row-sub">共 {{ store.purchasedSongs.length }} 首已购曲目</div>
                </div>
              </div>

              <div
                v-for="alb in store.purchasedAlbums"
                :key="alb.id"
                class="lib-pl-row-item"
                :class="{ active: rightStageMode === 'album' && store.activeAlbumDetail?.albumId === alb.id }"
                @click="selectPurchasedAlbumToRight(alb)"
              >
                <img :src="alb.coverUrl" class="lib-pl-row-cover" loading="lazy" alt="" />
                <div class="lib-pl-row-meta">
                  <div class="lib-pl-row-name">{{ alb.albumName }}</div>
                  <div class="lib-pl-row-sub">{{ alb.singerName }} · {{ alb.songCount }} 首</div>
                </div>
              </div>
            </div>
          </template>
        </div>
      </aside>

      <!-- 右侧歌曲列表舞台：与左侧留出明显间隔区分左右，且歌曲列表部分全面采用无框设计 -->
      <main class="library-right-frameless-stage">
        <!-- 右侧顶部无框歌单/曲库信息栏 -->
        <div class="lib-right-frameless-header">
          <div class="lrf-info-left">
            <img
              v-if="rightStageCover"
              :src="rightStageCover"
              class="lrf-cover"
              :class="{ round: rightStageMode === 'artist' }"
              alt=""
            />
            <div class="lrf-title-block">
              <div class="lrf-eyebrow">{{ rightStageEyebrow }}</div>
              <div class="lrf-title-row">
                <h2 class="lrf-title">{{ rightStageTitle }}</h2>
                <span class="lrf-count">{{ rightStageSongs.length }} 首曲目</span>
              </div>
              <p v-if="rightStageSubtitle" class="lrf-sub">{{ rightStageSubtitle }}</p>
            </div>
          </div>

          <div class="lrf-actions-right">
            <div class="frameless-sublist-search">
              <PlumpIcon name="search" :size="15" />
              <input
                v-model="songFilterKw"
                type="text"
                placeholder="筛选歌名 / 歌手 / 专辑..."
              />
              <button v-if="songFilterKw" class="clear-inline-btn" @click="songFilterKw = ''">
                ×
              </button>
            </div>

            <button
              v-if="isCurrentSongInRightStage"
              class="frameless-hover-btn"
              title="定位到当前正在播放的歌曲"
              @click="scrollToPlayingSongInLibrary"
            >
              <PlumpIcon name="compass" :size="16" />
              <span>定位播放</span>
            </button>

            <button
              class="frameless-hover-btn primary-tint"
              @click="playAllRightStage"
            >
              <PlumpIcon name="play" :size="16" />
              <span>播放全部</span>
            </button>
          </div>
        </div>

        <!-- 右侧无框歌曲列表（双列高密度沉浸排布，无任何卡片框） -->
        <div v-if="filteredRightStageSongs.length === 0" class="lib-right-empty">
          <PlumpIcon name="music" :size="32" />
          <p>{{ store.isLoadingLibrary ? "正在同步歌单曲目..." : "当前列表暂无匹配的歌曲" }}</p>
        </div>

        <div v-else class="frameless-songs-grid-2col lib-frameless-songs">
          <div
            v-for="(song, idx) in filteredRightStageSongs"
            :key="song.hash + '_' + idx"
            :data-lib-song-hash="song.hash"
            class="frameless-song-row"
            :class="{ active: store.currentSong?.hash === song.hash }"
            @click="store.playSongFromList(song, rightStageSongs)"
            @contextmenu.prevent="store.openSongContextMenu($event, song, rightStageSongs)"
          >
            <span class="fs-index">
              <template v-if="store.currentSong?.hash === song.hash && store.isPlaying">
                <span class="mini-eq-bars"><i></i><i></i><i></i></span>
              </template>
              <template v-else>
                {{ String(idx + 1).padStart(2, "0") }}
              </template>
            </span>

            <div class="fs-cover-wrap">
              <img :src="song.coverUrl" class="fs-cover" loading="lazy" alt="" />
              <div class="fs-cover-play">
                <PlumpIcon
                  :name="store.currentSong?.hash === song.hash && store.isPlaying ? 'pause' : 'play'"
                  :size="15"
                  color="#fff"
                />
              </div>
            </div>

            <div class="fs-meta">
              <div class="fs-title-line">
                <span class="fs-title">{{ song.title }}</span>
                <span v-if="song.hasHiRes" class="fs-badge hires">Hi-Res</span>
                <span v-else-if="song.hasSq" class="fs-badge sq">SQ</span>
                <span v-if="song.isVip" class="fs-badge vip">VIP</span>
              </div>
              <div class="fs-sub-line">
                <span>{{ song.artist }}</span>
                <span v-if="song.album" class="fs-album">· {{ song.album }}</span>
              </div>
            </div>

            <div class="fs-actions" @click.stop>
              <button
                class="fs-icon-btn"
                :class="{ liked: store.isSongLiked(song.hash) }"
                title="喜欢"
                @click="store.toggleFavoriteSong(song)"
              >
                <PlumpIcon
                  :name="store.isSongLiked(song.hash) ? 'heart-filled' : 'heart'"
                  :size="16"
                />
              </button>
              <button
                class="fs-icon-btn"
                title="添加到歌单"
                @click="store.addToPlaylistModalSong = song"
              >
                <PlumpIcon name="plus" :size="16" />
              </button>
              <button
                class="fs-icon-btn"
                title="更多操作"
                @click="store.openSongContextMenu($event, song, rightStageSongs)"
              >
                <PlumpIcon name="more" :size="16" />
              </button>
            </div>

            <span class="fs-duration">{{ formatDuration(song.durationSeconds) }}</span>
          </div>
        </div>
      </main>
    </section>

    <!-- 新建歌单弹窗 -->
    <transition name="fade-pop">
      <div v-if="showCreateModal" class="modal-backdrop" @click.self="showCreateModal = false">
        <div class="compact-dialog glass-popover">
          <h3>新建自建歌单</h3>
          <p class="dialog-sub">创建后将同步至你的酷狗概念版云端资产</p>
          <input
            v-model="newPlaylistName"
            type="text"
            class="dialog-input"
            placeholder="请输入新歌单名称..."
            @keydown.enter="submitCreatePlaylist"
          />
          <div class="dialog-actions">
            <button class="frameless-hover-btn" @click="showCreateModal = false">取消</button>
            <button class="frameless-hover-btn primary-tint" @click="submitCreatePlaylist">
              立即创建
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 导入外部歌单弹窗 -->
    <transition name="fade-pop">
      <div v-if="showImportModal" class="modal-backdrop" @click.self="showImportModal = false">
        <div class="compact-dialog glass-popover">
          <h3>导入外部歌单</h3>
          <p class="dialog-sub">支持粘贴酷狗歌单码、分享口令或歌单链接自动解析收藏</p>
          <input
            v-model="importInputText"
            type="text"
            class="dialog-input"
            placeholder="粘贴歌单链接、口令或 global_collection_id..."
            @keydown.enter="submitImportPlaylist"
          />
          <div class="dialog-actions">
            <button class="frameless-hover-btn" @click="showImportModal = false">取消</button>
            <button class="frameless-hover-btn primary-tint" @click="submitImportPlaylist">
              解析并导入
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import type { FollowedArtistItem, PlaylistItem, PurchasedAlbumItem, Song } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

type RightStageMode = "liked" | "recent" | "playlist" | "artist" | "album" | "purchased_songs";
const rightStageMode = ref<RightStageMode>("liked");
const songFilterKw = ref("");

const showCreateModal = ref(false);
const newPlaylistName = ref("");
const showImportModal = ref(false);
const importInputText = ref("");

// 左上角四个分类标题严格沿用 App 端设计：自建、收藏、关注、已购
const assetTabs = computed(() => [
  {
    id: "created" as const,
    label: "自建",
    count: store.createdPlaylists.length,
    color: "#2b8cff",
  },
  {
    id: "collected" as const,
    label: "收藏",
    count: store.collectedPlaylists.length,
    color: "#ff6b6b",
  },
  {
    id: "artists" as const,
    label: "关注",
    count: store.followedArtists.length,
    color: "#15c493",
  },
  {
    id: "purchased" as const,
    label: "已购",
    count: store.purchasedAlbums.length + store.purchasedSongs.length,
    color: "#f5a623",
  },
]);

const activePlaylistGroup = computed(() => {
  return store.libraryAssetTab === "collected"
    ? store.collectedPlaylists
    : store.createdPlaylists;
});

function handleSelectAssetTab(tabId: "created" | "collected" | "artists" | "purchased") {
  store.libraryAssetTab = tabId;
  if (tabId === "artists" && store.followedArtists.length === 0) {
    store.loadFollowedArtists();
  } else if (tabId === "purchased" && store.purchasedAlbums.length === 0 && store.purchasedSongs.length === 0) {
    store.loadPurchasedAssets();
  }
}

function activateLikedMode() {
  rightStageMode.value = "liked";
  songFilterKw.value = "";
}

function activateRecentMode() {
  rightStageMode.value = "recent";
  songFilterKw.value = "";
  if (store.recentHistorySongs.length === 0) {
    store.loadRecentHistory();
  }
}

function selectPlaylistToRight(pl: PlaylistItem) {
  rightStageMode.value = "playlist";
  songFilterKw.value = "";
  store.selectPlaylist(pl);
}

async function selectArtistToRight(art: FollowedArtistItem) {
  rightStageMode.value = "artist";
  songFilterKw.value = "";
  await store.openArtistDetail(String(art.singerId || art.artistId || ""), art.name, art.avatarUrl);
  // 保持在当前分栏视图，不遮挡左栏
  store.libraryDetailMode = "playlists";
}

async function selectPurchasedAlbumToRight(alb: PurchasedAlbumItem) {
  rightStageMode.value = "album";
  songFilterKw.value = "";
  await store.openAlbumDetail(String(alb.id || alb.albumId || ""), alb.albumName, alb.singerName || alb.artistName || "", alb.coverUrl);
  store.libraryDetailMode = "playlists";
}

const rightStageSongs = computed<Song[]>(() => {
  switch (rightStageMode.value) {
    case "liked":
      return store.likedSongs;
    case "recent":
      return store.recentHistorySongs;
    case "playlist":
      return store.playlistSongs;
    case "artist":
      return store.activeArtistSongs;
    case "album":
      return store.activeAlbumSongs;
    case "purchased_songs":
      return store.purchasedSongs.map((item: any) => item.song || item);
    default:
      return store.likedSongs;
  }
});

const filteredRightStageSongs = computed(() => {
  const kw = songFilterKw.value.trim().toLowerCase();
  if (!kw) return rightStageSongs.value;
  return rightStageSongs.value.filter(
    (s) =>
      s.title.toLowerCase().includes(kw) ||
      s.artist.toLowerCase().includes(kw) ||
      (s.album && s.album.toLowerCase().includes(kw))
  );
});

const rightStageTitle = computed(() => {
  switch (rightStageMode.value) {
    case "liked":
      return "我喜欢的音乐";
    case "recent":
      return "最近播放记录";
    case "playlist":
      return store.selectedPlaylist?.name || "精选歌单";
    case "artist":
      return store.activeArtistDetail?.name || "关注歌手代表作";
    case "album":
      return store.activeAlbumDetail?.albumName || "已购数字专辑";
    case "purchased_songs":
      return "已购尊享单曲";
  }
});

const rightStageEyebrow = computed(() => {
  switch (rightStageMode.value) {
    case "liked":
      return "MY FAVORITES · 红心曲库";
    case "recent":
      return "RECENT PLAYED · 听歌足迹";
    case "playlist":
      return store.libraryAssetTab === "collected" ? "COLLECTED PLAYLIST · 收藏歌单" : "CREATED PLAYLIST · 自建歌单";
    case "artist":
      return "FOLLOWED ARTIST · 关注音乐人";
    case "album":
    case "purchased_songs":
      return "PURCHASED ASSET · 已购数字音乐";
  }
});

const rightStageSubtitle = computed(() => {
  if (rightStageMode.value === "playlist" && store.selectedPlaylist?.description) {
    return store.selectedPlaylist.description;
  }
  if (rightStageMode.value === "artist" && store.activeArtistDetail?.intro) {
    return store.activeArtistDetail.intro.slice(0, 80);
  }
  return "双击顶部导航栏「我的」可随时同步云端歌单与红心曲库";
});

const rightStageCover = computed(() => {
  switch (rightStageMode.value) {
    case "liked":
      return store.likedSongs[0]?.coverUrl || store.likedPlaylistMeta?.coverUrl || "";
    case "recent":
      return store.recentHistorySongs[0]?.coverUrl || "";
    case "playlist":
      return store.selectedPlaylist?.coverUrl || store.playlistSongs[0]?.coverUrl || "";
    case "artist":
      return store.activeArtistDetail?.avatarUrl || "";
    case "album":
      return store.activeAlbumDetail?.coverUrl || "";
    case "purchased_songs":
      return (store.purchasedSongs[0] as any)?.song?.coverUrl || (store.purchasedSongs[0] as any)?.coverUrl || "";
  }
});

function playAllLiked() {
  rightStageMode.value = "liked";
  if (store.likedSongs.length > 0) {
    store.playSongFromList(store.likedSongs[0], store.likedSongs);
  }
}

function playAllRecent() {
  rightStageMode.value = "recent";
  if (store.recentHistorySongs.length > 0) {
    store.playSongFromList(store.recentHistorySongs[0], store.recentHistorySongs);
  }
}

function playAllRightStage() {
  if (rightStageSongs.value.length > 0) {
    store.playSongFromList(rightStageSongs.value[0], rightStageSongs.value);
  }
}

const isCurrentSongInRightStage = computed(() => {
  if (!store.currentSong) return false;
  return rightStageSongs.value.some((s) => s.hash === store.currentSong?.hash);
});

function scrollToPlayingSongInLibrary() {
  if (!store.currentSong) return;
  const el = document.querySelector(`[data-lib-song-hash="${store.currentSong.hash}"]`);
  if (el) el.scrollIntoView({ behavior: "smooth", block: "center" });
}

function openCreateModal() {
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
  importInputText.value = "";
  showImportModal.value = true;
}

async function submitImportPlaylist() {
  const raw = importInputText.value.trim();
  if (!raw) return;
  showImportModal.value = false;
  await store.importExternalPlaylist(raw);
}

function confirmDeletePlaylist(pl: PlaylistItem) {
  if (window.confirm(`确定要删除自建歌单「${pl.name}」吗？`)) {
    store.deletePlaylist(pl);
  }
}

onMounted(() => {
  if (store.userPlaylists.length === 0) {
    store.loadUserPlaylists();
  }
  if (store.recentHistorySongs.length === 0) {
    store.loadRecentHistory();
  }
});
</script>
