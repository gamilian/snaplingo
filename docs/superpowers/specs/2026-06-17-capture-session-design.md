# Capture Session Screenshot Architecture Design

> 日期：2026-06-17
> 类型：架构设计
> 状态：已按用户确认方向整理

## 1. 背景

SnapLingo 当前截图链路接近“前端进入选区 → 后端截区域 → 前端编辑”。这条链路能跑通基础截图，但不适合 Snipaste-like 体验：

- 选区时应看到冻结的真实屏幕，而不是黑色遮罩
- 选区完成后不应再次截取真实屏幕，否则屏幕变化会影响结果
- 多显示器、Retina/HiDPI、负坐标显示器需要统一坐标模型
- OCR 应使用原图裁剪，复制/保存/贴图应使用合成图
- 窗口吸附、放大镜、取色、贴图都需要稳定的截图会话上下文

因此，本设计把截图从“截图 API 调用”升级为一等领域概念：Capture Session。

## 2. 目标

1. 建立 Capture Session 生命周期：创建、交互、渲染、输出、取消、清理。
2. 冻结截图开始瞬间的桌面画面，后续所有裁剪和输出都基于同一份冻结数据。
3. 明确 Logical Pixels 与 Physical Pixels 的转换规则。
4. 让截图、OCR、OCR + 翻译共享同一套截图会话模型。
5. 保持 SnapLingo 现有分层架构：Commands 薄层、Application 管业务、Infrastructure 管平台适配。

## 3. 非目标

第一轮不实现以下能力：

- UI 元素级检测
- 滚动长截图
- 录屏/GIF
- 贴图分组和持久化
- Linux Wayland 下完整窗口检测保证

这些能力应建立在 Capture Session 模型之上，后续分阶段加入。

## 4. 架构

### 4.1 模块划分

```
Commands Layer
  capture_session_commands.rs
    -> create_capture_session
    -> cancel_capture_session
    -> render_capture_output
    -> output_capture
    -> run_capture_ocr

Application Layer
  CaptureSessionService
    -> 会话生命周期
    -> 冻结帧缓存
    -> 坐标转换入口

  ImageCompositionService
    -> 裁剪
    -> 标注合成
    -> 格式转换

  CaptureOutputService
    -> 复制
    -> 保存
    -> 贴图
    -> 成功截图历史

Domain Layer
  capture.rs
    -> CaptureSessionId
    -> MonitorSnapshot
    -> LogicalRect
    -> PhysicalRect
    -> VirtualDesktop
    -> AnnotationCommand
    -> CaptureOutputAction

Infrastructure Layer
  ScreenshotBackend
    -> 显示器枚举
    -> 显示器截图
    -> 可选窗口枚举
    -> 权限状态

  ClipboardBackend
    -> 图片复制

  PinnedWindowBackend
    -> 置顶贴图窗口
```

### 4.2 依赖方向

- Commands 只做参数转换和错误转换。
- Application 使用 Domain 类型编排业务。
- Application 依赖 Infrastructure trait，不依赖具体平台实现。
- Infrastructure 不知道 Capture Mode、OCR、Translation。
- Frontend 不直接调用底层 `capture_region` 作为主流程。

## 5. Capture Session 生命周期

### 5.1 创建

用户触发截图快捷键后：

1. `WorkflowService` 根据 Capture Mode 请求创建 Capture Session。
2. `CaptureSessionService` 调用 `ScreenshotBackend` 枚举显示器。
3. 后端捕获每个显示器的冻结图。
4. 后端生成 `CaptureSessionId`，把冻结帧保存在内存中。
5. 返回 `CaptureSessionView` 给前端：session id、monitor geometry、scale factor、图片数据。

### 5.2 交互

前端在冻结画面上完成：

- 鼠标拖选
- 选区移动和缩放
- 尺寸提示
- 工具条展示
- 标注命令收集
- OCR/复制/保存/贴图按钮触发

前端只传带坐标空间的 rect，不做真实图像裁剪。

### 5.3 输出

输出时：

1. 前端调用 `output_capture(session_id, logical_rect, annotations, action)`。
2. `CaptureSessionService` 将 logical rect 转换为 physical rect。
3. `ImageCompositionService` 从冻结帧裁剪原图。
4. 如果输出需要标注，合成 annotation layer。
5. `CaptureOutputService` 执行复制、保存或贴图。
6. 输出成功后写入截图历史。
7. 对一次性模式，释放 session；对编辑态仍需继续操作时保留 session。

