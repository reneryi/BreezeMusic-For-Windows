<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import type { Song } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const searchKeyword = ref("");
const sortMode = ref<"default" | "title" | "artist" | "duration">("default");
const isIntroExpanded = ref(false);

// 批量多选模式状态
const isBatchMode = ref(false);
const selectedHashes = ref<Set<string>>(new Set());
const showBatchAddToPlaylistModal = ref(false);

// 编辑歌单弹窗状态
const isEditModalOpen = ref(false);
const editTitle = ref("");
const editIntro = ref("");
const editCoverUrl = ref("");
const isSavingEdit = ref(false);

// 删除确认弹窗
const showDeleteConfirmModal = ref(false);

const playlist = computed(() => store.selectedPlaylist);
const isLikedPlaylist = computed(
  () => Boolean(playlist.value?.isLiked || playlist.value?.title === "我喜欢")
);
const canEditOrRemove = computed(
  () => Boolean(playlist.value?.isCreatedByMe && playlist.value?.listId)
);

watch(
  () => store.selectedPlaylist?.id,
  () => {
    searchKeyword.value = "";
    sortMode.value = "default";
    isBatchMode.value = false;
    selectedHashes.value = new Set();
    isIntroExpanded.value = false;
  }
);

const filteredSongs = computed<Song[]>(() => {
  const kw = searchKeyword.value.trim().toLowerCase();
  let list = [...store.playlistSongs];
  if (kw) {
    list = list.filter(
      (s) =>
        s.title.toLowerCase().includes(kw) ||
        s.artist.toLowerCase().includes(kw) ||
        (s.album || "").toLowerCase().includes(kw)
    );
  }
  if (sortMode.value === "title") {
    list.sort((a, b) => a.title.localeCompare(b.title, "zh-CN"));
  } else if (sortMode.value === "artist") {
    list.sort((a, b) => a.artist.localeCompare(b.artist, "zh-CN"));
  } else if (sortMode.value === "duration") {
    list.sort((a, b) => (b.durationMs || 0) - (a.durationMs || 0));
  }
  return list;
});

const coverImage = computed(() => {
  return (
    playlist.value?.coverUrl ||
    store.playlistSongs[0]?.coverUrl ||
    ""
  );
});

const playlistIntroText = computed(() => {
  const raw = (
    playlist.value?.intro ||
    playlist.value?.description ||
    ""
  ).trim();
  if (raw) return raw;
  if (isLikedPlaylist.value) {
    return "你在酷狗概念版标记红心的全部珍藏单曲，支持云端实时双向同步与无损畅听。";
  }
  return `由 ${playlist.value?.creator || "微风乐迷"} 创建的精选歌单，共收录 ${
    Math.max(playlist.value?.trackCount || 0, store.playlistSongs.length)
  } 首高品质曲目。`;
});

function playAll() {
  if (filteredSongs.value.length === 0) return;
  store.playSongFromList(
    filteredSongs.value[0],
    filteredSongs.value,
    playlist.value?.title || "歌单播放"
  );
}

function shufflePlayAll() {
  if (filteredSongs.value.length === 0) return;
  store.playMode = "shuffle";
  const randIdx = Math.floor(Math.random() * filteredSongs.value.length);
  store.playSongFromList(
    filteredSongs.value[randIdx],
    filteredSongs.value,
    playlist.value?.title || "歌单随机漫游"
  );
}

function openEditModal() {
  if (!playlist.value) return;
  editTitle.value = playlist.value.title || playlist.value.name || "";
  editIntro.value = playlist.value.intro || playlist.value.description || "";
  editCoverUrl.value = playlist.value.coverUrl || "";
  isEditModalOpen.value = true;
}

async function savePlaylistEdit() {
  if (!playlist.value?.listId || !editTitle.value.trim()) return;
  isSavingEdit.value = true;
  try {
    const ok = await store.updatePlaylistMeta(
      playlist.value.listId,
      editTitle.value.trim(),
      editIntro.value.trim(),
      editCoverUrl.value.trim()
    );
    if (ok) {
      isEditModalOpen.value = false;
    }
  } finally {
    isSavingEdit.value = false;
  }
}

