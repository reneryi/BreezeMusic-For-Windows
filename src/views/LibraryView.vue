<template>
  <!-- 我的页：全屏无裁剪渲染，仅划分为左右两个内容展示区与各自独立的滚动区域 -->
  <div class="library-page library-fixed-viewport" @contextmenu.prevent>
    <!-- ==================== 顶部三张核心卡片（高度降低 10% 至 154px，极简大气排版，持久 DOM 丝滑横向展开） ==================== -->
    <section
      class="library-hero-trio enlarged-trio"
      :class="{
        'is-liked-expanded': expandedTopCard === 'liked',
        'is-recent-expanded': expandedTopCard === 'recent',
      }"
    >
      <!-- 1. 我喜欢的音乐（左卡：点击向右平滑占满，右侧两卡自然向右退场） -->
      <div
        class="lib-stereo-card fav-stereo-card"
        :class="{
          active: rightStageMode === 'liked',
          'is-expanded-full': expandedTopCard === 'liked',
          'is-vanished-left': expandedTopCard === 'recent',
        }"
        @click="activateLikedMode"
      >
        <div
          v-if="store.likedSongs[0]?.coverUrl"
          class="hero-cover-blur-bg"
          :style="{ backgroundImage: `url(${store.likedSongs[0].coverUrl})` }"
        ></div>
        <div class="lib-card-ambient-orb coral-orb"></div>

        <!-- 左侧常驻主体（封面叠层 + 简洁标题与曲目信息，展开时不销毁 DOM，平滑过渡） -->
        <div class="lib-card-main-left">
          <div class="lib-card-visual" :class="{ 'visual-expanded': expandedTopCard === 'liked' }">
            <div v-if="store.likedSongs[2]?.coverUrl" class="lib-visual-third">
              <img :src="store.likedSongs[2].coverUrl" draggable="false" alt="" />
            </div>
            <div v-if="store.likedSongs[1]?.coverUrl" class="lib-visual-back">
              <img :src="store.likedSongs[1].coverUrl" draggable="false" alt="" />
            </div>
            <div class="lib-visual-front">
              <img
                v-if="store.likedSongs[0]?.coverUrl"
                :src="store.likedSongs[0].coverUrl"
                draggable="false"
                alt=""
              />
              <div v-else class="lib-visual-placeholder coral">
                <PlumpIcon name="heart-filled" :size="28" color="#fff" />
              </div>
              <span class="lib-visual-corner-tag coral">
                <PlumpIcon name="heart-filled" :size="12" color="#fff" />
              </span>
            </div>
          </div>

          <div class="lib-card-copy">
            <div class="lib-card-eyebrow">FAVORITES · 红心曲库</div>
            <div class="lib-card-title-row">
              <h3 class="lib-card-title">我喜欢的音乐</h3>
              <span class="lib-card-count-pill">{{ store.likedSongs.length }}</span>
            </div>

            <!-- 简洁大气的副信息行：常态展示代表曲目，展开态展示纯粹的声学统计 -->
            <div class="lib-card-meta-stream">
              <template v-if="expandedTopCard === 'liked'">
                <span class="lcm-stat">{{ store.likedSongs.length }} 首珍藏曲目</span>
                <template v-if="likedHiResCount > 0">
                  <span class="lcm-dot">·</span>
                  <span class="lcm-stat highlight">{{ likedHiResCount }} 首无损 / Hi-Res</span>
                </template>
                <span class="lcm-dot">·</span>
                <span class="lcm-stat">{{ likedTotalDurationText }}</span>
              </template>
              <template v-else>
                <div v-if="store.likedSongs[0]" class="lib-preview-line">
                  <span class="lp-song">{{ store.likedSongs[0].title }}</span>
                  <span class="lp-artist">— {{ store.likedSongs[0].artist }}</span>
                </div>
                <div v-if="store.likedSongs[1]" class="lib-preview-line secondary">
                  <span class="lp-song">{{ store.likedSongs[1].title }}</span>
                  <span class="lp-artist">— {{ store.likedSongs[1].artist }}</span>
                </div>
                <div v-if="!store.likedSongs[0]" class="lib-preview-line secondary">
                  <span>双端同步红心珍藏</span>
                </div>
              </template>
            </div>
          </div>
        </div>

        <!-- 展开态平滑浮现的精选曲目画廊（持久存在于 DOM 中，随卡片展开自然滑入） -->
        <div
          class="lib-card-expanded-gallery"
          :class="{ 'is-visible': expandedTopCard === 'liked' }"
          @click.stop
        >
          <div
            v-for="song in store.likedSongs.slice(0, 3)"
            :key="song.hash"
            class="lib-gallery-track-pill"
            :class="{ playing: store.currentSong?.hash === song.hash }"
            @click.stop="store.playSongFromList(song, store.likedSongs)"
          >
            <img :src="song.coverUrl" class="lgtp-cover" draggable="false" alt="" />
            <div class="lgtp-info">
              <div class="lgtp-title">{{ song.title }}</div>
              <div class="lgtp-artist">{{ song.artist }}</div>
            </div>
          </div>
        </div>

        <!-- 右侧极简控制区 -->
        <div class="lib-card-right-zone" @click.stop>
          <button
            v-if="expandedTopCard === 'liked'"
            class="lib-card-glass-btn"
            title="随机播放我喜欢的音乐"
            @click.stop="shufflePlayLiked"
          >
            <PlumpIcon name="shuffle" :size="15" color="#fff" />
            <span>随机漫游</span>
          </button>
          <button
            v-if="expandedTopCard === 'liked'"
            class="lib-card-icon-orb"
            title="收起展开态"
            @click.stop="collapseExpandedTopCard"
          >
            <PlumpIcon name="close" :size="14" color="#fff" />
          </button>
          <button
            class="lib-card-play-fab coral"
            title="立即播放我喜欢的音乐"
            @click.stop="playAllLiked"
          >
            <PlumpIcon name="play" :size="20" color="#fff" />
          </button>
        </div>
      </div>

      <!-- 2. 最近播放（中卡：点击向两侧平滑占满，左卡向左退场、右卡向右退场） -->
      <div
        class="lib-stereo-card recent-stereo-card"
        :class="{
          active: rightStageMode === 'recent',
          'is-expanded-full': expandedTopCard === 'recent',
          'is-vanished-right': expandedTopCard === 'liked',
        }"
        @click="activateRecentMode"
      >
        <div
          v-if="store.recentHistorySongs[0]?.coverUrl"
          class="hero-cover-blur-bg"
          :style="{ backgroundImage: `url(${store.recentHistorySongs[0].coverUrl})` }"
        ></div>
        <div class="lib-card-ambient-orb cyan-orb"></div>

        <div class="lib-card-main-left">
          <div class="lib-card-visual" :class="{ 'visual-expanded': expandedTopCard === 'recent' }">
            <div v-if="store.recentHistorySongs[2]?.coverUrl" class="lib-visual-third">
              <img :src="store.recentHistorySongs[2].coverUrl" draggable="false" alt="" />
            </div>
            <div v-if="store.recentHistorySongs[1]?.coverUrl" class="lib-visual-back">
              <img :src="store.recentHistorySongs[1].coverUrl" draggable="false" alt="" />
            </div>
            <div class="lib-visual-front">
              <img
                v-if="store.recentHistorySongs[0]?.coverUrl"
                :src="store.recentHistorySongs[0].coverUrl"
                draggable="false"
                alt=""
              />
              <div v-else class="lib-visual-placeholder blue">
                <PlumpIcon name="timer" :size="28" color="#fff" />
              </div>
              <span class="lib-visual-corner-tag blue">
                <PlumpIcon name="timer" :size="12" color="#fff" />
              </span>
            </div>
          </div>

          <div class="lib-card-copy">
            <div class="lib-card-eyebrow">RECENT PLAYED · 听歌足迹</div>
            <div class="lib-card-title-row">
              <h3 class="lib-card-title">最近播放</h3>
              <span class="lib-card-count-pill">{{ store.recentHistorySongs.length }}</span>
            </div>

            <div class="lib-card-meta-stream">
              <template v-if="expandedTopCard === 'recent'">
                <span class="lcm-stat">{{ store.recentHistorySongs.length }} 首近期记录</span>
                <template v-if="recentHiResCount > 0">
                  <span class="lcm-dot">·</span>
                  <span class="lcm-stat highlight">{{ recentHiResCount }} 首无损 / Hi-Res</span>
                </template>
                <span class="lcm-dot">·</span>
                <span class="lcm-stat">{{ recentTotalDurationText }}</span>
              </template>
              <template v-else>
                <div v-if="store.recentHistorySongs[0]" class="lib-preview-line">
                  <span class="lp-song">{{ store.recentHistorySongs[0].title }}</span>
                  <span class="lp-artist">— {{ store.recentHistorySongs[0].artist }}</span>
                </div>
                <div v-if="store.recentHistorySongs[1]" class="lib-preview-line secondary">
                  <span class="lp-song">{{ store.recentHistorySongs[1].title }}</span>
                  <span class="lp-artist">— {{ store.recentHistorySongs[1].artist }}</span>
                </div>
                <div v-if="!store.recentHistorySongs[0]" class="lib-preview-line secondary">
                  <span>自动同步本机与云端播放轨迹</span>
                </div>
              </template>
            </div>
          </div>
        </div>

        <!-- 展开态平滑浮现的最近播放画廊 -->
        <div
          class="lib-card-expanded-gallery"
          :class="{ 'is-visible': expandedTopCard === 'recent' }"
          @click.stop
        >
          <div
            v-for="song in store.recentHistorySongs.slice(0, 3)"
            :key="song.hash"
            class="lib-gallery-track-pill"
            :class="{ playing: store.currentSong?.hash === song.hash }"
            @click.stop="store.playSongFromList(song, store.recentHistorySongs)"
          >
            <img :src="song.coverUrl" class="lgtp-cover" draggable="false" alt="" />
            <div class="lgtp-info">
              <div class="lgtp-title">{{ song.title }}</div>
              <div class="lgtp-artist">{{ song.artist }}</div>
            </div>
          </div>
        </div>

        <div class="lib-card-right-zone" @click.stop>
          <button
            v-if="expandedTopCard === 'recent'"
            class="lib-card-glass-btn"
            title="随机播放最近听过"
            @click.stop="shufflePlayRecent"
          >
            <PlumpIcon name="shuffle" :size="15" color="#fff" />
            <span>随机漫游</span>
          </button>
          <button
            v-if="expandedTopCard === 'recent'"
            class="lib-card-icon-orb"
            title="收起展开态"
            @click.stop="collapseExpandedTopCard"
          >
            <PlumpIcon name="close" :size="14" color="#fff" />
          </button>
          <button
            class="lib-card-play-fab blue"
            title="立即播放最近听过"
            @click.stop="playAllRecent"
          >
            <PlumpIcon name="play" :size="20" color="#fff" />
          </button>
        </div>
      </div>

      <!-- 3. 心流舱 · Focus Pod（右卡：独立表现形式，左/中卡展开时自然向右退场） -->
      <div
        class="lib-stereo-card focus-stereo-card"
        :class="{
          'is-vanished-right': expandedTopCard === 'liked' || expandedTopCard === 'recent',
        }"
        @click="store.isFocusPodOpen = true"
      >
        <div class="lib-card-ambient-orb mint-orb"></div>

        <div class="lib-card-main-left">
          <div class="lib-card-visual">
            <div class="lib-visual-front focus-orb-visual">
              <div class="focus-orb-ring"></div>
              <div class="focus-orb-ring inner"></div>
              <PlumpIcon name="sparkles" :size="26" color="#fff" />
            </div>
          </div>

          <div class="lib-card-copy">
            <div class="lib-card-eyebrow">FOCUS POD · 概念专注舱</div>
            <div class="lib-card-title-row">
              <h3 class="lib-card-title">心流舱</h3>
              <span class="lib-card-count-pill">白噪音 × 番茄钟</span>
            </div>
            <div class="lib-card-meta-stream">
              <div class="lib-preview-line">
                <span class="lp-song">雨声 · 潮汐 · 篝火 · 黑胶底噪</span>
              </div>
              <div class="lib-preview-line secondary">
                <span>四轨环境声混音与专注计时</span>
              </div>
            </div>
          </div>
        </div>

        <div class="lib-card-right-zone" @click.stop>
          <button
            class="lib-card-play-fab mint"
            title="开启概念心流舱"
            @click.stop="store.isFocusPodOpen = true"
          >
            <PlumpIcon name="compass" :size="20" color="#fff" />
          </button>
        </div>
      </div>
    </section>

    <!-- ==================== 分类标题与工具栏：已购右侧新增收起/展开左侧栏按钮，右侧删除多余提示文字 ==================== -->
    <div class="library-category-header-bar">
      <div class="lib-category-left-group">
        <div class="app-sliding-tabs lib-horizontal-tabs">
          <button
            v-for="tab in assetTabs"
            :key="tab.id"
            class="app-sliding-tab-btn"
            :class="{ active: store.libraryAssetTab === tab.id }"
            @click="handleSelectAssetTab(tab.id)"
          >
            <span class="tab-label">{{ tab.label }}</span>
            <span class="tab-count">{{ tab.count }}</span>
            <span
              class="tab-bottom-indicator"
              :style="{
                backgroundColor: tab.color,
                opacity: store.libraryAssetTab === tab.id ? 1 : 0,
                transform: store.libraryAssetTab === tab.id ? 'scaleX(1)' : 'scaleX(0.2)',
              }"
            ></span>
          </button>
        </div>

        <!-- 在「已购」右边新增收起/展开按钮：收起后左侧仅显示小封面/小头像图标，右侧歌曲列表自然伸展 -->
        <button
          class="lib-collapse-sidebar-btn"
          :class="{ collapsed: isLeftSidebarCollapsed }"
          :title="isLeftSidebarCollapsed ? '展开左侧歌单与歌手列表' : '收起左侧列表为小图标栏'"
          @click="toggleLeftSidebarCollapse"
        >
          <PlumpIcon
            :name="isLeftSidebarCollapsed ? 'chevron-right' : 'arrow-left'"
            :size="15"
          />
          <span>{{ isLeftSidebarCollapsed ? "展开" : "收起" }}</span>
        </button>
      </div>

      <!-- 最右侧：创建歌单、导入外部歌单、视图切换 纯 UI 图标按钮（已删除“长按可拖动排序”字样） -->
      <div class="lib-top-right-icon-tools">
        <button
          class="frameless-icon-square-btn"
          title="新建自建歌单"
          @click="openCreateModal"
        >
          <PlumpIcon name="plus" :size="18" />
        </button>
        <button
          class="frameless-icon-square-btn"
          title="导入外部歌单链接或酷狗歌单码"
          @click="openImportModal"
        >
          <PlumpIcon name="upload" :size="18" />
        </button>
        <button
          v-if="!isLeftSidebarCollapsed"
          class="frameless-icon-square-btn"
          :title="store.isLibraryGridView ? '切换左侧歌单为列表视图' : '切换左侧歌单为宫格视图'"
          @click="store.isLibraryGridView = !store.isLibraryGridView"
        >
          <PlumpIcon :name="store.isLibraryGridView ? 'list' : 'grid'" :size="18" />
        </button>
      </div>
    </div>

    <!-- ==================== 全屏双区展示舞台（无硬边界截断，左右两区各自独立上下滚动，左侧收起时右侧自然伸展） ==================== -->
    <section
      class="library-dual-independent-stage"
      :class="{ 'is-left-collapsed': isLeftSidebarCollapsed }"
    >
      <!-- 左侧独立滑动区域（支持展开态宫格/列表 & 收起态紧凑小封面/小头像图标栏） -->
      <aside
        class="library-left-frameless-region"
        :class="{ 'is-collapsed-rail': isLeftSidebarCollapsed }"
        @contextmenu.prevent.stop
      >
        <!-- ==================== 形态 1：左侧收起态（根据当前分类标题仅显示小封面或歌手小头像） ==================== -->
        <template v-if="isLeftSidebarCollapsed">
          <!-- A. 自建 / 收藏歌单小封面图标列 -->
          <template v-if="store.libraryAssetTab === 'created' || store.libraryAssetTab === 'collected'">
            <TransitionGroup
              name="lib-drag-flip"
              tag="div"
              class="lib-collapsed-mini-rail"
            >
              <div
                v-for="(pl, idx) in activePlaylistGroup"
                :key="pl.id || pl.title"
                :data-lib-drag-idx="idx"
                class="lib-mini-rail-item"
                :class="{
                  active: isPlaylistHighlighted(pl),
                  'is-dragging-item': dragState.active && dragState.fromIdx === idx,
                  'is-drag-over-item': dragState.active && dragState.overIdx === idx && dragState.fromIdx !== idx,
                }"
                :title="`${pl.name || pl.title} (${pl.trackCount}首)`"
                @mousedown="onLeftItemMouseDown($event, idx)"
                @click="onLeftItemClick(() => selectPlaylistToRight(pl))"
                @contextmenu.prevent.stop="onLeftPlaylistRightClick(pl)"
              >
                <img
                  :src="pl.coverUrl"
                  class="lib-mini-rail-img"
                  loading="lazy"
                  draggable="false"
                  alt=""
                  @contextmenu.prevent.stop
                />
              </div>
            </TransitionGroup>
          </template>

          <!-- B. 关注歌手小头像图标列 -->
          <template v-else-if="store.libraryAssetTab === 'artists'">
            <TransitionGroup
              name="lib-drag-flip"
              tag="div"
              class="lib-collapsed-mini-rail"
            >
              <div
                v-for="(art, idx) in store.followedArtists"
                :key="art.singerId || art.artistId || art.name"
                :data-lib-drag-idx="idx"
                class="lib-mini-rail-item round"
                :class="{
                  active: isArtistHighlighted(art),
                  'is-dragging-item': dragState.active && dragState.fromIdx === idx,
                  'is-drag-over-item': dragState.active && dragState.overIdx === idx && dragState.fromIdx !== idx,
                }"
                :title="art.name"
                @mousedown="onLeftItemMouseDown($event, idx)"
                @click="onLeftItemClick(() => selectArtistToRight(art))"
                @contextmenu.prevent.stop
              >
                <img
                  :src="art.avatarUrl"
                  class="lib-mini-rail-img round"
                  loading="lazy"
                  draggable="false"
                  alt=""
                  @contextmenu.prevent.stop
                />
              </div>
            </TransitionGroup>
          </template>

          <!-- C. 已购音乐小封面图标列 -->
          <template v-else-if="store.libraryAssetTab === 'purchased'">
            <TransitionGroup
              name="lib-drag-flip"
              tag="div"
              class="lib-collapsed-mini-rail"
            >
              <div
                v-for="(entry, idx) in purchasedCombinedEntries"
                :key="entry.key"
                :data-lib-drag-idx="idx"
                class="lib-mini-rail-item"
                :class="{
                  active: isPurchasedEntryHighlighted(entry),
                  'is-dragging-item': dragState.active && dragState.fromIdx === idx,
                  'is-drag-over-item': dragState.active && dragState.overIdx === idx && dragState.fromIdx !== idx,
                }"
                :title="entry.kind === 'songs' ? `已购尊享单曲合集 (${store.purchasedSongs.length}首)` : `${entry.album?.albumName || ''}`"
                @mousedown="onLeftItemMouseDown($event, idx)"
                @click="onLeftItemClick(() => selectPurchasedEntryToRight(entry))"
                @contextmenu.prevent.stop
              >
                <div v-if="entry.kind === 'songs'" class="lib-mini-rail-fallback">
                  <PlumpIcon name="sparkles" :size="17" color="#fff" />
                </div>
                <img
                  v-else-if="entry.album"
                  :src="entry.album.coverUrl"
                  class="lib-mini-rail-img"
                  loading="lazy"
                  draggable="false"
                  alt=""
                  @contextmenu.prevent.stop
                />
              </div>
            </TransitionGroup>
          </template>
        </template>

        <!-- ==================== 形态 2：左侧常规展开态 ==================== -->
        <template v-else>
          <!-- A. 自建歌单 / 收藏歌单 -->
          <template v-if="store.libraryAssetTab === 'created' || store.libraryAssetTab === 'collected'">
            <div v-if="activePlaylistGroup.length === 0" class="lib-left-empty">
              <PlumpIcon name="folder" :size="28" />
              <p>{{ store.libraryAssetTab === "created" ? "暂无自建歌单，点击右上角 + 新建" : "暂无收藏歌单，点击右上角图标可导入" }}</p>
            </div>

            <!-- 宫格模式 -->
            <TransitionGroup
              v-else-if="store.isLibraryGridView"
              name="lib-drag-flip"
              tag="div"
              class="lib-playlist-grid"
            >
              <div
                v-for="(pl, idx) in activePlaylistGroup"
                :key="pl.id || pl.title"
                :data-lib-drag-idx="idx"
                class="lib-pl-grid-item"
                :class="{
                  active: isPlaylistHighlighted(pl),
                  'is-dragging-item': dragState.active && dragState.fromIdx === idx,
                  'is-drag-over-item': dragState.active && dragState.overIdx === idx && dragState.fromIdx !== idx,
                }"
                @mousedown="onLeftItemMouseDown($event, idx)"
                @click="onLeftItemClick(() => selectPlaylistToRight(pl))"
                @contextmenu.prevent.stop="onLeftPlaylistRightClick(pl)"
              >
                <div class="lib-pl-cover-wrap">
                  <img
                    :src="pl.coverUrl"
                    class="lib-pl-cover"
                    loading="lazy"
                    draggable="false"
                    alt=""
                    @contextmenu.prevent.stop
                  />
                  <span class="lib-pl-count-tag">{{ pl.trackCount }}首</span>
                </div>
                <div class="lib-pl-name">{{ pl.name || pl.title }}</div>
              </div>
            </TransitionGroup>

            <!-- 列表模式 -->
            <TransitionGroup
              v-else
              name="lib-drag-flip"
              tag="div"
              class="lib-playlist-list"
            >
              <div
                v-for="(pl, idx) in activePlaylistGroup"
                :key="pl.id || pl.title"
                :data-lib-drag-idx="idx"
                class="lib-pl-row-item"
                :class="{
                  active: isPlaylistHighlighted(pl),
                  'is-dragging-item': dragState.active && dragState.fromIdx === idx,
                  'is-drag-over-item': dragState.active && dragState.overIdx === idx && dragState.fromIdx !== idx,
                }"
                @mousedown="onLeftItemMouseDown($event, idx)"
                @click="onLeftItemClick(() => selectPlaylistToRight(pl))"
                @contextmenu.prevent.stop="onLeftPlaylistRightClick(pl)"
              >
                <img
                  :src="pl.coverUrl"
                  class="lib-pl-row-cover"
                  loading="lazy"
                  draggable="false"
                  alt=""
                  @contextmenu.prevent.stop
                />
                <div class="lib-pl-row-meta">
                  <div class="lib-pl-row-name">{{ pl.name || pl.title }}</div>
                  <div class="lib-pl-row-sub">
                    {{ pl.trackCount }} 首 · {{ pl.creatorName || "概念版歌单" }}
                  </div>
                </div>
                <button
                  v-if="store.libraryAssetTab === 'created' && pl.listId > 0"
                  class="frameless-icon-square-btn danger-hover"
                  title="删除歌单"
                  @mousedown.stop
                  @click.stop="confirmDeletePlaylist(pl)"
                >
                  <PlumpIcon name="trash" :size="14" />
                </button>
              </div>
            </TransitionGroup>
          </template>

          <!-- B. 关注歌手列表 -->
          <template v-else-if="store.libraryAssetTab === 'artists'">
            <div v-if="store.followedArtists.length === 0" class="lib-left-empty">
              <PlumpIcon name="user" :size="28" />
              <p>暂无关注的歌手</p>
            </div>
            <TransitionGroup
              v-else
              name="lib-drag-flip"
              tag="div"
              class="lib-playlist-list"
            >
              <div
                v-for="(art, idx) in store.followedArtists"
                :key="art.singerId || art.artistId || art.name"
                :data-lib-drag-idx="idx"
                class="lib-pl-row-item"
                :class="{
                  active: isArtistHighlighted(art),
                  'is-dragging-item': dragState.active && dragState.fromIdx === idx,
                  'is-drag-over-item': dragState.active && dragState.overIdx === idx && dragState.fromIdx !== idx,
                }"
                @mousedown="onLeftItemMouseDown($event, idx)"
                @click="onLeftItemClick(() => selectArtistToRight(art))"
                @contextmenu.prevent.stop
              >
                <img
                  :src="art.avatarUrl"
                  class="lib-pl-row-cover round"
                  loading="lazy"
                  draggable="false"
                  alt=""
                  @contextmenu.prevent.stop
                />
                <div class="lib-pl-row-meta">
                  <div class="lib-pl-row-name">{{ art.name }}</div>
                  <div class="lib-pl-row-sub">
                    {{ art.albumCount ? `${art.albumCount} 张专辑` : "已关注音乐人" }}
                  </div>
                </div>
                <PlumpIcon name="chevron-right" :size="14" />
              </div>
            </TransitionGroup>
          </template>

          <!-- C. 已购数字专辑与单曲 -->
          <template v-else-if="store.libraryAssetTab === 'purchased'">
            <div
              v-if="purchasedCombinedEntries.length === 0"
              class="lib-left-empty"
            >
              <PlumpIcon name="disc" :size="28" />
              <p>暂无已购数字专辑或单曲</p>
            </div>
            <TransitionGroup
              v-else
              name="lib-drag-flip"
              tag="div"
              class="lib-playlist-list"
            >
              <div
                v-for="(entry, idx) in purchasedCombinedEntries"
                :key="entry.key"
                :data-lib-drag-idx="idx"
                class="lib-pl-row-item"
                :class="{
                  active: isPurchasedEntryHighlighted(entry),
                  'is-dragging-item': dragState.active && dragState.fromIdx === idx,
                  'is-drag-over-item': dragState.active && dragState.overIdx === idx && dragState.fromIdx !== idx,
                }"
                @mousedown="onLeftItemMouseDown($event, idx)"
                @click="onLeftItemClick(() => selectPurchasedEntryToRight(entry))"
                @contextmenu.prevent.stop
              >
                <template v-if="entry.kind === 'songs'">
                  <div class="lib-purchased-icon-cover">
                    <PlumpIcon name="sparkles" :size="20" color="#fff" />
                  </div>
                  <div class="lib-pl-row-meta">
                    <div class="lib-pl-row-name">已购尊享单曲合集</div>
                    <div class="lib-pl-row-sub">共 {{ store.purchasedSongs.length }} 首已购曲目</div>
                  </div>
                </template>
                <template v-else-if="entry.album">
                  <img
                    :src="entry.album.coverUrl"
                    class="lib-pl-row-cover"
                    loading="lazy"
                    draggable="false"
                    alt=""
                    @contextmenu.prevent.stop
                  />
                  <div class="lib-pl-row-meta">
                    <div class="lib-pl-row-name">{{ entry.album.albumName }}</div>
                    <div class="lib-pl-row-sub">{{ entry.album.singerName }} · {{ entry.album.songCount }} 首</div>
                  </div>
                </template>
              </div>
            </TransitionGroup>
          </template>
        </template>
      </aside>

      <!-- 右侧独立歌曲列表区域（全屏无裁剪边界，左侧收起后平滑横向伸展） -->
      <main class="library-right-frameless-region">
        <!-- 歌曲列表顶部标题栏：左侧新播放按钮，紧邻右侧搜索，最右侧排序（已移除搜索右侧多余的歌单标识胶囊） -->
        <div class="lib-right-frameless-header">
          <div class="lrf-left-toolbar">
            <!-- 1. 左侧：播放全部按钮 -->
            <button
              class="lrf-new-play-all-btn"
              :disabled="filteredAndSortedRightStageSongs.length === 0"
              title="播放当前列表全部歌曲"
              @click="playAllRightStage"
            >
              <span class="lrf-play-orb">
                <PlumpIcon name="play" :size="14" color="#ffffff" />
              </span>
              <span class="lrf-play-label">播放全部</span>
              <span class="lrf-play-count">{{ filteredAndSortedRightStageSongs.length }}</span>
            </button>

            <!-- 2. 播放按钮右侧紧邻：列表内搜索框 -->
            <div class="frameless-sublist-search lrf-inline-search">
              <PlumpIcon name="search" :size="15" />
              <input
                v-model="songFilterKw"
                type="text"
                placeholder="搜索歌名 / 歌手 / 专辑..."
              />
              <button v-if="songFilterKw" class="clear-inline-btn" @click="songFilterKw = ''">
                ×
              </button>
            </div>

            <!-- 定位当前播放歌曲按钮 -->
            <button
              v-if="isCurrentSongInRightStage"
              class="frameless-hover-btn lrf-locate-btn"
              title="定位到当前正在播放的歌曲"
              @click="scrollToPlayingSongInLibrary"
            >
              <PlumpIcon name="locate" :size="15" />
              <span>定位</span>
            </button>
          </div>

          <!-- 3. 最右侧：沿用 App 端分类标题设计的排序栏（默认排序、歌名、歌手名、时长最长 + 小横条） -->
          <div class="app-sliding-tabs lrf-sort-sliding-tabs">
            <button
              v-for="sTab in songSortTabs"
              :key="sTab.id"
              class="app-sliding-tab-btn lrf-sort-tab-btn"
              :class="{ active: songSortMode === sTab.id }"
              @click="handleSelectSongSort(sTab.id)"
            >
              <span class="tab-label">{{ sTab.label }}</span>
              <span
                class="tab-bottom-indicator"
                :style="{
                  backgroundColor: sTab.color,
                  opacity: songSortMode === sTab.id ? 1 : 0,
                  transform: songSortMode === sTab.id ? 'scaleX(1)' : 'scaleX(0.2)',
                }"
              ></span>
            </button>
          </div>
        </div>

        <!-- 右侧独立滚动的歌曲列表容器（纯水平横向平滑过渡动画，适当延长动画时长） -->
        <div class="lib-right-songs-scroller">
          <transition name="lib-songs-stream">
            <div :key="songListStreamKey" class="lib-songs-stream-stage">
              <div v-if="filteredAndSortedRightStageSongs.length === 0" class="lib-right-empty">
                <PlumpIcon name="music" :size="32" />
                <p>{{ store.isLoadingLibrary ? "正在同步歌单曲目..." : "当前列表暂无匹配的歌曲" }}</p>
              </div>

              <div v-else class="frameless-songs-grid-2col lib-frameless-songs">
                <div
                  v-for="(song, idx) in filteredAndSortedRightStageSongs"
                  :key="song.hash + '_' + idx"
                  :data-lib-song-hash="song.hash"
                  class="frameless-song-row"
                  :class="{ active: store.currentSong?.hash === song.hash }"
                  @click="store.playSongFromList(song, filteredAndSortedRightStageSongs)"
                  @contextmenu.prevent="store.openSongContextMenu($event, song, filteredAndSortedRightStageSongs)"
                >
                  <span class="fs-index">
                    <template v-if="store.currentSong?.hash === song.hash && store.isPlaying">
                      <span class="mini-eq-bars"><i></i><i></i><i></i></span>
                    </template>
                    <template v-else>
                      {{ String(idx + 1).padStart(2, "0") }}
                    </template>
                  </span>

                  <div class="fs-cover-wrap">
                    <img
                      :src="song.coverUrl"
                      class="fs-cover"
                      loading="lazy"
                      draggable="false"
                      alt=""
                    />
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
                      @click="store.openSongContextMenu($event, song, filteredAndSortedRightStageSongs)"
                    >
                      <PlumpIcon name="more" :size="16" />
                    </button>
                  </div>

                  <span class="fs-duration">{{ formatDuration(song.durationSeconds) }}</span>
                </div>
              </div>
            </div>
          </transition>
        </div>
      </main>
    </section>

    <!-- 新建歌单弹窗 -->
    <transition name="fade-pop">
      <div v-if="showCreateModal" class="modal-backdrop" @click.self="showCreateModal = false">
        <div class="compact-dialog glass-popover">
          <h3>新建自建歌单</h3>
          <p class="dialog-sub">创建后将同步至你的酷狗概念版云端资产</p>
          <input
            v-model="newPlaylistName"
            type="text"
            class="dialog-input"
            placeholder="请输入新歌单名称..."
            @keydown.enter="submitCreatePlaylist"
          />
          <div class="dialog-actions">
            <button class="frameless-hover-btn" @click="showCreateModal = false">取消</button>
            <button class="frameless-hover-btn primary-tint" @click="submitCreatePlaylist">
              立即创建
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 导入外部歌单弹窗 -->
    <transition name="fade-pop">
      <div v-if="showImportModal" class="modal-backdrop" @click.self="showImportModal = false">
        <div class="compact-dialog glass-popover">
          <h3>导入外部歌单</h3>
          <p class="dialog-sub">支持粘贴酷狗歌单码、分享口令或歌单链接自动解析收藏</p>
          <input
            v-model="importInputText"
            type="text"
            class="dialog-input"
            placeholder="粘贴歌单链接、口令或 global_collection_id..."
            @keydown.enter="submitImportPlaylist"
          />
          <div class="dialog-actions">
            <button class="frameless-hover-btn" @click="showImportModal = false">取消</button>
            <button class="frameless-hover-btn primary-tint" @click="submitImportPlaylist">
              解析并导入
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import type { FollowedArtistItem, PlaylistItem, PurchasedAlbumItem, Song } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

