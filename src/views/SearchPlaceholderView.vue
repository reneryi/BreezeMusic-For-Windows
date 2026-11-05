<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import type { FollowedArtist, Playlist, SearchCategoryTab, Song } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const searchInputRef = ref<HTMLInputElement | null>(null);
const searchFormWrapRef = ref<HTMLElement | null>(null);
const infiniteSentinelRef = ref<HTMLElement | null>(null);
const searchFocused = ref(false);
const activeBoardIndex = ref(0);
const expandedFoldKeys = reactive(new Set<string>());

const artistRegions = [
  { id: 0, label: "全部" },
  { id: 1, label: "华语" },
  { id: 2, label: "欧美" },
  { id: 3, label: "日韩" },
];

const resultCategoryTabs = computed<
  Array<{ id: SearchCategoryTab; label: string; count?: number }>
>(() => [
  { id: "all", label: "综合" },
  { id: "songs", label: "单曲", count: store.searchTotals.songs },
  { id: "albums", label: "专辑", count: store.searchTotals.albums },
  { id: "artists", label: "歌手", count: store.searchTotals.artists },
  { id: "playlists", label: "歌单", count: store.searchTotals.playlists },
]);

const visibleHotBoards = computed(() =>
  (store.searchDiscovery.hotBoards || []).slice(0, 5)
);

const currentBoardEntries = computed(() => {
  const boards = visibleHotBoards.value;
  if (boards.length > 0) {
    const idx = Math.min(activeBoardIndex.value, boards.length - 1);
    return boards[idx]?.entries || [];
  }
  return store.hotSearchKeywords.map((kw) => ({
    keyword: kw,
    reason: "",
    iconType: 0,
  }));
});

const showSuggestPopover = computed(
  () =>
    searchFocused.value &&
    store.searchKeyword.trim().length > 0 &&
    store.searchSuggestions.length > 0
);

function getSongKey(song: Song, idx: number): string {
  return `${song.hash || song.id || "s"}_${song.albumAudioId || 0}_${idx}`;
}

function getFoldKey(song: Song): string {
  return `${song.hash || song.id}_${song.albumAudioId || 0}_${song.albumId || ""}`;
}

function toggleFoldedVersions(song: Song) {
  const k = getFoldKey(song);
  if (expandedFoldKeys.has(k)) {
    expandedFoldKeys.delete(k);
  } else {
    expandedFoldKeys.add(k);
  }
}

function formatArtistStats(art: FollowedArtist): string {
  const parts: string[] = [];
  if (art.songCount && art.songCount > 0) {
    parts.push(`单曲 ${art.songCount}`);
  }
  if (art.albumCount && art.albumCount > 0) {
    parts.push(`专辑 ${art.albumCount}`);
  }
  if (art.fansCount && art.fansCount > 0) {
    parts.push(
      art.fansCount >= 10000
        ? `${(art.fansCount / 10000).toFixed(1).replace(/\.0$/, "")}w 粉丝`
        : `${art.fansCount} 粉丝`
    );
  }
  return parts.join(" · ") || "酷狗音乐人";
}

function handleSearchKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    if (e.isComposing || e.keyCode === 229) return;
    e.preventDefault();
    submitSearch();
  } else if (e.key === "Escape") {
    searchFocused.value = false;
  }
}

function submitSearch(customKw?: string) {
  const target = (
    customKw !== undefined ? customKw : store.searchKeyword
  ).trim();
  if (!target) {
    store.openSearchLanding(false);
    return;
  }
  searchFocused.value = false;
  expandedFoldKeys.clear();
  store.executeSearch(target);
}

function clearSearchInput() {
  store.searchKeyword = "";
  store.searchSuggestions = [];
  expandedFoldKeys.clear();
  store.openSearchLanding(true);
  nextTick(() => {
    searchInputRef.value?.focus();
  });
}

function removeSingleHistory(item: string) {
  store.searchHistory = store.searchHistory.filter((h) => h !== item);
  localStorage.setItem(
    "bm_search_history",
    JSON.stringify(store.searchHistory)
  );
}

function openSearchedPlaylist(pl: Playlist) {
  store.openPlaylistDetail(pl);
}

function playAllHotSongs() {
  const list = store.searchDiscovery.hotSongs;
  if (list.length > 0) {
    store.playSongFromList(list[0], list, "实时热搜歌曲");
  }
}

const currentTabHasMore = computed(() => {
  const tab = store.searchCategoryTab;
  if (tab === "songs") return store.searchHasMore.songs;
  if (tab === "albums") return store.searchHasMore.albums;
  if (tab === "artists") return store.searchHasMore.artists;
  if (tab === "playlists") return store.searchHasMore.playlists;
  return false;
});

const currentTabLoadingMore = computed(() => {
  const tab = store.searchCategoryTab;
  if (tab === "songs") return store.isLoadingMoreSearch.songs;
  if (tab === "albums") return store.isLoadingMoreSearch.albums;
  if (tab === "artists") return store.isLoadingMoreSearch.artists;
  if (tab === "playlists") return store.isLoadingMoreSearch.playlists;
  return false;
});

function triggerLoadMoreCurrentTab() {
  const tab = store.searchCategoryTab;
  if (
    tab === "songs" ||
    tab === "albums" ||
    tab === "artists" ||
    tab === "playlists"
  ) {
    store.loadMoreSearchCategory(tab);
  }
}

let intersectionObserver: IntersectionObserver | null = null;

function setupInfiniteObserver() {
  if (intersectionObserver) {
    intersectionObserver.disconnect();
    intersectionObserver = null;
  }
  if (typeof IntersectionObserver === "undefined") return;
  const rootEl = document.querySelector(
    ".skyline-stage-viewport"
  ) as HTMLElement | null;
  intersectionObserver = new IntersectionObserver(
    (entries) => {
      const hit = entries.some((entry) => entry.isIntersecting);
      if (
        hit &&
        store.searchViewMode === "results" &&
        currentTabHasMore.value &&
        !currentTabLoadingMore.value &&
        !store.isSearching
      ) {
        triggerLoadMoreCurrentTab();
      }
    },
    {
      root: rootEl || null,
      rootMargin: "360px 0px",
      threshold: 0.01,
    }
  );
  if (infiniteSentinelRef.value) {
    intersectionObserver.observe(infiniteSentinelRef.value);
  }
}

