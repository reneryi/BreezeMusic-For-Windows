<script setup lang="ts">
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

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
    <header class="discover-top-header">
      <div>
        <h1 class="discover-main-title">
          全曲库检索 · 「{{ store.searchKeyword || "热搜漫游" }}」
        </h1>
        <div class="discover-greeting-line">
          <span class="greeting-dot"></span>
          <span>
            支持单曲、歌单（双向收藏同步）、数字专辑与歌手四维直连搜索
          </span>
        </div>
      </div>

      <!-- 四维分类胶囊切换 -->
      <div class="pill-tabs">
        <button
          class="pill-tab-btn"
          :class="{ active: store.searchCategoryTab === 'songs' }"
          @click="store.searchCategoryTab = 'songs'"
        >
          单曲 ({{ store.searchResultsSongs.length }})
        </button>
        <button
          class="pill-tab-btn"
          :class="{ active: store.searchCategoryTab === 'playlists' }"
          @click="store.searchCategoryTab = 'playlists'"
        >
          歌单 ({{ store.searchResultsPlaylists.length }})
        </button>
        <button
          class="pill-tab-btn"
          :class="{ active: store.searchCategoryTab === 'albums' }"
          @click="store.searchCategoryTab = 'albums'"
        >
          专辑 ({{ store.searchResultsAlbums.length }})
        </button>
        <button
          class="pill-tab-btn"
          :class="{ active: store.searchCategoryTab === 'artists' }"
          @click="store.searchCategoryTab = 'artists'"
        >
          歌手 ({{ store.searchResultsArtists.length }})
        </button>
      </div>
    </header>

    <!-- 快捷热搜胶囊条 -->
    <div class="hot-search-strip mb-18">
      <span class="hs-label">热门检索：</span>
      <button
        v-for="kw in store.hotSearchKeywords.slice(0, 10)"
        :key="kw"
        class="sd-chip"
        @click="store.executeSearch(kw)"
      >
        {{ kw }}
      </button>
    </div>

    <div v-if="store.isSearching" class="loading-box">
      正在检索酷狗概念版无损曲库并补全 Hi-Res / SQ 音质标签...
    </div>

    <!-- 1. 单曲结果（桌面双列无框软润卡片） -->
    <div
      v-else-if="store.searchCategoryTab === 'songs'"
      class="dual-column-song-grid"
    >
      <div
        v-for="(song, idx) in store.searchResultsSongs"
        :key="song.hash || song.id"
        class="plump-song-card"
        :class="{
          'track-card-playing': store.currentSong?.hash === song.hash,
        }"
        @click="
          store.playSongFromList(
            song,
            store.searchResultsSongs,
            `搜索：${store.searchKeyword}`
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
            <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
            <span v-else-if="song.isLossless" class="mini-badge sq">SQ</span>
            <span v-if="song.isVip" class="mini-badge vip">VIP</span>
            <span>{{ song.artist }}</span>
            <span v-if="song.album" class="psc-album">· {{ song.album }}</span>
          </div>
        </div>
        <div class="psc-dur">{{ formatDuration(song.durationMs) }}</div>
      </div>
    </div>

    <!-- 2. 歌单结果（支持一键收藏同步至「我的 -> 收藏歌单」） -->
    <div
      v-else-if="store.searchCategoryTab === 'playlists'"
      class="album-cards-grid"
    >
      <div
        v-for="pl in store.searchResultsPlaylists"
        :key="pl.id"
        class="plump-album-card"
        @click="
          store.activeTab = 'library';
          store.libraryAssetTab = 'collected';
          store.selectPlaylist(pl);
        "
      >
        <img v-if="pl.coverUrl" :src="pl.coverUrl" class="palb-cover" />
        <div class="palb-info">
          <div class="palb-name">{{ pl.title }}</div>
          <div class="palb-sub">
            {{ pl.trackCount }} 首 · {{ pl.creatorName || "概念歌单" }}
          </div>
        </div>
        <button
          class="tonal-pill-btn sm"
          @click.stop="store.toggleCollectPlaylist(pl)"
        >
          {{ store.isPlaylistCollected(pl) ? "已收藏" : "收藏" }}
        </button>
      </div>
    </div>

    <!-- 3. 专辑结果 -->
    <div
      v-else-if="store.searchCategoryTab === 'albums'"
      class="album-cards-grid"
    >
      <div
        v-for="alb in store.searchResultsAlbums"
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
        <img v-if="alb.coverUrl" :src="alb.coverUrl" class="palb-cover" />
        <div class="palb-info">
          <div class="palb-name">{{ alb.albumName }}</div>
          <div class="palb-sub">
            {{ alb.artistName }} · {{ alb.publishDate || "数字发行" }}
          </div>
        </div>
      </div>
    </div>

    <!-- 4. 歌手结果 -->
    <div v-else class="artist-cards-grid">
      <div
        v-for="art in store.searchResultsArtists"
        :key="art.artistId"
        class="plump-artist-card"
        @click="store.openArtistDetail(art.artistId, art.name, art.avatarUrl)"
      >
        <img v-if="art.avatarUrl" :src="art.avatarUrl" class="pac-avatar" />
        <div v-else class="pac-avatar fallback">{{ art.name.slice(0, 1) }}</div>
        <div class="pac-info">
          <div class="pac-name">{{ art.name }}</div>
          <div class="pac-sub">单曲 {{ art.songCount }} 首 · 点击查看主页</div>
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
</template>
