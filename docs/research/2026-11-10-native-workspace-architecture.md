# 原生跨平台 Rust 工作区架构：成熟项目源码对照

日期：2026-11-10。用途：为 SnapLingo 后续架构决策提供证据；本文不是迁移完成记录。SnapLingo 观察基线为 `cebc309`，设计不以保留该基线的目录或耦合为前提。

## 结论

建议按“产品组装、业务能力、共享 UI、平台能力、持久化”划分实际职责，再用 Cargo package 固定需要独立编译、依赖隔离或复用的部分。功能内部优先聚合状态、规则、输入、展示与测试，避免按技术名词拆成大量薄 crate。

成熟项目并没有统一的 `apps/crates/packages` 目录模板。它们更一致的做法是让产品入口完成组装，把窗口系统、渲染和可移植行为之间的 Interface 说清楚，并把平台选择限制在具体位置。**目录规范应该表达依赖关系；换目录名本身不会降低耦合。** 以下“源码事实”和“SnapLingo 建议”分开记录。

## 取样和证据范围

选择 Zed/GPUI、WezTerm、iced、Slint，是为了对照原生桌面产品和跨平台 UI 框架两种维护目标。这里把“非 vibecoding”理解为参考有公开实现、真实平台后端与维护历史的工程项目，不对项目是否使用 AI 辅助编码作未经核实的保证，也不声称这是按热度严格排名的前四名。

通过 GitHub 官方仓库的 Git 协议取得 HEAD，再从 `raw.githubusercontent.com` 读取同一提交下的文件；下文链接固定到提交，避免默认分支继续变动后证据漂移。GitHub REST API 在调查时限流，未使用它提供的 stars、提交统计等数字。