watch(
  [
    () => store.searchViewMode,
    () => store.searchCategoryTab,
    () => store.isSearching,
    infiniteSentinelRef,
  ],
  () => {
    nextTick(() => {
      setupInfiniteObserver();
    });
  }
);

function handleOutsideMouseDown(e: MouseEvent) {
  const target = e.target as Node;
  if (searchFormWrapRef.value && !searchFormWrapRef.value.contains(target)) {
    searchFocused.value = false;
  }
}

onMounted(() => {
  store.loadSearchDiscovery(false);
  if (!store.searchKeyword.trim()) {
    store.searchViewMode = "landing";
  }
  window.addEventListener("mousedown", handleOutsideMouseDown);
  nextTick(() => {
    setupInfiniteObserver();
    if (store.searchViewMode === "landing") {
      searchInputRef.value?.focus();
    }
  });
});

onBeforeUnmount(() => {
  window.removeEventListener("mousedown", handleOutsideMouseDown);
  if (intersectionObserver) {
    intersectionObserver.disconnect();
    intersectionObserver = null;
  }
});
</script>

<template>
  <div class="page-scroll-canvas search-hub-page" @click="store.closeSongContextMenu()">
    <!-- ==================== 顶部核心搜索栏 ==================== -->
    <div ref="searchFormWrapRef" class="search-command-shell">
      <div class="search-command-row">
        <button
          v-if="store.searchViewMode === 'results'"
          type="button"
          class="search-back-landing-btn"
          title="返回搜索探索页"
          @click="store.openSearchLanding(false)"
        >
          <PlumpIcon name="arrow-left" :size="16" />
          <span>热搜榜</span>
        </button>

        <form
          role="search"
          class="search-hero-form"
          :class="{ focused: searchFocused }"
          @submit.prevent="submitSearch()"
        >
          <label class="visually-hidden" for="independent-search-input">
            搜索歌曲、歌手、专辑、歌单
          </label>
          <PlumpIcon
            name="nav-search"
            :size="19"
            color="var(--primary)"
            accent="var(--cyan, #00e5ff)"
            class="shf-leading-icon"
          />
          <input
            id="independent-search-input"
            ref="searchInputRef"
            v-model="store.searchKeyword"
            type="search"
            name="q"
            class="shf-input"
            placeholder="搜索歌曲、歌手、专辑、歌单..."
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            spellcheck="false"
            @focus="searchFocused = true"
            @keydown="handleSearchKeydown"
          />
          <button
            v-if="store.searchKeyword"
            type="button"
            class="shf-clear-btn"
            aria-label="清空搜索词"
            title="清空"
            @click="clearSearchInput"
          >
            <PlumpIcon name="close" :size="13" />
          </button>
          <button type="submit" class="shf-submit-btn">
            <span>搜索</span>
          </button>
        </form>
      </div>

      <!-- 实时联想词浮层 -->
      <transition name="fade-pop">
        <div v-if="showSuggestPopover" class="shf-suggest-popover">
          <button
            v-for="(sug, idx) in store.searchSuggestions"
            :key="`${sug}_${idx}`"
            type="button"
            class="shf-suggest-item"
            @click="submitSearch(sug)"
          >
            <PlumpIcon name="nav-search" :size="15" />
            <span>{{ sug }}</span>
          </button>
        </div>
      </transition>

      <!-- 最近搜索历史（仅在探索态展示，简洁清爽） -->
      <div
        v-if="
          store.searchViewMode === 'landing' && store.searchHistory.length > 0
        "
        class="search-history-bar"
      >
        <span class="shb-title">历史记录</span>
        <div class="shb-pills">
          <div
            v-for="item in store.searchHistory.slice(0, 12)"
            :key="item"
            class="shb-pill"
          >
            <button
              type="button"
              class="shb-pill-text"
              @click="submitSearch(item)"
            >
              {{ item }}
            </button>
            <button
              type="button"
              class="shb-pill-del"
              aria-label="删除该条历史"
              @click.stop="removeSingleHistory(item)"
            >
              ×
            </button>
          </div>
        </div>
        <button
          type="button"
          class="shb-clear-all"
          @click="store.clearSearchHistory()"
        >
          清空
        </button>
      </div>
    </div>

    <!-- ==================== 视图一：独立搜索探索页（实时热门歌手 + 实时热搜歌曲与热搜榜） ==================== -->
    <div v-if="store.searchViewMode === 'landing'" class="search-landing-canvas">
      <!-- 1. 实时热门歌手陈列廊 -->
      <section class="sl-panel sl-artists-panel">
        <div class="sl-panel-head">
          <div class="sl-panel-title-wrap">
            <h3 class="sl-panel-title">热门歌手</h3>
            <div class="sl-region-tabs">
              <button
                v-for="reg in artistRegions"
                :key="reg.id"
                type="button"
                class="sl-region-pill"
                :class="{ active: store.hotArtistCategory === reg.id }"
                @click="store.switchHotArtistCategory(reg.id)"
              >
                {{ reg.label }}
              </button>
            </div>
          </div>

          <button
            type="button"
            class="sl-refresh-btn"
            :disabled="store.isLoadingSearchDiscovery"
            title="刷新实时热搜数据"
            @click="store.loadSearchDiscovery(true)"
          >
            <PlumpIcon
              name="refresh"
              :size="14"
              :class="{ 'spin-anim': store.isLoadingSearchDiscovery }"
            />
            <span>刷新</span>
          </button>
        </div>

        <div
          v-if="
            (store.isLoadingSearchDiscovery || store.isChangingHotArtistCat) &&
            store.searchDiscovery.hotArtists.length === 0
          "
          class="sl-empty-hint"
        >
          正在获取实时热门歌手...
        </div>

        <div v-else class="sl-hot-artists-grid">
          <button
            v-for="(art, idx) in store.searchDiscovery.hotArtists.slice(0, 12)"
            :key="art.artistId"
            type="button"
            class="sl-hot-artist-card"
            @click="store.openArtistDetail(art.artistId, art.name, art.avatarUrl)"
          >
            <div class="shac-avatar-box">
              <img
                v-if="art.avatarUrl"
                :src="art.avatarUrl"
                class="shac-avatar"
                loading="lazy"
                draggable="false"
                alt=""
              />
              <div v-else class="shac-avatar fallback">
                {{ art.name.slice(0, 1) }}
              </div>
              <span v-if="idx < 3" class="shac-rank-badge" :class="`top-${idx + 1}`">
                {{ idx + 1 }}
              </span>
            </div>
            <span class="shac-name">{{ art.name }}</span>
            <span class="shac-meta">{{ formatArtistStats(art) }}</span>
          </button>
        </div>
      </section>

      <!-- 2. 双栏实时热搜矩阵：左侧「热搜歌曲」实时可播榜 + 右侧「热搜词榜」多维风向标 -->
      <div class="sl-dual-bento">
        <!-- 左栏：实时热搜歌曲（可直接点击播放） -->
        <section class="sl-panel sl-hot-songs-panel">
          <div class="sl-panel-head">
            <div class="sl-panel-title-wrap">
              <PlumpIcon
                name="flame"
                :size="19"
                color="#ff5a79"
                accent="#ffb347"
              />
              <h3 class="sl-panel-title">热搜歌曲</h3>
            </div>
            <button
              v-if="store.searchDiscovery.hotSongs.length > 0"
              type="button"
              class="sl-play-all-pill"
              @click="playAllHotSongs"
            >
              <PlumpIcon name="play" :size="13" color="#ffffff" />
              <span>播放全部</span>
            </button>
          </div>

          <div
            v-if="
              store.isLoadingSearchDiscovery &&
              store.searchDiscovery.hotSongs.length === 0
            "
            class="sl-empty-hint"
          >
            正在同步实时热搜歌曲...
          </div>

          <div v-else class="sl-hot-songs-grid">
            <div
              v-for="(song, idx) in store.searchDiscovery.hotSongs.slice(0, 16)"
              :key="getSongKey(song, idx)"
              class="sl-hot-song-row"
              :class="{ playing: store.currentSong?.hash === song.hash }"
              @click="
                store.playSongFromList(
                  song,
                  store.searchDiscovery.hotSongs,
                  '实时热搜歌曲'
                )
              "
              @contextmenu.prevent="
                store.openSongContextMenu(
                  $event,
                  song,
                  store.searchDiscovery.hotSongs
                )
              "
            >
              <span class="shsr-rank" :class="{ top: idx < 3 }">
                {{ String(idx + 1).padStart(2, "0") }}
              </span>
              <img
                v-if="song.coverUrl"
                :src="song.coverUrl"
                class="shsr-cover"
                loading="lazy"
                draggable="false"
                alt=""
              />
              <div v-else class="shsr-cover fallback">
                <PlumpIcon name="disc" :size="16" />
              </div>
              <div class="shsr-info">
                <div class="shsr-title-line">
                  <span class="shsr-title">{{ song.title }}</span>
                  <span
                    v-if="song.isHiRes || song.hasHiRes"
                    class="mini-badge hires"
                  >
                    Hi-Res
                  </span>
                  <span
                    v-else-if="song.isLossless || song.hasSq"
                    class="mini-badge sq"
                  >
                    SQ
                  </span>
                  <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                </div>
                <div class="shsr-artist-line">
                  <button
                    type="button"
                    class="inline-meta-jump"
                    @click.stop="store.navigateToSongArtist(song)"
                  >
                    {{ song.artist }}
                  </button>
                </div>
              </div>
              <span class="shsr-dur">{{ formatDuration(song) }}</span>
            </div>
          </div>
        </section>

        <!-- 右栏：实时多维热搜榜 -->
        <section class="sl-panel sl-hot-boards-panel">
          <div class="sl-panel-head">
            <div class="sl-board-tabs">
              <template v-if="visibleHotBoards.length > 0">
                <button
                  v-for="(board, bIdx) in visibleHotBoards"
                  :key="board.name"
                  type="button"
                  class="sl-board-tab"
                  :class="{ active: activeBoardIndex === bIdx }"
                  @click="activeBoardIndex = bIdx"
                >
                  {{ board.name }}
                </button>
              </template>
              <h3 v-else class="sl-panel-title">热搜榜</h3>
            </div>
          </div>

          <div class="sl-board-entries">
            <button
              v-for="(entry, eIdx) in currentBoardEntries.slice(0, 20)"
              :key="`${entry.keyword}_${eIdx}`"
              type="button"
              class="sl-board-entry"
              @click="submitSearch(entry.keyword)"
            >
              <span class="sbe-rank" :class="{ top: eIdx < 3 }">
                {{ eIdx + 1 }}
              </span>
              <span class="sbe-keyword">{{ entry.keyword }}</span>
              <span v-if="entry.reason" class="sbe-reason">
                {{ entry.reason }}
              </span>
              <span v-else-if="eIdx < 3 || entry.iconType === 1" class="sbe-hot-dot">
                热
              </span>
            </button>
          </div>
        </section>
      </div>
    </div>

    <!-- ==================== 视图二：全量无上限搜索结果页（综合 / 单曲 / 专辑 / 歌手 / 歌单） ==================== -->
    <div v-else class="search-results-canvas">
      <!-- 五维结果导航栏（严格顺序：综合、单曲、专辑、歌手、歌单） -->
      <div class="sr-tabs-bar" role="tablist" aria-label="搜索结果分类">
        <div class="sr-tabs-group">
          <button
            v-for="tab in resultCategoryTabs"
            :key="tab.id"
            type="button"
            role="tab"
            class="sr-tab-btn"
            :aria-selected="store.searchCategoryTab === tab.id"
            :class="{ active: store.searchCategoryTab === tab.id }"
            @click="store.searchCategoryTab = tab.id"
          >
            <span>{{ tab.label }}</span>
            <em v-if="tab.count !== undefined && tab.count > 0" class="sr-tab-count">
              {{ tab.count }}
            </em>
          </button>
        </div>

        <button
          v-if="
            (store.searchCategoryTab === 'songs' ||
              store.searchCategoryTab === 'all') &&
            store.searchResultsSongs.length > 0
          "
          type="button"
          class="sl-play-all-pill"
          @click="
            store.playSongFromList(
              store.searchResultsSongs[0],
              store.searchResultsSongs,
              `搜索：${store.searchKeyword}`
            )
          "
        >
          <PlumpIcon name="play" :size="13" color="#ffffff" />
          <span>播放全部</span>
        </button>
      </div>

      <div v-if="store.isSearching" class="sr-loading-state">
        正在检索「{{ store.searchKeyword }}」的全部内容...
      </div>

      <!-- 1. 综合 Tab -->
      <div
        v-else-if="store.searchCategoryTab === 'all'"
        class="search-comprehensive-grid"
      >
        <!-- 最佳匹配歌手焦点卡 -->
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
          <div class="siah-left">
            <img
              v-if="store.searchResultsArtists[0].avatarUrl"
              :src="store.searchResultsArtists[0].avatarUrl"
              class="ldh-avatar"
              style="width: 64px; height: 64px"
              alt=""
            />
            <div v-else class="pac-avatar fallback">
              {{ store.searchResultsArtists[0].name.slice(0, 1) }}
            </div>
            <div>
              <div class="siah-name-row">
                <span class="siah-name">
                  {{ store.searchResultsArtists[0].name }}
                </span>
                <span class="mini-badge sq">歌手</span>
              </div>
              <div class="siah-meta">
                {{ formatArtistStats(store.searchResultsArtists[0]) }}
              </div>
            </div>
          </div>
          <div class="siah-actions" @click.stop>
            <button
              type="button"
              class="tonal-pill-btn sm primary"
              @click="store.toggleFollowArtist(store.searchResultsArtists[0])"
            >
              {{
                store.searchResultsArtists[0].isFollowed ? "已关注" : "关注"
              }}
            </button>
          </div>
        </div>

        <!-- 单曲区块 -->
        <section v-if="store.searchResultsSongs.length > 0">
          <div class="sr-sec-head">
            <h4 class="sr-sec-title">单曲</h4>
            <button
              type="button"
              class="sr-more-link"
              @click="store.searchCategoryTab = 'songs'"
            >
              全部 {{ store.searchTotals.songs }} 首 ›
            </button>
          </div>
          <div class="dual-column-song-grid">
            <div
              v-for="(song, idx) in store.searchResultsSongs.slice(0, 12)"
              :key="getSongKey(song, idx)"
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
                store.openSongContextMenu(
                  $event,
                  song,
                  store.searchResultsSongs
                )
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
                alt=""
              />
              <div v-else class="psc-cover-fallback">
                <PlumpIcon name="disc" :size="18" />
              </div>
              <div class="psc-main">
                <div class="psc-title">{{ song.title }}</div>
                <div class="psc-sub">
                  <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                  <span
                    v-if="song.isHiRes || song.hasHiRes"
                    class="mini-badge hires"
                  >
                    Hi-Res
                  </span>
                  <span
                    v-else-if="song.isLossless || song.hasSq || song.isSq"
                    class="mini-badge sq"
                  >
                    SQ
                  </span>
                  <button
                    type="button"
                    class="inline-meta-jump"
                    @click.stop="store.navigateToSongArtist(song)"
                  >
                    {{ song.artist }}
                  </button>
                  <button
                    v-if="song.album"
                    type="button"
                    class="inline-meta-jump psc-album"
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

        <!-- 专辑区块 -->
        <section v-if="store.searchResultsAlbums.length > 0">
          <div class="sr-sec-head">
            <h4 class="sr-sec-title">专辑</h4>
            <button
              type="button"
              class="sr-more-link"
              @click="store.searchCategoryTab = 'albums'"
            >
              全部 {{ store.searchTotals.albums }} 张 ›
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
              <img
                v-if="alb.coverUrl"
                :src="alb.coverUrl"
                class="palb-cover"
                loading="lazy"
                alt=""
              />
              <div class="palb-info">
                <div class="palb-name">{{ alb.albumName }}</div>
                <div class="palb-sub">
                  {{ alb.artistName }}
                  <template v-if="alb.publishDate">
                    · {{ alb.publishDate }}
                  </template>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- 歌手区块 -->
        <section v-if="store.searchResultsArtists.length > 1">
          <div class="sr-sec-head">
            <h4 class="sr-sec-title">歌手</h4>
            <button
              type="button"
              class="sr-more-link"
              @click="store.searchCategoryTab = 'artists'"
            >
              全部 {{ store.searchTotals.artists }} 位 ›
            </button>
          </div>
          <div class="artist-cards-grid">
            <div
              v-for="art in store.searchResultsArtists.slice(0, 6)"
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
                loading="lazy"
                alt=""
              />
              <div v-else class="pac-avatar fallback">
                {{ art.name.slice(0, 1) }}
              </div>
              <div class="pac-info">
                <div class="pac-name">{{ art.name }}</div>
                <div class="pac-sub">{{ formatArtistStats(art) }}</div>
              </div>
              <button
                type="button"
                class="tonal-pill-btn sm"
                @click.stop="store.toggleFollowArtist(art)"
              >
                {{ art.isFollowed ? "已关注" : "关注" }}
              </button>
            </div>
          </div>
        </section>

        <!-- 歌单区块 -->
        <section v-if="store.searchResultsPlaylists.length > 0">
          <div class="sr-sec-head">
            <h4 class="sr-sec-title">歌单</h4>
            <button
              type="button"
              class="sr-more-link"
              @click="store.searchCategoryTab = 'playlists'"
            >
              全部 {{ store.searchTotals.playlists }} 个 ›
            </button>
          </div>
          <div class="album-cards-grid">
            <div
              v-for="pl in store.searchResultsPlaylists.slice(0, 6)"
              :key="pl.id"
              class="plump-album-card"
              @click="openSearchedPlaylist(pl)"
            >
              <img
                v-if="pl.coverUrl"
                :src="pl.coverUrl"
                class="palb-cover"
                loading="lazy"
                alt=""
              />
              <div class="palb-info">
                <div class="palb-name">{{ pl.title }}</div>
                <div class="palb-sub">
                  {{ pl.trackCount }} 首
                  <template v-if="pl.creatorName || pl.creator">
                    · {{ pl.creatorName || pl.creator }}
                  </template>
                </div>
              </div>
              <button
                type="button"
                class="tonal-pill-btn sm"
                @click.stop="store.toggleCollectPlaylist(pl)"
              >
                {{ store.isPlaylistCollected(pl) ? "已收藏" : "收藏" }}
              </button>
            </div>
          </div>
        </section>

        <div
          v-if="
            store.searchResultsSongs.length === 0 &&
            store.searchResultsAlbums.length === 0 &&
            store.searchResultsArtists.length === 0 &&
            store.searchResultsPlaylists.length === 0
          "
          class="sr-empty-state"
        >
          未找到与「{{ store.searchKeyword }}」匹配的结果
        </div>
      </div>

      <!-- 2. 单曲全量无上限列表（含官方 Grp 折叠版本展开） -->
      <div v-else-if="store.searchCategoryTab === 'songs'">
        <div
          v-if="store.searchResultsSongs.length === 0"
          class="sr-empty-state"
        >
          暂无匹配单曲
        </div>
        <div v-else class="sr-songs-stack">
          <div
            v-for="(song, idx) in store.searchResultsSongs"
            :key="getSongKey(song, idx)"
            class="sr-song-entry-wrap"
          >
            <div
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
                store.openSongContextMenu(
                  $event,
                  song,
                  store.searchResultsSongs
                )
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
                alt=""
              />
              <div v-else class="psc-cover-fallback">
                <PlumpIcon name="disc" :size="18" />
              </div>
              <div class="psc-main">
                <div class="psc-title">{{ song.title }}</div>
                <div class="psc-sub">
                  <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                  <span
                    v-if="song.isHiRes || song.hasHiRes"
                    class="mini-badge hires"
                  >
                    Hi-Res
                  </span>
                  <span
                    v-else-if="song.isLossless || song.hasSq || song.isSq"
                    class="mini-badge sq"
                  >
                    SQ
                  </span>
                  <button
                    type="button"
                    class="inline-meta-jump"
                    @click.stop="store.navigateToSongArtist(song)"
                  >
                    {{ song.artist }}
                  </button>
                  <button
                    v-if="song.album"
                    type="button"
                    class="inline-meta-jump psc-album"
                    @click.stop="store.navigateToSongAlbum(song)"
                  >
                    · {{ song.album }}
                  </button>
                </div>
              </div>
              <button
                v-if="song.foldedVersions && song.foldedVersions.length > 0"
                type="button"
                class="sr-fold-ver-btn"
                @click.stop="toggleFoldedVersions(song)"
              >
                {{
                  expandedFoldKeys.has(getFoldKey(song))
                    ? "收起版本"
                    : `${song.foldedVersions.length} 个其他版本`
                }}
              </button>
              <div class="psc-dur">{{ formatDuration(song) }}</div>
            </div>

            <!-- 展开的官方折叠版本列表 -->
            <div
              v-if="
                song.foldedVersions &&
                song.foldedVersions.length > 0 &&
                expandedFoldKeys.has(getFoldKey(song))
              "
              class="sr-folded-children"
            >
              <div
                v-for="(ver, vIdx) in song.foldedVersions"
                :key="getSongKey(ver, vIdx)"
                class="sr-folded-row"
                :class="{
                  'track-card-playing': store.currentSong?.hash === ver.hash,
                }"
                @click="
                  store.playSongFromList(
                    ver,
                    song.foldedVersions,
                    `${song.title} · 版本合集`
                  )
                "
                @contextmenu.prevent="
                  store.openSongContextMenu($event, ver, song.foldedVersions)
                "
              >
                <span class="sfr-branch">└</span>
                <div class="psc-main">
                  <div class="psc-title">{{ ver.title }}</div>
                  <div class="psc-sub">
                    <span v-if="ver.isVip" class="mini-badge vip">VIP</span>
                    <span
                      v-if="ver.isHiRes || ver.hasHiRes"
                      class="mini-badge hires"
                    >
                      Hi-Res
                    </span>
                    <span
                      v-else-if="ver.isLossless || ver.hasSq"
                      class="mini-badge sq"
                    >
                      SQ
                    </span>
                    <button
                      type="button"
                      class="inline-meta-jump"
                      @click.stop="store.navigateToSongArtist(ver)"
                    >
                      {{ ver.artist }}
                    </button>
                    <button
                      v-if="ver.album"
                      type="button"
                      class="inline-meta-jump psc-album"
                      @click.stop="store.navigateToSongAlbum(ver)"
                    >
                      · {{ ver.album }}
                    </button>
                  </div>
                </div>
                <div class="psc-dur">{{ formatDuration(ver) }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 3. 专辑全量无上限列表 -->
      <div v-else-if="store.searchCategoryTab === 'albums'">
        <div
          v-if="store.searchResultsAlbums.length === 0"
          class="sr-empty-state"
        >
          暂无匹配专辑
        </div>
        <div v-else class="album-cards-grid">
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
            <img
              v-if="alb.coverUrl"
              :src="alb.coverUrl"
              class="palb-cover"
              loading="lazy"
              alt=""
            />
            <div class="palb-info">
              <div class="palb-name">{{ alb.albumName }}</div>
              <div class="palb-sub">
                {{ alb.artistName }}
                <template v-if="alb.publishDate">
                  · {{ alb.publishDate }}
                </template>
                <template v-if="alb.songCount > 0">
                  · {{ alb.songCount }} 首
                </template>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 4. 歌手全量无上限列表 -->
      <div v-else-if="store.searchCategoryTab === 'artists'">
        <div
          v-if="store.searchResultsArtists.length === 0"
          class="sr-empty-state"
        >
          暂无匹配歌手
        </div>
        <div v-else class="artist-cards-grid">
          <div
            v-for="art in store.searchResultsArtists"
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
              loading="lazy"
              alt=""
            />
            <div v-else class="pac-avatar fallback">
              {{ art.name.slice(0, 1) }}
            </div>
            <div class="pac-info">
              <div class="pac-name">{{ art.name }}</div>
              <div class="pac-sub">{{ formatArtistStats(art) }}</div>
            </div>
            <button
              type="button"
              class="tonal-pill-btn sm"
              @click.stop="store.toggleFollowArtist(art)"
            >
              {{ art.isFollowed ? "已关注" : "关注" }}
            </button>
          </div>
        </div>
      </div>

      <!-- 5. 歌单全量无上限列表 -->
      <div v-else-if="store.searchCategoryTab === 'playlists'">
        <div
          v-if="store.searchResultsPlaylists.length === 0"
          class="sr-empty-state"
        >
          暂无匹配歌单
        </div>
        <div v-else class="album-cards-grid">
          <div
            v-for="pl in store.searchResultsPlaylists"
            :key="pl.id"
            class="plump-album-card"
            @click="openSearchedPlaylist(pl)"
          >
            <img
              v-if="pl.coverUrl"
              :src="pl.coverUrl"
              class="palb-cover"
              loading="lazy"
              alt=""
            />
            <div class="palb-info">
              <div class="palb-name">{{ pl.title }}</div>
              <div class="palb-sub">
                {{ pl.trackCount }} 首
                <template v-if="pl.creatorName || pl.creator">
                  · {{ pl.creatorName || pl.creator }}
                </template>
              </div>
            </div>
            <button
              type="button"
              class="tonal-pill-btn sm"
              @click.stop="store.toggleCollectPlaylist(pl)"
            >
              {{ store.isPlaylistCollected(pl) ? "已收藏" : "收藏" }}
            </button>
          </div>
        </div>
      </div>

      <!-- 底部自动触底无上限加载哨兵 -->
      <div
        v-if="store.searchCategoryTab !== 'all' && !store.isSearching"
        ref="infiniteSentinelRef"
        class="sr-infinite-sentinel"
      >
        <button
          v-if="currentTabHasMore"
          type="button"
          class="tonal-pill-btn sm"
          :disabled="currentTabLoadingMore"
          @click="triggerLoadMoreCurrentTab"
        >
          {{ currentTabLoadingMore ? "正在加载更多结果..." : "继续加载更多" }}
        </button>
        <span
          v-else-if="
            (store.searchCategoryTab === 'songs' &&
              store.searchResultsSongs.length > 0) ||
            (store.searchCategoryTab === 'albums' &&
              store.searchResultsAlbums.length > 0) ||
            (store.searchCategoryTab === 'artists' &&
              store.searchResultsArtists.length > 0) ||
            (store.searchCategoryTab === 'playlists' &&
              store.searchResultsPlaylists.length > 0)
          "
          class="sr-all-loaded-hint"
        >
          已加载全部结果
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.search-hub-page {
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding-bottom: 36px;
}

/* ==================== 顶部沉浸搜索框 ==================== */
.search-command-shell {
  position: relative;
  max-width: 780px;
  width: 100%;
  margin: 2px auto 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.search-command-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.search-back-landing-btn {
  height: 46px;
  padding: 0 16px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--text-primary) 10%, transparent);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 14px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.2s ease, transform 0.2s ease;
}

