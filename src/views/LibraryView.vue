<template>
  <!-- 我的页：全屏无裁剪渲染，划分为左右两个内容展示区与各自独立的滑动区域 -->
  <div class="library-page library-fixed-viewport" @contextmenu.prevent>
    <!-- ==================== 顶部三张核心卡片（154px 固定高度，CSS Grid 1fr/0fr 丝滑横向展开，无模糊底图，点击展开卡片即可返回） ==================== -->
    <section
      class="library-hero-trio enlarged-trio"
      :class="{
        'is-liked-expanded': expandedTopCard === 'liked',
        'is-recent-expanded': expandedTopCard === 'recent',
      }"
    >
      <!-- 1. 我喜欢的音乐（左卡：点击向右丝滑铺满，再次点击卡片即可返回三卡视图） -->
      <div
        class="lib-stereo-card fav-stereo-card"
        :class="{
          active: rightStageMode === 'liked',
          'is-expanded-full': expandedTopCard === 'liked',
          'is-vanished-left': expandedTopCard === 'recent',
        }"
        @click="activateLikedMode"
      >
        <div class="lib-card-inner-flow">
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
                  <PlumpIcon name="heart-filled" :size="26" color="#fff" />
                </div>
                <span class="lib-visual-corner-tag coral">
                  <PlumpIcon name="heart-filled" :size="11" color="#fff" />
                </span>
              </div>
            </div>

            <div class="lib-card-copy">
              <div class="lib-card-eyebrow">FAVORITES · 红心曲库</div>
              <div class="lib-card-title-row">
                <h3 class="lib-card-title">我喜欢的音乐</h3>
                <span class="lib-card-count-pill">{{ store.likedSongs.length }}</span>
              </div>

              <div class="lib-card-sub-stack">
                <div
                  class="lib-sub-layer preview-layer"
                  :class="{ 'layer-hidden': expandedTopCard === 'liked' }"
                >
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
                </div>

                <div
                  class="lib-sub-layer stats-layer"
                  :class="{ 'layer-active': expandedTopCard === 'liked' }"
                >
                  <span class="lcm-stat">{{ store.likedSongs.length }} 首珍藏</span>
                  <template v-if="likedHiResCount > 0">
                    <span class="lcm-dot">·</span>
                    <span class="lcm-stat highlight">{{ likedHiResCount }} 首无损 / Hi-Res</span>
                  </template>
                  <span class="lcm-dot">·</span>
                  <span class="lcm-stat">{{ likedTotalDurationText }}</span>
                </div>
              </div>
            </div>
          </div>

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

          <div class="lib-card-right-zone" @click.stop>
            <div
              class="lib-expanded-extra-actions"
              :class="{ 'is-visible': expandedTopCard === 'liked' }"
            >
              <button
                class="lib-card-glass-btn"
                title="随机播放我喜欢的音乐"
                @click.stop="shufflePlayLiked"
              >
                <PlumpIcon name="shuffle" :size="15" color="#fff" />
                <span>随机漫游</span>
              </button>
            </div>
            <button
              class="lib-card-play-fab coral"
              title="立即播放我喜欢的音乐"
              @click.stop="playAllLiked"
            >
              <PlumpIcon name="play" :size="20" color="#b82048" />
            </button>
          </div>
        </div>
      </div>

      <!-- 2. 最近播放（中卡：点击向两侧丝滑铺满，再次点击卡片即可返回三卡视图） -->
      <div
        class="lib-stereo-card recent-stereo-card"
        :class="{
          active: rightStageMode === 'recent',
          'is-expanded-full': expandedTopCard === 'recent',
          'is-vanished-right': expandedTopCard === 'liked',
        }"
        @click="activateRecentMode"
      >
        <div class="lib-card-inner-flow">
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
                  <PlumpIcon name="timer" :size="26" color="#fff" />
                </div>
                <span class="lib-visual-corner-tag blue">
                  <PlumpIcon name="timer" :size="11" color="#fff" />
                </span>
              </div>
            </div>

            <div class="lib-card-copy">
              <div class="lib-card-eyebrow">RECENT PLAYED · 听歌足迹</div>
              <div class="lib-card-title-row">
                <h3 class="lib-card-title">最近播放</h3>
                <span class="lib-card-count-pill">{{ store.recentHistorySongs.length }}</span>
              </div>

              <div class="lib-card-sub-stack">
                <div
                  class="lib-sub-layer preview-layer"
                  :class="{ 'layer-hidden': expandedTopCard === 'recent' }"
                >
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
                </div>

                <div
                  class="lib-sub-layer stats-layer"
                  :class="{ 'layer-active': expandedTopCard === 'recent' }"
                >
                  <span class="lcm-stat">{{ store.recentHistorySongs.length }} 首近期记录</span>
                  <template v-if="recentHiResCount > 0">
                    <span class="lcm-dot">·</span>
                    <span class="lcm-stat highlight">{{ recentHiResCount }} 首无损 / Hi-Res</span>
                  </template>
                  <span class="lcm-dot">·</span>
                  <span class="lcm-stat">{{ recentTotalDurationText }}</span>
                </div>
              </div>
            </div>
          </div>

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
            <div
              class="lib-expanded-extra-actions"
              :class="{ 'is-visible': expandedTopCard === 'recent' }"
            >
              <button
                class="lib-card-glass-btn"
                title="随机播放最近听过"
                @click.stop="shufflePlayRecent"
              >
                <PlumpIcon name="shuffle" :size="15" color="#fff" />
                <span>随机漫游</span>
              </button>
            </div>
            <button
              class="lib-card-play-fab blue"
              title="立即播放最近听过"
              @click.stop="playAllRecent"
            >
              <PlumpIcon name="play" :size="20" color="#12559e" />
            </button>
          </div>
        </div>
      </div>

      <!-- 3. 心流舱 · Focus Pod -->
      <div
        class="lib-stereo-card focus-stereo-card"
        :class="{
          'is-vanished-right': expandedTopCard === 'liked' || expandedTopCard === 'recent',
        }"
        @click="store.isFocusPodOpen = true"
      >
        <div class="lib-card-inner-flow">
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
              <div class="lib-card-sub-stack">
                <div class="lib-sub-layer preview-layer">
                  <div class="lib-preview-line">
                    <span class="lp-song">雨声 · 潮汐 · 篝火 · 黑胶底噪</span>
                  </div>
                  <div class="lib-preview-line secondary">
                    <span>四轨环境声混音与专注计时</span>
                  </div>
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
              <PlumpIcon name="compass" :size="20" color="#117868" />
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- ==================== 分类标题与工具栏 ==================== -->
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

        <!-- 在「已购」右边新增的收起/展开按钮：纯重绘 UI 图标，无文字 -->
        <button
          class="frameless-icon-square-btn lib-collapse-sidebar-icon-btn"
          :class="{ active: isLeftSidebarCollapsed }"
          :title="isLeftSidebarCollapsed ? '展开左侧歌单与歌手区域' : '收起左侧区域为小图标栏'"
          @click="toggleLeftSidebarCollapse"
        >
          <PlumpIcon
            :name="isLeftSidebarCollapsed ? 'sidebar-expand' : 'sidebar-collapse'"
            :size="19"
          />
        </button>
      </div>

      <!-- 最右侧：创建歌单、导入外部歌单/分享码、视图切换 纯 UI 图标按钮 -->
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
          title="导入外部歌单链接、分享口令或酷狗歌单码"
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

    <!-- ==================== 全屏双区展示舞台 ==================== -->
    <section
      class="library-dual-independent-stage"
      :class="{ 'is-left-collapsed': isLeftSidebarCollapsed }"
    >
      <!-- 左侧独立滑动区域 -->
      <aside
        class="library-left-frameless-region"
        :class="{ 'is-collapsed-rail': isLeftSidebarCollapsed }"
        @contextmenu.prevent.stop
      >
        <!-- 形态 1：左侧收起态 -->
        <template v-if="isLeftSidebarCollapsed">
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

        <!-- 形态 2：左侧常规展开态 -->
        <template v-else>
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
                  title="删除自建歌单"
                  @mousedown.stop
                  @click.stop="confirmDeletePlaylist(pl)"
                >
                  <PlumpIcon name="trash" :size="14" />
                </button>
                <button
                  v-else-if="store.libraryAssetTab === 'collected'"
                  class="frameless-icon-square-btn danger-hover"
                  title="取消收藏此歌单"
                  @mousedown.stop
                  @click.stop="store.toggleCollectPlaylist(pl)"
                >
                  <PlumpIcon name="trash" :size="14" />
                </button>
              </div>
            </TransitionGroup>
          </template>

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
                    {{ art.songCount ? `单曲 ${art.songCount} 首` : art.albumCount ? `${art.albumCount} 张专辑` : "已关注音乐人" }}
                  </div>
                </div>
                <PlumpIcon name="chevron-right" :size="14" />
              </div>
            </TransitionGroup>
          </template>

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

      <!-- 右侧独立歌曲/歌手/专辑详情展示区域 -->
      <main class="library-right-frameless-region">
        <!-- 1. 歌手主页 Hero 头卡（当 rightStageMode === 'artist' 时显示，含 热门单曲 / 专辑画廊 / 歌手百科 三子标签） -->
        <div
          v-if="rightStageMode === 'artist' && store.activeArtistDetail"
          class="lib-detail-hero-card"
        >
          <img
            v-if="store.activeArtistDetail.avatarUrl"
            :src="store.activeArtistDetail.avatarUrl"
            class="ldh-avatar"
            alt=""
            @click="store.hdArtworkPreviewUrl = store.activeArtistDetail.avatarUrl.replace('/400/', '/1000/')"
          />
          <div class="ldh-info">
            <div class="ldh-title-row">
              <span class="ldh-name">{{ store.activeArtistDetail.name }}</span>
              <span class="mini-badge sq">官方认证音乐人</span>
            </div>
            <div class="ldh-stats">
              <span>单曲 {{ store.activeArtistDetail.songCount || store.activeArtistSongs.length }}</span>
              <span>· 专辑 {{ store.activeArtistDetail.albumCount || store.activeArtistAlbums.length }}</span>
              <span v-if="store.activeArtistDetail.fansCount">
                · 乐迷 {{ formatFansCount(store.activeArtistDetail.fansCount) }}
              </span>
            </div>
            <div v-if="store.activeArtistDetail.intro" class="ldh-intro">
              {{ store.activeArtistDetail.intro }}
            </div>
          </div>

          <div class="ldh-actions">
            <div class="pill-tabs" style="padding: 3px">
              <button
                class="pill-tab-btn"
                :class="{ active: artistSubTab === 'songs' }"
                style="padding: 5px 12px !important; font-size: 13.5px !important"
                @click="artistSubTab = 'songs'"
              >
                热门单曲 ({{ store.activeArtistSongs.length }})
              </button>
              <button
                class="pill-tab-btn"
                :class="{ active: artistSubTab === 'albums' }"
                style="padding: 5px 12px !important; font-size: 13.5px !important"
                @click="artistSubTab = 'albums'"
              >
                专辑画廊 ({{ store.activeArtistAlbums.length }})
              </button>
              <button
                class="pill-tab-btn"
                :class="{ active: artistSubTab === 'bio' }"
                style="padding: 5px 12px !important; font-size: 13.5px !important"
                @click="artistSubTab = 'bio'"
              >
                歌手百科
              </button>
            </div>

            <button
              v-if="artistSubTab === 'songs'"
              class="tonal-pill-btn sm"
              title="切换歌手单曲最热 / 最新发行排序"
              @click="
                store.switchArtistSongSort(
                  store.artistSongSort === 'hot' ? 'new' : 'hot'
                )
              "
            >
              <span>{{ store.artistSongSort === "hot" ? "🔥 最热代表作" : "🆕 最新发行" }}</span>
            </button>

            <button
              class="tonal-pill-btn sm primary"
              @click="store.toggleFollowArtist(store.activeArtistDetail)"
            >
              <PlumpIcon
                :name="store.activeArtistDetail.isFollowed ? 'check' : 'plus'"
                :size="14"
              />
              <span>{{ store.activeArtistDetail.isFollowed ? "已关注" : "关注歌手" }}</span>
            </button>
          </div>
        </div>

        <!-- 2. 专辑详情 Hero 头卡（当 rightStageMode === 'album' 时显示） -->
        <div
          v-else-if="rightStageMode === 'album' && store.activeAlbumDetail"
          class="lib-detail-hero-card"
        >
          <img
            v-if="store.activeAlbumDetail.coverUrl"
            :src="store.activeAlbumDetail.coverUrl"
            class="ldh-album-cover"
            alt=""
            @click="store.hdArtworkPreviewUrl = store.activeAlbumDetail.coverUrl.replace('/400/', '/1000/')"
          />
          <div class="ldh-info">
            <div class="ldh-title-row">
              <span class="ldh-name">{{ store.activeAlbumDetail.albumName }}</span>
              <span class="mini-badge hires">正版数字专辑</span>
            </div>
            <div class="ldh-stats">
              <span
                style="cursor: pointer; color: var(--cyan)"
                @click="
                  store.openArtistDetail(
                    store.activeAlbumDetail.artistId,
                    store.activeAlbumDetail.artistName,
                    store.activeAlbumDetail.coverUrl
                  )
                "
              >
                歌手：{{ store.activeAlbumDetail.artistName || "未知歌手" }}
              </span>
              <span v-if="store.activeAlbumDetail.publishDate">
                · 发行于 {{ store.activeAlbumDetail.publishDate }}
              </span>
              <span v-if="store.activeAlbumDetail.company">
                · {{ store.activeAlbumDetail.company }}
              </span>
              <span>· 共 {{ store.activeAlbumSongs.length }} 首</span>
            </div>
            <div v-if="store.activeAlbumDetail.intro" class="ldh-intro">
              {{ store.activeAlbumDetail.intro }}
            </div>
          </div>
        </div>

        <!-- 歌曲列表顶部标题栏：左侧新播放按钮，紧邻右侧搜索、批量多选，最右侧排序 -->
        <div
          v-if="!(rightStageMode === 'artist' && artistSubTab !== 'songs')"
          class="lib-right-frameless-header"
        >
          <div class="lrf-left-toolbar">
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

            <button
              v-if="isCurrentSongInRightStage"
              class="frameless-hover-btn lrf-locate-btn"
              title="定位到当前正在播放的歌曲"
              @click="scrollToPlayingSongInLibrary"
            >
              <PlumpIcon name="locate" :size="15" />
              <span>定位</span>
            </button>

            <button
              v-if="filteredAndSortedRightStageSongs.length > 0"
              class="frameless-hover-btn"
              :class="{ 'primary-tint': isBatchMode }"
              title="开启批量多选管理"
              @click="toggleBatchMode"
            >
              <PlumpIcon name="check" :size="15" />
              <span>{{ isBatchMode ? `退出多选 (${selectedBatchHashes.size})` : "批量操作" }}</span>
            </button>
          </div>

          <!-- 最右侧：沿用 App 端分类标题设计的排序栏 -->
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

        <!-- 批量多选操作工具条 -->
        <div v-if="isBatchMode" class="batch-action-bar">
          <div style="display: flex; align-items: center; gap: 10px">
            <button class="tonal-pill-btn sm" @click="selectAllBatch">
              {{ selectedBatchHashes.size === filteredAndSortedRightStageSongs.length ? "取消全选" : "全选当前列表" }}
            </button>
            <span style="font-size: 13.5px; font-weight: 800">
              已选 {{ selectedBatchHashes.size }} 首曲目
            </span>
          </div>
          <div style="display: flex; align-items: center; gap: 8px">
            <button
              class="tonal-pill-btn sm primary"
              :disabled="selectedBatchHashes.size === 0"
              @click="playSelectedBatch"
            >
              <PlumpIcon name="play" :size="13" />
              <span>播放所选</span>
            </button>
            <button
              class="tonal-pill-btn sm"
              :disabled="selectedBatchHashes.size === 0"
              @click="likeSelectedBatch"
            >
              <PlumpIcon name="heart" :size="13" />
              <span>收藏至我喜欢</span>
            </button>
          </div>
        </div>

        <!-- 右侧独立滚动区域 -->
        <div class="lib-right-songs-scroller">
          <!-- 歌手主页子视图 2：专辑画廊 -->
          <div
            v-if="rightStageMode === 'artist' && artistSubTab === 'albums'"
            class="album-cards-grid"
          >
            <div
              v-if="store.activeArtistAlbums.length === 0"
              class="lib-right-empty"
              style="grid-column: 1 / -1"
            >
              <PlumpIcon name="disc" :size="30" />
              <p>暂无收录专辑</p>
            </div>
            <div
              v-for="alb in store.activeArtistAlbums"
              :key="alb.albumId"
              class="plump-album-card"
              @click="
                store.openAlbumDetail(
                  alb.albumId,
                  alb.albumName,
                  alb.artistName || store.activeArtistDetail?.name || '',
                  alb.coverUrl
                )
              "
            >
              <img v-if="alb.coverUrl" :src="alb.coverUrl" class="palb-cover" alt="" />
              <div class="palb-info">
                <div class="palb-name">{{ alb.albumName }}</div>
                <div class="palb-sub">
                  {{ alb.publishDate || "数字发行" }} · {{ alb.songCount || 10 }} 首
                </div>
              </div>
              <PlumpIcon name="chevron-right" :size="14" />
            </div>
          </div>

          <!-- 歌手主页子视图 3：歌手百科与人物志 -->
          <div
            v-else-if="rightStageMode === 'artist' && artistSubTab === 'bio'"
            class="artist-encyclopedia-card"
          >
            <h3 style="margin-bottom: 10px; font-size: 18px; color: var(--text-primary)">
              {{ store.activeArtistDetail?.name }} · 音乐人百科档案
            </h3>
            <p>
              {{
                store.activeArtistDetail?.longIntro ||
                store.activeArtistDetail?.intro ||
                "酷狗概念版官方收录音乐人，代表作品广受乐迷喜爱。"
              }}
            </p>
          </div>

          <!-- 常规歌曲列表视图 -->
          <transition v-else name="lib-songs-stream">
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
                  :class="{
                    active:
                      store.currentSong?.hash === song.hash ||
                      (isBatchMode && selectedBatchHashes.has(song.hash)),
                  }"
                  @click="
                    isBatchMode
                      ? toggleBatchSong(song.hash)
                      : store.playSongFromList(song, filteredAndSortedRightStageSongs)
                  "
                  @contextmenu.prevent="store.openSongContextMenu($event, song, filteredAndSortedRightStageSongs)"
                >
                  <span class="fs-index">
                    <template v-if="isBatchMode">
                      <PlumpIcon
                        name="check"
                        :size="15"
                        :accent="
                          selectedBatchHashes.has(song.hash)
                            ? 'var(--cyan)'
                            : 'rgba(148,163,184,0.35)'
                        "
                      />
                    </template>
                    <template v-else-if="store.currentSong?.hash === song.hash && store.isPlaying">
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
                      <span v-if="song.hasHiRes || song.isHiRes" class="fs-badge hires">Hi-Res</span>
                      <span v-else-if="song.hasSq || song.isLossless" class="fs-badge sq">SQ</span>
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

                  <span class="fs-duration">{{ formatDuration(song) }}</span>
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

    <!-- 导入外部歌单 / 分享口令 / 多行文本歌单弹窗 -->
    <transition name="fade-pop">
      <div v-if="showImportModal" class="modal-backdrop" @click.self="showImportModal = false">
        <div class="compact-dialog glass-popover" style="width: min(92vw, 520px)">
          <h3>导入外部歌单与分享口令</h3>
          <p class="dialog-sub">
            支持粘贴酷狗分享短链 (t1.kugou.com)、歌单码、global_collection_id、微风口令 (#KGHash:...) 或多行「歌名 - 歌手」文本列表：
          </p>
          <textarea
            v-model="importInputText"
            class="dialog-input"
            style="height: 110px; padding: 12px 14px; resize: none; line-height: 1.5"
            placeholder="粘贴酷狗分享链接、口令或多行「歌名 - 歌手」列表..."
          ></textarea>
          <div class="dialog-actions">
            <button class="frameless-hover-btn" @click="showImportModal = false">取消</button>
            <button class="frameless-hover-btn primary-tint" @click="submitImportPlaylist">
              智能解析并导入
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useAppStore } from "../stores/appStore";
import { formatDuration } from "../api/kugou";
import type { FollowedArtistItem, PlaylistItem, PurchasedAlbumItem, Song } from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

