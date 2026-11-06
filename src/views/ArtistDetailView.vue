<script setup lang="ts">
import { ref, computed } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const songFilterKeyword = ref("");
const isBioExpanded = ref(false);

const artist = computed(() => store.activeArtistDetail);

const filteredSongs = computed(() => {
  const kw = songFilterKeyword.value.trim().toLowerCase();
  if (!kw) return store.activeArtistSongs;
  return store.activeArtistSongs.filter(
    (s) =>
      s.title.toLowerCase().includes(kw) ||
      (s.album || "").toLowerCase().includes(kw)
  );
});

const displaySongCount = computed(() => {
  const c = artist.value?.songCount || 0;
  return Math.max(c, store.activeArtistSongs.length);
});

const displayAlbumCount = computed(() => {
  const c = artist.value?.albumCount || 0;
  return Math.max(c, store.activeArtistAlbums.length);
});

const formattedFans = computed(() => {
  const fans = artist.value?.fansCount || 0;
  if (!fans || fans <= 0) return "";
  if (fans >= 10000) {
    return `${(fans / 10000).toFixed(1).replace(/\.0$/, "")}w 粉丝`;
  }
  return `${fans} 粉丝`;
});

const bioParagraphs = computed(() => {
  const raw = (artist.value?.longIntro || artist.value?.intro || "").trim();
  if (!raw) {
    return [
      `${artist.value?.name || "该歌手"}，华语及国际流行音乐代表性创作者，正版曲库收录其多张高解析度录音室专辑与热门代表作。`,
    ];
  }
  return raw
    .split(/\r?\n+/)
    .map((p) => p.trim())
    .filter((p) => p.length > 0);
});

const shortIntroSummary = computed(() => {
  return bioParagraphs.value.slice(0, 3).join(" ");
});

function playAllSongs() {
  if (filteredSongs.value.length === 0) return;
  store.playSongFromList(
    filteredSongs.value[0],
    filteredSongs.value,
    `${artist.value?.name || "歌手"} · 热门代表作`
  );
}

async function refreshArtistPage() {
  if (!artist.value) return;
  await store.openArtistDetail(
    artist.value.artistId || "0",
    artist.value.name,
    artist.value.avatarUrl
  );
  store.notify(`已刷新歌手「${artist.value.name}」主页数据`);
}
</script>

