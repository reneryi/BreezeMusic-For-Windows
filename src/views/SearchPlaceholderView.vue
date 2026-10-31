<script setup lang="ts">
import { ref } from "vue";
import { useAppStore } from "../stores/appStore";
import { kugouClient } from "../api/kugou";
import type { Song } from "../types";
import { Search, Flame } from "lucide-vue-next";

const store = useAppStore();
const keyword = ref("");
const isSearching = ref(false);
const results = ref<Song[]>([]);

const hotTags = [
  "周杰伦",
  "林俊杰",
  "陈奕迅",
  "邓紫棋",
  "陶喆",
  "告五人",
  "川井憲次",
  "八三夭",
];

async function runSearch(kw?: string) {
  if (kw) keyword.value = kw;
  const q = keyword.value.trim();
  if (!q) return;
  isSearching.value = true;
  try {
    results.value = await kugouClient.searchSongs(q, 1, 40);
  } catch (e: any) {
    store.showBanner(`搜索失败：${e}`);
  } finally {
    isSearching.value = false;
  }
}

function formatDuration(ms: number): string {
  if (!ms || ms <= 0) return "03:30";
  const sec = Math.floor(ms / 1000);
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}
</script>

<template>
  <div class="page-scroll-canvas">
    <header class="discover-top-header">
      <div>
        <h1 class="discover-main-title">搜索</h1>
        <div class="discover-greeting-line">
          <span class="greeting-dot"></span>
          <span>概念版全曲库直连检索 · 自动补全 SQ / Hi-Res 无损音源规格</span>
        </div>
      </div>
    </header>

    <div class="glass-search-hero">
      <Search :size="19" class="cyan-ico" />
      <input
        v-model="keyword"
        type="text"
        placeholder="搜索歌曲、歌手、影视原声或专辑（按 Enter 立即搜索）..."
        class="glass-search-input"
        @keyup.enter="runSearch()"
      />
      <button class="primary-btn" :disabled="isSearching" @click="runSearch()">
        {{ isSearching ? "检索中..." : "搜索" }}
      </button>
    </div>

    <div class="hot-search-strip">
      <span class="hot-label">
        <Flame :size="14" />
        热门检索：
      </span>
      <button
        v-for="tag in hotTags"
        :key="tag"
        class="hot-pill"
        @click="runSearch(tag)"
      >
        {{ tag }}
      </button>
    </div>

    <div v-if="results.length > 0" class="modern-track-list">
      <div
        v-for="(song, idx) in results"
        :key="song.hash || song.id"
        class="modern-track-item"
        :class="{ playing: store.currentSong?.hash === song.hash }"
        @click="store.playSongFromList(song, results)"
      >
        <div class="t-num">
          <span v-if="store.currentSong?.hash === song.hash" class="t-playing">
            ▶
          </span>
          <span v-else>{{ String(idx + 1).padStart(2, "0") }}</span>
        </div>
        <img
          v-if="song.coverUrl"
          :src="song.coverUrl"
          class="t-cover"
          loading="lazy"
        />
        <div v-else class="t-cover-placeholder">♪</div>
        <div class="t-main">
          <div class="t-title">{{ song.title }}</div>
          <div class="t-sub">
            <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
            <span v-else-if="song.isLossless" class="mini-badge sq">SQ</span>
            <span v-if="song.isVip" class="mini-badge vip">VIP</span>
            <span>{{ song.artist }}</span>
            <span v-if="song.album" class="t-album">· {{ song.album }}</span>
          </div>
        </div>
        <div class="t-duration">{{ formatDuration(song.durationMs) }}</div>
      </div>
    </div>
  </div>
</template>
