<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../stores/appStore";
import { Crown, X, CheckCircle2, RefreshCw, Shield } from "lucide-vue-next";

const store = useAppStore();

const now = new Date();
const currentYearMonthText = `${now.getFullYear()}年${String(
  now.getMonth() + 1
).padStart(2, "0")}月 · 领取日历`;
const todayDayNum = now.getDate();

const daysInMonth = computed(() => {
  return new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
});

const calendarCells = computed(() => {
  const recMap = new Map<number, string>();
  const records = store.vipStatus?.records || [];
  for (const r of records) {
    if (r.received && r.day) {
      const parts = r.day.split("-");
      const d = Number(parts[parts.length - 1]);
      if (d > 0) {
        recMap.set(d, (r.vipType || "svip").toLowerCase());
      }
    }
  }
  const cells = [];
  for (let d = 1; d <= daysInMonth.value; d++) {
    const vType = recMap.get(d) || "";
    cells.push({
      day: d,
      isToday: d === todayDayNum,
      claimed: Boolean(vType) || (d === todayDayNum && store.vipStatus?.todayClaimed),
      isSvip: vType === "svip" || (d === todayDayNum && store.vipStatus?.svipActive),
    });
  }
  return cells;
});

function maskId(raw?: string): string {
  if (!raw || raw.length < 10) return raw || "-";
  return `${raw.slice(0, 8)}••••${raw.slice(-6)}`;
}
</script>

<template>
  <div class="modal-backdrop" @click.self="store.isVipCenterOpen = false">
    <div class="glass-modal vip-center-modal">
      <div class="modal-header">
        <div class="vip-modal-title-row">
          <div class="vip-crown-circle">
            <Crown :size="18" />
          </div>
          <h3>VIP 权益中心</h3>
        </div>
        <div class="modal-head-actions">
          <button
            class="icon-btn"
            title="刷新权益状态"
            @click="store.refreshAccountAndVip()"
          >
            <RefreshCw :size="17" />
          </button>
          <button class="icon-btn" @click="store.isVipCenterOpen = false">
            <X :size="20" />
          </button>
        </div>
      </div>

      <!-- 顶部鎏金概念版 VIP 会员卡 -->
      <div class="vip-gold-hero-card">
        <div class="vip-hero-top">
          <div>
            <div class="vip-hero-title">概念版 VIP 会员</div>
            <div class="vip-hero-sub">
              {{
                store.vipStatus?.todayClaimed
                  ? "今日已领：概念版 VIP"
                  : "今日待领取免费无损畅听权益"
              }}
            </div>
          </div>
          <span class="vip-sync-pill">已同步</span>
        </div>

        <div class="vip-benefit-pills">
          <div class="vip-benefit-row">
            <CheckCircle2 :size="14" class="gold-check" />
            <span>
              畅听 VIP：{{
                store.vipStatus?.tvipActive
                  ? `有效 · ${store.vipStatus.tvipExpiry}`
                  : "签到即可激活"
              }}
            </span>
          </div>
          <div class="vip-benefit-row">
            <CheckCircle2 :size="14" class="gold-check" />
            <span>
              概念版权益：{{
                store.vipStatus?.svipActive
                  ? `有效 · ${store.vipStatus.svipExpiry}`
                  : "签到并自动升级"
              }}
            </span>
          </div>
        </div>

        <button
          class="vip-gold-claim-cta"
          :disabled="store.isClaimingVip"
          @click="store.claimVipDaily()"
        >
          <Crown :size="16" />
          <span>
            {{
              store.isClaimingVip
                ? "正在领取并升级概念版 VIP..."
                : "一键领取今日 VIP"
            }}
          </span>
        </button>
      </div>

      <!-- 当月签到领取日历矩阵 -->
      <div class="vip-calendar-card">
        <div class="cal-header">
          <span class="cal-title">{{ currentYearMonthText }}</span>
          <span class="cal-count-pill">
            已领 {{ store.vipStatus?.monthClaimedDays || 0 }} /
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

      <!-- 设备标识与音源鉴权凭证 -->
      <div class="vip-device-card">
        <div class="dev-card-head">
          <Shield :size="15" class="cyan-ico" />
          <span>设备标识与音源鉴权凭证（用于概念版无损直链签发）</span>
        </div>
        <div class="dev-kv-row">
          <span>DFID 指纹</span>
          <code>{{ maskId(store.vipStatus?.dfid) }}</code>
        </div>
        <div class="dev-kv-row">
          <span>GUID 标识</span>
          <code>{{ maskId(store.vipStatus?.guid) }}</code>
        </div>
      </div>
    </div>
  </div>
</template>
