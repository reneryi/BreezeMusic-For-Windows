<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../stores/appStore";
import type { AudioQualityValue } from "../types";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return "约 28.5 MB";
  return (bytes / (1024 * 1024)).toFixed(2) + " MB";
}

const canRemoveFromCurrentPlaylist = computed(() => {
  if (
    store.activeTab !== "playlist_detail" &&
    (store.activeTab !== "library" || store.libraryDetailMode !== "playlist")
  ) {
    return false;
  }
  if (store.selectedPlaylist?.isLiked) return true;
  if (store.selectedPlaylist?.isCreatedByMe && store.selectedPlaylist?.listId) {
    return true;
  }
  return false;
});

function handlePlayNow() {
  if (store.contextMenu.song) {
    store.playSongFromList(store.contextMenu.song);
  }
  store.closeSongContextMenu();
}

function handlePlayNext() {
  if (store.contextMenu.song) {
    store.insertPlayNext(store.contextMenu.song);
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

function handleRemoveFromPlaylist() {
  if (store.contextMenu.song) {
    store.removeSongFromCurrentPlaylist(store.contextMenu.song);
  }
  store.closeSongContextMenu();
}

function handleViewArtist() {
  const s = store.contextMenu.song;
  store.closeSongContextMenu();
  if (s) {
    store.navigateToSongArtist(s);
  }
}

function handleViewAlbum() {
  const s = store.contextMenu.song;
  store.closeSongContextMenu();
  if (s) {
    store.navigateToSongAlbum(s);
  }
}

function handleOpenComments() {
  const s = store.contextMenu.song;
  store.closeSongContextMenu();
  if (s) {
    store.openCommentsModal(s);
  }
}

function handlePreviewHdCover() {
  const s = store.contextMenu.song;
  if (s?.coverUrl) {
    store.hdArtworkPreviewUrl = s.coverUrl.replace("/400/", "/1000/");
  }
  store.closeSongContextMenu();
}

function handleShareSong() {
  const s = store.contextMenu.song;
  if (s) {
    store.shareModalSong = s;
  }
  store.closeSongContextMenu();
}

function handleOpenDetail() {
  if (store.contextMenu.song) {
    store.openSongDetailModal(store.contextMenu.song);
  }
  store.closeSongContextMenu();
}

function handleOpenDownloadModal() {
  if (store.contextMenu.song) {
    store.downloadModalSong = store.contextMenu.song;
  }
  store.closeSongContextMenu();
}

const downloadTierChoices = computed(() => {
  const s = store.downloadModalSong;
  if (!s) return [];
  const tiers: {
    val: AudioQualityValue;
    label: string;
    badge: string;
    sizeText: string;
    available: boolean;
  }[] = [
    {
      val: "high",
      label: "Hi-Res 高解析度母带 (24bit FLAC)",
      badge: "Hi-Res",
      sizeText: formatBytes(s.sizeHiRes || 36_500_000),
      available: Boolean(s.isHiRes || s.hashHiRes),
    },
    {
      val: "flac",
      label: "SQ 无损音质 (16bit/44.1kHz FLAC)",
      badge: "SQ",
      sizeText: formatBytes(s.sizeFlac || 24_800_000),
      available: Boolean(s.isLossless || s.hashFlac || s.isHiRes),
    },
    {
      val: "320",
      label: "HQ 高品音质 (320kbps MP3)",
      badge: "320K",
      sizeText: formatBytes(s.size320 || 9_200_000),
      available: true,
    },
    {
      val: "128",
      label: "标准音质 (128kbps OGG/MP3)",
      badge: "128K",
      sizeText: formatBytes(s.size128 || 3_800_000),
      available: true,
    },
  ];
  return tiers;
});
</script>

<template>
  <!-- 1. 全局鼠标右键上下文菜单（对齐 Android SongActionBottomSheet 全量动作） -->
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

      <button class="scm-item" @click="handlePlayNext">
        <PlumpIcon name="top" :size="15" accent="var(--cyan)" />
        <span>下一首播放 (队列置顶)</span>
      </button>

      <button class="scm-item" @click="handleToggleLike">
        <PlumpIcon
          name="heart"
          :size="15"
          :color="
            store.isSongLiked(store.contextMenu.song)
              ? '#ff4d6d'
              : 'currentColor'
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

      <button
        v-if="canRemoveFromCurrentPlaylist"
        class="scm-item"
        style="color: var(--coral)"
        @click="handleRemoveFromPlaylist"
      >
        <PlumpIcon name="close" :size="15" color="var(--coral)" />
        <span>从当前歌单移除</span>
      </button>

      <button class="scm-item" @click="handleOpenComments">
        <PlumpIcon name="comment" :size="15" />
        <span>查看歌曲评论</span>
      </button>

      <button class="scm-item" @click="handleViewArtist">
        <PlumpIcon name="user" :size="15" />
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

      <button
        v-if="store.contextMenu.song.coverUrl"
        class="scm-item"
        @click="handlePreviewHdCover"
      >
        <PlumpIcon name="image" :size="15" />
        <span>查看 / 保存 1000P 超清封面</span>
      </button>

      <button class="scm-item" @click="handleShareSong">
        <PlumpIcon name="share" :size="15" />
        <span>分享歌曲口令 / 2K 歌词海报...</span>
      </button>

      <button class="scm-item" @click="handleOpenDetail">
        <PlumpIcon name="wave" :size="15" />
        <span>音频流百科与硬件参数 (STREAMINFO)</span>
      </button>

      <!-- 仅当用户在「设置 -> 功能与隐私管控」中开启音乐下载开关后才显示下载项 -->
      <button
        v-if="store.downloadFeatureUnlocked"
        class="scm-item highlight"
        @click="handleOpenDownloadModal"
      >
        <PlumpIcon name="download" :size="15" accent="#00e5a3" />
        <span>多音质下载 (内嵌 1000P 封面/歌词)...</span>
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
            v-for="pl in store.userPlaylists.filter((p) => p.isCreatedByMe && (p.title || p.name || '').trim() !== '默认收藏')"
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

  <!-- 3. 音频硬件参数与元数据百科详情弹窗 (对齐 Android SongDetailSheet.kt + fetchSongDetail) -->
  <transition name="fade">
    <div
      v-if="store.songDetailModalSong"
      class="modal-backdrop"
      @click.self="store.songDetailModalSong = null"
    >
      <div class="compact-glass-modal" style="width: 490px">
        <div class="modal-header">
          <h3>音频流硬件规格与唱片百科 · STREAMINFO</h3>
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
                <span v-if="store.isLoadingSongDetail" class="mini-badge vip">
                  正在探测流头...
                </span>
              </div>
            </div>
          </div>

          <div class="spec-kv-grid">
            <div class="spec-kv-cell">
              <span class="k">所属专辑</span>
              <span class="v">
                {{
                  store.activeSongDetailInfo?.albumName ||
                  store.songDetailModalSong.album ||
                  "单曲发行"
                }}
              </span>
            </div>
            <div class="spec-kv-cell">
              <span class="k">发行公司 / 厂牌</span>
              <span class="v">
                {{
                  store.activeSongDetailInfo?.company ||
                  "酷狗概念版正版数字曲库"
                }}
              </span>
            </div>
            <div class="spec-kv-cell">
              <span class="k">发行日期</span>
              <span class="v">
                {{
                  store.activeSongDetailInfo?.publishTime ||
                  store.songDetailModalSong.publishDate ||
                  "官方曲库收录"
                }}
              </span>
            </div>
            <div class="spec-kv-cell">
              <span class="k">最高音质档位</span>
              <span class="v">
                {{
                  store.activeSongDetailInfo?.audioQuality ||
                  (store.songDetailModalSong.isHiRes
                    ? "24bit / 96.0kHz Hi-Res 母带"
                    : store.songDetailModalSong.isLossless
                    ? "16bit / 44.1kHz FLAC 无损"
                    : "320kbps HQ 高品")
                }}
              </span>
            </div>
            <div class="spec-kv-cell">
              <span class="k">硬件采样率 / 位深</span>
              <span class="v">
                {{
                  store.activeSongDetailInfo?.sampleRate ||
                  (store.songDetailModalSong.isHiRes
                    ? "96.0 kHz / 24 bit"
                    : "44.1 kHz / 16 bit")
                }}
              </span>
            </div>
            <div class="spec-kv-cell">
              <span class="k">实测码率与大小</span>
              <span class="v">
                {{ store.activeSongDetailInfo?.bitRate || "920 kbps" }} ·
                {{
                  store.activeSongDetailInfo?.fileSize ||
                  formatBytes(
                    store.songDetailModalSong.sizeHiRes ||
                      store.songDetailModalSong.sizeFlac ||
                      store.songDetailModalSong.size320
                  )
                }}
              </span>
            </div>
            <div class="spec-kv-cell full">
              <span class="k">曲目音源 MD5 Hash</span>
              <code class="v">
                {{
                  store.activeSongDetailInfo?.fileHash ||
                  store.songDetailModalSong.hash.toUpperCase()
                }}
              </code>
            </div>
          </div>
        </div>
      </div>
    </div>
  </transition>

  <!-- 4. 多音质下载档位选择与二进制标签内嵌弹窗 (对齐 Android SongDownloadDialog.kt) -->
  <transition name="fade">
    <div
      v-if="store.downloadModalSong"
      class="modal-backdrop"
      @click.self="store.downloadModalSong = null"
    >
      <div class="compact-glass-modal" style="width: 480px">
        <div class="modal-header">
          <div>
            <h3>选择下载音质档位与标签配置</h3>
            <p class="vip-sub-caption">
              默认二进制内嵌 1000×1000 高清封面、完整歌手专辑标签与双语时间轴歌词
            </p>
          </div>
          <button
            class="icon-close-btn"
            @click="store.downloadModalSong = null"
          >
            <PlumpIcon name="close" :size="15" />
          </button>
        </div>

        <div class="spec-hero-row">
          <img
            v-if="store.downloadModalSong.coverUrl"
            :src="store.downloadModalSong.coverUrl"
            class="spec-cover"
          />
          <div>
            <div class="spec-song-title">
              {{ store.downloadModalSong.title }}
            </div>
            <div class="spec-song-artist">
              {{ store.downloadModalSong.artist }} ·
              {{ store.downloadModalSong.album || "单曲" }}
            </div>
          </div>
        </div>

        <!-- 行内快捷内嵌与防重复下载开关 -->
        <div style="display: flex; flex-wrap: wrap; gap: 10px; margin: 10px 0">
          <label class="bg-pill-btn" style="cursor: pointer; display: inline-flex; align-items: center; gap: 6px">
            <input v-model="store.embedCoverAndTags" type="checkbox" />
            <span>内嵌 1000P 封面与元数据</span>
          </label>
          <label class="bg-pill-btn" style="cursor: pointer; display: inline-flex; align-items: center; gap: 6px">
            <input v-model="store.exportStandaloneLrc" type="checkbox" />
            <span>导出同名 .lrc 歌词</span>
          </label>
          <label class="bg-pill-btn" style="cursor: pointer; display: inline-flex; align-items: center; gap: 6px">
            <input v-model="store.skipExistingDownloaded" type="checkbox" />
            <span>跳过同名已下载文件</span>
          </label>
        </div>

        <div class="playlist-pick-list">
          <button
            v-for="tier in downloadTierChoices"
            :key="tier.val"
            class="playlist-pick-row"
            :style="{ opacity: tier.available ? 1 : 0.65 }"
            @click="
              store.triggerDownloadSong(store.downloadModalSong!, tier.val)
            "
          >
            <div class="ppr-left">
              <span
                class="mini-badge"
                :class="
                  tier.val === 'high'
                    ? 'hires'
                    : tier.val === 'flac'
                    ? 'sq'
                    : 'vip'
                "
              >
                {{ tier.badge }}
              </span>
              <div>
                <div class="ppr-title">{{ tier.label }}</div>
                <div class="ppr-count">预估大小：{{ tier.sizeText }}</div>
              </div>
            </div>
            <PlumpIcon name="download" :size="16" color="var(--cyan)" />
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>
