<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { useAppStore } from "../stores/appStore";
import { kugouClient } from "../api/kugou";
import type { LoginCandidate } from "../types";
import {
  X,
  QrCode,
  Smartphone,
  RefreshCw,
  CheckCircle2,
} from "lucide-vue-next";

const store = useAppStore();

const loginMode = ref<"qr" | "sms">("qr");

// 扫码登录状态 (对齐酷狗状态码: 0=过期, 1=等待扫码, 2=已扫码待确认, 4=授权成功)
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
    // status === 4 代表用户在手机 App 中点击了确认授权并已完成 /v5/login_by_token 补全！
    if (res.status === 4 && res.session) {
      stopPolling();
      await store.onLoginSuccess(res.session, res.profile);
    } else if (res.status === 0) {
      // status === 0 代表二维码过期
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
    store.showBanner(msg);
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
      await store.onLoginSuccess(outcome.session, outcome.profile);
    }
  } catch (e: any) {
    errorMsg.value = `${e}`;
  } finally {
    smsLoggingIn.value = false;
  }
}

watch(loginMode, (mode) => {
  errorMsg.value = "";
  if (mode === "qr") {
    startQrLogin();
  } else {
    stopPolling();
  }
});

onMounted(() => {
  if (loginMode.value === "qr") {
    startQrLogin();
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
  <div class="modal-backdrop" @click.self="store.isLoginModalOpen = false">
    <div class="glass-modal login-modal">
      <div class="modal-header">
        <div>
          <h3 class="modal-title">登录酷狗概念版账号</h3>
          <p class="modal-subtitle">
            端侧直连 (appid=3116) · 已修复扫码确认状态码与凭证自动补全
          </p>
        </div>
        <button class="icon-btn" @click="store.isLoginModalOpen = false">
          <X :size="20" />
        </button>
      </div>

      <div class="mode-tabs">
        <button
          class="mode-tab"
          :class="{ active: loginMode === 'qr' }"
          @click="loginMode = 'qr'"
        >
          <QrCode :size="16" />
          <span>酷狗扫码登录</span>
        </button>
        <button
          class="mode-tab"
          :class="{ active: loginMode === 'sms' }"
          @click="loginMode = 'sms'"
        >
          <Smartphone :size="16" />
          <span>手机短信登录</span>
        </button>
      </div>

      <div v-if="errorMsg" class="error-alert">{{ errorMsg }}</div>

      <!-- 1. 二维码扫码登录视图 -->
      <div v-if="loginMode === 'qr'" class="qr-panel">
        <div class="qr-box">
          <img v-if="qrImg" :src="qrImg" alt="QR Code" class="qr-image" />
          <div v-else class="qr-loading">正在生成二维码...</div>

          <!-- status === 2: 已扫码，等待在手机 App 点击确认 -->
          <div v-if="qrStatusCode === 2" class="qr-overlay scanned">
            <CheckCircle2 :size="36" />
            <span>扫码成功</span>
            <small>请在手机酷狗 App 中点击「确认登录」</small>
          </div>

          <!-- status === 0: 二维码已过期 -->
          <div v-if="qrStatusCode === 0" class="qr-overlay expired">
            <button class="primary-btn" @click="startQrLogin">
              <RefreshCw :size="15" />
              <span>二维码已过期 · 点击刷新</span>
            </button>
          </div>
        </div>
        <p class="qr-hint">{{ qrStatusText }}</p>
        <button class="ghost-btn" @click="startQrLogin">
          <RefreshCw :size="14" />
          <span>重新生成二维码</span>
        </button>
      </div>

      <!-- 2. 手机短信验证码登录视图 -->
      <div v-else class="sms-panel">
        <template v-if="accountCandidates.length === 0">
          <div class="form-group">
            <label>手机号码</label>
            <input
              v-model="mobile"
              type="tel"
              maxlength="11"
              placeholder="请输入 11 位中国大陆手机号"
              class="text-input"
            />
          </div>

          <div class="form-group">
            <label>短信验证码</label>
            <div class="sms-code-row">
              <input
                v-model="smsCode"
                type="text"
                maxlength="8"
                placeholder="请输入短信验证码"
                class="text-input"
                @keyup.enter="handleSmsLogin()"
              />
              <button
                class="secondary-btn"
                :disabled="smsSending || smsCooldown > 0"
                @click="handleSendSms"
              >
                {{
                  smsCooldown > 0
                    ? `${smsCooldown}s 后重发`
                    : smsSending
                    ? "发送中..."
                    : "获取验证码"
                }}
              </button>
            </div>
          </div>

          <button
            class="primary-btn full-width submit-btn"
            :disabled="smsLoggingIn"
            @click="handleSmsLogin()"
          >
            {{ smsLoggingIn ? "正在验证登录..." : "验证并登录" }}
          </button>
        </template>

        <!-- 同手机号多酷狗账号选择器 -->
        <template v-else>
          <div class="candidate-title">
            该手机号下绑定了多个酷狗账号，请选择要登录的账号：
          </div>
          <div class="candidate-list">
            <div
              v-for="cand in accountCandidates"
              :key="cand.userId"
              class="candidate-item"
              @click="handleSmsLogin(cand.userId)"
            >
              <img
                v-if="cand.avatarUrl"
                :src="cand.avatarUrl"
                class="candidate-avatar"
              />
              <div v-else class="candidate-avatar-fallback">
                {{ cand.nickname.slice(0, 1) }}
              </div>
              <div class="candidate-info">
                <div class="candidate-name">{{ cand.nickname }}</div>
                <div class="candidate-id">UID: {{ cand.userId }}</div>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