function toggleBatchMode() {
  isBatchMode.value = !isBatchMode.value;
  selectedHashes.value = new Set();
}

function toggleSongChecked(song: Song) {
  const next = new Set(selectedHashes.value);
  if (next.has(song.hash)) {
    next.delete(song.hash);
  } else {
    next.add(song.hash);
  }
  selectedHashes.value = next;
}

function toggleSelectAll() {
  if (selectedHashes.value.size >= filteredSongs.value.length) {
    selectedHashes.value = new Set();
  } else {
    selectedHashes.value = new Set(filteredSongs.value.map((s) => s.hash));
  }
}

const selectedSongsList = computed(() =>
  filteredSongs.value.filter((s) => selectedHashes.value.has(s.hash))
);

function batchPlaySelected() {
  if (selectedSongsList.value.length === 0) return;
  store.playSongFromList(
    selectedSongsList.value[0],
    selectedSongsList.value,
    `${playlist.value?.title || "歌单"} · 已选曲目`
  );
}

async function handleBatchAddToTarget(listId: number) {
  if (selectedSongsList.value.length === 0) return;
  await store.batchAddSongsToPlaylist(listId, selectedSongsList.value);
  showBatchAddToPlaylistModal.value = false;
  isBatchMode.value = false;
  selectedHashes.value = new Set();
}

async function handleBatchRemoveSelected() {
  if (selectedSongsList.value.length === 0) return;
  const ok = await store.batchRemoveSongsFromPlaylist(selectedSongsList.value);
  if (ok) {
    isBatchMode.value = false;
    selectedHashes.value = new Set();
  }
}

async function confirmDeleteOrUncollect() {
  if (!playlist.value) return;
  showDeleteConfirmModal.value = false;
  if (playlist.value.isCreatedByMe && playlist.value.listId) {
    await store.deletePlaylist(playlist.value);
    store.navigateBack();
  } else {
    await store.toggleCollectPlaylist(playlist.value);
    store.navigateBack();
  }
}
</script>