type RightStageMode = "liked" | "recent" | "playlist" | "artist" | "album" | "purchased_songs";
type ExpandedTopCard = "none" | "liked" | "recent";
type SongSortMode = "default" | "title" | "artist" | "duration";

const rightStageMode = ref<RightStageMode>("liked");
const expandedTopCard = ref<ExpandedTopCard>("none");
const songFilterKw = ref("");
const songSortMode = ref<SongSortMode>("default");

// 左侧歌单/歌手/已购栏收起为小图标栏的状态（持久化记录）
const isLeftSidebarCollapsed = ref<boolean>(
  localStorage.getItem("bm_lib_left_collapsed") === "true"
);

function toggleLeftSidebarCollapse() {
  isLeftSidebarCollapsed.value = !isLeftSidebarCollapsed.value;
  localStorage.setItem("bm_lib_left_collapsed", String(isLeftSidebarCollapsed.value));
}

const showCreateModal = ref(false);
const newPlaylistName = ref("");
const showImportModal = ref(false);
const importInputText = ref("");

// 四个分类标题：自建、收藏、关注、已购
const assetTabs = computed(() => [
  {
    id: "created" as const,
    label: "自建",
    count: store.createdPlaylists.length,
    color: "#2b8cff",
  },
  {
    id: "collected" as const,
    label: "收藏",
    count: store.collectedPlaylists.length,
    color: "#ff6b6b",
  },
  {
    id: "artists" as const,
    label: "关注",
    count: store.followedArtists.length,
    color: "#15c493",
  },
  {
    id: "purchased" as const,
    label: "已购",
    count: store.purchasedAlbums.length + (store.purchasedSongs.length > 0 ? 1 : 0),
    color: "#f5a623",
  },
]);

