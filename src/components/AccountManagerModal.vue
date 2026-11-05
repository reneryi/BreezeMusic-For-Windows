<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../stores/appStore";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

const currentUid = computed(() => store.session?.userId || store.profile?.userId || "");

const levelNum = computed(() => {
  return store.profile?.userGrade || store.profile?.userLevel || 1;
});

const listenHoursText = computed(() => {
  const sec = store.profile?.listenDurationSec || 0;
  const totalMin = Math.max(store.profile?.listenDurationMin || 0, Math.floor(sec / 60));
  const hours = Math.floor(totalMin / 60);
  const mins = totalMin % 60;
  if (hours > 0) return `${hours} 小时 ${mins} 分钟`;
  return `${mins} 分钟`;
});

const nextLevelRemainText = computed(() => {
  const diff = store.profile?.nextLevelRemainMin || 0;
  if (diff > 0) {
    const h = Math.floor(diff / 60);
    const m = diff % 60;
    return h > 0 ? `距离 Lv.${levelNum.value + 1} 还需 ${h} 小时 ${m} 分` : `距离 Lv.${levelNum.value + 1} 还需 ${m} 分钟`;
  }
  return "双水位线云端实时同步已就绪";
});

function openAddAccount() {
  store.isAccountManagerOpen = false;
  store.isLoginModalOpen = true;
}

function openVipCenter() {
  store.isAccountManagerOpen = false;
  store.isVipCenterOpen = true;
}

async function handleLogout() {
  await store.logout();
  store.isAccountManagerOpen = false;
}
</script>