.search-back-landing-btn:hover {
  background: var(--bg-card-hover);
  transform: translateY(-1px);
}

.search-hero-form {
  flex: 1;
  min-width: 0;
  height: 50px;
  padding: 0 6px 0 18px;
  border-radius: 999px;
  background: var(--bg-card);
  border: 1px solid color-mix(in srgb, var(--text-primary) 10%, transparent);
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.14);
  display: flex;
  align-items: center;
  gap: 10px;
  transition: border-color 0.22s ease, box-shadow 0.22s ease, background 0.22s ease;
}

:global(.scheme-watercolor_light) .search-hero-form,
:global(.scheme-light) .search-hero-form {
  background: rgba(255, 255, 255, 0.92);
  border-color: rgba(15, 23, 42, 0.09);
  box-shadow: 0 10px 26px rgba(15, 23, 42, 0.06);
}

.search-hero-form.focused {
  border-color: color-mix(in srgb, var(--primary) 65%, transparent);
  box-shadow: 0 12px 32px rgba(43, 140, 255, 0.18);
}

.shf-leading-icon {
  flex-shrink: 0;
}

.shf-input {
  flex: 1;
  min-width: 0;
  height: 100%;
  border: none;
  outline: none;
  background: transparent;
  font-size: 15.5px;
  font-weight: 500;
  color: var(--text-primary);
}