// 歌曲列表最右侧排序栏（沿用 App 端分类标题与底部小横条指示器）
const songSortTabs: Array<{ id: SongSortMode; label: string; color: string }> = [
  { id: "default", label: "默认排序", color: "#2b8cff" },
  { id: "title", label: "歌名", color: "#00e5ff" },
  { id: "artist", label: "歌手名", color: "#15c493" },
  { id: "duration", label: "时长最长", color: "#f5a623" },
];

function handleSelectSongSort(mode: SongSortMode) {
  songSortMode.value = mode;
}

const activePlaylistGroup = computed(() => {
  return store.libraryAssetTab === "collected"
    ? store.collectedPlaylists
    : store.createdPlaylists;
});

interface PurchasedCombinedEntry {
  key: string;
  kind: "songs" | "album";
  album?: PurchasedAlbumItem;
}

const purchasedCombinedEntries = computed<PurchasedCombinedEntry[]>(() => {
  const raw: PurchasedCombinedEntry[] = [];
  if (store.purchasedSongs.length > 0) {
    raw.push({ key: "__purchased_songs__", kind: "songs" });
  }
  for (const alb of store.purchasedAlbums) {
    raw.push({
      key: String(alb.id || alb.albumId || alb.albumName),
      kind: "album",
      album: alb,
    });
  }
  const orderIds = store.purchasedOrderIds;
  if (!orderIds || orderIds.length === 0) return raw;
  const orderMap = new Map<string, number>();
  orderIds.forEach((id, idx) => orderMap.set(id, idx));
  return [...raw].sort((a, b) => {
    const ia = orderMap.get(a.key);
    const ib = orderMap.get(b.key);
    if (ia !== undefined && ib !== undefined) return ia - ib;
    if (ia !== undefined) return -1;
    if (ib !== undefined) return 1;
    return 0;
  });
});

