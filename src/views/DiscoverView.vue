<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../stores/appStore";
import {
  Play,
  RefreshCw,
  ChevronRight,
  ArrowLeft,
  Trophy,
  Radio,
} from "lucide-vue-next";

const store = useAppStore();

const now = new Date();
const todayMonthDay = `${String(now.getMonth() + 1).padStart(2, "0")}/${String(
  now.getDate()
).padStart(2, "0")}`;
const todayMonthNum = String(now.getMonth() + 1).padStart(2, "0");
const todayDayNum = String(now.getDate()).padStart(2, "0");

const greetingSubtitle = computed(() => {
  const h = now.getHours();
  if (h < 6) return "深夜静谧 · 耳机里的温柔陪你入眠";
  if (h < 12) return "晨间微风 · 开启清爽元气的一天";
  if (h < 18) return "午后暇光 · 让无损旋律伴你专注";
  return "暮色微澜 · 卸下疲惫沉浸纯粹帧享";
});

const dailyHeroCover = computed(
  () => store.recommendSongs[0]?.coverUrl || ""
);

const newSongTabs = [
  { key: "CHINESE", label: "华语" },
  { key: "WESTERN", label: "欧美" },
  { key: "JPKR", label: "日韩" },
  { key: "CANTON", label: "粤语" },
  { key: "TODAY", label: "全部" },
];

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
    <!-- ==================== 1. 发现页主 Bento 卡片画廊 ==================== -->
    <template v-if="store.discoverSubView === 'home'">
      <header class="discover-top-header">
        <div>
          <h1 class="discover-main-title">发现</h1>
          <div class="discover-greeting-line">
            <span class="greeting-dot"></span>
            <span>{{ greetingSubtitle }}</span>
          </div>
        </div>
      </header>

      <!-- 双大卡 Bento 行：左「每日推荐 11/01」+ 右「猜你喜欢 · 随心听」 -->
      <section class="bento-hero-grid">
        <!-- 卡片 A：每日推荐 Hero 卡片（复刻视频 00:00 视觉） -->
        <div
          class="bento-daily-card"
          @click="store.discoverSubView = 'daily'"
        >
          <div
            class="bento-daily-bg"
            :style="
              dailyHeroCover
                ? { backgroundImage: `url(${dailyHeroCover})` }
                : {}
            "
          ></div>
          <div class="bento-daily-content">
            <img
              v-if="dailyHeroCover"
              :src="dailyHeroCover"
              class="daily-hero-thumb"
            />
            <div v-else class="daily-hero-thumb-placeholder">♪</div>

            <div class="daily-hero-center">
              <div class="daily-date-badge">{{ todayMonthDay }}</div>
              <div class="daily-hero-title">每日推荐</div>
              <div class="daily-hero-desc">懂你的今日专属旋律</div>
            </div>

            <button
              class="coral-play-fab"
              title="立即播放今日推荐"
              @click.stop="
                store.recommendSongs.length > 0 &&
                  store.playSongFromList(
                    store.recommendSongs[0],
                    store.recommendSongs
                  )
              "
            >
              <Play :size="22" />
            </button>
          </div>
        </div>

        <!-- 卡片 B：猜你喜欢 · 随心听（复刻视频 00:00 翠绿渐变卡片） -->
        <div class="bento-fm-card">
          <div class="fm-card-header">
            <div class="fm-title-group">
              <Radio :size="16" class="emerald-ico" />
              <span class="fm-card-title">猜你喜欢</span>
              <span class="fm-card-sub">· 随心听</span>
            </div>
            <button
              class="fm-refresh-btn"
              title="换一首随心听"
              @click="store.nextPersonalFmTrack(false)"
            >
              <RefreshCw
                :size="15"
                :class="{ 'spin-icon': store.isLoadingFm }"
              />
            </button>
          </div>

          <div
            v-if="store.currentFmSong"
            class="fm-track-body"
            @click="
              store.playSongFromList(
                store.currentFmSong,
                store.personalFmSongs
              )
            "
          >
            <img
              v-if="store.currentFmSong.coverUrl"
              :src="store.currentFmSong.coverUrl"
              class="fm-cover"
            />
            <div v-else class="fm-cover-placeholder">♪</div>

            <div class="fm-meta">
              <div class="fm-song-name">{{ store.currentFmSong.title }}</div>
              <div class="fm-artist-row">
                <span
                  v-if="store.currentFmSong.isHiRes"
                  class="mini-badge hires"
                >
                  Hi-Res
                </span>
                <span
                  v-else-if="store.currentFmSong.isLossless"
                  class="mini-badge sq"
                >
                  SQ
                </span>
                <span v-if="store.currentFmSong.isVip" class="mini-badge vip">
                  VIP
                </span>
                <span class="fm-artist-text">
                  {{ store.currentFmSong.artist }}
                </span>
              </div>
              <div v-if="store.nextFmSong" class="fm-next-preview">
                下一首：{{ store.nextFmSong.title }}
              </div>
            </div>

            <button
              class="emerald-play-btn"
              @click.stop="
                store.playSongFromList(
                  store.currentFmSong,
                  store.personalFmSongs
                )
              "
            >
              <Play :size="18" />
            </button>
          </div>
        </div>
      </section>

      <!-- 模块 2：新歌速递（华语 / 欧美 / 日韩 / 粤语 / 全部） -->
      <section class="discover-block">
        <div class="block-header">
          <div class="block-title-row">
            <h2 class="block-title">新歌速递</h2>
            <span class="block-sub">近 7 日首发 · 概念版无损音源</span>
          </div>

          <div class="block-actions">
            <div class="pill-tabs">
              <button
                v-for="t in newSongTabs"
                :key="t.key"
                class="pill-tab-btn"
                :class="{ active: store.newSongPeriod === t.key }"
                @click="store.loadNewSongExpress(t.key)"
              >
                {{ t.label }}
              </button>
            </div>
            <button
              v-if="store.newSongs.length > 0"
              class="accent-link-btn"
              @click="
                store.playSongFromList(store.newSongs[0], store.newSongs)
              "
            >
              <Play :size="14" />
              <span>全播</span>
            </button>
          </div>
        </div>

        <div class="newsong-bento-grid">
          <div
            v-for="song in store.newSongs.slice(0, 9)"
            :key="song.hash || song.id"
            class="newsong-card-item"
            :class="{ playing: store.currentSong?.hash === song.hash }"
            @click="store.playSongFromList(song, store.newSongs)"
          >
            <img
              v-if="song.coverUrl"
              :src="song.coverUrl"
              class="ns-cover"
              loading="lazy"
            />
            <div v-else class="ns-cover-placeholder">♪</div>
            <div class="ns-info">
              <div class="ns-title">{{ song.title }}</div>
              <div class="ns-meta">
                <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
                <span v-else-if="song.isLossless" class="mini-badge sq">SQ</span>
                <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                <span class="ns-artist">{{ song.artist }}</span>
              </div>
            </div>
            <span class="ns-date">
              {{ song.publishDate ? song.publishDate.slice(5) : todayMonthDay.replace('/', '-') }}
            </span>
          </div>
        </div>
      </section>

      <!-- 模块 3：巅峰排行榜画廊 -->
      <section class="discover-block">
        <div class="block-header">
          <div class="block-title-row">
            <h2 class="block-title">巅峰排行榜</h2>
            <span class="block-sub">官方实时榜单 · 点击查看完整榜单</span>
          </div>
        </div>

        <div class="rank-bento-grid">
          <div
            v-for="rk in store.rankList.slice(0, 6)"
            :key="rk.rankId"
            class="rank-bento-card"
            @click="store.openRankDetail(rk)"
          >
            <div class="rank-card-top">
              <div class="rank-icon-badge">
                <Trophy :size="18" />
              </div>
              <div class="rank-head-meta">
                <div class="rank-name">{{ rk.rankName }}</div>
                <div class="rank-freq">{{ rk.updateFrequency || "每日更新" }}</div>
              </div>
              <ChevronRight :size="16" class="rank-arrow" />
            </div>

            <div class="rank-preview-lines">
              <div
                v-for="(line, lIdx) in rk.topSongsPreview.slice(0, 3)"
                :key="lIdx"
                class="rank-preview-line"
              >
                {{ line }}
              </div>
              <div
                v-if="rk.topSongsPreview.length === 0"
                class="rank-preview-line muted"
              >
                点击加载 {{ rk.rankName }} 实时 TOP 曲目
              </div>
            </div>
          </div>
        </div>
      </section>
    </template>

    <!-- ==================== 2. 每日推荐全屏沉浸页（复刻视频 00:01 红字 01/11 头部） ==================== -->
    <template v-else-if="store.discoverSubView === 'daily'">
      <div class="subpage-banner">
        <button class="back-pill" @click="store.discoverSubView = 'home'">
          <ArrowLeft :size="16" />
          <span>返回发现</span>
        </button>

        <div class="daily-detail-hero">
          <div class="daily-big-date">
            <span class="big-day">{{ todayDayNum }}</span>
            <span class="big-month">/ {{ todayMonthNum }}</span>
          </div>
          <p class="daily-detail-sub">
            根据你的听歌口味 · 每日 6:00 更新 {{ store.recommendSongs.length }} 首
          </p>
          <button
            v-if="store.recommendSongs.length > 0"
            class="coral-play-all-pill"
            @click="
              store.playSongFromList(
                store.recommendSongs[0],
                store.recommendSongs
              )
            "
          >
            <Play :size="16" />
            <span>播放全部</span>
          </button>
        </div>
      </div>

      <div class="modern-track-list">
        <div
          v-for="(song, idx) in store.recommendSongs"
          :key="song.hash || song.id"
          class="modern-track-item"
          :class="{ playing: store.currentSong?.hash === song.hash }"
          @click="store.playSongFromList(song, store.recommendSongs)"
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
    </template>

    <!-- ==================== 3. 巅峰排行榜详情页 ==================== -->
    <template v-else-if="store.discoverSubView === 'rank' && store.activeRank">
      <div class="subpage-banner">
        <button class="back-pill" @click="store.discoverSubView = 'home'">
          <ArrowLeft :size="16" />
          <span>返回发现</span>
        </button>

        <div class="daily-detail-hero">
          <h2 class="rank-detail-big-title">{{ store.activeRank.rankName }}</h2>
          <p class="daily-detail-sub">
            {{ store.activeRank.updateFrequency }} · 共
            {{ store.activeRankSongs.length }} 首上榜曲目
          </p>
          <button
            v-if="store.activeRankSongs.length > 0"
            class="coral-play-all-pill"
            @click="
              store.playSongFromList(
                store.activeRankSongs[0],
                store.activeRankSongs
              )
            "
          >
            <Play :size="16" />
            <span>播放榜单全部</span>
          </button>
        </div>
      </div>

      <div v-if="store.isLoadingRankSongs" class="loading-box">
        正在拉取榜单实时曲目与无损音源规格...
      </div>

      <div v-else class="modern-track-list">
        <div
          v-for="(song, idx) in store.activeRankSongs"
          :key="song.hash || song.id"
          class="modern-track-item"
          :class="{ playing: store.currentSong?.hash === song.hash }"
          @click="store.playSongFromList(song, store.activeRankSongs)"
        >
          <div class="t-num" :class="{ top3: idx < 3 }">
            {{ String(idx + 1).padStart(2, "0") }}
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
            </div>
          </div>
          <div class="t-duration">{{ formatDuration(song.durationMs) }}</div>
        </div>
      </div>
    </template>
  </div>
</template>
