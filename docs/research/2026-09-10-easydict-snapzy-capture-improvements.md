# Easydict / Snapzy 截图实现对比与改进

核查日期：2026-09-10。以 SnapLingo `1ea163d` 及工作区已有的截图修复为基础，读取两项目的第一方源码；没有运行上游应用，也没有进行 Snipaste 的体验或速度对照。

| 项目 | 本次核查的固定版本 |
| --- | --- |
| Easydict | `dev`，[3abcb22c22dc38e29811c710bf2f15ac76d31d4f](https://github.com/tisfeng/Easydict/tree/3abcb22c22dc38e29811c710bf2f15ac76d31d4f) |
| Snapzy | `master`，[fcd08eb14986b0ab17baaa891c8e3f4fb59347b5](https://github.com/duongductrong/Snapzy/tree/fcd08eb14986b0ab17baaa891c8e3f4fb59347b5) |

适合直接借鉴的三项已经实施：窗口候选从一次 WindowServer 查询中取得全部属性；只需要几何信息的操作不再复制冻结图片；OCR 等待移到截图窗口之外的后台任务。Easydict 的局部键盘监听和统一清理也印证了上一轮修复的方向。

## 比较与取舍

| 方面 | 上游实现 | SnapLingo 的处理 |
| --- | --- | --- |
| 截图快捷键 | Easydict 使用本地事件监听，结束时统一移除 [E1][E2] | 上一轮已删除截图编辑键的全局注册，交给聚焦的截图 WebView；保留文本输入自身的编辑行为 |
| 窗口候选 | Snapzy 一次读取窗口字典，再按系统顺序解析属性；标题可缺失 [S1] | 本轮采用，替换逐窗口、逐属性的重复系统查询 |
| 冻结底图 | Snapzy 的快照保存几何和不可变 `CGImage`，传递快照不复制整张图片 [S2] | 保留同帧冻结语义，本轮消除元数据路径上的 PNG 深拷贝 |
| OCR 等待 | Easydict 把图片交给浮窗或后台查询控制器；Snapzy 固定像素后立即关闭选区，再异步 OCR [E2][E3][S3] | 本轮采用：后端持有所选图片和请求身份，关闭截图窗口后继续识别与交付结果 |
| 多屏与缩放 | Snapzy 并发取得各屏图像，并用返回图片的真实尺寸校准比例 [S4] | 作为后续真实多屏验收和性能优化依据；本轮没有切换抓屏后端 |
| OCR 图像适配 | Snapzy 将可能来自 IOSurface 的图像绘制为标准 sRGB 位图后送入 Vision [S5] | 当前入口是编码图片的 `NSData`，没有同类故障证据；不额外增加一次全图转换 |

Easydict 的监听器会消费匹配到的所有键盘事件 [E1]，因此只借鉴局部作用域和清理责任。SnapLingo 的输入框、文本标注与输入法组合输入仍应自行处理按键。

Easydict 在创建覆盖层时抓取背景，选区确认后又延迟 0.1 秒抓取结果 [E2][E4]。动态页面或临时弹窗在两次抓取之间可能变化。SnapLingo 已经保证背景、预览和最终输出使用同一会话像素，应继续保留；本轮也没有照搬 Snapzy 的整套原生窗口或录屏架构。

## 已实施：一次读取窗口列表

原路径调用 `xcap::Window::all()` 后，对每个窗口读取最小化状态、标题、应用名、位置和尺寸。实际依赖 `xcap 0.9.6` 的 macOS 实现中，这些 getter 会经由 `get_window_cf_dictionary` 再次调用 `CGWindowListCopyWindowInfo`；最小化检查还涉及最大化和显示器查询。它们不是读取已缓存字段。

参考 Snapzy 的窗口快照思路 [S1]，新增 [macos_window_list.rs](/Users/gamilian/work/code/snaplingo/src-tauri/src/infrastructure/system/screenshot/macos_window_list.rs)，一次取得屏幕上的窗口，再从每条字典解析 ID、进程、标题、应用名和几何信息。窗口候选与控件命中查询共用这个读取入口，保留系统前后顺序和原有自家截图窗口、贴图窗口过滤规则。

解析器接受缺失标题的可见窗口，并过滤隐藏、透明、不共享或缺少有效身份／几何字段的记录。它没有引入常驻监听或后台轮询。新增的三个字典样例测试覆盖无标题窗口、负坐标和无效记录。

## 已实施：几何查询不复制整屏 PNG

在 [session.rs](/Users/gamilian/work/code/snaplingo/src-tauri/src/application/capture/session.rs) 中，原来构造 `layout_snapshots` 的方式是先 clone 整个快照、再清空 PNG；既发生过像素复制，清空后也仍可能保留分配容量。光标位置、控件命中和逻辑坐标转换还会通过 `get_session` 复制含有图片的整个会话。

现在直接从借用的快照复制 ID 和几何字段，创建空的图片向量。这三个查询只取布局副本，并在调用系统接口或异步等待前释放会话锁。需要最终输出的路径仍使用原来的冻结图片。

这与 Snapzy 复用不可变图像的原则一致 [S2]，但实现按现有 Rust 模型做了局部修改，没有重写整个图片存储结构。

## 定向测量与回归

窗口枚举探针调用生产候选函数，每次运行取 11 个样本；两次运行均在本机完成，没有抓取桌面图片，也不输出窗口标题。

| 版本 | 中位耗时 | 最小值 | 最大值 | 候选数 |
| --- | ---: | ---: | ---: | ---: |
| 修改前 | 14.391 ms | 13.714 ms | 148.678 ms | 2 |
| 修改后 | 0.152 ms | 0.137 ms | 86.209 ms | 3 |

这是局部函数的两次观测，不是整次截图的启动延迟。候选数量不同，未完成逐窗口 ID 对照；不能据此认定新增项具体是哪种弹窗，也不能把表格换算为整个截图流程的提速倍数。系统查询仍有明显抖动。

内存回归使用固定 4 MiB 不透明图片载荷，通过仅测试构建启用的线程局部分配计数验证生产会话方法；不读取桌面或用户数据库。

| 操作 | 修改前分配字节 | 修改后分配字节 |
| --- | ---: | ---: |
| 创建会话，扣除原始 4 MiB 载荷 | 4,195,929 | 1,625 |
| 光标位置 | 4,194,555 | 111 |
| 控件命中 | 4,194,587 | 143 |
| 坐标转换 | 4,194,555 | 111 |

回归测试先在原实现失败，修改后通过。测试断言元数据分配低于 64 KiB，不绑定表格中的精确字节数。这些数值衡量单次调用的 Rust 分配量，不等同于应用常驻内存或峰值内存。

两个探针可重跑：

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib benchmark_capture_window_candidates -- --ignored --nocapture
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib geometry_queries_do_not_duplicate_frozen_screen_pixels -- --nocapture
```

窗口计时需要真实 macOS WindowServer，默认标记为 ignored；内存回归进入常规测试集合。

| 本轮检查 | 结果 |
| --- | --- |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml --quiet` | 557 项库测试、13 项集成测试通过；1 个手动计时探针默认跳过 |
| `npm test` | 123 个文件、912 项测试通过 |
| `npm run build` | TypeScript 检查与 Vite 生产构建通过 |
| `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets` | 本机 macOS 所有目标检查通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `git diff --check` | 通过 |

两项性能修改之后，又完成了 OCR 后台交接及对应前后端修改。上述表格记录最终工作区的检查结果；[上一轮核查](/Users/gamilian/work/code/snaplingo/docs/research/2026-09-09-capture-interaction-and-shortcut-audit.md) 保留当时的历史结果。

## 已实施：将 OCR 等待移出截图交互

修复前，截图 runtime 等待 `runCaptureOcr`，再打开结果窗口或复制文字，最后结束截图会话。静默 OCR 的成功提示还在截图层停留 550 ms，截图层的存活时间包含识别等待。

Easydict 在截图完成回调中移交图片，随即统一清理覆盖层和监听；识别在结果窗口或后台查询控制器中继续 [E2][E3]。Snapzy 的 OCR 路径更直接：先固定选区像素，调用 `cancelSelection()`，然后启动异步识别与进度反馈 [S3]。可借鉴的用户体验是“选区确认后恢复原应用操作，识别结果稍后到达”。

原流程不能仅把 `finishSession` 移到 OCR 调用前：macOS 会销毁不活跃的截图 WebView，[销毁回调](/Users/gamilian/work/code/snaplingo/src-tauri/src/infrastructure/system/capture_window/tauri.rs:329) 又会取消它绑定的会话，而异步编排仍由该 WebView 承载。现在由 [后端 runtime](/Users/gamilian/work/code/snaplingo/src-tauri/src/application/capture/runtime.rs:367) 完成两阶段交接：

1. `prepare_capture_ocr` 取得独立的选区图片、必要的预览图和设置快照；准备失败时截图界面仍可显示错误。原有同帧像素和 OCR 边缘留白保持不变。
2. 前端结束当前选区流程，调用 `complete_capture_ocr` 确认交接。后端隐藏窗口、恢复应用、取消截图会话，随后启动自己拥有的 OCR 任务。交接前取消会丢弃准备数据；重复提交不会执行两次识别。

任务完成后按原意图复制文字、显示 OCR 图文结果或打开翻译窗口。静默 OCR 的等待与成功提示改到菜单栏，不再占用截图层；隐藏静默状态的设置仍有效。识别失败通过独立错误提示反馈；没有识别到文字时不清空剪贴板。

开始新截图和应用退出都会使旧 OCR 任务失效。结果窗口使用预留的请求 ID，后打开的手动翻译请求不会被旧 OCR 覆盖。原生同步识别调用不保证立即中断，但其过期结果不会再交付。

macOS 还在准备选区时记录剪贴板 `changeCount`，交付静默 OCR 时再次检查。如果用户在其他应用执行过新的复制，则丢弃这次旧的后台写入。这项系统版本检查当前只在 macOS 接入；其他平台没有在本轮增加对应系统实现。

静默复制继续遵循保留格式和去除中文空格设置；后台和前端使用同一组固定文本样例回归换行、URL、中文空格及日／韩文分隔，防止迁移后文字处理发生变化。

已通过的回归包括：识别被人为暂停时会话已经关闭、迟到的窗口销毁清理不影响已交接任务、交接前取消、新截图使旧任务失效、重复提交、错误和空结果、隐藏提示、新复制优先，以及新翻译窗口请求优先。

另外运行了真实隐藏 WKWebView 的原生探针：从 JavaScript 调用 Rust 命令，销毁发起调用的窗口后，再启动并完成后台任务。结果为：

```text
PASS: native invocation continued after WKWebView destruction; owned job completed
```

该探针验证 Tauri 原生调用与窗口生命周期的边界，不运行真实 OCR、不截取桌面、不修改剪贴板。临时源码已归档到 [本机探针](/Users/gamilian/work/code/snaplingo/target/capture-audit/capture_ocr_handoff_probe.rs)，不保留在发布的 Cargo examples 中。生产任务行为由上述回归测试覆盖；这不替代真实多屏、全屏 Space 和实际识别速度的人工验收。

## 翻译与后续核查

翻译侧应基于现有能力继续改进，不能沿用旧对比文档中的缺失清单：

- [OCR 结果模型](/Users/gamilian/work/code/snaplingo/src-tauri/src/domain/ocr.rs:12) 已有文本行、置信度、边界框和可选语言字段；[OCR 协调器](/Users/gamilian/work/code/snaplingo/src-tauri/src/application/providers/ocr/coordinator.rs:210) 会在引擎没有提供语言时从文本补充推断。macOS Vision 已有空结果／低置信度时的受控恢复。
- [翻译结果模型](/Users/gamilian/work/code/snaplingo/src-tauri/src/domain/translation.rs:65) 已有错误代码、信息和可重试标记，不需要再引入一套同义结构。
- [结果窗口 OCR](/Users/gamilian/work/code/snaplingo/src/application/result-window/runtime.ts:449) 已用 generation 判断请求是否仍有效。Easydict 同样有结果代次和任务取消 [E5]；本次截图后台任务也增加了相应的代次和结果请求保护。

Snapzy 的 sRGB 标准化、更多恢复分支以及多屏并发是后续有针对性的参考 [S4][S5]。应先用真实的色彩空间、多缩放显示器、小字和竖排样例确定问题，再选择对应改动；本轮没有取得这些场景的准确率或端到端时延对照。

以上改动保留在工作区，尚未替换 `/Applications/SnapLingo.app`；当前安装版不会自动获得这些修复。

## 第一方源码索引

- [E1 — Easydict：局部事件监听与移除](https://github.com/tisfeng/Easydict/blob/3abcb22c22dc38e29811c710bf2f15ac76d31d4f/Easydict/Swift/Feature/Screenshot/Screenshot/Screenshot%2BEventMonitor.swift#L16-L52)
- [E2 — Easydict：统一结束截图与选区后延迟抓屏](https://github.com/tisfeng/Easydict/blob/3abcb22c22dc38e29811c710bf2f15ac76d31d4f/Easydict/Swift/Feature/Screenshot/Screenshot/Screenshot.swift#L65-L119)
- [E3 — Easydict：截图翻译、静默 OCR 与 OCR 窗口](https://github.com/tisfeng/Easydict/blob/3abcb22c22dc38e29811c710bf2f15ac76d31d4f/Easydict/objc/ViewController/Window/WindowManager/EZWindowManager.m#L861-L904)
- [E4 — Easydict：创建覆盖层时抓取背景](https://github.com/tisfeng/Easydict/blob/3abcb22c22dc38e29811c710bf2f15ac76d31d4f/Easydict/Swift/Feature/Screenshot/Screenshot/ScreenshotOverlayView.swift#L13-L20)
- [E5 — Easydict：结果代次与查询取消](https://github.com/tisfeng/Easydict/blob/3abcb22c22dc38e29811c710bf2f15ac76d31d4f/Easydict/Swift/Service/Model/QueryService.swift#L188-L278)
- [S1 — Snapzy：一次读取窗口字典、按顺序生成候选](https://github.com/duongductrong/Snapzy/blob/fcd08eb14986b0ab17baaa891c8e3f4fb59347b5/Snapzy/Services/Capture/WindowSelectionQueryService.swift#L52-L153)
- [S2 — Snapzy：冻结图片及几何快照](https://github.com/duongductrong/Snapzy/blob/fcd08eb14986b0ab17baaa891c8e3f4fb59347b5/Snapzy/Services/Capture/FrozenAreaCaptureSession.swift#L12-L108)
- [S3 — Snapzy：固定像素后关闭选区，再执行 OCR](https://github.com/duongductrong/Snapzy/blob/fcd08eb14986b0ab17baaa891c8e3f4fb59347b5/Snapzy/Features/Capture/CaptureViewModel.swift#L2405-L2473)
- [S4 — Snapzy：多屏并发抓取与真实像素比例](https://github.com/duongductrong/Snapzy/blob/fcd08eb14986b0ab17baaa891c8e3f4fb59347b5/Snapzy/Services/Capture/ScreenCaptureManager.swift#L340-L411)
- [S5 — Snapzy：Vision 输入图像标准化](https://github.com/duongductrong/Snapzy/blob/fcd08eb14986b0ab17baaa891c8e3f4fb59347b5/Snapzy/Services/Media/OCRService.swift#L54-L115)
