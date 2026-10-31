# BreezeMusic · 微风音乐 (Windows 桌面概念版 v0.2.0)

基于 **Rust + Tauri 2.0 + Vue 3 (TypeScript + Pinia)** 构建的第三方「酷狗概念版 (`appid=3116`, `clientver=11440`)」Windows 桌面客户端。

---

## 一、v0.2.0 重大更新与修复说明

### 1. 四大核心问题精准修复
1. **扫码登录确认无响应修复 (`src-tauri/src/kugou/api.rs` & `LoginModal.vue`)**
   - 修正了 `/v2/get_userinfo_qrcode` 状态码映射：`status == 1` 为等待扫码、`status == 2` 为已扫码待手机端确认、`status == 4` 为手机端已确认授权、`status == 0` 为已过期。
   - 在检测到 `status == 4` 后，自动调用 `/v5/login_by_token` (`refresh_session_token`) 补全概念版 `t1` 与 `vip_token` 完整凭证。
2. **音质切换与 `0 kbps` 显示修复 (`api.rs` -> `deduce_quality_and_kbps` & `probe_stream_head_bytes`)**
   - 解决了酷狗 `/v6/priv_url` 在返回 FLAC / Hi-Res 无损流时不返回 `bitrate` 导致显示 `0 kbps` 的问题：自动通过 `(file_size_bytes * 8) / duration_sec / 1000` 反推精确码率。
   - 移植了 Android 端遵循 RFC 9639 的 **`Range: bytes=0-511` 硬件级流头探测**，直接解析远程音频流前 42~512 字节的 `FLAC STREAMINFO` / `OGG Vorbis` 识别头，在播放条与播放页实时展示如 `96.0 kHz · 24bit · 2304 kbps · FLAC` 的真实硬件规格，切换音质时自动无缝续播当前时间进度。
3. **听歌时长与等级对账修复 (`api.rs` -> `fetch_full_profile`)**
   - 严格对齐 Android 端 `ListeningGradeManager.kt` 与 `MusicRepository.userGradeInfo`，并发聚合 `/v3/get_my_info`、`/v4/get_grade_info` 与 `/v2/get_grade_info` 的秒级 `d_sec`，叠加标准版《酷狗音乐》本地缓存补偿值 `resolve_standard_client_local_diff_sec`，并格式化为与 Android 端完全一致的 **`Lv.11 · 累计听歌 X 小时 Y 分`**。
4. **歌曲时长 `--:--` 与「默认收藏 0 首」体验修复**
   - 从 `/v2/get_res_privilege/lite` 的 `relate_goods` 与 `audio_info` 中多源兜底提取 `timelength`，彻底消除列表 `--:--`。
   - 加载用户歌单时智能默认选中「我喜欢」或第一个曲目数 `> 0` 的歌单，不再默认停留在 0 首的空歌单。

### 2. 全新「双模多形态」桌面级 UI 架构（严格遵循设置归口原则）
- **无边框毛玻璃一体化窗口 (`decorations: false`)**：定制 `TitleBar.vue` 接管拖拽与窗口控制，全局彻底清除原生白色滚动条。
- **主页面保持极致干净**：发现页与我的页绝不堆砌杂乱的设置开关，所有全局外观、布局模式、默认音质与自动签到开关统一收口在 **「设置」** 页；而播放页专属的舞台/背景切换、歌词设置（2×3 六大四字主题网格）与 10 段母带参量均衡器 (PEQ) 则内嵌在 **「播放页左下角专属工具坞」**。
- **完整还原并升级 Android 端特色模块**：
  - **发现页 (`DiscoverView.vue`)**：「11/01 每日推荐」模糊封面大横幅（含全屏详情页）+「猜你喜欢 · 随心听」翠绿渐变私人 FM 卡 +「新歌速递」（华语/欧美/日韩/粤语/全部 5 语种流光切换）+「巅峰排行榜」矩阵（含榜单详情页）。
  - **我的页 (`LibraryView.vue`)**：个人等级与小时分钟时长卡 +「VIP 权益中心」鎏金卡（点击弹出含 **当月签到日历矩阵** 的 `VipCenterModal.vue`）+「我喜欢的音乐」红心卡 +「最近播放」青蓝卡 +「概念投喂 · Focus Pod」幻紫卡（点击弹出含 **番茄钟与 WebAudio 实时程序化白噪音合成器** 的 `FocusPodModal.vue`）。
  - **播放页 (`NowPlayingView.vue`)**：支持 **4 款左侧舞台形态**（流光黑胶唱盘 + S型唱臂 / 复古磁带机 + VU 电平表 / 赛博霓虹频谱 / 极简画廊方封）× **5 款动态光学背景**（流体极光 / 律动光晕 / 封面虚化 / 暗夜星尘 / 深邃纯黑）× **无边框上下渐隐 KRC 逐字流光歌词舞台**。
  - **设置页 (`SettingsPlaceholderView.vue`)**：支持一键切换 **三大桌面架构模式**（灵动悬浮岛画布 `Floating Bento Island` / 极简纯粹模式 `Zen Minimalist` / 桌面置顶迷你黑胶挂件 `Mini Turntable Widget`）、三套明暗色彩主题与四档默认在线音质。

---

## 二、快速运行指南

```bash
cd breezemusic-windows
npm install
npm run tauri dev
```
