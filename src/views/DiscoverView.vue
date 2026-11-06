<template>
  <div class="discover-page">
    <!-- ==================== 顶部沉浸双雄舞台（固定高度 168px 不变，CSS Grid 1fr/0fr 丝滑横向展开，点击卡片即可展开/返回） ==================== -->
    <div
      v-if="
        store.discoverSubView === 'home' ||
        store.discoverSubView === 'daily' ||
        store.discoverSubView === 'fm_detail'
      "
      class="discover-hero-flow-container"
    >
      <!-- 日推 & 猜你喜欢 横向可展开双卡舞台（展开时绝不增大高度） -->
      <div
        class="bento-hero-stage"
        :class="{
          'mode-daily-expanded': isDailyExpandedMode,
          'mode-fm-expanded': isFmExpandedMode,
        }"
      >
        <!-- 左卡：每日推荐 (点击展开，展开态点击卡片任意区域即可返回发现首页) -->
        <div
          class="hero-immersive-card daily-immersive-card"
          :class="{
            'is-expanded-banner': isDailyExpandedMode,
            'is-collapsed-left': isFmExpandedMode,
          }"
          @click="handleDailyCardClick"
        >
          <div class="hero-card-inner">
            <div class="hero-card-main-left">
              <!-- 三封面交错立体叠层 -->
              <div class="staggered-three-covers" :class="{ enlarged: isDailyExpandedMode }">
                <div class="stagger-cover-layer layer-back">
                  <img
                    v-if="dailyCoverTriple[2]"
                    :src="dailyCoverTriple[2]"
                    draggable="false"
                    alt="daily-back"
                  />
                </div>
                <div class="stagger-cover-layer layer-mid">
                  <img
                    v-if="dailyCoverTriple[1]"
                    :src="dailyCoverTriple[1]"
                    draggable="false"
                    alt="daily-mid"
                  />
                </div>
                <div class="stagger-cover-layer layer-front">
                  <img
                    v-if="dailyCoverTriple[0]"
                    :src="dailyCoverTriple[0]"
                    draggable="false"
                    alt="daily-front"
                  />
                  <div class="stagger-date-badge">
                    <span class="stagger-month">{{ todayMonthEng }}</span>
                    <span class="stagger-day">{{ todayDayNum }}</span>
                  </div>
                </div>
              </div>

              <!-- 精简大气的日推信息区（已剔除无用口号文本） -->
              <div class="hero-card-copy">
                <div class="hero-eyebrow-line">
                  <span>DAILY · {{ todayDateLabel }}</span>
                </div>

                <div class="hero-title-row">
                  <h2 class="hero-main-title">每日推荐</h2>
                  <span class="hero-count-pill">{{ store.recommendSongs.length || 30 }} 首</span>
                </div>

                <!-- 双层平滑切换副信息：常态展示两首代表作，展开态展示无损统计 -->
                <div class="hero-sub-stack">
                  <div
                    class="hero-sub-layer preview-layer"
                    :class="{ 'layer-hidden': isDailyExpandedMode }"
                  >
                    <div
                      v-for="s in store.recommendSongs.slice(0, 2)"
                      :key="s.hash"
                      class="hero-preview-line"
                    >
                      <span class="preview-song">{{ s.title }}</span>
                      <span class="preview-artist">— {{ s.artist }}</span>
                    </div>
                  </div>

                  <div
                    class="hero-sub-layer stats-layer"
                    :class="{ 'layer-active': isDailyExpandedMode }"
                  >
                    <span class="h-stat">今日定制 {{ store.recommendSongs.length || 30 }} 首</span>
                    <template v-if="dailyHiResCount > 0">
                      <span class="h-dot">·</span>
                      <span class="h-stat highlight">{{ dailyHiResCount }} 首无损 / Hi-Res</span>
                    </template>
                    <span class="h-dot">·</span>
                    <span class="h-stat muted">点击卡片返回</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- 展开态平滑浮现的精选日推曲目画廊 -->
            <div
              class="hero-expanded-gallery"
              :class="{ 'is-visible': isDailyExpandedMode }"
              @click.stop
            >
              <div
                v-for="song in store.recommendSongs.slice(0, 3)"
                :key="song.hash"
                class="lib-gallery-track-pill"
                :class="{ playing: store.currentSong?.hash === song.hash }"
                @click.stop="store.playSongFromList(song, store.recommendSongs)"
              >
                <img :src="song.coverUrl" class="lgtp-cover" draggable="false" alt="" />
                <div class="lgtp-info">
                  <div class="lgtp-title">{{ song.title }}</div>
                  <div class="lgtp-artist">{{ song.artist }}</div>
                </div>
              </div>
            </div>

            <!-- 右侧控制区 -->
            <div class="hero-card-actions" @click.stop>
              <div
                class="hero-expanded-extra-btns is-visible"
              >
                <button
                  class="hero-circle-icon-btn"
                  :disabled="store.isLoadingRecommend"
                  title="刷新每日推荐"
                  @click.stop="store.loadDailyRecommend(true)"
                >
                  <PlumpIcon
                    name="refresh"
                    :size="18"
                    color="#fff"
                    :class="{ 'is-spinning': store.isLoadingRecommend }"
                  />
                </button>
                <button
                  v-if="isCurrentSongInList(store.recommendSongs)"
                  class="hero-circle-icon-btn"
                  title="定位当前播放歌曲"
                  @click.stop="scrollToCurrentPlayingSong"
                >
                  <PlumpIcon name="locate" :size="18" color="#fff" />
                </button>
              </div>

              <button
                class="hero-stereo-play-fab"
                title="立即播放今日全部推荐"
                @click.stop="playAllDaily"
              >
                <PlumpIcon name="play" :size="20" color="#0f3878" />
              </button>
            </div>
          </div>
        </div>

        <!-- 右卡：猜你喜欢 · 私人漫游 (重构视觉与横向播控，点击展开，展开态点击卡片即可返回) -->
        <div
          class="hero-immersive-card fm-immersive-card"
          :class="{
            'is-expanded-banner': isFmExpandedMode,
            'is-collapsed-right': isDailyExpandedMode,
          }"
          @click="handleFmCardClick"
        >
          <div class="hero-card-inner">
            <div class="hero-card-main-left">
              <!-- 双层漫游封面叠放 -->
              <div class="fm-stereo-visual" :class="{ enlarged: isFmExpandedMode }">
                <div v-if="store.personalFmSongs[2]?.coverUrl" class="fm-third-cover">
                  <img :src="store.personalFmSongs[2].coverUrl" draggable="false" alt="" />
                </div>
                <div v-if="store.nextFmSong?.coverUrl" class="fm-rear-cover">
                  <img :src="store.nextFmSong.coverUrl" draggable="false" alt="next-fm" />
                </div>
                <div class="fm-front-cover">
                  <img
                    v-if="store.currentFmSong?.coverUrl"
                    :src="store.currentFmSong.coverUrl"
                    draggable="false"
                    alt="current-fm"
                  />
                  <div v-else class="fm-cover-fallback">
                    <PlumpIcon name="radio" :size="28" color="#fff" />
                  </div>
                  <div v-if="isFmPlaying" class="fm-playing-wave">
                    <span></span><span></span><span></span><span></span>
                  </div>
                  <span v-else class="fm-corner-badge">
                    <PlumpIcon name="radio" :size="11" color="#fff" />
                  </span>
                </div>
              </div>

              <!-- 猜你喜欢重构文案区 -->
              <div class="hero-card-copy">
                <div class="hero-eyebrow-line fm-eyebrow">
                  <span>PERSONAL FM · 私人声学漫游</span>
                </div>

                <div class="hero-title-row">
                  <h2 class="hero-main-title">猜你喜欢</h2>
                  <span class="hero-count-pill fm-pill">
                    {{ isFmPlaying ? "漫游中" : "随心听" }}
                  </span>
                </div>

                <div class="hero-sub-stack">
                  <div
                    class="hero-sub-layer preview-layer"
                    :class="{ 'layer-hidden': isFmExpandedMode }"
                  >
                    <div v-if="store.currentFmSong" class="hero-preview-line">
                      <span class="preview-song">{{ store.currentFmSong.title }}</span>
                      <span class="preview-artist">— {{ store.currentFmSong.artist }}</span>
                    </div>
                    <div v-if="store.nextFmSong" class="hero-preview-line secondary">
                      <span class="preview-artist">下一首：{{ store.nextFmSong.title }} — {{ store.nextFmSong.artist }}</span>
                    </div>
                  </div>

                  <div
                    class="hero-sub-layer stats-layer"
                    :class="{ 'layer-active': isFmExpandedMode }"
                  >
                    <span class="h-stat">漫游候选 {{ store.personalFmSongs.length }} 首</span>
                    <span class="h-dot">·</span>
                    <span class="h-stat highlight">无损私人电台</span>
                    <span class="h-dot">·</span>
                    <span class="h-stat muted">点击卡片返回</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- 展开态平滑浮现的漫游曲目画廊 -->
            <div
              class="hero-expanded-gallery"
              :class="{ 'is-visible': isFmExpandedMode }"
              @click.stop
            >
              <div
                v-for="song in store.personalFmSongs.slice(0, 3)"
                :key="song.hash"
                class="lib-gallery-track-pill"
                :class="{ playing: store.currentSong?.hash === song.hash }"
                @click.stop="store.playSongFromList(song, store.personalFmSongs)"
              >
                <img :src="song.coverUrl" class="lgtp-cover" draggable="false" alt="" />
                <div class="lgtp-info">
                  <div class="lgtp-title">{{ song.title }}</div>
                  <div class="lgtp-artist">{{ song.artist }}</div>
                </div>
              </div>
            </div>

            <!-- 右侧横向一体化播控簇 -->
            <div class="hero-card-actions" @click.stop>
              <div
                class="hero-expanded-extra-btns is-visible"
              >
                <button
                  class="lib-card-glass-btn"
                  :disabled="store.isLoadingFm"
                  title="为漫游池追加 20 首新曲"
                  @click.stop="store.loadPersonalFm(true)"
                >
                  <PlumpIcon name="plus" :size="14" color="#fff" />
                  <span>+20首</span>
                </button>
                <button
                  class="hero-circle-icon-btn"
                  :disabled="store.isLoadingFm"
                  title="换一批漫游推荐"
                  @click.stop="store.loadPersonalFm(false)"
                >
                  <PlumpIcon
                    name="refresh"
                    :size="17"
                    color="#fff"
                    :class="{ 'is-spinning': store.isLoadingFm }"
                  />
                </button>
              </div>

              <div class="fm-horizontal-controls">
                <button
                  v-if="store.currentFmSong"
                  class="hero-circle-icon-btn"
                  :class="{ liked: store.isSongLiked(store.currentFmSong.hash) }"
                  title="喜欢当前漫游歌曲"
                  @click.stop="store.toggleFavoriteSong(store.currentFmSong)"
                >
                  <PlumpIcon
                    :name="store.isSongLiked(store.currentFmSong.hash) ? 'heart-filled' : 'heart'"
                    :size="17"
                    :color="store.isSongLiked(store.currentFmSong.hash) ? '#ff8a9b' : '#fff'"
                  />
                </button>
                <button
                  class="hero-circle-icon-btn"
                  title="跳至下一首漫游"
                  @click.stop="store.nextPersonalFmTrack(true)"
                >
                  <PlumpIcon name="next" :size="17" color="#fff" />
                </button>
                <button
                  class="hero-stereo-play-fab fm-fab"
                  :title="isFmPlaying ? '暂停漫游' : '立即播放猜你喜欢'"
                  @click.stop="toggleFmPlay"
                >
                  <PlumpIcon :name="isFmPlaying ? 'pause' : 'play'" :size="20" color="#881d5f" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ==================== 点击日推或猜你喜欢后：自然浮现的独立无框歌曲列表 ==================== -->
      <transition name="hero-songlist-enter">
        <div
          v-if="store.discoverSubView === 'daily' || store.discoverSubView === 'fm_detail'"
          class="hero-expanded-song-stage"
        >
          <div class="frameless-sublist-toolbar">
            <div class="frameless-sublist-info">
              <span class="sublist-badge">
                {{ store.discoverSubView === "daily" ? "DAILY RECOMMEND" : "PERSONAL ROAMING" }}
              </span>
              <span class="sublist-desc">
                共 {{ activeHeroSongs.length }} 首精选曲目 · 点击顶部卡片即可收起返回发现页
              </span>
            </div>
            <div style="display: flex; align-items: center; gap: 10px">
              <button
                v-if="isCurrentSongInList(activeHeroSongs)"
                class="frameless-hover-btn"
                title="定位当前播放歌曲"
                @click="scrollToCurrentPlayingSong"
              >
                <PlumpIcon name="locate" :size="15" />
                <span>定位在播</span>
              </button>
              <div class="frameless-sublist-search">
                <PlumpIcon name="search" :size="15" />
                <input
                  v-model="heroSongFilter"
                  type="text"
                  placeholder="在当前列表中筛选歌名 / 歌手 / 专辑..."
                />
                <button v-if="heroSongFilter" class="clear-inline-btn" @click="heroSongFilter = ''">
                  ×
                </button>
              </div>
            </div>
          </div>

          <div class="frameless-songs-grid-2col">
            <div
              v-for="(song, index) in filteredHeroSongs"
              :key="song.hash + '_' + index"
              :data-song-hash="song.hash"
              class="frameless-song-row"
              :class="{ active: store.currentSong?.hash === song.hash }"
              @click="store.playSongFromList(song, activeHeroSongs)"
              @contextmenu.prevent="store.openSongContextMenu($event, song, activeHeroSongs)"
            >
              <span class="fs-index">
                <template v-if="store.currentSong?.hash === song.hash && store.isPlaying">
                  <span class="mini-eq-bars"><i></i><i></i><i></i></span>
                </template>
                <template v-else>
                  {{ String(index + 1).padStart(2, "0") }}
                </template>
              </span>

              <div class="fs-cover-wrap">
                <img :src="song.coverUrl" class="fs-cover" loading="lazy" alt="" />
                <div class="fs-cover-play">
                  <PlumpIcon
                    :name="store.currentSong?.hash === song.hash && store.isPlaying ? 'pause' : 'play'"
                    :size="15"
                    color="#fff"
                  />
                </div>
              </div>

              <div class="fs-meta">
                <div class="fs-title-line">
                  <span class="fs-title">{{ song.title }}</span>
                  <span v-if="song.hasHiRes || song.isHiRes" class="fs-badge hires">Hi-Res</span>
                  <span v-else-if="song.hasSq || song.isLossless" class="fs-badge sq">SQ</span>
                  <span v-if="song.isVip" class="fs-badge vip">VIP</span>
                </div>
                <div class="fs-sub-line">
                  <button
                    class="inline-meta-btn"
                    title="点击进入歌手独立主页"
                    @click.stop="store.navigateToSongArtist(song)"
                  >
                    {{ song.artist }}
                  </button>
                  <button
                    v-if="song.album"
                    class="inline-meta-btn fs-album"
                    title="点击进入专辑独立页面"
                    @click.stop="store.navigateToSongAlbum(song)"
                  >
                    · {{ song.album }}
                  </button>
                </div>
              </div>

              <div class="fs-actions" @click.stop>
                <button
                  class="fs-icon-btn"
                  :class="{ liked: store.isSongLiked(song.hash) }"
                  title="喜欢"
                  @click="store.toggleFavoriteSong(song)"
                >
                  <PlumpIcon
                    :name="store.isSongLiked(song.hash) ? 'heart-filled' : 'heart'"
                    :size="16"
                  />
                </button>
                <button
                  class="fs-icon-btn"
                  title="添加到歌单"
                  @click="store.addToPlaylistModalSong = song"
                >
                  <PlumpIcon name="plus" :size="16" />
                </button>
                <button
                  class="fs-icon-btn"
                  title="更多操作"
                  @click="store.openSongContextMenu($event, song, activeHeroSongs)"
                >
                  <PlumpIcon name="more" :size="16" />
                </button>
              </div>

              <span class="fs-duration">{{ formatDuration(song) }}</span>
            </div>
          </div>
        </div>
      </transition>

      <!-- ==================== 发现页下半部分主体内容 ==================== -->
      <div
        v-show="store.discoverSubView === 'home'"
        class="discover-home-lower"
        :class="{
          'lower-blur-exit': store.discoverHeroExpanding !== 'none',
        }"
      >
        <!-- 1. 新歌速递 -->
        <section class="discover-block">
          <div class="block-header newsong-header">
            <div class="block-title-with-tabs">
              <h3 class="block-title">新歌速递</h3>
              <span class="block-sub-date">/ {{ newSongDateRangeLabel }}</span>

              <div class="app-sliding-tabs newsong-sliding-tabs">
                <button
                  v-for="area in newSongAreas"
                  :key="area.id"
                  class="app-sliding-tab-btn"
                  :class="{ active: store.newSongPeriod === area.id }"
                  @click="store.loadNewSongExpress(area.id)"
                >
                  <span class="tab-label">{{ area.label }}</span>
                  <span
                    class="tab-bottom-indicator"
                    :style="{
                      backgroundColor: area.color,
                      opacity: store.newSongPeriod === area.id ? 1 : 0,
                      transform: store.newSongPeriod === area.id ? 'scaleX(1)' : 'scaleX(0.2)',
                    }"
                  ></span>
                </button>
              </div>
            </div>

            <div class="block-right-frameless-actions">
              <button
                class="frameless-icon-square-btn discover-refresh-icon-btn"
                :disabled="store.isLoadingNewSongs"
                title="刷新全部新歌分类"
                @click="store.refreshAllNewSongTags(true)"
              >
                <PlumpIcon
                  name="refresh"
                  :size="20"
                  :class="{ 'is-spinning': store.isLoadingNewSongs }"
                />
              </button>
              <button
                class="frameless-hover-btn"
                title="播放当前分类全部新歌"
                @click="playAllNewSongs"
              >
                <PlumpIcon name="play" :size="16" />
                <span>全播</span>
              </button>
              <button
                class="frameless-hover-btn"
                title="查看全部新歌速递"
                @click="openNewSongsSubpage"
              >
                <span>查看全部</span>
                <PlumpIcon name="chevron-right" :size="15" />
              </button>
            </div>
          </div>

          <div class="newsong-frameless-grid-3x4">
            <div
              v-for="(song, idx) in sortedTop12NewSongs"
              :key="song.hash + '_' + idx"
              class="newsong-frameless-item"
              :class="{ active: store.currentSong?.hash === song.hash }"
              @click="store.playSongFromList(song, sortedNewSongsAll)"
              @contextmenu.prevent="store.openSongContextMenu($event, song, sortedNewSongsAll)"
            >
              <span class="ns-col-rank">
                <template v-if="store.currentSong?.hash === song.hash && store.isPlaying">
                  <span class="mini-eq-bars"><i></i><i></i><i></i></span>
                </template>
                <template v-else>
                  {{ String(idx + 1).padStart(2, "0") }}
                </template>
              </span>

              <div class="ns-cover-box">
                <img :src="song.coverUrl" class="ns-cover-img" loading="lazy" alt="" />
                <div class="ns-cover-overlay">
                  <PlumpIcon
                    :name="store.currentSong?.hash === song.hash && store.isPlaying ? 'pause' : 'play'"
                    :size="15"
                    color="#fff"
                  />
                </div>
              </div>

              <div class="ns-meta-col">
                <div class="ns-song-title-row">
                  <span class="ns-song-title">{{ song.title }}</span>
                  <span v-if="song.hasHiRes || song.isHiRes" class="fs-badge hires">Hi-Res</span>
                  <span v-else-if="song.hasSq || song.isLossless" class="fs-badge sq">SQ</span>
                </div>
                <div class="ns-song-sub-row">
                  <button
                    class="inline-meta-btn ns-song-artist"
                    title="点击进入歌手主页"
                    @click.stop="store.navigateToSongArtist(song)"
                  >
                    {{ song.artist }}
                  </button>
                  <span v-if="song.publishDate" class="ns-song-date">
                    · {{ formatShortPublishDate(song.publishDate) }}
                  </span>
                </div>
              </div>

              <button
                class="ns-like-btn"
                :class="{ liked: store.isSongLiked(song.hash) }"
                title="收藏到我喜欢"
                @click.stop="store.toggleFavoriteSong(song)"
              >
                <PlumpIcon
                  :name="store.isSongLiked(song.hash) ? 'heart-filled' : 'heart'"
                  :size="15"
                />
              </button>
            </div>
          </div>
        </section>

        <!-- 2. 巅峰排行榜（重构概念版多彩渐变榜单卡 + 顶部分类快选 + 规范化前三名预览） -->
        <section class="discover-block">
          <div class="block-header rank-home-header">
            <div class="block-title-with-tabs">
              <h3 class="block-title">巅峰排行榜</h3>
              <span class="block-sub-date">/ 官方权威潮流风向标</span>

              <div class="app-sliding-tabs rank-sliding-tabs">
                <button
                  v-for="cat in rankHomeCategoryTabs"
                  :key="cat.id"
                  type="button"
                  class="app-sliding-tab-btn"
                  :class="{ active: homeRankCategory === cat.id }"
                  @click="homeRankCategory = cat.id"
                >
                  <span class="tab-label">{{ cat.label }}</span>
                  <span
                    class="tab-bottom-indicator"
                    :style="{
                      backgroundColor: cat.color,
                      opacity: homeRankCategory === cat.id ? 1 : 0,
                      transform: homeRankCategory === cat.id ? 'scaleX(1)' : 'scaleX(0.2)',
                    }"
                  ></span>
                </button>
              </div>
            </div>
            <div class="block-right-frameless-actions">
              <button
                class="frameless-icon-square-btn discover-refresh-icon-btn"
                :disabled="store.isLoadingRanks"
                title="刷新榜单"
                @click="store.loadRankList(true)"
              >
                <PlumpIcon
                  name="refresh"
                  :size="20"
                  :class="{ 'is-spinning': store.isLoadingRanks }"
                />
              </button>
              <button
                class="frameless-hover-btn"
                @click="openAllRanksOverview"
              >
                <span>全部榜单 ({{ store.rankList.length || 57 }})</span>
                <PlumpIcon name="chevron-right" :size="15" />
              </button>
            </div>
          </div>

          <!-- 巅峰榜单概念版卡片矩阵（2行 × 4列 = 8张卡片） -->
          <div class="rank-double-row-grid">
            <div
              v-for="(rk, rIdx) in homeDoubleRowRanks"
              :key="rk.rankId"
              class="rank-concept-card"
              :class="getRankThemeClass(rk, rIdx)"
              @click="store.openRankDetail(rk)"
            >
              <div class="rcc-top-header">
                <div class="rcc-cover-wrap">
                  <img
                    v-if="rk.coverUrl"
                    :src="rk.coverUrl"
                    class="rcc-cover-img"
                    loading="lazy"
                    draggable="false"
                    alt=""
                  />
                  <div v-else class="rcc-cover-fallback">
                    <PlumpIcon name="trophy" :size="24" color="#fff" />
                  </div>
                </div>

                <div class="rcc-head-meta">
                  <h4 class="rcc-title">{{ rk.name || rk.rankName }}</h4>
                  <div class="rcc-sub-line">
                    <span class="rcc-accent-dot"></span>
                    <span class="rcc-sub-text">
                      {{ rk.category || "官方热榜" }} · {{ rk.updateFrequency || "每天更新" }}
                    </span>
                  </div>
                </div>

                <button
                  type="button"
                  class="rcc-play-circle-btn"
                  title="直接播放该榜单"
                  @click.stop="store.playRankDirectly(rk)"
                >
                  <PlumpIcon name="play" :size="15" color="#fff" />
                </button>
              </div>

              <div class="rcc-divider"></div>

              <div class="rcc-tracks-list">
                <div
                  v-for="(item, lIdx) in getRankPreviewStructured(rk)"
                  :key="lIdx"
                  class="rcc-track-row"
                >
                  <span class="rcc-rank-num" :class="'top-' + (lIdx + 1)">
                    {{ lIdx + 1 }}
                  </span>
                  <div class="rcc-track-main">
                    <span class="rcc-song-name">{{ item.title }}</span>
                    <span v-if="item.artist" class="rcc-song-artist"> - {{ item.artist }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- 3. 精选场景电台 -->
        <section class="discover-block">
          <div class="block-header">
            <div class="block-title-with-tabs">
              <h3 class="block-title">精选场景电台</h3>
              <span class="block-sub-date">/ 概念版全天候声学漫游频率</span>
            </div>
            <div class="block-right-frameless-actions">
              <button
                class="frameless-hover-btn"
                @click="openRadioPlaza"
              >
                <PlumpIcon name="radio" :size="16" />
                <span>电台广场</span>
                <PlumpIcon name="chevron-right" :size="15" />
              </button>
            </div>
          </div>

          <div class="radio-single-row-corridor">
            <div
              v-for="(ch, idx) in singleRowFmChannels"
              :key="ch.id"
              class="radio-cover-card"
              :class="{ active: store.activeFmChannel?.id === ch.id }"
              @click="playFmChannelFromHome(ch)"
            >
              <div class="rc-cover-stage">
                <img
                  v-if="getStationCover(ch, idx)"
                  :src="getStationCover(ch, idx)"
                  class="rc-cover-img"
                  loading="lazy"
                  alt=""
                />
                <div v-else class="rc-cover-fallback">
                  <PlumpIcon name="radio" :size="28" color="#fff" />
                </div>
                <div class="rc-cover-shade"></div>
                <span class="rc-freq-tag">{{ ch.freqLabel }}</span>
                <button
                  class="rc-play-btn"
                  title="立即调频播放"
                  @click.stop="playFmChannelFromHome(ch)"
                >
                  <PlumpIcon
                    :name="store.activeFmChannel?.id === ch.id && store.isPlaying ? 'pause' : 'play'"
                    :size="15"
                    color="#fff"
                  />
                </button>
              </div>
              <div class="rc-meta">
                <div class="rc-name">{{ ch.name }}</div>
                <div class="rc-desc">{{ ch.description }}</div>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>

    <!-- ==================== 二级页面 A：新歌速递 · 极简时间轴折叠视图 ==================== -->
    <div v-else-if="store.discoverSubView === 'new_songs'" class="discover-subpage frameless-subpage ns-standalone-page">
      <div class="subpage-frameless-header ns-clean-header">
        <div class="sfh-left">
          <button
            type="button"
            class="frameless-hover-btn"
            @click="store.discoverSubView = 'home'"
          >
            <PlumpIcon name="back" :size="17" />
            <span>返回</span>
          </button>

          <div class="ns-header-title-group">
            <h2 class="sfh-title">新歌速递</h2>
            <span class="ns-header-date-pill">{{ newSongDateRangeLabel }}</span>
          </div>

          <div class="app-sliding-tabs">
            <button
              v-for="area in newSongAreas"
              :key="area.id"
              type="button"
              class="app-sliding-tab-btn"
              :class="{ active: store.newSongPeriod === area.id }"
              @click="store.loadNewSongExpress(area.id)"
            >
              <span class="tab-label">{{ area.label }}</span>
              <span
                class="tab-bottom-indicator"
                :style="{
                  backgroundColor: area.color,
                  opacity: store.newSongPeriod === area.id ? 1 : 0,
                  transform: store.newSongPeriod === area.id ? 'scaleX(1)' : 'scaleX(0.2)',
                }"
              ></span>
            </button>
          </div>
        </div>

        <div class="sfh-right">
          <button
            v-if="groupedNewSongsByDate.length > 1"
            type="button"
            class="frameless-hover-btn"
            @click="toggleAllDateGroups"
          >
            <span>{{ isAllDateGroupsCollapsed ? "全部展开" : "全部收起" }}</span>
          </button>
          <button
            type="button"
            class="frameless-icon-square-btn discover-refresh-icon-btn"
            :disabled="store.isLoadingNewSongs"
            title="刷新全部新歌分类"
            @click="store.refreshAllNewSongTags(true)"
          >
            <PlumpIcon
              name="refresh"
              :size="20"
              :class="{ 'is-spinning': store.isLoadingNewSongs }"
            />
          </button>
          <button
            type="button"
            class="frameless-hover-btn primary-tint"
            :disabled="sortedNewSongsAll.length === 0"
            @click="playAllNewSongs"
          >
            <PlumpIcon name="play" :size="16" />
            <span>播放全部 ({{ sortedNewSongsAll.length }})</span>
          </button>
        </div>
      </div>

      <div v-if="store.isLoadingNewSongs && groupedNewSongsByDate.length === 0" class="ns-empty-hint">
        <PlumpIcon name="refresh" :size="20" class="is-spinning" />
        <span>正在同步最近一周发行的歌曲...</span>
      </div>

      <div v-else-if="groupedNewSongsByDate.length === 0" class="ns-empty-hint">
        <span>暂无最近一周发行的曲目，可点击右上角刷新重试</span>
      </div>

      <div v-else class="ns-frameless-timeline">
        <section
          v-for="group in groupedNewSongsByDate"
          :key="group.dateKey"
          class="ns-frameless-date-group"
          :class="{ 'is-collapsed': isDateGroupCollapsed(group.dateKey) }"
        >
          <div
            class="ns-collapsible-date-bar"
            role="button"
            tabindex="0"
            :aria-expanded="!isDateGroupCollapsed(group.dateKey)"
            @click="toggleDateGroup(group.dateKey)"
            @keydown.enter.prevent="toggleDateGroup(group.dateKey)"
            @keydown.space.prevent="toggleDateGroup(group.dateKey)"
          >
            <div class="ns-date-title-wrap">
              <span class="ns-date-dot"></span>
              <span class="ns-date-title">{{ group.displayLabel }}</span>
              <span class="ns-date-count">{{ group.songs.length }} 首</span>
            </div>

            <div class="ns-date-right-tools">
              <button
                type="button"
                class="ns-date-play-mini"
                title="播放该日全部新歌"
                @click.stop="playDateGroupSongs(group.songs)"
              >
                <PlumpIcon name="play" :size="13" />
                <span>播放</span>
              </button>
              <span
                class="ns-collapse-chevron"
                :class="{ collapsed: isDateGroupCollapsed(group.dateKey) }"
              >
                <PlumpIcon name="chevron-down" :size="16" />
              </span>
            </div>
          </div>

          <div
            class="ns-date-collapse-body"
            :class="{ collapsed: isDateGroupCollapsed(group.dateKey) }"
            :inert="isDateGroupCollapsed(group.dateKey) ? true : undefined"
          >
            <div class="ns-date-collapse-inner">
              <div class="frameless-songs-grid-2col">
                <div
                  v-for="(song, sIdx) in group.songs"
                  :key="song.hash + '_' + sIdx"
                  class="frameless-song-row"
                  :class="{ active: store.currentSong?.hash === song.hash }"
                  @click="store.playSongFromList(song, sortedNewSongsAll)"
                  @contextmenu.prevent="store.openSongContextMenu($event, song, sortedNewSongsAll)"
                >
                  <span class="fs-index">
                    <template v-if="store.currentSong?.hash === song.hash && store.isPlaying">
                      <span class="mini-eq-bars"><i></i><i></i><i></i></span>
                    </template>
                    <template v-else>
                      {{ sIdx + 1 }}
                    </template>
                  </span>
                  <div class="fs-cover-wrap">
                    <img :src="song.coverUrl" class="fs-cover" loading="lazy" alt="" />
                    <div class="fs-cover-play">
                      <PlumpIcon
                        :name="store.currentSong?.hash === song.hash && store.isPlaying ? 'pause' : 'play'"
                        :size="15"
                        color="#fff"
                      />
                    </div>
                  </div>
                  <div class="fs-meta">
                    <div class="fs-title-line">
                      <span class="fs-title">{{ song.title }}</span>
                      <span v-if="song.hasHiRes || song.isHiRes" class="fs-badge hires">Hi-Res</span>
                      <span v-else-if="song.hasSq || song.isLossless || song.isSq" class="fs-badge sq">SQ</span>
                      <span v-if="song.isVip" class="fs-badge vip">VIP</span>
                    </div>
                    <div class="fs-sub-line">
                      <button
                        type="button"
                        class="inline-meta-btn"
                        title="点击进入歌手独立主页"
                        @click.stop="store.navigateToSongArtist(song)"
                      >
                        {{ song.artist }}
                      </button>
                      <button
                        v-if="song.album"
                        type="button"
                        class="inline-meta-btn fs-album"
                        title="点击进入专辑独立页面"
                        @click.stop="store.navigateToSongAlbum(song)"
                      >
                        · {{ song.album }}
                      </button>
                    </div>
                  </div>
                  <div class="fs-actions" @click.stop>
                    <button
                      type="button"
                      class="fs-icon-btn"
                      :class="{ liked: store.isSongLiked(song.hash) }"
                      title="喜欢"
                      @click="store.toggleFavoriteSong(song)"
                    >
                      <PlumpIcon
                        :name="store.isSongLiked(song.hash) ? 'heart-filled' : 'heart'"
                        :size="16"
                      />
                    </button>
                    <button
                      type="button"
                      class="fs-icon-btn"
                      title="查看评论"
                      @click="store.openCommentsModal(song)"
                    >
                      <PlumpIcon name="comment" :size="15" />
                    </button>
                    <button
                      type="button"
                      class="fs-icon-btn"
                      title="添加到歌单"
                      @click="store.addToPlaylistModalSong = song"
                    >
                      <PlumpIcon name="plus" :size="16" />
                    </button>
                  </div>
                  <span class="fs-duration">{{ formatDuration(song) }}</span>
                </div>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>

    <!-- ==================== 二级页面 B：巅峰排行榜独立全宽详情页（去除冗余左侧切榜栏 · 开放式大视野头部 + 全量500首分段导航） ==================== -->
    <div v-else-if="store.discoverSubView === 'rank'" class="discover-subpage frameless-subpage rank-standalone-page">
      <!-- 顶部无框导航与工具条（预留右上角窗口控制胶囊安全边距） -->
      <div class="subpage-frameless-header rank-open-topbar">
        <div class="sfh-left">
          <button
            type="button"
            class="frameless-hover-btn"
            @click="store.discoverSubView = 'home'"
          >
            <PlumpIcon name="back" :size="17" />
            <span>返回发现</span>
          </button>

          <div class="rank-topbar-breadcrumb">
            <span class="rtb-cat-tag" :class="getRankThemeClass(store.activeRank)">
              {{ store.activeRank?.category || "官方热榜" }}
            </span>
            <span class="rtb-divider">/</span>
            <span class="rtb-name">{{ store.activeRank?.name || store.activeRank?.rankName || "巅峰排行榜" }}</span>
          </div>
        </div>

        <div class="sfh-right">
          <button
            type="button"
            class="frameless-icon-square-btn discover-refresh-icon-btn"
            :disabled="store.isLoadingRankSongs || store.isLoadingMoreRankSongs"
            title="刷新当前榜单完整曲库"
            @click="store.refreshActiveRank()"
          >
            <PlumpIcon
              name="refresh"
              :size="19"
              :class="{ 'is-spinning': store.isLoadingRankSongs || store.isLoadingMoreRankSongs }"
            />
          </button>

          <button
            v-if="isCurrentSongInList(store.activeRankSongs)"
            type="button"
            class="frameless-hover-btn"
            title="定位当前播放歌曲"
            @click="scrollToCurrentPlayingRankSong"
          >
            <PlumpIcon name="locate" :size="15" />
            <span>定位在播</span>
          </button>

          <button
            type="button"
            class="frameless-hover-btn primary-tint"
            @click="openAllRanksOverview"
          >
            <PlumpIcon name="grid" :size="16" />
            <span>榜单总览 ({{ store.rankList.length || 57 }})</span>
          </button>
        </div>
      </div>

      <!-- 开放式旗舰榜单横幅舞台（对齐 Android RankDetailScreen 开放式头部 + 桌面端高密度交互） -->
      <div class="rank-hero-banner-card" :class="getRankThemeClass(store.activeRank)">
        <div class="rhb-left">
          <div class="rhb-cover-box">
            <img
              v-if="activeRankCover"
              :src="activeRankCover"
              class="rhb-cover-img"
              draggable="false"
              alt=""
            />
            <div v-else class="rhb-cover-fallback">
              <PlumpIcon name="trophy" :size="42" color="#fff" />
            </div>
            <span class="rhb-count-badge">
              {{ store.activeRankSongs.length || store.activeRank?.totalSongs || 100 }} 首
            </span>
          </div>

          <div class="rhb-info-col">
            <div class="rhb-eyebrow-row">
              <span class="rhb-pill accent">
                <span class="rhb-dot"></span>
                {{ store.activeRank?.category || "官方热榜" }}
              </span>
              <span class="rhb-pill">
                {{ store.activeRank?.updateFrequency || "每天更新" }}
              </span>
              <span class="rhb-pill">
                共 {{ store.activeRankSongs.length }} 首上榜曲目
              </span>
              <span v-if="activeRankHiResCount > 0" class="rhb-pill highlight">
                {{ activeRankHiResCount }} 首无损 / Hi-Res
              </span>
              <span v-if="store.isLoadingMoreRankSongs" class="rhb-pill syncing">
                <PlumpIcon name="refresh" :size="12" class="is-spinning" />
                <span>正在同步完整榜单曲库...</span>
              </span>
            </div>

            <h1 class="rhb-title">
              {{ store.activeRank?.name || store.activeRank?.rankName || "巅峰排行榜" }}
            </h1>

            <div class="rhb-cta-row">
              <button
                type="button"
                class="rhb-primary-play-btn"
                :disabled="filteredRankSongsWithIndex.length === 0"
                @click="playAllActiveRank"
              >
                <PlumpIcon name="play" :size="16" color="#fff" />
                <span>播放全部 ({{ filteredRankSongsWithIndex.length }})</span>
              </button>

              <button
                type="button"
                class="rhb-secondary-btn"
                :disabled="filteredRankSongsWithIndex.length === 0"
                @click="shufflePlayActiveRank"
              >
                <PlumpIcon name="shuffle" :size="15" />
                <span>随机漫游</span>
              </button>

              <button
                type="button"
                class="rhb-secondary-btn"
                @click="openAllRanksOverview"
              >
                <PlumpIcon name="grid" :size="15" />
                <span>切换榜单</span>
              </button>
            </div>
          </div>
        </div>

        <div class="rhb-right">
          <div class="rhb-search-box">
            <PlumpIcon name="search" :size="15" />
            <input
              v-model="store.rankSearchKeyword"
              type="text"
              placeholder="筛选本榜歌曲 / 歌手 / 专辑..."
            />
            <button
              v-if="store.rankSearchKeyword"
              type="button"
              class="clear-inline-btn"
              @click="store.rankSearchKeyword = ''"
            >
              ×
            </button>
          </div>

          <!-- 多页大榜（如 TOP500 共 500 首）快速区间分段导航条 -->
          <div
            v-if="rankRangeSegments.length > 1 && !store.rankSearchKeyword.trim()"
            class="rank-range-pills"
          >
            <button
              v-for="seg in rankRangeSegments"
              :key="seg.key"
              type="button"
              class="rank-range-pill"
              :class="{ active: activeRankRangeKey === seg.key }"
              @click="activeRankRangeKey = seg.key"
            >
              {{ seg.label }}
            </button>
          </div>
        </div>
      </div>

      <!-- 全宽双列无框榜单曲目网格 -->
      <div v-if="store.isLoadingRankSongs && store.activeRankSongs.length === 0" class="ns-empty-hint">
        <PlumpIcon name="refresh" :size="20" class="is-spinning" />
        <span>正在同步「{{ store.activeRank?.name || "巅峰榜单" }}」实时权威曲库...</span>
      </div>

      <div v-else-if="filteredRankSongsWithIndex.length === 0" class="ns-empty-hint">
        <span>未找到匹配的榜单曲目，可清空筛选词或点击右上角刷新重试</span>
      </div>

      <div v-else class="rank-fullwidth-tracks">
        <div class="frameless-songs-grid-2col">
          <div
            v-for="item in filteredRankSongsWithIndex"
            :key="item.song.hash + '_' + item.rankNo"
            :data-rank-song-hash="item.song.hash"
            class="frameless-song-row"
            :class="{ active: store.currentSong?.hash === item.song.hash }"
            @click="store.playSongFromList(item.song, store.activeRankSongs, store.activeRank?.name || '巅峰榜单')"
            @contextmenu.prevent="store.openSongContextMenu($event, item.song, store.activeRankSongs)"
          >
            <span class="fs-index" :class="item.rankNo <= 3 ? 'rank-top-' + item.rankNo : ''">
              <template v-if="store.currentSong?.hash === item.song.hash && store.isPlaying">
                <span class="mini-eq-bars"><i></i><i></i><i></i></span>
              </template>
              <template v-else>
                {{ item.rankNo }}
              </template>
            </span>

            <div class="fs-cover-wrap">
              <img :src="item.song.coverUrl" class="fs-cover" loading="lazy" alt="" />
              <div class="fs-cover-play">
                <PlumpIcon
                  :name="store.currentSong?.hash === item.song.hash && store.isPlaying ? 'pause' : 'play'"
                  :size="15"
                  color="#fff"
                />
              </div>
            </div>

            <div class="fs-meta">
              <div class="fs-title-line">
                <span class="fs-title">{{ item.song.title }}</span>
                <span v-if="item.song.hasHiRes || item.song.isHiRes" class="fs-badge hires">Hi-Res</span>
                <span v-else-if="item.song.hasSq || item.song.isLossless" class="fs-badge sq">SQ</span>
                <span v-if="item.song.isVip" class="fs-badge vip">VIP</span>
              </div>
              <div class="fs-sub-line">
                <button
                  type="button"
                  class="inline-meta-btn"
                  title="点击进入歌手独立主页"
                  @click.stop="store.navigateToSongArtist(item.song)"
                >
                  {{ item.song.artist }}
                </button>
                <button
                  v-if="item.song.album"
                  type="button"
                  class="inline-meta-btn fs-album"
                  title="点击进入专辑独立页面"
                  @click.stop="store.navigateToSongAlbum(item.song)"
                >
                  · {{ item.song.album }}
                </button>
              </div>
            </div>

            <div class="fs-actions" @click.stop>
              <button
                type="button"
                class="fs-icon-btn"
                :class="{ liked: store.isSongLiked(item.song.hash) }"
                title="喜欢"
                @click="store.toggleFavoriteSong(item.song)"
              >
                <PlumpIcon
                  :name="store.isSongLiked(item.song.hash) ? 'heart-filled' : 'heart'"
                  :size="16"
                />
              </button>
              <button
                type="button"
                class="fs-icon-btn"
                title="查看评论"
                @click="store.openCommentsModal(item.song)"
              >
                <PlumpIcon name="comment" :size="15" />
              </button>
              <button
                type="button"
                class="fs-icon-btn"
                title="添加到歌单"
                @click="store.addToPlaylistModalSong = item.song"
              >
                <PlumpIcon name="plus" :size="16" />
              </button>
            </div>

            <span class="fs-duration">{{ formatDuration(item.song) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- ==================== 二级页面 C：电台广场 ==================== -->
    <div v-else-if="store.discoverSubView === 'radio'" class="discover-subpage radio-plaza-page">
      <div class="subpage-frameless-header">
        <div class="sfh-left">
          <button class="frameless-hover-btn" @click="store.discoverSubView = 'home'">
            <PlumpIcon name="back" :size="17" />
            <span>返回发现</span>
          </button>
          <h2 class="sfh-title">电台广场</h2>

          <div class="app-sliding-tabs">
            <button
              v-for="cat in radioPlazaCategories"
              :key="cat.id"
              class="app-sliding-tab-btn"
              :class="{ active: store.selectedFmCategory === cat.id }"
              @click="store.selectedFmCategory = cat.id"
            >
              <span class="tab-label">{{ cat.label }}</span>
              <span
                class="tab-bottom-indicator"
                :style="{
                  backgroundColor: cat.color,
                  opacity: store.selectedFmCategory === cat.id ? 1 : 0,
                  transform: store.selectedFmCategory === cat.id ? 'scaleX(1)' : 'scaleX(0.2)',
                }"
              ></span>
            </button>
          </div>
        </div>

        <div class="sfh-right">
          <button
            class="frameless-hover-btn"
            :disabled="store.isLoadingFmStation"
            @click="refreshCurrentRadioStation"
          >
            <PlumpIcon
              name="refresh"
              :size="16"
              :class="{ 'is-spinning': store.isLoadingFmStation }"
            />
            <span>换一批频率曲目</span>
          </button>
          <button
            class="frameless-hover-btn primary-tint"
            :disabled="store.isLoadingFmStation"
            @click="appendCurrentRadioStation"
          >
            <PlumpIcon name="plus" :size="16" />
            <span>+20首漫游续杯</span>
          </button>
        </div>
      </div>

      <div class="radio-plaza-hero-split">
        <div class="rp-now-tuning-deck">
          <div class="rp-deck-vinyl-wrap">
            <div class="rp-deck-vinyl" :class="{ spinning: store.isPlaying }"></div>
            <img
              v-if="activePlazaCover"
              :src="activePlazaCover"
              class="rp-deck-cover"
              alt=""
            />
          </div>
          <div class="rp-deck-info">
            <div class="rp-freq-pill">
              <span class="live-dot"></span>
              <span>{{ store.activeFmChannel?.freqLabel || "FM 98.6 MHz" }} · 正在调频广播</span>
            </div>
            <h3 class="rp-station-title">{{ store.activeFmChannel?.name || "流行风向标" }}</h3>
            <p class="rp-station-desc">
              {{ store.activeFmChannel?.description || "全天候智能音乐伴听 · 热歌与品味共振" }}
            </p>
            <div class="rp-deck-actions">
              <button class="frameless-hover-btn primary-tint" @click="playActiveStationAll">
                <PlumpIcon name="play" :size="16" />
                <span>立即播放本台 ({{ store.fmStationSongs.length }})</span>
              </button>
              <button class="frameless-hover-btn" @click="pickRandomFmChannel">
                <PlumpIcon name="shuffle" :size="16" />
                <span>随机探索电台</span>
              </button>
            </div>
          </div>
        </div>

        <div class="rp-stations-wall">
          <div
            v-for="(ch, idx) in filteredPlazaChannels"
            :key="ch.id"
            class="rp-station-tile"
            :class="{ active: store.activeFmChannel?.id === ch.id }"
            @click="selectAndPlayPlazaChannel(ch)"
          >
            <div class="rp-tile-cover-wrap">
              <img
                v-if="getStationCover(ch, idx)"
                :src="getStationCover(ch, idx)"
                class="rp-tile-cover"
                loading="lazy"
                alt=""
              />
              <span class="rp-tile-freq">{{ ch.freqLabel }}</span>
              <button
                class="rp-tile-play"
                @click.stop="selectAndPlayPlazaChannel(ch, true)"
              >
                <PlumpIcon
                  :name="store.activeFmChannel?.id === ch.id && store.isPlaying ? 'pause' : 'play'"
                  :size="15"
                  color="#fff"
                />
              </button>
            </div>
            <div class="rp-tile-meta">
              <div class="rp-tile-name">{{ ch.name }}</div>
              <div class="rp-tile-listeners">{{ ch.listenerCountText }}</div>
            </div>
          </div>
        </div>
      </div>

      <div class="rp-stream-section">
        <div class="frameless-sublist-toolbar">
          <div class="frameless-sublist-info">
            <span class="sublist-badge">LIVE STREAM</span>
            <span class="sublist-desc">
              「{{ store.activeFmChannel?.name || "精选电台" }}」实时漫游曲库 ({{ store.fmStationSongs.length }} 首)
            </span>
          </div>
        </div>

        <div class="frameless-songs-grid-2col">
          <div
            v-for="(song, idx) in store.fmStationSongs"
            :key="song.hash + '_' + idx"
            class="frameless-song-row"
            :class="{ active: store.currentSong?.hash === song.hash }"
            @click="store.playSongFromList(song, store.fmStationSongs)"
            @contextmenu.prevent="store.openSongContextMenu($event, song, store.fmStationSongs)"
          >
            <span class="fs-index">{{ String(idx + 1).padStart(2, "0") }}</span>
            <div class="fs-cover-wrap">
              <img :src="song.coverUrl" class="fs-cover" loading="lazy" alt="" />
              <div class="fs-cover-play">
                <PlumpIcon
                  :name="store.currentSong?.hash === song.hash && store.isPlaying ? 'pause' : 'play'"
                  :size="15"
                  color="#fff"
                />
              </div>
            </div>
            <div class="fs-meta">
              <div class="fs-title-line">
                <span class="fs-title">{{ song.title }}</span>
                <span v-if="song.hasHiRes || song.isHiRes" class="fs-badge hires">Hi-Res</span>
                <span v-else-if="song.hasSq || song.isLossless" class="fs-badge sq">SQ</span>
              </div>
              <div class="fs-sub-line">
                <span>{{ song.artist }}</span>
                <span v-if="song.album" class="fs-album">· {{ song.album }}</span>
              </div>
            </div>
            <div class="fs-actions" @click.stop>
              <button
                class="fs-icon-btn"
                :class="{ liked: store.isSongLiked(song.hash) }"
                @click="store.toggleFavoriteSong(song)"
              >
                <PlumpIcon
                  :name="store.isSongLiked(song.hash) ? 'heart-filled' : 'heart'"
                  :size="16"
                />
              </button>
              <button class="fs-icon-btn" @click="store.addToPlaylistModalSong = song">
                <PlumpIcon name="plus" :size="16" />
              </button>
            </div>
            <span class="fs-duration">{{ formatDuration(song) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 全部榜单分类总览弹窗（五维分类 + 搜索过滤 + 概念版横向榜单卡片） -->
    <transition name="fade-pop">
      <div
        v-if="store.isRankGridModalOpen"
        class="modal-backdrop rank-overview-backdrop"
        @click.self="store.isRankGridModalOpen = false"
      >
        <div class="rank-overview-dialog glass-popover">
          <div class="rom-header">
            <div class="rom-title-group">
              <h3>全部官方巅峰榜单</h3>
              <span class="rom-total-pill">共 {{ store.rankList.length }} 个权威榜单</span>
            </div>

            <div class="rom-header-right">
              <div class="rom-search-box">
                <PlumpIcon name="search" :size="14" class="rom-search-icon" />
                <input
                  v-model="rankModalSearchKeyword"
                  type="text"
                  placeholder="搜索榜单名称或热歌..."
                  class="rom-search-input"
                />
                <button
                  v-if="rankModalSearchKeyword"
                  class="rom-search-clear"
                  @click="rankModalSearchKeyword = ''"
                >
                  <PlumpIcon name="close" :size="11" />
                </button>
              </div>
              <button class="frameless-hover-btn rom-close-btn" @click="store.isRankGridModalOpen = false" title="关闭">
                <PlumpIcon name="close" :size="16" />
              </button>
            </div>
          </div>

          <!-- 分类胶囊导航栏 -->
          <div class="rom-category-tabs">
            <button
              v-for="cat in rankOverviewCategories"
              :key="cat.key"
              class="rom-cat-pill"
              :class="{ active: rankModalCategory === cat.key }"
              @click="rankModalCategory = cat.key"
            >
              <span>{{ cat.label }}</span>
              <span class="rom-cat-count">{{ cat.count }}</span>
            </button>
          </div>

          <!-- 分类榜单分组滚动区 -->
          <div class="rom-scroll-body">
            <div
              v-if="groupedModalRankSections.length === 0"
              class="rom-empty-state"
            >
              没有找到匹配的榜单
            </div>

            <section
              v-for="group in groupedModalRankSections"
              :key="group.category"
              class="rom-category-section"
            >
              <div class="rom-sec-header">
                <div class="rom-sec-title-wrap">
                  <span class="rom-sec-accent-bar"></span>
                  <h4 class="rom-sec-title">{{ group.category }}</h4>
                  <span class="rom-sec-sub">{{ group.subtitle }}</span>
                </div>
                <span class="rom-sec-badge">{{ group.ranks.length }} 个榜单</span>
              </div>

              <div class="rom-cards-grid">
                <div
                  v-for="(rk, rIdx) in group.ranks"
                  :key="rk.rankId"
                  class="rank-overview-card"
                  :class="[
                    getRankThemeClass(rk, rIdx),
                    { 'is-active-rank': store.activeRank?.rankId === rk.rankId }
                  ]"
                  @click="store.openRankDetail(rk)"
                >
                  <div class="roc-cover-wrap">
                    <img
                      v-if="rk.coverUrl"
                      :src="rk.coverUrl"
                      class="roc-cover-img"
                      loading="lazy"
                      draggable="false"
                      alt=""
                    />
                    <div v-else class="roc-cover-fallback">
                      <span>{{ (rk.name || rk.rankName || "榜单").slice(0, 4) }}</span>
                    </div>
                  </div>

                  <div class="roc-info">
                    <div class="roc-title-row">
                      <h5 class="roc-title" :title="rk.name || rk.rankName">{{ rk.name || rk.rankName }}</h5>
                    </div>
                    <div class="roc-meta-row">
                      <span class="roc-freq-tag">{{ rk.updateFrequency || "定期更新" }}</span>
                      <span v-if="rk.totalSongs > 0" class="roc-count-tag">{{ rk.totalSongs }}首</span>
                    </div>
                    <p
                      v-if="getRankFirstPreviewText(rk)"
                      class="roc-top1-preview"
                      :title="getRankFirstPreviewText(rk)"
                    >
                      TOP1 · {{ getRankFirstPreviewText(rk) }}
                    </p>
                  </div>

                  <button
                    class="roc-play-btn"
                    title="直接播放此榜单"
                    @click.stop="store.playRankDirectly(rk)"
                  >
                    <PlumpIcon name="play" :size="13" color="currentColor" />
                  </button>
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { NEW_SONG_AREAS, useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import type { FmChannel, RankItem, Song } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const heroSongFilter = ref("");
function scrollDiscoverStageToTop() {
  const stage = document.querySelector(".skyline-stage-viewport") as HTMLElement | null;
  if (stage) {
    stage.scrollTop = 0;
  }
}

watch(
  () => store.discoverSubView,
  () => {
    scrollDiscoverStageToTop();
    nextTick(() => {
      scrollDiscoverStageToTop();
      requestAnimationFrame(scrollDiscoverStageToTop);
    });
  }
);

function openNewSongsSubpage() {
  store.discoverSubView = "new_songs";
  scrollDiscoverStageToTop();
  nextTick(scrollDiscoverStageToTop);
}

const isDailyExpandedMode = computed(
  () => store.discoverHeroExpanding === "daily" || store.discoverSubView === "daily"
);
const isFmExpandedMode = computed(
  () => store.discoverHeroExpanding === "fm" || store.discoverSubView === "fm_detail"
);

const now = new Date();
const todayDayNum = String(now.getDate()).padStart(2, "0");
const monthNames = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
const todayMonthEng = monthNames[now.getMonth()];
const weekNames = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
const todayDateLabel = `${now.getMonth() + 1}月${now.getDate()}日 · ${weekNames[now.getDay()]}`;

const newSongDateRangeLabel = computed(() => {
  const end = new Date();
  const start = new Date(end.getTime() - 7 * 86400000);
  const fmt = (d: Date) =>
    `${String(d.getMonth() + 1).padStart(2, "0")}.${String(d.getDate()).padStart(2, "0")}`;
  return `${fmt(start)} - ${fmt(end)}`;
});

const dailyCoverTriple = computed(() => {
  const covers = store.recommendSongs
    .map((s) => s.coverUrl)
    .filter((u) => Boolean(u));
  return [
    covers[0] || store.currentSong?.coverUrl || "",
    covers[1] || covers[0] || "",
    covers[2] || covers[1] || covers[0] || "",
  ];
});

const dailyHiResCount = computed(
  () => store.recommendSongs.filter((s) => s.hasHiRes || s.hasSq || s.isLossless || s.isHiRes).length
);

const isFmPlaying = computed(
  () =>
    Boolean(
      store.isPlaying &&
        store.currentSong &&
        store.currentFmSong &&
        store.currentSong.hash === store.currentFmSong.hash
    )
);

function handleDailyCardClick() {
  if (isDailyExpandedMode.value) {
    collapseToDiscoverHome();
    return;
  }
  heroSongFilter.value = "";
  store.openDailyRecommendPage();
}

function handleFmCardClick() {
  if (isFmExpandedMode.value) {
    collapseToDiscoverHome();
    return;
  }
  heroSongFilter.value = "";
  store.openGuessYouLikePage();
}

function collapseToDiscoverHome() {
  store.discoverHeroExpanding = "none";
  store.discoverSubView = "home";
}

function playAllDaily() {
  if (store.recommendSongs.length > 0) {
    store.playSongFromList(store.recommendSongs[0], store.recommendSongs);
  } else {
    store.loadDailyRecommend();
  }
}

function toggleFmPlay() {
  if (isFmPlaying.value) {
    store.togglePlayPause();
  } else if (store.currentFmSong) {
    store.playSongFromList(store.currentFmSong, store.personalFmSongs);
  } else {
    store.nextPersonalFmTrack(true);
  }
}

const activeHeroSongs = computed<Song[]>(() => {
  if (store.discoverSubView === "fm_detail") {
    return store.personalFmSongs;
  }
  return store.recommendSongs;
});

const filteredHeroSongs = computed(() => {
  const kw = heroSongFilter.value.trim().toLowerCase();
  if (!kw) return activeHeroSongs.value;
  return activeHeroSongs.value.filter(
    (s) =>
      s.title.toLowerCase().includes(kw) ||
      s.artist.toLowerCase().includes(kw) ||
      (s.album && s.album.toLowerCase().includes(kw))
  );
});

function isCurrentSongInList(list: Song[]) {
  if (!store.currentSong) return false;
  return list.some((s) => s.hash === store.currentSong?.hash);
}

function scrollToCurrentPlayingSong() {
  if (!store.currentSong) return;
  const el = document.querySelector(`[data-song-hash="${store.currentSong.hash}"]`);
  if (el) {
    el.scrollIntoView({ behavior: "smooth", block: "center" });
  }
}

function scrollToCurrentPlayingRankSong() {
  if (!store.currentSong) return;
  const el = document.querySelector(`[data-rank-song-hash="${store.currentSong.hash}"]`);
  if (el) {
    el.scrollIntoView({ behavior: "smooth", block: "center" });
  }
}

// ==================== 新歌速递（涵盖六大分类：华语、粤语、欧美、日韩、ACG、影视 + 同日期首字母排序与可折叠时间轴） ====================
const newSongAreas = NEW_SONG_AREAS;
const collapsedDateKeys = ref<Record<string, boolean>>({});

watch(
  () => store.newSongPeriod,
  () => {
    collapsedDateKeys.value = {};
  }
);

const COLLATOR_LETTERS = "ABCDEFGHJKLMNOPQRSTWXYZ".split("");
const COLLATOR_BOUNDARIES = "阿八嚓哒妸发旮哈讥咔垃妈拿噢啪期然撒塌挖昔压匝".split("");
const KR_CHOSEONG_INITIALS = [
  "G", "K", "N", "D", "T", "R", "M", "B", "P",
  "S", "S", "A", "J", "C", "C", "K", "T", "P", "H",
];
const pinyinCollator =
  typeof Intl !== "undefined"
    ? new Intl.Collator("zh-CN-u-co-pinyin", { numeric: true, sensitivity: "base" })
    : null;

function stripLeadingPunctuation(text: string): string {
  return (text || "")
    .replace(/^[\s《〈「『【〔（(["'‘“·~!@#$%^&*_\-+=:;,.?/\\|<>]+/u, "")
    .trim();
}

function extractFirstLetter(text: string): string {
  const cleaned = stripLeadingPunctuation(text);
  if (!cleaned) return "#";
  const firstChar = cleaned[0];
  const norm = firstChar.normalize("NFD")[0]?.toUpperCase() || firstChar.toUpperCase();
  if (norm >= "A" && norm <= "Z") return norm;
  if (norm >= "0" && norm <= "9") return "#";

  const code = firstChar.codePointAt(0) || 0;
  // 日文平假名 / 片假名五十音罗马字首字母映射
  if ((code >= 0x3041 && code <= 0x3096) || (code >= 0x30a1 && code <= 0x30fa)) {
    const h = code >= 0x30a1 && code <= 0x30fa ? code - 0x60 : code;
    if (h <= 0x304a) return "A";
    if (h <= 0x3054) return "K";
    if (h <= 0x305e) return "S";
    if (h <= 0x3069) return "T";
    if (h <= 0x306e) return "N";
    if (h <= 0x307d) return "H";
    if (h <= 0x3082) return "M";
    if (h <= 0x3088) return "Y";
    if (h <= 0x308d) return "R";
    return "W";
  }
  // 韩文谚文音节初声罗马字首字母映射
  if (code >= 0xac00 && code <= 0xd7a3) {
    const choseongIdx = Math.floor((code - 0xac00) / 588);
    return KR_CHOSEONG_INITIALS[choseongIdx] || "#";
  }
  // 中文汉字拼音首字母映射
  if (code >= 0x4e00 && code <= 0x9fff) {
    for (let i = COLLATOR_BOUNDARIES.length - 1; i >= 0; i--) {
      const cmp = pinyinCollator
        ? pinyinCollator.compare(firstChar, COLLATOR_BOUNDARIES[i])
        : firstChar.localeCompare(COLLATOR_BOUNDARIES[i], "zh-CN");
      if (cmp >= 0) {
        return COLLATOR_LETTERS[i];
      }
    }
  }
  return "#";
}

function compareSongsSameDateAlpha(a: Song, b: Song): number {
  const la = extractFirstLetter(a.title);
  const lb = extractFirstLetter(b.title);
  if (la !== lb) {
    if (la === "#") return 1;
    if (lb === "#") return -1;
    return la.localeCompare(lb);
  }
  const cleanA = stripLeadingPunctuation(a.title) || a.title;
  const cleanB = stripLeadingPunctuation(b.title) || b.title;
  return pinyinCollator
    ? pinyinCollator.compare(cleanA, cleanB)
    : cleanA.localeCompare(cleanB, "zh-CN");
}

function normalizeDateKey(raw: string): string {
  const cleaned = (raw || "").trim();
  if (/^\d{4}-\d{2}-\d{2}/.test(cleaned)) {
    return cleaned.slice(0, 10);
  }
  return "";
}

function formatShortPublishDate(raw: string): string {
  if (!raw || raw.length < 10) return "";
  return raw.slice(5, 10).replace("-", ".");
}

function formatDateGroupLabel(dateKey: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(dateKey);
  if (!m) return dateKey;
  const y = Number(m[1]);
  const mo = Number(m[2]);
  const da = Number(m[3]);
  const dObj = new Date(y, mo - 1, da);
  const cur = new Date();
  const todayMidnight = new Date(cur.getFullYear(), cur.getMonth(), cur.getDate()).getTime();
  const targetMidnight = dObj.getTime();
  const diffDays = Math.round((todayMidnight - targetMidnight) / 86400000);
  const mmdd = `${String(mo).padStart(2, "0")}月${String(da).padStart(2, "0")}日`;
  if (diffDays === 0) return `${mmdd} · 今日`;
  if (diffDays === 1) return `${mmdd} · 昨日`;
  return `${mmdd} · ${weekNames[dObj.getDay()]}`;
}

const sortedNewSongsAll = computed<Song[]>(() => {
  const list = store.newSongs
    .map((s) => ({
      ...s,
      publishDate: normalizeDateKey(s.publishDate),
    }))
    .filter((s) => Boolean(s.publishDate));

  return list.sort((a, b) => {
    if (a.publishDate !== b.publishDate) {
      return b.publishDate.localeCompare(a.publishDate);
    }
    return compareSongsSameDateAlpha(a, b);
  });
});

const sortedTop12NewSongs = computed(() => sortedNewSongsAll.value.slice(0, 12));

function playAllNewSongs() {
  if (sortedNewSongsAll.value.length > 0) {
    store.playSongFromList(sortedNewSongsAll.value[0], sortedNewSongsAll.value);
  }
}

function playDateGroupSongs(songs: Song[]) {
  if (songs.length > 0) {
    store.playSongFromList(songs[0], songs);
  }
}

const groupedNewSongsByDate = computed(() => {
  const map = new Map<string, Song[]>();
  for (const s of sortedNewSongsAll.value) {
    const d = s.publishDate;
    if (!map.has(d)) map.set(d, []);
    map.get(d)!.push(s);
  }
  return Array.from(map.entries()).map(([dateKey, songs]) => ({
    dateKey,
    displayLabel: formatDateGroupLabel(dateKey),
    songs: [...songs].sort(compareSongsSameDateAlpha),
  }));
});

function isDateGroupCollapsed(dateKey: string): boolean {
  return Boolean(collapsedDateKeys.value[dateKey]);
}

function toggleDateGroup(dateKey: string) {
  collapsedDateKeys.value = {
    ...collapsedDateKeys.value,
    [dateKey]: !collapsedDateKeys.value[dateKey],
  };
}

const isAllDateGroupsCollapsed = computed(() => {
  const groups = groupedNewSongsByDate.value;
  if (groups.length === 0) return false;
  return groups.every((g) => Boolean(collapsedDateKeys.value[g.dateKey]));
});

function toggleAllDateGroups() {
  const shouldCollapse = !isAllDateGroupsCollapsed.value;
  const nextMap: Record<string, boolean> = {};
  for (const g of groupedNewSongsByDate.value) {
    nextMap[g.dateKey] = shouldCollapse;
  }
  collapsedDateKeys.value = nextMap;
}

// ==================== 巅峰排行榜：五维分类、概念版卡片、全量榜单总览与详情页 ====================
const RANK_CATEGORY_METAS = [
  { key: "官方热榜", label: "官方热榜", subtitle: "全站潮流核心风向标，实时捕捉热播单曲" },
  { key: "特色音乐", label: "特色音乐", subtitle: "短视频、影视综艺、ACG 与世代热歌精选" },
  { key: "曲风流派", label: "曲风流派", subtitle: "电音、说唱、摇滚、民谣、国风与纯音乐垂类榜单" },
  { key: "地区语种", label: "地区语种", subtitle: "华语、粤语、欧美、日韩及小语种地区热歌榜" },
  { key: "全球权威", label: "全球权威", subtitle: "Billboard、Oricon、UK、Melon 等国际权威榜单" },
] as const;

const homeRankCategory = ref<string>("featured");
const rankHomeCategoryTabs = [
  { id: "featured", key: "featured", label: "精选推荐", color: "#ff5b6b" },
  { id: "官方热榜", key: "官方热榜", label: "官方热榜", color: "#ff5b6b" },
  { id: "特色音乐", key: "特色音乐", label: "特色音乐", color: "#10b981" },
  { id: "曲风流派", key: "曲风流派", label: "曲风流派", color: "#9b6bff" },
  { id: "地区语种", key: "地区语种", label: "地区语种", color: "#18bfff" },
  { id: "全球权威", key: "全球权威", label: "全球权威", color: "#f59e0b" },
];

const homeDoubleRowRanks = computed(() => {
  if (homeRankCategory.value === "featured") {
    return store.rankList.slice(0, 8);
  }
  const matched = store.rankList.filter((r) => (r.category || "特色音乐") === homeRankCategory.value);
  return (matched.length > 0 ? matched : store.rankList).slice(0, 8);
});

const rankModalCategory = ref<string>("all");
const rankModalSearchKeyword = ref<string>("");

function openAllRanksOverview() {
  rankModalSearchKeyword.value = "";
  store.isRankGridModalOpen = true;
}

const rankOverviewCategories = computed(() => {
  const total = store.rankList.length;
  const counts: Record<string, number> = {};
  for (const r of store.rankList) {
    const c = r.category || "特色音乐";
    counts[c] = (counts[c] || 0) + 1;
  }
  return [
    { key: "all", label: "全部榜单", count: total },
    ...RANK_CATEGORY_METAS.map((m) => ({
      key: m.key,
      label: m.label,
      count: counts[m.key] || 0,
    })).filter((item) => item.count > 0),
  ];
});

const groupedModalRankSections = computed(() => {
  const kw = rankModalSearchKeyword.value.trim().toLowerCase();
  const targetCat = rankModalCategory.value;

  const filtered = store.rankList.filter((rk) => {
    if (targetCat !== "all" && (rk.category || "特色音乐") !== targetCat) {
      return false;
    }
    if (!kw) return true;
    const displayName = (rk.name || rk.rankName || "").toLowerCase();
    const inName = displayName.includes(kw);
    const inCat = (rk.category || "").toLowerCase().includes(kw);
    const previews = rk.songPreviews || rk.topSongsPreview || [];
    const inPreview = previews.some((p) => p.toLowerCase().includes(kw));
    return inName || inCat || inPreview;
  });

  const sections: Array<{ category: string; subtitle: string; ranks: RankItem[] }> = [];
  for (const meta of RANK_CATEGORY_METAS) {
    const items = filtered.filter((r) => (r.category || "特色音乐") === meta.key);
    if (items.length > 0) {
      sections.push({
        category: meta.label,
        subtitle: meta.subtitle,
        ranks: items,
      });
    }
  }

  // 兜底：若存在未归入预设五类的榜单，追加显示
  const knownKeys = new Set<string>(RANK_CATEGORY_METAS.map((m) => m.key));
  const others = filtered.filter((r) => !knownKeys.has(r.category || "特色音乐"));
  if (others.length > 0) {
    sections.push({
      category: "其他榜单",
      subtitle: "更多垂直领域特色榜单",
      ranks: others,
    });
  }

  return sections;
});

function getRankThemeClass(rk: RankItem | null | undefined, fallbackIdx = 0): string {
  if (!rk) return "rank-theme-coral";
  const name = rk.name || rk.rankName || "";
  const cat = rk.category || "";
  if (name.includes("飙升") || name.includes("TOP500") || name.includes("热歌")) {
    return "rank-theme-coral";
  }
  if (name.includes("新歌") || cat === "地区语种") {
    return "rank-theme-cyan";
  }
  if (cat === "曲风流派" || name.includes("电音") || name.includes("说唱") || name.includes("ACG")) {
    return "rank-theme-violet";
  }
  if (cat === "全球权威" || name.includes("Billboard") || name.includes("UK") || name.includes("金曲")) {
    return "rank-theme-amber";
  }
  if (cat === "特色音乐") {
    return "rank-theme-emerald";
  }
  const themes = [
    "rank-theme-coral",
    "rank-theme-cyan",
    "rank-theme-violet",
    "rank-theme-amber",
    "rank-theme-emerald",
  ];
  return themes[fallbackIdx % themes.length];
}

function sanitizePreviewRawLine(raw: string): string {
  return (raw || "")
    .trim()
    .replace(/^\d+\s*[.、．\-\s]\s*/, "")
    .trim();
}

function getRankPreviewLines(rk: RankItem): string[] {
  const previews = rk.songPreviews && rk.songPreviews.length > 0 ? rk.songPreviews : rk.topSongsPreview;
  if (previews && previews.length > 0) {
    return previews.slice(0, 3).map(sanitizePreviewRawLine).filter(Boolean);
  }
  const fallbackPool = store.recommendSongs.length > 0 ? store.recommendSongs : store.newSongs;
  if (fallbackPool.length >= 3) {
    const offset = rk.rankId % Math.max(1, fallbackPool.length - 3);
    return fallbackPool.slice(offset, offset + 3).map((s) => `${s.title} - ${s.artist}`);
  }
  return ["概念版实时热播曲目", "无损母带音源飙升中", "点击查看完整权威榜单"];
}

function getRankPreviewStructured(rk: RankItem): Array<{ title: string; artist: string }> {
  const lines = getRankPreviewLines(rk);
  return lines.map((line) => {
    const clean = sanitizePreviewRawLine(line);
    const sepIdx = clean.indexOf(" - ");
    if (sepIdx > 0) {
      return {
        title: clean.slice(0, sepIdx).trim(),
        artist: clean.slice(sepIdx + 3).trim(),
      };
    }
    return { title: clean, artist: "" };
  });
}

function getRankFirstPreviewText(rk: RankItem): string {
  const previews = rk.songPreviews && rk.songPreviews.length > 0 ? rk.songPreviews : rk.topSongsPreview;
  if (previews && previews.length > 0) {
    return sanitizePreviewRawLine(previews[0]);
  }
  return "";
}

const activeRankRangeKey = ref<string>("all");

watch(
  () => store.activeRank?.rankId,
  () => {
    activeRankRangeKey.value = "all";
  }
);

const rankRangeSegments = computed(() => {
  const total = store.activeRankSongs.length;
  if (total <= 100) return [];
  const segs: Array<{ key: string; label: string; start: number; end: number }> = [
    { key: "all", label: `全部 (${total})`, start: 1, end: total },
  ];
  const step = 100;
  for (let start = 1; start <= total; start += step) {
    const end = Math.min(total, start + step - 1);
    segs.push({
      key: `${start}-${end}`,
      label: `${start}-${end}`,
      start,
      end,
    });
  }
  return segs;
});

const activeRankCover = computed(() => {
  if (store.activeRank?.coverUrl) return store.activeRank.coverUrl;
  const firstWithCover = store.activeRankSongs.find((s) => Boolean(s.coverUrl));
  return firstWithCover?.coverUrl || "";
});

const activeRankHiResCount = computed(
  () =>
    store.activeRankSongs.filter(
      (s) => s.hasHiRes || s.hasSq || s.isLossless || s.isHiRes
    ).length
);

const filteredRankSongsWithIndex = computed<Array<{ song: Song; rankNo: number }>>(() => {
  const kw = store.rankSearchKeyword.trim().toLowerCase();
  const allWithRank = store.activeRankSongs.map((song, idx) => ({
    song,
    rankNo: idx + 1,
  }));

  if (kw) {
    return allWithRank.filter(
      ({ song }) =>
        song.title.toLowerCase().includes(kw) ||
        song.artist.toLowerCase().includes(kw) ||
        (song.album && song.album.toLowerCase().includes(kw))
    );
  }

  if (activeRankRangeKey.value !== "all") {
    const parts = activeRankRangeKey.value.split("-").map(Number);
    if (parts.length === 2 && !Number.isNaN(parts[0]) && !Number.isNaN(parts[1])) {
      const [start, end] = parts;
      return allWithRank.filter(({ rankNo }) => rankNo >= start && rankNo <= end);
    }
  }

  return allWithRank;
});

function playAllActiveRank() {
  if (store.activeRankSongs.length > 0) {
    store.playSongFromList(store.activeRankSongs[0], store.activeRankSongs);
  }
}

function shufflePlayActiveRank() {
  if (store.activeRankSongs.length === 0) return;
  const rndIdx = Math.floor(Math.random() * store.activeRankSongs.length);
  store.playSongFromList(store.activeRankSongs[rndIdx], store.activeRankSongs);
}

// ==================== 精选场景电台 & 电台广场 ====================
const singleRowFmChannels = computed(() => store.fmChannels.slice(0, 6));

const radioCoverFallbackPool = computed(() => {
  const pool: string[] = [];
  for (const s of store.newSongs) {
    if (s.coverUrl && !pool.includes(s.coverUrl)) pool.push(s.coverUrl);
  }
  for (const s of store.recommendSongs) {
    if (s.coverUrl && !pool.includes(s.coverUrl)) pool.push(s.coverUrl);
  }
  for (const s of store.personalFmSongs) {
    if (s.coverUrl && !pool.includes(s.coverUrl)) pool.push(s.coverUrl);
  }
  return pool;
});

function getStationCover(ch: FmChannel, idx: number): string {
  if (ch.coverUrl && !ch.coverUrl.includes("20210519")) {
    return ch.coverUrl;
  }
  const pool = radioCoverFallbackPool.value;
  if (pool.length > 0) {
    return pool[idx % pool.length];
  }
  return store.currentSong?.coverUrl || "";
}

function playFmChannelFromHome(ch: FmChannel) {
  store.activeFmChannel = ch;
  store.loadFmStationSongs(ch.id, false, true);
}

function openRadioPlaza() {
  store.discoverSubView = "radio";
  scrollDiscoverStageToTop();
  nextTick(scrollDiscoverStageToTop);
  if (!store.activeFmChannel && store.fmChannels.length > 0) {
    store.activeFmChannel = store.fmChannels[0];
    store.loadFmStationSongs(store.fmChannels[0].id, false, false);
  }
}

const radioPlazaCategories = [
  { id: "全部", label: "全部频率", color: "#2b8cff" },
  { id: "推荐电台", label: "推荐", color: "#ff6b6b" },
  { id: "场景", label: "场景", color: "#15c493" },
  { id: "心情", label: "心情", color: "#f5a623" },
  { id: "曲风", label: "曲风", color: "#8e75ff" },
  { id: "年代语种", label: "年代语种", color: "#00c4a7" },
];

const filteredPlazaChannels = computed(() => {
  const cat = store.selectedFmCategory;
  if (!cat || cat === "全部") return store.fmChannels;
  const list = store.fmChannels.filter((c) => c.category.includes(cat) || cat.includes(c.category));
  return list.length > 0 ? list : store.fmChannels;
});

const activePlazaCover = computed(() => {
  if (store.activeFmChannel) {
    const c = getStationCover(store.activeFmChannel, 0);
    if (c) return c;
  }
  return store.fmStationSongs[0]?.coverUrl || store.currentSong?.coverUrl || "";
});

function selectAndPlayPlazaChannel(ch: FmChannel, autoPlay = true) {
  store.activeFmChannel = ch;
  store.loadFmStationSongs(ch.id, false, autoPlay);
}

function refreshCurrentRadioStation() {
  const cid = store.activeFmChannel?.id || store.fmChannels[0]?.id || "hot";
  store.loadFmStationSongs(cid, false, false);
}

function appendCurrentRadioStation() {
  const cid = store.activeFmChannel?.id || store.fmChannels[0]?.id || "hot";
  store.loadFmStationSongs(cid, true, false);
}

function playActiveStationAll() {
  if (store.fmStationSongs.length > 0) {
    store.playSongFromList(store.fmStationSongs[0], store.fmStationSongs);
  } else if (store.activeFmChannel) {
    store.loadFmStationSongs(store.activeFmChannel.id, false, true);
  }
}

function pickRandomFmChannel() {
  if (store.fmChannels.length === 0) return;
  const idx = Math.floor(Math.random() * store.fmChannels.length);
  const picked = store.fmChannels[idx];
  selectAndPlayPlazaChannel(picked, true);
}
</script>

<style scoped>
.inline-meta-btn {
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.inline-meta-btn:hover {
  color: var(--primary);
  text-decoration: underline;
}

.ns-clean-header {
  padding-bottom: 14px;
}

.ns-header-title-group {
  display: inline-flex;
  align-items: baseline;
  gap: 10px;
}

.ns-header-date-pill {
  font-size: 14px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: var(--text-secondary);
}

.ns-empty-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 64px 20px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-secondary);
}

.ns-frameless-date-group {
  margin-bottom: 16px;
}

.ns-frameless-date-group.is-collapsed {
  margin-bottom: 8px;
}

.ns-collapsible-date-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 12px;
  border-bottom: 1px solid var(--border-player-top);
  cursor: pointer;
  user-select: none;
  transition: background 0.18s ease;
}

.ns-collapsible-date-bar:hover {
  background: var(--bg-row-hover);
}

.ns-collapsible-date-bar:focus-visible {
  outline: 2px solid var(--primary);
  outline-offset: 2px;
}

.ns-date-right-tools {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}

.ns-date-play-mini {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 10px;
  border-radius: 999px;
  border: none;
  background: var(--bg-hover-square);
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  opacity: 0;
  transform: translateX(4px);
  transition: opacity 0.18s ease, transform 0.18s ease, color 0.18s ease, background 0.18s ease;
}

.ns-collapsible-date-bar:hover .ns-date-play-mini,
.ns-date-play-mini:focus-visible {
  opacity: 1;
  transform: translateX(0);
}

.ns-date-play-mini:hover {
  color: #ffffff;
  background: var(--primary);
}

.ns-collapse-chevron {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 8px;
  color: var(--text-secondary);
  transition: transform 0.26s cubic-bezier(0.22, 1, 0.36, 1), color 0.18s ease;
}

.ns-collapsible-date-bar:hover .ns-collapse-chevron {
  color: var(--text-primary);
}

.ns-collapse-chevron.collapsed {
  transform: rotate(-90deg);
}

.ns-date-collapse-body {
  display: grid;
  grid-template-rows: 1fr;
  opacity: 1;
  margin-top: 8px;
  transition:
    grid-template-rows 0.28s cubic-bezier(0.22, 1, 0.36, 1),
    opacity 0.22s ease,
    margin-top 0.24s ease;
}

.ns-date-collapse-body.collapsed {
  grid-template-rows: 0fr;
  opacity: 0;
  margin-top: 0;
  pointer-events: none;
}

.ns-date-collapse-inner {
  overflow: hidden;
  min-height: 0;
}

@media (prefers-reduced-motion: reduce) {
  .ns-collapse-chevron,
  .ns-date-collapse-body,
  .ns-date-play-mini {
    transition: none !important;
  }
}
</style>