.shf-input::-webkit-search-cancel-button {
  -webkit-appearance: none;
  appearance: none;
}

.shf-input::placeholder {
  color: var(--text-muted);
}

.shf-clear-btn {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: none;
  background: color-mix(in srgb, var(--text-primary) 8%, transparent);
  color: var(--text-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
}

.shf-clear-btn:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 14%, transparent);
}

.shf-submit-btn {
  height: 38px;
  padding: 0 20px;
  border-radius: 999px;
  border: none;
  background: linear-gradient(135deg, var(--primary), #19b5fe);
  color: #ffffff;
  font-size: 14.5px;
  font-weight: 700;
  cursor: pointer;
  flex-shrink: 0;
  transition: transform 0.18s ease, box-shadow 0.18s ease;
}

.shf-submit-btn:hover {
  transform: scale(1.02);
  box-shadow: 0 6px 16px rgba(43, 140, 255, 0.34);
}

/* 实时搜索建议浮层 */
.shf-suggest-popover {
  position: absolute;
  top: 56px;
  left: 0;
  right: 0;
  padding: 8px;
  border-radius: 20px;
  background: var(--bg-elevated);
  backdrop-filter: blur(28px);
  border: 1px solid color-mix(in srgb, var(--text-primary) 10%, transparent);
  box-shadow: 0 20px 48px rgba(0, 0, 0, 0.34);
  z-index: 80;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 4px;
}

:global(.scheme-watercolor_light) .shf-suggest-popover,
:global(.scheme-light) .shf-suggest-popover {
  background: rgba(255, 255, 255, 0.96);
  box-shadow: 0 18px 42px rgba(15, 23, 42, 0.12);
}

@media (max-width: 720px) {
  .shf-suggest-popover {
    grid-template-columns: 1fr;
  }
}

.shf-suggest-item {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 9px 12px;
  border-radius: 12px;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 14.5px;
  text-align: left;
  cursor: pointer;
  overflow: hidden;
}

.shf-suggest-item span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shf-suggest-item:hover {
  background: color-mix(in srgb, var(--primary) 12%, transparent);
  color: var(--primary);
}

/* 搜索历史条 */
.search-history-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  padding: 0 6px;
}