function formatTotalHoursMinutes(songs: Song[]): string {
  const totalSec = songs.reduce((acc, s) => {
    const sec = s.durationSeconds ?? Math.floor((s.durationMs || 0) / 1000);
    return acc + (Number.isFinite(sec) ? sec : 0);
  }, 0);
  const hours = Math.floor(totalSec / 3600);
  const mins = Math.max(1, Math.floor((totalSec % 3600) / 60));
  return hours > 0 ? `约 ${hours} 小时 ${mins} 分` : `约 ${mins} 分钟`;
}

const likedHiResCount = computed(
  () => store.likedSongs.filter((s) => s.hasHiRes || s.hasSq || s.isLossless || s.isHiRes).length
);
const likedTotalDurationText = computed(() => formatTotalHoursMinutes(store.likedSongs));

const recentHiResCount = computed(
  () => store.recentHistorySongs.filter((s) => s.hasHiRes || s.hasSq || s.isLossless || s.isHiRes).length
);
const recentTotalDurationText = computed(() => formatTotalHoursMinutes(store.recentHistorySongs));

function isPlaylistHighlighted(pl: PlaylistItem): boolean {
  if (expandedTopCard.value !== "none") return false;
  if (rightStageMode.value !== "playlist") return false;
  return Boolean(
    store.selectedPlaylist &&
      (store.selectedPlaylist.id === pl.id || store.selectedPlaylist.title === pl.title)
  );
}

