<script setup lang="ts">
import { ref, computed } from "vue";
import { useAppStore } from "../stores/appStore";
import type {
  AppColorScheme,
  AudioQualityValue,
  DesktopLayoutMode,
  PlayerBackgroundStyle,
  PlayerStageStyle,
  WindowMaterialStyle,
} from "../types";
import PlumpIcon from "../components/PlumpIcon.vue";

const store = useAppStore();

type SettingsCategoryKey =
  | "appearance"
  | "audio_dsp"
  | "lyrics"
  | "control"
  | "storage_privacy"
  | "download"
  | "feedback_about";

const activeCategory = ref<SettingsCategoryKey>("appearance");
const showPlainCredentials = ref(false);

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

const categories = computed(() => {
  const base: {
    id: SettingsCategoryKey;
    title: string;
    subtitle: string;
    icon: string;
  }[] = [
    {
      id: "appearance",
      title: "视听与桌面外观",
      subtitle: "天际线画布 · 色彩主题 · 窗口材质",
      icon: "sparkles",
    },
    {
      id: "audio_dsp",
      title: "音质与母带音效",
      subtitle: "64-bit DSP · 默认音质 · 音量均衡",
      icon: "wave",
    },
    {
      id: "lyrics",
      title: "歌词与桌面悬浮词",
      subtitle: "六大主题 · 景深虚化 · 声卡校准",
      icon: "lyric",
    },
    {
      id: "control",
      title: "桌面控制与快捷键",
      subtitle: "自动续播 · 睡眠定时 · 托盘行为",
      icon: "settings",
    },
    {
      id: "storage_privacy",
      title: "存储缓存与功能管控",
      subtitle: "四维分项清理 · 下载功能合规解锁",
      icon: "shield",
    },
  ];

  // 对齐 Android 端合规隐藏设计：仅当手动开启「音乐下载与歌词保存」后才显示独立「下载设置」分类
  if (store.downloadFeatureUnlocked) {
    base.push({
      id: "download",
      title: "音乐下载与标签内嵌",
      subtitle: "下载音质 · 保存目录 · 1000P封面内嵌",
      icon: "download",
    });
  }

  base.push({
    id: "feedback_about",
    title: "问题反馈与关于微风",
    subtitle: "GitHub Issue 规范指南 · 更新日志",
    icon: "compass",
  });

  return base;
});

const layoutOptions: {
  id: DesktopLayoutMode;
  title: string;
  desc: string;
}[] = [
  {
    id: "floating_bento",
    title: "顶部天际线中枢画廊 (Skyline Command & Bento · 推荐)",
    desc: "彻底告别左侧边栏审美疲劳：顶部一体化天际线胶囊中枢 + 100% 全宽无框画布 + 底部悬浮控播坞",
  },
  {
    id: "zen_minimal",
    title: "极简纯粹通栏模式 (Zen Minimalist)",
    desc: "收敛四周浮动呼吸边距，采用紧凑沉浸式桌面排版，适合分屏办公与安静听歌",
  },
  {
    id: "mini_turntable",
    title: "Windows 桌面置顶迷你黑胶挂件 (Mini Turntable Widget)",
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
    swatch: "linear-gradient(135deg, #000000, #141418)",
  },
  {
    id: "watercolor_light",
    title: "水彩浸润 (Watercolor Light · 浅色通透)",
    swatch: "linear-gradient(135deg, #eef5fc, #d8eafc)",
  },
];

const materialOptions: { id: WindowMaterialStyle; label: string }[] = [
  { id: "mica_glow", label: "Mica 云母流光" },
  { id: "acrylic_frost", label: "Acrylic 亚克力磨砂" },
  { id: "pure_solid", label: "纯粹实色渲染" },
];

const onlineQualityOptions: { val: AudioQualityValue; name: string }[] = [
  { val: "high", name: "Hi-Res 高解析无损 (24bit/96kHz+)" },
  { val: "flac", name: "SQ 无损音质 (16bit FLAC)" },
  { val: "320", name: "HQ 高品音质 (320kbps)" },
  { val: "128", name: "标准音质 (128kbps)" },
];

const downloadQualityOptions: { val: AudioQualityValue; name: string }[] = [
  { val: "high", name: "Hi-Res 母带优先" },
  { val: "flac", name: "SQ 无损 FLAC" },
  { val: "320", name: "HQ 320K MP3" },
];