<template>
  <section class="standalone-detail-page artist-standalone-page">
    <!-- 顶部导航条（无冗余重复面包屑） -->
    <div class="sdp-top-bar">
      <button class="tonal-pill-btn" @click="store.navigateBack()">
        <PlumpIcon name="arrow-left" :size="16" />
        <span>返回</span>
      </button>

      <div class="sdp-top-actions">
        <button
          class="tonal-pill-btn"
          :disabled="store.isLoadingArtist"
          @click="refreshArtistPage"
        >
          <PlumpIcon
            name="refresh"
            :size="15"
            :class="{ 'spin-anim': store.isLoadingArtist }"
          />
          <span>{{ store.isLoadingArtist ? "正在同步..." : "刷新" }}</span>
        </button>
      </div>
    </div>

    <!-- 歌手视觉头图横幅 -->
    <div class="artist-hero-banner">
      <div class="artist-avatar-halo">
        <img
          v-if="artist?.avatarUrl"
          :src="artist.avatarUrl"
          class="artist-hero-avatar"
          alt="artist"
        />
        <div v-else class="artist-hero-avatar fallback">
          <PlumpIcon name="user" :size="48" color="#fff" />
        </div>
      </div>

      <div class="artist-hero-meta">
        <div class="artist-badge-row">
          <span class="artist-verified-pill">PRO ARTIST · 官方入驻音乐人</span>
          <span v-if="formattedFans" class="artist-fans-pill">
            {{ formattedFans }}
          </span>
        </div>

        <h1 class="artist-hero-name">{{ artist?.name || "歌手主页" }}</h1>

        <div class="artist-stat-counters">
          <div class="stat-box" @click="store.artistDetailSubTab = 'songs'">
            <span class="num">{{ displaySongCount }}</span>
            <span class="lbl">单曲</span>
          </div>
          <div class="stat-divider"></div>
          <div class="stat-box" @click="store.artistDetailSubTab = 'albums'">
            <span class="num">{{ displayAlbumCount }}</span>
            <span class="lbl">专辑</span>
          </div>
        </div>

        <p
          class="artist-short-intro"
          :class="{ expanded: isBioExpanded }"
          title="点击查看完整歌手百科介绍"
          @click="store.artistDetailSubTab = 'about'"
        >
          {{ shortIntroSummary }}
        </p>

        <div class="artist-hero-actions">
          <button class="primary-btn" @click="playAllSongs">
            <PlumpIcon name="play" :size="16" color="#fff" />
            <span>播放热门代表作</span>
          </button>

          <button
            v-if="artist"
            class="tonal-pill-btn"
            :class="{ primary: artist.isFollowed }"
            @click="store.toggleFollowArtist(artist)"
          >
            <PlumpIcon :name="artist.isFollowed ? 'check' : 'plus'" :size="15" />
            <span>{{ artist.isFollowed ? "已关注歌手" : "关注歌手" }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- 子导航胶囊（代表作单曲 / 发行专辑 / 歌手百科介绍） -->
    <div class="artist-subnav-bar">
      <div class="pill-tabs">
        <button
          class="pill-tab-btn"
          :class="{ active: store.artistDetailSubTab === 'songs' }"
          @click="store.artistDetailSubTab = 'songs'"
        >
          代表作单曲 ({{ displaySongCount }})
        </button>
        <button
          class="pill-tab-btn"
          :class="{ active: store.artistDetailSubTab === 'albums' }"
          @click="store.artistDetailSubTab = 'albums'"
        >
          发行专辑 ({{ displayAlbumCount }})
        </button>
        <button
          class="pill-tab-btn"
          :class="{ active: store.artistDetailSubTab === 'about' }"
          @click="store.artistDetailSubTab = 'about'"
        >
          歌手百科
        </button>
      </div>

      <!-- 单曲 Tab 工具栏：筛选 + 最热/最新切换 -->
      <div v-if="store.artistDetailSubTab === 'songs'" class="artist-songs-tools">
        <div class="sdp-search-input-box">
          <PlumpIcon name="search" :size="15" />
          <input
            v-model="songFilterKeyword"
            type="text"
            placeholder="筛选当前已加载曲目..."
          />
          <button v-if="songFilterKeyword" class="clear-btn" @click="songFilterKeyword = ''">
            <PlumpIcon name="close" :size="13" />
          </button>
        </div>

        <div class="sdp-sort-pills">
          <button
            class="sort-pill"
            :class="{ active: store.artistSongsSort === 'hot' }"
            @click="store.switchArtistSongSort('hot')"
          >
            最热代表作
          </button>
          <button
            class="sort-pill"
            :class="{ active: store.artistSongsSort === 'new' }"
            @click="store.switchArtistSongSort('new')"
          >
            最新发行
          </button>
        </div>
      </div>

      <!-- 专辑 Tab 工具栏：最新/最热专辑切换 -->
      <div v-else-if="store.artistDetailSubTab === 'albums'" class="artist-songs-tools">
        <div class="sdp-sort-pills">
          <button
            class="sort-pill"
            :class="{ active: store.artistAlbumsSort === 'new' }"
            @click="store.switchArtistAlbumSort('new')"
          >
            最新发行
          </button>
          <button
            class="sort-pill"
            :class="{ active: store.artistAlbumsSort === 'hot' }"
            @click="store.switchArtistAlbumSort('hot')"
          >
            最热专辑
          </button>
        </div>
      </div>
    </div>

    <!-- 1. 单曲列表视图 -->
    <div v-if="store.artistDetailSubTab === 'songs'" class="sdp-right-tracks-pane">
      <div class="sdp-track-table-head">
        <span class="col-idx">#</span>
        <span class="col-title">歌曲</span>
        <span class="col-artist">歌手</span>
        <span class="col-album">专辑</span>
        <span class="col-dur">时长</span>
        <span class="col-act">操作</span>
      </div>

      <div v-if="store.isLoadingArtist && store.activeArtistSongs.length === 0" class="sdp-empty-state">
        正在高速加载歌手代表作...
      </div>
      <div v-else-if="filteredSongs.length === 0" class="sdp-empty-state">
        暂无匹配单曲
      </div>

      <div v-else class="sdp-track-list">
        <div
          v-for="(song, idx) in filteredSongs"
          :key="song.hash + '_' + idx"
          class="sdp-track-row"
          :class="{ playing: store.currentSong?.hash === song.hash }"
          @click="store.playSongFromList(song, filteredSongs, `${artist?.name} · 代表作`)"
          @contextmenu.prevent="store.openSongContextMenu($event, song, filteredSongs)"
        >
          <div class="col-idx">
            <PlumpIcon
              v-if="store.currentSong?.hash === song.hash"
              name="wave"
              :size="15"
              color="var(--primary)"
            />
            <span v-else>{{ String(idx + 1).padStart(2, "0") }}</span>
          </div>

          <div class="col-title">
            <img v-if="song.coverUrl" :src="song.coverUrl" class="row-cover" loading="lazy" alt="" />
            <div class="row-title-meta">
              <div class="row-song-title-line">
                <span class="row-song-name">{{ song.title }}</span>
                <span v-if="song.isHiRes || song.hasHiRes" class="mini-badge hires">Hi-Res</span>
                <span v-else-if="song.isSq || song.isLossless || song.hasSq" class="mini-badge sq">SQ</span>
                <span v-else-if="song.hash320" class="mini-badge hq">HQ</span>
                <span v-if="song.isVip" class="mini-badge vip">VIP</span>
              </div>
            </div>
          </div>

          <div class="col-artist">
            <button class="clickable-meta-link" @click.stop="store.navigateToSongArtist(song)">
              {{ song.artist }}
            </button>
          </div>

          <div class="col-album">
            <button
              v-if="song.album"
              class="clickable-meta-link"
              @click.stop="store.navigateToSongAlbum(song)"
            >
              {{ song.album }}
            </button>
            <span v-else class="muted-dash">—</span>
          </div>

          <div class="col-dur">{{ formatDuration(song) }}</div>

          <div class="col-act" @click.stop>
            <button
              class="row-icon-btn"
              :class="{ liked: store.isSongLiked(song) }"
              title="喜欢"
              @click="store.toggleFavoriteSong(song)"
            >
              <PlumpIcon :name="store.isSongLiked(song) ? 'heart-filled' : 'heart'" :size="15" />
            </button>
            <button class="row-icon-btn" title="查看评论" @click="store.openCommentsModal(song)">
              <PlumpIcon name="comment" :size="15" />
            </button>
            <button class="row-icon-btn" title="收藏到歌单" @click="store.addToPlaylistModalSong = song">
              <PlumpIcon name="playlist-add" :size="15" />
            </button>
          </div>
        </div>

        <div v-if="store.artistSongsHasMore && !songFilterKeyword" class="sdp-load-more-row">
          <button
            class="tonal-pill-btn"
            :disabled="store.isLoadingMoreArtistSongs"
            @click="store.loadMoreArtistSongs()"
          >
            {{ store.isLoadingMoreArtistSongs ? "正在加载更多单曲..." : "加载更多歌手单曲" }}
          </button>
        </div>
      </div>
    </div>

    <!-- 2. 专辑网格视图（支持全量分页加载） -->
    <div v-else-if="store.artistDetailSubTab === 'albums'" class="artist-albums-section">
      <div
        v-if="store.activeArtistAlbums.length === 0"
        class="sdp-empty-state"
      >
        暂无收录专辑数据
      </div>
      <template v-else>
        <div class="artist-albums-grid">
          <div
            v-for="alb in store.activeArtistAlbums"
            :key="alb.albumId + '_' + alb.albumName"
            class="artist-album-card"
            @click="store.openAlbumDetail(alb.albumId, alb.albumName, alb.artistName || artist?.name || '', alb.coverUrl)"
          >
            <div class="aac-cover-wrap">
              <img v-if="alb.coverUrl" :src="alb.coverUrl" class="aac-cover" loading="lazy" alt="album" />
              <div v-else class="aac-cover fallback">
                <PlumpIcon name="disc" :size="36" color="#fff" />
              </div>
              <span class="aac-vinyl-rim"></span>
            </div>
            <div class="aac-title">{{ alb.albumName }}</div>
            <div class="aac-sub">
              <span>{{ alb.publishDate || "数字发行" }}</span>
              <span v-if="alb.songCount"> · {{ alb.songCount }} 首</span>
            </div>
          </div>
        </div>

        <div v-if="store.artistAlbumsHasMore" class="sdp-load-more-row">
          <button
            class="tonal-pill-btn"
            :disabled="store.isLoadingMoreArtistAlbums"
            @click="store.loadMoreArtistAlbums()"
          >
            {{ store.isLoadingMoreArtistAlbums ? "正在加载更多专辑..." : "加载更多歌手专辑" }}
          </button>
        </div>
      </template>
    </div>

    <!-- 3. 歌手百科完整文本介绍视图 -->
    <div v-else class="artist-bio-card">
      <h3>关于 {{ artist?.name }}</h3>
      <div class="bio-paragraphs">
        <p v-for="(para, i) in bioParagraphs" :key="i">{{ para }}</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.standalone-detail-page {
  padding: 8px 22px 110px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.sdp-top-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  min-height: 46px;
  padding-right: 184px;
}

.artist-hero-banner {
  display: flex;
  align-items: center;
  gap: 28px;
  padding: 26px 30px;
  border-radius: 26px;
  background: linear-gradient(
    135deg,
    color-mix(in srgb, var(--primary) 14%, var(--bg-card)),
    color-mix(in srgb, #6366f1 9%, var(--bg-card))
  );
  box-shadow: var(--shadow-soft);
}

:global(.scheme-watercolor_light) .artist-hero-banner,
:global(.scheme-light) .artist-hero-banner {
  background: linear-gradient(
    135deg,
    rgba(224, 242, 254, 0.88),
    rgba(238, 242, 255, 0.88),
    rgba(255, 255, 255, 0.92)
  );
  box-shadow: 0 12px 32px rgba(15, 23, 42, 0.06);
}

@media (max-width: 780px) {
  .artist-hero-banner {
    flex-direction: column;
    align-items: flex-start;
  }
}

.artist-avatar-halo {
  width: 144px;
  height: 144px;
  border-radius: 50%;
  padding: 4px;
  background: linear-gradient(135deg, var(--primary), #a855f7);
  flex-shrink: 0;
  box-shadow: 0 16px 38px rgba(0, 0, 0, 0.28);
}

.artist-hero-avatar {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  object-fit: cover;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-elevated);
}

.artist-hero-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.artist-badge-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.artist-verified-pill {
  font-size: 11.5px;
  font-weight: 800;
  padding: 3px 10px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--primary) 16%, transparent);
  color: var(--primary);
}

