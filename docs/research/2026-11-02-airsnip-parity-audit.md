# SnapLingo 对 AirSnip 的功能补齐复核

复核日期：2026-11-02（Asia/Shanghai）。当前工作区：`codex/airsnip-parity`，基础 HEAD：`2a101ae07872c7d4c9171c29454323742fa73365`。

**本轮审计发现的功能缺口已经实现，并通过完整自动检查与实际组件 UI 核验。SnapLingo 已覆盖 AirSnip 的截图、六种标注、文本、三格式输出、滚动长截图、贴图、设置和界面缩放能力。** 默认快捷键、部分窗口策略和平台交付方式仍有明确差别，列在下文；这不是逐像素、性能或所有平台已实测的结论。实现目前位于本工作区和本机安装包中，尚未提交或发布。

此报告已用实现后的结果替换先前的“尚未对齐”表。AirSnip 的功能事实以独立的 [79 条实际行为清单](2026-11-02-airsnip-current-feature-inventory.md) 为准，避免把 README/PRD 中的计划功能混入补齐范围。

## 固定对照基线

- 最新正式版 [v0.6.0](https://github.com/bd-dxg/airsnip/releases/tag/v0.6.0)，源码提交 `13dfee6a8a0249187a0f215c8d06d462032c176e`。
- annotated tag 对象 `8c49d7300932e0459408e17e2294f793e4fcb894` 指向上述提交，不能将 tag 对象当作源码提交。
- 当前 main `f3e0718f486845f42ba0e556a9c95b68a733facb`。统一界面缩放属于 main 新增行为，尚未进入正式 Release。
- 只读取官方参考源码并独立实现行为；未复制 GPL AirSnip 源码或品牌素材进入 MIT SnapLingo，也未运行 Windows AirSnip。

## 已补齐的具体缺口

| 参考 ID | 本轮最终行为 | 本地实现证据 |
| --- | --- | --- |
| E11 | 滚轮按鼠标位置调整悬停对象，未悬停时调整当前工具默认；模糊调整 radius，Ctrl/Cmd 为三倍步进 | [编辑交互](../../src/application/capture-workspace/editing.ts)、[尺寸计划](../../src/application/capture-workspace/captureEditorRuntime.ts) |
| E12、T04、T07 | 线宽 1–32，模糊 4–64 / 默认 16，文字控件 10–96；文字 −/+ 步进 2；文字手柄按纵向比例调整字号 8–160 | [范围](../../src/application/capture-workspace/annotationStyle.ts)、[手柄](../../src/application/capture-workspace/annotationGeometry.ts)、[属性栏](../../src/views/CaptureWorkspace/captureEditorToolbar.tsx) |
| E13 | Ctrl/Cmd+Shift+Z 与 Ctrl/Cmd+Y 重做；历史最多 100 步；清空改用工具栏或 Ctrl/Cmd+Alt+Backspace | [快捷操作](../../src/application/capture-workspace/captureActions.ts)、[历史](../../src/application/capture-workspace/annotationHistory.ts) |
| E14 | “记住上次标注工具”开关生效；本次运行内记忆，窗口重建后仍从原生运行状态恢复；关闭后每次从矩形开始 | [前端运行时](../../src/application/capture-workspace/runtime.ts)、[原生运行时](../../src-tauri/src/application/capture/runtime.rs)、[设置](../../src/views/SettingsWindow/Screenshot/ScreenshotSettingsPage.tsx) |
| T01、T05、T06 | 中文 IME 输入保留；Enter 提交，Shift/Ctrl/Cmd+Enter 换行，Esc / Ctrl/Cmd+Z 放弃；已有文字双击再编辑 | [文字编辑器](../../src/views/CaptureWorkspace/capturePreviewPresentation.tsx)、[草稿模型](../../src/application/capture-workspace/textAnnotationDraft.ts) |
| O02–O04 | 原生另存为提供 PNG/JPEG/WebP；按后缀编码，支持 .jpg/.jpeg，缺后缀补当前格式，未知后缀报错；成功后记忆目录/格式，目录失效回配置目录 | [对话框](../../src/platform/tauri/capture.ts)、[输出](../../src-tauri/src/application/capture/output.rs)、[偏好保留](../../src-tauri/src/application/settings/configuration.rs) |
| O05、O06 | WebP 改为无损 VP8L；增加纯 Unix 毫秒命名，保留日期时间/日期及原有命名选项 | [编码与命名](../../src-tauri/src/application/capture/output.rs) |
| O08、S07 | 编辑器 F3 贴当前选区；全局贴图入口在截图期间路由到当前选区，避免再贴一次剪贴板；空闲时保留剪贴板/恢复入口 | [前端键](../../src/application/capture-workspace/captureActions.ts)、[原生路由](../../src-tauri/src/application/capture/runtime.rs) |
| C05–C08 | Space+拖动选区在绘图工具激活时也生效；方向键 1 / Shift 5px；点击选区外重选；最小选区 2px；普通预览 Enter 复制 | [交互](../../src/application/capture-workspace/runtime.ts)、[编辑](../../src/application/capture-workspace/editing.ts) |
| P02、P03、P05、P06 | 虚拟桌面居中 + 20px 级联并夹回边界；透明窗口保留 PNG alpha；滚轮缩放保持中心，比例 0.1–8 / 边长 4096；透明度 12/255–1 / 步进 24/255 | [位置](../../src-tauri/src/infrastructure/system/window_position.rs)、[窗口](../../src-tauri/src/infrastructure/system/pinned_window/tauri.rs)、[缩放适配器](../../src/platform/tauri/pinnedWindow.ts)、[模型](../../src/application/pinned-image/model.ts) |
| P07、P08、S03 | 聚焦贴图内的边框组合键可配置，并同步全部贴图与后续默认；保留 B 默认键，支持修饰键/数字/F 键 | [组合键](../../src/application/pinned-image/borderShortcut.ts)、[设置入口](../../src/views/SettingsWindow/Screenshot/PinBorderShortcutSettings.tsx)、[贴图 View](../../src/views/PinnedImageWindow/index.tsx) |
| P09 | 右键“关闭全部贴图”跨所有组关闭原生窗口，并清空可恢复队列 | [原生运行时](../../src-tauri/src/application/pinned_image/runtime.rs)、[菜单](../../src/views/PinnedImageWindow/index.tsx) |
| P10、P11 | 复制/保存输出当前缩放后的尺寸，保留内容 alpha，不烘焙边框/窗口透明度；快速保存默认目录日期时间 PNG，同时保留三格式另存为 | [导出](../../src-tauri/src/application/pinned_image/state.rs)、[前端输出](../../src/application/pinned-image/runtime.ts) |
| L07、L11、L12 | 临时 Enter / Esc / Ctrl/Cmd+Z；手动完成补采最后一帧；高度可配置，达到高度/总像素上限连续裁剪末段并自动复制 + 保存默认目录日期时间 PNG | [任务](../../src-tauri/src/application/capture/scrolling.rs)、[拼接器](../../src-tauri/src/application/capture/scrolling_image.rs)、[原生快捷键](../../src-tauri/src/infrastructure/system/scrolling_window.rs) |
| L08–L10 | 独立透明预览窗，OS 点击/滚轮穿透且不抢焦点；较宽侧底部布局并避让控制条；空间不足禁用预览按钮；独立穿透视口边框，控制条优先下方/上方；采帧排除重叠窗口并统一关闭 | [窗口](../../src-tauri/src/infrastructure/system/scrolling_window.rs)、[View](../../src/views/ScrollingCaptureWindow/index.tsx)、[单采样器](../../src/application/scrolling-capture/runtime.ts) |
| S03–S06 | 标注/长截图可录字母、数字、F1–F24；物理 code 避免 IME Process 键；内置操作键保留；已注册 OS 键通过原生录制通道显示冲突，不误触原动作；Windows Win 修饰键独立显示/存储 | [录制](../../src/views/SettingsWindow/Screenshot/AnnotationShortcutSettings.tsx)、[录制桥](../../src/views/SettingsWindow/Screenshot/useNativeShortcutRecording.ts)、[原生策略](../../src-tauri/src/application/hotkeys/policy.rs) |
| S11、S12 | 关于增加项目地址与可搜索第三方许可正文，按锁文件生成/核查；单实例插件在数据库/日志初始化前生效，重复启动唤起首个进程设置 | [关于](../../src/views/SettingsWindow/General/GeneralPage.tsx)、[许可生成](../../script/third-party-notices.mjs)、[启动](../../src-tauri/src/lib.rs) |
| U01、U02 | 设置、截图工具/字体/手柄/HUD/光标、长图控件共用 UI 倍率；自动分辨率补偿 .6 指数、最高 3 倍，避免重复叠加 OS DPI；标注与图像坐标保持原值 | [倍率](../../src/application/capture-workspace/interfaceScale.ts)、[原生倍率](../../src-tauri/src/application/capture/interface_scale.rs)、[设置缩放](../../src/platform/tauri/settingsWindow.ts) |

## 79 条行为的覆盖范围

| 完整清单范围 | 覆盖结论 |
| --- | --- |
| C01 | 托盘常驻/便携交付已支持；Windows 单 exe、管理员要求与资源体积属于平台实现差别，见下文 |
| C02–C10 | 冻结多屏、框选、调整、移动、重选、取消、复制及随选区布局工具栏已覆盖；保留 SnapLingo 原有分层 Esc 操作 |
| E01–E14 | 六种参考工具及曲线箭头/控制柄、圆角、Multiply 区域高亮、模糊、聚光、多对象编辑、样式、滚轮、历史和上次工具记忆已覆盖 |
| T01–T07 | 系统字体、粗斜体、中文输入、字号按钮/滚轮、再编辑、提交/放弃及文字缩放已覆盖；另保留文本选择/粘贴能力 |
| O01–O08 | 剪贴板图片、三格式另存为、偏好、后缀、无损 WebP、命名、复制/保存/贴图已覆盖；错误时保留编辑内容 |
| L01–L13 | 手动纵向上下拼接、固定页眉页脚、无位移/失败处理、撤销、收尾、预览、边框、限高与自动输出已覆盖；另提供独立复制/另存为/贴图 |
| P01–P12 | 截图/剪贴板来源、透明置顶窗、位置、移动、缩放、透明度、共享边框、关闭全部、当前尺寸输出及快速 PNG 保存已覆盖；保留分组等额外能力 |
| S01–S13 | 托盘、设置、改键/冲突、全局贴图路由、自启、主题、持久化、关于、单实例已覆盖；Windows 托盘恢复由所锁定 tray-icon 提供 |
| U01–U02 | main 的统一界面倍率与自动分辨率补偿已覆盖；按各平台逻辑坐标处理 OS DPI |

Windows 图片互操作使用锁定的 `arboard 3.6.1`：写入 PNG 和 DIBV5，读取优先 PNG；DIB / BITMAP 与 DIBV5 的标准转换由 Windows 提供。不能因本项目没有手写 CF_BITMAP 分支就判断为无位图支持。依据：[锁定依赖源码](https://docs.rs/crate/arboard/3.6.1/source/src/platform/windows.rs)、[Microsoft 格式合成规则](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats#synthesized-clipboard-formats)。具体软件的互操作仍列入 Windows 实机验收。

## 保留的产品与实现差别

- **品牌与默认值**：使用 SnapLingo 身份、MIT 和项目地址。保留现有字母工具键、B 边框键、线宽默认值及分层 Esc；这些键可通过设置调整。对方赞赏二维码属于其项目内容。
- **样式记忆**：工具在进程内记忆；SnapLingo 仍保留原有线宽/字号默认设置和编辑样式行为，AirSnip 每次截图重置样式。首次运行仍从矩形开始。
- **贴图**：保留初始适配大小及原图供后续高质量重采样；浏览器负责显示缩放，导出使用 Lanczos 重采样，没有复制其 120ms 原生快速/精细采样策略。没有强制新增 20 张上限；原有分组、缩略、灰度、反色、旋转、隐藏/恢复继续可用。
- **输出恢复**：无后缀自动补齐；JPEG 保留质量设置。复制/保存失败保留内容，长图部分成功明确显示已保存路径或已复制，并允许重试；AirSnip 部分失败直接退出。长图采样间隔为 250ms，不声称与其 90ms 性能一致。
- **系统集成**：全局 F3 路由当前截图，而非临时注销/重新注册；自启使用 Tauri 平台能力，并读取真实系统状态。设置存入现有 SQLite，最后保存偏好不会被设置页旧快照覆盖。
- **Windows 交付**：已接入 portable ZIP、依赖资源/许可、校验和、CI 与启动 smoke 脚本；仍依赖 WebView2。没有改成 AirSnip 的纯 Windows 单 exe，也没有体积/内存/延迟等性能对齐结论。
- **范围**：AirSnip 的 OCR 是占位，录屏/GIF/云同步/历史不属于其已有能力，未以此扩展本轮任务。SnapLingo 已有 OCR/翻译/历史/收藏保持原有范围。

## 实际核验结果

- `npm test`：130 个文件通过，**1011 项通过，1 项 Windows 条件测试跳过**。
- `cargo test --all-targets`：**654 项通过，2 项手动测试忽略**；架构检查 **9 项**、集成检查 **3 项**通过。忽略项为真实 bundled medium OCR smoke 和本机 WindowServer 计时探针。
- `cargo check --all-targets`、`cargo fmt --all -- --check`、前端类型检查/生产构建、`git diff --check` 均通过。
- 新增回归覆盖悬停模糊、工具改键优先级、文字提交、输出后缀/无损、保存偏好、全体关闭/当前尺寸导出、限高连续裁剪、自动输出部分失败、窗口重建后的工具记忆、串行中心缩放及预览布局。
- macOS 原生 `hotkey_recording_scope` 通过：实际 Carbon 注册键在录制时仅释放后回传且不触发原动作；取消/失焦恢复正常路由，Cmd+C 保持未注册。独立 `single_instance_scope` 已验证第二进程在 setup 前退出，并通知首个进程。
- [自包含的实际组件预览](../../designs/ui-refresh/airsnip-parity.html) 使用内存原生端口，不写真实应用数据。已核验设置 100%/200%、浅/深色、许可正文搜索、截图选区/区域模糊/中文文字/−+粗体/Enter 提交、贴图关闭全部和长图完成反馈；预览在窄容器内保留窗口实际尺寸并可滚动。
- UI 截图：[150% 编辑器](../../designs/ui-refresh/screenshots/parity-capture-text.jpg)、[200% 深色长图控件](../../designs/ui-refresh/screenshots/parity-long-200-dark.jpg)。
- `npm run tauri:build` 重新生成并验证本机 **SnapLingo 0.1.22 aarch64 app / DMG（本地预发布构建）**；使用现有稳定本地签名证书。未公证 beta 的 Gatekeeper 拒绝符合当前交付流程。

## 验收边界与交付

本地验证用 DMG：`target/release/bundle/dmg/SnapLingo_0.1.22_aarch64.dmg`；该 0.1.22 工件用于此前验证，并非本次 0.1.23 发布产物。现有 `/Applications/SnapLingo.app` 和用户数据未覆盖。

Windows 原生交互、Windows 125%/150% 与负坐标混合 DPI、多应用剪贴板互操作、explorer/睡眠恢复以及 Linux X11/Wayland 仍需对应环境验收。本轮没有这些机器的运行结果；不能把 CI 脚本接入或 macOS 构建成功表述为 Windows 包已生成/发布，或全部跨平台桌面行为已实测。
