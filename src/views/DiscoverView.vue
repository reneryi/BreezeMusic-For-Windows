<script setup lang="ts">
import { computed, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import type { Song } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const now = new Date();
const todayMonthDay = `${String(now.getMonth() + 1).padStart(2, "0")}/${String(
  now.getDate()
).padStart(2, "0")}`;
const todayMonthNum = String(now.getMonth() + 1).padStart(2, "0");
const todayDayNum = String(now.getDate()).padStart(2, "0");

// 近 7 日日期范围标签 (如 10.26 - 11.01)
const weekAgo = new Date(now.getTime() - 6 * 24 * 3600 * 1000);
const periodRangeLabel = `${String(weekAgo.getMonth() + 1).padStart(
  2,
  "0"
)}.${String(weekAgo.getDate()).padStart(2, "0")} - ${String(
  now.getMonth() + 1
).padStart(2, "0")}.${String(now.getDate()).padStart(2, "0")}`;

const greetingSubtitle = computed(() => {
  const h = now.getHours();
  if (h < 6) return "深夜静谧 · 耳机里的温柔陪你入眠";
  if (h < 9) return "晨光微熹 · 用第一缕旋律唤醒耳朵";
  if (h < 12) return "清风伴读 · 轻快节拍开启元气上午";
  if (h < 14) return "午间小憩 · 卸下忙碌享片刻松弛";
  if (h < 18) return "午后漫游 · 一杯咖啡与恰到好处的律动";
  if (h < 21) return "暮色微醺 · 晚风与落日余晖皆入歌";
  return "夜幕微蓝 · 卸下喧嚣沉浸私人旋律";
});

// 每日推荐三封面叠层 (StaggeredThreeCovers)
const dailyThreeCovers = computed(() => {
  return store.recommendSongs
    .map((s) => s.coverUrl)
    .filter((u) => Boolean(u))
    .slice(0, 3);
});

const newSongTabs = [
  { key: "CHINESE", label: "华语" },
  { key: "WESTERN", label: "欧美" },
  { key: "JPKR", label: "日韩" },
  { key: "CANTON", label: "粤语" },
  { key: "TODAY", label: "全部" },
];

// 新歌速递独立页：按发行日期 (publishDate) 折叠时间轴 + A-Z 首字母提取
const collapsedDates = ref<Record<string, boolean>>({});
const activeAlphaFilter = ref<string>("ALL");

function extractFirstLetter(title: string): string {
  const clean = title.replace(/^[\s\W_]+/, "").trim();
  if (!clean) return "#";
  const code = clean.charCodeAt(0);
  if ((code >= 65 && code <= 90) || (code >= 97 && code <= 122)) {
    return clean[0].toUpperCase();
  }
  // 中文标题使用 Unicode 区间映射常见拼音首字母段
  const alphabet = "ABCDEFGHJKLMNOPQRSTWXYZ";
  return alphabet[code % alphabet.length] || "A";
}

const newSongDateGroups = computed(() => {
  const map = new Map<string, Song[]>();
  const source =
    activeAlphaFilter.value === "ALL"
      ? store.newSongs
      : store.newSongs.filter(
          (s) => extractFirstLetter(s.title) === activeAlphaFilter.value
        );

  for (const song of source) {
    const d = song.publishDate || `${now.getFullYear()}-${todayMonthNum}-${todayDayNum}`;
    if (!map.has(d)) map.set(d, []);
    map.get(d)!.push(song);
  }
  return Array.from(map.entries()).map(([date, songs]) => ({
    date,
    songs,
  }));
});

const alphabetList = [
  "ALL",
  ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ".split(""),
];

function toggleAllDates(collapse: boolean) {
  const next: Record<string, boolean> = {};
  for (const g of newSongDateGroups.value) {
    next[g.date] = collapse;
  }
  collapsedDates.value = next;
}

// 榜单分类与搜索过滤
const rankCategories = ["全部", "官方热榜", "特色曲风", "全球权威"];
const filteredRankGrid = computed(() => {
  if (store.selectedRankCategory === "全部") return store.rankList;
  return store.rankList.filter(
    (r) => r.category === store.selectedRankCategory
  );
});

const filteredActiveRankSongs = computed(() => {
  const kw = store.rankSearchKeyword.trim().toLowerCase();
  if (!kw) return store.activeRankSongs;
  return store.activeRankSongs.filter(
    (s) =>
      s.title.toLowerCase().includes(kw) ||
      s.artist.toLowerCase().includes(kw) ||
      s.album.toLowerCase().includes(kw)
  );
});

// 电台分类过滤
const fmCategoryTabs = ["全部", "推荐电台", "场景", "心情", "曲风", "年代语种"];
const homeFmStations = computed(() => {
  if (store.selectedFmCategory === "全部") return store.fmChannels;
  return store.fmChannels.filter(
    (c) => c.category === store.selectedFmCategory
  );
});

// 一键平滑居中定位当前播放歌曲
function locateCurrentPlayingSong() {
  const el = document.querySelector(".track-card-playing");
  if (el) {
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    store.notify("已为您定位到当前播放歌曲");
  } else {
    store.notify("当前播放歌曲不在本列表中");
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
    <!-- ==================== 1. 发现页主天际线 Bento 画布 ==================== -->
    <template v-if="store.discoverSubView === 'home'">
      <header class="discover-top-header">
        <div>
          <h1 class="discover-main-title">探索发现</h1>
          <div class="discover-greeting-line">
            <span class="greeting-dot"></span>
            <span>{{ greetingSubtitle }}</span>
          </div>
        </div>

        <div class="discover-header-actions">
          <button
            class="tonal-pill-btn"
            title="刷新今日推荐与随心听曲库"
            @click="
              store.loadDailyRecommend();
              store.loadPersonalFm(false);
              store.notify('已同步刷新概念版发现页推荐流');
            "
          >
            <PlumpIcon name="refresh" :size="15" />
            <span>焕新推荐</span>
          </button>
        </div>
      </header>

      <!-- 首屏黄金双子星横幅：左「三封面叠层每日推荐」+ 右「猜你喜欢 · 随心听电台舱」 -->
      <section class="bento-hero-grid">
        <!-- 卡片 A：每日推荐（三封面立体叠层 StaggeredThreeCovers + 巨型日期） -->
        <div
          class="bento-daily-card"
          @click="store.discoverSubView = 'daily'"
        >
          <div
            class="bento-daily-bg"
            :style="
              dailyThreeCovers[0]
                ? { backgroundImage: `url(${dailyThreeCovers[0]})` }
                : {}
            "
          ></div>
          <div class="bento-daily-content">
            <!-- 三张专辑封面交错立体叠层（对齐 Android StaggeredThreeCovers） -->
            <div class="staggered-three-covers">
              <img
                v-if="dailyThreeCovers[2]"
                :src="dailyThreeCovers[2]"
                class="stc-img layer-back"
              />
              <img
                v-if="dailyThreeCovers[1]"
                :src="dailyThreeCovers[1]"
                class="stc-img layer-mid"
              />
              <img
                v-if="dailyThreeCovers[0]"
                :src="dailyThreeCovers[0]"
                class="stc-img layer-front"
              />
              <div v-else class="daily-hero-thumb-placeholder">
                <PlumpIcon name="disc" :size="28" />
              </div>
            </div>

            <div class="daily-hero-center">
              <div class="daily-date-badge">{{ todayMonthDay }} · 每日 6:00 更新</div>
              <div class="daily-hero-title">每日推荐</div>
              <div class="daily-hero-desc">
                循着你的听歌轨迹 · 今日精选 {{ store.recommendSongs.length || 30 }} 首无损旋律
              </div>
              <div
                v-if="store.recommendSongs.length > 0"
                class="daily-preview-mini"
              >
                <span
                  v-for="s in store.recommendSongs.slice(0, 2)"
                  :key="s.hash"
                  class="dp-chip"
                >
                  {{ s.title }}
                </span>
              </div>
            </div>

            <button
              class="coral-play-fab"
              title="立即播放今日推荐全部曲目"
              @click.stop="
                store.recommendSongs.length > 0 &&
                  store.playSongFromList(
                    store.recommendSongs[0],
                    store.recommendSongs,
                    '每日推荐'
                  )
              "
            >
              <PlumpIcon name="play" :size="22" color="#ffffff" />
            </button>
          </div>
        </div>

        <!-- 卡片 B：猜你喜欢 · 私人随心听（实时可交互微缩电台 + 20首漫游候选抽屉） -->
        <div class="bento-fm-card">
          <div class="fm-card-header">
            <div class="fm-title-group">
              <PlumpIcon
                name="radio"
                :size="18"
                color="#00e5a3"
                accent="#00e5ff"
              />
              <span class="fm-card-title">猜你喜欢</span>
              <span class="fm-card-sub">· 随心听 (5大AI引擎)</span>
            </div>

            <div class="fm-header-btns">
              <button
                class="fm-pill-mini"
                title="查看当前 20 首随心听漫游候选池"
                @click.stop="
                  store.isPersonalFmDrawerOpen = !store.isPersonalFmDrawerOpen
                "
              >
                <PlumpIcon name="queue" :size="14" />
                <span>漫游池 ({{ store.personalFmSongs.length }})</span>
              </button>
              <button
                class="fm-refresh-btn"
                title="重调 5 大 AI 推荐引擎换一批"
                @click.stop="store.loadPersonalFm(false)"
              >
                <PlumpIcon
                  name="refresh"
                  :size="15"
                  :class="{ 'spin-icon': store.isLoadingFm }"
                />
              </button>
            </div>
          </div>

          <div
            v-if="store.currentFmSong"
            class="fm-track-body"
            @click="
              store.playSongFromList(
                store.currentFmSong,
                store.personalFmSongs,
                '猜你喜欢 · 随心听'
              )
            "
            @contextmenu="store.openSongContextMenu($event, store.currentFmSong)"
          >
            <img
              v-if="store.currentFmSong.coverUrl"
              :src="store.currentFmSong.coverUrl"
              class="fm-cover"
            />
            <div v-else class="fm-cover-placeholder">
              <PlumpIcon name="disc" :size="24" />
            </div>

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
                下一首预告：{{ store.nextFmSong.title }} -
                {{ store.nextFmSong.artist }}
              </div>
            </div>

            <div class="fm-transport-cluster" @click.stop>
              <button
                class="emerald-play-btn"
                title="播放 / 暂停随心听"
                @click="
                  store.currentSong?.hash === store.currentFmSong?.hash
                    ? store.togglePlayPause()
                    : store.playSongFromList(
                        store.currentFmSong,
                        store.personalFmSongs,
                        '猜你喜欢 · 随心听'
                      )
                "
              >
                <PlumpIcon
                  :name="
                    store.isPlaying &&
                    store.currentSong?.hash === store.currentFmSong?.hash
                      ? 'pause'
                      : 'play'
                  "
                  :size="19"
                  color="#041f18"
                />
              </button>
              <button
                class="fm-skip-btn"
                title="切到下一首随心听"
                @click="store.nextPersonalFmTrack(true)"
              >
                <PlumpIcon name="next" :size="17" color="#e2f8f0" />
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- 猜你喜欢 · 随心听可展开漫游候选池抽屉（支持 +20首无缝续杯） -->
      <transition name="pop-spring">
        <section
          v-if="store.isPersonalFmDrawerOpen"
          class="fm-roaming-drawer"
        >
          <div class="fmd-head">
            <div class="fmd-title">
              <PlumpIcon name="radio" :size="18" color="#00e5a3" />
              <span>猜你喜欢 · 实时漫游候选列表 ({{ store.personalFmSongs.length }} 首)</span>
            </div>
            <div class="fmd-actions">
              <button
                class="tonal-pill-btn emerald"
                @click="store.loadPersonalFm(true)"
              >
                <PlumpIcon name="plus" :size="14" />
                <span>+20 首无缝续杯</span>
              </button>
              <button
                class="tonal-pill-btn"
                @click="store.isPersonalFmDrawerOpen = false"
              >
                收起
              </button>
            </div>
          </div>

          <div class="dual-column-song-grid compact">
            <div
              v-for="(s, idx) in store.personalFmSongs"
              :key="s.hash || idx"
              class="plump-song-card"
              :class="{
                'track-card-playing': store.currentSong?.hash === s.hash,
              }"
              @click="
                store.personalFmIndex = idx;
                store.playSongFromList(
                  s,
                  store.personalFmSongs,
                  '猜你喜欢 · 随心听'
                );
              "
              @contextmenu="store.openSongContextMenu($event, s)"
            >
              <img v-if="s.coverUrl" :src="s.coverUrl" class="psc-cover" />
              <div class="psc-main">
                <div class="psc-title">{{ s.title }}</div>
                <div class="psc-sub">
                  <span v-if="s.isHiRes" class="mini-badge hires">Hi-Res</span>
                  <span v-else-if="s.isLossless" class="mini-badge sq">SQ</span>
                  <span>{{ s.artist }}</span>
                </div>
              </div>
              <span class="psc-dur">{{ formatDuration(s.durationMs) }}</span>
            </div>
          </div>
        </section>
      </transition>

      <!-- 模块 2：新歌速递（5 语种切换 + 3×3 微卡矩阵 + 独立时间轴/A-Z索引页入口） -->
      <section class="discover-block">
        <div class="block-header">
          <div class="block-title-row">
            <h2 class="block-title">新歌速递</h2>
            <span class="block-period-tag">/ {{ periodRangeLabel }}</span>
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
              class="tonal-pill-btn"
              @click="
                store.playSongFromList(
                  store.newSongs[0],
                  store.newSongs,
                  '新歌速递'
                )
              "
            >
              <PlumpIcon name="play" :size="14" color="var(--cyan)" />
              <span>全播</span>
            </button>
            <button
              class="tonal-pill-btn primary"
              @click="store.discoverSubView = 'new_songs'"
            >
              <span>查看全部时间轴</span>
              <PlumpIcon name="chevron-right" :size="14" />
            </button>
          </div>
        </div>

        <div class="newsong-bento-grid">
          <div
            v-for="song in store.newSongs.slice(0, 9)"
            :key="song.hash || song.id"
            class="newsong-card-item"
            :class="{ playing: store.currentSong?.hash === song.hash }"
            @click="
              store.playSongFromList(song, store.newSongs, '新歌速递')
            "
            @contextmenu="store.openSongContextMenu($event, song)"
          >
            <div class="ns-cover-box">
              <img
                v-if="song.coverUrl"
                :src="song.coverUrl"
                class="ns-cover"
                loading="lazy"
              />
              <div v-else class="ns-cover-placeholder">
                <PlumpIcon name="disc" :size="20" />
              </div>
              <span class="ns-date-pill">
                {{
                  song.publishDate
                    ? song.publishDate.slice(5)
                    : todayMonthDay.replace("/", "-")
                }}
              </span>
            </div>

            <div class="ns-info">
              <div class="ns-title">{{ song.title }}</div>
              <div class="ns-meta">
                <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
                <span v-else-if="song.isLossless" class="mini-badge sq">SQ</span>
                <span v-if="song.isVip" class="mini-badge vip">VIP</span>
                <span class="ns-artist">{{ song.artist }}</span>
              </div>
            </div>

            <button
              class="ns-more-orb"
              title="更多操作"
              @click.stop="store.openSongContextMenu($event, song)"
            >
              <PlumpIcon name="more" :size="16" />
            </button>
          </div>
        </div>
      </section>

      <!-- 模块 3：精选场景电台走廊（带 FM 赫兹频率铭牌 + 一键调频漫游） -->
      <section class="discover-block">
        <div class="block-header">
          <div class="block-title-row">
            <h2 class="block-title">精选场景电台</h2>
            <span class="block-sub">全天候 FM 赫兹漫游 · 轻触卡片即刻调频</span>
          </div>

          <div class="block-actions">
            <div class="pill-tabs">
              <button
                v-for="cat in fmCategoryTabs"
                :key="cat"
                class="pill-tab-btn"
                :class="{ active: store.selectedFmCategory === cat }"
                @click="store.selectedFmCategory = cat"
              >
                {{ cat }}
              </button>
            </div>
            <button
              class="tonal-pill-btn primary"
              @click="store.activeTab = 'radio'"
            >
              <span>进入调频台</span>
              <PlumpIcon name="chevron-right" :size="14" />
            </button>
          </div>
        </div>

        <div class="fm-corridor-grid">
          <div
            v-for="ch in homeFmStations.slice(0, 5)"
            :key="ch.id"
            class="fm-station-card"
            :class="{ active: store.activeFmChannel?.id === ch.id }"
            @click="
              store.loadFmStationSongs(ch.id, false, true);
              store.activeTab = 'radio';
            "
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
                title="立即调频播放该电台"
                @click.stop="store.loadFmStationSongs(ch.id, false, true)"
              >
                <PlumpIcon name="play" :size="15" color="#ffffff" />
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- 模块 4：巅峰排行榜画廊（含「全部榜单宫格」选择器） -->
      <section class="discover-block">
        <div class="block-header">
          <div class="block-title-row">
            <h2 class="block-title">巅峰排行榜</h2>
            <span class="block-sub">官方实时榜单 · 覆盖热榜、特色曲风与全球权威</span>
          </div>

          <div class="block-actions">
            <button
              class="tonal-pill-btn primary"
              @click="store.isRankGridModalOpen = true"
            >
              <PlumpIcon name="grid" :size="15" />
              <span>全部榜单 ({{ store.rankList.length }})</span>
            </button>
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
              <img
                v-if="rk.coverUrl"
                :src="rk.coverUrl"
                class="rank-thumb-img"
              />
              <div v-else class="rank-icon-badge">
                <PlumpIcon name="trophy" :size="20" color="#ffd54f" />
              </div>
              <div class="rank-head-meta">
                <div class="rank-name">{{ rk.rankName }}</div>
                <div class="rank-freq">
                  {{ rk.category }} · {{ rk.updateFrequency || "每日更新" }}
                </div>
              </div>
              <PlumpIcon name="chevron-right" :size="16" />
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
                点击查看 {{ rk.rankName }} 实时 TOP 100 曲目
              </div>
            </div>
          </div>
        </div>
      </section>
    </template>

    <!-- ==================== 2. 每日推荐全屏双列沉浸视图（含当前播放定位） ==================== -->
    <template v-else-if="store.discoverSubView === 'daily'">
      <div class="subpage-banner">
        <div class="subpage-top-bar">
          <button
            class="tonal-pill-btn"
            @click="store.discoverSubView = 'home'"
          >
            <PlumpIcon name="back" :size="15" />
            <span>返回发现</span>
          </button>

          <div class="subpage-right-tools">
            <button
              class="tonal-pill-btn"
              title="平滑滚动定位到当前正在播放的歌曲"
              @click="locateCurrentPlayingSong"
            >
              <PlumpIcon name="locate" :size="15" />
              <span>定位当前播放</span>
            </button>
            <button
              class="tonal-pill-btn"
              @click="store.loadDailyRecommend()"
            >
              <PlumpIcon name="refresh" :size="15" />
              <span>刷新推荐</span>
            </button>
          </div>
        </div>

        <div class="daily-detail-hero">
          <div class="daily-big-date">
            <span class="big-day">{{ todayDayNum }}</span>
            <span class="big-month">/ {{ todayMonthNum }} 每日专属推荐</span>
          </div>
          <p class="daily-detail-sub">
            循着你的听歌轨迹 · 每日 6:00 更新 {{ store.recommendSongs.length }} 首高解析音源
          </p>
          <button
            v-if="store.recommendSongs.length > 0"
            class="coral-play-all-pill"
            @click="
              store.playSongFromList(
                store.recommendSongs[0],
                store.recommendSongs,
                '每日推荐'
              )
            "
          >
            <PlumpIcon name="play" :size="16" color="#ffffff" />
            <span>播放全部 ({{ store.recommendSongs.length }})</span>
          </button>
        </div>
      </div>

      <!-- 桌面端双列无框软润歌曲卡片网格 -->
      <div class="dual-column-song-grid">
        <div
          v-for="(song, idx) in store.recommendSongs"
          :key="song.hash || song.id"
          class="plump-song-card"
          :class="{
            'track-card-playing': store.currentSong?.hash === song.hash,
          }"
          @click="
            store.playSongFromList(song, store.recommendSongs, '每日推荐')
          "
          @contextmenu="store.openSongContextMenu($event, song)"
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
          />
          <div v-else class="psc-cover-fallback">
            <PlumpIcon name="disc" :size="18" />
          </div>

          <div class="psc-main">
            <div class="psc-title">{{ song.title }}</div>
            <div class="psc-sub">
              <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
              <span v-else-if="song.isLossless" class="mini-badge sq">SQ</span>
              <span v-if="song.isVip" class="mini-badge vip">VIP</span>
              <span>{{ song.artist }}</span>
              <span v-if="song.album" class="psc-album">· {{ song.album }}</span>
            </div>
          </div>

          <div class="psc-actions" @click.stop>
            <button
              class="psc-act-btn"
              title="喜欢"
              @click="store.toggleFavoriteSong(song)"
            >
              <PlumpIcon
                name="heart"
                :size="15"
                :filled="store.isSongLiked(song)"
                :color="store.isSongLiked(song) ? '#ff4d6d' : 'currentColor'"
              />
            </button>
            <button
              class="psc-act-btn"
              title="添加到歌单"
              @click="store.addToPlaylistModalSong = song"
            >
              <PlumpIcon name="playlist-add" :size="15" />
            </button>
            <button
              class="psc-act-btn"
              title="更多操作"
              @click="store.openSongContextMenu($event, song)"
            >
              <PlumpIcon name="more" :size="15" />
            </button>
          </div>

          <div class="psc-dur">{{ formatDuration(song.durationMs) }}</div>
        </div>
      </div>
    </template>

    <!-- ==================== 3. 新歌速递独立页面（按日期折叠时间轴 + 右侧 A-Z 索引） ==================== -->
    <template v-else-if="store.discoverSubView === 'new_songs'">
      <div class="subpage-banner compact">
        <div class="subpage-top-bar">
          <button
            class="tonal-pill-btn"
            @click="store.discoverSubView = 'home'"
          >
            <PlumpIcon name="back" :size="15" />
            <span>返回发现</span>
          </button>

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

          <div class="subpage-right-tools">
            <button class="tonal-pill-btn" @click="toggleAllDates(false)">
              展开全部日期
            </button>
            <button class="tonal-pill-btn" @click="toggleAllDates(true)">
              折叠全部日期
            </button>
            <button
              v-if="store.newSongs.length > 0"
              class="coral-play-all-pill sm"
              @click="
                store.playSongFromList(
                  store.newSongs[0],
                  store.newSongs,
                  '新歌速递'
                )
              "
            >
              <PlumpIcon name="play" :size="14" color="#ffffff" />
              <span>播放全部 ({{ store.newSongs.length }})</span>
            </button>
          </div>
        </div>
      </div>

      <div class="newsong-timeline-layout">
        <!-- 左侧 93%：按精确发行日期分组的时间轴双列网格 -->
        <div class="ns-timeline-main">
          <div
            v-for="group in newSongDateGroups"
            :key="group.date"
            class="ns-date-group"
          >
            <div
              class="ns-date-header"
              @click="collapsedDates[group.date] = !collapsedDates[group.date]"
            >
              <div class="ns-date-left">
                <span class="ns-timeline-dot"></span>
                <span class="ns-date-title">{{ group.date }}</span>
                <span class="ns-date-count">{{ group.songs.length }} 首新歌</span>
              </div>
              <span class="ns-collapse-label">
                {{ collapsedDates[group.date] ? "展开" : "收起" }}
              </span>
            </div>

            <div
              v-if="!collapsedDates[group.date]"
              class="dual-column-song-grid"
            >
              <div
                v-for="(song, idx) in group.songs"
                :key="song.hash || idx"
                class="plump-song-card"
                :class="{
                  'track-card-playing': store.currentSong?.hash === song.hash,
                }"
                @click="
                  store.playSongFromList(song, store.newSongs, '新歌速递')
                "
                @contextmenu="store.openSongContextMenu($event, song)"
              >
                <img
                  v-if="song.coverUrl"
                  :src="song.coverUrl"
                  class="psc-cover"
                />
                <div class="psc-main">
                  <div class="psc-title">{{ song.title }}</div>
                  <div class="psc-sub">
                    <span v-if="song.isHiRes" class="mini-badge hires">Hi-Res</span>
                    <span v-else-if="song.isLossless" class="mini-badge sq">SQ</span>
                    <span>{{ song.artist }}</span>
                    <span v-if="song.album" class="psc-album">
                      · {{ song.album }}
                    </span>
                  </div>
                </div>
                <div class="psc-dur">{{ formatDuration(song.durationMs) }}</div>
              </div>
            </div>
          </div>
        </div>

        <!-- 右侧悬浮 A-Z 首字母快速索引栏 -->
        <aside class="ns-alpha-rail">
          <button
            v-for="ch in alphabetList"
            :key="ch"
            class="alpha-pill"
            :class="{ active: activeAlphaFilter === ch }"
            @click="activeAlphaFilter = ch"
          >
            {{ ch === "ALL" ? "全" : ch }}
          </button>
        </aside>
      </div>
    </template>

    <!-- ==================== 4. 巅峰排行榜详情双栏工作台 ==================== -->
    <template v-else-if="store.discoverSubView === 'rank' && store.activeRank">
      <div class="rank-detail-workspace">
        <!-- 左栏（固定 280px）：当前榜单海报信息 + 同分类榜单快捷切换列 -->
        <aside class="rank-left-sidebar">
          <button
            class="tonal-pill-btn full-w"
            @click="store.discoverSubView = 'home'"
          >
            <PlumpIcon name="back" :size="15" />
            <span>返回发现画廊</span>
          </button>

          <div class="rank-hero-box">
            <img
              v-if="store.activeRank.coverUrl"
              :src="store.activeRank.coverUrl"
              class="rank-big-cover"
            />
            <div v-else class="rank-big-cover-fallback">
              <PlumpIcon name="trophy" :size="38" color="#ffd54f" />
            </div>
            <h2 class="rank-side-title">{{ store.activeRank.rankName }}</h2>
            <p class="rank-side-sub">
              {{ store.activeRank.category }} ·
              {{ store.activeRank.updateFrequency || "每日更新" }}
            </p>
            <button
              v-if="filteredActiveRankSongs.length > 0"
              class="coral-play-all-pill full-w"
              @click="
                store.playSongFromList(
                  filteredActiveRankSongs[0],
                  filteredActiveRankSongs,
                  store.activeRank.rankName
                )
              "
            >
              <PlumpIcon name="play" :size="16" color="#ffffff" />
              <span>播放榜单 ({{ filteredActiveRankSongs.length }})</span>
            </button>
          </div>

          <div class="rank-quick-switcher">
            <div class="rqs-label">快捷切换榜单</div>
            <button
              v-for="rk in store.rankList"
              :key="rk.rankId"
              class="rqs-item"
              :class="{ active: store.activeRank?.rankId === rk.rankId }"
              @click="store.openRankDetail(rk)"
            >
              <span>{{ rk.rankName }}</span>
              <span class="rqs-cat">{{ rk.category }}</span>
            </button>
          </div>
        </aside>

        <!-- 右栏：榜单内搜索过滤 + TOP 3 皇冠曲目与双列流 -->
        <div class="rank-right-tracks">
          <div class="rank-tracks-toolbar">
            <div class="rank-search-box">
              <PlumpIcon name="search" :size="15" color="var(--text-secondary)" />
              <input
                v-model="store.rankSearchKeyword"
                type="text"
                placeholder="搜索榜单内歌曲 / 歌手..."
                class="rank-search-input"
              />
            </div>

            <button
              class="tonal-pill-btn"
              @click="locateCurrentPlayingSong"
            >
              <PlumpIcon name="locate" :size="15" />
              <span>定位当前播放</span>
            </button>
          </div>

          <div v-if="store.isLoadingRankSongs" class="loading-box">
            正在拉取榜单实时曲目与概念版无损音源规格...
          </div>

          <div v-else class="dual-column-song-grid">
            <div
              v-for="(song, idx) in filteredActiveRankSongs"
              :key="song.hash || song.id"
              class="plump-song-card"
              :class="{
                'track-card-playing': store.currentSong?.hash === song.hash,
                'top-three-card': idx < 3,
              }"
              @click="
                store.playSongFromList(
                  song,
                  filteredActiveRankSongs,
                  store.activeRank?.rankName
                )
              "
              @contextmenu="store.openSongContextMenu($event, song)"
            >
              <div class="psc-index" :class="`rank-top-${idx + 1}`">
                {{ String(idx + 1).padStart(2, "0") }}
              </div>
              <img
                v-if="song.coverUrl"
                :src="song.coverUrl"
                class="psc-cover"
                loading="lazy"
              />
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
        </div>
      </div>
    </template>

    <!-- 全部榜单分类宫格选择器弹窗 (RankGridPickerSheet) -->
    <transition name="fade">
      <div
        v-if="store.isRankGridModalOpen"
        class="modal-backdrop"
        @click.self="store.isRankGridModalOpen = false"
      >
        <div class="vip-modal-card rank-grid-modal">
          <div class="modal-header">
            <div class="vip-title-row">
              <div class="vip-icon-orb">
                <PlumpIcon name="trophy" :size="19" color="#ffd54f" />
              </div>
              <h3>巅峰榜单全集宫格</h3>
            </div>
            <button
              class="icon-close-btn"
              @click="store.isRankGridModalOpen = false"
            >
              <PlumpIcon name="close" :size="15" />
            </button>
          </div>

          <div class="pill-tabs mb-14">
            <button
              v-for="cat in rankCategories"
              :key="cat"
              class="pill-tab-btn"
              :class="{ active: store.selectedRankCategory === cat }"
              @click="store.selectedRankCategory = cat"
            >
              {{ cat }}
            </button>
          </div>

          <div class="rank-picker-grid">
            <button
              v-for="rk in filteredRankGrid"
              :key="rk.rankId"
              class="rank-picker-cell"
              @click="store.openRankDetail(rk)"
            >
              <img
                v-if="rk.coverUrl"
                :src="rk.coverUrl"
                class="rpc-cover"
              />
              <div v-else class="rpc-fallback">
                <PlumpIcon name="trophy" :size="20" color="#ffd54f" />
              </div>
              <div class="rpc-meta">
                <div class="rpc-name">{{ rk.rankName }}</div>
                <div class="rpc-sub">
                  {{ rk.category }} · {{ rk.updateFrequency || "每日更新" }}
                </div>
              </div>
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>
