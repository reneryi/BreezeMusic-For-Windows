<template>
  <!-- 顶层全屏无框悬浮交互层：不切割背景，下滑浏览时悬浮胶囊自然遮挡下方内容 -->
  <div class="skyline-floating-overlay">
    <!-- 顶部边缘拖拽热区 -->
    <div class="skyline-top-drag-strip" data-tauri-drag-region></div>

    <!-- 播放页左上角悬浮返回按钮（仅在进入全屏播放页时显示） -->
    <button
      v-if="store.isNowPlayingOpen"
      class="skyline-np-floating-back"
      title="收起播放页 (Esc)"
      @click="store.isNowPlayingOpen = false"
    >
      <PlumpIcon name="chevron-down" :size="18" />
      <span>收起舞台</span>
    </button>

    <!-- 居中悬浮岛：顶部导航胶囊 + 响应式搜索框（小窗口自动收缩为圆形按钮，无快捷键标签） -->
    <div class="skyline-floating-center">
      <nav class="skyline-nav-capsule" aria-label="主导航">
        <button
          v-for="item in navItems"
          :key="item.id"
          class="skyline-nav-btn"
          :class="{ active: store.activeTab === item.id && !store.isNowPlayingOpen }"
          :title="item.hint"
          @click="handleNavClick(item.id)"
          @dblclick.stop="handleNavDblClick(item.id)"
        >
          <PlumpIcon :name="item.icon" :size="17" />
          <span>{{ item.label }}</span>
        </button>
      </nav>

      <!-- 响应式悬浮搜索组件：大窗为无快捷键胶囊输入条，小窗收缩为圆形悬浮按钮 -->
      <div class="skyline-search-wrap" ref="searchBoxRef">
        <!-- 小窗口且未展开时：显示为精致圆形悬浮按钮 -->
        <button
          v-if="isCompactWindow && !compactSearchExpanded"
          class="skyline-search-orb-btn"
          title="展开搜索 (Ctrl + K)"
          @click="expandCompactSearch"
        >
          <PlumpIcon name="search" :size="18" />
        </button>

        <!-- 宽窗口常驻 或 小窗口点击展开后：显示悬浮搜索输入胶囊 -->
        <div
          v-else
          class="skyline-search-pill"
          :class="{
            focused: searchFocused,
            'compact-expanded-pill': isCompactWindow && compactSearchExpanded,
          }"
        >
          <PlumpIcon name="search" :size="17" class="search-pill-icon" />
          <input
            ref="searchInputRef"
            v-model="store.searchKeyword"
            type="text"
            class="skyline-search-input"
            placeholder="搜索歌曲、歌手、专辑、歌单..."
            @focus="onSearchFocus"
            @input="onSearchInput"
            @keydown.enter="triggerSearch(store.searchKeyword)"
            @keydown.esc="collapseCompactSearch"
          />
          <button
            v-if="store.searchKeyword || (isCompactWindow && compactSearchExpanded)"
            class="search-clear-mini"
            title="清空或收起"
            @click.stop="handleClearOrCollapseSearch"
          >
            <PlumpIcon name="close" :size="14" />
          </button>
        </div>

        <!-- 实时联想词、历史搜索与热搜榜下拉面板 -->
        <transition name="fade-pop">
          <div
            v-if="searchFocused && (store.searchSuggestions.length > 0 || store.searchHistory.length > 0 || store.hotKeywords.length > 0)"
            class="search-suggest-dropdown glass-popover"
          >
            <template v-if="store.searchKeyword.trim() && store.searchSuggestions.length > 0">
              <div class="suggest-group-title">实时搜索联想</div>
              <button
                v-for="(sug, i) in store.searchSuggestions"
                :key="i"
                class="suggest-row"
                @click="triggerSearch(sug)"
              >
                <PlumpIcon name="search" :size="15" />
                <span>{{ sug }}</span>
              </button>
            </template>
            <template v-else>
              <template v-if="store.searchHistory.length > 0">
                <div class="suggest-group-title">
                  <span>最近搜索</span>
                  <button class="text-link-btn" @click.stop="store.clearSearchHistory()">清空</button>
                </div>
                <div class="suggest-chips">
                  <button
                    v-for="(h, idx) in store.searchHistory.slice(0, 8)"
                    :key="idx"
                    class="suggest-chip"
                    @click="triggerSearch(h)"
                  >
                    {{ h }}
                  </button>
                </div>
              </template>

              <template v-if="store.hotKeywords.length > 0">
                <div class="suggest-group-title" style="margin-top: 4px">
                  <span>全网热搜</span>
                </div>
                <div class="sd-hot-grid">
                  <button
                    v-for="(hk, hIdx) in store.hotKeywords.slice(0, 8)"
                    :key="hIdx"
                    class="sd-hot-item"
                    @click="triggerSearch(hk)"
                  >
                    <span class="sd-hot-rank" :class="{ top: hIdx < 3 }">{{ hIdx + 1 }}</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{{ hk }}</span>
                  </button>
                </div>
              </template>
            </template>
          </div>
        </transition>
      </div>
    </div>

    <!-- 右上角悬浮区：左侧【头像+等级+VIP 融合可展开动效卡片】 + 右侧【明暗切换 + 三个窗口控制按钮统一长形悬浮胶囊】 -->
    <div class="skyline-floating-right">
      <!-- 1. 头像 & VIP 权益中心融合动效容器 -->
      <div
        ref="profileCardRef"
        class="profile-vip-morph-wrapper"
      >
        <div
          class="profile-vip-morph-card"
          :class="{ 'is-expanded': store.isVipCenterOpen }"
          @click="handleProfileCapsuleClick"
        >
          <!-- 顶部区域：收起态显示【头像 + 等级 + VIP标识】；展开态头像平滑放大至左上方，右侧展示【昵称 + 等级 + 听歌时长 + VIP标识】 -->
          <div class="pvm-top-row">
            <div class="pvm-avatar-shell">
              <img
                v-if="store.profile?.avatarUrl"
                :src="store.profile.avatarUrl"
                class="pvm-avatar-img"
                alt="avatar"
              />
              <div v-else class="pvm-avatar-fallback">
                <PlumpIcon name="user" :size="store.isVipCenterOpen ? 24 : 16" color="#fff" />
              </div>
              <span v-if="store.isLoggedIn" class="pvm-online-dot"></span>
            </div>

            <!-- 收起态紧凑标签组：仅显示等级与 VIP 标识 -->
            <div v-if="!store.isVipCenterOpen" class="pvm-collapsed-badges">
              <span class="pvm-lv-badge">
                Lv.{{ store.profile?.userLevel || 1 }}
              </span>
              <span
                class="pvm-vip-badge"
                :class="{ active: store.vipStatus?.isVip || store.vipStatus?.todayClaimed }"
              >
                <PlumpIcon name="crown" :size="13" />
                <span>VIP</span>
              </span>
            </div>

            <!-- 展开态卡片右上方信息区：昵称、等级、听歌时长、VIP标识与操作 -->
            <div v-else class="pvm-expanded-user-meta" @click.stop>
              <div class="pvm-name-row">
                <span class="pvm-nickname">
                  {{ store.isLoggedIn ? store.profile?.nickname : "未登录概念版账号" }}
                </span>
                <div class="pvm-top-mini-actions">
                  <button
                    v-if="!store.isLoggedIn"
                    class="pvm-mini-link"
                    @click.stop="openLoginFromCard"
                  >
                    去登录
                  </button>
                  <button
                    v-else
                    class="pvm-mini-link"
                    title="退出当前账号"
                    @click.stop="store.logout()"
                  >
                    退出
                  </button>
                  <button
                    class="pvm-close-btn"
                    title="收起卡片"
                    @click.stop="store.isVipCenterOpen = false"
                  >
                    <PlumpIcon name="close" :size="14" />
                  </button>
                </div>
              </div>

              <div class="pvm-stats-badges-row">
                <span class="pvm-lv-badge lg">
                  Lv.{{ store.profile?.userLevel || 1 }}
                </span>
                <span class="pvm-duration-badge" title="累计听歌时长（每 60 秒实时上报概念版云端）">
                  <PlumpIcon name="clock" :size="13" />
                  <span>{{ listenHoursText }}</span>
                </span>
                <span
                  class="pvm-vip-badge lg"
                  :class="{ active: store.vipStatus?.isVip || store.vipStatus?.todayClaimed }"
                >
                  <PlumpIcon name="crown" :size="13" />
                  <span>
                    {{
                      store.vipStatus?.svipActive
                        ? "概念 VIP"
                        : store.vipStatus?.tvipActive
                        ? "畅听 VIP"
                        : "VIP 权益"
                    }}
                  </span>
                </span>
              </div>
            </div>
          </div>

          <!-- 展开态下方内容：精简版 VIP 有效期 + 一键签到 + 当月签到热力图 + 打开应用自动领取开关 + 设备鉴权诊断折叠台 -->
          <transition name="pvm-body-fade">
            <div v-if="store.isVipCenterOpen" class="pvm-expanded-body" @click.stop>
              <!-- 1. 精简版 VIP 有效期与每日签到领取 -->
              <div class="pvm-validity-box">
                <div class="pvm-validity-lines">
                  <div class="pvm-val-row">
                    <span class="pvm-val-label">概念 VIP</span>
                    <span class="pvm-val-date">
                      {{ store.vipStatus?.svipActive ? `有效至 ${conciseDate(store.vipStatus.svipExpiry)}` : "签到后升级激活" }}
                    </span>
                  </div>
                  <div class="pvm-val-row">
                    <span class="pvm-val-label">畅听 VIP</span>
                    <span class="pvm-val-date">
                      {{ store.vipStatus?.tvipActive ? `有效至 ${conciseDate(store.vipStatus.tvipExpiry)}` : "每日免费签到即领" }}
                    </span>
                  </div>
                </div>

                <button
                  class="pvm-claim-btn"
                  :disabled="store.isClaimingVip"
                  @click.stop="store.claimVipDaily(false)"
                >
                  <PlumpIcon name="crown" :size="15" color="#1a1200" />
                  <span>
                    {{
                      store.isClaimingVip
                        ? "正在签到..."
                        : store.vipStatus?.todayClaimed
                        ? "今日已领 · 核验"
                        : "领 1 天 VIP"
                    }}
                  </span>
                </button>
              </div>

              <!-- 2. 当月签到热力图 -->
              <div class="pvm-heatmap-section">
                <div class="pvm-heatmap-head">
                  <span class="pvm-hm-title">{{ currentMonth }} 月签到热力图</span>
                  <span class="pvm-hm-count">
                    已领 {{ store.vipStatus?.monthClaimedDays || (store.vipStatus?.todayClaimed ? 1 : 0) }} / {{ daysInMonth }} 天
                  </span>
                </div>

                <div class="pvm-heatmap-grid">
                  <div
                    v-for="cell in calendarCells"
                    :key="cell.day"
                    class="pvm-hm-cell"
                    :class="{
                      claimed: cell.claimed,
                      svip: cell.isSvip,
                      today: cell.isToday,
                    }"
                    :title="`${cell.dateStr}${cell.claimed ? ' · 已签到' : ''}`"
                  >
                    {{ cell.day }}
                  </div>
                </div>

                <div class="pvm-hm-legend">
                  <span><i class="hm-dot gold"></i>概念VIP</span>
                  <span><i class="hm-dot cyan"></i>畅听VIP</span>
                  <span><i class="hm-dot green"></i>今日</span>
                </div>
              </div>

              <!-- 3. 打开应用自动领取开关 -->
              <div class="pvm-auto-claim-row">
                <div class="pvm-ac-text">
                  <span class="pvm-ac-title">打开应用时自动领取 VIP</span>
                  <span class="pvm-ac-sub">启动时静默检测并签到当日概念版权益</span>
                </div>
                <label class="plump-switch sm">
                  <input v-model="store.autoClaimVipOnStartup" type="checkbox" />
                  <span class="plump-slider"></span>
                </label>
              </div>

              <!-- 4. 设备标识与音源鉴权诊断折叠栏 -->
              <div class="pvm-diag-footer">
                <button
                  class="pvm-diag-toggle-btn"
                  @click.stop="showDeviceDiag = !showDeviceDiag"
                >
                  <PlumpIcon name="shield" :size="14" color="var(--cyan)" />
                  <span>设备凭证与音源网关诊断</span>
                  <span style="margin-left: auto; opacity: 0.7">{{ showDeviceDiag ? "收起" : "展开" }}</span>
                </button>

                <div v-if="showDeviceDiag" class="pvm-diag-panel">
                  <div class="pvm-diag-kv">
                    <span>DFID 指纹</span>
                    <code>{{ formatShortCred(store.vipStatus?.dfid || store.gatewayDiag?.dfid) }}</code>
                  </div>
                  <div class="pvm-diag-kv">
                    <span>GUID 标识</span>
                    <code>{{ formatShortCred(store.vipStatus?.guid || store.gatewayDiag?.guid) }}</code>
                  </div>
                  <div class="pvm-diag-actions">
                    <button
                      class="tonal-pill-btn sm"
                      :disabled="store.isDiagnosingGateway"
                      @click.stop="store.resignDeviceAndRefresh()"
                    >
                      <PlumpIcon name="refresh" :size="13" />
                      <span>刷新设备指纹</span>
                    </button>
                    <button
                      class="tonal-pill-btn sm primary"
                      :disabled="store.isDiagnosingGateway"
                      @click.stop="store.runGatewayDiagnostic()"
                    >
                      <PlumpIcon name="wave" :size="13" />
                      <span>
                        {{
                          store.gatewayDiag
                            ? `网关正常 (${store.gatewayDiag.latencyMs}ms)`
                            : "网关测速诊断"
                        }}
                      </span>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </transition>
        </div>
      </div>

      <!-- 2. 明暗切换按钮 + 三个窗口控制按钮：统一设计为长形悬浮胶囊，与导航栏视觉高度一致 -->
      <div class="skyline-win-floating-capsule">
        <button
          class="win-capsule-btn theme-btn"
          title="切换明亮 / 深邃外观模式"
          @click="toggleColorScheme"
        >
          <PlumpIcon
            :name="store.colorScheme === 'watercolor_light' || store.colorScheme === 'light' ? 'moon' : 'sun'"
            :size="16"
          />
        </button>

        <span class="win-capsule-divider"></span>

        <button class="win-capsule-btn" title="最小化" @click="minimizeWin">
          <PlumpIcon name="minimize" :size="14" />
        </button>
        <button class="win-capsule-btn" title="最大化 / 还原" @click="toggleMaximizeWin">
          <PlumpIcon name="maximize" :size="14" />
        </button>
        <button class="win-capsule-btn close" title="关闭窗口" @click="closeWin">
          <PlumpIcon name="close" :size="14" />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useAppStore, type MainTab } from "../stores/appStore";
