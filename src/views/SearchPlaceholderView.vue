<script setup lang="ts">
import { ref, watch } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration, kugouClient } from "../api/kugou";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const songPage = ref(1);
const songHasMore = ref(true);
const isLoadingMoreSongs = ref(false);

const playlistPage = ref(1);
const playlistHasMore = ref(true);
const isLoadingMorePlaylists = ref(false);

const albumPage = ref(1);
const albumHasMore = ref(true);
const isLoadingMoreAlbums = ref(false);

const artistPage = ref(1);
const artistHasMore = ref(true);
const isLoadingMoreArtists = ref(false);

watch(
  () => store.searchKeyword,
  () => {
    songPage.value = 1;
    songHasMore.value = true;
    playlistPage.value = 1;
    playlistHasMore.value = true;
    albumPage.value = 1;
    albumHasMore.value = true;
    artistPage.value = 1;
    artistHasMore.value = true;
  }
);

function openSearchedPlaylist(pl: any) {
  store.openPlaylistDetail(pl);
}

async function loadMoreSongs() {
  const kw = store.searchKeyword.trim();
  if (!kw || isLoadingMoreSongs.value || !songHasMore.value) return;
  isLoadingMoreSongs.value = true;
  try {
    const nextP = songPage.value + 1;
    const more = await kugouClient.searchSongs(kw, nextP, 35);
    if (more.length === 0) {
      songHasMore.value = false;
    } else {
      const seen = new Set(store.searchResultsSongs.map((s) => s.hash));
      const unique = more.filter((s) => !seen.has(s.hash));
      store.searchResultsSongs.push(...unique);
      songPage.value = nextP;
      if (more.length < 20 || unique.length === 0) songHasMore.value = false;
    }
  } catch {
    songHasMore.value = false;
  } finally {
    isLoadingMoreSongs.value = false;
  }
}

async function loadMorePlaylists() {
  const kw = store.searchKeyword.trim();
  if (!kw || isLoadingMorePlaylists.value || !playlistHasMore.value) return;
  isLoadingMorePlaylists.value = true;
  try {
    const nextP = playlistPage.value + 1;
    const more = await kugouClient.searchPlaylists(kw, nextP);
    if (more.length === 0) {
      playlistHasMore.value = false;
    } else {
      const seen = new Set(store.searchResultsPlaylists.map((p) => p.id));
      const unique = more.filter((p) => !seen.has(p.id));
      store.searchResultsPlaylists.push(...unique);
      playlistPage.value = nextP;
      if (more.length < 15 || unique.length === 0) playlistHasMore.value = false;
    }
  } catch {
    playlistHasMore.value = false;
  } finally {
    isLoadingMorePlaylists.value = false;
  }
}

async function loadMoreAlbums() {
  const kw = store.searchKeyword.trim();
  if (!kw || isLoadingMoreAlbums.value || !albumHasMore.value) return;
  isLoadingMoreAlbums.value = true;
  try {
    const nextP = albumPage.value + 1;
    const more = await kugouClient.searchAlbums(kw, nextP);
    if (more.length === 0) {
      albumHasMore.value = false;
    } else {
      const seen = new Set(store.searchResultsAlbums.map((a) => a.albumId));
      const unique = more.filter((a) => !seen.has(a.albumId));
      store.searchResultsAlbums.push(...unique);
      albumPage.value = nextP;
      if (more.length < 15 || unique.length === 0) albumHasMore.value = false;
    }
  } catch {
    albumHasMore.value = false;
  } finally {
    isLoadingMoreAlbums.value = false;
  }
}

async function loadMoreArtists() {
  const kw = store.searchKeyword.trim();
  if (!kw || isLoadingMoreArtists.value || !artistHasMore.value) return;
  isLoadingMoreArtists.value = true;
  try {
    const nextP = artistPage.value + 1;
    const more = await kugouClient.searchArtists(kw, nextP);
    if (more.length === 0) {
      artistHasMore.value = false;
    } else {
      const seen = new Set(store.searchResultsArtists.map((a) => a.artistId));
      const unique = more.filter((a) => !seen.has(a.artistId));
      store.searchResultsArtists.push(...unique);
      artistPage.value = nextP;
      if (more.length < 10 || unique.length === 0) artistHasMore.value = false;
    }
  } catch {
    artistHasMore.value = false;
  } finally {
    isLoadingMoreArtists.value = false;
  }
}
</script>

