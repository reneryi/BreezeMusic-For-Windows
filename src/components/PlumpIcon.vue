<script setup lang="ts">
/**
 * PlumpIcon.vue — 微风音乐专属「软润饱满 (Plump & Organic)」双色矢量图标引擎
 * 严格对齐 Android 端 NowPlayingCanvasControls.kt 设计哲学：
 * 1. 彻底摒弃细线条线框 (Wireframe) 的干瘪感；
 * 2. 采用大圆角贝塞尔饱满填充 (Plump Filled Shapes) + 3.0px 超粗圆头流体笔画 + 双色柔光层叠 (Duotone Soft Layer)；
 * 3. 支持播放/暂停圆润三角与胶囊双柱、饱满红心、双色声波、歌词牌、海报星芒、天际线导航等全场景图标。
 */
withDefaults(
  defineProps<{
    name: string;
    size?: number;
    color?: string;
    accent?: string;
    filled?: boolean;
  }>(),
  {
    size: 20,
    color: "currentColor",
    accent: "var(--cyan, #00e5ff)",
    filled: true,
  }
);
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    class="plump-svg-icon"
  >
    <!-- 1. 播放 (圆润饱满大圆角贝塞尔三角，对齐 createPlumpPlayTrianglePath) -->
    <template v-if="name === 'play'">
      <path
        d="M7.2 4.6C5.6 3.7 4 4.8 4 6.6V17.4C4 19.2 5.6 20.3 7.2 19.4L18.4 13.9C20.1 13.0 20.1 11.0 18.4 10.1L7.2 4.6Z"
        :fill="color"
      />
    </template>

    <!-- 2. 暂停 (双圆润饱满胶囊柱，对齐 drawContinuousPlayPauseMorph) -->
    <template v-else-if="name === 'pause'">
      <rect x="5" y="4" width="5.2" height="16" rx="2.6" :fill="color" />
      <rect x="13.8" y="4" width="5.2" height="16" rx="2.6" :fill="color" />
    </template>

    <!-- 3. 上一首 (圆润胶囊竖柱 + 饱满圆角三角，对齐 RedrawnSkipButton) -->
    <template v-else-if="name === 'prev'">
      <rect x="4.2" y="5.5" width="3.4" height="13" rx="1.7" :fill="color" />
      <path
        d="M17.4 5.3C18.8 4.4 20.4 5.4 20.4 7.0V17.0C20.4 18.6 18.8 19.6 17.4 18.7L9.4 13.7C8.1 12.9 8.1 11.1 9.4 10.3L17.4 5.3Z"
        :fill="color"
      />
    </template>

    <!-- 4. 下一首 (饱满圆角三角 + 圆润胶囊竖柱，对齐 RedrawnSkipButton) -->
    <template v-else-if="name === 'next'">
      <path
        d="M6.6 5.3C5.2 4.4 3.6 5.4 3.6 7.0V17.0C3.6 18.6 5.2 19.6 6.6 18.7L14.6 13.7C15.9 12.9 15.9 11.1 14.6 10.3L6.6 5.3Z"
        :fill="color"
      />
      <rect x="16.4" y="5.5" width="3.4" height="13" rx="1.7" :fill="color" />
    </template>

    <!-- 5. 软润饱满爱心 (对齐 PlumpHeartCanvasIcon 三次贝塞尔曲线) -->
    <template v-else-if="name === 'heart' || name === 'heart-filled'">
      <path
        v-if="filled || name === 'heart-filled'"
        d="M12 20.6C11.4 20.6 10.8 20.4 10.3 20.0C5.4 16.4 2.2 12.7 2.2 8.4C2.2 5.1 4.8 2.7 8.0 2.7C9.7 2.7 11.1 3.5 12 4.7C12.9 3.5 14.3 2.7 16.0 2.7C19.2 2.7 21.8 5.1 21.8 8.4C21.8 12.7 18.6 16.4 13.7 20.0C13.2 20.4 12.6 20.6 12 20.6Z"
        :fill="color"
      />
      <path
        v-else
        d="M12 19.8C6.4 15.8 3.2 12.3 3.2 8.4C3.2 5.7 5.3 3.7 8.0 3.7C9.6 3.7 11.1 4.5 12 5.9C12.9 4.5 14.4 3.7 16.0 3.7C18.7 3.7 20.8 5.7 20.8 8.4C20.8 12.3 17.6 15.8 12 19.8Z"
        :fill="color"
        fill-opacity="0.18"
        :stroke="color"
        stroke-width="2.7"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>

    <!-- 6. 发现/指南针 (双层软润圆盘 + 饱满菱形指针) -->
    <template v-else-if="name === 'compass'">
      <circle cx="12" cy="12" r="9.5" :fill="color" fill-opacity="0.2" />
      <path
        d="M15.6 8.4L13.9 13.9L8.4 15.6L10.1 10.1L15.6 8.4Z"
        :fill="color"
        :stroke="color"
        stroke-width="2.2"
        stroke-linejoin="round"
      />
      <circle cx="12" cy="12" r="1.6" :fill="accent" />
    </template>

    <!-- 7. 我的音乐库 (三枚软润圆角胶囊叠层，第二枚倾斜律动) -->
    <template v-else-if="name === 'library'">
      <rect x="3.5" y="4.5" width="4.2" height="15" rx="2.1" :fill="color" />
      <rect
        x="9.2"
        y="4.5"
        width="4.2"
        height="15"
        rx="2.1"
        :fill="color"
        fill-opacity="0.72"
      />
      <rect
        x="14.5"
        y="5.2"
        width="4.2"
        height="14.5"
        rx="2.1"
        transform="rotate(-12 14.5 5.2)"
        :fill="accent"
      />
    </template>

    <!-- 8. 电台/调频台 (饱满声波发射塔 + 软润圆弧波纹) -->
    <template v-else-if="name === 'radio'">
      <rect
        x="3"
        y="7.5"
        width="18"
        height="13"
        rx="4.5"
        :fill="color"
        fill-opacity="0.22"
      />
      <path
        d="M6.5 7.5L15.5 3.5"
        :stroke="color"
        stroke-width="2.8"
        stroke-linecap="round"
      />
      <circle cx="8.5" cy="14" r="3.2" :fill="accent" />
      <rect x="14" y="11.5" width="4" height="2.2" rx="1.1" :fill="color" />
      <rect x="14" y="15" width="2.8" height="2.2" rx="1.1" :fill="color" />
    </template>

    <!-- 9. 搜索 (饱满双层放大镜) -->
    <template v-else-if="name === 'search'">
      <circle cx="10.8" cy="10.8" r="6.8" :fill="color" fill-opacity="0.18" />
      <circle
        cx="10.8"
        cy="10.8"
        r="6.2"
        :stroke="color"
        stroke-width="2.9"
      />
      <path
        d="M15.8 15.8L20.2 20.2"
        :stroke="accent"
        stroke-width="3.2"
        stroke-linecap="round"
      />
    </template>

    <!-- 10. 设置 (圆润丰满六瓣梅花齿轮 + 中心柔光核) -->
    <template v-else-if="name === 'settings'">
      <path
        d="M12 2.8C13.1 2.8 14.0 3.6 14.2 4.7C14.7 4.9 15.2 5.2 15.6 5.5C16.6 5.0 17.8 5.3 18.4 6.3C19.0 7.2 18.7 8.4 17.9 9.1C18.1 9.6 18.1 10.2 18.1 10.8C19.0 11.4 19.2 12.7 18.6 13.6C18.0 14.6 16.8 14.9 15.8 14.4C15.3 14.8 14.8 15.1 14.2 15.3C14.0 16.4 13.1 17.2 12 17.2C10.9 17.2 10.0 16.4 9.8 15.3C9.2 15.1 8.7 14.8 8.2 14.4C7.2 14.9 6.0 14.6 5.4 13.6C4.8 12.7 5.0 11.4 5.9 10.8C5.9 10.2 5.9 9.6 6.1 9.1C5.3 8.4 5.0 7.2 5.6 6.3C6.2 5.3 7.4 5.0 8.4 5.5C8.8 5.2 9.3 4.9 9.8 4.7C10.0 3.6 10.9 2.8 12 2.8Z"
        transform="translate(0 2)"
        :fill="color"
        fill-opacity="0.25"
        :stroke="color"
        stroke-width="2.4"
        stroke-linejoin="round"
      />
      <circle cx="12" cy="12" r="3.2" :fill="accent" />
    </template>

    <!-- 11. 皇冠 VIP (圆润饱满三峰皇冠 + 宝石底托) -->
    <template v-else-if="name === 'crown'">
      <path
        d="M3.5 16.5L2.4 8.2C2.2 6.9 3.6 6.1 4.6 6.9L8.2 9.8L10.8 4.6C11.3 3.6 12.7 3.6 13.2 4.6L15.8 9.8L19.4 6.9C20.4 6.1 21.8 6.9 21.6 8.2L20.5 16.5C20.3 17.9 19.1 19 17.7 19H6.3C4.9 19 3.7 17.9 3.5 16.5Z"
        :fill="color"
      />
      <circle cx="12" cy="14" r="1.5" fill="#fff" fill-opacity="0.85" />
    </template>

    <!-- 12. 母带声波 / 蝰蛇音效 (对齐 PlumpAudioWaveCanvasIcon 4根胶囊声柱) -->
    <template v-else-if="name === 'wave' || name === 'sliders'">
      <rect x="3.4" y="7.5" width="3.2" height="9" rx="1.6" :fill="color" />
      <rect x="8.1" y="3.5" width="3.2" height="17" rx="1.6" :fill="accent" />
      <rect x="12.8" y="6.0" width="3.2" height="12" rx="1.6" :fill="color" />
      <rect x="17.5" y="8.5" width="3.2" height="7" rx="1.6" :fill="color" />
    </template>

    <!-- 13. 桌面歌词牌 (对齐 PlumpDesktopLyricCanvasIcon) -->
    <template v-else-if="name === 'lyric'">
      <rect
        x="2.8"
        y="4.2"
        width="18.4"
        height="15.6"
        rx="5.2"
        :fill="color"
        fill-opacity="0.22"
      />
      <path
        d="M7 9.8H17"
        :stroke="color"
        stroke-width="2.8"
        stroke-linecap="round"
      />
      <path
        d="M7 14.4H13.6"
        :stroke="accent"
        stroke-width="2.8"
        stroke-linecap="round"
      />
    </template>

    <!-- 14. 歌词海报星芒 (对齐 PlumpPosterSparkleCanvasIcon) -->
    <template v-else-if="name === 'poster' || name === 'sparkles'">
      <rect
        x="3"
        y="3"
        width="18"
        height="18"
        rx="5.6"
        :fill="color"
        fill-opacity="0.2"
      />
      <path
        d="M7 11H16.2"
        :stroke="color"
        stroke-width="2.7"
        stroke-linecap="round"
      />
      <path
        d="M7 15.2H13"
        :stroke="accent"
        stroke-width="2.7"
        stroke-linecap="round"
      />
      <circle cx="16.4" cy="7.2" r="2.1" :fill="accent" />
    </template>

    <!-- 15. 添加到歌单 (对齐 PlumpPlaylistAddCanvasIcon) -->
    <template v-else-if="name === 'playlist-add'">
      <path
        d="M3.5 6.5H18.5"
        :stroke="color"
        stroke-width="2.8"
        stroke-linecap="round"
      />
      <path
        d="M3.5 12H13"
        :stroke="color"
        stroke-width="2.8"
        stroke-linecap="round"
      />
      <path
        d="M3.5 17.5H10.5"
        :stroke="color"
        stroke-width="2.8"
        stroke-linecap="round"
      />
      <path
        d="M17.5 12.5V19.5M14 16H21"
        :stroke="accent"
        stroke-width="2.9"
        stroke-linecap="round"
      />
    </template>

    <!-- 16. 播放队列 (对齐 RedrawnQueueButton) -->
    <template v-else-if="name === 'queue'">
      <rect x="3.5" y="5" width="17" height="3.2" rx="1.6" :fill="color" />
      <rect x="3.5" y="10.4" width="12.5" height="3.2" rx="1.6" :fill="accent" />
      <rect x="3.5" y="15.8" width="15" height="3.2" rx="1.6" :fill="color" />
    </template>

    <!-- 17. 评论气泡 (对齐 RedesignedCommentCountButton 饱满圆润气泡) -->
    <template v-else-if="name === 'comment'">
      <path
        d="M12 3.2C6.8 3.2 2.8 6.8 2.8 11.4C2.8 13.7 3.9 15.8 5.6 17.2L4.6 20.1C4.4 20.7 5.0 21.2 5.6 20.9L9.1 19.2C10.0 19.5 11.0 19.6 12 19.6C17.2 19.6 21.2 16.0 21.2 11.4C21.2 6.8 17.2 3.2 12 3.2Z"
        :fill="color"
        fill-opacity="0.24"
      />
      <circle cx="8.6" cy="11.4" r="1.5" :fill="color" />
      <circle cx="12.0" cy="11.4" r="1.5" :fill="accent" />
      <circle cx="15.4" cy="11.4" r="1.5" :fill="color" />
    </template>

    <!-- 18. 更多三圆点 (对齐 PlumpMoreDotsCanvasIcon) -->
    <template v-else-if="name === 'more'">
      <circle cx="5.5" cy="12" r="2.2" :fill="color" />
      <circle cx="12" cy="12" r="2.2" :fill="accent" />
      <circle cx="18.5" cy="12" r="2.2" :fill="color" />
    </template>

    <!-- 19. 列表循环 / 单曲循环 / 随机播放 (对齐 AnimatedPlayModeButton) -->
    <template v-else-if="name === 'repeat-all'">
      <path
        d="M5 9C5 6.8 6.8 5 9 5H17.5M17.5 5L15 2.8M17.5 5L15 7.2M19 15C19 17.2 17.2 19 15 19H6.5M6.5 19L9 16.8M6.5 19L9 21.2"
        :stroke="color"
        stroke-width="2.7"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'repeat-one'">
      <path
        d="M5 9C5 6.8 6.8 5 9 5H17.5M19 15C19 17.2 17.2 19 15 19H6.5"
        :stroke="accent"
        stroke-width="2.7"
        stroke-linecap="round"
      />
      <path
        d="M12 9V15M10.4 10.5L12 9"
        :stroke="color"
        stroke-width="2.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'shuffle'">
      <path
        d="M3.8 7.6C9.5 7.6 11.5 16.4 19.2 16.4M19.2 16.4L16.5 14.0M19.2 16.4L16.5 18.8"
        :stroke="color"
        stroke-width="2.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
      <path
        d="M3.8 16.4C9.5 16.4 11.5 7.6 19.2 7.6M19.2 7.6L16.5 5.2M19.2 7.6L16.5 10.0"
        :stroke="accent"
        stroke-width="2.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>

    <!-- 20. 刷新同步 (流体圆弧双箭头) -->
    <template v-else-if="name === 'refresh'">
      <path
        d="M19.2 10.2C18.4 6.5 15.1 3.8 11.2 3.8C6.7 3.8 3.2 7.4 3.2 12C3.2 16.6 6.7 20.2 11.2 20.2C14.8 20.2 17.8 18.0 18.9 14.8"
        :stroke="color"
        stroke-width="2.8"
        stroke-linecap="round"
      />
      <path
        d="M19.6 5.5V10.4H14.7"
        :stroke="accent"
        stroke-width="2.8"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>

    <!-- 21. 定位当前歌曲 (饱满同心靶心) -->
    <template v-else-if="name === 'locate'">
      <circle cx="12" cy="12" r="7.5" :stroke="color" stroke-width="2.6" />
      <circle cx="12" cy="12" r="3.2" :fill="accent" />
      <path
        d="M12 2.2V4.5M12 19.5V21.8M2.2 12H4.5M19.5 12H21.8"
        :stroke="color"
        stroke-width="2.6"
        stroke-linecap="round"
      />
    </template>

    <!-- 22. 历史时钟 / 番茄钟 -->
    <template v-else-if="name === 'history' || name === 'timer'">
      <circle cx="12" cy="12" r="9.2" :fill="color" fill-opacity="0.2" />
      <circle cx="12" cy="12" r="8.2" :stroke="color" stroke-width="2.5" />
      <path
        d="M12 7.5V12.2L15.2 14.2"
        :stroke="accent"
        stroke-width="2.8"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>

    <!-- 23. 奖杯榜单 (饱满圆润金杯) -->
    <template v-else-if="name === 'trophy'">
      <path
        d="M6.5 4H17.5V10.5C17.5 13.5 15.0 16 12 16C9.0 16 6.5 13.5 6.5 10.5V4Z"
        :fill="color"
      />
      <path
        d="M6.5 6H4.2C3.5 6 3 6.6 3 7.3C3 9.5 4.5 11.2 6.5 11.5M17.5 6H19.8C20.5 6 21 6.6 21 7.3C21 9.5 19.5 11.2 17.5 11.5"
        :stroke="color"
        stroke-width="2.5"
        stroke-linecap="round"
      />
      <rect x="8.2" y="18.2" width="7.6" height="3" rx="1.5" :fill="accent" />
    </template>

    <!-- 24. 黑胶唱片 (饱满双环黑胶) -->
    <template v-else-if="name === 'disc'">
      <circle cx="12" cy="12" r="9.5" :fill="color" fill-opacity="0.26" />
      <circle cx="12" cy="12" r="8.5" :stroke="color" stroke-width="2.4" />
      <circle cx="12" cy="12" r="3.4" :fill="accent" />
      <circle cx="12" cy="12" r="1.2" fill="#090d16" />
    </template>

    <!-- 25. 音量喇叭 -->
    <template v-else-if="name === 'volume'">
      <path
        d="M11.2 5.2L7.2 8.6H4.5C3.4 8.6 2.5 9.5 2.5 10.6V13.4C2.5 14.5 3.4 15.4 4.5 15.4H7.2L11.2 18.8C12.3 19.7 14 18.9 14 17.5V6.5C14 5.1 12.3 4.3 11.2 5.2Z"
        :fill="color"
      />
      <path
        d="M17.2 9.0C18.3 10.6 18.3 13.4 17.2 15.0M20.0 6.8C22.0 9.8 22.0 14.2 20.0 17.2"
        :stroke="accent"
        stroke-width="2.6"
        stroke-linecap="round"
      />
    </template>

    <!-- 26. 下载 / 导入 / 文件夹 / 盾牌 -->
    <template v-else-if="name === 'download'">
      <rect
        x="4"
        y="16.8"
        width="16"
        height="3.4"
        rx="1.7"
        :fill="color"
        fill-opacity="0.35"
      />
      <path
        d="M12 4.2V14.2M12 14.2L8.2 10.5M12 14.2L15.8 10.5"
        :stroke="accent"
        stroke-width="3.0"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'upload'">
      <rect
        x="4"
        y="16.8"
        width="16"
        height="3.4"
        rx="1.7"
        :fill="color"
        fill-opacity="0.35"
      />
      <path
        d="M12 14.5V4.5M12 4.5L8.2 8.2M12 4.5L15.8 8.2"
        :stroke="accent"
        stroke-width="3.0"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'shield'">
      <path
        d="M12 2.8L4.5 6.0V11.6C4.5 16.5 7.7 20.8 12 22.2C16.3 20.8 19.5 16.5 19.5 11.6V6.0L12 2.8Z"
        :fill="color"
        fill-opacity="0.24"
        :stroke="color"
        stroke-width="2.4"
        stroke-linejoin="round"
      />
      <path
        d="M9.2 12.2L11.2 14.2L15.2 10.0"
        :stroke="accent"
        stroke-width="2.8"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>

    <!-- 27. 基础方向、加号、网格、列表、关闭 -->
    <template v-else-if="name === 'plus'">
      <path
        d="M12 5V19M5 12H19"
        :stroke="color"
        stroke-width="3.1"
        stroke-linecap="round"
      />
    </template>
    <template v-else-if="name === 'back' || name === 'arrow-left'">
      <path
        d="M19 12H5.5M5.5 12L11 6.5M5.5 12L11 17.5"
        :stroke="color"
        stroke-width="2.9"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'chevron-right'">
      <path
        d="M9.5 6.5L15 12L9.5 17.5"
        :stroke="color"
        stroke-width="2.9"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'chevron-down'">
      <path
        d="M6.5 9.5L12 15L17.5 9.5"
        :stroke="color"
        stroke-width="2.9"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'grid'">
      <rect x="3.5" y="3.5" width="7.2" height="7.2" rx="2.6" :fill="color" />
      <rect x="13.3" y="3.5" width="7.2" height="7.2" rx="2.6" :fill="accent" />
      <rect x="3.5" y="13.3" width="7.2" height="7.2" rx="2.6" :fill="color" />
      <rect x="13.3" y="13.3" width="7.2" height="7.2" rx="2.6" :fill="color" />
    </template>
    <template v-else-if="name === 'list'">
      <rect x="3.5" y="5" width="17" height="3.4" rx="1.7" :fill="color" />
      <rect x="3.5" y="10.3" width="17" height="3.4" rx="1.7" :fill="accent" />
      <rect x="3.5" y="15.6" width="17" height="3.4" rx="1.7" :fill="color" />
    </template>
    <template v-else-if="name === 'close'">
      <path
        d="M6.8 6.8L17.2 17.2M17.2 6.8L6.8 17.2"
        :stroke="color"
        stroke-width="3.0"
        stroke-linecap="round"
      />
    </template>
    <template v-else-if="name === 'check'">
      <circle cx="12" cy="12" r="9.5" :fill="accent" fill-opacity="0.22" />
      <path
        d="M7.8 12.3L10.8 15.3L16.5 9.2"
        :stroke="accent"
        stroke-width="3.0"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'repeat'">
      <path
        d="M5 9C5 6.8 6.8 5 9 5H17.5M17.5 5L15 2.8M17.5 5L15 7.2M19 15C19 17.2 17.2 19 15 19H6.5M6.5 19L9 16.8M6.5 19L9 21.2"
        :stroke="color"
        stroke-width="2.7"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'sparkle'">
      <path
        d="M12 2.8L14.3 9.2L20.8 11.5L14.3 13.8L12 20.2L9.7 13.8L3.2 11.5L9.7 9.2L12 2.8Z"
        :fill="color"
      />
      <circle cx="18.5" cy="5.5" r="1.8" :fill="accent" />
    </template>
    <template v-else-if="name === 'tuning'">
      <path
        d="M4 7H20M4 17H20"
        :stroke="color"
        stroke-width="2.6"
        stroke-linecap="round"
      />
      <circle cx="9" cy="7" r="3" :fill="accent" />
      <circle cx="15" cy="17" r="3" :fill="color" />
    </template>
    <template v-else-if="name === 'volume-mute'">
      <path
        d="M11 5.5L6.8 9H4.2C3.5 9 3 9.5 3 10.2V13.8C3 14.5 3.5 15 4.2 15H6.8L11 18.5C11.8 19.1 13 18.5 13 17.5V6.5C13 5.5 11.8 4.9 11 5.5Z"
        :fill="color"
      />
      <path
        d="M16.2 9.5L20.8 14.5M20.8 9.5L16.2 14.5"
        :stroke="accent"
        stroke-width="2.6"
        stroke-linecap="round"
      />
    </template>
    <template v-else-if="name === 'minimize'">
      <path d="M5 12H19" :stroke="color" stroke-width="2.8" stroke-linecap="round" />
    </template>
    <template v-else-if="name === 'maximize'">
      <rect
        x="5"
        y="5"
        width="14"
        height="14"
        rx="3.2"
        :stroke="color"
        stroke-width="2.5"
      />
    </template>
    <template v-else-if="name === 'user'">
      <circle cx="12" cy="8.2" r="4.1" :fill="color" />
      <path
        d="M4.8 19.5C5.6 15.8 8.5 14 12 14C15.5 14 18.4 15.8 19.2 19.5"
        :stroke="accent"
        stroke-width="2.8"
        stroke-linecap="round"
      />
    </template>
    <template v-else-if="name === 'sun'">
      <circle cx="12" cy="12" r="4.6" :fill="accent" />
      <path
        d="M12 3V5.2M12 18.8V21M3 12H5.2M18.8 12H21M5.6 5.6L7.2 7.2M16.8 16.8L18.4 18.4M18.4 5.6L16.8 7.2M7.2 16.8L5.6 18.4"
        :stroke="color"
        stroke-width="2.4"
        stroke-linecap="round"
      />
    </template>
    <template v-else-if="name === 'moon'">
      <path
        d="M20.2 14.6C19.1 15.1 17.8 15.4 16.5 15.4C11.5 15.4 7.5 11.4 7.5 6.4C7.5 5.1 7.8 3.9 8.3 2.8C4.8 4.1 2.4 7.5 2.4 11.5C2.4 16.7 6.6 20.9 11.8 20.9C15.8 20.9 19.1 18.3 20.2 14.6Z"
        :fill="color"
      />
    </template>
    <template v-else-if="name === 'clock'">
      <circle cx="12" cy="12" r="8.8" :stroke="color" stroke-width="2.6" />
      <path
        d="M12 7.5V12.2L15.2 14.2"
        :stroke="accent"
        stroke-width="2.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'folder'">
      <path
        d="M3.2 7.4C3.2 5.9 4.4 4.8 5.8 4.8H9.4L11.5 7.1H18.2C19.6 7.1 20.8 8.3 20.8 9.7V16.8C20.8 18.2 19.6 19.4 18.2 19.4H5.8C4.4 19.4 3.2 18.2 3.2 16.8V7.4Z"
        :fill="color"
        fill-opacity="0.28"
        :stroke="color"
        stroke-width="2.2"
      />
    </template>
    <template v-else-if="name === 'trash'">
      <path
        d="M4.5 7H19.5M9.5 4.2H14.5M6.8 7L7.6 18.4C7.7 19.5 8.6 20.2 9.7 20.2H14.3C15.4 20.2 16.3 19.5 16.4 18.4L17.2 7"
        :stroke="color"
        stroke-width="2.4"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </template>
    <template v-else-if="name === 'music'">
      <path
        d="M9 17.5V6.5L19 4.5V15.5"
        :stroke="color"
        stroke-width="2.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
      <circle cx="6.5" cy="17.5" r="2.8" :fill="accent" />
      <circle cx="16.5" cy="15.5" r="2.8" :fill="color" />
    </template>
    <template v-else>
      <circle cx="12" cy="12" r="8" :fill="color" fill-opacity="0.25" />
      <circle cx="12" cy="12" r="3.5" :fill="accent" />
    </template>
  </svg>
</template>

<style scoped>
.plump-svg-icon {
  display: inline-block;
  vertical-align: middle;
  flex-shrink: 0;
  transition: transform 0.28s cubic-bezier(0.22, 1.35, 0.36, 1),
    filter 0.25s ease;
}
</style>