import { kugouClient } from "../api/kugou";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

const navItems: Array<{
  id: MainTab;
  label: string;
  hint: string;
  icon: "compass" | "tuning" | "library" | "settings";
}> = [
  { id: "discover", label: "发现", hint: "单击进入发现页 · 双击整体刷新发现页内容", icon: "compass" },
  { id: "library", label: "我的", hint: "单击进入我的音乐 · 双击同步云端音乐资产", icon: "library" },
  { id: "radio", label: "调音台", hint: "概念版母带声学调音台与蝰蛇音效工作站", icon: "tuning" },
  { id: "settings", label: "设置", hint: "外观材质、播放偏好、设备凭证与下载设置", icon: "settings" },
];

// 窗口宽度监听：小窗口/非全屏窗口（<=1400px）时搜索框自动收缩为圆形按钮
const windowWidth = ref(typeof window !== "undefined" ? window.innerWidth : 1240);
const isCompactWindow = computed(() => windowWidth.value <= 1400);
const compactSearchExpanded = ref(false);
const showDeviceDiag = ref(false);

const searchFocused = ref(false);
const searchBoxRef = ref<HTMLElement | null>(null);
const searchInputRef = ref<HTMLInputElement | null>(null);
const profileCardRef = ref<HTMLElement | null>(null);
let suggestTimer: number | null = null;