<template>
  <section class="standalone-detail-page playlist-standalone-page">
    <!-- 顶部面包屑与操作条 -->
    <div class="sdp-top-bar">
      <button class="tonal-pill-btn" @click="store.navigateBack()">
        <PlumpIcon name="arrow-left" :size="16" />
        <span>返回上一页</span>
      </button>

      <div class="sdp-breadcrumb">
        <span>歌单独立工作台</span>
        <span v-if="playlist"> · {{ playlist.title }}</span>
      </div>

      <div class="sdp-top-actions">
        <!-- 歌单内独立的刷新按钮 -->
        <button
          class="tonal-pill-btn"
          :disabled="store.isLoadingLibrary"
          title="独立刷新当前歌单云端曲目与元数据"
          @click="store.refreshCurrentPlaylistOnly()"
        >
          <PlumpIcon name="refresh" :size="15" />
          <span>{{ store.isLoadingLibrary ? "正在同步..." : "刷新歌单" }}</span>
        </button>

        <button
          v-if="canEditOrRemove && !isLikedPlaylist"
          class="tonal-pill-btn"
          title="编辑歌单名称与简介"
          @click="openEditModal"
        >
          <PlumpIcon name="tuning" :size="15" />
          <span>编辑歌单</span>
        </button>

        <button
          class="tonal-pill-btn"
          :class="{ primary: isBatchMode }"
          @click="toggleBatchMode"
        >
          <PlumpIcon name="queue" :size="15" />
          <span>{{ isBatchMode ? `退出批量管理 (${selectedHashes.size})` : "批量操作" }}</span>
        </button>

        <button
          v-if="!isLikedPlaylist && playlist"
          class="tonal-pill-btn danger-outline"
          @click="showDeleteConfirmModal = true"
        >
          <PlumpIcon name="trash" :size="15" />
          <span>{{ playlist.isCreatedByMe ? "删除歌单" : "取消收藏" }}</span>
        </button>
      </div>
    </div>

    <!-- 歌单双栏大屏布局：左栏固定艺术海报与简介卡片 + 右栏曲目流 -->
    <div class="sdp-two-pane-layout">
      <!-- 左栏：封面海报、元信息、简介、快捷动作 -->
      <aside class="sdp-left-hero-card">
        <div class="sdp-cover-showcase">
          <img
            v-if="coverImage"
            :src="coverImage"
            class="sdp-cover-img"
            alt="cover"
          />
          <div v-else class="sdp-cover-fallback">
            <PlumpIcon :name="isLikedPlaylist ? 'heart-filled' : 'library'" :size="52" color="#fff" />
          </div>
          <span class="sdp-track-count-badge">
            {{ Math.max(playlist?.trackCount || 0, store.playlistSongs.length) }} 首曲目
          </span>
        </div>

        <h1 class="sdp-title">{{ playlist?.title || "精选歌单" }}</h1>

        <div class="sdp-creator-row">
          <span class="sdp-creator-tag">
            {{ isLikedPlaylist ? "♥ 我喜欢的音乐" : playlist?.isCreatedByMe ? "自建云歌单" : "收藏歌单" }}
          </span>
          <span class="sdp-creator-name">
            {{ playlist?.creator || store.profile?.nickname || "酷狗概念版" }}
          </span>
        </div>

        <div
          class="sdp-intro-box"
          :class="{ expanded: isIntroExpanded }"
          @click="isIntroExpanded = !isIntroExpanded"
        >
          <p class="sdp-intro-text">{{ playlistIntroText }}</p>
          <span v-if="playlistIntroText.length > 56" class="sdp-intro-more">
            {{ isIntroExpanded ? "收起简介" : "展开完整介绍" }}
          </span>
        </div>

        <div class="sdp-primary-cta-group">
          <button class="primary-btn full-w" @click="playAll">
            <PlumpIcon name="play" :size="17" color="#fff" />
            <span>播放全部 ({{ filteredSongs.length }})</span>
          </button>
          <button class="tonal-pill-btn full-w" @click="shufflePlayAll">
            <PlumpIcon name="shuffle" :size="16" />
            <span>随机漫游播放</span>
          </button>
        </div>
      </aside>

      <!-- 右栏：歌单内搜索、排序、批量管理栏与高密度曲目列表 -->
      <div class="sdp-right-tracks-pane">
        <!-- 歌单内搜索与排序栏 -->
        <div class="sdp-tracks-toolbar">
          <div class="sdp-search-input-box">
            <PlumpIcon name="search" :size="15" />
            <input
              v-model="searchKeyword"
              type="text"
              placeholder="在歌单内搜索歌曲、歌手或专辑..."
            />
            <button v-if="searchKeyword" class="clear-btn" @click="searchKeyword = ''">
              <PlumpIcon name="close" :size="13" />
            </button>
          </div>

          <div class="sdp-sort-pills">
            <button
              class="sort-pill"
              :class="{ active: sortMode === 'default' }"
              @click="sortMode = 'default'"
            >
              默认排序
            </button>
            <button
              class="sort-pill"
              :class="{ active: sortMode === 'title' }"
              @click="sortMode = 'title'"
            >
              按歌名 A-Z
            </button>
            <button
              class="sort-pill"
              :class="{ active: sortMode === 'artist' }"
              @click="sortMode = 'artist'"
            >
              按歌手
            </button>
            <button
              class="sort-pill"
              :class="{ active: sortMode === 'duration' }"
              @click="sortMode = 'duration'"
            >
              按时长
            </button>
          </div>
        </div>

        <!-- 批量多选操作浮动条 -->
        <transition name="fade">
          <div v-if="isBatchMode" class="sdp-batch-bar">
            <label class="batch-check-all" @click="toggleSelectAll">
              <input
                type="checkbox"
                :checked="
                  filteredSongs.length > 0 &&
                  selectedHashes.size === filteredSongs.length
                "
              />
              <span>全选 ({{ selectedHashes.size }} / {{ filteredSongs.length }})</span>
            </label>

            <div class="batch-actions">
              <button
                class="tonal-pill-btn sm primary"
                :disabled="selectedHashes.size === 0"
                @click="batchPlaySelected"
              >
                <PlumpIcon name="play" :size="14" />
                <span>播放已选</span>
              </button>
              <button
                class="tonal-pill-btn sm"
                :disabled="selectedHashes.size === 0"
                @click="showBatchAddToPlaylistModal = true"
              >
                <PlumpIcon name="playlist-add" :size="14" />
                <span>批量收藏到歌单</span>
              </button>
              <button
                v-if="canEditOrRemove"
                class="tonal-pill-btn sm danger-outline"
                :disabled="selectedHashes.size === 0"
                @click="handleBatchRemoveSelected"
              >
                <PlumpIcon name="trash" :size="14" />
                <span>批量移除 ({{ selectedHashes.size }})</span>
              </button>
            </div>
          </div>
        </transition>

        <!-- 曲目列表表头 -->
        <div class="sdp-track-table-head">
          <span class="col-idx">#</span>
          <span class="col-title">歌曲标题 / 音质标识</span>
          <span class="col-artist">歌手 (点击跳转主页)</span>
          <span class="col-album">专辑 (点击跳转专辑)</span>
          <span class="col-dur">时长</span>
          <span class="col-act">操作</span>
        </div>

        <div v-if="store.isLoadingLibrary && store.playlistSongs.length === 0" class="sdp-empty-state">
          正在从酷狗概念版云端加载歌单曲目...
        </div>
        <div v-else-if="filteredSongs.length === 0" class="sdp-empty-state">
          {{ searchKeyword ? "未找到匹配的歌单曲目" : "当前歌单暂无曲目" }}
        </div>

        <div v-else class="sdp-track-list">
          <div
            v-for="(song, idx) in filteredSongs"
            :key="song.hash + '_' + idx"
            class="sdp-track-row"
            :class="{
              playing: store.currentSong?.hash === song.hash,
              selected: isBatchMode && selectedHashes.has(song.hash),
            }"
            @dblclick="store.playSongFromList(song, filteredSongs, playlist?.title || '歌单')"
            @contextmenu.prevent="store.openSongContextMenu($event, song, playlist)"
          >
            <div class="col-idx" @click.stop="isBatchMode ? toggleSongChecked(song) : store.playSongFromList(song, filteredSongs, playlist?.title || '歌单')">
              <input
                v-if="isBatchMode"
                type="checkbox"
                :checked="selectedHashes.has(song.hash)"
                @change="toggleSongChecked(song)"
              />
              <span v-else>{{ idx + 1 }}</span>
            </div>

            <div class="col-title" @click="store.playSongFromList(song, filteredSongs, playlist?.title || '歌单')">
              <img
                v-if="song.coverUrl"
                :src="song.coverUrl"
                class="row-cover"
                alt=""
              />
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

            <!-- 点击歌手准确跳转（多歌手弹出选择框） -->
            <div class="col-artist">
              <button
                class="clickable-meta-link"
                title="点击进入歌手主页"
                @click.stop="store.navigateToSongArtist(song)"
              >
                {{ song.artist }}
              </button>
            </div>

            <!-- 点击专辑准确跳转（缺失专辑 ID 自动反查补全） -->
            <div class="col-album">
              <button
                v-if="song.album"
                class="clickable-meta-link"
                title="点击进入专辑详情页"
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
                <PlumpIcon
                  :name="store.isSongLiked(song) ? 'heart-filled' : 'heart'"
                  :size="15"
                />
              </button>
              <button
                class="row-icon-btn"
                title="查看歌曲评论弹窗"
                @click="store.openCommentsModal(song)"
              >
                <PlumpIcon name="comment" :size="15" />
              </button>
              <button
                class="row-icon-btn"
                title="添加到歌单"
                @click="store.addToPlaylistModalSong = song"
              >
                <PlumpIcon name="playlist-add" :size="15" />
              </button>
              <button
                v-if="canEditOrRemove"
                class="row-icon-btn danger"
                title="从当前歌单移除"
                @click="store.removeSongFromCurrentPlaylist(song)"
              >
                <PlumpIcon name="close" :size="14" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 编辑歌单信息弹窗 -->
    <transition name="fade">
      <div
        v-if="isEditModalOpen"
        class="modal-backdrop"
        @click.self="isEditModalOpen = false"
      >
        <div class="compact-glass-modal" style="width: 440px">
          <div class="modal-header">
            <h3>编辑歌单信息</h3>
            <button class="icon-close-btn" @click="isEditModalOpen = false">
              <PlumpIcon name="close" :size="15" />
            </button>
          </div>

          <div style="display: flex; flex-direction: column; gap: 12px; margin-top: 8px">
            <div>
              <div class="cfg-label">歌单名称</div>
              <input
                v-model="editTitle"
                type="text"
                class="glass-input"
                placeholder="输入歌单名称..."
              />
            </div>
            <div>
              <div class="cfg-label">歌单简介 / 心情描述</div>
              <textarea
                v-model="editIntro"
                rows="3"
                class="glass-input"
                style="height: auto; padding: 10px 12px; resize: vertical"
                placeholder="写下这张歌单的灵感与介绍..."
              ></textarea>
            </div>
            <div>
              <div class="cfg-label">自定义封面图片 URL (可选)</div>
              <input
                v-model="editCoverUrl"
                type="text"
                class="glass-input"
                placeholder="留空则默认使用首曲高清封面"
              />
            </div>

            <div style="display: flex; justify-content: flex-end; gap: 10px; margin-top: 6px">
              <button class="tonal-pill-btn" @click="isEditModalOpen = false">取消</button>
              <button
                class="primary-btn"
                :disabled="!editTitle.trim() || isSavingEdit"
                @click="savePlaylistEdit"
              >
                {{ isSavingEdit ? "正在保存..." : "保存并同步云端" }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </transition>

    <!-- 批量添加到歌单弹窗 -->
    <transition name="fade">
      <div
        v-if="showBatchAddToPlaylistModal"
        class="modal-backdrop"
        @click.self="showBatchAddToPlaylistModal = false"
      >
        <div class="compact-glass-modal" style="width: 420px">
          <div class="modal-header">
            <h3>批量收藏 {{ selectedHashes.size }} 首歌曲至歌单</h3>
            <button class="icon-close-btn" @click="showBatchAddToPlaylistModal = false">
              <PlumpIcon name="close" :size="15" />
            </button>
          </div>
          <div class="playlist-pick-list">
            <button
              v-for="pl in store.userPlaylists.filter((p) => p.isCreatedByMe && p.listId)"
              :key="pl.id"
              class="playlist-pick-row"
              @click="handleBatchAddToTarget(pl.listId)"
            >
              <div class="ppr-left">
                <PlumpIcon :name="pl.isLiked ? 'heart' : 'library'" :size="18" />
                <div>
                  <div class="ppr-title">{{ pl.title }}</div>
                  <div class="ppr-count">{{ pl.trackCount }} 首曲目</div>
                </div>
              </div>
              <PlumpIcon name="plus" :size="15" />
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 删除/取消收藏确认弹窗 -->
    <transition name="fade">
      <div
        v-if="showDeleteConfirmModal"
        class="modal-backdrop"
        @click.self="showDeleteConfirmModal = false"
      >
        <div class="compact-glass-modal" style="width: 380px">
          <div class="modal-header">
            <h3>{{ playlist?.isCreatedByMe ? "确认删除云歌单" : "确认取消收藏歌单" }}</h3>
            <button class="icon-close-btn" @click="showDeleteConfirmModal = false">
              <PlumpIcon name="close" :size="15" />
            </button>
          </div>
          <p style="font-size: 14px; color: var(--text-secondary); margin: 12px 0 18px">
            确定要{{ playlist?.isCreatedByMe ? "删除" : "取消收藏" }}歌单「{{ playlist?.title }}」吗？该操作将同步至酷狗概念版云端。
          </p>
          <div style="display: flex; justify-content: flex-end; gap: 10px">
            <button class="tonal-pill-btn" @click="showDeleteConfirmModal = false">取消</button>
            <button class="primary-btn" style="background: #f43f5e" @click="confirmDeleteOrUncollect">
              确认{{ playlist?.isCreatedByMe ? "删除" : "取消收藏" }}
            </button>
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
  flex-wrap: wrap;
  min-height: 46px;
  padding-right: 184px;
}
.sdp-breadcrumb {
  font-size: 13.5px;
  font-weight: 700;
  color: var(--text-secondary);
}
.sdp-top-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.sdp-two-pane-layout {
  display: grid;
  grid-template-columns: 300px 1fr;
  gap: 24px;
  align-items: start;
}
@media (max-width: 980px) {
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
.sdp-cover-showcase {
  width: 100%;
  aspect-ratio: 1 / 1;
  border-radius: 18px;
  overflow: hidden;
  position: relative;
  box-shadow: 0 18px 42px rgba(0, 0, 0, 0.45);
  background: linear-gradient(135deg, rgba(0, 229, 255, 0.22), rgba(108, 92, 231, 0.35));
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
}
.sdp-track-count-badge {
  position: absolute;
  right: 10px;
  bottom: 10px;
  padding: 4px 10px;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.68);
  backdrop-filter: blur(8px);
  color: #fff;
  font-size: 12px;
  font-weight: 700;
}
.sdp-title {
  font-size: 21px;
  font-weight: 800;
  line-height: 1.3;
  margin: 0;
}
.sdp-creator-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.sdp-creator-tag {
  font-size: 11.5px;
  font-weight: 700;
  padding: 2px 9px;
  border-radius: 999px;
  background: rgba(0, 229, 255, 0.14);
  color: var(--cyan);
}
.sdp-creator-name {
  font-size: 13px;
  color: var(--text-secondary);
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
.sdp-intro-box.expanded .sdp-intro-text {
  -webkit-line-clamp: unset;
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
  margin-top: 4px;
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
  flex: 1;
  max-width: 340px;
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
  flex-wrap: wrap;
}
.sort-pill {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: var(--text-secondary);
  padding: 5px 12px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
}
.sort-pill.active {
  background: rgba(0, 229, 255, 0.16);
  color: var(--cyan);
  border-color: rgba(0, 229, 255, 0.4);
}
.sdp-batch-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-radius: 14px;
  background: rgba(0, 229, 255, 0.09);
  border: 1px solid rgba(0, 229, 255, 0.28);
  flex-wrap: wrap;
  gap: 10px;
}
.batch-check-all {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
}
.batch-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.sdp-track-table-head,
.sdp-track-row {
  display: grid;
  grid-template-columns: 42px minmax(180px, 2.2fr) minmax(110px, 1.1fr) minmax(110px, 1.1fr) 68px 118px;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
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
  text-align: center;
  cursor: pointer;
}
.col-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  cursor: pointer;
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
  font-size: 13px;
  cursor: pointer;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: color 0.15s ease;
}
.clickable-meta-link:hover {
  color: var(--cyan);
  text-decoration: underline;
}
.muted-dash {
  color: var(--text-secondary);
  opacity: 0.5;
}
.col-dur {
  font-size: 12.5px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
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
.row-icon-btn.danger:hover {
  color: #f43f5e;
}
.sdp-empty-state {
  text-align: center;
  padding: 56px 20px;
  color: var(--text-secondary);
  font-size: 14px;
}
.danger-outline {
  border-color: rgba(244, 63, 94, 0.4) !important;
  color: #fb7185 !important;
}
</style>