function isArtistHighlighted(art: FollowedArtistItem): boolean {
  if (expandedTopCard.value !== "none") return false;
  if (rightStageMode.value !== "artist") return false;
  const activeId = String(store.activeArtistDetail?.singerId || store.activeArtistDetail?.artistId || "");
  const itemId = String(art.singerId || art.artistId || "");
  return Boolean(activeId && activeId === itemId) || store.activeArtistDetail?.name === art.name;
}

function isPurchasedEntryHighlighted(entry: PurchasedCombinedEntry): boolean {
  if (expandedTopCard.value !== "none") return false;
  if (entry.kind === "songs") {
    return rightStageMode.value === "purchased_songs";
  }
  if (rightStageMode.value === "album" && entry.album) {
    const activeId = String(store.activeAlbumDetail?.albumId || "");
    const itemId = String(entry.album.id || entry.album.albumId || "");
    return Boolean(activeId && activeId === itemId) || store.activeAlbumDetail?.albumName === entry.album.albumName;
  }
  return false;
}

function handleSelectAssetTab(tabId: "created" | "collected" | "artists" | "purchased") {
  store.libraryAssetTab = tabId;
  if (tabId === "artists" && store.followedArtists.length === 0) {
    store.loadFollowedArtists();
  } else if (tabId === "purchased" && store.purchasedAlbums.length === 0 && store.purchasedSongs.length === 0) {
    store.loadPurchasedAssets();
  }
}