function formatShortCred(val?: string) {
  if (!val) return "已就绪";
  if (val.length <= 12) return val;
  return `${val.slice(0, 6)}••••${val.slice(-4)}`;
}

function onWindowResize() {
  windowWidth.value = window.innerWidth;
  if (!isCompactWindow.value) {
    compactSearchExpanded.value = false;
  }
}

function expandCompactSearch() {
  compactSearchExpanded.value = true;
  nextTick(() => {
    searchInputRef.value?.focus();
  });
}

function collapseCompactSearch() {
  searchFocused.value = false;
  if (isCompactWindow.value && !store.searchKeyword.trim()) {
    compactSearchExpanded.value = false;
  }
}

function handleClearOrCollapseSearch() {
  if (store.searchKeyword) {
    store.searchKeyword = "";
    store.searchSuggestions = [];
    searchInputRef.value?.focus();
  } else {
    compactSearchExpanded.value = false;
    searchFocused.value = false;
  }
}

function handleNavClick(id: MainTab) {
  store.isNowPlayingOpen = false;
  if (id === "discover" && store.activeTab === "discover") {
    store.discoverSubView = "home";
  }
  store.activeTab = id;
}

function handleNavDblClick(id: MainTab) {
  store.isNowPlayingOpen = false;
  store.activeTab = id;
  if (id === "discover") {
    store.discoverSubView = "home";
    store.refreshDiscoverAll();
  } else if (id === "library") {
    store.notify("正在同步云端音乐资产与听歌等级...");
    Promise.all([
      store.refreshAccountAndVip(false),
      store.loadUserPlaylists(),
      store.loadFollowedArtists(),
      store.loadPurchasedAssets(),
    ]).then(() => {
      store.notify("云端音乐资产同步完成 ✨");
    });
  }
}