type RightStageMode = "liked" | "recent" | "playlist" | "artist" | "album" | "purchased_songs";
type ExpandedTopCard = "none" | "liked" | "recent";
type SongSortMode = "default" | "title" | "artist" | "duration";

const rightStageMode = ref<RightStageMode>("liked");
const artistSubTab = ref<"songs" | "albums" | "bio">("songs");
const expandedTopCard = ref<ExpandedTopCard>("none");
const songFilterKw = ref("");
const songSortMode = ref<SongSortMode>("default");

// 批量多选模式状态
const isBatchMode = ref(false);
const selectedBatchHashes = ref<Set<string>>(new Set());

function toggleBatchMode() {
  isBatchMode.value = !isBatchMode.value;
  selectedBatchHashes.value = new Set();
}

function toggleBatchSong(hash: string) {
  const next = new Set(selectedBatchHashes.value);
  if (next.has(hash)) next.delete(hash);
  else next.add(hash);
  selectedBatchHashes.value = next;
}

function selectAllBatch() {
  if (selectedBatchHashes.value.size === filteredAndSortedRightStageSongs.value.length) {
    selectedBatchHashes.value = new Set();
  } else {
    selectedBatchHashes.value = new Set(
      filteredAndSortedRightStageSongs.value.map((s) => s.hash)
    );
  }
}