<template>
  <transition name="fade">
    <div
      v-if="store.isAccountManagerOpen"
      class="modal-backdrop"
      @click.self="store.isAccountManagerOpen = false"
    >
      <div class="vip-modal-card account-mgr-modal">
        <div class="modal-header">
          <div class="vip-title-row">
            <div class="acct-mgr-orb">
              <PlumpIcon name="user" :size="20" color="#2B8CFF" />
            </div>
            <div>
              <h3>酷狗概念版 · 账号与多会话管理</h3>
              <p class="vip-sub-caption">
                支持多账号凭证持久化保存、秒级无感切换与听歌等级双水位线校准
              </p>
            </div>
          </div>
          <button class="icon-close-btn" @click="store.isAccountManagerOpen = false">
            <PlumpIcon name="close" :size="16" />
          </button>
        </div>

        <!-- 当前登录账号核心概览卡 -->
        <div v-if="store.isLoggedIn" class="acct-current-hero">
          <div class="acct-current-main">
            <img
              v-if="store.profile?.avatarUrl || store.session?.avatarUrl"
              :src="store.profile?.avatarUrl || store.session?.avatarUrl"
              class="acct-current-avatar"
              alt="avatar"
            />
            <div v-else class="acct-current-avatar acct-avatar-fallback">
              {{ (store.profile?.nickname || store.session?.nickname || "微").slice(0, 1) }}
            </div>

            <div class="acct-current-info">
              <div class="acct-name-row">
                <span class="acct-nickname">
                  {{ store.profile?.nickname || store.session?.nickname || "微风乐迷" }}
                </span>
                <span class="acct-lv-pill">Lv.{{ levelNum }}</span>
                <span
                  class="acct-vip-pill"
                  :class="{ active: store.vipStatus?.svipActive || store.vipStatus?.tvipActive || store.profile?.isVip }"
                >
                  {{
                    store.vipStatus?.svipActive
                      ? "概念 SVIP"
                      : store.vipStatus?.tvipActive || store.profile?.isVip
                      ? "畅听 VIP"
                      : "概念版用户"
                  }}
                </span>
              </div>
              <div class="acct-uid-line">
                UID: {{ currentUid }} · 听歌累计 <strong>{{ listenHoursText }}</strong>
              </div>
              <div class="acct-progress-hint">{{ nextLevelRemainText }}</div>
            </div>
          </div>

          <div class="acct-current-actions">
            <button
              class="acct-action-btn primary-soft"
              :disabled="store.isSyncingListeningGrade"
              @click="store.syncListeningGradeCloud(true)"
            >
              <PlumpIcon name="refresh" :size="14" :class="{ 'spin-anim': store.isSyncingListeningGrade }" />
              <span>{{ store.isSyncingListeningGrade ? "同步中..." : "同步等级时长" }}</span>
            </button>
            <button class="acct-action-btn gold-soft" @click="openVipCenter">
              <PlumpIcon name="crown" :size="14" />
              <span>VIP 权益中心</span>
            </button>
          </div>
        </div>

        <!-- 已保存的多账号列表 -->
        <div class="acct-saved-section">
          <div class="acct-section-head">
            <span>本地已保存的酷狗账号 ({{ store.savedAccounts.length }})</span>
            <button class="acct-link-btn" @click="openAddAccount">
              + 扫码 / 手机验证码添加账号
            </button>
          </div>

          <div v-if="store.savedAccounts.length > 0" class="acct-saved-list">
            <div
              v-for="acc in store.savedAccounts"
              :key="acc.userId"
              class="acct-saved-item"
              :class="{ current: acc.userId === currentUid }"
            >
              <div class="acct-saved-left">
                <img
                  v-if="acc.avatarUrl"
                  :src="acc.avatarUrl"
                  class="acct-saved-avatar"
                  alt=""
                />
                <div v-else class="acct-saved-avatar acct-avatar-fallback">
                  {{ (acc.nickname || "酷").slice(0, 1) }}
                </div>
                <div class="acct-saved-meta">
                  <div class="acct-saved-name-row">
                    <span class="acct-saved-name">{{ acc.nickname || `用户 ${acc.userId}` }}</span>
                    <span v-if="acc.userId === currentUid" class="acct-active-tag">当前在线</span>
                  </div>
                  <div class="acct-saved-uid">UID: {{ acc.userId }}</div>
                </div>
              </div>

              <div class="acct-saved-btns">
                <button
                  v-if="acc.userId !== currentUid"
                  class="acct-switch-btn"
                  @click="store.switchAccount(acc.userId)"
                >
                  切换此账号
                </button>
                <button
                  class="acct-remove-btn"
                  title="从本地移除此账号凭证"
                  @click="store.removeSavedAccount(acc.userId)"
                >
                  移除
                </button>
              </div>
            </div>
          </div>
          <div v-else class="acct-empty-tip">
            当前暂无已保存的账号会话，点击右上角可扫码或验证码登录酷狗账号。
          </div>
        </div>

        <!-- 底部操作栏 -->
        <div class="acct-modal-footer">
          <button class="acct-footer-btn add-btn" @click="openAddAccount">
            <PlumpIcon name="user" :size="15" />
            <span>登录其他账号 / 重新刷新 Token</span>
          </button>
          <button
            v-if="store.isLoggedIn"
            class="acct-footer-btn danger-btn"
            @click="handleLogout"
          >
            <span>退出当前登录</span>
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.account-mgr-modal {
  width: min(620px, 92vw);
  max-height: 84vh;
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 24px;
}

.acct-mgr-orb {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: var(--primary-soft);
  border: 1px solid var(--border-accent);
  display: flex;
  align-items: center;
  justify-content: center;
}

.acct-current-hero {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 20px;
  border-radius: 18px;
  background: linear-gradient(
    135deg,
    rgba(43, 140, 255, 0.14) 0%,
    rgba(0, 196, 167, 0.08) 100%
  );
  border: 1px solid var(--border-accent);
}

:global(.scheme-watercolor_light) .acct-current-hero,
:global(.scheme-light) .acct-current-hero {
  background: linear-gradient(135deg, #e8f2ff 0%, #edfaf7 100%);
  border-color: rgba(26, 115, 232, 0.22);
}

.acct-current-main {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
}

.acct-current-avatar {
  width: 58px;
  height: 58px;
  border-radius: 50%;
  object-fit: cover;
  border: 2px solid var(--primary);
  flex-shrink: 0;
}

.acct-avatar-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--primary-soft);
  color: var(--primary);
  font-weight: 800;
  font-size: 18px;
}