function activateLikedMode() {
  rightStageMode.value = "liked";
  expandedTopCard.value = expandedTopCard.value === "liked" ? "none" : "liked";
  songFilterKw.value = "";
}

function activateRecentMode() {
  rightStageMode.value = "recent";
  expandedTopCard.value = expandedTopCard.value === "recent" ? "none" : "recent";
  songFilterKw.value = "";
  if (store.recentHistorySongs.length === 0) {
    store.loadRecentHistory();
  }
}

function collapseExpandedTopCard() {
  expandedTopCard.value = "none";
}

function selectPlaylistToRight(pl: PlaylistItem) {
  expandedTopCard.value = "none";
  rightStageMode.value = "playlist";
  songFilterKw.value = "";
  store.selectPlaylist(pl);
}

async function selectArtistToRight(art: FollowedArtistItem) {
  expandedTopCard.value = "none";
  rightStageMode.value = "artist";
  songFilterKw.value = "";
  await store.openArtistDetail(String(art.singerId || (art as any).artistId || ""), art.name, art.avatarUrl);
  store.libraryDetailMode = "playlists";
}

async function selectPurchasedAlbumToRight(alb: PurchasedAlbumItem) {
  expandedTopCard.value = "none";
  rightStageMode.value = "album";
  songFilterKw.value = "";
  await store.openAlbumDetail(
    String(alb.id || (alb as any).albumId || ""),
    alb.albumName,
    alb.singerName || (alb as any).artistName || "",
    alb.coverUrl
  );
  store.libraryDetailMode = "playlists";
}