function onSearchFocus() {
  searchFocused.value = true;
}

function onSearchInput() {
  if (suggestTimer) window.clearTimeout(suggestTimer);
  const kw = store.searchKeyword.trim();
  if (!kw) {
    store.searchSuggestions = [];
    return;
  }
  suggestTimer = window.setTimeout(async () => {
    try {
      store.searchSuggestions = await kugouClient.getSearchSuggest(kw);
    } catch {
      store.searchSuggestions = [];
    }
  }, 180);
}

function triggerSearch(kw: string) {
  const q = kw.trim();
  if (!q) return;
  searchFocused.value = false;
  compactSearchExpanded.value = false;
  store.executeSearch(q);
}

function handleProfileCapsuleClick() {
  if (!store.isVipCenterOpen) {
    store.isVipCenterOpen = true;
  }
}

function openLoginFromCard() {
  store.isVipCenterOpen = false;
  store.isLoginModalOpen = true;
}

const listenHoursText = computed(() => {
  const mins =
    store.profile?.listenDurationMinutes ??
    store.profile?.listenDurationMin ??
    0;
  return `${Math.round(mins / 60)}h`;
});

function conciseDate(raw?: string) {
  if (!raw) return "未激活";
  return raw.slice(0, 10);
}

const now = new Date();
const currentYear = now.getFullYear();
const currentMonth = now.getMonth() + 1;
const todayDay = now.getDate();

