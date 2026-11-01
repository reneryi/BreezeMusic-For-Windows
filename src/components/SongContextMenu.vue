<script setup lang="ts">
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return "约 28.5 MB";
  return (bytes / (1024 * 1024)).toFixed(2) + " MB";
}

function handlePlayNow() {
  if (store.contextMenu.song) {
    store.playSongFromList(store.contextMenu.song);
  }
  store.closeSongContextMenu();
}

function handleToggleLike() {
  if (store.contextMenu.song) {
    store.toggleFavoriteSong(store.contextMenu.song);
  }
  store.closeSongContextMenu();
}

function handleOpenAddToPlaylist() {
  if (store.contextMenu.song) {
    store.addToPlaylistModalSong = store.contextMenu.song;
  }
  store.closeSongContextMenu();
}

function handleViewArtist() {
  const s = store.contextMenu.song;
  if (s) {
    store.openArtistDetail(s.artistId, s.artist, s.coverUrl);
  }
  store.closeSongContextMenu();
}

function handleViewAlbum() {
  const s = store.contextMenu.song;
  if (s) {
    store.openAlbumDetail(s.albumId, s.album || s.title, s.artist, s.coverUrl);
  }
  store.closeSongContextMenu();
}

function handleOpenDetail() {
  if (store.contextMenu.song) {
    store.songDetailModalSong = store.contextMenu.song;
  }
  store.closeSongContextMenu();
}

function handleDownload() {
  if (store.contextMenu.song) {
    store.triggerDownloadSong(store.contextMenu.song);
  }
  store.closeSongContextMenu();
}
</script>