### 5.4 取消和清理

- `Esc` 或关闭按钮取消 session。
- 超时未使用的 session 自动清理。
- App 退出时清理全部 session。
- 创建新 session 前可选择取消旧 session，避免内存堆积。

## 6. 坐标模型

必须引入显式类型，禁止跨层裸传 `x/y/width/height`。

```rust
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub struct MonitorSnapshot {
    pub id: MonitorId,
    pub logical_bounds: LogicalRect,
    pub physical_bounds: PhysicalRect,
    pub scale_factor: f64,
    pub image_png_base64: String,
}
```

转换规则：

- 前端鼠标事件产生 logical rect。
- 后端根据 monitor bounds 和 scale factor 转换到 physical rect。
- 跨显示器选区先分割到多个 monitor，再裁剪并拼接。
- 单显示器第一版可以先实现完整，跨显示器选区可以在多屏阶段实现。

## 7. 工作流对接

### Screenshot Mode

```
hotkey -> create session -> select/edit -> output(copy/save/pin) -> history
```

### OCR Mode

```
hotkey -> create session -> select -> crop original image -> OCR -> result window
```

### OCR + Translation Mode

```
hotkey -> create session -> select -> OCR -> result window -> translation stream
```

### Screenshot Editor OCR

编辑器内 OCR 使用同一 session 的原图裁剪，不使用带标注的合成图。

## 8. 分阶段交付

### Phase 1: Session Foundation

- Domain 类型
- `ScreenshotBackend` 扩展
- `CaptureSessionService`
- 基础 session 创建和取消
- 单显示器冻结图返回

### Phase 2: Frozen Selection UI

- 前端显示真实冻结图
- 选区交互
- 原位编辑入口
- 移除主流程二次截图

### Phase 3: Output Pipeline

- 裁剪冻结图
- 复制到剪贴板
- 保存文件
- OCR 原图裁剪
- 输出成功后写历史

### Phase 4: Multi-Monitor and DPI

- 全显示器捕获
- virtual desktop geometry
- logical/physical 转换
- 负坐标和混合 DPI

### Phase 5: Snipaste Interaction Polish

- 放大镜
- 取色
- 键盘微调
- 上次成功截图区域
- 截图历史区域回放

### Phase 6: Window Detection

- 窗口枚举
- 鼠标命中窗口
- `Tab` 切换检测模式
- 不支持平台降级为手动选区

### Phase 7: Pin/Paste

- 基础置顶贴图
- 缩放、透明度、关闭
- 最近关闭贴图恢复

## 9. 测试策略

### Rust Unit Tests

- 坐标转换
- rect normalize
- session 创建/取消/清理
- 裁剪和跨 monitor 分割
- 输出成功才写历史

### Rust Integration Tests

- mock screenshot backend 创建 session
- mock clipboard backend 验证复制
- mock output backend 验证保存/贴图

### Frontend Tests

- 选区 normalize
- keyboard shortcut handling
- session state transitions
- annotations 不影响 OCR 请求

### Manual Verification

- macOS Retina 单屏
- macOS 外接普通屏
- 显示器在主屏左侧产生负坐标
- 截图开始后改变屏幕内容，输出仍等于冻结画面

## 10. 风险和决策

### 风险：全屏图片 base64 传输过大

第一版可以沿用 base64，先保证架构正确。后续可优化为临时文件路径、asset URL 或共享缓存。

### 风险：跨显示器选区复杂

先把单显示器和当前 monitor 内选区做完整，多显示器作为独立 phase。Domain 类型从第一天支持多显示器，避免接口重写。

### 风险：Wayland 能力不一致

截图和窗口检测分离。Wayland 截图失败时给出权限/portal 提示；窗口检测失败时降级到手动选区。

### 风险：Session 内存占用

限制同时活跃 session 数量，默认只保留当前 session。输出成功或取消后释放冻结帧。

## 11. 成功标准

- 用户触发截图后看到真实冻结画面
- 选区输出不受截图后屏幕变化影响
- OCR 使用无标注原图
- 复制/保存/贴图使用合成图
- 坐标转换集中在后端可测试模块
- Commands 层保持薄，前端不直接承担平台截图复杂度