const daysInMonth = computed(() => new Date(currentYear, currentMonth, 0).getDate());

const calendarCells = computed(() => {
  const recs = store.vipStatus?.records || [];
  const cells: Array<{
    day: number;
    dateStr: string;
    claimed: boolean;
    isSvip: boolean;
    isToday: boolean;
  }> = [];

  for (let d = 1; d <= daysInMonth.value; d++) {
    const mm = String(currentMonth).padStart(2, "0");
    const dd = String(d).padStart(2, "0");
    const dateStr = `${currentYear}-${mm}-${dd}`;
    const found = recs.find((r) => r.day === dateStr && r.received);
    cells.push({
      day: d,
      dateStr,
      claimed: Boolean(found) || (d === todayDay && Boolean(store.vipStatus?.todayClaimed)),
      isSvip: found?.vipType === "svip" || Boolean(store.vipStatus?.svipActive),
      isToday: d === todayDay,
    });
  }
  return cells;
});

function toggleColorScheme() {
  if (store.colorScheme === "watercolor_light" || store.colorScheme === "light") {
    store.colorScheme = "abyssal_dark";
    store.notify("已切换为深邃夜幕外观");
  } else {
    store.colorScheme = "watercolor_light";
    store.notify("已切换为纯净微风浅色外观");
  }
}

async function minimizeWin() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  } catch {
    store.notify("浏览器预览模式：最小化窗口");
  }
}

async function toggleMaximizeWin() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().toggleMaximize();
  } catch {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen().catch(() => {});
    } else {
      document.exitFullscreen().catch(() => {});
    }
  }
}

async function closeWin() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    if (store.closeToTray) {
      await getCurrentWindow().hide();
      store.notify("微风音乐已最小化至系统托盘");
    } else {
      await getCurrentWindow().close();
    }
  } catch {
    store.notify("浏览器预览模式：关闭窗口");
  }
}

function handleOutsideClick(e: MouseEvent) {
  const target = e.target as Node;
  if (searchBoxRef.value && !searchBoxRef.value.contains(target)) {
    searchFocused.value = false;
    if (isCompactWindow.value && !store.searchKeyword.trim()) {
      compactSearchExpanded.value = false;
    }
  }
  if (
    store.isVipCenterOpen &&
    profileCardRef.value &&
    !profileCardRef.value.contains(target)
  ) {
    store.isVipCenterOpen = false;
  }
}

onMounted(() => {
  onWindowResize();
  window.addEventListener("mousedown", handleOutsideClick);
  window.addEventListener("resize", onWindowResize);
});

onBeforeUnmount(() => {
  window.removeEventListener("mousedown", handleOutsideClick);
  window.removeEventListener("resize", onWindowResize);
});
</script>