const changelogVersions = [
  {
    ver: "v1.4.0 (Windows Full Parity & Adaptive Upgrade)",
    date: "2026-11-01",
    items: [
      "彻底重构「水彩浸润 (Watercolor Light)」浅色主题与 WebView2 按钮/标题颜色继承机制，100% 满足 WCAG 高对比度可读性",
      "新增多级响应式断点 (@media <=920px / <=1180px / >=1600px) 与窗口尺寸记忆，修复迷你黑胶挂件与最大化窗口切换失真、歌单列高度截断与标题挤压",
      "Rust 端侧全量对齐 Android 仓库：补齐 CSCC (d.kugou.com) 三段式真实听歌时长上报、歌单全量多页拉取 (突破 100 首限制)、歌单云端双向收藏/删歌、评论二层楼中楼/点赞/发评、搜索实时联想词、KTV 伴奏音轨切换与酷狗蝰蛇音效库",
      "新增纯 Rust 零外部依赖 FLAC (Vorbis Comment + Picture) 与 MP3 (ID3v2.3 UTF-16LE + APIC + USLT) 二进制标签、1000×1000 高清封面及双语歌词内嵌引擎",
    ],
  },
  {
    ver: "v1.3.2 (Windows Skyline Edition)",
    date: "2026-11-01",
    items: [
      "全新「方案二：顶部天际线中枢画廊 (Skyline Command & Bento)」架构，彻底取消左侧导航栏，释放 100% 横向视野",
      "全量重绘「软润饱满 (Plump & Organic)」双色矢量图标引擎，全面消除 1px 线框边框设计，采用无框色阶高差 (Borderless Tonal UI)",
      "完整补齐精选场景电台调谐器、新歌速递日期时间轴与 A-Z 首字母检索、关注歌手、已购数字专辑、2K 歌词海报与合规隐藏式下载中心",
    ],
  },
  {
    ver: "v0.2.0 (Desktop Core)",
    date: "2026-10-31",
    items: [
      "修复扫码登录状态码 4 授权响应与概念版 token 补全",
      "移植 RFC 9639 FLAC STREAMINFO 硬件级真实采样率探测与无损码率反推",
      "对齐 Android 端 Lv.11 听歌等级与秒级 d_sec 累计时长对账",
    ],
  },
];

async function copyEnvDiagnostic() {
  const text = `BreezeMusic Windows v1.4.0 (Skyline Edition)\nOS: Windows 11 Desktop (Tauri 2.0 + WebView2)\nProtocol: KuGou Concept Lite (appid=3116, clientver=11440)\nAudio: 64-bit Pure LTI Biquad WebAudio DSP (${store.preferredQuality})`;
  try {
    await navigator.clipboard.writeText(text);
    store.notify("已一键复制系统与音源环境信息，可直接粘贴至 GitHub Issue");
  } catch {
    store.notify(text);
  }
}
</script>

