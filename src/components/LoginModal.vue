<script setup lang="ts">
import { ref, onUnmounted, watch } from "vue";
import { useAppStore } from "../stores/appStore";
import { kugouClient } from "../api/kugou";
import type { LoginCandidate } from "../types";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

const loginMode = ref<"qr" | "sms">("qr");

// 扫码登录状态 (0=过期, 1=等待扫码, 2=已扫码待确认, 4=授权成功)
const qrImg = ref("");
const qrKey = ref("");
const qrStatusText = ref("正在获取概念版登录二维码...");
const qrStatusCode = ref(1);
let pollTimer: number | null = null;

// 手机验证码登录状态
const mobile = ref("");
const smsCode = ref("");
const smsSending = ref(false);
const smsCooldown = ref(0);
const smsLoggingIn = ref(false);
const errorMsg = ref("");
const accountCandidates = ref<LoginCandidate[]>([]);

let cooldownTimer: number | null = null;

async function startQrLogin() {
  stopPolling();
  errorMsg.value = "";
  qrStatusText.value = "正在请求酷狗概念版二维码...";
  qrImg.value = "";
  qrStatusCode.value = 1;
  try {
    const res = await kugouClient.createQrLogin();
    qrKey.value = res.qrcodeKey;
    qrImg.value = res.qrcodeImg;
    qrStatusCode.value = 1;
    qrStatusText.value = "请使用手机「酷狗音乐 / 酷狗概念版」扫一扫登录";
    pollTimer = window.setInterval(pollQrStatus, 2000);
  } catch (e: any) {
    errorMsg.value = `二维码获取失败：${e}`;
    qrStatusText.value = "二维码加载失败，请点击刷新重试";
  }
}

async function pollQrStatus() {
  if (!qrKey.value) return;
  try {
    const res = await kugouClient.checkQrLogin(qrKey.value);
    qrStatusCode.value = res.status;
    qrStatusText.value = res.message;
    if (res.status === 4 && res.session) {
      stopPolling();
      await store.handleLoginSuccess(res.session, res.profile);
    } else if (res.status === 0) {
      stopPolling();
    }
  } catch (e) {
    console.warn("QR poll error:", e);
  }
}