.artist-fans-pill {
  font-size: 12.5px;
  font-weight: 700;
  padding: 3px 10px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--text-primary) 7%, transparent);
  color: var(--text-secondary);
}

.artist-hero-name {
  font-size: 28px;
  font-weight: 900;
  margin: 0;
  color: var(--text-primary);
}

.artist-stat-counters {
  display: flex;
  align-items: center;
  gap: 18px;
}

.stat-box {
  display: flex;
  align-items: baseline;
  gap: 6px;
  cursor: pointer;
}

.stat-box .num {
  font-size: 18px;
  font-weight: 800;
  color: var(--primary);
}

.stat-box .lbl {
  font-size: 13px;
  color: var(--text-secondary);
}

.stat-divider {
  width: 1px;
  height: 14px;
  background: color-mix(in srgb, var(--text-primary) 14%, transparent);
}

.artist-short-intro {
  margin: 0;
  font-size: 13.5px;
  line-height: 1.58;
  color: var(--text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  cursor: pointer;
}

.artist-short-intro:hover {
  color: var(--text-primary);
}

.artist-hero-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 4px;
}

.artist-subnav-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  flex-wrap: wrap;
}

.artist-songs-tools {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.sdp-search-input-box {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 14px;
  height: 36px;
  border-radius: 999px;
  background: var(--bg-card);
  min-width: 220px;
}

:global(.scheme-watercolor_light) .sdp-search-input-box,
:global(.scheme-light) .sdp-search-input-box {
  background: rgba(255, 255, 255, 0.86);
}

.sdp-search-input-box input {
  background: transparent;
  border: none;
  color: var(--text-primary);
  font-size: 13px;
  width: 100%;
  outline: none;
}

.clear-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
}

