# SnapLingo 与 AirSnip 功能差距对照

本文记录补齐功能前的比较基线；新增功能的当前行为见 [功能文档](../FEATURES.md)。

调研日期：2026-11-01（Asia/Shanghai）。目标是找出 SnapLingo 当前相对 AirSnip 缺少的用户功能，区分完整缺失、已有但能力较窄，以及 AirSnip 尚未发布的实现。

## 对照基线与方法

- **SnapLingo**：本地 master，提交 [2a101ae0](https://github.com/gamilian/snaplingo/tree/2a101ae07872c7d4c9171c29454323742fa73365)，开始调研时工作区干净。
- **AirSnip 已发布版本**：[v0.6.0](https://github.com/bd-dxg/airsnip/releases/tag/v0.6.0)，发布于 2026-10-31 19:36:59 中国时间，实际源码提交 [13dfee6a](https://github.com/bd-dxg/airsnip/tree/13dfee6a8a0249187a0f215c8d06d462032c176e)。`8c49d730` 是 annotated tag 对象，2026-11-02 复核纠正。
- **AirSnip 当前 main**：[f3e0718f](https://github.com/bd-dxg/airsnip/tree/f3e0718f486845f42ba0e556a9c95b68a733facb)，提交于 2026-11-01 17:37:48 中国时间。相对 v0.6.0 新增截图界面的 DPI/分辨率缩放及手动缩放设置；下表对此单独标记。
- 核对 README、实际工具入口、对象模型、输入处理和导出渲染。PRD 的规划不算已实现功能。此次为静态源码审查，没有运行 AirSnip 或做同机性能测试。

## 已发布功能中的主要差距

| 功能 | AirSnip 的实际能力 | SnapLingo 当前情况 | 判定 |
| --- | --- | --- | --- |
| **滚动长截图** | 以选区作为视口，用户手动滚动，程序抓取变化帧并拼接；识别固定页眉/页脚；显示实时缩略预览；可撤销最后一段，Enter 完成。[A1]、[A2]、[A3]、[A18] | 捕获模式和快捷键中没有长截图；当前截图工作区没有多帧采集、拼接和长图预览流程。[S1]、[S2] | **完整缺失，最大功能差距**。不能将跨显示器合成的全屏截图等同于滚动长截图。 |
| **区域模糊** | 拖出区域后模糊，可调强度，支持圆角；实现使用多次盒式模糊近似高斯。[A4]、[A5] | 实际工具与命令模型提供笔刷式马赛克，渲染采用块状像素化；没有独立 Blur 命令或工具。[S3]、[S4]、[S5] | **完整缺失**。SnapLingo README 虽写了 blur，当前实现仍只有马赛克。 |
| **聚光灯** | 将导出选区整体暗化，再将指定区域恢复原亮度；可叠加多块，效果写入导出图片。[A4]、[A6] | 有选区外遮罩设置，但标注模型及导出渲染没有聚光对象。[S3]、[S5]、[S6] | **完整缺失**。截图时的选择遮罩与可导出的聚光标注是不同能力。 |
| **曲线箭头与控制点编辑** | 三次贝塞尔箭头，两个曲率控制柄；双击恢复直线；可分别拖起点/终点。[A4]、[A7] | 箭头只有 start/end，支持直线绘制和整体移动；没有曲率字段及箭头端点编辑动作。[S3]、[S7] | **曲线与专用控制柄缺失**，普通直线箭头已有。 |
| **圆角矩形与区域圆角** | 矩形、高亮、模糊和聚光区域都有圆角半径，选中后可拖控制柄调整。[A4]、[A7]、[A8] | 矩形可描边、填充和缩放，但命令模型没有圆角参数，预览直接绘制直角矩形。[S3]、[S4] | **圆角能力缺失**，普通矩形和椭圆已有。 |
| **字体选择、粗体与斜体** | 枚举系统字体；字体、粗斜体、字号可作用于正在输入或已选中的文本对象。[A4]、[A9]、[A10] | 文本可编辑，可调颜色与字号；预览使用固定字体栈，Rust 导出使用内置 Noto Sans CJK 常规字体，没有字体族、粗体或斜体字段。[S3]、[S4]、[S15] | **部分具备**，缺少字体与字形选择。 |
| **区域荧光高亮与正片叠底** | 可拖出矩形高亮区域，使用正片叠底混色，保留底图深色文字；区域可缩放及加圆角。[A4]、[A5] | 已有笔刷式 highlight，H 可启用，使用普通透明度混合；没有矩形区域高亮与正片叠底渲染。[S3]、[S4]、[S5]、[S8] | **部分具备**，不应将荧光笔整体列为缺失。 |
| **更完整的对象缩放编辑** | 除矩形/区域外，文本可用包围框缩放，箭头有端点和曲率控制柄。[A7] | 已支持选中对象、移动、删除、修改样式及双击编辑文本；八向缩放实现只处理矩形和椭圆。[S7]、[S9]、[S16] | **部分具备**，剩余差距是文本拖拽缩放、箭头的专用控制柄。 |
| **标注工具快捷键自定义** | 六种工具的切换键及长截图键可在设置中改键，并检测重复配置。[A11]、[A12] | 全局截图/OCR/翻译/贴图快捷键可改；编辑器的 A/O/P/H/M/T/E 等工具键写死在代码中。[S2]、[S8] | **工具改键缺失**，全局快捷键配置已有。AirSnip 的复制、保存等快捷键仍有固定处理，不宜把 README 的“全部快捷键”解释成所有动作均可改键。 |
| **贴图初始位置与边框开关** | 截图贴图可选择原选区位置或屏幕中央；贴图边框可切换，并持久化为后续贴图的默认值。[A11]、[A13] | 截图设置中的贴图偏好为透明度与阴影；没有原位置/居中选择或独立边框开关。[S6]、[S10] | **两项体验设置缺失**，基本贴图及批量分组管理已有。 |

## main 已实现但尚未发布的差距

| 功能 | AirSnip main | SnapLingo 当前情况 |
| --- | --- | --- |
| **截图界面缩放设置** | 自动取显示器 DPI 与分辨率补偿的较大倍率；也可手动选择 100% / 125% / 150% / 175% / 200%。只放大工具栏、手柄、尺寸提示等控件，不改变标注或导出图。[A14] | 有显示器 scale_factor、逻辑/物理坐标和画布像素比处理，不能称为“没有高 DPI 支持”；缺少独立的截图控件缩放设置。[S1]、[S4]、[S6] |

该功能来自 main 的提交 f3e0718f，v0.6.0 的配置中没有 UiScale。不能把它算作 v0.6.0 的已发布功能。

## 已具备的功能与对照边界

- **双击复制**：SnapLingo 在预览状态、未激活标注工具时，双击可复制完成。[S11]
- **二次编辑基础能力**：对象选中、移动、删除、颜色/粗细/填充修改，以及文本再编辑均已实现；矩形和椭圆有缩放手柄。[S7]、[S9]、[S16]
- **Shift 约束**：正方形/正圆、水平/垂直/斜向直线、轴向移动和缩放保持比例已有对应处理。[S7]、[S8]
- **贴图管理**：SnapLingo 已支持旋转、翻转、灰度、反色、缩略模式、分组、组隐藏与组销毁；不应将 AirSnip 的“关闭全部”直接推断为 SnapLingo 完全没有批量管理。[S10]、[S12]
- **OCR/翻译**：AirSnip 的 src/ocr/mod.rs 只有“二期占位”说明，没有实际 OCR 或翻译流程。录屏/GIF、云同步、跨平台和历史管理在其当前 PRD 中列为非目标。[A15]、[A16] SnapLingo 已有 OCR、多引擎翻译、历史/收藏及跨平台实现；平台正式发包范围仍以本项目 README 为准。[S13]

## 交付方式与性能优势

AirSnip 官方提供 Windows 单文件绿色 ZIP，README 声称 exe 1.06 MB、待机 Private Working Set 1.2 MB、2560×1440 下热键到可用画面 124 ms。[A17] SnapLingo 当前官方 Windows 发包为 NSIS EXE 与 MSI，没有对应的绿色 ZIP 发布形式。[S14]

“免安装绿色包”属于可明确对照的交付差距。体积、常驻占用和延迟属于性能指标：本次没有按同一机器、同一分辨率和相同测量口径复测，不能直接给出 SnapLingo 慢多少或占用高多少的结论。

## 建议优先级

以下是基于截图、OCR 和翻译用途的产品判断，不是已承诺的实现计划：

1. **先补模糊与聚光灯**：直接改善截图标注，范围较集中；模糊还可消除 README 与实现不一致的问题。
2. **将长截图作为独立重点功能**：它是最大缺口，并能扩展长段文字 OCR/翻译场景。第一版可从用户手动滚动、自动拼接、预览和撤销最后一段开始；工作量大于新增单个标注工具。
3. **再完善曲线箭头、圆角与文字样式**：复用现有对象选中、撤销和样式修改能力，补齐必要的模型与控制柄。
4. **随后补区域高亮/正片叠底、文本缩放、工具改键和贴图偏好**。截图控件缩放可结合高分屏使用反馈安排；绿色 Windows 包与性能应单独验证。

## 证据来源

AirSnip 源码链接固定到 main f3e0718f；除截图界面缩放外，上述主要功能也已在 v0.6.0 存在。SnapLingo 源码固定到本次本地提交 2a101ae0。

[A1]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/annotate.rs#L190-L203
[A2]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/engine.rs#L80-L104
[A3]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/session.rs#L142-L213
[A4]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/editor/mod.rs#L68-L155
[A5]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/render/region.rs#L31-L122
[A6]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/render/bake.rs#L11-L32
[A7]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/objedit/drag.rs#L30-L85
[A8]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/editor/corner.rs#L13-L55
[A9]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/font/mod.rs#L18-L95
[A10]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/render/fonts.rs#L14-L39
[A11]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/mod.rs#L37-L113
[A12]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/panel/interact.rs#L190-L225
[A13]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/mod.rs#L251-L259
[A14]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/mod.rs#L140-L191
[A15]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/ocr/mod.rs#L1-L4
[A16]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/screenshot-tool-PRD.md#L10-L23
[A17]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/README.md#L12-L26
[S1]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/domain/capture.ts#L19-L70
[S2]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src-tauri/src/domain/hotkey_config.rs#L8-L87
[S3]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/domain/capture.ts#L102-L178
[S4]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/views/CaptureWorkspace/captureAnnotationCanvas.tsx#L194-L293
[S5]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src-tauri/src/application/capture/image_composer.rs#L649-L705
[S6]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src-tauri/src/domain/config.rs#L5-L107
[S7]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/application/capture-workspace/annotationGeometry.ts#L95-L180
[S8]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/application/capture-workspace/annotationStyle.ts#L11-L243
[S9]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/application/capture-workspace/captureEditorRuntime.ts#L97-L179
[S10]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/views/PinnedImageWindow/index.tsx#L830-L875
[S11]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/application/capture-workspace/runtime.ts#L1289-L1323
[S12]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/application/pinned-image/model.ts#L176-L342
[S13]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/README.md#L5-L22
[S14]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/docs/RELEASING.md#L7-L12

[A18]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/present.rs#L22-L30
[S15]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src-tauri/src/application/capture/image_composer.rs#L9-L77
[S16]: https://github.com/gamilian/snaplingo/blob/2a101ae07872c7d4c9171c29454323742fa73365/src/application/capture-workspace/captureEditorRuntime.ts#L650-L691