<template>
  <div class="page-scroll-canvas" @click="store.closeSongContextMenu()">
    <header class="discover-top-header">
      <div class="block-title-with-tabs" style="display: flex; align-items: center; gap: 12px">
        <button class="tonal-pill-btn sm" @click="store.navigateBack()">
          <PlumpIcon name="arrow-left" :size="15" />
          <span>返回</span>
        </button>
        <h3 class="block-title">独立搜索中心：「{{ store.searchKeyword || "热搜漫游" }}」</h3>
      </div>

      <!-- 五维分类胶囊切换（对齐 Android SearchViewModel：综合 / 单曲 / 歌单 / 专辑 / 歌手） -->
      <div class="pill-tabs">
        <button
          class="pill-tab-btn"
          :class="{ active: store.searchCategoryTab === 'all' }"
          @click="store.searchCategoryTab = 'all'"
        >
          综合总览
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
        v-for="kw in store.hotSearchKeywords.slice(0, 12)"
        :key="kw"
        class="sd-chip"
        @click="store.executeSearch(kw)"
      >
        {{ kw }}
      </button>
    </div>

    <div v-if="store.isSearching" class="loading-box">
      正在检索酷狗概念版无损曲库并补全 Hi-Res / SQ / VIP 音质标签...
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
              单曲 {{ store.searchResultsArtists[0].songCount || 30 }} 首 · 专辑 {{ store.searchResultsArtists[0].albumCount || 8 }} 张 · 点击进入独立歌手主页
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
            <span>进入歌手主页</span>
          </button>
        </div>
      </div>

      <!-- 热门单曲 Top 12 -->
      <section v-if="store.searchResultsSongs.length > 0">
        <div class="block-header" style="margin-bottom: 10px">
          <div class="block-title-row">
            <h4 class="block-title" style="font-size: 18.5px">单曲精选</h4>
          </div>
          <div style="display: flex; gap: 8px">
            <button
              class="tonal-pill-btn sm primary"
              @click="
                store.playSongFromList(
                  store.searchResultsSongs[0],
                  store.searchResultsSongs,
                  `搜索：${store.searchKeyword}`
                )
              "
            >
              <PlumpIcon name="play" :size="14" />
              <span>播放全部单曲</span>
            </button>
            <button
              class="tonal-pill-btn sm"
              @click="store.searchCategoryTab = 'songs'"
            >
              查看全部 {{ store.searchResultsSongs.length }} 首 >
            </button>
          </div>
        </div>
        <div class="dual-column-song-grid">
          <div
            v-for="(song, idx) in store.searchResultsSongs.slice(0, 12)"
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
                <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                <span v-if="song.isHiRes || song.hasHiRes" class="mini-badge hires">Hi-Res</span>
                <span v-else-if="song.isLossless || song.hasSq || song.isSq" class="mini-badge sq">SQ</span>
                <button
                  class="inline-meta-jump"
                  title="点击跳转歌手主页"
                  @click.stop="store.navigateToSongArtist(song)"
                >
                  {{ song.artist }}
                </button>
                <button
                  v-if="song.album"
                  class="inline-meta-jump psc-album"
                  title="点击跳转专辑详情页"
                  @click.stop="store.navigateToSongAlbum(song)"
                >
                  · {{ song.album }}
                </button>
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
            全部 {{ store.searchResultsAlbums.length }} 张专辑 >
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
            全部 {{ store.searchResultsPlaylists.length }} 个歌单 >
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
                {{ pl.trackCount }} 首 · {{ pl.creatorName || pl.creator || "概念歌单" }}
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

    <!-- 1. 单曲结果（支持歌手/专辑点击跳转与分页加载） -->
    <div v-else-if="store.searchCategoryTab === 'songs'">
      <div class="dual-column-song-grid">
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
              <span v-if="song.isVip" class="mini-badge vip">VIP</span>
              <span v-if="song.isHiRes || song.hasHiRes" class="mini-badge hires">Hi-Res</span>
              <span v-else-if="song.isLossless || song.hasSq || song.isSq" class="mini-badge sq">SQ</span>
              <button
                class="inline-meta-jump"
                title="点击进入歌手独立主页"
                @click.stop="store.navigateToSongArtist(song)"
              >
                {{ song.artist }}
              </button>
              <button
                v-if="song.album"
                class="inline-meta-jump psc-album"
                title="点击进入专辑独立页面"
                @click.stop="store.navigateToSongAlbum(song)"
              >
                · {{ song.album }}
              </button>
            </div>
          </div>
          <div class="psc-dur">{{ formatDuration(song) }}</div>
        </div>
      </div>

      <div v-if="songHasMore && store.searchResultsSongs.length > 0" class="load-more-center">
        <button class="tonal-pill-btn" :disabled="isLoadingMoreSongs" @click="loadMoreSongs">
          {{ isLoadingMoreSongs ? "正在加载更多单曲..." : "加载更多搜索单曲" }}
        </button>
      </div>
    </div>

    <!-- 2. 歌单结果（点击进入独立歌单详情页，支持一键收藏与分页） -->
    <div v-else-if="store.searchCategoryTab === 'playlists'">
      <div class="album-cards-grid">
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
              {{ pl.trackCount }} 首 · {{ pl.creatorName || pl.creator || "概念歌单" }}
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
      <div v-if="playlistHasMore && store.searchResultsPlaylists.length > 0" class="load-more-center">
        <button class="tonal-pill-btn" :disabled="isLoadingMorePlaylists" @click="loadMorePlaylists">
          {{ isLoadingMorePlaylists ? "正在加载更多歌单..." : "加载更多搜索歌单" }}
        </button>
      </div>
    </div>

    <!-- 3. 专辑结果（点击进入独立专辑详情页，支持分页） -->
    <div v-else-if="store.searchCategoryTab === 'albums'">
      <div class="album-cards-grid">
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
      <div v-if="albumHasMore && store.searchResultsAlbums.length > 0" class="load-more-center">
        <button class="tonal-pill-btn" :disabled="isLoadingMoreAlbums" @click="loadMoreAlbums">
          {{ isLoadingMoreAlbums ? "正在加载更多专辑..." : "加载更多搜索专辑" }}
        </button>
      </div>
    </div>

    <!-- 4. 歌手结果（点击进入独立歌手主页，支持分页） -->
    <div v-else>
      <div class="artist-cards-grid">
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
            <div class="pac-sub">
              单曲 {{ art.songCount || 30 }} 首 · 专辑 {{ art.albumCount || 6 }} 张 · 点击进入独立主页
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
      <div v-if="artistHasMore && store.searchResultsArtists.length > 0" class="load-more-center">
        <button class="tonal-pill-btn" :disabled="isLoadingMoreArtists" @click="loadMoreArtists">
          {{ isLoadingMoreArtists ? "正在加载更多歌手..." : "加载更多搜索歌手" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.inline-meta-jump {
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.inline-meta-jump:hover {
  color: var(--cyan);
  text-decoration: underline;
}
.load-more-center {
  display: flex;
  justify-content: center;
  padding: 20px 0 8px;
}
</style>