.sdp-sort-pills {
  display: flex;
  gap: 6px;
  padding: 3px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
}

.sort-pill {
  border: none;
  background: transparent;
  color: var(--text-secondary);
  padding: 5px 13px;
  border-radius: 999px;
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.18s ease;
}

.sort-pill.active {
  background: var(--primary);
  color: #ffffff;
}

.sdp-right-tracks-pane {
  border-radius: 24px;
  background: var(--bg-card);
  padding: 16px 18px;
}

:global(.scheme-watercolor_light) .sdp-right-tracks-pane,
:global(.scheme-light) .sdp-right-tracks-pane {
  background: rgba(255, 255, 255, 0.86);
  box-shadow: 0 10px 28px rgba(15, 23, 42, 0.04);
}

.sdp-track-table-head,
.sdp-track-row {
  display: grid;
  grid-template-columns: 42px minmax(180px, 2.2fr) minmax(110px, 1.1fr) minmax(110px, 1.1fr) 68px 104px;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
}

.sdp-track-table-head {
  font-size: 12.5px;
  font-weight: 700;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.sdp-track-row {
  border-radius: 14px;
  cursor: pointer;
  transition: background 0.16s ease, transform 0.16s ease;
}

.sdp-track-row:hover {
  background: color-mix(in srgb, var(--primary) 9%, transparent);
}

.sdp-track-row.playing {
  background: color-mix(in srgb, var(--primary) 15%, transparent);
}

.col-idx {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-muted);
  text-align: center;
  display: flex;
  align-items: center;
  justify-content: center;
}

