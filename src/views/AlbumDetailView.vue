<script setup lang="ts">
import { ref, computed } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const searchKeyword = ref("");
const isIntroModalOpen = ref(false);

const album = computed(() => store.activeAlbumDetail);

const filteredSongs = computed(() => {
  const kw = searchKeyword.value.trim().toLowerCase();
  if (!kw) return store.activeAlbumSongs;
  return store.activeAlbumSongs.filter(
    (s) =>
      s.title.toLowerCase().includes(kw) ||
      s.artist.toLowerCase().includes(kw)
  );
});

const albumCover = computed(
  () => album.value?.coverUrl || store.activeAlbumSongs[0]?.coverUrl || ""
);

const albumIntroText = computed(() => {
  const raw = (album.value?.intro || "").trim();
  if (raw) return raw;
  return `《${album.value?.albumName || "数字专辑"}》由 ${
    album.value?.artistName || "官方音乐人"
  } 创作发行，收录 ${Math.max(
    album.value?.songCount || 0,
    store.activeAlbumSongs.length
  )} 首正版高解析度音轨。`;
});

function playAll() {
  if (filteredSongs.value.length === 0) return;
  store.playSongFromList(
    filteredSongs.value[0],
    filteredSongs.value,
    `专辑《${album.value?.albumName || "专辑"}》`
  );
}

function shufflePlay() {
  if (filteredSongs.value.length === 0) return;
  store.playMode = "shuffle";
  const rand = Math.floor(Math.random() * filteredSongs.value.length);
  store.playSongFromList(
    filteredSongs.value[rand],
    filteredSongs.value,
    `专辑《${album.value?.albumName || "专辑"}》`
  );
}

async function refreshAlbum() {
  if (!album.value) return;
  await store.openAlbumDetail(
    album.value.albumId || "0",
    album.value.albumName,
    album.value.artistName,
    album.value.coverUrl
  );
  store.notify(`已刷新专辑《${album.value.albumName}》曲目与信息`);
}

function goToAlbumArtist() {
  if (!album.value) return;
  store.openArtistDetail(
    album.value.artistId || "0",
    album.value.artistName,
    album.value.coverUrl
  );
}
</script>

