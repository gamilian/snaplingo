# 从 Easydict 与 Snapzy 借鉴的非截图能力

> 调研基线：Easydict [`eaff88b4`](https://github.com/tisfeng/Easydict/tree/eaff88b4e981714c127b62aa8068f8ba5587cfcf)，Snapzy [`bf8ca39f`](https://github.com/duongductrong/Snapzy/tree/bf8ca39f8e13915b203b9aa3c73066b75f94284f)。本文只列出对 SnapLingo（跨平台截图、OCR、翻译、BYOK）有明确价值的想法，不构成实现计划。

## 可直接采用（产品原则与小范围能力）

| 想法 | 证据 | 对 SnapLingo 的最小落地 |
| --- | --- | --- |
| **按能力解释并分级权限** | Snapzy 的 onboarding 将 Screen Recording、保存目录标为必需，Accessibility、通知等标为可选；用户可从单项重新授权。[`OnboardingPermissionsView.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Features/Onboarding/Components/OnboardingPermissionsView.swift) | 在现有权限页显示“截图必需 / 智能选区增强 / 划词翻译增强”，将缺权限降级为可用的矩形截图或手输翻译，不在启动时索要全部权限。 |
| **快捷键冲突可见、而不是静默失败** | Snapzy 将可配置快捷键分为全局、叠层和悬浮卡片作用域，拒绝同一命名空间的重复绑定，并提示系统截图快捷键冲突。[`SHORTCUTS.md`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/docs/SHORTCUTS.md) | 在设置中做跨平台的“已占用/本应用重复”校验；默认未绑定高风险或非核心动作。无需复刻 Carbon/Fn 支持。 |
| **轻量的捕获后快速操作** | Snapzy 的 Quick Access 统一承载复制、保存/打开、编辑、钉图、删除等操作，并让快捷键和点击复用同一 action path。[`QUICK_ACCESS.md`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/docs/QUICK_ACCESS.md) | 只做一张临时卡片：复制、保存、贴图、OCR/翻译；复用 SnapLingo 既有的输出命令，不做卡片堆栈、云上传或手势系统。 |
| **历史记录的明确保留和删除语义** | Snapzy 将记录、缩略图、临时文件、历史清理分开，保留策略有时间和数量上限；清除历史不删除用户导出的原文件。[`HISTORY.md`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/docs/HISTORY.md) | 把同样的语义固化到 SnapLingo History/Favorites：历史开关、条数/天数上限、清除历史与删除导出文件必须是不同动作。 |

## 需要适配后采用

| 想法 | 为什么有价值 | 必需约束与证据 |
| --- | --- | --- |
| **本地优先、远端 OCR 明示上传边界** | Snapzy 默认 Apple Vision；可选的自定义远端模型才会把图像以 JPEG base64 发往用户指定端点。[`OCRService.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Media/OCRService.swift)、[`RemoteOCRService.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Media/OCR/RemoteOCRService.swift) | SnapLingo 已支持本地与 BYOK OCR；在 provider 表单和每次切换远端时标明“图像/文本会离开设备”、目标域名与是否保留。API 密钥保持平台凭据存储，不要以此为由增加默认云端 OCR。 |
| **提供商的“连通性测试”而非以真实截图试错** | Snapzy 给远端 OCR 提供小型生成图的 `testConnection`，将鉴权、网络、响应格式错误分开呈现。[`RemoteOCRService.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Media/OCR/RemoteOCRService.swift) | 为翻译、OCR、TTS provider 添加显式“测试连接”；使用无敏感的固定样例，绝不自动上传剪贴板或最近截图。 |
| **跨应用划词的渐进回退** | Easydict 将选中文本取得设计为 Accessibility → 浏览器 AppleScript → 模拟复制的降级链，并只在首次实际使用该能力时请求辅助功能权限。[`select-text-flow.md`](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/docs/architecture/select-text-flow.md)、[`GUIDE.md`](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/docs/user-docs/en/GUIDE.md) | 抽象为每个平台的 `SelectedTextSource`：先调用低权限路径，失败再显示“复制后翻译”的显式操作。模拟复制会改写剪贴板、AppleScript 需自动化授权，均须让用户选择，不可静默执行。 |
| **可恢复的快捷键/权限问题诊断** | Snapzy 会检查 Screen Recording，也记录“系统已授权但当前 app identity 不可用”的状态；快捷键冲突能链接到对应设置页。[`ScreenCaptureManager.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Capture/ScreenCaptureManager.swift)、[`SystemScreenshotShortcutManager.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Shortcuts/SystemScreenshotShortcutManager.swift) | 将错误建模为可行动状态（缺权限、冲突、未配置 provider、网络/认证失败），每个状态只提供相关的修复入口。平台的签名/系统设置深链留在基础设施层。 |
| **可编辑历史的副数据，而非只存缩略图** | Snapzy 以带签名的 sidecar 保存标注会话，历史恢复时优先恢复可编辑会话，缺失时才打开扁平图。[`HISTORY.md`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/docs/HISTORY.md)、[`ANNOTATE.md`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/docs/ANNOTATE.md) | 仅当“从历史重新编辑标注”成为明确需求时再引入；把 schema 版本、原图引用、标注命令与清理生命周期一起设计，避免孤儿 sidecar。 |

## 不建议借鉴

- **不要把 Snapzy 的完整原生快捷键/事件系统移进 Tauri。**Carbon、Fn 全局监视、临时 hover 注册、系统偏好读取和多套键盘事件模型是 macOS 专用且维护量很高；SnapLingo 应仅保留一份跨平台快捷键注册表，平台差异置于适配器。证据见 [`SHORTCUTS.md`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/docs/SHORTCUTS.md)。
- **不要为普通截图或划词默认开启侵入性 Accessibility/事件注入。**Snapzy 的实时穿透捕获依赖全局 event tap；Easydict 的最终回退会模拟 `Cmd+C`。两者会影响前台应用行为，只应作为显式增强能力。证据见 [`AreaSelectionWindow.swift`](https://github.com/duongductrong/Snapzy/blob/bf8ca39f8e13915b203b9aa3c73066b75f94284f/Snapzy/Services/Capture/AreaSelectionWindow.swift) 与 [Easydict 选择文本流程](https://github.com/tisfeng/Easydict/blob/eaff88b4e981714c127b62aa8068f8ba5587cfcf/docs/architecture/select-text-flow.md)。
- **不要提前复制 Quick Access 的云、录屏、GIF、编辑器和复杂手势栈。**它们是 Snapzy 截图套件的产品宽度，不是 SnapLingo OCR/翻译闭环的短板；先验证一张最小的捕获后卡片是否提高“复制/翻译/贴图”完成率。

## 建议优先级

1. 权限状态页与快捷键冲突提示。
2. 最小捕获后卡片，以及历史记录的保留/删除语义。
3. Provider 测试连接与远端数据外发提示。
4. 只有在划词翻译覆盖率确有问题时，做带用户确认的跨应用文本获取回退链。
