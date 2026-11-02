<template>
  <div class="discover-page">
    <!-- ==================== 顶部沉浸双雄舞台（首页 + 日推展开态 + 猜你喜欢展开态 共用同一套过渡 DOM） ==================== -->
    <div
      v-if="
        store.discoverSubView === 'home' ||
        store.discoverSubView === 'daily' ||
        store.discoverSubView === 'fm_detail'
      "
      class="discover-hero-flow-container"
    >
      <!-- 日推 & 猜你喜欢 横向可展开双卡舞台 -->
      <div
        class="bento-hero-stage"
        :class="{
          'mode-daily-expanded': isDailyExpandedMode,
          'mode-fm-expanded': isFmExpandedMode,
        }"
      >
        <!-- 左卡：每日推荐 30 首 (点击非按钮区域向右横向占满并化为页面标题栏) -->
        <div
          class="hero-immersive-card daily-immersive-card"
          :class="{
            'is-expanded-banner': isDailyExpandedMode,
            'is-collapsed-left': isFmExpandedMode,
          }"
          @click="handleDailyCardClick"
        >
          <div class="hero-ambient-glow daily-glow-a"></div>
          <div class="hero-ambient-glow daily-glow-b"></div>

          <div class="hero-card-inner">
            <!-- 复刻 Android 端 StaggeredThreeCovers 错落三封面 3D 立体堆叠 -->
            <div class="staggered-three-covers" :class="{ enlarged: isDailyExpandedMode }">
              <div class="stagger-cover-layer layer-left">
                <img
                  v-if="dailyCoverTriple[1]"
                  :src="dailyCoverTriple[1]"
                  alt="daily-left"
                />
              </div>
              <div class="stagger-cover-layer layer-right">
                <img
                  v-if="dailyCoverTriple[2]"
                  :src="dailyCoverTriple[2]"
                  alt="daily-right"
                />
              </div>
              <div class="stagger-cover-layer layer-center">
                <img
                  v-if="dailyCoverTriple[0]"
                  :src="dailyCoverTriple[0]"
                  alt="daily-center"
                />
                <div class="stagger-date-overlay">
                  <span class="stagger-month">{{ todayMonthEng }}</span>
                  <span class="stagger-day">{{ todayDayNum }}</span>
                </div>
              </div>
            </div>

            <!-- 卡片主体文案：全无框沉浸式设计，展示实用曲目与推荐基因信息 -->
            <div class="hero-card-copy">
              <div class="hero-eyebrow-line">
                <span class="hero-eyebrow-dot"></span>
                <span>CONCEPT DAILY · {{ todayDateLabel }} · 每日 06:00 焕新</span>
              </div>

              <div class="hero-title-row">
                <h2 class="hero-main-title">每日推荐</h2>
                <span class="hero-count-inline">{{ store.recommendSongs.length || 30 }} 首专属定制</span>
              </div>

              <p class="hero-subtitle-text">
                依据你的近期红心基因与完播偏好实时演算 · 帧享无损声学母带
              </p>

              <!-- 首页态显示前两首实时预览（无框纯文本流），展开态显示当前播放状态 -->
              <div v-if="!isDailyExpandedMode" class="hero-frameless-previews">
                <div
                  v-for="(s, idx) in store.recommendSongs.slice(0, 2)"
                  :key="s.hash"
                  class="hero-preview-line"
                >
                  <span class="preview-note">♪</span>
                  <span class="preview-song">{{ s.title }}</span>
                  <span class="preview-sep">-</span>
                  <span class="preview-artist">{{ s.artist }}</span>
                  <span v-if="idx === 0 && s.hasHiRes" class="preview-tag">Hi-Res</span>
                </div>
              </div>
              <div v-else class="hero-expanded-meta-line">
                <span>正在浏览今日完整定制歌单 · 双击顶部「发现」可整体重算推荐池</span>
              </div>
            </div>

            <!-- 右侧按钮区：未展开时为立体大播放圆钮，展开后自然变化为页面级无框操作栏 -->
            <div class="hero-card-actions" @click.stop>
              <template v-if="!isDailyExpandedMode">
                <button
                  class="hero-stereo-play-fab"
                  title="立即播放今日全部推荐"
                  @click.stop="playAllDaily"
                >
                  <PlumpIcon name="play" :size="22" />
                </button>
              </template>
              <template v-else>
                <div class="hero-banner-action-group">
                  <button
                    class="frameless-hover-btn primary-tint"
                    @click.stop="playAllDaily"
                  >
                    <PlumpIcon name="play" :size="16" />
                    <span>播放全部 ({{ store.recommendSongs.length }})</span>
                  </button>
                  <button
                    class="frameless-hover-btn"
                    :disabled="store.isLoadingRecommend"
                    @click.stop="store.loadDailyRecommend()"
                  >
                    <PlumpIcon name="refresh" :size="16" />
                    <span>{{ store.isLoadingRecommend ? "刷新中..." : "刷新日推" }}</span>
                  </button>
                  <button
                    v-if="isCurrentSongInList(store.recommendSongs)"
                    class="frameless-hover-btn"
                    @click.stop="scrollToCurrentPlayingSong"
                  >
                    <PlumpIcon name="compass" :size="16" />
                    <span>定位当前</span>
                  </button>
                  <button
                    class="frameless-hover-btn collapse-btn"
                    title="收起返回发现页"
                    @click.stop="collapseToDiscoverHome"
                  >
                    <PlumpIcon name="back" :size="16" />
                    <span>返回发现</span>
                  </button>
                </div>
              </template>
            </div>
          </div>
        </div>

        <!-- 右卡：猜你喜欢 · 私人漫游 (点击非按钮区域向左横向占满并化为页面标题栏) -->
        <div
          class="hero-immersive-card fm-immersive-card"
          :class="{
            'is-expanded-banner': isFmExpandedMode,
            'is-collapsed-right': isDailyExpandedMode,
          }"
          @click="handleFmCardClick"
        >
          <div class="hero-ambient-glow fm-glow-a"></div>
          <div class="hero-ambient-glow fm-glow-b"></div>

          <div class="hero-card-inner">
            <!-- 3D 立体黑胶与双层漫游封面堆叠 -->
            <div class="fm-stereo-visual" :class="{ enlarged: isFmExpandedMode }">
              <div class="fm-vinyl-disc" :class="{ spinning: isFmPlaying }">
                <div class="fm-vinyl-grooves"></div>
              </div>
              <div v-if="store.nextFmSong?.coverUrl" class="fm-rear-cover">
                <img :src="store.nextFmSong.coverUrl" alt="next-fm" />
              </div>
              <div class="fm-front-cover">
                <img
                  v-if="store.currentFmSong?.coverUrl"
                  :src="store.currentFmSong.coverUrl"
                  alt="current-fm"
                />
                <div v-else class="fm-cover-fallback">
                  <PlumpIcon name="radio" :size="32" color="#fff" />
                </div>
                <div v-if="isFmPlaying" class="fm-playing-wave">
                  <span></span><span></span><span></span><span></span>
                </div>
              </div>
            </div>

            <!-- 猜你喜欢无框沉浸信息区 -->
            <div class="hero-card-copy">
              <div class="hero-eyebrow-line fm-eyebrow">
                <span class="hero-eyebrow-dot"></span>
                <span>GUESS YOU LIKE · 私人声学漫游 · {{ store.personalFmSongs.length }} 首在列</span>
              </div>

              <div class="hero-title-row">
                <h2 class="hero-main-title">
                  {{ isFmExpandedMode ? "猜你喜欢 · 私人漫游" : (store.currentFmSong?.title || "猜你喜欢") }}
                </h2>
              </div>

              <p class="hero-subtitle-text fm-subtitle">
                <template v-if="!isFmExpandedMode && store.currentFmSong">
                  {{ store.currentFmSong.artist }}
                  <span v-if="store.currentFmSong.album"> · 《{{ store.currentFmSong.album }}》</span>
                </template>
                <template v-else>
                  不设限私人曲库 · 基于听歌偏好全天候无缝探索未知好歌
                </template>
              </p>

              <div class="hero-frameless-previews">
                <div v-if="store.nextFmSong" class="hero-preview-line">
                  <span class="preview-note">⏭</span>
                  <span class="preview-artist">下一首漫游：</span>
                  <span class="preview-song">{{ store.nextFmSong.title }} - {{ store.nextFmSong.artist }}</span>
                </div>
                <div v-else class="hero-preview-line">
                  <span class="preview-note">✦</span>
                  <span class="preview-artist">点击卡片展开完整漫游歌单，或点击右侧按钮直接切歌</span>
                </div>
              </div>
            </div>

            <!-- 右侧按钮区：未展开时为无框悬浮播控图标组，展开后自然变化为页面级操作栏 -->
            <div class="hero-card-actions" @click.stop>
              <template v-if="!isFmExpandedMode">
                <div class="fm-compact-controls">
                  <button
                    class="hero-stereo-play-fab fm-fab"
                    :title="isFmPlaying ? '暂停漫游' : '立即播放猜你喜欢'"
                    @click.stop="toggleFmPlay"
                  >
                    <PlumpIcon :name="isFmPlaying ? 'pause' : 'play'" :size="21" />
                  </button>
                  <div class="fm-sub-icons">
                    <button
                      class="fm-frameless-icon-btn"
                      title="跳至下一首漫游"
                      @click.stop="store.nextPersonalFmTrack(true)"
                    >
                      <PlumpIcon name="next" :size="18" />
                    </button>
                    <button
                      v-if="store.currentFmSong"
                      class="fm-frameless-icon-btn"
                      :class="{ liked: store.isSongLiked(store.currentFmSong.hash) }"
                      title="收藏当前漫游歌曲"
                      @click.stop="store.toggleFavoriteSong(store.currentFmSong)"
                    >
                      <PlumpIcon
                        :name="store.isSongLiked(store.currentFmSong.hash) ? 'heart-filled' : 'heart'"
                        :size="17"
                      />
                    </button>
                    <button
                      class="fm-frameless-icon-btn"
                      title="换一批漫游推荐"
                      @click.stop="store.loadPersonalFm(false)"
                    >
                      <PlumpIcon name="refresh" :size="16" />
                    </button>
                  </div>
                </div>
              </template>
              <template v-else>
                <div class="hero-banner-action-group">
                  <button
                    class="frameless-hover-btn primary-tint"
                    @click.stop="playAllPersonalFm"
                  >
                    <PlumpIcon name="play" :size="16" />
                    <span>全播漫游 ({{ store.personalFmSongs.length }})</span>
                  </button>
                  <button
                    class="frameless-hover-btn"
                    @click.stop="store.nextPersonalFmTrack(true)"
                  >
                    <PlumpIcon name="next" :size="16" />
                    <span>下一首</span>
                  </button>
                  <button
                    class="frameless-hover-btn"
                    :disabled="store.isLoadingFm"
                    @click.stop="store.loadPersonalFm(false)"
                  >
                    <PlumpIcon name="refresh" :size="16" />
                    <span>换一批</span>
                  </button>
                  <button
                    class="frameless-hover-btn"
                    :disabled="store.isLoadingFm"
                    @click.stop="store.loadPersonalFm(true)"
                  >
                    <PlumpIcon name="plus" :size="16" />
                    <span>+20首续杯</span>
                  </button>
                  <button
                    class="frameless-hover-btn collapse-btn"
                    title="收起返回发现页"
                    @click.stop="collapseToDiscoverHome"
                  >
                    <PlumpIcon name="back" :size="16" />
                    <span>返回发现</span>
                  </button>
                </div>
              </template>
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
                共 {{ activeHeroSongs.length }} 首精选曲目 · 点击任意曲目开启无损播放，右键呼出更多操作
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

      <!-- ==================== 发现页下半部分主体内容（点击日推/猜你喜欢时虚化并向左消失） ==================== -->
      <div
        v-show="store.discoverSubView === 'home'"
        class="discover-home-lower"
        :class="{
          'lower-blur-exit': store.discoverHeroExpanding !== 'none',
        }"
      >
        <!-- 1. 新歌速递：分类标题沿用 App 端设计（含底部滑动横条）置于标题右侧；右侧刷新/全播/查看全部无框，悬停高亮；下方 3×4=12 首列优先无框网格 -->
        <section class="discover-block">
          <div class="block-header newsong-header">
            <div class="block-title-with-tabs">
              <h3 class="block-title">新歌速递</h3>
              <span class="block-sub-date">/ {{ newSongDateRangeLabel }}</span>

              <!-- 沿用 App 端 ConceptSlidingCategoryTabs（文字 + 底部滑动指示横条）置于标题右侧 -->
              <div class="app-sliding-tabs newsong-sliding-tabs">
                <button
                  v-for="(area, idx) in newSongAreas"
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

            <!-- 右侧：刷新、全播、查看全部（无框设计，鼠标悬停显示圆角方形高亮框） -->
            <div class="block-right-frameless-actions">
              <button
                class="frameless-hover-btn"
                :disabled="store.isLoadingNewSongs"
                title="刷新当前语种新歌"
                @click="store.loadNewSongExpress(store.newSongPeriod)"
              >
                <PlumpIcon name="refresh" :size="16" />
                <span>{{ store.isLoadingNewSongs ? "刷新中" : "刷新" }}</span>
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
                @click="store.discoverSubView = 'new_songs'"
              >
                <span>查看全部</span>
                <PlumpIcon name="chevron-right" :size="15" />
              </button>
            </div>
          </div>

          <!-- 下方歌曲：无框设计，嵌入背景，显示 3×4=12 首歌（从上到下再从左到右，按时间倒序+同时间按首字母排序） -->
          <div class="newsong-frameless-grid-3x4">
            <div
              v-for="(song, idx) in sortedTop12NewSongs"
              :key="song.hash + '_' + idx"
              class="newsong-frameless-item"
              :class="{ active: store.currentSong?.hash === song.hash }"
              @click="store.playSongFromList(song, sortedNewSongsAll)"
              @contextmenu.prevent="store.openSongContextMenu($event, song, sortedNewSongsAll)"
            >
              <span class="ns-col-rank">{{ String(idx + 1).padStart(2, "0") }}</span>

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

        <!-- 2. 巅峰排行榜（与电台互换位置后置于前方，双行显示，左侧放大封面+播放按钮，右侧紧凑榜单预览） -->
        <section class="discover-block">
          <div class="block-header">
            <div class="block-title-with-tabs">
              <h3 class="block-title">巅峰排行榜</h3>
              <span class="block-sub-date">/ 官方权威潮流风向标 · 双行矩阵</span>
            </div>
            <div class="block-right-frameless-actions">
              <button
                class="frameless-hover-btn"
                :disabled="store.isLoadingRanks"
                @click="store.loadRankList()"
              >
                <PlumpIcon name="refresh" :size="16" />
                <span>刷新榜单</span>
              </button>
              <button
                class="frameless-hover-btn"
                @click="store.isRankGridModalOpen = true"
              >
                <span>全部榜单 ({{ store.rankList.length }})</span>
                <PlumpIcon name="chevron-right" :size="15" />
              </button>
            </div>
          </div>

          <!-- 双行榜单卡片矩阵（2行 × 4列 = 8张卡片，卡片左侧大尺寸封面 + 独立播放按钮） -->
          <div class="rank-double-row-grid">
            <div
              v-for="rk in homeDoubleRowRanks"
              :key="rk.rankId"
              class="rank-horizontal-card"
              @click="store.openRankDetail(rk)"
            >
              <!-- 左侧增大榜单头像 + 悬浮播放按钮 -->
              <div class="rank-left-cover-wrap">
                <img :src="rk.coverUrl" class="rank-left-cover" loading="lazy" alt="" />
                <span class="rank-freq-badge">{{ rk.updateFrequency || "每日更新" }}</span>
                <button
                  class="rank-cover-play-btn"
                  title="直接播放该榜单曲目"
                  @click.stop="store.playRankDirectly(rk)"
                >
                  <PlumpIcon name="play" :size="17" color="#fff" />
                </button>
              </div>

              <!-- 右侧榜单标题与前三曲目预览 -->
              <div class="rank-right-body">
                <div class="rank-card-title-row">
                  <h4 class="rank-card-name">{{ rk.name }}</h4>
                  <span class="rank-card-cat">{{ rk.category || "官方榜" }}</span>
                </div>

                <div class="rank-card-previews">
                  <div
                    v-for="(line, lIdx) in getRankPreviewLines(rk)"
                    :key="lIdx"
                    class="rank-preview-item"
                  >
                    <span class="rp-num" :class="'top-' + (lIdx + 1)">{{ lIdx + 1 }}</span>
                    <span class="rp-text">{{ line }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- 3. 精选场景电台（置于最后，获取真实封面并单行显示，删除分类标题栏，右上角改为“电台广场”二级页入口） -->
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

          <!-- 单行显示带封面的电台卡片 -->
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
                    :size="16"
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

    <!-- ==================== 二级页面 A：新歌速递 · 查看全部（全无框设计） ==================== -->
    <div v-else-if="store.discoverSubView === 'new_songs'" class="discover-subpage frameless-subpage">
      <div class="subpage-frameless-header">
        <div class="sfh-left">
          <button class="frameless-hover-btn" @click="store.discoverSubView = 'home'">
            <PlumpIcon name="back" :size="17" />
            <span>返回发现</span>
          </button>
          <h2 class="sfh-title">新歌速递 · 全部曲库</h2>

          <!-- App 端风格滑动分类栏 -->
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
            class="frameless-hover-btn"
            @click="store.loadNewSongExpress(store.newSongPeriod)"
          >
            <PlumpIcon name="refresh" :size="16" />
            <span>刷新</span>
          </button>
          <button class="frameless-hover-btn primary-tint" @click="playAllNewSongs">
            <PlumpIcon name="play" :size="16" />
            <span>播放全部 ({{ sortedNewSongsAll.length }})</span>
          </button>
        </div>
      </div>

      <!-- 按发行日期倒序 + 同日按首字母无框分组列表 -->
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

    <!-- ==================== 二级页面 B：榜单详情页（全无框歌曲列表） ==================== -->
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

    <!-- ==================== 二级页面 C：全新重构的“电台广场”（二级页面，不再是一级导航） ==================== -->
    <div v-else-if="store.discoverSubView === 'radio'" class="discover-subpage radio-plaza-page">
      <!-- 顶部无框导航与 App 端风格滑动分类栏 -->
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

      <!-- 电台广场主舞台：左侧当前锁定电台 3D 播控舱 + 右侧全封面电台矩阵 -->
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

        <!-- 分类电台封面墙 -->
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

      <!-- 当前电台实时漫游歌曲流（无框双列） -->
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

    <!-- 全部榜单网格选择弹窗 -->
    <transition name="fade-pop">
      <div
        v-if="store.isRankGridModalOpen"
        class="modal-backdrop"
        @click.self="store.isRankGridModalOpen = false"
      >
        <div class="rank-all-modal glass-popover">
          <div class="modal-top-bar">
            <h3>全部官方巅峰榜单 ({{ store.rankList.length }})</h3>
            <button class="frameless-hover-btn" @click="store.isRankGridModalOpen = false">
              <PlumpIcon name="close" :size="16" />
            </button>
          </div>
          <div class="rank-modal-grid">
            <div
              v-for="rk in store.rankList"
              :key="rk.rankId"
              class="rank-horizontal-card"
              @click="store.openRankDetail(rk)"
            >
              <div class="rank-left-cover-wrap">
                <img :src="rk.coverUrl" class="rank-left-cover" loading="lazy" alt="" />
                <button
                  class="rank-cover-play-btn"
                  @click.stop="store.playRankDirectly(rk)"
                >
                  <PlumpIcon name="play" :size="16" color="#fff" />
                </button>
              </div>
              <div class="rank-right-body">
                <h4 class="rank-card-name">{{ rk.name }}</h4>
                <span class="rank-card-cat">{{ rk.updateFrequency || "每日更新" }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import type { FmChannel, RankItem, Song } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

const heroSongFilter = ref("");
const newSongSearchKw = ref("");

// 判断是否处于日推或猜你喜欢的横向占满模式
const isDailyExpandedMode = computed(
  () => store.discoverHeroExpanding === "daily" || store.discoverSubView === "daily"
);
const isFmExpandedMode = computed(
  () => store.discoverHeroExpanding === "fm" || store.discoverSubView === "fm_detail"
);

// 日期文案
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

// 复刻 Android StaggeredThreeCovers 的三封面源
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

const isFmPlaying = computed(
  () =>
    Boolean(
      store.isPlaying &&
        store.currentSong &&
        store.currentFmSong &&
        store.currentSong.hash === store.currentFmSong.hash
    )
);

// 点击日推卡片（非按钮区域）：猜你喜欢向右消失，日推向右扩展占满，下方虚化左移，浮现日推歌曲
function handleDailyCardClick() {
  if (isDailyExpandedMode.value) return;
  heroSongFilter.value = "";
  store.openDailyRecommendPage();
}

// 点击猜你喜欢卡片（非按钮区域）：日推向左消失，猜你喜欢向左扩展占满，下方虚化左移，浮现猜你喜欢歌曲
function handleFmCardClick() {
  if (isFmExpandedMode.value) return;
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

function playAllPersonalFm() {
  if (store.personalFmSongs.length > 0) {
    store.playSongFromList(store.personalFmSongs[0], store.personalFmSongs);
  } else {
    store.loadPersonalFm(false);
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

// ==================== 新歌速递：分类栏 + 时间顺序&同时间按首字母排序 + 3×4=12首列优先 ====================
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

// 严格按时间顺序（最新在前），同时间按首字母排序
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

// 首页展示 3×4 = 12 首（若不足 12 首则按实际数量展示）
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

// ==================== 巅峰排行榜：双行显示（2行 × 4列 = 8个榜单） ====================
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

// ==================== 精选场景电台 & 电台广场（单行封面显示 + 独立二级电台广场） ====================
const singleRowFmChannels = computed(() => store.fmChannels.slice(0, 6));

const radioCoverFallbackPool = computed(() => {
  const pool: string[] = [];
  for (const s of store.recommendSongs) {
    if (s.coverUrl && !pool.includes(s.coverUrl)) pool.push(s.coverUrl);
  }
  for (const s of store.newSongs) {
    if (s.coverUrl && !pool.includes(s.coverUrl)) pool.push(s.coverUrl);
  }
  for (const r of store.rankList) {
    if (r.coverUrl && !pool.includes(r.coverUrl)) pool.push(r.coverUrl);
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
