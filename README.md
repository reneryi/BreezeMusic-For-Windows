# BreezeMusic · 微风音乐 (Windows 桌面概念版 v1.4.0 · Skyline Edition)

基于 **Rust + Tauri 2.0 + Vue 3 (TypeScript + Pinia)** 构建的第三方「酷狗概念版 (`appid=3116`, `clientver=11440`)」Windows 桌面客户端，协议层参考对标 [MakcRe/KuGouMusicApi](https://github.com/MakcRe/KuGouMusicApi) 与 Android 端 `reneryi/breezemusic` v1.4.0。

## 一、核心设计理念：方案二「顶部天际线中枢画廊 (Skyline Command & Bento)」

1. **彻底告别左侧导航栏审美疲劳**：
   - 移除传统 `220px` 左侧固定导航栏，将导航、搜索与个人身份收拢至 **顶部天际线中枢 (`TitleBar.vue`)**，释放 **100% 全宽无遮挡桌面画布**。
   - **顶部流光数字身份舱**：右上角常驻展示 `[头像] 昵称 · ✦ Lv.11 · 累计听歌 X 小时 Y 分`（带 CSCC 实时听歌心跳指示灯）与 `👑 概念 VIP / 每日签到` 鎏金入口。
2. **软润饱满双色矢量图标 (`PlumpIcon.vue`) + 100% 零边框线设计 (Borderless Tonal UI)**：
   - 对齐 Android 端 `NowPlayingCanvasControls.kt`，全量重绘软润饱满的大圆角贝塞尔填充图标与双色微光层次，彻底弃用细线条线框图标。
   - 全局移除 `1px solid` 线框分割，采用 **无框色阶高差 (Surface Elevation)**、柔和空气感投影与 120fps 欠阻尼弹簧过渡，并深度适配「水彩浸润 (Watercolor Light)」高对比浅色与「深邃夜幕 (Abyssal Dark)」双色阶外观。
3. **Windows v1.4.0 全功能对等移植与桌面级架构升华**：
   - **探索发现 (`DiscoverView.vue`)**：
     - 首屏黄金双子星：**三封面交错立体叠层 (`StaggeredThreeCovers`) 每日推荐卡** + **实时可交互「猜你喜欢 · 随心听」私人 FM 卡**（含 20 首漫游候选池抽屉与 `+20首` 无缝续杯）。
     - **新歌速递**：首页 5 语种 `3×3` 微卡网格；独立页面采用 **精确发行日期 (`yyyy-MM-dd`) 折叠时间轴 + 右侧悬浮 A-Z 首字母快速索引条**。
     - **精选场景电台走廊** 与 **巅峰排行榜画廊**（基于真并发轮询聚合 Top 3 榜单预览、全部榜单分类宫格选择器、左窄右宽双栏榜单工作台、榜内实时搜索与当前播放歌曲定位）。
   - **母带声学调音台与蝰蛇音效工作站 (`RadioView.vue`)**：
     - 内置 **64-bit 风格母带级 WebAudio DSP 引擎**（10 段参量均衡器 + 5 大声学目标参考曲线 + A/B/C 自定义记忆槽 + 卷积混响与 M/S 立体声宽度矩阵 + 酷狗蝰蛇音效预设库）。
     - 支持 **Fletcher-Munson 夜间小音量等响度动态补偿**、**ReplayGain 响度均衡** 与 **0~8 秒跨曲平滑淡入淡出过渡**。
   - **我的音乐资产 (`LibraryView.vue`)**：
     - 上层 **快捷三联胶囊矩阵**（我喜欢的音乐、最近播放、概念专注舱 `Focus Pod` 番茄钟 × 4 路程序化白噪音混音台）。
     - 下层 **四维无框滑块 Tab (`自建歌单 / 收藏歌单 / 关注歌手 / 已购音乐`)**，支持歌单全量多页突破 100 首拉取、云端双向收藏/删歌、新建/删除歌单、批量匹配导入外部歌单/分享口令，以及左窄右宽同屏展开歌单/歌手主页 (`ArtistDetailView.vue`) / 专辑详情 (`AlbumDetailView.vue`)。
   - **平板级双栏沉浸播放舞台 (`NowPlayingView.vue`) & 独立透明桌面歌词 (`DesktopLyricWindow.vue`)**：
     - 沿用并升华 Android 平板横屏 `LandscapePlayerPage.kt`（左 `46%` 视觉控播舞台 : 右 `54%` 全高无框上下渐隐 KRC 逐字歌词舞台），支持一键切换 **KTV 伴奏音轨**、二层楼中楼热评抽屉、2K 歌词海报工坊与 1000P 原图预览。
     - 独立进程级透明置顶桌面歌词窗口，支持定长自由拖拽、硬件级鼠标穿透锁定（支持系统托盘菜单、设置页或全局快捷键 `Ctrl + Shift + L` 随时解锁）及蓝牙音频延迟校准同步。
   - **高可靠音源管线与纯 Rust 二进制标签引擎**：
     - 接入多网关候选节点重试、RFC 9639 FLAC `STREAMINFO` 真实采样率/位深探测，以及酷狗主备多 CDN 直链（`url` + `backupUrl`）无缝自动切源容灾。
     - 内置零外部依赖纯 Rust **FLAC (`Vorbis Comment` + `Picture`)** 与 **MP3 (`ID3v2.3 UTF-16LE` + `TYER`/`TDAT` + `APIC` + `USLT`)** 高清封面及双语歌词内嵌引擎。

## 二、快速运行与构建指南

```bash
npm install
npm run tauri dev
```

生产环境类型检查与构建：

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```
