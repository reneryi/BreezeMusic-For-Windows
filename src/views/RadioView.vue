<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const categories = ["全部", "推荐电台", "场景", "心情", "曲风", "年代语种"];

const filteredStations = computed(() => {
  if (store.selectedFmCategory === "全部") return store.fmChannels;
  return store.fmChannels.filter(
    (c) => c.category === store.selectedFmCategory
  );
});

const activeChannelIdx = computed(() => {
  const idx = store.fmChannels.findIndex(
    (c) => c.id === store.activeFmChannel?.id
  );
  return idx >= 0 ? idx : 0;
});

function onTunerDialChange(e: Event) {
  const idx = Number((e.target as HTMLInputElement).value);
  const target = store.fmChannels[idx];
  if (target) {
    store.loadFmStationSongs(target.id, false, true);
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
  <div class="page-scroll-canvas" @click="store.closeSongContextMenu()">
    <header class="discover-top-header">
      <div>
        <h1 class="discover-main-title">随心听调频台 · FM Tuner</h1>
        <div class="discover-greeting-line">
          <span class="greeting-dot"></span>
          <span>拨动 FM 赫兹频率刻度尺 · 漫游全天候高解析场景电台</span>
        </div>
      </div>

      <div class="pill-tabs">
        <button
          v-for="cat in categories"
          :key="cat"
          class="pill-tab-btn"
          :class="{ active: store.selectedFmCategory === cat }"
          @click="store.selectedFmCategory = cat"
        >
          {{ cat }}
        </button>
      </div>
    </header>

    <!-- 上半部：复古未来主义 FM 硬件调谐器舞台 (对齐 Android RadioScreen.kt) -->
    <section class="radio-tuner-deck">
      <!-- 左侧：旋转电台黑胶唱盘 -->
      <div class="tuner-vinyl-col">
        <div
          class="tuner-vinyl-disc"
          :class="{ spinning: store.isPlaying }"
        >
          <img
            v-if="
              store.currentSong?.coverUrl ||
              store.fmStationSongs[0]?.coverUrl
            "
            :src="
              store.currentSong?.coverUrl ||
              store.fmStationSongs[0]?.coverUrl
            "
            class="tuner-vinyl-cover"
          />
          <div v-else class="tuner-vinyl-fallback">
            <PlumpIcon name="radio" :size="28" color="#00e5ff" />
          </div>
          <div class="tuner-vinyl-spindle"></div>
        </div>
      </div>

      <!-- 中部：发光 FM 赫兹刻度尺与滑块调谐器 -->
      <div class="tuner-dial-col">
        <div class="tuner-freq-readout">
          <span class="freq-led-dot"></span>
          <span class="freq-big-num">
            {{ store.activeFmChannel?.freqLabel || "FM 98.4 MHz" }}
          </span>
          <span class="freq-cat-pill">
            {{ store.activeFmChannel?.category || "推荐电台" }}
          </span>
          <span class="freq-listener-text">
            {{ store.activeFmChannel?.listenerCountText || "48.2k 正在漫游" }}
          </span>
        </div>

        <h2 class="tuner-station-title">
          {{ store.activeFmChannel?.name || "流行风向标" }}
        </h2>
        <p class="tuner-station-desc">
          {{
            store.activeFmChannel?.description ||
            "全天候智能音乐伴听 · 热歌与品味同频共振"
          }}
        </p>

        <!-- 拟真赫兹刻度条 -->
        <div class="tuner-scale-box">
          <div class="tuner-ticks">
            <span v-for="n in 21" :key="n" class="tick-bar" :class="{ major: n % 5 === 1 }"></span>
          </div>
          <input
            type="range"
            min="0"
            :max="Math.max(0, store.fmChannels.length - 1)"
            :value="activeChannelIdx"
            class="tuner-range-slider"
            title="左右拖动切换 FM 频道"
            @input="onTunerDialChange"
          />
        </div>

        <div class="tuner-action-row">
          <button
            class="coral-play-all-pill sm"
            @click="
              store.fmStationSongs.length > 0 &&
                store.playSongFromList(
                  store.fmStationSongs[0],
                  store.fmStationSongs,
                  store.activeFmChannel?.name || '随心听电台'
                )
            "
          >
            <PlumpIcon name="play" :size="15" color="#ffffff" />
            <span>播放电台 ({{ store.fmStationSongs.length }})</span>
          </button>

          <button
            class="tonal-pill-btn"
            @click="
              store.activeFmChannel &&
                store.loadFmStationSongs(store.activeFmChannel.id, false, false)
            "
          >
            <PlumpIcon
              name="refresh"
              :size="15"
              :class="{ 'spin-icon': store.isLoadingFmStation }"
            />
            <span>换一批</span>
          </button>

          <button
            class="tonal-pill-btn emerald"
            @click="
              store.activeFmChannel &&
                store.loadFmStationSongs(store.activeFmChannel.id, true, false)
            "
          >
            <PlumpIcon name="plus" :size="14" />
            <span>+20 首漫游续杯</span>
          </button>
        </div>
      </div>
    </section>

    <!-- 中部：分类电台快速矩阵 -->
    <section class="discover-block">
      <div class="fm-corridor-grid">
        <div
          v-for="ch in filteredStations"
          :key="ch.id"
          class="fm-station-card"
          :class="{ active: store.activeFmChannel?.id === ch.id }"
          @click="store.loadFmStationSongs(ch.id, false, true)"
        >
          <div class="fsc-top">
            <span class="fsc-freq-pill">{{ ch.freqLabel }}</span>
            <span class="fsc-cat-tag">{{ ch.category }}</span>
          </div>
          <div class="fsc-name">{{ ch.name }}</div>
          <div class="fsc-desc">{{ ch.description }}</div>
          <div class="fsc-bottom">
            <span class="fsc-listeners">{{ ch.listenerCountText }}</span>
            <button
              class="fsc-play-orb"
              @click.stop="store.loadFmStationSongs(ch.id, false, true)"
            >
              <PlumpIcon name="play" :size="15" color="#ffffff" />
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- 下半部：当前调频电台漫游曲目流 -->
    <section class="discover-block">
      <div class="block-header">
        <div class="block-title-row">
          <h2 class="block-title">
            「{{ store.activeFmChannel?.name || "电台" }}」漫游曲目池
          </h2>
          <span class="block-sub">
            共 {{ store.fmStationSongs.length }} 首无损候选曲目
          </span>
        </div>
      </div>

      <div v-if="store.isLoadingFmStation" class="loading-box">
        正在调频至【{{ store.activeFmChannel?.name }}】并解析无损音源规格...
      </div>

      <div v-else class="dual-column-song-grid">
        <div
          v-for="(song, idx) in store.fmStationSongs"
          :key="song.hash || idx"
          class="plump-song-card"
          :class="{
            'track-card-playing': store.currentSong?.hash === song.hash,
          }"
          @click="
            store.playSongFromList(
              song,
              store.fmStationSongs,
              store.activeFmChannel?.name || '随心听电台'
            )
          "
          @contextmenu="store.openSongContextMenu($event, song)"
        >
          <div class="psc-index">
            {{ String(idx + 1).padStart(2, "0") }}
          </div>
          <img v-if="song.coverUrl" :src="song.coverUrl" class="psc-cover" />
          <div class="psc-main">
            <div class="psc-title">{{ song.title }}</div>
            <div class="psc-sub">
              <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
              <span v-else-if="song.isLossless" class="mini-badge sq">SQ</span>
              <span v-if="song.isVip" class="mini-badge vip">VIP</span>
              <span>{{ song.artist }}</span>
            </div>
          </div>
          <div class="psc-dur">{{ formatDuration(song.durationMs) }}</div>
        </div>
      </div>
    </section>
  </div>
</template>
