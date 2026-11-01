<script setup lang="ts">
import { computed, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

// 新建歌单与导入外部歌单弹窗状态
const showCreateModal = ref(false);
const newPlaylistName = ref("");
const showImportModal = ref(false);
const importPlaylistName = ref("");
const importRawText = ref("");
const isImporting = ref(false);
const trackFilterKeyword = ref("");

const activeListSongs = computed(() => {
  const base =
    store.libraryAssetTab === "liked"
      ? store.likedSongs
      : store.libraryAssetTab === "recent"
      ? store.recentHistorySongs
      : store.playlistSongs;
  const kw = trackFilterKeyword.value.trim().toLowerCase();
  if (!kw) return base;
  return base.filter(
    (s) =>
      s.title.toLowerCase().includes(kw) ||
      s.artist.toLowerCase().includes(kw) ||
      s.album.toLowerCase().includes(kw)
  );
});

const rawActiveListCount = computed(() => {
  if (store.libraryAssetTab === "liked") {
    return Math.max(
      store.likedPlaylistMeta?.trackCount || 0,
      store.likedSongs.length
    );
  }
  if (store.libraryAssetTab === "recent") {
    return store.recentHistorySongs.length;
  }
  return Math.max(
    store.selectedPlaylist?.trackCount || 0,
    store.playlistSongs.length
  );
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
  store.libraryDetailMode = "playlist";
  store.libraryAssetTab = "liked";
  trackFilterKeyword.value = "";
  if (store.likedSongs.length === 0 && store.likedPlaylistMeta) {
    store.selectPlaylist(store.likedPlaylistMeta);
  }
}

function openRecentSongs() {
  store.libraryDetailMode = "playlist";
  store.libraryAssetTab = "recent";
  trackFilterKeyword.value = "";
  if (store.recentHistorySongs.length === 0) {
    store.loadRecentHistory();
  }
}

function locateCurrentPlayingSong() {
  const el = document.querySelector(".track-card-playing");
  if (el) {
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    store.notify("已为您平滑居中定位到当前播放歌曲");
  } else {
    store.notify("当前播放歌曲不在本歌单列表中");
  }
}

async function handleCreateSubmit() {
  if (!newPlaylistName.value.trim()) return;
  const ok = await store.createNewPlaylist(newPlaylistName.value.trim());
  if (ok) {
    newPlaylistName.value = "";
    showCreateModal.value = false;
  }
}

async function handleImportSubmit() {
  if (!importPlaylistName.value.trim() || !importRawText.value.trim()) return;
  isImporting.value = true;
  try {
    await store.importExternalPlaylist(
      importPlaylistName.value.trim(),
      importRawText.value
    );
    importPlaylistName.value = "";
    importRawText.value = "";
    showImportModal.value = false;
  } finally {
    isImporting.value = false;
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
  <div class="page-scroll-canvas" @click="store.closeSongContextMenu()">
    <!-- ==================== 子视图 A：歌手主页详情 (ArtistDetailScreen：热门单曲 / 专辑列表 / 歌手百科) ==================== -->
    <template
      v-if="store.libraryDetailMode === 'artist' && store.activeArtistDetail"
    >
      <div class="subpage-banner">
        <div class="subpage-top-bar">
          <button
            class="tonal-pill-btn"
            @click="store.libraryDetailMode = 'playlist'"
          >
            <PlumpIcon name="back" :size="15" />
            <span>返回我的音乐</span>
          </button>

          <div class="pill-tabs">
            <button
              class="pill-tab-btn"
              :class="{ active: store.artistDetailSubTab === 'songs' }"
              @click="store.artistDetailSubTab = 'songs'"
            >
              代表单曲 ({{
                store.activeArtistDetail.songCount ||
                store.activeArtistSongs.length
              }})
            </button>
            <button
              class="pill-tab-btn"
              :class="{ active: store.artistDetailSubTab === 'albums' }"
              @click="store.artistDetailSubTab = 'albums'"
            >
              发行专辑 ({{
                store.activeArtistAlbums.length ||
                store.activeArtistDetail.albumCount ||
                0
              }})
            </button>
            <button
              class="pill-tab-btn"
              :class="{ active: store.artistDetailSubTab === 'about' }"
              @click="store.artistDetailSubTab = 'about'"
            >
              歌手百科简介
            </button>
          </div>

          <div
            v-if="store.artistDetailSubTab === 'songs'"
            class="pill-tabs"
          >
            <button
              class="pill-tab-btn"
              :class="{ active: store.artistSongsSort === 'hot' }"
              @click="
                store.artistSongsSort = 'hot';
                store.openArtistDetail(
                  store.activeArtistDetail!.artistId,
                  store.activeArtistDetail!.name,
                  store.activeArtistDetail!.avatarUrl
                );
              "
            >
              最热
            </button>
            <button
              class="pill-tab-btn"
              :class="{ active: store.artistSongsSort === 'new' }"
              @click="
                store.artistSongsSort = 'new';
                store.openArtistDetail(
                  store.activeArtistDetail!.artistId,
                  store.activeArtistDetail!.name,
                  store.activeArtistDetail!.avatarUrl
                );
              "
            >
              最新
            </button>
          </div>
        </div>

        <div class="artist-hero-row">
          <img
            v-if="store.activeArtistDetail.avatarUrl"
            :src="store.activeArtistDetail.avatarUrl"
            class="artist-hero-avatar"
          />
          <div v-else class="artist-hero-avatar fallback">
            {{ store.activeArtistDetail.name.slice(0, 1) }}
          </div>
          <div class="artist-hero-meta">
            <h2 class="artist-hero-name">{{ store.activeArtistDetail.name }}</h2>
            <p class="artist-hero-stats">
              单曲
              {{
                store.activeArtistDetail.songCount ||
                store.activeArtistSongs.length
              }}
              首 · 专辑
              {{
                store.activeArtistDetail.albumCount ||
                store.activeArtistAlbums.length ||
                12
              }}
              张 · 粉丝 {{ store.activeArtistDetail.fansCount || "18.6w" }}
            </p>
            <p v-if="store.activeArtistDetail.intro" class="artist-hero-intro">
              {{ store.activeArtistDetail.intro }}
            </p>
          </div>
          <div class="artist-hero-actions">
            <button
              class="tonal-pill-btn"
              :class="{ primary: !store.activeArtistDetail.isFollowed }"
              @click="store.toggleFollowArtist(store.activeArtistDetail!)"
            >
              <PlumpIcon
                :name="store.activeArtistDetail.isFollowed ? 'check' : 'plus'"
                :size="15"
              />
              <span>
                {{
                  store.activeArtistDetail.isFollowed ? "已关注" : "关注歌手"
                }}
              </span>
            </button>
            <button
              v-if="store.activeArtistSongs.length > 0"
              class="coral-play-all-pill sm"
              @click="
                store.playSongFromList(
                  store.activeArtistSongs[0],
                  store.activeArtistSongs,
                  store.activeArtistDetail!.name
                )
              "
            >
              <PlumpIcon name="play" :size="15" color="#ffffff" />
              <span>播放热门 ({{ store.activeArtistSongs.length }})</span>
            </button>
          </div>
        </div>
      </div>

      <div v-if="store.isLoadingArtist" class="loading-box">
        正在加载歌手代表作品与概念版无损音源规格...
      </div>

      <!-- 1. 歌手热门单曲 -->
      <div
        v-else-if="store.artistDetailSubTab === 'songs'"
        class="dual-column-song-grid"
      >
        <div
          v-for="(song, idx) in store.activeArtistSongs"
          :key="song.hash || idx"
          class="plump-song-card"
          :class="{
            'track-card-playing': store.currentSong?.hash === song.hash,
          }"
          @click="
            store.playSongFromList(
              song,
              store.activeArtistSongs,
              store.activeArtistDetail.name
            )
          "
          @contextmenu="store.openSongContextMenu($event, song)"
        >
          <div class="psc-index">{{ String(idx + 1).padStart(2, "0") }}</div>
          <img v-if="song.coverUrl" :src="song.coverUrl" class="psc-cover" />
          <div class="psc-main">
            <div class="psc-title">{{ song.title }}</div>
            <div class="psc-sub">
              <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
              <span v-else-if="song.isLossless" class="mini-badge sq">SQ</span>
              <span v-if="song.isVip" class="mini-badge vip">VIP</span>
              <span>{{ song.artist }}</span>
              <span v-if="song.album" class="psc-album">· {{ song.album }}</span>
            </div>
          </div>
          <div class="psc-actions" @click.stop>
            <button
              class="psc-act-btn"
              title="喜欢"
              @click="store.toggleFavoriteSong(song)"
            >
              <PlumpIcon
                name="heart"
                :size="15"
                :filled="store.isSongLiked(song)"
                :color="store.isSongLiked(song) ? '#ff4d6d' : 'currentColor'"
              />
            </button>
            <button
              class="psc-act-btn"
              title="添加到歌单"
              @click="store.addToPlaylistModalSong = song"
            >
              <PlumpIcon name="playlist-add" :size="15" />
            </button>
            <button
              class="psc-act-btn"
              title="更多操作"
              @click="store.openSongContextMenu($event, song)"
            >
              <PlumpIcon name="more" :size="15" />
            </button>
          </div>
          <div class="psc-dur">{{ formatDuration(song.durationMs) }}</div>
        </div>
      </div>

      <!-- 2. 歌手发行专辑网格 -->
      <div
        v-else-if="store.artistDetailSubTab === 'albums'"
        class="album-cards-grid"
      >
        <div
          v-if="store.activeArtistAlbums.length === 0"
          class="empty-box"
          style="grid-column: 1 / -1"
        >
          暂未获取到该歌手的独立数字专辑列表
        </div>
        <div
          v-for="alb in store.activeArtistAlbums"
          :key="alb.albumId"
          class="plump-album-card"
          @click="
            store.openAlbumDetail(
              alb.albumId,
              alb.albumName,
              alb.artistName || store.activeArtistDetail.name,
              alb.coverUrl
            )
          "
        >
          <img v-if="alb.coverUrl" :src="alb.coverUrl" class="palb-cover" />
          <div class="palb-info">
            <div class="palb-name">{{ alb.albumName }}</div>
            <div class="palb-sub">
              {{ alb.publishDate || "数字发行" }} · {{ alb.songCount || 10 }} 首
            </div>
          </div>
        </div>
      </div>

      <!-- 3. 歌手百科长简介 -->
      <div v-else class="settings-card-panel">
        <h3>关于 {{ store.activeArtistDetail.name }}</h3>
        <p
          style="
            font-size: 13.5px;
            line-height: 1.75;
            color: var(--text-secondary);
            white-space: pre-wrap;
            margin-top: 10px;
          "
        >
          {{
            store.activeArtistDetail.intro ||
            "酷狗概念版官方音乐人，收录高解析度 Hi-Res 与 SQ 无损母带作品。"
          }}
        </p>
      </div>
    </template>

    <!-- ==================== 子视图 B：专辑详情页 (AlbumDetailScreen) ==================== -->
    <template
      v-else-if="store.libraryDetailMode === 'album' && store.activeAlbumDetail"
    >
      <div class="subpage-banner">
        <div class="subpage-top-bar">
          <button
            class="tonal-pill-btn"
            @click="store.libraryDetailMode = 'playlist'"
          >
            <PlumpIcon name="back" :size="15" />
            <span>返回我的音乐</span>
          </button>
        </div>

        <div class="artist-hero-row">
          <img
            v-if="store.activeAlbumDetail.coverUrl"
            :src="store.activeAlbumDetail.coverUrl"
            class="album-hero-cover"
          />
          <div class="artist-hero-meta">
            <h2 class="artist-hero-name">
              {{ store.activeAlbumDetail.albumName }}
            </h2>
            <p class="artist-hero-stats">
              歌手：{{ store.activeAlbumDetail.artistName || "群星" }} ·
              发行日期：{{
                store.activeAlbumDetail.publishDate || "数字正版"
              }}
              · 出品：{{
                store.activeAlbumDetail.publishCompany || "概念版正版曲库"
              }}
              · 共 {{ store.activeAlbumSongs.length }} 首曲目
            </p>
            <p v-if="store.activeAlbumDetail.intro" class="artist-hero-intro">
              {{ store.activeAlbumDetail.intro }}
            </p>
          </div>
          <button
            v-if="store.activeAlbumSongs.length > 0"
            class="coral-play-all-pill sm"
            @click="
              store.playSongFromList(
                store.activeAlbumSongs[0],
                store.activeAlbumSongs,
                store.activeAlbumDetail!.albumName
              )
            "
          >
            <PlumpIcon name="play" :size="15" color="#ffffff" />
            <span>播放整张专辑 ({{ store.activeAlbumSongs.length }})</span>
          </button>
        </div>
      </div>

      <div v-if="store.isLoadingAlbum" class="loading-box">
        正在解析专辑曲目与无损音质标签...
      </div>
      <div v-else class="dual-column-song-grid">
        <div
          v-for="(song, idx) in store.activeAlbumSongs"
          :key="song.hash || idx"
          class="plump-song-card"
          :class="{
            'track-card-playing': store.currentSong?.hash === song.hash,
          }"
          @click="
            store.playSongFromList(
              song,
              store.activeAlbumSongs,
              store.activeAlbumDetail.albumName
            )
          "
          @contextmenu="store.openSongContextMenu($event, song)"
        >
          <div class="psc-index">{{ String(idx + 1).padStart(2, "0") }}</div>
          <img v-if="song.coverUrl" :src="song.coverUrl" class="psc-cover" />
          <div class="psc-main">
            <div class="psc-title">{{ song.title }}</div>
            <div class="psc-sub">
              <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
              <span v-else-if="song.isLossless" class="mini-badge sq">SQ</span>
              <span v-if="song.isVip" class="mini-badge vip">VIP</span>
              <span>{{ song.artist }}</span>
            </div>
          </div>
          <div class="psc-actions" @click.stop>
            <button
              class="psc-act-btn"
              title="喜欢"
              @click="store.toggleFavoriteSong(song)"
            >
              <PlumpIcon
                name="heart"
                :size="15"
                :filled="store.isSongLiked(song)"
                :color="store.isSongLiked(song) ? '#ff4d6d' : 'currentColor'"
              />
            </button>
            <button
              class="psc-act-btn"
              title="添加到歌单"
              @click="store.addToPlaylistModalSong = song"
            >
              <PlumpIcon name="playlist-add" :size="15" />
            </button>
          </div>
          <div class="psc-dur">{{ formatDuration(song.durationMs) }}</div>
        </div>
      </div>
    </template>

    <!-- ==================== 主视图：我的音乐资产与数字名片中心 ==================== -->
    <template v-else>
      <header class="discover-top-header">
        <div>
          <h1 class="discover-main-title">我的音乐资产</h1>
          <div class="discover-greeting-line">
            <span class="greeting-dot"></span>
            <span>云端歌单、听歌等级、关注歌手与数字专辑实时双向同步</span>
          </div>
        </div>

        <div v-if="store.isLoggedIn" class="discover-header-actions">
          <button
            class="tonal-pill-btn"
            @click="store.refreshAccountAndVip(false)"
          >
            <PlumpIcon
              name="refresh"
              :size="15"
              :class="{ 'spin-icon': store.isLoadingLibrary }"
            />
            <span>同步云端资产</span>
          </button>
          <button
            class="tonal-pill-btn"
            @click="store.isLoginModalOpen = true"
          >
            <span>切换账号</span>
          </button>
        </div>
      </header>

      <!-- 未登录引导卡片 -->
      <div v-if="!store.isLoggedIn" class="unauth-bento-card">
        <div class="unauth-bento-inner">
          <PlumpIcon name="crown" :size="36" color="#ffd54f" />
          <h3>登录酷狗概念版账号</h3>
          <p>
            支持酷狗 App 扫码登录与手机验证码登录（含同手机号多账号切换）。登录后自动同步「我喜欢的音乐」、云端歌单、关注歌手、已购数字专辑及听歌等级时长，并解锁每日免费概念版 VIP。
          </p>
          <button class="primary-btn" @click="store.isLoginModalOpen = true">
            <PlumpIcon name="crown" :size="16" color="#ffffff" />
            <span>立即登录账号</span>
          </button>
        </div>
      </div>

      <template v-else>
        <!-- 上层：快捷三联胶囊矩阵（我喜欢的音乐 + 最近播放 + 概念胶囊 Focus Pod 心流舱） -->
        <section class="library-shortcut-trio">
          <div
            class="lib-shortcut-card liked-card"
            :class="{ active: store.libraryAssetTab === 'liked' }"
            @click="openLikedSongs"
          >
            <div class="sc-top">
              <div class="sc-icon-circle red">
                <PlumpIcon name="heart-filled" :size="20" color="#ff4d6d" />
              </div>
              <span class="sc-pill-tag">红心同步</span>
            </div>
            <div class="sc-bottom">
              <div class="sc-title">我喜欢的音乐</div>
              <div class="sc-count">
                {{
                  Math.max(
                    store.likedPlaylistMeta?.trackCount || 0,
                    store.likedSongs.length
                  )
                }}
                首珍藏曲目
              </div>
            </div>
          </div>

          <div
            class="lib-shortcut-card recent-card"
            :class="{ active: store.libraryAssetTab === 'recent' }"
            @click="openRecentSongs"
          >
            <div class="sc-top">
              <div class="sc-icon-circle cyan">
                <PlumpIcon name="history" :size="20" color="#00e5ff" />
              </div>
              <span class="sc-pill-tag">多端漫游</span>
            </div>
            <div class="sc-bottom">
              <div class="sc-title">最近播放</div>
              <div class="sc-count">
                {{ store.recentHistorySongs.length }} 首聆听足迹
              </div>
            </div>
          </div>

          <div
            class="lib-shortcut-card focus-card"
            @click="store.isFocusPodOpen = true"
          >
            <div class="sc-top">
              <div class="sc-icon-circle violet">
                <PlumpIcon
                  name="timer"
                  :size="20"
                  color="#b388ff"
                  accent="#00e5ff"
                />
              </div>
              <span class="sc-pill-tag violet">开启心流 &gt;</span>
            </div>
            <div class="sc-bottom">
              <div class="sc-title">概念胶囊 · Focus Pod</div>
              <div class="sc-count">番茄钟专注计时 × 4 路实时白噪音混音</div>
            </div>
          </div>
        </section>

        <!-- 下层：开放式音乐资产滑块导航栏（自建 / 收藏 / 关注 / 已购 / 我喜欢 / 最近） -->
        <section class="library-explorer-section">
          <div class="explorer-tabs-bar">
            <div class="explorer-tabs-left">
              <button
                class="exp-tab"
                :class="{ active: store.libraryAssetTab === 'created' }"
                @click="
                  store.libraryAssetTab = 'created';
                  if (
                    store.selectedPlaylist?.isLiked &&
                    store.createdPlaylists.length > 0
                  ) {
                    store.selectPlaylist(store.createdPlaylists[0]);
                  }
                "
              >
                自建歌单 <span>{{ store.createdPlaylists.length }}</span>
              </button>
              <button
                class="exp-tab"
                :class="{ active: store.libraryAssetTab === 'collected' }"
                @click="
                  store.libraryAssetTab = 'collected';
                  if (store.collectedPlaylists.length > 0) {
                    store.selectPlaylist(store.collectedPlaylists[0]);
                  }
                "
              >
                收藏歌单 <span>{{ store.collectedPlaylists.length }}</span>
              </button>
              <button
                class="exp-tab"
                :class="{ active: store.libraryAssetTab === 'followed' }"
                @click="
                  store.libraryAssetTab = 'followed';
                  store.followedArtists.length === 0 &&
                    store.loadFollowedArtists();
                "
              >
                关注歌手 <span>{{ store.followedArtists.length }}</span>
              </button>
              <button
                class="exp-tab"
                :class="{ active: store.libraryAssetTab === 'purchased' }"
                @click="
                  store.libraryAssetTab = 'purchased';
                  store.purchasedAlbums.length === 0 &&
                    store.loadPurchasedAssets();
                "
              >
                已购音乐
                <span>
                  {{
                    store.purchasedAlbums.length + store.purchasedSongs.length
                  }}
                </span>
              </button>
              <button
                class="exp-tab"
                :class="{ active: store.libraryAssetTab === 'liked' }"
                @click="openLikedSongs"
              >
                我喜欢
                <span>
                  {{
                    Math.max(
                      store.likedPlaylistMeta?.trackCount || 0,
                      store.likedSongs.length
                    )
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

            <!-- 右侧随当前资产 Tab 动态显示的软润无框工具按钮 -->
            <div class="explorer-tabs-tools">
              <template v-if="store.libraryAssetTab === 'created'">
                <button
                  class="tonal-pill-btn sm"
                  title="新建自定义歌单"
                  @click="showCreateModal = true"
                >
                  <PlumpIcon name="plus" :size="14" color="var(--cyan)" />
                  <span>新建歌单</span>
                </button>
                <button
                  class="tonal-pill-btn sm"
                  title="批量匹配导入外部歌单"
                  @click="showImportModal = true"
                >
                  <PlumpIcon name="upload" :size="14" />
                  <span>导入外部歌单</span>
                </button>
              </template>
              <button
                v-if="
                  store.libraryAssetTab === 'created' ||
                  store.libraryAssetTab === 'collected'
                "
                class="tonal-pill-btn sm"
                :title="
                  store.isLibraryGridView ? '切换为紧凑列表' : '切换为双列网格'
                "
                @click="store.isLibraryGridView = !store.isLibraryGridView"
              >
                <PlumpIcon
                  :name="store.isLibraryGridView ? 'list' : 'grid'"
                  :size="14"
                />
              </button>
            </div>
          </div>

          <!-- 分支 1：关注歌手画廊网格 (followedArtists) -->
          <div
            v-if="store.libraryAssetTab === 'followed'"
            class="followed-artists-stage"
          >
            <div
              v-if="store.followedArtists.length === 0"
              class="empty-box"
            >
              <PlumpIcon name="compass" :size="32" />
              <p>暂未获取到已关注的歌手，可在搜索或播放页点击歌手名字添加关注</p>
            </div>

            <div v-else class="artist-cards-grid">
              <div
                v-for="art in store.followedArtists"
                :key="art.artistId"
                class="plump-artist-card"
                @click="
                  store.openArtistDetail(art.artistId, art.name, art.avatarUrl)
                "
              >
                <img
                  v-if="art.avatarUrl"
                  :src="art.avatarUrl"
                  class="pac-avatar"
                />
                <div v-else class="pac-avatar fallback">
                  {{ art.name.slice(0, 1) }}
                </div>
                <div class="pac-info">
                  <div class="pac-name">{{ art.name }}</div>
                  <div class="pac-sub">
                    单曲 {{ art.songCount }} 首 · 专辑 {{ art.albumCount }} 张
                  </div>
                </div>
                <button
                  class="tonal-pill-btn sm"
                  @click.stop="store.toggleFollowArtist(art)"
                >
                  {{ art.isFollowed ? "已关注" : "关注" }}
                </button>
              </div>
            </div>
          </div>

          <!-- 分支 2：已购数字专辑与已购单曲 (purchasedAlbums & purchasedSongs) -->
          <div
            v-else-if="store.libraryAssetTab === 'purchased'"
            class="purchased-assets-stage"
          >
            <div
              v-if="
                store.purchasedAlbums.length === 0 &&
                store.purchasedSongs.length === 0
              "
              class="empty-box"
            >
              <PlumpIcon name="disc" :size="32" />
              <p>当前账号暂无已购数字专辑或单曲资产</p>
            </div>

            <template v-else>
              <div v-if="store.purchasedAlbums.length > 0" class="mb-18">
                <h4 class="sub-sec-title">
                  数字专辑 · {{ store.purchasedAlbums.length }}
                </h4>
                <div class="album-cards-grid">
                  <div
                    v-for="alb in store.purchasedAlbums"
                    :key="alb.albumId"
                    class="plump-album-card"
                    @click="
                      store.openAlbumDetail(
                        alb.albumId,
                        alb.albumName,
                        alb.artistName,
                        alb.coverUrl
                      )
                    "
                  >
                    <img
                      v-if="alb.coverUrl"
                      :src="alb.coverUrl"
                      class="palb-cover"
                    />
                    <div class="palb-info">
                      <div class="palb-name">{{ alb.albumName }}</div>
                      <div class="palb-sub">
                        {{ alb.artistName }} · {{ alb.songCount }} 首
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <div v-if="store.purchasedSongs.length > 0">
                <h4 class="sub-sec-title">
                  已购单曲 · {{ store.purchasedSongs.length }}
                </h4>
                <div class="dual-column-song-grid">
                  <div
                    v-for="(item, idx) in store.purchasedSongs"
                    :key="item.song.hash || idx"
                    class="plump-song-card"
                    @click="
                      store.playSongFromList(
                        item.song,
                        store.purchasedSongs.map((p) => p.song),
                        '已购单曲'
                      )
                    "
                    @contextmenu="store.openSongContextMenu($event, item.song)"
                  >
                    <img
                      v-if="item.song.coverUrl"
                      :src="item.song.coverUrl"
                      class="psc-cover"
                    />
                    <div class="psc-main">
                      <div class="psc-title">{{ item.song.title }}</div>
                      <div class="psc-sub">
                        <span v-if="item.song.isHiRes" class="mini-badge hires">
                          Hi-Res
                        </span>
                        <span
                          v-else-if="item.song.isLossless"
                          class="mini-badge sq"
                        >
                          SQ
                        </span>
                        <span>{{ item.song.artist }}</span>
                      </div>
                    </div>
                    <div class="psc-dur">
                      {{ formatDuration(item.song.durationMs) }}
                    </div>
                  </div>
                </div>
              </div>
            </template>
          </div>

          <!-- 分支 3：自建歌单 / 收藏歌单 / 我喜欢 / 最近播放（左窄右宽同屏展开工作台） -->
          <div v-else class="explorer-workspace">
            <!-- 左栏：歌单胶片画廊（当处于自建或收藏 Tab 时展示） -->
            <div
              v-if="
                store.libraryAssetTab === 'created' ||
                store.libraryAssetTab === 'collected'
              "
              class="explorer-playlists-col"
              :class="{ 'grid-mode': store.isLibraryGridView }"
            >
              <!-- 置顶展示「我喜欢」以便在自建歌单列随时切回 -->
              <div
                v-if="
                  store.libraryAssetTab === 'created' && store.likedPlaylistMeta
                "
                class="exp-playlist-item"
                :class="{
                  active:
                    store.selectedPlaylist?.id === store.likedPlaylistMeta.id,
                }"
                @click="store.selectPlaylist(store.likedPlaylistMeta)"
              >
                <img
                  v-if="store.likedPlaylistMeta.coverUrl"
                  :src="store.likedPlaylistMeta.coverUrl"
                  class="exp-pl-cover"
                />
                <div v-else class="exp-pl-cover-fallback">
                  <PlumpIcon name="heart-filled" :size="20" color="#ff4d6d" />
                </div>
                <div class="exp-pl-info">
                  <div class="exp-pl-title">
                    {{ store.likedPlaylistMeta.title }}
                  </div>
                  <div class="exp-pl-sub">
                    {{
                      Math.max(
                        store.likedPlaylistMeta.trackCount,
                        store.likedSongs.length
                      )
                    }}
                    首歌曲
                  </div>
                </div>
              </div>

              <div
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
                <div v-else class="exp-pl-cover-fallback">
                  <PlumpIcon name="disc" :size="20" />
                </div>
                <div class="exp-pl-info">
                  <div class="exp-pl-title" :title="pl.title">
                    {{ pl.title }}
                  </div>
                  <div class="exp-pl-sub">{{ pl.trackCount }} 首歌曲</div>
                </div>
                <button
                  v-if="pl.isCreatedByMe && !pl.isLiked && pl.listId"
                  class="exp-pl-del-btn"
                  title="删除该自建歌单"
                  @click.stop="store.deletePlaylist(pl)"
                >
                  <PlumpIcon name="close" :size="12" />
                </button>
              </div>
            </div>

            <!-- 右栏：当前选中歌单 / 我喜欢 / 最近播放的双列曲目流 -->
            <div class="explorer-tracks-col">
              <div class="exp-tracks-header">
                <div class="eth-left">
                  <img
                    v-if="
                      store.libraryAssetTab !== 'liked' &&
                      store.libraryAssetTab !== 'recent' &&
                      store.selectedPlaylist?.coverUrl
                    "
                    :src="store.selectedPlaylist.coverUrl"
                    class="eth-cover"
                  />
                  <div>
                    <h3 class="exp-tracks-title">{{ activeListTitle }}</h3>
                    <span class="exp-tracks-count">
                      共 {{ rawActiveListCount }} 首曲目（已加载
                      {{ activeListSongs.length }} 首无损与高品曲目）
                    </span>
                  </div>
                </div>

                <div class="eth-actions">
                  <!-- 歌单内歌曲快速检索过滤框 -->
                  <div class="rank-search-box" style="max-width: 220px; height: 34px">
                    <PlumpIcon
                      name="search"
                      :size="14"
                      color="var(--text-secondary)"
                    />
                    <input
                      v-model="trackFilterKeyword"
                      type="text"
                      placeholder="筛选歌单内歌曲..."
                      class="rank-search-input"
                    />
                  </div>

                  <button
                    class="tonal-pill-btn sm"
                    title="定位到当前正在播放的歌曲"
                    @click="locateCurrentPlayingSong"
                  >
                    <PlumpIcon name="locate" :size="14" />
                    <span>定位播放</span>
                  </button>

                  <button
                    v-if="
                      store.selectedPlaylist &&
                      !store.selectedPlaylist.isCreatedByMe &&
                      store.libraryAssetTab !== 'liked' &&
                      store.libraryAssetTab !== 'recent'
                    "
                    class="tonal-pill-btn sm"
                    @click="store.toggleCollectPlaylist(store.selectedPlaylist)"
                  >
                    <PlumpIcon
                      name="heart"
                      :size="15"
                      :filled="
                        store.isPlaylistCollected(store.selectedPlaylist)
                      "
                      :color="
                        store.isPlaylistCollected(store.selectedPlaylist)
                          ? '#ff4d6d'
                          : 'currentColor'
                      "
                    />
                    <span>
                      {{
                        store.isPlaylistCollected(store.selectedPlaylist)
                          ? "已收藏"
                          : "收藏歌单"
                      }}
                    </span>
                  </button>

                  <button
                    v-if="activeListSongs.length > 0"
                    class="coral-play-all-pill sm"
                    @click="
                      store.playSongFromList(
                        activeListSongs[0],
                        activeListSongs,
                        activeListTitle
                      )
                    "
                  >
                    <PlumpIcon name="play" :size="15" color="#ffffff" />
                    <span>播放全部 ({{ activeListSongs.length }})</span>
                  </button>
                </div>
              </div>

              <div v-if="store.isLoadingLibrary" class="loading-box">
                正在同步云端全量曲目与概念版无损音源规格...
              </div>

              <div
                v-else-if="activeListSongs.length === 0"
                class="empty-box"
              >
                <PlumpIcon name="sparkles" :size="30" />
                <p>当前列表暂无匹配的歌曲，请从左侧选择有曲目的歌单</p>
              </div>

              <div v-else class="dual-column-song-grid">
                <div
                  v-for="(song, idx) in activeListSongs"
                  :key="song.hash || song.id"
                  class="plump-song-card"
                  :class="{
                    'track-card-playing': store.currentSong?.hash === song.hash,
                  }"
                  @click="
                    store.playSongFromList(
                      song,
                      activeListSongs,
                      activeListTitle
                    )
                  "
                  @contextmenu="store.openSongContextMenu($event, song)"
                >
                  <div class="psc-index">
                    <PlumpIcon
                      v-if="store.currentSong?.hash === song.hash"
                      name="wave"
                      :size="16"
                      color="var(--cyan)"
                    />
                    <span v-else>{{ String(idx + 1).padStart(2, "0") }}</span>
                  </div>
                  <img
                    v-if="song.coverUrl"
                    :src="song.coverUrl"
                    class="psc-cover"
                    loading="lazy"
                  />
                  <div v-else class="psc-cover-fallback">
                    <PlumpIcon name="disc" :size="18" />
                  </div>
                  <div class="psc-main">
                    <div class="psc-title">{{ song.title }}</div>
                    <div class="psc-sub">
                      <span v-if="song.isHiRes" class="mini-badge hires">
                        Hi-Res
                      </span>
                      <span v-else-if="song.isLossless" class="mini-badge sq">
                        SQ
                      </span>
                      <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                      <span>{{ song.artist }}</span>
                      <span v-if="song.album" class="psc-album">
                        · {{ song.album }}
                      </span>
                    </div>
                  </div>
                  <div class="psc-actions" @click.stop>
                    <button
                      class="psc-act-btn"
                      title="喜欢"
                      @click="store.toggleFavoriteSong(song)"
                    >
                      <PlumpIcon
                        name="heart"
                        :size="15"
                        :filled="store.isSongLiked(song)"
                        :color="
                          store.isSongLiked(song) ? '#ff4d6d' : 'currentColor'
                        "
                      />
                    </button>
                    <button
                      class="psc-act-btn"
                      title="添加到歌单"
                      @click="store.addToPlaylistModalSong = song"
                    >
                      <PlumpIcon name="playlist-add" :size="15" />
                    </button>
                    <button
                      class="psc-act-btn"
                      title="更多操作"
                      @click="store.openSongContextMenu($event, song)"
                    >
                      <PlumpIcon name="more" :size="15" />
                    </button>
                  </div>
                  <div class="psc-dur">{{ formatDuration(song.durationMs) }}</div>
                </div>
              </div>
            </div>
          </div>
        </section>
      </template>
    </template>

    <!-- 新建歌单弹窗 (对齐 CreatePlaylistDialog.kt) -->
    <transition name="fade">
      <div
        v-if="showCreateModal"
        class="modal-backdrop"
        @click.self="showCreateModal = false"
      >
        <div class="compact-glass-modal">
          <div class="modal-header">
            <h3>新建云端自定义歌单</h3>
            <button class="icon-close-btn" @click="showCreateModal = false">
              <PlumpIcon name="close" :size="15" />
            </button>
          </div>
          <input
            v-model="newPlaylistName"
            type="text"
            class="plump-text-input mb-14"
            placeholder="输入新歌单名称（例如：深夜写代码专属）"
            @keydown.enter="handleCreateSubmit"
          />
          <button class="primary-btn full-w" @click="handleCreateSubmit">
            <PlumpIcon name="plus" :size="16" color="#ffffff" />
            <span>立即创建并同步云端</span>
          </button>
        </div>
      </div>
    </transition>

    <!-- 导入外部歌单弹窗 (对齐 LibraryScreen.kt 导入外部歌单) -->
    <transition name="fade">
      <div
        v-if="showImportModal"
        class="modal-backdrop"
        @click.self="showImportModal = false"
      >
        <div class="compact-glass-modal">
          <div class="modal-header">
            <h3>批量匹配导入外部歌单</h3>
            <button class="icon-close-btn" @click="showImportModal = false">
              <PlumpIcon name="close" :size="15" />
            </button>
          </div>
          <input
            v-model="importPlaylistName"
            type="text"
            class="plump-text-input mb-10"
            placeholder="新歌单名称（例如：我的精选导入）"
          />
          <textarea
            v-model="importRawText"
            rows="6"
            class="plump-textarea mb-14"
            placeholder="曲目列表（每行一首：歌名 - 歌手）&#10;晴天 - 周杰伦&#10;江南 - 林俊杰"
          ></textarea>
          <button
            class="primary-btn full-w"
            :disabled="isImporting"
            @click="handleImportSubmit"
          >
            <PlumpIcon name="upload" :size="16" color="#ffffff" />
            <span>
              {{ isImporting ? "正在智能匹配导入..." : "开始匹配导入" }}
            </span>
          </button>
        </div>
      </div>
    </transition>
  </div>
</template>