function playSelectedBatch() {
  const picked = filteredAndSortedRightStageSongs.value.filter((s) =>
    selectedBatchHashes.value.has(s.hash)
  );
  if (picked.length > 0) {
    store.playSongFromList(picked[0], picked, "批量精选队列");
    isBatchMode.value = false;
  }
}

async function likeSelectedBatch() {
  const picked = filteredAndSortedRightStageSongs.value.filter((s) =>
    selectedBatchHashes.value.has(s.hash)
  );
  for (const s of picked) {
    if (!store.isSongLiked(s.hash)) {
      await store.toggleFavoriteSong(s);
    }
  }
  store.notify(`已将 ${picked.length} 首歌曲加入我喜欢 ❤️`);
  isBatchMode.value = false;
}

function formatFansCount(cnt: number): string {
  if (!cnt || cnt <= 0) return "10w+";
  if (cnt >= 10000) return `${(cnt / 10000).toFixed(1)}w`;
  return String(cnt);
}

// 同步跨视图跳转状态：当从播放页、右键菜单或搜索页打开歌手、专辑或歌单时，自动切换 rightStageMode
function syncFromStoreDetailMode() {
  const mode = store.libraryDetailMode;
  if (mode === "artist" || mode === "artist_detail") {
    expandedTopCard.value = "none";
    rightStageMode.value = "artist";
    artistSubTab.value = "songs";
  } else if (mode === "album" || mode === "album_detail") {
    expandedTopCard.value = "none";
    rightStageMode.value = "album";
  } else if (mode === "playlist" && store.selectedPlaylist) {
    if (store.selectedPlaylist.isLiked) {
      rightStageMode.value = "liked";
    } else {
      expandedTopCard.value = "none";
      rightStageMode.value = "playlist";
    }
  }
}

