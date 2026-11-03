<script setup lang="ts">
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

function openSearchedPlaylist(pl: any) {
  store.activeTab = "library";
  store.libraryAssetTab = "collected";
  store.libraryDetailMode = "playlist";
  store.selectPlaylist(pl);
}
</script>

<template>
  <div class="page-scroll-canvas" @click="store.closeSongContextMenu()">
    <header class="discover-top-header">
      <div class="block-title-with-tabs">
        <h3 class="block-title">检索词：「{{ store.searchKeyword || "热搜漫游" }}」</h3>
      </div>

      <!-- 五维分类胶囊切换（对齐 Android SearchViewModel：综合 / 单曲 / 歌单 / 专辑 / 歌手） -->
      <div class="pill-tabs">
        <button
          class="pill-tab-btn"
          :class="{ active: store.searchCategoryTab === 'all' }"
          @click="store.searchCategoryTab = 'all'"
        >
          综合
        </button>
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

    <!-- 0. 综合智能意图结果页（Artist Hero + 精选单曲 + 专辑 + 歌单） -->
    <div
      v-else-if="store.searchCategoryTab === 'all'"
      class="search-comprehensive-grid"
    >
      <!-- 意图命中歌手卡片 -->
      <div
        v-if="store.searchResultsArtists.length > 0"
        class="search-intent-artist-hero"
        @click="
          store.openArtistDetail(
            store.searchResultsArtists[0].artistId,
            store.searchResultsArtists[0].name,
            store.searchResultsArtists[0].avatarUrl
          )
        "
      >
        <div style="display: flex; align-items: center; gap: 16px">
          <img
            v-if="store.searchResultsArtists[0].avatarUrl"
            :src="store.searchResultsArtists[0].avatarUrl"
            class="ldh-avatar"
            style="width: 68px; height: 68px"
          />
          <div>
            <div style="display: flex; align-items: center; gap: 8px">
              <span style="font-size: 20px; font-weight: 900">
                {{ store.searchResultsArtists[0].name }}
              </span>
              <span class="mini-badge sq">最佳匹配音乐人</span>
            </div>
            <div style="font-size: 13.5px; color: var(--text-secondary); margin-top: 4px">
              单曲 {{ store.searchResultsArtists[0].songCount || 30 }} 首 · 点击进入歌手主页与百科
            </div>
          </div>
        </div>
        <div style="display: flex; align-items: center; gap: 10px" @click.stop>
          <button
            class="tonal-pill-btn sm primary"
            @click="store.toggleFollowArtist(store.searchResultsArtists[0])"
          >
            {{ store.searchResultsArtists[0].isFollowed ? "已关注" : "关注歌手" }}
          </button>
          <button
            class="primary-btn"
            style="padding: 8px 16px; font-size: 14px"
            @click="
              store.openArtistDetail(
                store.searchResultsArtists[0].artistId,
                store.searchResultsArtists[0].name,
                store.searchResultsArtists[0].avatarUrl
              )
            "
          >
            <span>查看代表作</span>
          </button>
        </div>
      </div>

      <!-- 热门单曲 Top 8 -->
      <section v-if="store.searchResultsSongs.length > 0">
        <div class="block-header" style="margin-bottom: 10px">
          <div class="block-title-row">
            <h4 class="block-title" style="font-size: 18.5px">单曲精选</h4>
          </div>
          <button
            class="tonal-pill-btn sm"
            @click="store.searchCategoryTab = 'songs'"
          >
            查看全部 {{ store.searchResultsSongs.length }} 首 >
          </button>
        </div>
        <div class="dual-column-song-grid">
          <div
            v-for="(song, idx) in store.searchResultsSongs.slice(0, 8)"
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
            @contextmenu.prevent="
              store.openSongContextMenu($event, song, store.searchResultsSongs)
            "
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
                <span v-if="song.isHiRes || song.hasHiRes" class="mini-badge hires">Hi-Res</span>
                <span v-else-if="song.isLossless || song.hasSq" class="mini-badge sq">SQ</span>
                <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                <span>{{ song.artist }}</span>
                <span v-if="song.album" class="psc-album">· {{ song.album }}</span>
              </div>
            </div>
            <div class="psc-dur">{{ formatDuration(song) }}</div>
          </div>
        </div>
      </section>

      <!-- 专辑与歌单双栏预览 -->
      <section v-if="store.searchResultsAlbums.length > 0">
        <div class="block-header" style="margin-bottom: 10px">
          <h4 class="block-title" style="font-size: 18.5px">相关数字专辑</h4>
          <button
            class="tonal-pill-btn sm"
            @click="store.searchCategoryTab = 'albums'"
          >
            更多专辑 >
          </button>
        </div>
        <div class="album-cards-grid">
          <div
            v-for="alb in store.searchResultsAlbums.slice(0, 6)"
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
      </section>

      <section v-if="store.searchResultsPlaylists.length > 0">
        <div class="block-header" style="margin-bottom: 10px">
          <h4 class="block-title" style="font-size: 18.5px">精选概念歌单</h4>
          <button
            class="tonal-pill-btn sm"
            @click="store.searchCategoryTab = 'playlists'"
          >
            更多歌单 >
          </button>
        </div>
        <div class="album-cards-grid">
          <div
            v-for="pl in store.searchResultsPlaylists.slice(0, 6)"
            :key="pl.id"
            class="plump-album-card"
            @click="openSearchedPlaylist(pl)"
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
      </section>
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
        @contextmenu.prevent="store.openSongContextMenu($event, song, store.searchResultsSongs)"
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
            <span v-if="song.isHiRes || song.hasHiRes" class="mini-badge hires">Hi-Res</span>
            <span v-else-if="song.isLossless || song.hasSq" class="mini-badge sq">SQ</span>
            <span v-if="song.isVip" class="mini-badge vip">VIP</span>
            <span>{{ song.artist }}</span>
            <span v-if="song.album" class="psc-album">· {{ song.album }}</span>
          </div>
        </div>
        <div class="psc-dur">{{ formatDuration(song) }}</div>
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
        @click="openSearchedPlaylist(pl)"
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