.col-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.row-cover {
  width: 40px;
  height: 40px;
  border-radius: 9px;
  object-fit: cover;
  flex-shrink: 0;
}

.row-title-meta {
  min-width: 0;
  flex: 1;
}

.row-song-title-line {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.row-song-name {
  font-size: 14.5px;
  font-weight: 700;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.col-artist,
.col-album {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.clickable-meta-link {
  background: none;
  border: none;
  padding: 0;
  color: var(--text-secondary);
  font-size: 13.5px;
  cursor: pointer;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.clickable-meta-link:hover {
  color: var(--primary);
  text-decoration: underline;
}

.muted-dash {
  color: var(--text-muted);
  opacity: 0.5;
}

.col-dur {
  font-size: 13px;
  color: var(--text-muted);
}

.col-act {
  display: flex;
  align-items: center;
  gap: 4px;
}

.row-icon-btn {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.row-icon-btn:hover {
  background: color-mix(in srgb, var(--text-primary) 10%, transparent);
  color: var(--text-primary);
}

.row-icon-btn.liked {
  color: #ff4d6d;
}

.sdp-load-more-row {
  display: flex;
  justify-content: center;
  padding: 18px 0 6px;
}

.artist-albums-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.artist-albums-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(176px, 1fr));
  gap: 18px;
}

.artist-album-card {
  padding: 14px;
  border-radius: 20px;
  background: var(--bg-card);
  cursor: pointer;
  transition: transform 0.2s ease, background 0.2s ease;
}

:global(.scheme-watercolor_light) .artist-album-card,
:global(.scheme-light) .artist-album-card {
  background: rgba(255, 255, 255, 0.86);
  box-shadow: 0 8px 22px rgba(15, 23, 42, 0.04);
}

.artist-album-card:hover {
  transform: translateY(-4px);
  background: color-mix(in srgb, var(--primary) 10%, var(--bg-card));
}

.aac-cover-wrap {
  width: 100%;
  aspect-ratio: 1 / 1;
  border-radius: 14px;
  overflow: hidden;
  margin-bottom: 10px;
  position: relative;
}

.aac-cover {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-elevated);
}

.aac-title {
  font-size: 14.5px;
  font-weight: 800;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.aac-sub {
  font-size: 12.5px;
  color: var(--text-secondary);
  margin-top: 4px;
}

.artist-bio-card {
  padding: 24px 28px;
  border-radius: 24px;
  background: var(--bg-card);
}

:global(.scheme-watercolor_light) .artist-bio-card,
:global(.scheme-light) .artist-bio-card {
  background: rgba(255, 255, 255, 0.86);
}

.artist-bio-card h3 {
  margin: 0 0 14px;
  font-size: 18px;
  color: var(--text-primary);
}

.bio-paragraphs {
  display: flex;
  flex-direction: column;
  gap: 12px;
  font-size: 14.5px;
  line-height: 1.75;
  color: var(--text-secondary);
}

.bio-paragraphs p {
  margin: 0;
}

.sdp-empty-state {
  text-align: center;
  padding: 56px 20px;
  color: var(--text-muted);
}

.spin-anim {
  animation: artist-spin 0.9s linear infinite;
}

@keyframes artist-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