watch(
  [
    () => store.libraryDetailMode,
    () => store.selectedPlaylist?.id,
    () => store.activeArtistDetail?.artistId,
    () => store.activeAlbumDetail?.albumId,
  ],
  () => {
    syncFromStoreDetailMode();
  },
  { immediate: true }
);

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
    const sec = s.durationSeconds || Math.floor((s.durationMs || 0) / 1000);
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
  store.libraryDetailMode = "playlists";
  expandedTopCard.value = expandedTopCard.value === "liked" ? "none" : "liked";
  songFilterKw.value = "";
}

function activateRecentMode() {
  rightStageMode.value = "recent";
  store.libraryDetailMode = "playlists";
  expandedTopCard.value = expandedTopCard.value === "recent" ? "none" : "recent";
  songFilterKw.value = "";
  if (store.recentHistorySongs.length === 0) {
    store.loadRecentHistory();
  }
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
  artistSubTab.value = "songs";
  songFilterKw.value = "";
  await store.openArtistDetail(String(art.singerId || (art as any).artistId || ""), art.name, art.avatarUrl);
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
  // 拦截浏览器原生图片右键菜单
}

// ==================== 鼠标左键长按拖动排序引擎 ====================
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
      const da = a.durationSeconds || Math.floor((a.durationMs || 0) / 1000);
      const db = b.durationSeconds || Math.floor((b.durationMs || 0) / 1000);
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

watch(songListStreamKey, () => {
  const scroller = document.querySelector(".lib-right-songs-scroller") as HTMLElement | null;
  if (scroller) scroller.scrollTop = 0;
  nextTick(() => {
    const el = document.querySelector(".lib-right-songs-scroller") as HTMLElement | null;
    if (el) el.scrollTop = 0;
  });
});

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
  syncFromStoreDetailMode();
}

function confirmDeletePlaylist(pl: PlaylistItem) {
  if (window.confirm(`确定要删除自建歌单「${pl.name || pl.title}」吗？`)) {
    store.deletePlaylist(pl);
  }
}

onMounted(() => {
  syncFromStoreDetailMode();
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