function stopPolling() {
  if (pollTimer !== null) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

async function handleSendSms() {
  errorMsg.value = "";
  if (!/^1[3-9]\d{9}$/.test(mobile.value.trim())) {
    errorMsg.value = "请输入有效的 11 位中国大陆手机号";
    return;
  }
  smsSending.value = true;
  try {
    const msg = await kugouClient.sendSmsCode(mobile.value.trim());
    store.notify(msg);
    smsCooldown.value = 60;
    cooldownTimer = window.setInterval(() => {
      smsCooldown.value -= 1;
      if (smsCooldown.value <= 0 && cooldownTimer !== null) {
        clearInterval(cooldownTimer);
        cooldownTimer = null;
      }
    }, 1000);
  } catch (e: any) {
    errorMsg.value = `${e}`;
  } finally {
    smsSending.value = false;
  }
}

async function handleSmsLogin(selectedUserId?: string) {
  errorMsg.value = "";
  if (!/^1[3-9]\d{9}$/.test(mobile.value.trim())) {
    errorMsg.value = "请输入有效的 11 位手机号";
    return;
  }
  if (!/^\d{4,8}$/.test(smsCode.value.trim())) {
    errorMsg.value = "请输入收到的短信验证码";
    return;
  }
  smsLoggingIn.value = true;
  try {
    const outcome = await kugouClient.loginWithSms(
      mobile.value.trim(),
      smsCode.value.trim(),
      selectedUserId
    );
    if (outcome.status === "choose_account" && outcome.candidates.length > 0) {
      accountCandidates.value = outcome.candidates;
      return;
    }
    if (outcome.session) {
      accountCandidates.value = [];
      await store.handleLoginSuccess(outcome.session, outcome.profile);
    }
  } catch (e: any) {
    errorMsg.value = `${e}`;
  } finally {
    smsLoggingIn.value = false;
  }
}

watch(
  () => store.isLoginModalOpen,
  (open) => {
    if (open && loginMode.value === "qr") {
      startQrLogin();
    } else {
      stopPolling();
    }
  }
);

watch(loginMode, (mode) => {
  errorMsg.value = "";
  if (mode === "qr") {
    startQrLogin();
  } else {
    stopPolling();
  }
});

onUnmounted(() => {
  stopPolling();
  if (cooldownTimer !== null) {
    clearInterval(cooldownTimer);
  }
});
</script>

<template>
  <transition name="fade">
    <div
      v-if="store.isLoginModalOpen"
      class="modal-backdrop"
      @click.self="store.isLoginModalOpen = false"
    >
      <div class="compact-glass-modal">
        <div class="modal-header">
          <div>
            <h3>登录酷狗概念版账号</h3>
            <p class="vip-sub-caption">
              端侧直连 (appid=3116) · 同步云歌单、听歌等级与每日无损 VIP
            </p>
          </div>
          <button
            class="icon-close-btn"
            @click="store.isLoginModalOpen = false"
          >
            <PlumpIcon name="close" :size="15" />
          </button>
        </div>

        <div class="pill-tabs mb-14 full-w" style="justify-content: center">
          <button
            class="pill-tab-btn"
            :class="{ active: loginMode === 'qr' }"
            @click="loginMode = 'qr'"
          >
            酷狗扫码登录
          </button>
          <button
            class="pill-tab-btn"
            :class="{ active: loginMode === 'sms' }"
            @click="loginMode = 'sms'"
          >
            手机短信验证码登录
          </button>
        </div>

        <div v-if="errorMsg" class="issue-guide-box mb-10" style="color: var(--coral)">
          {{ errorMsg }}
        </div>

        <!-- 1. 二维码扫码登录视图 -->
        <div
          v-if="loginMode === 'qr'"
          style="display: flex; flex-direction: column; align-items: center; gap: 12px"
        >
          <div
            style="
              width: 196px;
              height: 196px;
              border-radius: 22px;
              background: #ffffff;
              padding: 12px;
              display: flex;
              align-items: center;
              justify-content: center;
              position: relative;
            "
          >
            <img
              v-if="qrImg"
              :src="qrImg"
              alt="QR Code"
              style="width: 100%; height: 100%; object-fit: contain"
            />
            <div v-else style="color: #0f1d33; font-weight: 700; font-size: 12.5px">
              正在生成二维码...
            </div>
          </div>
          <p style="font-size: 12.5px; color: var(--text-secondary); text-align: center">
            {{ qrStatusText }}
          </p>
          <button class="tonal-pill-btn" @click="startQrLogin">
            <PlumpIcon name="refresh" :size="14" />
            <span>重新生成二维码</span>
          </button>
        </div>

        <!-- 2. 手机短信验证码登录视图 -->
        <div v-else>
          <template v-if="accountCandidates.length === 0">
            <input
              v-model="mobile"
              type="tel"
              maxlength="11"
              placeholder="请输入 11 位中国大陆手机号"
              class="plump-text-input mb-10"
            />

            <div style="display: flex; gap: 8px" class="mb-14">
              <input
                v-model="smsCode"
                type="text"
                maxlength="8"
                placeholder="短信验证码"
                class="plump-text-input"
                @keyup.enter="handleSmsLogin()"
              />
              <button
                class="tonal-pill-btn"
                style="white-space: nowrap"
                :disabled="smsSending || smsCooldown > 0"
                @click="handleSendSms"
              >
                {{
                  smsCooldown > 0
                    ? `${smsCooldown}s`
                    : smsSending
                    ? "发送中"
                    : "获取验证码"
                }}
              </button>
            </div>

            <button
              class="primary-btn full-w"
              :disabled="smsLoggingIn"
              @click="handleSmsLogin()"
            >
              {{ smsLoggingIn ? "正在验证登录..." : "验证并登录" }}
            </button>
          </template>

          <!-- 同手机号多酷狗账号选择器 -->
          <template v-else>
            <div class="cfg-label">该手机号绑定了多个账号，请选择：</div>
            <div class="playlist-pick-list">
              <button
                v-for="cand in accountCandidates"
                :key="cand.userId"
                class="playlist-pick-row"
                @click="handleSmsLogin(cand.userId)"
              >
                <div class="ppr-left">
                  <img
                    v-if="cand.avatarUrl"
                    :src="cand.avatarUrl"
                    style="width: 36px; height: 36px; border-radius: 50%"
                  />
                  <div>
                    <div class="ppr-title">{{ cand.nickname }}</div>
                    <div class="ppr-count">UID: {{ cand.userId }}</div>
                  </div>
                </div>
                <PlumpIcon name="chevron-right" :size="15" />
              </button>
            </div>
          </template>
        </div>
      </div>
    </div>
  </transition>
</template>