function selectPurchasedEntryToRight(entry: PurchasedCombinedEntry) {
  if (entry.kind === "songs") {
    expandedTopCard.value = "none";
    rightStageMode.value = "purchased_songs";
    songFilterKw.value = "";
  } else if (entry.album) {
    selectPurchasedAlbumToRight(entry.album);
  }
}

function onLeftPlaylistRightClick(_pl: PlaylistItem) {
  // 拦截浏览器原生图片菜单，预留后续歌单右键菜单能力
}

// ==================== 鼠标左键长按拖动排序引擎（支持展开态与小图标收起态） ====================
const dragState = reactive<{
  active: boolean;
  fromIdx: number;
  overIdx: number;
}>({
  active: false,
  fromIdx: -1,
  overIdx: -1,
});

let longPressTimer: number | null = null;
let pressStartX = 0;
let pressStartY = 0;
let suppressClickAfterDrag = false;

function clearLongPressTimer() {
  if (longPressTimer !== null) {
    window.clearTimeout(longPressTimer);
    longPressTimer = null;
  }
}

function onLeftItemMouseDown(ev: MouseEvent, idx: number) {
  if (ev.button !== 0) return;
  clearLongPressTimer();
  pressStartX = ev.clientX;
  pressStartY = ev.clientY;
  suppressClickAfterDrag = false;

  longPressTimer = window.setTimeout(() => {
    dragState.active = true;
    dragState.fromIdx = idx;
    dragState.overIdx = idx;
    suppressClickAfterDrag = true;
    document.body.classList.add("lib-dragging-cursor");
  }, 190);

  window.addEventListener("mousemove", onGlobalDragMouseMove, { passive: true });
  window.addEventListener("mouseup", onGlobalDragMouseUp, { passive: true });
}

function onGlobalDragMouseMove(ev: MouseEvent) {
  if (!dragState.active) {
    const dist = Math.hypot(ev.clientX - pressStartX, ev.clientY - pressStartY);
    if (dist > 8) {
      clearLongPressTimer();
    }
    return;
  }

  const targetEl = document
    .elementFromPoint(ev.clientX, ev.clientY)
    ?.closest("[data-lib-drag-idx]") as HTMLElement | null;
  if (!targetEl) return;

  const targetIdx = Number(targetEl.dataset.libDragIdx);
  if (!Number.isFinite(targetIdx) || targetIdx < 0 || targetIdx === dragState.fromIdx) {
    return;
  }

  dragState.overIdx = targetIdx;
  const tab = store.libraryAssetTab;
  if (tab === "created" || tab === "collected" || tab === "artists") {
    store.reorderLibraryAssetList(tab, dragState.fromIdx, targetIdx);
    dragState.fromIdx = targetIdx;
  } else if (tab === "purchased") {
    const keys = purchasedCombinedEntries.value.map((e) => e.key);
    if (dragState.fromIdx >= 0 && dragState.fromIdx < keys.length && targetIdx < keys.length) {
      const [movedKey] = keys.splice(dragState.fromIdx, 1);
      keys.splice(targetIdx, 0, movedKey);
      store.reorderLibraryAssetList("purchased", dragState.fromIdx, targetIdx, keys);
      dragState.fromIdx = targetIdx;
    }
  }
}

