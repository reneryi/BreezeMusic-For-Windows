<script setup lang="ts">
import { useAppStore } from "../stores/appStore";
import type { SongAuthor } from "../types";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

function chooseArtist(author: SongAuthor) {
  store.isMultiArtistModalOpen = false;
  store.navigateToSongArtist(null, author);
}
</script>

<template>
  <transition name="fade">
    <div
      v-if="store.isMultiArtistModalOpen"
      class="modal-backdrop"
      @click.self="store.isMultiArtistModalOpen = false"
    >
      <div class="compact-glass-modal" style="width: 420px; max-width: 92vw">
        <div class="modal-header">
          <div class="vip-title-row">
            <div class="vip-icon-orb cyan">
              <PlumpIcon name="user" :size="19" color="#00e5ff" />
            </div>
            <div>
              <h3>选择要前往的歌手主页</h3>
              <p class="vip-sub-caption">
                《{{ store.multiArtistSongTitle || "合唱单曲" }}》包含多位创作者
              </p>
            </div>
          </div>
          <button
            class="icon-close-btn"
            @click="store.isMultiArtistModalOpen = false"
          >
            <PlumpIcon name="close" :size="15" />
          </button>
        </div>

        <div style="display: flex; flex-direction: column; gap: 10px; max-height: 360px; overflow-y: auto; padding: 4px 2px">
          <button
            v-for="(author, idx) in store.multiArtistCandidates"
            :key="`${author.id}_${idx}`"
            class="multi-artist-pick-row"
            @click="chooseArtist(author)"
          >
            <div class="map-avatar-wrap">
              <img
                v-if="author.avatar"
                :src="author.avatar"
                alt="artist"
                class="map-avatar-img"
              />
              <div v-else class="map-avatar-fallback">
                {{ author.name.slice(0, 1) }}
              </div>
            </div>
            <div class="map-meta">
              <div class="map-name">{{ author.name }}</div>
              <div class="map-sub">
                {{ author.id && author.id !== "0" ? `歌手 ID: ${author.id} · 点击查看代表作与专辑` : "点击进入歌手详情页" }}
              </div>
            </div>
            <PlumpIcon name="chevron-right" :size="16" color="var(--text-secondary)" />
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.multi-artist-pick-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 14px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: var(--text-primary);
  text-align: left;
  cursor: pointer;
  transition: all 0.2s ease;
}
.multi-artist-pick-row:hover {
  background: rgba(0, 229, 255, 0.12);
  border-color: rgba(0, 229, 255, 0.35);
  transform: translateX(3px);
}
.map-avatar-wrap {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  overflow: hidden;
  flex-shrink: 0;
  background: linear-gradient(135deg, rgba(0, 229, 255, 0.25), rgba(108, 92, 231, 0.35));
  display: flex;
  align-items: center;
  justify-content: center;
}
.map-avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.map-avatar-fallback {
  font-size: 16px;
  font-weight: 800;
  color: #fff;
}
.map-meta {
  flex: 1;
  min-width: 0;
}
.map-name {
  font-size: 15px;
  font-weight: 700;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.map-sub {
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 2px;
}
</style>