| 项目 | 本次查阅的固定提交 | 作为样本的角色 |
| --- | --- | --- |
| [Zed](https://github.com/zed-industries/zed/tree/f0d896f2c18a543335eb155f729d36b624695591) | `f0d896f2c18a543335eb155f729d36b624695591` | 大型原生应用及其 GPUI 框架 |
| [WezTerm](https://github.com/wezterm/wezterm/tree/458a100c1bf4134d36cb5c42dd90e4c8266d9e1a) | `458a100c1bf4134d36cb5c42dd90e4c8266d9e1a` | 有独立业务引擎、GUI 和平台窗口实现的桌面产品 |
| [iced](https://github.com/iced-rs/iced/tree/8ff48c42b0d5188223501ea1ccc506fd5beaf36a) | `8ff48c42b0d5188223501ea1ccc506fd5beaf36a` | 状态、输入、渲染、窗口运行时的职责划分 |
| [Slint](https://github.com/slint-ui/slint/tree/d600063f3db7ebfc8f27d92cee6e68736774eca8) | `d600063f3db7ebfc8f27d92cee6e68736774eca8` | 可替换平台后端与不依赖真实窗口的测试 |

## 源码事实与可借鉴模式

| 维度 | Zed / GPUI | WezTerm | iced | Slint |
| --- | --- | --- | --- | --- |
| 产品组装 | `crates/zed/src/main.rs` 选择平台并初始化产品功能 [Z1] | `wezterm-gui` 依赖 `window`、`mux`、`wezterm-term`；`GuiFrontEnd` 初始化连接并处理 mux 通知 [W1] | 框架入口接受 `update`、`view`；产品提供自己的状态和消息 [I1] | Rust 公共入口通过 backend selector 运行事件循环 [S1] |
| 可移植 UI / 引擎 | `ui` 依赖 `gpui`、theme、icons；`gpui` 定义平台 Interface [Z2] | `term` 保留终端状态与解析，定义它消费的 Clipboard/AlertHandler 等 Interface [W2] | `iced_core` 定义布局、事件、Widget、Renderer 等值和 Interface；`iced_runtime` 声明 renderer-agnostic [I2] | `internal/core` 定义 Platform、WindowAdapter 和 Renderer 的关系 [S2] |
| 平台选择 | `gpui_platform::current_platform` 汇集 macOS、Windows、Linux 等选择；平台实现是独立 crate [Z3] | `window/src/os` 中按 OS 选择实现；它们留在一个 `window` crate [W3] | `iced_winit` 依赖窗口运行时和平台剪贴板，实现 shell 职责 [I3] | selector 在 Qt、winit、Linux KMS 等启用的后端间选择 [S3] |
| 渲染与测试 | PlatformWindow / PlatformTextSystem 独立命名；GPUI 有测试支持 [Z2][Z3] | `wezterm-gui` 与终端引擎、window 的 Cargo 依赖分离 [W1][W2] | `iced_renderer` 的 WGPU、tiny-skia 后端可选；core 定义 Renderer trait [I4] | testing backend 通过同一个 `set_platform` 接入，并支持 mock time [S4] |
| 不能照搬的部分 | 大量 crate 和广泛初始化注册适合其规模，不能证明小项目也需要同样拆分 | `window` 仍依赖产品 `config`，并非完全独立的通用窗口库 [W3] | 通用 GUI 框架的多后端、泛型层次，不是单产品必须复制的目录 | 多语言 API、编译器、内部后端包是框架需求，SnapLingo 无此产品要求 |

### 1. Zed：应用、UI 语言与平台实现各有所有者

**事实。** `crates/zed/src/main.rs::build_application` 从 `gpui_platform` 取得平台，再传给 GPUI Application；同一产品入口随后初始化 settings、project、editor、workspace 等功能。`ui/Cargo.toml` 依赖 GPUI 和主题等共享呈现资源。`gpui/src/platform.rs` 定义 `Platform`、`PlatformWindow`、`PlatformTextSystem`；`gpui_platform/Cargo.toml` 通过 target-specific dependencies 选择 `gpui_macos`、`gpui_windows`、`gpui_linux` 等实现。[Z1][Z2][Z3]

**SnapLingo 推论。** 产品应该有明确的组装入口，业务或 UI 模块不能为了获得依赖而反向访问该入口。UI 可以有统一的布局、交互和语义，而平台负责事件循环、窗口、文本服务和展示机制。没有必要因平台种类相同就立即拆出 `platform-macos`、`platform-windows` 两个 crate；Zed 的拆分是可选组织方式，不是跨平台的先决条件。

### 2. WezTerm：按业务能力拆，平台实现可以留在一个 crate

**事实。** `wezterm-gui` 是实际产品组装层，直接依赖 `wezterm-term`、`mux` 和 `window`。`term/src/terminal.rs` 把终端状态和解析组合成 `Terminal`，并声明 Clipboard、AlertHandler、DownloadHandler 等被业务引擎消费的 Interface。`window/src/os/mod.rs` 选择平台实现；`window` 仍依赖 `config`，其公开类型使用产品配置类型。[W1][W2][W3]

**SnapLingo 推论。** OCR 协调、截图编辑、滚动拼接等应该以业务能力形成深模块。模块的 Interface 隐藏真实复杂度，而不只是把下一层方法转发一遍。消费方拥有需要的 Interface、适配方满足它，是有实际源码先例的做法。也应承认部分产品专用模块没有必要发布成通用库：模块可跨平台，不代表必须与产品完全无关。

### 3. iced：状态和消息复用比统一窗口代码更有价值

**事实。** iced 的入口示例把状态修改放进 `update`，显示放进 `view`，以类型化 `Message` 连接。`iced_runtime` 的清单称其为与渲染器无关的运行时，依赖 core/futures；`iced_winit` 另管窗口运行时及剪贴板。`iced_renderer` 的 WGPU、tiny-skia 是可选依赖。注意 `iced_widget` 确实依赖 `iced_renderer` 和 `iced_runtime`，不能把这个项目描述成所有 UI 类型完全不依赖渲染包。[I1][I2][I3][I4]

**SnapLingo 推论。** 优先统一 `Snapshot → 输入/状态转换 → Intent → 更新后的 Snapshot`，并复用布局、命中区域、焦点、文本编辑状态与可访问性语义。平台执行截图、打开文件、权限请求等副作用，再把结果回送。不能仅共享颜色与按钮尺寸，就称为已完成 UI 复用。也无需为了借鉴此模式，把产品全面切换到 iced。

### 4. Slint：生产后端和测试后端应接入同一个 seam

**事实。** `Platform` 负责创建 WindowAdapter、事件循环等宿主能力；`WindowAdapter` 提供 Window 和 Renderer。backend selector 负责选择具体后端。testing backend 调用相同的 `set_platform`，既可无事件循环，也可带事件循环与模拟时钟。[S1][S2][S3][S4]

**SnapLingo 推论。** 共享布局、输入状态和异步结果有效性应可在无真实窗口时测试；真实窗口测试则验证坐标转换、DPI、IME、焦点、透明表面和系统可访问性。模拟后端测试通过，不能代替两种桌面系统的实际验收。若测试必须复制一套业务流程，或直接把实现文件挂进测试，先修正 Interface，而不是继续堆测试辅助入口。

## 对 SnapLingo 的设计建议

这一节是从样本推导的设计选择，不代表四个上游项目使用同一架构。

### 应先明确三种“复用”

1. **产品跨平台复用**：同一份截图规则、OCR/翻译协调、配置约束、历史收藏行为，在 macOS/Windows 使用。归业务模块。
2. **产品 UI 跨宿主复用**：同一份展示状态、布局、输入、用户意图和可访问性语义，用于不同原生窗口和无窗口预览。归 UI 与产品 presenter。
3. **跨产品 UI 库复用**：通用控件、主题、文字编辑或绘制设施，能被另一个应用使用。这要求稳定公共 Interface 和真实外部消费者，不能由前两项自动推导。

当前主要目标可以明确选择前两项。保留第三项的清晰分离位置，但不必立即建设完整 GUI 框架。若要真正提供通用 UI 库，则需要把 `Settings`、OCR 服务配置、历史收藏等产品页面与通用 UI 分开评估，而不是仅把现有页面移到一个叫 `ui` 的 crate。

### 推荐依赖方向与所有者

```text
app                产品启动、依赖组装、驻留 shell、功能 presenter、原生窗口宿主
   ├── core        按 capture / translation / ocr / library / settings 聚合业务
   ├── ui          通用呈现基础 + 产品视图；不自行寻找应用服务
   ├── platform    操作系统能力；按能力组织，能力内含各 OS 实现
   ├── storage     持久化实现、迁移和事务
   └── assets      有共享嵌入需求的资源

platform / storage / 外部 provider 实现 → 它们消费或实现的业务 Interface
UI 模型与业务模型之间的映射 → 产品 presenter
```

图中是职责关系，不要求每一行都立即是一个独立 package。`core` 名称也不要求把所有与产品相关的 Rust 代码塞进去：原生句柄、具体 HTTP 客户端、模型下载与推理引擎装载、SQLite 和应用事件循环都有更合适的所有者。

业务模块内放自己的模型、状态转换、用例和所需 Interface；按功能聚合，内部使用具体行为命名模块。最终设计统一采用这一规则，复杂功能按子能力继续拆分。DDD 和 `domain/application/infrastructure` 的逻辑职责通过 Interface 和依赖约束保留，不另建这三类目录。UI 模块内同样聚合该视图的输入、布局、状态和语义。宿主只执行需要 OS 参与的行为；产品 presenter 负责业务模型到视图模型的投影、请求有效性和操作顺序。具体规则见[目标架构设计](../architecture/native-workspace-design.md)。

按后续设计讨论，SnapLingo 只有一个桌面产品，推荐目标使用根目录 `app/`，省去 `apps/desktop/` 的中间层。基线实现路径仍是 `apps/desktop/`；这项简化是项目自身的选择，不是上游源码事实。

### crate 的拆分与合并条件

| 候选 package / 模块 | 建议职责 | 什么时候独立成 crate | 什么时候保持模块或合并 |
| --- | --- | --- | --- |
| `app` | 产品二进制、生命周期和具体依赖组装 | 现在就应是应用 package；运行时入口明确 | 单一产品无需 apps 容器；不按 OS 复制整个产品 package，除非启动/发布生命周期确实独立 |
| `core` | 可移植产品规则和用例 | 需在无 UI/OS/数据库依赖下编译和测试，理由已充分 | 内部先按功能模块组织；只有独立消费者或明显依赖成本时再拆 `capture` 等 crate |
| `ui` | 展示模型、输入状态、布局、语义及绘制 | 原生宿主、预览与测试需要消费同一实现，理由已充分 | 不因为 draw/layout/state 文件不同就拆包；若通用库有外部消费者，再分基础控件和产品页面 |
| `platform` | OS 能力实现与平台选择 | 需要将平台依赖隔离在业务/UI之外，理由已充分 | 单个 clipboard/dialogs/hotkeys 先是模块；按 OS 拆 crate 要有独立编译、发布或依赖维护收益 |
| `storage` | SQLite、事务、迁移、持久化图片 | 能用真实数据库独立测试，并隔离数据库依赖 | repository/transaction/migration 不是三个 crate；按存储一致性需求聚合 |
| `providers` | 云端 OCR/翻译协议、LLM API 和本地引擎接入 | 有桌面以外消费者，或需要隔离 HTTP/推理重依赖及可选 feature 时值得拆 | 纯转发注册表不能成为新层；服务选择、重试/取消策略与具体协议分清所有者 |
| `render` | 图形、文字绘制后端 | 出现第二个实际渲染后端/消费者，或需独立控制依赖和发布时再拆 | 单个渲染实现可先是 `ui` 的可选模块；独立目录不强制独立 package |
| `accessibility` | 共享语义投影 | 通用跨项目语义转换有独立消费者时才拆 | 语义树留 UI；原生 AX/UIA 接线留 host。不要把二者混成一个“可访问性全家桶” |
| `assets` | 被多个包实际使用的嵌入资源 | 当前确有共享嵌入和资源身份需求时保留 | 仅属于 desktop 的图标/打包模板放 app；不为了统一目录让底层依赖全部产品资源 |
| `runtime` / `common` / `ports` | 无单一业务所有者时易形成杂物集合 | 只有可明确命名的能力、独立生命周期与真实消费者才另立 package | 优先把执行器接线留产品，把 Interface 放消费模块；拒绝仅用技术名词解释职责 |

拆包应回答三个问题：调用者少知道了哪些细节；依赖图获得了什么可验证的约束；改变一个业务行为是否需要改更少的地方。若只增加一个转发调用和一个 `Cargo.toml`，通常不值得。

### 目录风格与架构验收应分开

- `apps/`、`crates/`、`tools/`、`docs/`、标准 `src/`、`tests/`、`examples/` 可以改善开源贡献者导航，但四个样本并不都使用这套根目录。
- `cfg(target_os)` 集中在平台选择和宿主实现；纯业务不以当前操作系统决定持久化规则。
- 共享 UI 的默认依赖不包含数据库、应用全局状态或原生窗口框架；渲染可按实际需要选启。
- 宿主接收该功能所需的有限依赖，而不是为了方便拿到整个 `AppState`。
- 用 Cargo 依赖图（包含 target-specific edges）检查包级规则；不要只扫描当前平台编译到的代码。
- 为共享行为、数据库事务、原生宿主分别设置验收；交叉编译只能证明编译，不能证明实际 IME/窗口生命周期正确。

不建议在这轮目录架构决策中顺便引入插件系统、跨进程 RPC、统一全局事件总线或更换 UI 框架。它们都需要独立的产品约束或测量结果，不能由“现代开源风格”推出。

## 第一手来源

[Z1]: https://github.com/zed-industries/zed/blob/f0d896f2c18a543335eb155f729d36b624695591/crates/zed/src/main.rs
[Z2]: https://github.com/zed-industries/zed/blob/f0d896f2c18a543335eb155f729d36b624695591/crates/gpui/src/platform.rs
[Z3]: https://github.com/zed-industries/zed/blob/f0d896f2c18a543335eb155f729d36b624695591/crates/gpui_platform/src/gpui_platform.rs
[W1]: https://github.com/wezterm/wezterm/blob/458a100c1bf4134d36cb5c42dd90e4c8266d9e1a/wezterm-gui/src/frontend.rs
[W2]: https://github.com/wezterm/wezterm/blob/458a100c1bf4134d36cb5c42dd90e4c8266d9e1a/term/src/terminal.rs
[W3]: https://github.com/wezterm/wezterm/blob/458a100c1bf4134d36cb5c42dd90e4c8266d9e1a/window/src/os/mod.rs
[I1]: https://github.com/iced-rs/iced/blob/8ff48c42b0d5188223501ea1ccc506fd5beaf36a/src/lib.rs
[I2]: https://github.com/iced-rs/iced/blob/8ff48c42b0d5188223501ea1ccc506fd5beaf36a/runtime/Cargo.toml
[I3]: https://github.com/iced-rs/iced/blob/8ff48c42b0d5188223501ea1ccc506fd5beaf36a/winit/Cargo.toml
[I4]: https://github.com/iced-rs/iced/blob/8ff48c42b0d5188223501ea1ccc506fd5beaf36a/renderer/Cargo.toml
[S1]: https://github.com/slint-ui/slint/blob/d600063f3db7ebfc8f27d92cee6e68736774eca8/api/rs/slint/lib.rs
[S2]: https://github.com/slint-ui/slint/blob/d600063f3db7ebfc8f27d92cee6e68736774eca8/internal/core/platform.rs
[S3]: https://github.com/slint-ui/slint/blob/d600063f3db7ebfc8f27d92cee6e68736774eca8/internal/backends/selector/lib.rs
[S4]: https://github.com/slint-ui/slint/blob/d600063f3db7ebfc8f27d92cee6e68736774eca8/internal/backends/testing/lib.rs

补充依赖和类型定义证据：

- Zed：[工作区清单](https://github.com/zed-industries/zed/blob/f0d896f2c18a543335eb155f729d36b624695591/Cargo.toml)、[产品依赖](https://github.com/zed-industries/zed/blob/f0d896f2c18a543335eb155f729d36b624695591/crates/zed/Cargo.toml)、[UI 依赖](https://github.com/zed-industries/zed/blob/f0d896f2c18a543335eb155f729d36b624695591/crates/ui/Cargo.toml)、[平台选择依赖](https://github.com/zed-industries/zed/blob/f0d896f2c18a543335eb155f729d36b624695591/crates/gpui_platform/Cargo.toml)。
- WezTerm：[GUI 依赖](https://github.com/wezterm/wezterm/blob/458a100c1bf4134d36cb5c42dd90e4c8266d9e1a/wezterm-gui/Cargo.toml)、[终端引擎依赖](https://github.com/wezterm/wezterm/blob/458a100c1bf4134d36cb5c42dd90e4c8266d9e1a/term/Cargo.toml)、[window 依赖](https://github.com/wezterm/wezterm/blob/458a100c1bf4134d36cb5c42dd90e4c8266d9e1a/window/Cargo.toml)、[window 公共类型](https://github.com/wezterm/wezterm/blob/458a100c1bf4134d36cb5c42dd90e4c8266d9e1a/window/src/lib.rs)。
- iced：[core 类型](https://github.com/iced-rs/iced/blob/8ff48c42b0d5188223501ea1ccc506fd5beaf36a/core/src/lib.rs)、[Renderer Interface](https://github.com/iced-rs/iced/blob/8ff48c42b0d5188223501ea1ccc506fd5beaf36a/core/src/renderer.rs)、[widget 真实依赖](https://github.com/iced-rs/iced/blob/8ff48c42b0d5188223501ea1ccc506fd5beaf36a/widget/Cargo.toml)。
- Slint：[WindowAdapter Interface](https://github.com/slint-ui/slint/blob/d600063f3db7ebfc8f27d92cee6e68736774eca8/internal/core/window.rs)、[selector 依赖](https://github.com/slint-ui/slint/blob/d600063f3db7ebfc8f27d92cee6e68736774eca8/internal/backends/selector/Cargo.toml)。
