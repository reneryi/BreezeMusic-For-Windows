<script setup lang="ts">
import { useAppStore } from "../stores/appStore";
import type {
  AppColorScheme,
  AudioQualityValue,
  DesktopLayoutMode,
  PlayerBackgroundStyle,
  PlayerStageStyle,
} from "../types";
import {
  Layout,
  Palette,
  Volume2,
  ShieldCheck,
  UserCheck,
  Sparkles,
  LogOut,
  LogIn,
} from "lucide-vue-next";

const store = useAppStore();

const layoutOptions: {
  id: DesktopLayoutMode;
  title: string;
  desc: string;
}[] = [
  {
    id: "floating_bento",
    title: "灵动悬浮岛画布 (Floating Bento Island · 推荐)",
    desc: "四周留出呼吸边距，悬浮毛玻璃导航岛 + Bento 流光卡片矩阵 + 底部悬浮胶囊播放坞",
  },
  {
    id: "zen_minimal",
    title: "极简纯粹模式 (Zen Minimalist)",
    desc: "收敛光晕与浮动边距，采用紧凑无干扰的桌面通栏排版，适合专注办公与安静听歌",
  },
  {
    id: "mini_turntable",
    title: "桌面迷你黑胶挂件 (Mini Turntable Widget)",
    desc: "将窗口缩小为 420×168 置顶桌面小挂件，悬浮展示旋转黑胶与实时单行 KRC 歌词",
  },
];

const colorOptions: { id: AppColorScheme; title: string; swatch: string }[] = [
  {
    id: "abyssal_dark",
    title: "幽邃微蓝 (Abyssal Dark · 概念版经典)",
    swatch: "linear-gradient(135deg, #0a0e17, #162844)",
  },
  {
    id: "oled_black",
    title: "深邃纯黑 (Pure OLED)",
    swatch: "linear-gradient(135deg, #000000, #121212)",
  },
  {
    id: "watercolor_light",
    title: "水彩浸润 (Watercolor Light · 浅色通透)",
    swatch: "linear-gradient(135deg, #f2f6fc, #dcebfa)",
  },
];
</script>

