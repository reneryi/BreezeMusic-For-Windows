<script setup lang="ts">
import { computed, ref } from "vue";
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();
const showPlainCredentials = ref(false);

const now = new Date();
const currentYear = now.getFullYear();
const currentMonth = now.getMonth() + 1;
const todayDay = now.getDate();

const daysInMonth = computed(() => {
  return new Date(currentYear, currentMonth, 0).getDate();
});

const calendarCells = computed(() => {
  const recs = store.vipStatus?.records || [];
  const cells: {
    day: number;
    dateStr: string;
    claimed: boolean;
    isSvip: boolean;
    isToday: boolean;
  }[] = [];

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

function formatCred(val?: string) {
  if (!val) return "已就绪";
  if (showPlainCredentials.value) return val;
  if (val.length <= 10) return val;
  return `${val.slice(0, 6)}••••••${val.slice(-4)}`;
}

async function copyCred(label: string, val?: string) {
  if (!val) return;
  try {
    await navigator.clipboard.writeText(val);
    store.notify(`已复制 ${label} 至剪贴板`);
  } catch {
    store.notify(`${label}: ${val}`);
  }
}
</script>

<template>
  <transition name="fade">
    <div
      v-if="store.isVipCenterOpen"
      class="modal-backdrop"
      @click.self="store.isVipCenterOpen = false"
    >
      <div class="vip-modal-card vip-dual-modal">
        <div class="modal-header">
          <div class="vip-title-row">
            <div class="vip-icon-orb">
              <PlumpIcon name="crown" :size="20" color="#ffd54f" />
            </div>
            <div>
              <h3>VIP 权益与鉴权中心</h3>
              <p class="vip-sub-caption">
                酷狗概念版 (appid=3116) 每日免费领无损 VIP · 设备鉴权诊断
              </p>
            </div>
          </div>
          <button class="icon-close-btn" @click="store.isVipCenterOpen = false">
            <PlumpIcon name="close" :size="16" />
          </button>
        </div>

        <!-- 桌面端双栏黑金布局：左栏会员状态与自动签到 + 右栏月度签到日历与设备指纹诊断 -->
        <div class="vip-dual-grid">
          <!-- 左栏 -->
          <div class="vip-left-col">
            <div class="vip-hero-banner">
              <div class="vip-hero-top">
                <div>
                  <div class="vip-badge-pill">CONCEPT EDITION · 3116</div>
                  <h4 class="vip-hero-title">
                    {{
                      store.vipStatus?.svipActive
                        ? "概念版 VIP 会员生效中"
                        : store.vipStatus?.tvipActive
                        ? "畅听 VIP 会员生效中"
                        : "每日签到免费领概念版 VIP"
                    }}
                  </h4>
                  <p class="vip-hero-sub">
                    {{
                      store.vipStatus?.statusMessage ||
                      "登录后每日可免费签到领取 1 天畅听 VIP 并升级概念版权益"
                    }}
                  </p>
                </div>
              </div>

              <div class="vip-benefit-rows">
                <div class="vip-benefit-row">
                  <PlumpIcon name="check" :size="16" accent="#ffd54f" />
                  <span>
                    畅听 VIP：{{
                      store.vipStatus?.tvipActive
                        ? `有效 · 到期 ${store.vipStatus.tvipExpiry}`
                        : "签到立即激活"
                    }}
                  </span>
                </div>
                <div class="vip-benefit-row">
                  <PlumpIcon name="check" :size="16" accent="#00e5ff" />
                  <span>
                    概念 VIP：{{
                      store.vipStatus?.svipActive
                        ? `有效 · 到期 ${store.vipStatus.svipExpiry}`
                        : "签到后自动触发激励升级"
                    }}
                  </span>
                </div>
              </div>

              <button
                class="vip-gold-claim-cta"
                :disabled="store.isClaimingVip"
                @click="store.claimVipDaily(false)"
              >
                <PlumpIcon name="crown" :size="17" color="#1a1200" />
                <span>
                  {{
                    store.isClaimingVip
                      ? "正在签到并升级概念版 VIP..."
                      : store.vipStatus?.todayClaimed
                      ? "今日已领 · 再次核验升级状态"
                      : "一键领取今日概念版 VIP"
                  }}
                </span>
              </button>
            </div>

            <!-- 启动自动签到守护卡片（对齐 Android VipCenterBottomSheet） -->
            <div class="vip-auto-guard-card">
              <div class="guard-info">
                <div class="guard-title">打开应用时自动静默领取</div>
                <div class="guard-desc">
                  启动客户端时自动检测并静默签到当日 1 天概念版 VIP 权益
                </div>
              </div>
              <label class="plump-switch">
                <input
                  v-model="store.autoClaimVipOnStartup"
                  type="checkbox"
                />
                <span class="plump-slider"></span>
              </label>
            </div>
          </div>

          <!-- 右栏：当月签到日历矩阵 + 设备标识与音源鉴权凭证 -->
          <div class="vip-right-col">
            <div class="vip-calendar-card">
              <div class="cal-header">
                <span class="cal-title">
                  {{ currentYear }} 年 {{ currentMonth }} 月签到日历
                </span>
                <span class="cal-count-pill">
                  本月已领 {{ store.vipStatus?.monthClaimedDays || (store.vipStatus?.todayClaimed ? 1 : 0) }} /
                  {{ daysInMonth }} 天
                </span>
              </div>

              <div class="cal-grid">
                <div
                  v-for="cell in calendarCells"
                  :key="cell.day"
                  class="cal-day-cell"
                  :class="{
                    claimed: cell.claimed,
                    svip: cell.isSvip,
                    today: cell.isToday,
                  }"
                >
                  {{ cell.day }}
                </div>
              </div>

              <div class="cal-legend">
                <span><i class="dot gold"></i> 概念 VIP</span>
                <span><i class="dot cyan"></i> 畅听 VIP</span>
                <span><i class="dot green"></i> 今日</span>
              </div>
            </div>

            <!-- 设备标识与音源鉴权凭证台 -->
            <div class="vip-device-card">
              <div class="dev-card-head">
                <div class="dev-head-left">
                  <PlumpIcon name="shield" :size="16" color="#00e5ff" />
                  <span>设备标识与音源鉴权凭证</span>
                </div>
                <button
                  class="dev-toggle-plain"
                  @click="showPlainCredentials = !showPlainCredentials"
                >
                  {{ showPlainCredentials ? "隐藏明文" : "展开明文" }}
                </button>
              </div>

              <div class="dev-kv-row">
                <span>DFID 设备指纹</span>
                <div class="dev-kv-right">
                  <code>{{ formatCred(store.vipStatus?.dfid || store.gatewayDiag?.dfid) }}</code>
                  <button
                    class="dev-copy-btn"
                    @click="copyCred('DFID', store.vipStatus?.dfid)"
                  >
                    复制
                  </button>
                </div>
              </div>

              <div class="dev-kv-row">
                <span>GUID 会话标识</span>
                <div class="dev-kv-right">
                  <code>{{ formatCred(store.vipStatus?.guid || store.gatewayDiag?.guid) }}</code>
                  <button
                    class="dev-copy-btn"
                    @click="copyCred('GUID', store.vipStatus?.guid)"
                  >
                    复制
                  </button>
                </div>
              </div>

              <div class="dev-actions-row">
                <button
                  class="tonal-action-btn"
                  :disabled="store.isDiagnosingGateway"
                  @click="store.resignDeviceAndRefresh()"
                >
                  <PlumpIcon name="refresh" :size="14" />
                  <span>刷新设备标识</span>
                </button>
                <button
                  class="tonal-action-btn cyan"
                  :disabled="store.isDiagnosingGateway"
                  @click="store.runGatewayDiagnostic()"
                >
                  <PlumpIcon name="wave" :size="14" />
                  <span>
                    {{
                      store.gatewayDiag
                        ? `链路正常 (${store.gatewayDiag.latencyMs}ms)`
                        : "音源链路诊断"
                    }}
                  </span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>