function onGlobalDragMouseUp() {
  clearLongPressTimer();
  window.removeEventListener("mousemove", onGlobalDragMouseMove);
  window.removeEventListener("mouseup", onGlobalDragMouseUp);

  if (dragState.active) {
    dragState.active = false;
    dragState.fromIdx = -1;
    dragState.overIdx = -1;
    document.body.classList.remove("lib-dragging-cursor");
    window.setTimeout(() => {
      suppressClickAfterDrag = false;
    }, 80);
  }
}

function onLeftItemClick(action: () => void) {
  if (suppressClickAfterDrag) {
    suppressClickAfterDrag = false;
    return;
  }
  action();
}

// ==================== 右侧歌曲列表数据源、筛选、排序与过渡 Key ====================
const rightStageSongs = computed<Song[]>(() => {
  switch (rightStageMode.value) {
    case "liked":
      return store.likedSongs;
    case "recent":
      return store.recentHistorySongs;
    case "playlist":
      return store.playlistSongs;
    case "artist":
      return store.activeArtistSongs;
    case "album":
      return store.activeAlbumSongs;
    case "purchased_songs":
      return store.purchasedSongs.map((item: any) => item.song || item);
    default:
      return store.likedSongs;
  }
});

const filteredAndSortedRightStageSongs = computed<Song[]>(() => {
  const kw = songFilterKw.value.trim().toLowerCase();
  const base = !kw
    ? rightStageSongs.value
    : rightStageSongs.value.filter(
        (s) =>
          s.title.toLowerCase().includes(kw) ||
          s.artist.toLowerCase().includes(kw) ||
          (s.album && s.album.toLowerCase().includes(kw))
      );

  if (songSortMode.value === "default") {
    return base;
  }
  const sorted = [...base];
  if (songSortMode.value === "title") {
    sorted.sort((a, b) =>
      (a.title || "").localeCompare(b.title || "", "zh-Hans-CN", { sensitivity: "base" })
    );
  } else if (songSortMode.value === "artist") {
    sorted.sort((a, b) =>
      (a.artist || "").localeCompare(b.artist || "", "zh-Hans-CN", { sensitivity: "base" })
    );
  } else if (songSortMode.value === "duration") {
    sorted.sort((a, b) => {
      const da = a.durationSeconds ?? Math.floor((a.durationMs || 0) / 1000);
      const db = b.durationSeconds ?? Math.floor((b.durationMs || 0) / 1000);
      return db - da;
    });
  }
  return sorted;
});

const songListStreamKey = computed(() => {
  let entityId = "";
  if (rightStageMode.value === "playlist") {
    entityId = String(store.selectedPlaylist?.id || store.selectedPlaylist?.title || "");
  } else if (rightStageMode.value === "artist") {
    entityId = String(store.activeArtistDetail?.singerId || store.activeArtistDetail?.artistId || "");
  } else if (rightStageMode.value === "album") {
    entityId = String(store.activeAlbumDetail?.albumId || "");
  }
  return `${rightStageMode.value}__${entityId}__${songSortMode.value}`;
});

function playAllLiked() {
  rightStageMode.value = "liked";
  if (store.likedSongs.length > 0) {
    store.playSongFromList(store.likedSongs[0], store.likedSongs);
  }
}

function shufflePlayLiked() {
  rightStageMode.value = "liked";
  if (store.likedSongs.length > 0) {
    store.playMode = "shuffle";
    const randIdx = Math.floor(Math.random() * store.likedSongs.length);
    store.playSongFromList(store.likedSongs[randIdx], store.likedSongs);
  }
}

function playAllRecent() {
  rightStageMode.value = "recent";
  if (store.recentHistorySongs.length > 0) {
    store.playSongFromList(store.recentHistorySongs[0], store.recentHistorySongs);
  }
}

function shufflePlayRecent() {
  rightStageMode.value = "recent";
  if (store.recentHistorySongs.length > 0) {
    store.playMode = "shuffle";
    const randIdx = Math.floor(Math.random() * store.recentHistorySongs.length);
    store.playSongFromList(store.recentHistorySongs[randIdx], store.recentHistorySongs);
  }
}

function playAllRightStage() {
  if (filteredAndSortedRightStageSongs.value.length > 0) {
    store.playSongFromList(
      filteredAndSortedRightStageSongs.value[0],
      filteredAndSortedRightStageSongs.value
    );
  }
}

const isCurrentSongInRightStage = computed(() => {
  if (!store.currentSong) return false;
  return rightStageSongs.value.some((s) => s.hash === store.currentSong?.hash);
});

function scrollToPlayingSongInLibrary() {
  if (!store.currentSong) return;
  const el = document.querySelector(`[data-lib-song-hash="${store.currentSong.hash}"]`);
  if (el) el.scrollIntoView({ behavior: "smooth", block: "center" });
}

function openCreateModal() {
  newPlaylistName.value = "";
  showCreateModal.value = true;
}

async function submitCreatePlaylist() {
  const name = newPlaylistName.value.trim();
  if (!name) return;
  showCreateModal.value = false;
  await store.createNewPlaylist(name);
}

function openImportModal() {
  importInputText.value = "";
  showImportModal.value = true;
}

async function submitImportPlaylist() {
  const raw = importInputText.value.trim();
  if (!raw) return;
  showImportModal.value = false;
  await store.importExternalPlaylist(raw);
}

function confirmDeletePlaylist(pl: PlaylistItem) {
  if (window.confirm(`确定要删除自建歌单「${pl.name || pl.title}」吗？`)) {
    store.deletePlaylist(pl);
  }
}

onMounted(() => {
  if (store.userPlaylists.length === 0) {
    store.loadUserPlaylists();
  }
  if (store.recentHistorySongs.length === 0) {
    store.loadRecentHistory();
  }
});

onBeforeUnmount(() => {
  clearLongPressTimer();
  window.removeEventListener("mousemove", onGlobalDragMouseMove);
  window.removeEventListener("mouseup", onGlobalDragMouseUp);
  document.body.classList.remove("lib-dragging-cursor");
});
</script>