.shb-title {
  font-size: 13px;
  color: var(--text-muted);
  flex-shrink: 0;
}

.shb-pills {
  display: flex;
  align-items: center;
  gap: 7px;
  flex-wrap: wrap;
  flex: 1;
}

.shb-pill {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  background: var(--bg-card);
  border: 1px solid color-mix(in srgb, var(--text-primary) 7%, transparent);
  overflow: hidden;
}

:global(.scheme-watercolor_light) .shb-pill,
:global(.scheme-light) .shb-pill {
  background: rgba(255, 255, 255, 0.82);
}

.shb-pill-text {
  padding: 4px 8px 4px 12px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
}

.shb-pill-text:hover {
  color: var(--primary);
}

.shb-pill-del {
  padding: 4px 9px 4px 2px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: 13px;
  cursor: pointer;
}

.shb-pill-del:hover {
  color: var(--coral);
}

.shb-clear-all {
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: 12.5px;
  cursor: pointer;
}

.shb-clear-all:hover {
  color: var(--coral);
}

/* ==================== 独立搜索探索页 ==================== */
.search-landing-canvas {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.sl-panel {
  padding: 20px 22px;
  border-radius: 24px;
  background: var(--bg-card);
  border: 1px solid color-mix(in srgb, var(--text-primary) 7%, transparent);
}

:global(.scheme-watercolor_light) .sl-panel,
:global(.scheme-light) .sl-panel {
  background: rgba(255, 255, 255, 0.84);
  border-color: rgba(15, 23, 42, 0.07);
  box-shadow: 0 10px 28px rgba(15, 23, 42, 0.04);
}

.sl-panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.sl-panel-title-wrap {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.sl-panel-title {
  font-size: 18px;
  font-weight: 800;
  margin: 0;
  color: var(--text-primary);
}

.sl-region-tabs {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
}

.sl-region-pill {
  padding: 4px 12px;
  border-radius: 999px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.18s ease;
}

.sl-region-pill.active {
  background: var(--primary);
  color: #ffffff;
}

.sl-refresh-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 12px;
  border-radius: 999px;
  border: none;
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

.sl-refresh-btn:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 11%, transparent);
}