<template>
  <div class="page-scroll-canvas" @click="store.closeSongContextMenu()">

    <!-- 平板与桌面端专属 Master-Detail 左导航右内容双栏布局 -->
    <div class="settings-master-detail">
      <!-- 左侧：二级分类软润胶囊栏 -->
      <aside class="settings-master-nav">
        <button
          v-for="cat in categories"
          :key="cat.id"
          class="settings-nav-card"
          :class="{ active: activeCategory === cat.id }"
          @click="activeCategory = cat.id"
        >
          <div class="snc-icon">
            <PlumpIcon
              :name="cat.icon"
              :size="19"
              :color="activeCategory === cat.id ? '#ffffff' : 'currentColor'"
              :accent="
                activeCategory === cat.id ? '#00e5ff' : 'var(--primary)'
              "
            />
          </div>
          <div class="snc-texts">
            <div class="snc-title">{{ cat.title }}</div>
            <div class="snc-sub">{{ cat.subtitle }}</div>
          </div>
        </button>
      </aside>

      <!-- 右侧：对应分类的无框色阶设置面板 -->
      <div class="settings-detail-pane">
        <!-- 1. 视听与桌面外观 -->
        <template v-if="activeCategory === 'appearance'">
          <section class="settings-card-panel">
            <h3>Windows 桌面架构模式</h3>
            <p class="panel-desc">
              自由切换「顶部天际线中枢画廊」、「极简纯粹通栏」或「桌面置顶迷你黑胶挂件」
            </p>
            <div class="layout-cards-grid">
              <div
                v-for="opt in layoutOptions"
                :key="opt.id"
                class="layout-option-card"
                :class="{ active: store.layoutMode === opt.id }"
                @click="store.layoutMode = opt.id"
              >
                <div class="loc-title">{{ opt.title }}</div>
                <div class="loc-desc">{{ opt.desc }}</div>
              </div>
            </div>
          </section>

          <section class="settings-card-panel">
            <h3>全局色彩主题与动态取色</h3>
            <div class="color-scheme-row mb-14">
              <button
                v-for="c in colorOptions"
                :key="c.id"
                class="color-scheme-pill"
                :class="{ active: store.colorScheme === c.id }"
                @click="store.colorScheme = c.id"
              >
                <span
                  class="color-swatch-circle"
                  :style="{ background: c.swatch }"
                ></span>
                <span>{{ c.title }}</span>
              </button>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">Windows 窗口光学材质</div>
                <div class="stl-sub">
                  适配 Windows 11 原生云母与亚克力通透空气感层次
                </div>
              </div>
              <div class="bg-choice-pills">
                <button
                  v-for="m in materialOptions"
                  :key="m.id"
                  class="bg-pill-btn"
                  :class="{ active: store.windowMaterial === m.id }"
                  @click="store.windowMaterial = m.id"
                >
                  {{ m.label }}
                </button>
              </div>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">专辑封面智能动态取色</div>
                <div class="stl-sub">
                  实时提取当前播放歌曲封面的主副色渲染背景光晕、按钮与进度条
                </div>
              </div>
              <label class="plump-switch">
                <input v-model="store.dynamicCoverTint" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">播放页默认舞台形态与光学背景</div>
                <div class="stl-sub">可在播放页左下角工具栏随时实时切换</div>
              </div>
              <div class="bg-choice-pills">
                <button
                  v-for="st in (['vinyl_turntable', 'gallery_poster', 'retro_cassette', 'cyber_spectrum'] as PlayerStageStyle[])"
                  :key="st"
                  class="bg-pill-btn"
                  :class="{ active: store.playerStageStyle === st }"
                  @click="store.playerStageStyle = st"
                >
                  {{
                    st === "vinyl_turntable"
                      ? "黑胶唱盘"
                      : st === "gallery_poster"
                      ? "棱镜方封"
                      : st === "retro_cassette"
                      ? "复古磁带"
                      : "赛博频谱"
                  }}
                </button>
              </div>
            </div>
          </section>
        </template>

        <!-- 2. 音质与母带音效 -->
        <template v-else-if="activeCategory === 'audio_dsp'">
          <section class="settings-card-panel">
            <h3>默认在线播放音质档位</h3>
            <p class="panel-desc">
              通过概念版 /v2/get_res_privilege/lite 自动匹配最高规格并支持平滑降级
            </p>
            <div class="quality-choice-row">
              <button
                v-for="q in onlineQualityOptions"
                :key="q.val"
                class="q-choice-pill"
                :class="{ active: store.preferredQuality === q.val }"
                @click="store.switchAudioQuality(q.val)"
              >
                {{ q.name }}
              </button>
            </div>
          </section>

          <section class="settings-card-panel">
            <h3>母带级 DSP 与听感增强</h3>
            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">音量均衡 (ReplayGain)</div>
                <div class="stl-sub">自动平衡不同年代与专辑曲目间的响度差异</div>
              </div>
              <label class="plump-switch">
                <input v-model="store.replayGainEnabled" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">夜间小音量等响度补偿</div>
                <div class="stl-sub">
                  低音量聆听时自动补偿低频下潜与极高频空气细节
                </div>
              </div>
              <label class="plump-switch">
                <input v-model="store.nightLoudnessEnabled" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">副歌速刷模式 (Climax 30s)</div>
                <div class="stl-sub">直达歌曲副歌高潮精华段落连播</div>
              </div>
              <label class="plump-switch">
                <input v-model="store.climaxModeEnabled" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">播放 / 暂停音量平滑渐入渐出</div>
                <div class="stl-sub">消除切换瞬间生硬断音</div>
              </div>
              <label class="plump-switch">
                <input v-model="store.playPauseFadeEnabled" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>
          </section>
        </template>

        <!-- 3. 歌词与桌面悬浮词 -->
        <template v-else-if="activeCategory === 'lyrics'">
          <section class="settings-card-panel">
            <h3>系统级自由拖动桌面歌词与定长规格</h3>
            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">系统级跨应用穿透桌面歌词</div>
                <div class="stl-sub">
                  在屏幕内任意位置自由拖动悬浮显示实时逐字双语歌词，支持规定长度与锁定防误触
                </div>
              </div>
              <label class="plump-switch">
                <input v-model="store.desktopLyricEnabled" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">桌面歌词规定长度 ({{ store.desktopLyricWidth }}px)</div>
                <div class="stl-sub">
                  自由设定桌面歌词悬浮条固定宽度 (480px ~ 1280px)，超出自动优雅截断不溢出
                </div>
              </div>
              <div style="display: flex; align-items: center; gap: 10px">
                <input
                  v-model.number="store.desktopLyricWidth"
                  type="range"
                  min="480"
                  max="1280"
                  step="20"
                />
                <button
                  class="tonal-pill-btn sm"
                  @click="
                    store.desktopLyricX = null;
                    store.desktopLyricY = null;
                    store.desktopLyricWidth = 760;
                    store.notify('已重置桌面歌词位置与规定长度 (760px)');
                  "
                >
                  重置位置
                </button>
              </div>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">桌面歌词字号 ({{ store.desktopLyricFontSize }}px) 与主题色</div>
                <div class="stl-sub">实时调整桌面悬浮逐字扫光字号与色彩</div>
              </div>
              <div style="display: flex; align-items: center; gap: 8px">
                <input
                  v-model.number="store.desktopLyricFontSize"
                  type="range"
                  min="18"
                  max="38"
                  step="1"
                />
                <button
                  v-for="c in ['#00e5ff', '#ffd54f', '#ff6b8b', '#00e5a3', '#b388ff']"
                  :key="c"
                  :style="{
                    width: '22px',
                    height: '22px',
                    borderRadius: '50%',
                    background: c,
                    border: store.desktopLyricColor === c ? '2px solid #fff' : '1px solid rgba(255,255,255,0.2)',
                    cursor: 'pointer'
                  }"
                  @click="store.desktopLyricColor = c"
                ></button>
              </div>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">阶梯光学景深虚化</div>
                <div class="stl-sub">非当前唱诵行随距离自然柔和虚化</div>
              </div>
              <label class="plump-switch">
                <input v-model="store.lyricBlurEnabled" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">
                  外置声卡 / 蓝牙音频输出延迟补偿 ({{ store.bluetoothDelayMs }}ms)
                </div>
                <div class="stl-sub">
                  连接蓝牙耳机或无线音箱时自动校准 KRC 逐字扫光同步偏移
                </div>
              </div>
              <input
                v-model.number="store.bluetoothDelayMs"
                type="range"
                min="-500"
                max="1000"
                step="25"
              />
            </div>
          </section>
        </template>

        <!-- 4. 桌面控制与快捷键 -->
        <template v-else-if="activeCategory === 'control'">
          <section class="settings-card-panel">
            <h3>播放行为与概念版 VIP 静默守护</h3>
            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">启动时自动静默签到领取概念版 VIP</div>
                <div class="stl-sub">
                  已登录状态下，每次启动自动检测并领取今日免费概念版 VIP
                </div>
              </div>
              <label class="plump-switch">
                <input v-model="store.autoClaimVipOnStartup" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">启动时自动恢复上次播放队列</div>
                <div class="stl-sub">打开应用后自动续播上次曲目</div>
              </div>
              <label class="plump-switch">
                <input v-model="store.autoResumeOnStartup" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">关闭主窗口时最小化至系统托盘</div>
                <div class="stl-sub">保持后台无损音乐播放不中断</div>
              </div>
              <label class="plump-switch">
                <input v-model="store.closeToTray" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>
          </section>

          <section class="settings-card-panel">
            <h3>桌面全局快捷键一览</h3>
            <div class="hotkey-grid">
              <div class="hotkey-row">
                <span>呼出 / 收起天际线搜索指令舱</span>
                <kbd>Ctrl + K</kbd>
              </div>
              <div class="hotkey-row">
                <span>关闭当前弹窗或返回主画布</span>
                <kbd>Esc</kbd>
              </div>
              <div class="hotkey-row">
                <span>封面红心收藏 / 取消收藏</span>
                <kbd>双击播放页封面</kbd>
              </div>
              <div class="hotkey-row">
                <span>呼出歌曲上下文操作菜单</span>
                <kbd>鼠标右键点击任意歌曲</kbd>
              </div>
            </div>
          </section>
        </template>

        <!-- 5. 存储缓存与功能管控（含合规隐藏式下载解锁开关） -->
        <template v-else-if="activeCategory === 'storage_privacy'">
          <section class="settings-card-panel">
            <h3>四维存储缓存明细与独立清理</h3>
            <p class="panel-desc">
              对齐 Android 端 AudioCacheManager 四维分项统计，精准清理不伤账号配置
            </p>
            <div class="cache-grid">
              <div class="cache-item-card">
                <div>
                  <div class="ci-title">流媒体音频缓存</div>
                  <div class="ci-val">{{ store.cacheStats.audioStreamMb }} MB</div>
                </div>
                <button
                  class="tonal-pill-btn sm"
                  @click="store.clearCacheCategory('audio')"
                >
                  清理
                </button>
              </div>

              <div class="cache-item-card">
                <div>
                  <div class="ci-title">高清封面与评论图片缓存</div>
                  <div class="ci-val">
                    {{ store.cacheStats.artworkImageMb }} MB
                  </div>
                </div>
                <button
                  class="tonal-pill-btn sm"
                  @click="store.clearCacheCategory('image')"
                >
                  清理
                </button>
              </div>

              <div class="cache-item-card">
                <div>
                  <div class="ci-title">歌单与发现页本地快照</div>
                  <div class="ci-val">
                    {{ store.cacheStats.playlistSnapshotMb }} MB
                  </div>
                </div>
                <button
                  class="tonal-pill-btn sm"
                  @click="store.clearCacheCategory('snapshot')"
                >
                  清理
                </button>
              </div>

              <div class="cache-item-card">
                <div>
                  <div class="ci-title">逐字歌词与音源直链缓存</div>
                  <div class="ci-val">
                    {{ store.cacheStats.lyricAndUrlMb }} MB
                  </div>
                </div>
                <button
                  class="tonal-pill-btn sm"
                  @click="store.clearCacheCategory('lyric')"
                >
                  清理
                </button>
              </div>
            </div>
          </section>

          <section class="settings-card-panel">
            <h3>功能与隐私透明管控（解锁本地下载）</h3>
            <p class="panel-desc">
              为保持界面纯粹合规，音乐下载功能默认隐藏。开启下方开关后，左侧将解锁独立的「音乐下载与标签内嵌」设置项及歌曲右键下载菜单
            </p>
            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">音乐下载与歌词保存</div>
                <div class="stl-sub">
                  允许将在线无损音频、1000P 高清封面标签及 .lrc 歌词保存至本地磁盘
                </div>
              </div>
              <label class="plump-switch">
                <input
                  v-model="store.downloadFeatureUnlocked"
                  type="checkbox"
                />
                <span class="plump-slider"></span>
              </label>
            </div>
          </section>

          <section class="settings-card-panel">
            <div class="dev-card-head" style="margin-bottom: 12px;">
              <div class="dev-head-left">
                <PlumpIcon name="shield" :size="17" color="#00e5ff" />
                <h3 style="margin: 0;">设备标识与音源鉴权凭证</h3>
              </div>
              <button
                class="tonal-pill-btn sm"
                @click="showPlainCredentials = !showPlainCredentials"
              >
                {{ showPlainCredentials ? "隐藏明文" : "展开明文" }}
              </button>
            </div>
            <p class="panel-desc">
              酷狗概念版 (appid=3116) 硬件设备指纹与会话鉴权标识，支持一键重签刷新与网关链路延迟诊断
            </p>

            <div class="dev-kv-row">
              <span>DFID 设备指纹</span>
              <div class="dev-kv-right">
                <code>{{ formatCred(store.vipStatus?.dfid || store.gatewayDiag?.dfid) }}</code>
                <button
                  class="dev-copy-btn"
                  @click="copyCred('DFID', store.vipStatus?.dfid || store.gatewayDiag?.dfid)"
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
                  @click="copyCred('GUID', store.vipStatus?.guid || store.gatewayDiag?.guid)"
                >
                  复制
                </button>
              </div>
            </div>

            <div class="dev-actions-row" style="margin-top: 14px; display: flex; gap: 12px;">
              <button
                class="tonal-pill-btn"
                :disabled="store.isDiagnosingGateway"
                @click="store.resignDeviceAndRefresh()"
              >
                <PlumpIcon name="refresh" :size="15" />
                <span>刷新设备标识</span>
              </button>
              <button
                class="tonal-pill-btn primary"
                :disabled="store.isDiagnosingGateway"
                @click="store.runGatewayDiagnostic()"
              >
                <PlumpIcon name="sparkles" :size="15" />
                <span>音源链路诊断</span>
              </button>
            </div>
          </section>
        </template>

        <!-- 6. 音乐下载与标签内嵌（仅在开启合规开关后显示） -->
        <template v-else-if="activeCategory === 'download'">
          <section class="settings-card-panel">
            <h3>多音质本地下载与二进制标签内嵌</h3>
            <div class="quality-choice-row mb-14">
              <button
                v-for="q in downloadQualityOptions"
                :key="q.val"
                class="q-choice-pill"
                :class="{ active: store.downloadQuality === q.val }"
                @click="store.downloadQuality = q.val"
              >
                {{ q.name }}
              </button>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">音乐保存目录</div>
                <div class="stl-sub">{{ store.downloadMusicDir }}</div>
              </div>
              <input
                v-model="store.downloadMusicDir"
                type="text"
                class="plump-text-input w-260"
              />
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">内嵌 1000P 高清封面与歌曲信息</div>
                <div class="stl-sub">
                  自动写入 1000×1000 专辑封面、歌名、歌手、专辑与年份标签
                </div>
              </div>
              <label class="plump-switch">
                <input v-model="store.embedCoverAndTags" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>

            <div class="setting-toggle-line">
              <div>
                <div class="stl-title">独立保存同名 .lrc 歌词文件</div>
                <div class="stl-sub">
                  下载歌曲时同步在歌词目录下生成含双语译文的 .lrc 文件
                </div>
              </div>
              <label class="plump-switch">
                <input v-model="store.exportStandaloneLrc" type="checkbox" />
                <span class="plump-slider"></span>
              </label>
            </div>
          </section>

          <section class="settings-card-panel">
            <h3>下载历史记录 ({{ store.downloadHistory.length }})</h3>
            <div v-if="store.downloadHistory.length === 0" class="empty-box compact">
              暂无本地下载记录，右键点击任意歌曲即可选择「下载歌曲」
            </div>
            <div v-else class="dual-column-song-grid">
              <div
                v-for="rec in store.downloadHistory"
                :key="rec.id"
                class="plump-song-card"
              >
                <div class="psc-main">
                  <div class="psc-title">{{ rec.title }} - {{ rec.artist }}</div>
                  <div class="psc-sub">
                    <span class="mini-badge hires">{{ rec.qualityLabel }}</span>
                    <span>{{ rec.fileSizeText }} · {{ rec.savedPath }}</span>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </template>

        <!-- 7. 问题反馈与关于微风 -->
        <template v-else-if="activeCategory === 'feedback_about'">
          <section class="settings-card-panel">
            <h3>标准 GitHub Issue 反馈规范指南</h3>
            <p class="panel-desc">
              提交问题时建议遵循「[分类] + 触发场景 + 核心现象」标题规范，并附带环境信息
            </p>
            <div class="issue-guide-box mb-14">
              <div class="ig-line">
                <strong>标题示例：</strong> [音源解析] 播放部分 Hi-Res 曲目时采样率探测超时回退
              </div>
              <div class="ig-line">
                <strong>正文四要素：</strong> 系统版本 + 复现步骤 + 预期与实际对比 + 歌曲名称/Hash
              </div>
            </div>
            <button class="primary-btn" @click="copyEnvDiagnostic">
              <PlumpIcon name="check" :size="16" color="#ffffff" />
              <span>一键复制当前运行环境信息</span>
            </button>
          </section>

          <section class="settings-card-panel">
            <h3>版本更新日志与 GPL-3.0 开源声明</h3>
            <div class="changelog-list">
              <div
                v-for="log in changelogVersions"
                :key="log.ver"
                class="changelog-card"
              >
                <div class="cl-head">
                  <strong>{{ log.ver }}</strong>
                  <span>{{ log.date }}</span>
                </div>
                <ul class="cl-ul">
                  <li v-for="(it, idx) in log.items" :key="idx">{{ it }}</li>
                </ul>
              </div>
            </div>
          </section>
        </template>
      </div>
    </div>
  </div>
</template>
