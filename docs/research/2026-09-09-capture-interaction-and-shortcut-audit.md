# 截图交互与快捷键作用域核查

核查基线：2026-09-09，工作区提交 `1ea163d`。以下诊断记录描述修复前的行为及当时的验证结果；后续修复已完成，见文末「已实施修复与验证」。已安装应用尚未替换。

基线结论：最值得优先处理的是截图快捷键的作用域、异常退出清理、启动等待和选区绘制时序。已复现一个前端退出清理缺口和一个拖选多等待一帧的问题；原生探针确认隐藏窗口不会释放截图全局快捷键。这能解释“看起来没有在截图，复制却被吃掉”的一种路径，但尚不能确认用户遇到的那一次就是该路径。

## 修复前流程

```text
F1 松开
  → begin_capture_presentation：记录前台应用，全局注册 Esc、Cmd+C/S/Z/Y
  → 隐藏旧截图窗口
  → 抓取各屏合成像素并编码 PNG
  → 枚举窗口候选、获取光标，保存 CaptureSession
  → macOS 创建新的隐藏 WebView
  → React 启动，取得会话信息，再取回各屏 PNG 的 Base64
  → 全部底图解码、画布准备完成，显示并聚焦截图窗口
  → 悬停识别 / 拖选 / 微调
  → 普通截图：后端裁剪并编码预览，再返回编辑状态
  → 复制、保存、贴图或 OCR
  → 前端隐藏窗口，调用取消会话
  → 后端结束 presentation、注销全局键、恢复焦点、销毁闲置窗口
```

截图底图、预览与输出使用同一会话的冻结像素，这个一致性保证应保留。普通截图选区完成后进入编辑；`screenshot-copy` 直接复制；OCR 与翻译模式会等待对应处理，再结束截图会话。

主链路位于 `src-tauri/src/application/capture/runtime.rs:73`、`src-tauri/src/application/capture/session.rs:83`、`src/application/capture-workspace/runtime.ts:1583` 和 `src/views/CaptureWorkspace/useCaptureWorkspaceRuntimeView.ts:169`。

## 快捷键问题

### 已证实：输入占用与窗口可见性不是同一个生命周期

`src-tauri/src/infrastructure/system/capture_window/macos.rs:45` 在截图窗口创建前，调用系统全局快捷键注册，取得 `Escape`、`CmdOrCtrl+KeyC/S/Z/Y`。它们使用 `tauri-plugin-global-shortcut`，macOS 底层是 Carbon `RegisterEventHotKey`。

隐藏窗口只执行窗口操作，不释放这些注册。只有 presentation 计数从 1 降到 0 时，`end_capture_presentation` 才注销它们。复制回调只检查能否取得窗口，未验证窗口可见、聚焦或会话有效；即使回调直接返回，已经被系统全局注册消费的按键也不会自动交还原应用。

原生探针调用生产函数、真实注册系统快捷键，结果如下。探针不创建数据库，不注入用户按键；结束前显式释放了所有注册。

| 阶段 | 有截图窗口 | presentation 活跃 | 五个截图全局键已注册 |
| --- | --- | --- | --- |
| 初始 | 否 | 否 | 否 |
| begin 之后 | 否 | 是 | 是 |
| 调用 hide 之后 | 否 | 是 | 是 |
| 调用 end 之后 | 否 | 否 | 否 |

运行过的构建命令：`cargo run --locked --manifest-path src-tauri/Cargo.toml --example capture_shortcut_audit`。临时 example 已移出源码，源码保留在 `target/capture-audit/capture_shortcut_audit.rs`，本机可重跑已构建的 `target/debug/examples/capture_shortcut_audit`。

### 已复现：取得会话信息失败后，退出不清理原生会话

最小场景：原生会话已经创建并有已知 ID；前端 `getCaptureSession(id)` 出错；错误界面退出。

- `startSession` 只在成功取得会话之后，才把 ID 加入 `provisionalSessionIds`；失败时没有保留传入的 `requestedSessionId` 作为待清理资源。
- `cancelSession` 在 `state.session` 为空时只隐藏窗口并重置状态，未调用后端取消。
- `dispose` 同样找不到这个 ID，无法补充清理。