.sl-hot-artists-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(126px, 1fr));
  gap: 12px;
}

.sl-hot-artist-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 14px 10px 12px;
  border-radius: 18px;
  border: 1px solid transparent;
  background: color-mix(in srgb, var(--text-primary) 3.5%, transparent);
  color: var(--text-primary);
  cursor: pointer;
  transition: transform 0.2s ease, background 0.2s ease, border-color 0.2s ease;
}

:global(.scheme-watercolor_light) .sl-hot-artist-card,
:global(.scheme-light) .sl-hot-artist-card {
  background: rgba(241, 245, 249, 0.72);
}

.sl-hot-artist-card:hover {
  transform: translateY(-3px);
  background: color-mix(in srgb, var(--primary) 10%, transparent);
  border-color: color-mix(in srgb, var(--primary) 26%, transparent);
}

.shac-avatar-box {
  position: relative;
  width: 68px;
  height: 68px;
}

.shac-avatar {
  width: 68px;
  height: 68px;
  border-radius: 50%;
  object-fit: cover;
  box-shadow: 0 8px 18px rgba(0, 0, 0, 0.22);
}

.shac-avatar.fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  font-weight: 800;
  color: #ffffff;
  background: linear-gradient(135deg, var(--primary), #19b5fe);
}

.shac-rank-badge {
  position: absolute;
  right: -2px;
  bottom: -2px;
  width: 21px;
  height: 21px;
  border-radius: 50%;
  font-size: 11.5px;
  font-weight: 800;
  color: #ffffff;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 3px 8px rgba(0, 0, 0, 0.28);
}