.acct-current-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.acct-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.acct-nickname {
  font-size: 17px;
  font-weight: 800;
  color: var(--text-main);
}

.acct-lv-pill {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 800;
  color: var(--accent-mint);
  background: var(--accent-mint-soft);
}

.acct-vip-pill {
  padding: 2px 9px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 700;
  color: var(--text-sub);
  background: var(--bg-glass-hover);
}

.acct-vip-pill.active {
  color: var(--accent-gold);
  background: var(--accent-gold-soft);
}

.acct-uid-line {
  font-size: 12.5px;
  color: var(--text-sub);
}

.acct-uid-line strong {
  color: var(--text-main);
  font-weight: 700;
}

.acct-progress-hint {
  font-size: 11.5px;
  color: var(--text-muted);
}

.acct-current-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex-shrink: 0;
}

.acct-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 13px;
  border-radius: 10px;
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.2s ease;
}

.acct-action-btn.primary-soft {
  background: var(--primary-soft);
  color: var(--primary);
  border-color: var(--border-accent);
}

.acct-action-btn.primary-soft:hover:not(:disabled) {
  background: var(--primary);
  color: #fff;
}

.acct-action-btn.gold-soft {
  background: var(--accent-gold-soft);
  color: var(--accent-gold);
  border-color: rgba(255, 213, 79, 0.28);
}

.acct-action-btn.gold-soft:hover {
  transform: translateY(-1px);
}

.acct-saved-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.acct-section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12.5px;
  font-weight: 700;
  color: var(--text-sub);
}

.acct-link-btn {
  background: none;
  border: none;
  color: var(--primary);
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
}

.acct-link-btn:hover {
  text-decoration: underline;
}

.acct-saved-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 240px;
  overflow-y: auto;
}

.acct-saved-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 11px 14px;
  border-radius: 14px;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  transition: all 0.2s ease;
}

.acct-saved-item.current {
  border-color: var(--border-accent);
  background: var(--primary-soft);
}

.acct-saved-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.acct-saved-avatar {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  object-fit: cover;
}

.acct-saved-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.acct-saved-name {
  font-size: 13.5px;
  font-weight: 700;
  color: var(--text-main);
}

.acct-active-tag {
  font-size: 10.5px;
  font-weight: 800;
  padding: 2px 7px;
  border-radius: 999px;
  background: var(--accent-mint-soft);
  color: var(--accent-mint);
}

.acct-vip-mini {
  font-size: 10px;
  font-weight: 800;
  padding: 1px 6px;
  border-radius: 6px;
  background: var(--accent-gold-soft);
  color: var(--accent-gold);
}

.acct-saved-uid {
  font-size: 11.5px;
  color: var(--text-muted);
}

.acct-saved-btns {
  display: flex;
  align-items: center;
  gap: 8px;
}

.acct-switch-btn {
  padding: 5px 12px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 700;
  background: var(--primary);
  color: #fff;
  border: none;
  cursor: pointer;
}

.acct-remove-btn {
  padding: 5px 10px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--border-subtle);
  cursor: pointer;
}

.acct-remove-btn:hover {
  color: var(--accent-coral);
  border-color: var(--accent-coral);
}

.acct-empty-tip {
  padding: 22px;
  text-align: center;
  font-size: 13px;
  color: var(--text-muted);
  border-radius: 14px;
  background: var(--bg-card);
  border: 1px dashed var(--border-subtle);
}

.acct-modal-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-top: 8px;
  border-top: 1px solid var(--border-subtle);
}

.acct-footer-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 16px;
  border-radius: 12px;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  border: 1px solid var(--border-subtle);
  background: var(--bg-card);
  color: var(--text-main);
  transition: all 0.2s ease;
}

.acct-footer-btn.add-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
}

.acct-footer-btn.danger-btn {
  color: var(--accent-coral);
  background: var(--accent-coral-soft);
  border-color: rgba(255, 107, 91, 0.25);
}

.acct-footer-btn.danger-btn:hover {
  background: var(--accent-coral);
  color: #fff;
}
</style>
