# Windows 截图与 OCR 实现对照

> 调研日期：2026-10-28。对照 Microsoft PowerToys Text Extractor、ShareX 官方文档和 SnapLingo 当前源码；目的是找 Windows 桌面验收重点，不把同类产品的行为当作 SnapLingo 的既定需求。

## 结论

SnapLingo 的 Windows 功能形态与成熟工具的常见流程一致：全局快捷键打开全屏选区层，用户拖选后执行 OCR/复制或进入结果流程；它还采用会话冻结屏幕像素的架构。已确认一个 Windows 划词翻译问题：临时读取选区会替换系统剪贴板，原剪贴板含图片或富文本时不能完整恢复。除此之外，最需要真实桌面验证的是多显示器混合 DPI 下，覆盖层范围、指针位置、选区矩形和最终裁剪是否完全对齐。

## 同类产品的公开行为

| 产品 | 官方说明的交互 | 对 SnapLingo 的参照 |
| --- | --- | --- |
| Microsoft PowerToys Text Extractor | 默认 `Win+Shift+T` 打开覆盖层，拖动选区；识别后文字复制到剪贴板并关闭，`Esc` 可取消。OCR 语言取决于 Windows 已安装的 OCR 语言包。 | 验证快捷键触发、跨屏覆盖层、拖选、OCR 结果及 Esc 取消；无语言包时须给出可理解结果。 |
| ShareX | 区域捕获可拖矩形，也可点选被检测到的窗口或控件；默认 Quick capture 在松开鼠标后完成。捕获后的任务可配置为 OCR。其 OCR 使用 Windows OCR，可改语言、编辑结果并复制。 | 验证 SnapLingo 的鼠标松开提交语义、窗口/控件候选，以及 OCR 语言与结果处理。ShareX 的流程更可配置，不表示 SnapLingo 必须复制其全部选项。 |

来源： [PowerToys Text Extractor](https://learn.microsoft.com/en-us/windows/powertoys/text-extractor)、[ShareX Region Capture](https://getsharex.com/docs/region-capture)、[ShareX OCR](https://getsharex.com/docs/ocr)。

## SnapLingo 当前实现

- Windows 截图适配器通过 XCap 枚举显示器、捕获冻结快照和区域图像；Windows 单窗口覆盖层以主显示器比例建立统一坐标系。见 [`windows.rs`](../../src-tauri/src/infrastructure/system/screenshot/windows.rs) 和 [`coordinates.rs`](../../src-tauri/src/application/capture/coordinates.rs)。
- Windows 覆盖层在显示时用物理坐标和主显示器比例设置 frame，并按覆盖层所在显示器的 Tauri scale 调整 WebView zoom。混合 DPI 显示器跨越、不同排列方向、负坐标都需要用真机确认，而不是仅检查坐标数学测试。见 [`capture_window/tauri.rs`](../../src-tauri/src/infrastructure/system/capture_window/tauri.rs)。
- Windows 后端有 UI Automation 控件检测、系统 OCR 和 `Ctrl+C` 选中文本获取实现。系统 OCR 依赖 Windows OCR 运行时及可用语言能力。见 [`screenshot/windows.rs`](../../src-tauri/src/infrastructure/system/screenshot/windows.rs)、[`ocr/windows.rs`](../../src-tauri/src/infrastructure/system/ocr/windows.rs) 和 [`selection/windows/shortcut_copy.rs`](../../src-tauri/src/infrastructure/system/selection/windows/shortcut_copy.rs)。
- `ShortcutCopySelectionMethod` 只保存 `arboard::Clipboard::get_text()` 的结果，并在剪贴板序号未再次变化时调用 `set_text()` 恢复。若原剪贴板没有文本格式，保存步骤会失败且不会恢复；若原剪贴板含 HTML/RTF 等富文本与纯文本，恢复也只写纯文本。因此成功的划词翻译会将这些剪贴板内容换成新选区文本/纯文本。见 [`shortcut_copy.rs`](../../src-tauri/src/infrastructure/system/selection/windows/shortcut_copy.rs)。
- CI 在 Windows Server runner 上构建、运行 Rust/前端自动化、安装并启动 NSIS/MSI 包；安装 smoke 脚本不执行截图、拖选、OCR 或多显示器交互。仓库发布指南也把这些列为真实 Windows 10/11 的人工验收项。见 [CI workflow](../../.github/workflows/ci.yml)、[Windows smoke test](../../script/smoke-test-windows.ps1) 和 [RELEASING.md](../RELEASING.md)。

## 判断与验收重点

现有实现覆盖了与 PowerToys/ShareX 相同的核心截图/OCR交互，还增加了冻结帧、标注与后续翻译流程。已确认的行为缺陷是：Windows 划词翻译不能完整保留非纯文本剪贴板数据。多显示器混合 DPI 的坐标、覆盖层和选区则仍属于“未完成桌面验收”，现有源码与数学单测不足以判定有错或无错。Microsoft 的高 DPI 指南说明，桌面程序须正确处理 DPI awareness 与显示器 DPI 变化；因此数学单测通过并不能证明 Tauri/WebView2 窗口在真实混合 DPI 桌面中的行为正确。参见 [High DPI Desktop Application Development on Windows](https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows)。

优先在 Windows 10/11 x64 验收：

1. 单屏 100%、150%、200% 缩放下分别触发截图，确认遮罩覆盖全屏、鼠标按下/拖动/松开可用、选区边缘与预览/输出像素一致，Esc 能退出。
2. 双屏混合 DPI，分别把副屏放在主屏左侧、右侧和上方；在主副屏单独选区及跨屏拖选，检查负坐标、预览、OCR 输入和保存图像是否一致。
3. 从不同显示器触发快捷键，并在屏幕设置改变后重新启动/刷新会话，确认 overlay 不错位或停留在错误显示器。
4. 验证系统 OCR 在语言包已安装、缺失以及用户请求语言不可用时的结果/提示；确认全局快捷键冲突时有日志或可恢复路径。
5. 在剪贴板分别放入截图、富文本和纯文本后执行划词翻译，确认当前实现会把截图替换为选区文本、把富文本降为纯文本；若要保留内容，应实现多格式剪贴板快照/恢复，或改用不改写剪贴板的文本获取途径。

## 边界

PowerToys 文档称其建议用 Snipping Tool 进行截图，所以它只适合作为 OCR 覆盖层快捷交互的对照。ShareX 文档描述的窗口/控件检测和 OCR 工作流是官方产品行为，不是 SnapLingo 必须具备的等价要求。该调研没有 Windows 真机或桌面自动化环境，不能替代上述验收。