<template>
  <section class="standalone-detail-page album-standalone-page">
    <!-- 顶部面包屑与返回栏 -->
    <div class="sdp-top-bar">
      <button class="tonal-pill-btn" @click="store.navigateBack()">
        <PlumpIcon name="arrow-left" :size="16" />
        <span>返回上一页</span>
      </button>

      <div class="sdp-breadcrumb">
        <span>专辑独立详情页</span>
        <span v-if="album"> · 《{{ album.albumName }}》</span>
      </div>

      <div class="sdp-top-actions">
        <button
          class="tonal-pill-btn"
          :disabled="store.isLoadingAlbum"
          @click="refreshAlbum"
        >
          <PlumpIcon name="refresh" :size="15" />
          <span>{{ store.isLoadingAlbum ? "正在同步..." : "刷新专辑" }}</span>
        </button>
      </div>
    </div>

    <!-- 双栏大屏布局：左栏黑胶数字唱片封面与发行参数 + 右栏专辑音轨列表 -->
    <div class="sdp-two-pane-layout">
      <aside class="sdp-left-hero-card">
        <div class="album-vinyl-showcase">
          <div class="vinyl-disc-shadow"></div>
          <img
            v-if="albumCover"
            :src="albumCover"
            class="sdp-cover-img"
            alt="album"
          />
          <div v-else class="sdp-cover-fallback">
            <PlumpIcon name="disc" :size="52" color="#fff" />
          </div>
          <span class="sdp-track-count-badge">
            共 {{ Math.max(album?.songCount || 0, store.activeAlbumSongs.length) }} 首音轨
          </span>
        </div>

        <h1 class="sdp-title">{{ album?.albumName || "专辑详情" }}</h1>

        <div class="album-meta-kv">
          <div class="kv-row">
            <span class="k">创作歌手</span>
            <button class="v-link" @click="goToAlbumArtist">
              {{ album?.artistName || store.activeAlbumSongs[0]?.artist || "未知歌手" }}
            </button>
          </div>
          <div class="kv-row">
            <span class="k">发行时间</span>
            <span class="v">{{ album?.publishDate || "正版数字发行" }}</span>
          </div>
          <div class="kv-row">
            <span class="k">发行厂牌</span>
            <span class="v">{{ album?.publishCompany || "酷狗概念版正版曲库" }}</span>
          </div>
        </div>

        <div class="sdp-intro-box" @click="isIntroModalOpen = true">
          <p class="sdp-intro-text">{{ albumIntroText }}</p>
          <span class="sdp-intro-more">点击查看完整唱片文案 →</span>
        </div>

        <div class="sdp-primary-cta-group">
          <button class="primary-btn full-w" @click="playAll">
            <PlumpIcon name="play" :size="17" color="#fff" />
            <span>播放整张专辑 ({{ filteredSongs.length }})</span>
          </button>
          <button class="tonal-pill-btn full-w" @click="shufflePlay">
            <PlumpIcon name="shuffle" :size="16" />
            <span>随机播放专辑</span>
          </button>
        </div>
      </aside>

      <div class="sdp-right-tracks-pane">
        <div class="sdp-tracks-toolbar">
          <div class="sdp-search-input-box">
            <PlumpIcon name="search" :size="15" />
            <input
              v-model="searchKeyword"
              type="text"
              placeholder="在专辑内检索曲目名称..."
            />
            <button v-if="searchKeyword" class="clear-btn" @click="searchKeyword = ''">
              <PlumpIcon name="close" :size="13" />
            </button>
          </div>

          <span style="font-size: 12.5px; color: var(--text-secondary)">
            最高支持 Hi-Res 24bit 母带级无损播放
          </span>
        </div>

        <div class="sdp-track-table-head">
          <span class="col-idx">Track</span>
          <span class="col-title">曲目标题 / 音质规格</span>
          <span class="col-artist">演唱者 (点击跳转)</span>
          <span class="col-dur">时长</span>
          <span class="col-act">操作</span>
        </div>

        <div v-if="store.isLoadingAlbum && store.activeAlbumSongs.length === 0" class="sdp-empty-state">
          正在高速加载专辑音轨...
        </div>
        <div v-else-if="filteredSongs.length === 0" class="sdp-empty-state">
          该专辑暂无可展示的曲目
        </div>

        <div v-else class="sdp-track-list">
          <div
            v-for="(song, idx) in filteredSongs"
            :key="song.hash + '_' + idx"
            class="sdp-track-row"
            :class="{ playing: store.currentSong?.hash === song.hash }"
            @dblclick="store.playSongFromList(song, filteredSongs, `专辑《${album?.albumName}》`)"
            @contextmenu.prevent="store.openSongContextMenu($event, song)"
          >
            <div class="col-idx" @click="store.playSongFromList(song, filteredSongs, `专辑《${album?.albumName}》`)">
              {{ String(idx + 1).padStart(2, "0") }}
            </div>

            <div class="col-title" @click="store.playSongFromList(song, filteredSongs, `专辑《${album?.albumName}》`)">
              <div class="row-title-meta">
                <div class="row-song-name">{{ song.title }}</div>
                <div class="row-badges">
                  <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                  <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
                  <span v-else-if="song.isSq || song.isLossless" class="mini-badge sq">SQ</span>
                  <span v-else-if="song.hash320" class="mini-badge hq">HQ</span>
                </div>
              </div>
            </div>

            <div class="col-artist">
              <button
                class="clickable-meta-link"
                @click.stop="store.navigateToSongArtist(song)"
              >
                {{ song.artist }}
              </button>
            </div>

            <div class="col-dur">{{ formatDuration(song) }}</div>

            <div class="col-act" @click.stop>
              <button
                class="row-icon-btn"
                :class="{ liked: store.isSongLiked(song) }"
                @click="store.toggleFavoriteSong(song)"
              >
                <PlumpIcon :name="store.isSongLiked(song) ? 'heart-filled' : 'heart'" :size="15" />
              </button>
              <button class="row-icon-btn" title="查看歌曲评论" @click="store.openCommentsModal(song)">
                <PlumpIcon name="comment" :size="15" />
              </button>
              <button class="row-icon-btn" title="收藏至歌单" @click="store.addToPlaylistModalSong = song">
                <PlumpIcon name="playlist-add" :size="15" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 完整专辑文案弹窗 -->
    <transition name="fade">
      <div
        v-if="isIntroModalOpen"
        class="modal-backdrop"
        @click.self="isIntroModalOpen = false"
      >
        <div class="compact-glass-modal" style="width: 540px; max-height: 76vh; display: flex; flex-direction: column">
          <div class="modal-header">
            <h3>《{{ album?.albumName }}》专辑完整介绍</h3>
            <button class="icon-close-btn" @click="isIntroModalOpen = false">
              <PlumpIcon name="close" :size="15" />
            </button>
          </div>
          <div style="overflow-y: auto; line-height: 1.75; font-size: 14px; color: var(--text-secondary); white-space: pre-wrap; padding: 6px 2px">
            {{ albumIntroText }}
          </div>
        </div>
      </div>
    </transition>
  </section>