生产前端 runtime 的诊断结果：正常加载后退出会调用原生取消；注入读取失败后，界面隐藏且状态变为 `idle`，原生取消调用次数仍为 0；直接 dispose 的场景也为 0。结合上面的原生探针，这条失败路径会留下全局按键占用。

关键位置：`src/application/capture-workspace/runtime.ts:1583`、`:1630`、`:1360`、`:443`；后端释放入口为 `src-tauri/src/application/capture/runtime.rs:226`。

这属于故障注入复现，不是用户那次现场故障的完整回放。现有日志没有证明这次读取失败曾经发生。

### 仍需验证的两条路径

| 假设及可验证预测 | 代码依据 | 本轮结论 |
| --- | --- | --- |
| 截图期间再次按 F1，旧 WebView 被销毁但旧会话未结束；若复现，退出新会话后 presentation 仍活跃 | 后端每次启动增加计数；macOS 重建窗口；原生销毁没有按会话 ID 释放的钩子，React 清理只接在组件卸载上 | 存在生命周期风险，未完成真实连续按键复现 |
| 系统注销失败后，内部标志误报已释放；若复现，应看到注销警告但系统仍占用按键 | `macos.rs:657` 先 `swap(false)`，再调用可能失败的 unregister；失败只记日志 | 现有日志未发现注册/注销失败，不能认定是本次根因 |

另外，`captureActions.ts:542` 在输出或预览生成期间忽略失焦，错误/加载状态也不触发失焦取消；全局键因而可能跨越原应用重新成为前台的阶段。此处需要区分保存对话框等合法临时失焦与真正离开截图。

设置模块已经禁止用户把 `Cmd+C` 配成常驻全局快捷键（`src-tauri/src/application/hotkeys/policy.rs:78`），截图原生适配器却绕过该策略直接注册。这说明需要统一的是输入作用域及其所有权。

## 体感差距的具体来源

### 启动有实测等待，且每次都重建 macOS WebView

运行中的 `/Applications/SnapLingo.app` 二进制时间为 2026-09-08 23:06；工作区 release 二进制时间为 2026-09-09 10:09，两者均标为 0.1.4，不能当作同一构建。

安装版数据库最近一次截图记录发生于北京时间 2026-09-09 09:19:48：

| 阶段 | 日志值 |
| --- | --- |
| 按下到松开 F1 | 139.5 ms |
| 抓取并编码各屏快照 | 74.5 ms |
| 枚举 30 个窗口候选 | 160.1 ms |
| 创建会话总计 | 258.0 ms |
| 创建截图窗口 | 176.7 ms |
| 窗口创建完成到前端取回会话 | 500.4 ms |
| 松开 F1 到前端取得会话信息 | 939.1 ms |

阶段数值有包含关系，不应全部相加。939.1 ms 尚未包括完整底图传输、解码和最终显示，也不是当前源码或 Snipaste 的对照基准。

同次截图是一块 3840×2160 像素屏幕，冻结 PNG 为 19,934,849 字节，转为 Base64 约 26.6 MB。主链路必须等窗口候选、WebView 启动和底图准备完成，才能出现选区反馈。

当前源码仍明确关闭 macOS 预热和复用（`capture_window/tauri.rs:445`）；测试说明重建窗口是为重新定位到全屏 Space。优化应保留这项约束，不能只把两个布尔值翻转。源数据 PNG/Base64 的成本也需要与 WebView 初始化分别计时。

最近一次正常退出日志完整记录了五个全局键注销，因此不能据此声称安装版一直占用 `Cmd+C`。

### 已复现：拖选绘制多排了一次动画帧

实际链路：`CaptureWorkspaceView` 的 pointermove → `capturePointerFrame` 的 requestAnimationFrame → runtime 更新选区 → React 提交 → `captureSelectionOverlayRuntime` 再 requestAnimationFrame → Canvas 绘制。

诊断挂载真实 CaptureWorkspace，派发 PointerEvent，并观察绘图调用；只替换图像解码和最终绘图，以便可控地逐帧推进。结果：