.shac-rank-badge.top-1 {
  background: linear-gradient(135deg, #ff5a79, #ff8a5b);
}
.shac-rank-badge.top-2 {
  background: linear-gradient(135deg, #f59e0b, #fbbf24);
}
.shac-rank-badge.top-3 {
  background: linear-gradient(135deg, #2b8cff, #00c6ff);
}

.shac-name {
  font-size: 14.5px;
  font-weight: 700;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shac-meta {
  font-size: 12px;
  color: var(--text-muted);
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 双栏热搜矩阵 */
.sl-dual-bento {
  display: grid;
  grid-template-columns: minmax(0, 1.45fr) minmax(300px, 1fr);
  gap: 18px;
  align-items: start;
}

@media (max-width: 1060px) {
  .sl-dual-bento {
    grid-template-columns: 1fr;
  }
}

.sl-play-all-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 15px;
  border-radius: 999px;
  border: none;
  background: linear-gradient(135deg, var(--primary), #19b5fe);
  color: #ffffff;
  font-size: 13.5px;
  font-weight: 700;
  cursor: pointer;
  transition: transform 0.18s ease, box-shadow 0.18s ease;
}

.sl-play-all-pill:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(43, 140, 255, 0.32);
}

.sl-hot-songs-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px 12px;
}

@media (max-width: 780px) {
  .sl-hot-songs-grid {
    grid-template-columns: 1fr;
  }
}

.sl-hot-song-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 14px;
  cursor: pointer;
  transition: background 0.18s ease, transform 0.18s ease;
}

.sl-hot-song-row:hover {
  background: color-mix(in srgb, var(--primary) 9%, transparent);
  transform: translateX(2px);
}

.sl-hot-song-row.playing {
  background: color-mix(in srgb, var(--primary) 15%, transparent);
}

.shsr-rank {
  width: 22px;
  font-size: 13.5px;
  font-weight: 800;
  color: var(--text-muted);
  text-align: center;
  flex-shrink: 0;
}

.shsr-rank.top {
  color: #ff5a79;
}

.shsr-cover {
  width: 42px;
  height: 42px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
}

.shsr-cover.fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--text-primary) 8%, transparent);
}

