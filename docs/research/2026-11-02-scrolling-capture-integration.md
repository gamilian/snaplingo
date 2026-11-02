# 手动滚动长截图的最小接入方案

调查日期：2026-11-02（Asia/Shanghai）。Snaplingo 基准 HEAD：`2a101ae07872c7d4c9171c29454323742fa73365`；本报告读取的是正在实现标注功能的工作区，因此本地行号可能继续变化。AirSnip 参考：`f3e0718f486845f42ba0e556a9c95b68a733facb`。只核查源码，未运行桌面或跨平台验收。

## 结论

现有抓屏、坐标、PNG 合成和输出足以支撑手动滚动长截图。需要增加实时帧入口、拼接状态，以及不会阻挡滚动目标的小型控制窗口。普通 Capture Session 必须继续保持冻结：重复 `hydrate` 或 `freeze` 不会获得滚动后的画面。最小实时入口应直接调用现有 `CaptureSessionSource.capture_monitor_snapshot(monitor_id)`，使用原会话的坐标元数据裁剪，再把新增条带交给纯 Rust 拼接器。[冻结检查](../../src-tauri/src/application/capture/session.rs#L453) · [抓屏接口](../../src-tauri/src/application/capture/source.rs#L21) · [图像合成](../../src-tauri/src/application/capture/render.rs#L357)

## 已有接口与可复用部分

| 部分 | 现状及接入方式 | 证据 |
| --- | --- | --- |
| 实时抓屏 | 三个平台已提供按 monitor ID 抓取新 PNG 的接口；由会话或 runtime 包装调用，不反复创建整个截图会话 | [Source](../../src-tauri/src/application/capture/source.rs#L21)、[macOS](../../src-tauri/src/infrastructure/system/screenshot/macos.rs#L689)、[Windows](../../src-tauri/src/infrastructure/system/screenshot/windows.rs#L41)、[Linux](../../src-tauri/src/infrastructure/system/screenshot/linux.rs#L20) |
| 冻结布局 | Session 存储 `layout_snapshots`、`desktop_scale`、屏幕快照；从中建立固定滚动视口 | [CaptureSession](../../src-tauri/src/application/capture/session.rs#L20) |
| 逻辑/物理转换 | 原逻辑选区与 monitor 相交，扣除屏幕逻辑原点并乘该会话的 scale；现有合成代码还处理小数缩放边缘夹取 | [合成计划](../../src-tauri/src/application/capture/render.rs#L357) |
| PNG 裁剪 | `CaptureImageComposer.crop_png` 接收图内物理坐标；负坐标和越界会报错 | [crop_png](../../src-tauri/src/application/capture/image_composer.rs#L118) |
| 复制/保存/贴图 | 完成时调用已有 `CaptureOutput.copy_png`、`save_image`，需要贴图时使用已有 `CapturePinOutput.pin_png` | [输出](../../src-tauri/src/application/capture/output.rs#L95)、[贴图 port](../../src-tauri/src/application/capture/runtime_host.rs#L13) |
| 截图窗口生命周期 | Runtime 已按 session ID 记录 presentation ownership，取消会恢复/移除会话并释放窗口 | [取消流程](../../src-tauri/src/application/capture/runtime.rs#L270)、[按 ID 结束 presentation](../../src-tauri/src/application/capture/runtime.rs#L592) |
| 组装 | 现有 composition 创建并共享 `CaptureSessions`、composer、output、runtime；无需再选一套 OS 抓屏实现 | [组装](../../src-tauri/src/composition/capture_runtime.rs#L29) |

## 建议的最小接口

以下是接口建议，不是已经存在的实现。可以由现有 `CaptureSessionRuntime` 持有一个滚动任务，避免建立可配置插件体系。

```rust
// 启动时从原会话计算，只保存当前长截图实际需要的屏幕与图内矩形。
struct ScrollingViewport {
    monitor_id: String,
    crop_rect: PhysicalRect, // 相对该 monitor PNG，不能使用桌面绝对 x/y
    expected_frame_size: (u32, u32),
}

async fn begin_scrolling_capture(session_id, logical_rect) -> ScrollCaptureView;
async fn capture_scrolling_frame(scroll_id) -> ScrollCaptureView;
fn undo_scrolling_segment(scroll_id) -> ScrollCaptureView;
async fn finish_scrolling_capture(scroll_id, output_action) -> ();
async fn cancel_scrolling_capture(scroll_id) -> ();
```

`ScrollCaptureView` 只需状态、尺寸、能否撤销、失败提示，以及有上限的缩略图。整张长图留在 Rust 内存中，完成时编码；每拍不通过 IPC 返回全图或所有 monitor base64。现有 Session 的 view 已会编码整屏 PNG，重复使用该 view 会产生不必要的传输。[Session view](../../src-tauri/src/application/capture/session.rs#L650)

实时帧入口的步骤：读取原会话中的选中屏幕元数据 → 按 ID 抓新 PNG → 校验屏幕尺寸/布局未变 → 使用原布局计算好的 `crop_rect` 裁剪 → 解码成固定尺寸 RGBA → 拼接。首版可要求视口完全在一个显示器内：现有 Windows/Linux 区域抓屏自身也是单屏契约；跨屏长截图没有必要混入本次最小实现。[区域抓屏限制](../../src-tauri/src/infrastructure/system/screenshot/xcap_common.rs#L230)

如果使用按 monitor 抓图后的元数据，Windows 必须沿用原会话中归一化后的 `logical_bounds`、`scale_factor`，只替换 PNG。Windows 的会话使用主屏 scale 统一整个桌面；对“仅副屏”的新快照重新调用 `normalize` 会把副屏当作主屏，改变选区坐标解释。[Windows policy](../../src-tauri/src/infrastructure/system/screenshot/windows.rs#L37) · [normalize](../../src-tauri/src/application/capture/coordinates.rs#L21)

## 必须处理的生命周期与坐标陷阱

1. **隐藏覆盖层前先进入 scrolling 状态。** 当前 `selecting` 和 `preview` 失焦会取消会话；开始滚动必须让 blur 不走普通取消，并停止原截图视图的自动 reveal。完成/取消时恢复正常处理。[失焦判断](../../src/application/capture-workspace/captureActions.ts#L453) · [自动 reveal](../../src/application/capture-workspace/captureWindowVisibility.ts#L19)

2. **开始滚动不能调用现有 restore-and-finish 操作。** `restore_capture_snapshot_windows_for_session` 还会结束 presentation 并销毁 inactive capture window；Windows/Linux 可能直接销毁仍作为 UI owner 的 WebView。开始时只 hide，滚动任务仍持有 session，统一在完成/取消时清理。[restore 流程](../../src-tauri/src/application/capture/runtime.rs#L305) · [窗口销毁](../../src-tauri/src/infrastructure/system/capture_window/tauri.rs#L200)

3. **macOS 隐藏窗口不等于恢复原应用焦点。** `hide_capture_window_for_current_space` 只 `orderOut` 并恢复光标；原应用恢复另在 `end_capture_presentation` / `restore_capture_window_activation`。滚动开始应隐藏覆盖层并恢复源应用，但保留该会话的 presentation ownership；控制窗应不抢焦点。[hide](../../src-tauri/src/infrastructure/system/capture_window/macos.rs#L223) · [restore activation](../../src-tauri/src/infrastructure/system/capture_window/macos.rs#L83) · [结束](../../src-tauri/src/infrastructure/system/capture_window/macos.rs#L46)

4. **现有隐藏集合没有覆盖新控制窗。** backend 只隐藏 `capture`，恢复集合为空；不能以为添加 preview/control label 后会自动从抓屏排除。优先把两个窗口放到视口外；两侧空间不足时隐藏缩略预览，仅保留不覆盖视口的控制入口。滚动框提示可放在视口边界外，不能烘焙进每帧。[隐藏/恢复集合](../../src-tauri/src/infrastructure/system/capture_window/backend.rs#L41) · [AirSnip 预览布局](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/preview.rs#L1)

5. **区域抓屏的 macOS 坐标契约有疑点。** `ScreenRegion` 声明为物理屏幕像素，但当前 macOS adapter 直接将数值传入 Core Graphics `CGRect`。它尚未在普通冻结合成路径使用；把它接入实时长截图前须验证 Retina/副屏坐标。按 monitor 抓图后在 PNG 内裁剪能避开此风险。[域契约](../../src-tauri/src/domain/capture.rs#L70) · [macOS 实现](../../src-tauri/src/infrastructure/system/screenshot/macos.rs#L772)

6. **桌面绝对物理坐标不能直接传给 crop_png。** `CaptureSessions.logical_rect_to_physical` 加上 `physical_bounds.x/y` 返回桌面坐标，副屏可能为负。图内裁剪必须扣除该屏 physical origin，或直接采用 render 中相对 logical origin 的转换，并夹取 PNG 宽高。[绝对物理转换](../../src-tauri/src/application/capture/session.rs#L863) · [相对转换](../../src-tauri/src/application/capture/render.rs#L392) · [裁剪验证](../../src-tauri/src/application/capture/image_composer.rs#L118)

7. **只采新帧，不改变冻结会话。** Frame worker 不覆盖普通 Session 的 snapshots；这样失败或取消可以回到原截图。每次 `await` 后检查 scroll ID/取消状态，旧 IPC、旧定时器不能影响新任务。现有 presentation 释放已经有按 ID 防重复机制，应保留。[Session snapshots](../../src-tauri/src/application/capture/session.rs#L20) · [防重复释放](../../src-tauri/src/application/capture/runtime.rs#L592)

8. **采帧串行执行。** Windows monitor capture 已使用 `spawn_blocking`；每拍必须等待上一拍抓屏/匹配结束，避免 worker 堆积。把采集、匹配、PNG 编码留在后台；前端只接收增量状态与限幅缩略图。不要把 `capture_monitor_snapshot` 的同步 PNG 工作放入 UI 主线程。[Windows 后台抓屏](../../src-tauri/src/infrastructure/system/screenshot/windows.rs#L47)

## 需要实现的拼接行为

AirSnip 的真实实现是纵向手动滚动：定时取帧、识别重叠、保留固定页眉页脚、支持上下两个方向、撤销最后一段、实时缩略图，用户主动完成；静帧不会自动结束。[拼接入口](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/session.rs#L115) · [匹配](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/stitch.rs#L71) · [固定区与撤销](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/engine.rs#L77)

建议独立实现的行为：

- 用完整重叠区域的一致性检验纵向位移，重复行/空白区域不应仅凭一个局部条带决定拼接位置；不匹配时保留已完成内容并提示放慢滚动。[参考行为](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/stitch.rs#L71)
- 初始帧需缓冲到出现可判断的位移后才确定固定页眉/页脚，避免把静帧整张错误认作固定区域。[初帧处理](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/engine.rs#L80)
- 一次拼入条带是一次撤销，记录方向和新增行数；上下方向要分别追加/前置，不能每次重新复制全长图片。[新增条带](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/engine.rs#L134) · [撤销](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/engine.rs#L156)
- 缩略图按最终目标尺寸抽样，不先编码整张长图；高度/像素总数都要有上限，完成时考虑全图编码的额外一份内存。AirSnip 当前行缓冲仍在内存内，PRD 中的超大图临时文件不能当作已经具备的参考能力。[thumbnail](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/engine.rs#L188) · [内存行缓冲](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/rows.rs#L10)
- 完成前补抓最后一帧，避免滚动后立刻结束遗漏最后一屏；失败时允许保存已经拼好的部分。[收尾](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/window.rs#L215)

对齐 AirSnip 无需另造长图编辑器：其 `longshot::run` 完成后实际直接复制并保存 PNG；PRD 的“长截图结果再编辑”不等于当前实现。[真实输出路径](https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/mod.rs#L22)

## 核心验收

纯逻辑测试覆盖：下滚/上滚、固定页眉页脚、重复文本/空白、无位移帧、匹配失败、段撤销、高度及像素上限、限幅缩略图。Runtime 测试覆盖：使用新 frame source 而非缓存；隐藏前状态切换；完成与取消各释放一次；旧 worker 不覆盖新任务；副屏原 scale 元数据保留；小数缩放、负桌面坐标转图内坐标正确。

桌面验收仍需实机：macOS Retina 与全屏 Space、Windows 125%/150% 与负坐标副屏、Linux X11/Wayland；确认页面能持续滚动，控制/预览不出现在结果中，完成/取消恢复原应用。当前只读研究没有验证这些运行结果。