```text
{ firstFramePaints: 0, secondFramePaints: 1 }
```

这证明在该调度条件下，多等待了一次帧调度。60 Hz 下额外一帧约 16.7 ms；这不是屏幕到眼睛的端到端延迟测量。位置见 `capturePointerFrame.ts:32`、`CaptureWorkspaceView.tsx:289`、`captureSelectionOverlayRuntime.tsx:183`。

### 选区确认、缩放后的预览仍依赖后端图像往返

`runtime.ts:760` 将状态设为 preview，但同时设置 `isRenderingOutput=true`，等待 `renderCaptureOutput`。后端从 PNG 解码、裁剪/合成、编码 PNG，再以 Base64 返回。工具栏的大部分操作绑定这个 busy 状态，因此“已经松手”和“能继续编辑”之间存在等待。

这条路径对初次框选、调整选区以及最后输出有重复的图像处理成本。普通标注已经有前端画布，不应笼统地说每一笔标注都会完整走后端重绘。证据位于 `application/capture/image_composer.rs:160`、`application/capture/render.rs:42`、`captureEditorToolbar.tsx:297`。

OCR 模式的处理效果排列为识别 → 记录选区 → 结束会话，因此识别耗时也会延长截图层和输入占用；应将“结束选区交互”与“保留冻结像素完成后台处理”分开。

### Snipaste 的常见操作已经有较多对应实现