</template>

<style scoped>
.standalone-detail-page {
  padding: 16px 26px 110px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.sdp-top-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}
.sdp-breadcrumb {
  font-size: 13.5px;
  font-weight: 700;
  color: var(--text-secondary);
}
.sdp-two-pane-layout {
  display: grid;
  grid-template-columns: 310px 1fr;
  gap: 24px;
  align-items: start;
}
@media (max-width: 960px) {
  .sdp-two-pane-layout {
    grid-template-columns: 1fr;
  }
}
.sdp-left-hero-card {
  padding: 22px;
  border-radius: 24px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.09);
  display: flex;
  flex-direction: column;
  gap: 14px;
  position: sticky;
  top: 16px;
}
.album-vinyl-showcase {
  width: 100%;
  aspect-ratio: 1 / 1;
  border-radius: 18px;
  overflow: hidden;
  position: relative;
  box-shadow: 0 18px 42px rgba(0, 0, 0, 0.45);
}
.sdp-cover-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.sdp-cover-fallback {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #0ea5e9, #6366f1);
}
.sdp-track-count-badge {
  position: absolute;
  right: 10px;
  bottom: 10px;
  padding: 4px 10px;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.72);
  color: #fff;
  font-size: 12px;
  font-weight: 700;
}
.sdp-title {
  font-size: 21px;
  font-weight: 800;
  margin: 0;
}
.album-meta-kv {
  display: flex;
  flex-direction: column;
  gap: 7px;
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
}
.kv-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12.8px;
}
.kv-row .k {
  color: var(--text-secondary);
}
.kv-row .v {
  color: var(--text-primary);
  font-weight: 600;
}
.v-link {
  background: none;
  border: none;
  padding: 0;
  color: var(--cyan);
  font-weight: 700;
  cursor: pointer;
}
.v-link:hover {
  text-decoration: underline;
}
.sdp-intro-box {
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  cursor: pointer;
}
.sdp-intro-text {
  margin: 0;
  font-size: 12.8px;
  line-height: 1.55;
  color: var(--text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.sdp-intro-more {
  display: inline-block;
  margin-top: 4px;
  font-size: 11.5px;
  color: var(--cyan);
}
.sdp-primary-cta-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.full-w {
  width: 100%;
  justify-content: center;
}
.sdp-right-tracks-pane {
  border-radius: 24px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.sdp-tracks-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.sdp-search-input-box {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  height: 36px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.09);
  min-width: 240px;
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
.sdp-track-table-head,
.sdp-track-row {
  display: grid;
  grid-template-columns: 52px minmax(200px, 2.5fr) minmax(130px, 1.3fr) 72px 106px;
  align-items: center;
  gap: 12px;
  padding: 9px 12px;
}
.sdp-track-table-head {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-secondary);
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
}
.sdp-track-row {
  border-radius: 12px;
  transition: background 0.16s ease;
}
.sdp-track-row:hover {
  background: rgba(255, 255, 255, 0.05);
}
.sdp-track-row.playing {
  background: rgba(0, 229, 255, 0.11);
}
.col-idx {
  font-size: 12.5px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
  cursor: pointer;
}
.col-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  cursor: pointer;
}
.row-song-name {
  font-size: 14px;
  font-weight: 700;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.row-badges {
  display: flex;
  gap: 4px;
  margin-top: 3px;
}
.col-artist {
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
  font-size: 13px;
  cursor: pointer;
}
.clickable-meta-link:hover {
  color: var(--cyan);
  text-decoration: underline;
}
.col-dur {
  font-size: 12.5px;
  color: var(--text-secondary);
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
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-primary);
}
.row-icon-btn.liked {
  color: #ff4d6d;
}
.sdp-empty-state {
  text-align: center;
  padding: 56px 20px;
  color: var(--text-secondary);
}
</style>
