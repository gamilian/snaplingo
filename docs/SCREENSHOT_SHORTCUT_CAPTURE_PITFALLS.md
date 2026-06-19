# 快捷键截图排障记录

这份文档记录本次修复 macOS Release 版快捷键截图时踩过的坑。以后如果出现“按快捷键没反应”“截图黑屏/白屏”“提示权限但已经授权”“选区后没有进入编辑页”等问题，先按这里排查。

## 最终目标

按配置的截屏快捷键后应满足：

- 不隐藏当前桌面上的其它应用窗口。
- 使用触发瞬间的真实屏幕画面作为选区背景。
- 进入十字星框选状态。
- 选区完成后回到主窗口的截图编辑页，并显示最近截图。
- 如果缺少屏幕录制权限，每次触发都要有可见提示。

## 坑 1：快捷键显示值不等于后端可注册值

界面展示的是 `⇧⌘R`，但后端全局快捷键插件需要解析器支持的 accelerator。

修复点：

- 后端注册使用物理键格式：`CmdOrCtrl+Shift+KeyR`。
- 前端设置页和默认配置继续展示用户友好的 `⇧⌘R`。
- 全局快捷键回调只处理 `ShortcutState::Pressed`，避免按下和抬起各触发一次。

相关文件：

- `src-tauri/src/lib.rs`
- `src-tauri/src/infrastructure/system/shortcut.rs`
- `src/stores/settingsStore.ts`
- `src/components/SettingsWindow/Screenshot/HotkeysPage.tsx`

## 坑 2：先打开 overlay 再截图会截到自己

之前的错误路径是：先打开截图窗口、隐藏 SnapLingo 自己的窗口，或在 overlay 可见后再捕获屏幕。结果会出现黑屏、白屏，或只能看到 SnapLingo 自己，桌面其它应用都不见了。

正确顺序：

1. 快捷键触发后，后端创建 `CaptureSession`，先冻结当前屏幕快照。
2. 截图前不要隐藏 `main` 或 `pin-*` 窗口；只在已有 `capture` overlay 可见时隐藏 overlay 自己。
3. 创建或复用独立的 `capture` webview，并保持隐藏状态。
4. 前端拿到 session 后渲染预先捕获的屏幕快照，再 reveal capture 窗口进入十字星框选。

关键原则：

- 不要在截图前隐藏主窗口、贴图窗口或其它应用窗口。
- 不要在 overlay 可见后再调用截图 API。
- 不要用固定 sleep 等待前端加载完成。

相关文件：

- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/capture_session_commands.rs`
- `src/components/ScreenshotSession/index.tsx`
- `src/components/ScreenshotSession/captureWindowVisibility.ts`

## 坑 3：overlay reveal 有时机竞态

如果后端创建窗口后立刻 `show()`/`set_focus()`，前端可能还没完成 session 读取和截图背景渲染，结果 overlay 会闪黑、闪白或空白一帧。

修复点：

- 后端创建 capture 窗口时使用 `.visible(false)`。
- 复用已有 capture 窗口时只 emit `hotkey-triggered`，不在后端提前 `show()`/`set_focus()`。
- 前端 `ScreenshotSession` 进入 `selecting` 或 `preview` 后再调用 `show()`/`setFocus()`。
- reveal 后再恢复截图前临时隐藏的窗口，避免恢复动作覆盖 overlay。

相关文件：

- `src-tauri/src/commands/capture_session_commands.rs`
- `src/components/ScreenshotSession/captureWindowVisibility.ts`
- `src/components/ScreenshotSession/index.tsx`
- `src-tauri/capabilities/default.json`

注意：如果后续尝试 macOS transparent window，要单独评估 Tauri 的 `macos-private-api` 影响，不要把它作为默认排障手段。

## 坑 4：选区完成后没有回到编辑页

选区成功不代表用户能看到结果。之前裁剪后没有稳定进入截图编辑状态，所以看起来像“截图不知道有没有成功”。

修复点：

- `ScreenshotSession` 使用 session id 渲染选区输出。
- 普通截图选区后进入 preview/editor 状态。
- 复制、保存、贴图、OCR 等动作都通过 `render_capture_output` / `output_capture` 统一输出。
- 捕获或权限错误通过 capture session error 状态和日志暴露。

相关文件：

- `src/App.tsx`
- `src/components/ScreenshotSession/index.tsx`
- `src/components/ScreenshotSession/captureActions.ts`
- `src-tauri/src/commands/capture_session_commands.rs`

## 坑 5：macOS 权限已经给了，Release 仍然反复提示

macOS TCC 屏幕录制权限绑定的是应用身份，不只是应用名字。Release 构建如果使用 ad-hoc 签名、bundle id 改动，或 designated requirement 带每次构建变化的 cdhash，系统会把它当成另一个应用。

修复点：

- Bundle ID 固定为 `com.snaplingo.app`。
- Release 构建后用稳定的本地代码签名身份重新签名。
- 最终签名不能是 ad-hoc。
- designated requirement 不能绑定每次构建变化的 `cdhash H`。
- 重新创建 DMG，保证 DMG 里的 app 也是重签后的 app。

相关文件：

- `src-tauri/tauri.conf.json`
- `script/fix-macos-release-signing.mjs`
- `package.json`
- `script/build-release.sh`

注意：

- 不要随手改 bundle id，否则用户需要重新授权屏幕录制权限。
- 不要随手 `tccutil reset ScreenCapture com.snaplingo.app`，这会清掉用户当前可用的权限。
- 如果用户是从 DMG 安装测试，要确认安装后的 app 也是稳定签名版本。

## 坑 6：权限缺失时不能只提示一次

用户要求：如果权限不对，每次触发截图都需要有提示。

修复点：

- macOS 截图前先调用 `CGPreflightScreenCaptureAccess()`。
- 如果没有权限，每次都调用 `CGRequestScreenCaptureAccess()`。
- 如果系统不再弹原生权限框，后端仍返回中文错误。
- 主窗口通过 `screenshot-error` 每次展示应用层提示。
- 不使用一次性 `AtomicBool` 抑制后续提示。

相关文件：

- `src-tauri/src/infrastructure/system/screenshot/macos.rs`
- `src/App.tsx`

## 坑 7：Release 没有日志会让快捷键问题很难查

快捷键注册失败、触发失败、权限错误大多发生在后端。如果 Release 版不初始化 log plugin，排查会变成猜。

修复点：

- `tauri_plugin_log` 不再只在 debug build 初始化。
- 快捷键注册、快捷键触发、overlay ready、裁剪、窗口恢复都要有日志。

常用日志位置：

```bash
tail -f ~/Library/Logs/com.snaplingo.app/SnapLingo.log
```

## 验证清单

每次改截图链路后至少跑：

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Release 构建后额外检查：

```bash
npm run tauri:build
codesign --verify --deep --strict --verbose=2 target/release/bundle/macos/SnapLingo.app
hdiutil verify target/release/bundle/dmg/SnapLingo_0.1.0_aarch64.dmg
```

手动验收：

- 打开 Release 版 `SnapLingo.app`。
- 按当前配置的截屏快捷键。
- 桌面其它应用不能被隐藏。
- SnapLingo 主窗口和贴图窗口如果本来可见，也不能被隐藏。
- overlay 背景必须是真实桌面截图。
- 框选后进入截图编辑/preview 状态。
- 临时取消屏幕录制权限后，每次触发都能看到权限提示或应用错误提示。

## 快速定位表

| 症状 | 优先检查 |
| --- | --- |
| 按快捷键没反应 | 后端 accelerator 是否为 `CmdOrCtrl+Shift+KeyR`；Release 日志里是否有注册失败 |
| 快捷键触发两次 | 是否只处理 `ShortcutState::Pressed` |
| 截图黑屏或白屏 | 是否先捕获屏幕再 reveal overlay；是否误捕获 overlay 自己 |
| 其它应用或 SnapLingo 主窗口在截图里消失 | 是否在截图前隐藏了 `main`、`pin-*` 或其它窗口 |
| overlay 没背景 | capture window 是否在 session 渲染完成前被 show；`get_capture_session` 是否成功 |
| 选区后没结果 | `render_capture_output` / `output_capture` 是否成功；session 是否被过早 cancel |
| 已授权仍提示权限 | Release 是否稳定签名；bundle id 是否变化；最终签名是否仍是 ad-hoc |
| 权限不对只提示一次 | macOS 权限请求路径是否仍有一次性 guard |