核对 [Snipaste 官方快捷键](https://docs.snipaste.com/key-bindings) 后，当前代码已经具备 Tab 切换窗口/控件识别、R 恢复选区、历史选区、方向键微调、Enter 复制、右键退回、放大镜和多种标注等行为。不能把体感差异简单归结为缺少这些功能，也不能在未测量时推断 Snipaste 的内部实现或速度。

本轮证据更支持先改善输入释放、首屏出现、拖选反馈和选区到编辑的连续性。

## 建议实施顺序与验收

| 优先级 | 改动方向 | 验收信号 |
| --- | --- | --- |
| P0 | 将复制、保存、撤销等改为截图窗口作用域的原生键盘处理；全局层保留启动入口 | 截图窗口不活跃时，其他应用的 Cmd+C/S/Z/Y 和 Esc 始终可用；文本标注/保存对话框保留自己的编辑语义 |
| P0 | 后端按会话 ID 管理交互资源，退出与失败都幂等释放；前端加载失败也保留已传入 ID 供清理；处理窗口销毁和替换 | 普通退出、读取失败、连续 F1、渲染失败、销毁窗口之后都无残留注册；重复取消旧会话不影响新会话 |
| P1 | 合并鼠标采样、几何更新和选区绘制的帧调度 | 上述诊断在第一帧看到新选区；再用真实刷新率检查连续拖动 |
| P1 | 使用已解码冻结底图就地预览，输出时再编码；将预览等待与保存/OCR 等最终任务的 busy 状态分开 | 松手立即进入可操作编辑状态，调整选区不中断输入，最终像素仍与冻结帧一致 |
| P1 | 分段测量并压缩启动关键路径，减少大图字符串传输，评估能正确处理 Space 的窗口预热/复用方案 | 同一构建记录按键、冻结、窗口就绪、底图传输、解码、可交互的耗时；冷/热启动及多屏都验收 |
| P2 | 补齐原生输入与故障路径集成回归 | 原有前端单测之外，验证系统实际按键投递、焦点恢复、全屏 Space 与远程桌面 |

仅在全局回调里增加 `if (!active) return` 无法完成 P0：按键已经在系统注册层被截走。需要从注册作用域和释放责任上解决。

## 修复前验证记录与边界

既有相关测试：4 个文件、119 项全部通过。

```sh
npm test -- src/application/capture-workspace/runtime.test.ts src/views/CaptureWorkspace/CaptureWorkspaceLifecycle.test.ts src/application/capture-workspace/captureKeyboardHostRuntime.test.ts src/views/CaptureWorkspace/capturePointerFrame.test.ts
```

显式诊断套件：4 项中 1 个正常路径对照通过、3 个问题断言失败，两次执行结果一致。故障断言失败是本轮诊断证据，不表示已经修复。

```sh
npm test -- --config target/capture-audit/vitest.config.mjs
```

诊断源码存放在被 Git 忽略的 `target/capture-audit/`，使用 `.probe.ts` 和独立配置，不加入默认测试集合。原生探针构建、运行成功，退出前清理了临时系统注册。诊断阶段未修改应用实现；后续修复记录如下。

通过桌面 UI 工具读取安装版窗口时超时，未完成真实连续 F1/切换应用/外部应用复制的端到端复现；没有测量 Snipaste，也没有取得用户故障发生那一刻的窗口和焦点状态。上述故障注入、原生注册验证、帧调度验证与历史性能日志各自证明的范围已分别说明。

## 已实施修复与验证

本节记录 2026-09-09 在上述基线上完成的源码修复。

### 快捷键与会话清理

- 删除 macOS 截图适配器对 `Esc`、`Cmd+C/S/Z/Y` 的全局注册，以及对应的原生事件转发。截图编辑快捷键由聚焦的截图 WebView 处理，输入框、文本标注和输入法组合输入保留自身的编辑行为。
- 后端按会话 ID 记录 presentation 所有权；重复取消、未知 ID、先恢复再取消都不会重复减少 presentation 计数。窗口销毁回调绑定其会话 ID，防止旧窗口的清理影响新会话。
- 前端在读取原生会话前保存传入 ID，读取失败后仍可退出和清理。退出会同时清理恢复的旧会话与加载失败的新会话；隐藏窗口失败也会继续释放原生资源，失败的取消保留供后续重试。
- 修复 React StrictMode 的 effect 重连误取消同一个原生会话的问题，同时保留真实卸载时的清理。

永久原生探针 `src-tauri/examples/capture_shortcut_scope.rs` 已通过。它调用生产 presentation 函数并查询实际系统注册器：空闲、窗口创建前、重叠且隐藏的 presentation、仅剩一个 presentation、全部结束这五个阶段，五个编辑键均未被注册。该验证没有向其他应用注入按键。

### 截图交互与启动

- 保留 pointermove 的一次逐帧合并，选区画布在 React layout commit 中直接绘制，删除第二次 RAF。挂载真实截图视图的回归测试已确认第一帧即可画出最新选区。
- 确认选区与调整大小时，直接使用已解码的冻结屏幕图片绘制预览，不再等待后端裁剪、编码和返回图片。预览与标注分层绘制，标注更新无需重绘底图，马赛克仍采样冻结像素。
- 前端通过 `capture-image` 自定义协议读取缓存 PNG 二进制，IPC 只返回屏幕元数据，避免大型底图的 Base64 字符串传输。放大镜复用已解码的屏幕图片；最终复制、保存、贴图与 OCR 继续使用原会话像素。
- 新增独立的 `CaptureApp` 入口，截图窗口启动时只组装截图与必要设置模块；保留 macOS 为全屏 Space 重建窗口的策略。增加 `window_prepared`、`surface_decoded` 时间标记供后续实际启动测量。

在隐藏的真实 macOS WKWebView 中运行了生产图片协议处理器，使用已知 2×2 PNG 验证 Tauri URL 转换、PNG 解码、跨域画布读取与缺失图片响应：

```text
PASS: capture-image://localhost/%2Ffixture%2FDisplay%20A;
decode, canvas RGBA 12,34,56,255, missing image 404
```

此探针使用固定像素，不读取用户桌面或数据库。临时源码保存在 `target/capture-audit/capture_image_probe.rs`，已从 Cargo examples 中移除。

### 最终检查

| 检查 | 结果 |
| --- | --- |
| `npm test` | 123 个文件、908 项测试通过 |
| `npm run build` | TypeScript 与 Vite 生产构建通过 |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml --quiet` | 545 项库测试与 13 项集成测试通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets` | 通过 |
| `git diff --check` | 通过 |
| 原生快捷键作用域、WKWebView 图片协议探针 | 通过 |

仍未进行 Snipaste 对照测量、修复后端到端启动延迟测量，以及全屏 Space、多块物理显示器、远程桌面和外部应用实际复制的人工验收。构建成功及探针结果不替代这些场景的体验验收。

本次改动位于工作区，未提交、发布或替换 `/Applications/SnapLingo.app`。已安装旧版不会自动获得修复，需要用当前源码重新构建并启动应用。

## 2026-09-11：F1 等功能键的正常按键录入修复

用户反馈：点击录制后，F1 没有反应、仍在等待输入，F2/F3 也可能受影响；`Cmd+Shift+R` 可以设置，直接修改数据库可以配置 F1。带独立 F1 按键的键盘也有同样的问题，用户要求通过正常按键完成录入。

### 根因

实际设置页使用 `FeatureHotkeysSection`，原录制器只监听 DOM `keydown`。本机配置中 F1 已绑定截图、F3 已绑定贴图；已注册的全局快捷键由原生注册通道消费，事件无法继续到达页面。原生回调与页面录制器之间缺少转发，导致页面一直等待输入。后端本身支持 F1–F20，也允许不带修饰键保存，因此直接修改数据库能够配置。

早期 WKWebView 探针直接向自身投递 NSEvent，绕过了全局快捷键消费环节。它能验证标准 F1/F2/F3 的页面事件格式，但不能据此认定真实录入流程正常。

### 修复

- 已撤掉功能键选择框及 Fn 提示，保持点击快捷键后直接按键录入的交互；已有快捷键进入录制时也显示等待输入。
- 开始录制时，先订阅原生录制事件，再创建绑定设置窗口和唯一 ID 的录制会话。已注册的按键通过原有全局快捷键回调转发，未注册的按键继续使用 DOM 事件。DOM 功能键在 `event.code` 缺失或无法识别时也能通过 `event.key` 识别。
- 两条路径均在松键后提交，沿用原有注册、冲突校验与保存流程。录制期间消费已注册按键的按下和松开事件，避免录入这一按触发截图或贴图；保存完成后结束录制，保存失败保留原配置并显示错误。
- Esc、区域外点击、窗口失焦、关闭及组件卸载会清理录制状态。回调在转发前检查设置窗口焦点，事件仅发送到对应窗口；会话 ID 防止迟到事件和旧录制的清理影响新录制。
- 没有增加全局按键监听，也没有为录制暂停或重新注册全部快捷键。既有 Cmd+C 等编辑快捷键保护保持；录制结束后页面也不再消费这些按键。

### 验证

新增设置页录制回归测试 20 项，覆盖已注册 F1/F3/⌘F1 的原生转发、DOM 功能键、`Cmd+Shift+R`、松键保存、重复按键、保存期间路由、取消及失焦、Cmd+C、迟到启动和事件、保存失败，以及缺少 `crypto.randomUUID` 的旧 WebView。后端补充录制会话清理、原生快捷键格式转换和 F1 正常注册持久化测试。

原生探针 `src-tauri/examples/hotkey_recording_scope.rs` 使用真实 Tauri 注册器、生产回调、录制服务及窗口事件投递，将临时 F18 的 Carbon 快捷键事件投递到自身进程，验证录制前后的正常动作、录制期间松键转发且不执行原动作、取消及失焦清理，以及 Cmd+C 未被注册。探针通过后已注销临时快捷键：

```text
PASS: native F18 recorded on release; its action was suppressed only while recording; cancellation and focus loss restore normal routing; Cmd+C remains unregistered
```

| 检查 | 结果 |
| --- | --- |
| `npm test` | 124 个文件、933 项测试通过 |
| `npm run build` | TypeScript 与 Vite 生产构建通过 |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml --quiet` | 562 项库测试、13 项集成测试通过；1 项手动计时测试忽略 |
| `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets` | 通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `git diff --check` | 通过 |
| 原生快捷键录制探针 | 通过 |

原生探针没有向桌面或其他应用注入按键，结果不等同于用户物理 F1 键盘及外部应用复制的端到端人工验收。改动仍在源码与构建目录，未提交、发布或替换 `/Applications/SnapLingo.app`；已安装旧版需要重新构建更新才能获得修复。