<template>
  <div class="page-scroll-canvas">
    <header class="discover-top-header">
      <div>
        <h1 class="discover-main-title">设置</h1>
        <div class="discover-greeting-line">
          <span class="greeting-dot"></span>
          <span>
            全局界面风格、桌面窗口形态、默认音质与账号安全统一管理中心
          </span>
        </div>
      </div>
    </header>

    <div class="settings-sections-stack">
      <!-- 1. 桌面全局布局形态（花哨悬浮岛 vs 极简纯粹 vs 迷你黑胶挂件） -->
      <section class="settings-card-panel">
        <div class="panel-head">
          <Layout :size="18" class="cyan-ico" />
          <div>
            <h3>Windows 桌面布局架构模式</h3>
            <p>
              在这里自由切换「花哨灵动悬浮岛」、「极简纯粹通栏」或「桌面置顶迷你黑胶挂件」
            </p>
          </div>
        </div>

        <div class="layout-mode-cards">
          <button
            v-for="lm in layoutOptions"
            :key="lm.id"
            class="layout-choice-card"
            :class="{ active: store.layoutMode === lm.id }"
            @click="store.layoutMode = lm.id"
          >
            <div class="lc-title">{{ lm.title }}</div>
            <div class="lc-desc">{{ lm.desc }}</div>
          </button>
        </div>
      </section>

      <!-- 2. 外观配色与播放舞台默认偏好 -->
      <section class="settings-card-panel">
        <div class="panel-head">
          <Palette :size="18" class="cyan-ico" />
          <div>
            <h3>全局色彩主题与播放舞台</h3>
            <p>定制应用全局明暗色彩基调及封面动态流光氛围</p>
          </div>
        </div>

        <div class="color-scheme-row">
          <button
            v-for="cs in colorOptions"
            :key="cs.id"
            class="color-scheme-btn"
            :class="{ active: store.colorScheme === cs.id }"
            @click="store.colorScheme = cs.id"
          >
            <span class="cs-swatch" :style="{ background: cs.swatch }"></span>
            <span>{{ cs.title }}</span>
          </button>
        </div>

        <div class="settings-kv-row">
          <div>
            <div class="kv-title">默认播放页舞台形态</div>
            <div class="kv-desc">也可在播放页左下角工具坞中实时切换</div>
          </div>
          <select
            :value="store.playerStageStyle"
            class="settings-select"
            @change="
              store.playerStageStyle = ($event.target as HTMLSelectElement)
                .value as PlayerStageStyle
            "
          >
            <option value="vinyl_turntable">流光黑胶唱盘 (S型唱臂)</option>
            <option value="retro_cassette">复古磁带与 VU 电平表</option>
            <option value="cyber_spectrum">赛博霓虹频谱舞台</option>
            <option value="gallery_poster">极简画廊圆角方封</option>
          </select>
        </div>

        <div class="settings-kv-row">
          <div>
            <div class="kv-title">默认播放页背景光效</div>
            <div class="kv-desc">对齐 Android 端 5 款高定动态背景</div>
          </div>
          <select
            :value="store.playerBgStyle"
            class="settings-select"
            @change="
              store.playerBgStyle = ($event.target as HTMLSelectElement)
                .value as PlayerBackgroundStyle
            "
          >
            <option value="fluid_aurora">流体极光 (三色有机演化)</option>
            <option value="rhythm_halo">律动光晕 (同心声波呼吸)</option>
            <option value="cover_blur">封面虚化 (柔和双径向晕染)</option>
            <option value="starlight_nebula">暗夜星尘 (深空微光)</option>
            <option value="deep_oled">深邃纯黑 (OLED 零光污染)</option>
          </select>
        </div>
      </section>

      <!-- 3. 音源解析与硬件级 STREAMINFO 探测 -->
      <section class="settings-card-panel">
        <div class="panel-head">
          <Volume2 :size="18" class="cyan-ico" />
          <div>
            <h3>默认在线播放音质与流头探测</h3>
            <p>
              通过概念版 <code>/v2/get_res_privilege/lite</code> 补全各档位 Hash，并通过 HTTP Range
              读取音频流前 512 字节 <code>FLAC STREAMINFO</code> 还原真实采样率与位深
            </p>
          </div>
        </div>

        <div class="quality-cards-grid">
          <button
            v-for="q in [
              {
                val: 'high',
                title: 'Hi-Res 帧享母带',
                sub: '24bit / 96kHz~192kHz FLAC',
              },
              {
                val: 'flac',
                title: '无损 SQ 音质',
                sub: '16bit / 44.1kHz~48kHz FLAC',
              },
              {
                val: '320',
                title: '高品 HQ 320K',
                sub: '320kbps MP3 / OGG',
              },
              {
                val: '128',
                title: '标准 128K',
                sub: '省流 128kbps OGG / MP3',
              },
            ]"
            :key="q.val"
            class="quality-choice-card"
            :class="{ active: store.preferredQuality === q.val }"
            @click="store.switchQuality(q.val as AudioQualityValue)"
          >
            <div class="qc-title">{{ q.title }}</div>
            <div class="qc-sub">{{ q.sub }}</div>
          </button>
        </div>
      </section>

      <!-- 4. 账号与概念版 VIP 自动守护 -->
      <section class="settings-card-panel">
        <div class="panel-head">
          <UserCheck :size="18" class="cyan-ico" />
          <div>
            <h3>账号管理与每日概念版 VIP 自动签到</h3>
            <p>管理当前酷狗概念版直连会话与启动时自动领取 VIP 开关</p>
          </div>
        </div>

        <div class="settings-kv-row">
          <div>
            <div class="kv-title">启动应用时自动签到领取概念版 VIP</div>
            <div class="kv-desc">
              开启后，每日首次打开 Windows 客户端将自动触发概念版畅听 VIP 签到与 SVIP 升级
            </div>
          </div>
          <button
            class="toggle-pill-btn"
            :class="{ active: store.autoClaimVipOnStartup }"
            @click="store.autoClaimVipOnStartup = !store.autoClaimVipOnStartup"
          >
            {{ store.autoClaimVipOnStartup ? "已开启自动领取" : "已关闭" }}
          </button>
        </div>

        <div class="settings-kv-row">
          <div>
            <div class="kv-title">
              当前账号：{{
                store.isLoggedIn
                  ? `${store.session?.nickname} (UID: ${store.session?.userId})`
                  : "未登录访客态"
              }}
            </div>
            <div class="kv-desc">
              {{
                store.profile?.vipExpireText ||
                "登录账号后可同步云端歌单与听歌时长"
              }}
            </div>
          </div>
          <div class="row-btns">
            <button
              v-if="store.isLoggedIn"
              class="secondary-btn"
              @click="store.isVipCenterOpen = true"
            >
              <Sparkles :size="14" />
              <span>打开 VIP 权益中心</span>
            </button>
            <button
              v-if="store.isLoggedIn"
              class="ghost-btn"
              @click="store.logout()"
            >
              <LogOut :size="14" />
              <span>退出登录</span>
            </button>
            <button
              v-else
              class="primary-btn"
              @click="store.isLoginModalOpen = true"
            >
              <LogIn :size="14" />
              <span>立即登录</span>
            </button>
          </div>
        </div>
      </section>

      <!-- 5. 关于 BreezeMusic Windows -->
      <section class="settings-card-panel">
        <div class="panel-head">
          <ShieldCheck :size="18" class="cyan-ico" />
          <div>
            <h3>关于 BreezeMusic · 微风音乐 (Windows 概念版 v0.2.0)</h3>
            <p>
              Rust + Tauri 2.0 + Vue 3 纯端侧直连架构 · 协议对齐
              MakcRe/KuGouMusicApi (appid=3116, clientver=11440)
            </p>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