.shsr-info {
  flex: 1;
  min-width: 0;
}

.shsr-title-line {
  display: flex;
  align-items: center;
  gap: 5px;
}

.shsr-title {
  font-size: 14.5px;
  font-weight: 700;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shsr-artist-line {
  font-size: 12.5px;
  color: var(--text-secondary);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shsr-dur {
  font-size: 12.5px;
  color: var(--text-muted);
  flex-shrink: 0;
}

/* 右侧热搜词榜 */
.sl-board-tabs {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.sl-board-tab {
  padding: 5px 12px;
  border-radius: 999px;
  border: none;
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.18s ease;
}

.sl-board-tab.active {
  background: linear-gradient(135deg, #ff5a79, #ff8052);
  color: #ffffff;
}

.sl-board-entries {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 4px 10px;
}

@media (max-width: 520px) {
  .sl-board-entries {
    grid-template-columns: 1fr;
  }
}

.sl-board-entry {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 12px;
  border: none;
  background: transparent;
  color: var(--text-primary);
  text-align: left;
  cursor: pointer;
  min-width: 0;
  transition: background 0.16s ease, transform 0.16s ease;
}

.sl-board-entry:hover {
  background: color-mix(in srgb, var(--primary) 10%, transparent);
  transform: translateX(2px);
}

.sbe-rank {
  width: 18px;
  font-size: 13.5px;
  font-weight: 800;
  color: var(--text-muted);
  flex-shrink: 0;
}

.sbe-rank.top {
  color: #ff5a79;
}

.sbe-keyword {
  flex: 1;
  min-width: 0;
  font-size: 14px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sbe-reason {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, #ff5a79 14%, transparent);
  color: #ff5a79;
  max-width: 68px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}

.sbe-hot-dot {
  font-size: 11px;
  font-weight: 800;
  padding: 1px 5px;
  border-radius: 6px;
  background: rgba(255, 90, 121, 0.16);
  color: #ff5a79;
  flex-shrink: 0;
}

.sl-empty-hint {
  padding: 28px 0;
  text-align: center;
  color: var(--text-muted);
  font-size: 14px;
}

/* ==================== 搜索结果页 ==================== */
.search-results-canvas {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.sr-tabs-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  padding-bottom: 4px;
  border-bottom: 1px solid color-mix(in srgb, var(--text-primary) 8%, transparent);
}

.sr-tabs-group {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.sr-tab-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 999px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 15px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.2s ease;
}

.sr-tab-btn:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
}

.sr-tab-btn.active {
  background: linear-gradient(135deg, var(--primary), #19b5fe);
  color: #ffffff;
  box-shadow: 0 6px 16px rgba(43, 140, 255, 0.28);
}

.sr-tab-count {
  font-style: normal;
  font-size: 12px;
  font-weight: 700;
  opacity: 0.86;
}

.sr-sec-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}

.sr-sec-title {
  font-size: 17.5px;
  font-weight: 800;
  margin: 0;
  color: var(--text-primary);
}

.sr-more-link {
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 13.5px;
  font-weight: 600;
  cursor: pointer;
}

.sr-more-link:hover {
  color: var(--primary);
}

.siah-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.siah-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.siah-name {
  font-size: 19px;
  font-weight: 800;
}

.siah-meta {
  font-size: 13.5px;
  color: var(--text-secondary);
  margin-top: 4px;
}

.siah-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.sr-songs-stack {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 14px;
}

@media (max-width: 960px) {
  .sr-songs-stack {
    grid-template-columns: 1fr;
  }
}

.sr-song-entry-wrap {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.sr-fold-ver-btn {
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--primary) 30%, transparent);
  background: color-mix(in srgb, var(--primary) 10%, transparent);
  color: var(--primary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
  white-space: nowrap;
}

.sr-fold-ver-btn:hover {
  background: var(--primary);
  color: #ffffff;
}

.sr-folded-children {
  margin-left: 24px;
  padding: 6px 10px;
  border-radius: 14px;
  background: color-mix(in srgb, var(--text-primary) 4%, transparent);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.sr-folded-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 8px;
  border-radius: 10px;
  cursor: pointer;
}

.sr-folded-row:hover {
  background: color-mix(in srgb, var(--primary) 10%, transparent);
}

.sfr-branch {
  color: var(--text-muted);
  font-size: 13px;
  flex-shrink: 0;
}

.sr-loading-state,
.sr-empty-state {
  padding: 48px 16px;
  text-align: center;
  color: var(--text-muted);
  font-size: 15px;
}

.sr-infinite-sentinel {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 20px 0 8px;
}

.sr-all-loaded-hint {
  font-size: 13px;
  color: var(--text-muted);
}

.inline-meta-jump {
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  font: inherit;
  cursor: pointer;
}

.inline-meta-jump:hover {
  color: var(--primary);
  text-decoration: underline;
}

.spin-anim {
  animation: search-spin 0.9s linear infinite;
}

@keyframes search-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
