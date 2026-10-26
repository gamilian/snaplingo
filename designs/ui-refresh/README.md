# SnapLingo · UI 设计提案

本轮基于当前应用代码重新设计全部主要界面，提供三套可以切换比较的完整视觉方向。此目录是独立的交互设计预览，选择方向后可将它落实到生产组件。

## 设计方向

- **A · 澄明 / Native**：冷白底色、蓝色强调、清晰的侧栏、柔和分组；优先考虑熟悉的桌面操作体验。
- **B · 夜航 / Studio**：石墨底色、琥珀强调、窄工具栏与上下文导航；优先考虑高信息密度与键盘效率。
- **C · 松间 / Paper**：暖纸底色、松绿色强调、横向主导航、阅读式分组；优先考虑长时间使用的舒适感。

三套设计保留相同功能范围，分别调整导航、密度、字体、材质、控件与内容布局，而不仅是换色。

## 范围与假设

- 桌面端优先，在较小预览窗口中也可以完整操作。
- 设置覆盖通用、截图、翻译、OCR、翻译/OCR/系统语音服务，以及历史记录和收藏。
- 独立场景覆盖翻译浮窗、OCR 浮窗、截图标注、贴图和首次权限引导。
- 使用演示数据；预览设置只保存在浏览器本地。翻译、OCR、授权、API 连通性均不调用真实服务。
- 保留现有应用图标；以当前代码中的功能、术语和流程为依据。系统字体优先，无在线字体或图片依赖。
- 包括设置修改、快捷键录制、服务编辑/排序、搜索筛选、收藏备注、文本复制、标注撤销/重做与空白/加载/失败状态。
- 快捷键按用户反馈保留原版动效：点击后行内录制，边框高亮，2 秒脉冲提示；松开主键自动保存，Esc 或失焦取消。每行保留恢复默认和清除入口。

## 来源

功能依据：`src/views/SettingsWindow/`、`src/views/ResultWindow/`、`src/views/CaptureWorkspace/`、`src/views/PinnedImageWindow/`、`src/views/RequiredPermissionsGate.tsx`。

窗口框架参考 `baoyu-design/starter-components/macos-window.jsx`，采用相同的桌面窗口结构和交通灯尺寸逻辑。

快捷键录制与动效依据 `src/views/SettingsWindow/Hotkey/HotkeyDisplay.tsx`、`HotkeyRow.tsx` 和 `FeatureHotkeysSection.tsx`。

## 预览

从仓库根目录运行：

```sh
node designs/ui-refresh/build.mjs
python3 -m http.server 4311 --directory designs --bind 127.0.0.1
```

打开 <http://localhost:4311/ui-refresh/>。已构建的 `index.html`、`preview.js`、`styles.css` 和 `app-icon.svg` 可作为一个完整目录离线分发。

## 验收

2026-10-26 已在浏览器中完成：

- 三种设计的 7 个主页面及 5 个独立场景逐页检查，控制台无错误。
- 780 与 390 像素宽度下共检查 72 组页面/场景，无页面或工作区横向溢出；检查后恢复默认视口。
- 验证外观切换、设置刷新后保留、快捷键行内录制/松键保存、Esc 与失焦取消、清除/恢复和 F1 输入。三套方案的录制脉冲均以原版的 2 秒节奏运行，390 像素宽度下录制状态无溢出。
- 验证服务启停、排序和配置，资料库搜索、类型筛选、分页、备注与标签。
- 验证翻译和 OCR 的加载/失败/空白状态、示例语言一致性、文本编辑及权限引导。
- 实际在画布上标注，验证撤销/重做和 PNG 导出；原图 960 × 600，旋转后导出 600 × 960。
- 各方案截图及并排对比保存在 `screenshots/`。交付前恢复演示内容，未替用户选择首选方案。

并排对比入口：<http://localhost:4311/ui-refresh/?scheme=native&scene=workspace&page=translation&view=compare>。

本轮另外修复了应用源码中的截图窗口关闭等待问题，详见 `../../docs/research/2026-10-26-capture-startup-latency.md`。UI 方案仍待选择后落地。

## 正式主题接入

2026-10-26 起，澄明、夜航、松间三套方案已作为正式界面主题接入应用。通用设置中的“界面主题”负责选择三套布局和配色；“外观模式”继续负责浅色、深色和跟随系统。选择会持久化，并通过已有设置变更事件同步到设置窗口、翻译/OCR 浮窗、截图和贴图窗口。

正式组件主题检查入口：<http://localhost:4312/designs/ui-refresh/themes.html>。这是本地验证壳，使用正式设置、资料库、服务卡片、结果卡片和截图工具组件；不会连接真实服务。
