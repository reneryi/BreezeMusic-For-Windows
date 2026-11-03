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
                class="hero-expanded-extra-btns"
                :class="{ 'is-visible': isDailyExpandedMode }"
              >
                <button
                  class="hero-circle-icon-btn"
                  :disabled="store.isLoadingRecommend"
                  title="刷新每日推荐"
                  @click.stop="store.loadDailyRecommend()"
                >
                  <PlumpIcon name="refresh" :size="18" color="#fff" />
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
              <!-- 双层漫游封面叠放（移除生硬突出的黑胶圈，改为通透层叠封面 + 动态律动徽章） -->
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

              <!-- 猜你喜欢重构文案区：主标题固定为简洁大气的“猜你喜欢”，下方展示当前与下一首漫游曲目 -->
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

            <!-- 右侧横向一体化播控簇（告别原本上下堆叠的拥挤感） -->
            <div class="hero-card-actions" @click.stop>
              <div
                class="hero-expanded-extra-btns"
                :class="{ 'is-visible': isFmExpandedMode }"
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
                  <PlumpIcon name="refresh" :size="17" color="#fff" />
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
                  <span v-if="song.hasHiRes" class="fs-badge hires">Hi-Res</span>
                  <span v-else-if="song.hasSq" class="fs-badge sq">SQ</span>
                  <span v-if="song.isVip" class="fs-badge vip">VIP</span>
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

              <span class="fs-duration">{{ formatDuration(song.durationSeconds) }}</span>
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
        <!-- 1. 新歌速递：刷新按钮改为纯大图标（20px，无文字） -->
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

            <!-- 右侧：刷新改为纯大图标（无文字），保留全播与查看全部 -->
            <div class="block-right-frameless-actions">
              <button
                class="frameless-icon-square-btn discover-refresh-icon-btn"
                :disabled="store.isLoadingNewSongs"
                title="刷新当前语种新歌"
                @click="store.loadNewSongExpress(store.newSongPeriod)"
              >
                <PlumpIcon name="refresh" :size="20" />
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
                  <span v-if="song.hasHiRes" class="fs-badge hires">Hi-Res</span>
                  <span v-else-if="song.hasSq" class="fs-badge sq">SQ</span>
                </div>
                <div class="ns-song-sub-row">
                  <span class="ns-song-artist">{{ song.artist }}</span>
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

        <!-- 2. 巅峰排行榜：刷新改为纯大图标（无文字），全部榜单不显示数量，榜单卡片全新重构设计 -->
        <section class="discover-block">
          <div class="block-header">
            <div class="block-title-with-tabs">
              <h3 class="block-title">巅峰排行榜</h3>
              <span class="block-sub-date">/ 官方权威潮流风向标</span>
            </div>
            <div class="block-right-frameless-actions">
              <button
                class="frameless-icon-square-btn discover-refresh-icon-btn"
                :disabled="store.isLoadingRanks"
                title="刷新榜单"
                @click="store.loadRankList()"
              >
                <PlumpIcon name="refresh" :size="20" />
              </button>
              <button
                class="frameless-hover-btn"
                @click="store.isRankGridModalOpen = true"
              >
                <span>全部榜单</span>
                <PlumpIcon name="chevron-right" :size="15" />
              </button>
            </div>
          </div>

          <!-- 全新重构的巅峰榜单 Bento 卡片矩阵（2行 × 4列 = 8张卡片） -->
          <div class="rank-double-row-grid">
            <div
              v-for="(rk, rIdx) in homeDoubleRowRanks"
              :key="rk.rankId"
              class="rank-bento-card"
              :class="`rank-accent-${(rIdx % 4) + 1}`"
              @click="store.openRankDetail(rk)"
            >
              <!-- 左侧高清立体封面 + 悬浮快播按钮 -->
              <div class="rbc-cover-stage">
                <img :src="rk.coverUrl" class="rbc-cover-img" loading="lazy" draggable="false" alt="" />
                <button
                  class="rbc-play-fab"
                  title="直接播放该榜单"
                  @click.stop="store.playRankDirectly(rk)"
                >
                  <PlumpIcon name="play" :size="14" color="#fff" />
                </button>
              </div>

              <!-- 右侧重构排版：标题 + 更新胶囊 + 歌名/歌手分离的 Top 3 曲目列 -->
              <div class="rbc-body">
                <div class="rbc-head-row">
                  <h4 class="rbc-title">{{ rk.name }}</h4>
                  <span class="rbc-freq-pill">{{ rk.updateFrequency || rk.category || "每日更新" }}</span>
                </div>

                <div class="rbc-tracks-list">
                  <div
                    v-for="(item, lIdx) in getRankPreviewStructured(rk)"
                    :key="lIdx"
                    class="rbc-track-row"
                  >
                    <span class="rbc-rank-num" :class="'top-' + (lIdx + 1)">
                      {{ String(lIdx + 1).padStart(2, "0") }}
                    </span>
                    <span class="rbc-song-name">{{ item.title }}</span>
                    <span v-if="item.artist" class="rbc-song-artist">{{ item.artist }}</span>
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

    <!-- ==================== 二级页面 A：新歌速递 · 查看全部 ==================== -->
    <div v-else-if="store.discoverSubView === 'new_songs'" class="discover-subpage frameless-subpage">
      <div class="subpage-frameless-header">
        <div class="sfh-left">
          <button class="frameless-hover-btn" @click="store.discoverSubView = 'home'">
            <PlumpIcon name="back" :size="17" />
            <span>返回发现</span>
          </button>
          <h2 class="sfh-title">新歌速递 · 全部曲库</h2>

          <div class="app-sliding-tabs">
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

        <div class="sfh-right">
          <div class="frameless-sublist-search">
            <PlumpIcon name="search" :size="15" />
            <input
              v-model="newSongSearchKw"
              type="text"
              placeholder="筛选新歌名称 / 歌手..."
            />
          </div>
          <button
            class="frameless-icon-square-btn discover-refresh-icon-btn"
            title="刷新新歌"
            @click="store.loadNewSongExpress(store.newSongPeriod)"
          >
            <PlumpIcon name="refresh" :size="20" />
          </button>
          <button class="frameless-hover-btn primary-tint" @click="playAllNewSongs">
            <PlumpIcon name="play" :size="16" />
            <span>播放全部 ({{ sortedNewSongsAll.length }})</span>
          </button>
        </div>
      </div>

      <div class="ns-frameless-timeline">
        <div
          v-for="group in groupedNewSongsByDate"
          :key="group.dateKey"
          class="ns-frameless-date-group"
        >
          <div class="ns-frameless-date-bar">
            <div class="ns-date-title-wrap">
              <span class="ns-date-dot"></span>
              <span class="ns-date-title">{{ group.displayLabel }}</span>
              <span class="ns-date-count">{{ group.songs.length }} 首新发单曲</span>
            </div>
            <div class="ns-az-pills">
              <button
                v-for="letter in group.letters"
                :key="letter"
                class="ns-az-btn"
                @click="scrollToNewSongLetter(group.dateKey, letter)"
              >
                {{ letter }}
              </button>
            </div>
          </div>

          <div class="frameless-songs-grid-2col">
            <div
              v-for="(song, sIdx) in group.songs"
              :key="song.hash + '_' + sIdx"
              :id="getFirstLetterAnchorId(group.dateKey, song, sIdx, group.songs)"
              class="frameless-song-row"
              :class="{ active: store.currentSong?.hash === song.hash }"
              @click="store.playSongFromList(song, sortedNewSongsAll)"
              @contextmenu.prevent="store.openSongContextMenu($event, song, sortedNewSongsAll)"
            >
              <span class="fs-letter-badge">{{ extractFirstLetter(song.title) }}</span>
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
                  <span v-if="song.hasHiRes" class="fs-badge hires">Hi-Res</span>
                  <span v-else-if="song.hasSq" class="fs-badge sq">SQ</span>
                  <span v-if="song.isVip" class="fs-badge vip">VIP</span>
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
                <button
                  class="fs-icon-btn"
                  @click="store.addToPlaylistModalSong = song"
                >
                  <PlumpIcon name="plus" :size="16" />
                </button>
              </div>
              <span class="fs-duration">{{ formatDuration(song.durationSeconds) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ==================== 二级页面 B：榜单详情页 ==================== -->
    <div v-else-if="store.discoverSubView === 'rank'" class="discover-subpage frameless-subpage">
      <div class="subpage-frameless-header">
        <div class="sfh-left">
          <button class="frameless-hover-btn" @click="store.discoverSubView = 'home'">
            <PlumpIcon name="back" :size="17" />
            <span>返回发现</span>
          </button>
          <img
            v-if="store.activeRank?.coverUrl"
            :src="store.activeRank.coverUrl"
            class="sfh-rank-cover"
            alt=""
          />
          <div>
            <h2 class="sfh-title">{{ store.activeRank?.name || "巅峰排行榜" }}</h2>
            <div class="sfh-sub">
              {{ store.activeRank?.updateFrequency || "官方实时更新" }} · 共 {{ store.activeRankSongs.length }} 首上榜曲目
            </div>
          </div>
        </div>

        <div class="sfh-right">
          <div class="frameless-sublist-search">
            <PlumpIcon name="search" :size="15" />
            <input
              v-model="store.rankSearchKeyword"
              type="text"
              placeholder="筛选榜单内歌曲 / 歌手..."
            />
          </div>
          <button
            class="frameless-hover-btn"
            @click="store.isRankGridModalOpen = true"
          >
            <PlumpIcon name="grid" :size="16" />
            <span>切换榜单</span>
          </button>
          <button
            class="frameless-hover-btn primary-tint"
            @click="playAllActiveRank"
          >
            <PlumpIcon name="play" :size="16" />
            <span>播放全部</span>
          </button>
        </div>
      </div>

      <div class="frameless-songs-grid-2col">
        <div
          v-for="(song, idx) in filteredRankSongs"
          :key="song.hash + '_' + idx"
          class="frameless-song-row"
          :class="{ active: store.currentSong?.hash === song.hash }"
          @click="store.playSongFromList(song, store.activeRankSongs)"
          @contextmenu.prevent="store.openSongContextMenu($event, song, store.activeRankSongs)"
        >
          <span class="fs-index" :class="idx < 3 ? 'rank-top-' + (idx + 1) : ''">
            {{ idx + 1 }}
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
              <span v-if="song.hasHiRes" class="fs-badge hires">Hi-Res</span>
              <span v-else-if="song.hasSq" class="fs-badge sq">SQ</span>
              <span v-if="song.isVip" class="fs-badge vip">VIP</span>
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
          <span class="fs-duration">{{ formatDuration(song.durationSeconds) }}</span>
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
            <PlumpIcon name="refresh" :size="16" />
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
                <span v-if="song.hasHiRes" class="fs-badge hires">Hi-Res</span>
                <span v-else-if="song.hasSq" class="fs-badge sq">SQ</span>
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
            <span class="fs-duration">{{ formatDuration(song.durationSeconds) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 全部榜单网格选择弹窗（不显示榜单数量，采用重构后的榜单卡片） -->
    <transition name="fade-pop">
      <div
        v-if="store.isRankGridModalOpen"
        class="modal-backdrop"
        @click.self="store.isRankGridModalOpen = false"
      >
        <div class="rank-all-modal glass-popover">
          <div class="modal-top-bar">
            <h3>全部官方巅峰榜单</h3>
            <button class="frameless-hover-btn" @click="store.isRankGridModalOpen = false">
              <PlumpIcon name="close" :size="16" />
            </button>
          </div>
          <div class="rank-modal-grid">
            <div
              v-for="(rk, rIdx) in store.rankList"
              :key="rk.rankId"
              class="rank-bento-card"
              :class="`rank-accent-${(rIdx % 4) + 1}`"
              @click="store.openRankDetail(rk)"
            >
              <div class="rbc-cover-stage sm">
                <img :src="rk.coverUrl" class="rbc-cover-img" loading="lazy" draggable="false" alt="" />
                <button
                  class="rbc-play-fab"
                  @click.stop="store.playRankDirectly(rk)"
                >
                  <PlumpIcon name="play" :size="14" color="#fff" />
                </button>
              </div>
              <div class="rbc-body">
                <div class="rbc-head-row">
                  <h4 class="rbc-title">{{ rk.name }}</h4>
                </div>
                <span class="rbc-freq-pill" style="align-self: flex-start">
                  {{ rk.updateFrequency || "每日更新" }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useAppStore } from "../stores/appStore";
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

const newSongSearchKw = ref("");

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
  const start = new Date(end.getTime() - 6 * 86400000);
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

// 点击日推卡片：未展开则展开，已展开则直接收起返回发现页
function handleDailyCardClick() {
  if (isDailyExpandedMode.value) {
    collapseToDiscoverHome();
    return;
  }
  heroSongFilter.value = "";
  store.openDailyRecommendPage();
}

// 点击猜你喜欢卡片：未展开则展开，已展开则直接收起返回发现页
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

// ==================== 新歌速递 ====================
const newSongAreas = [
  { id: "CHINESE", label: "华语", color: "#2b8cff" },
  { id: "WESTERN", label: "欧美", color: "#ff6b6b" },
  { id: "JAPAN_KOREA", label: "日韩", color: "#15c493" },
  { id: "CANTONESE", label: "粤语", color: "#f5a623" },
  { id: "ALL", label: "全部", color: "#8e75ff" },
];

const COLLATOR_LETTERS = "ABCDEFGHJKLMNOPQRSTWXYZ".split("");
const COLLATOR_BOUNDARIES = "阿八嚓哒妸发旮哈讥咔垃妈拿噢啪期然撒塌挖昔压匝".split("");

function extractFirstLetter(text: string): string {
  const trimmed = (text || "").trim();
  if (!trimmed) return "#";
  const firstChar = trimmed[0].toUpperCase();
  if (firstChar >= "A" && firstChar <= "Z") return firstChar;
  if (firstChar >= "0" && firstChar <= "9") return "#";
  const code = firstChar.charCodeAt(0);
  if (code >= 0x4e00 && code <= 0x9fa5) {
    for (let i = COLLATOR_BOUNDARIES.length - 1; i >= 0; i--) {
      if (firstChar.localeCompare(COLLATOR_BOUNDARIES[i], "zh-CN") >= 0) {
        return COLLATOR_LETTERS[i];
      }
    }
  }
  return "#";
}

function normalizeDateKey(raw: string, fallbackIndex: number): string {
  const cleaned = (raw || "").trim();
  if (/^\d{4}-\d{2}-\d{2}/.test(cleaned)) {
    return cleaned.slice(0, 10);
  }
  const offsetDays = Math.min(6, Math.floor(fallbackIndex / 5));
  const d = new Date(Date.now() - offsetDays * 86400000);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function formatShortPublishDate(raw: string): string {
  if (!raw || raw.length < 10) return "最新上架";
  return raw.slice(5, 10).replace("-", ".");
}

const sortedNewSongsAll = computed<Song[]>(() => {
  const list = store.newSongs.map((s, idx) => ({
    ...s,
    publishDate: normalizeDateKey(s.publishDate, idx),
  }));
  return list.sort((a, b) => {
    if (a.publishDate !== b.publishDate) {
      return b.publishDate.localeCompare(a.publishDate);
    }
    const la = extractFirstLetter(a.title);
    const lb = extractFirstLetter(b.title);
    if (la !== lb) {
      if (la === "#") return 1;
      if (lb === "#") return -1;
      return la.localeCompare(lb);
    }
    return a.title.localeCompare(b.title, "zh-CN");
  });
});

const sortedTop12NewSongs = computed(() => sortedNewSongsAll.value.slice(0, 12));

function playAllNewSongs() {
  if (sortedNewSongsAll.value.length > 0) {
    store.playSongFromList(sortedNewSongsAll.value[0], sortedNewSongsAll.value);
  }
}

const groupedNewSongsByDate = computed(() => {
  const kw = newSongSearchKw.value.trim().toLowerCase();
  const base = kw
    ? sortedNewSongsAll.value.filter(
        (s) =>
          s.title.toLowerCase().includes(kw) ||
          s.artist.toLowerCase().includes(kw)
      )
    : sortedNewSongsAll.value;

  const map = new Map<string, Song[]>();
  for (const s of base) {
    const d = s.publishDate || "最新发行";
    if (!map.has(d)) map.set(d, []);
    map.get(d)!.push(s);
  }
  const todayStr = normalizeDateKey("", 0);
  return Array.from(map.entries()).map(([dateKey, songs]) => {
    const letters = Array.from(new Set(songs.map((s) => extractFirstLetter(s.title))));
    return {
      dateKey,
      displayLabel: dateKey === todayStr ? `今日首发 · ${dateKey}` : `发行日 · ${dateKey}`,
      songs,
      letters,
    };
  });
});

function getFirstLetterAnchorId(dateKey: string, song: Song, idx: number, list: Song[]) {
  const letter = extractFirstLetter(song.title);
  if (idx === 0 || extractFirstLetter(list[idx - 1].title) !== letter) {
    return `ns-${dateKey}-${letter}`;
  }
  return undefined;
}

function scrollToNewSongLetter(dateKey: string, letter: string) {
  const el = document.getElementById(`ns-${dateKey}-${letter}`);
  if (el) el.scrollIntoView({ behavior: "smooth", block: "center" });
}

// ==================== 巅峰排行榜：结构化提取前三名歌名与歌手 ====================
const homeDoubleRowRanks = computed(() => store.rankList.slice(0, 8));

function getRankPreviewLines(rk: RankItem): string[] {
  if (rk.songPreviews && rk.songPreviews.length > 0) {
    return rk.songPreviews.slice(0, 3);
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
    const clean = (line || "").trim();
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

const filteredRankSongs = computed(() => {
  const kw = store.rankSearchKeyword.trim().toLowerCase();
  if (!kw) return store.activeRankSongs;
  return store.activeRankSongs.filter(
    (s) =>
      s.title.toLowerCase().includes(kw) ||
      s.artist.toLowerCase().includes(kw)
  );
});

function playAllActiveRank() {
  if (store.activeRankSongs.length > 0) {
    store.playSongFromList(store.activeRankSongs[0], store.activeRankSongs);
  }
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