<template>
  <!-- 1. 全局鼠标右键上下文菜单（替代手机端 ⋮ 底部抽屉） -->
  <transition name="pop-spring">
    <div
      v-if="store.contextMenu.visible && store.contextMenu.song"
      class="song-context-menu"
      :style="{
        left: `${store.contextMenu.x}px`,
        top: `${store.contextMenu.y}px`,
      }"
      @click.stop
    >
      <div class="scm-head">
        <img
          v-if="store.contextMenu.song.coverUrl"
          :src="store.contextMenu.song.coverUrl"
          class="scm-cover"
        />
        <div class="scm-meta">
          <div class="scm-title">{{ store.contextMenu.song.title }}</div>
          <div class="scm-artist">{{ store.contextMenu.song.artist }}</div>
        </div>
      </div>

      <button class="scm-item" @click="handlePlayNow">
        <PlumpIcon name="play" :size="15" color="var(--cyan)" />
        <span>立即播放</span>
      </button>

      <button class="scm-item" @click="handleToggleLike">
        <PlumpIcon
          name="heart"
          :size="15"
          :color="
            store.isSongLiked(store.contextMenu.song) ? '#ff4d6d' : 'currentColor'
          "
        />
        <span>
          {{
            store.isSongLiked(store.contextMenu.song)
              ? "从「我喜欢」中移除"
              : "添加到「我喜欢的音乐」"
          }}
        </span>
      </button>

      <button class="scm-item" @click="handleOpenAddToPlaylist">
        <PlumpIcon name="playlist-add" :size="15" />
        <span>收藏到自建歌单...</span>
      </button>

      <button class="scm-item" @click="handleViewArtist">
        <PlumpIcon name="compass" :size="15" />
        <span>查看歌手：{{ store.contextMenu.song.artist }}</span>
      </button>

      <button
        v-if="store.contextMenu.song.album"
        class="scm-item"
        @click="handleViewAlbum"
      >
        <PlumpIcon name="disc" :size="15" />
        <span>查看专辑：{{ store.contextMenu.song.album }}</span>
      </button>

      <button class="scm-item" @click="handleOpenDetail">
        <PlumpIcon name="wave" :size="15" />
        <span>查看音频硬件参数 (STREAMINFO)</span>
      </button>

      <!-- 仅当用户在「设置 -> 功能与隐私管控」中开启音乐下载开关后才显示下载项 -->
      <button
        v-if="store.downloadFeatureUnlocked"
        class="scm-item highlight"
        @click="handleDownload"
      >
        <PlumpIcon name="download" :size="15" accent="#00e5a3" />
        <span>下载歌曲 (内嵌 1000P 封面/歌词)</span>
      </button>
    </div>
  </transition>

  <!-- 2. 添加到自建歌单弹窗 -->
  <transition name="fade">
    <div
      v-if="store.addToPlaylistModalSong"
      class="modal-backdrop"
      @click.self="store.addToPlaylistModalSong = null"
    >
      <div class="compact-glass-modal">
        <div class="modal-header">
          <h3>收藏至我的歌单</h3>
          <button
            class="icon-close-btn"
            @click="store.addToPlaylistModalSong = null"
          >
            <PlumpIcon name="close" :size="15" />
          </button>
        </div>
        <p class="modal-sub-desc">
          将「{{ store.addToPlaylistModalSong.title }}」同步至酷狗概念版云歌单
        </p>

        <div class="playlist-pick-list">
          <button
            v-for="pl in store.userPlaylists.filter((p) => p.isCreatedByMe)"
            :key="pl.id"
            class="playlist-pick-row"
            @click="
              store.addSongToUserPlaylist(
                pl.listId,
                store.addToPlaylistModalSong!
              )
            "
          >
            <div class="ppr-left">
              <PlumpIcon
                :name="pl.isLiked ? 'heart' : 'library'"
                :size="18"
                :color="pl.isLiked ? '#ff4d6d' : 'var(--primary)'"
              />
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

  <!-- 3. 音频硬件参数与元数据详情弹窗 (对齐 Android SongDetailSheet.kt) -->
  <transition name="fade">
    <div
      v-if="store.songDetailModalSong"
      class="modal-backdrop"
      @click.self="store.songDetailModalSong = null"
    >
      <div class="compact-glass-modal">
        <div class="modal-header">
          <h3>音频流硬件规格 · STREAMINFO</h3>
          <button
            class="icon-close-btn"
            @click="store.songDetailModalSong = null"
          >
            <PlumpIcon name="close" :size="15" />
          </button>
        </div>

        <div class="song-spec-sheet">
          <div class="spec-hero-row">
            <img
              v-if="store.songDetailModalSong.coverUrl"
              :src="store.songDetailModalSong.coverUrl"
              class="spec-cover"
            />
            <div>
              <div class="spec-song-title">
                {{ store.songDetailModalSong.title }}
              </div>
              <div class="spec-song-artist">
                {{ store.songDetailModalSong.artist }}
              </div>
              <div class="spec-badges-row">
                <span class="mini-badge hires">
                  {{ store.songDetailModalSong.highestQuality || "Hi-Res" }}
                </span>
                <span class="mini-badge sq">RFC 9639 STREAMINFO</span>
              </div>
            </div>
          </div>

          <div class="spec-kv-grid">
            <div class="spec-kv-cell">
              <span class="k">所属专辑</span>
              <span class="v">
                {{ store.songDetailModalSong.album || "单曲发行" }}
              </span>
            </div>
            <div class="spec-kv-cell">
              <span class="k">发行日期</span>
              <span class="v">
                {{ store.songDetailModalSong.publishDate || "官方曲库收录" }}
              </span>
            </div>
            <div class="spec-kv-cell">
              <span class="k">最高音质档位</span>
              <span class="v">
                {{
                  store.songDetailModalSong.isHiRes
                    ? "24bit / 96.0kHz Hi-Res 母带"
                    : store.songDetailModalSong.isLossless
                    ? "16bit / 44.1kHz FLAC 无损"
                    : "320kbps HQ 高品"
                }}
              </span>
            </div>
            <div class="spec-kv-cell">
              <span class="k">无损流大小</span>
              <span class="v">
                {{
                  formatBytes(
                    store.songDetailModalSong.sizeHiRes ||
                      store.songDetailModalSong.sizeFlac ||
                      store.songDetailModalSong.size320
                  )
                }}
              </span>
            </div>
            <div class="spec-kv-cell full">
              <span class="k">曲目音源 Hash</span>
              <code class="v">{{ store.songDetailModalSong.hash }}</code>
            </div>
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>
