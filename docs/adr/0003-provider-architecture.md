# ADR 0003: Provider Architecture

## Status
Accepted (2026-06-13), amended by ADR 0004 and ADR 0005

## Context

SnapLingo 需要支持多种 OCR 和 Translation Provider：

- 本地 Provider，例如 Tesseract
- 远程 Provider，例如 Baidu OCR、DeepL、Baidu Translation
- 自定义 Translation Provider，兼容 OpenAI、Anthropic、Gemini 协议

Provider 架构需要满足：

- OCR 和 Translation 能力类型安全
- 支持运行时激活状态变化
- 凭证存储在系统级安全存储中
- macOS、Windows、Linux 的平台差异隔离在 Infrastructure
- 测试可以通过稳定的 module interface 覆盖 Provider 行为

## Decision

Provider 代码按能力类型组织为垂直切片，位于 `src-tauri/src/application/providers/`。

```text
application/providers/
├─ common/
│  └─ provider.rs
├─ configuration.rs
├─ ocr/
│  ├─ trait_def.rs
│  ├─ coordinator.rs
│  └─ impls/
└─ translation/
   ├─ trait_def.rs
   ├─ coordinator.rs
   └─ impls/
```

每类 Provider 包含：

- 通用 Provider trait，用于共享元数据
- 能力专属 trait，例如 `OcrProvider` 或 `TranslationProvider`
- Coordinator module，负责 Provider 注册、激活状态、持久化、执行协调和运行时重配置
- `impls/` 下的具体 Provider implementation

早期的 Registry/Service 拆分不再是当前架构。ADR 0004 已将这两个 module 合并为 `OcrCoordinator` 和 `TranslationCoordinator`。

## Current Module Responsibilities

| Module | Responsibility |
| --- | --- |
| `common/provider.rs` | 共享 Provider 元数据 interface |
| `ocr/trait_def.rs` | OCR Provider 能力 interface |
| `translation/trait_def.rs` | Translation Provider 能力 interface |
| `ocr/coordinator.rs` | OCR Provider 单选激活、持久化、执行和运行时重配置 |
| `translation/coordinator.rs` | Translation Provider 多选激活、持久化、并发执行、排序、自定义 Provider 注册和运行时重配置 |
| `configuration.rs` | 凭证校验、自定义 Translation Provider 定义、LLM Translation Provider 构造 |
| `impls/` | 具体 Provider implementation |

## Platform Infrastructure

平台差异保留在 Infrastructure modules：

- Keychain：macOS Keychain、Windows Credential Manager、Linux Secret Service
- paths：平台相关的配置和数据路径
- HTTP：`HttpClient` interface 和 Reqwest adapter
- system modules：screenshot、hotkey、window、clipboard、TTS adapters

Application modules 通过 composition 使用这些 infrastructure interface 或 adapter；UI 和 command modules 不直接拥有平台差异知识。

## Consequences

正面影响：

- Provider 激活和执行知识集中在 Coordinator modules，locality 更好。
- Commands 保持薄层，只调用 Coordinator interface。
- 测试可以覆盖 Coordinator 行为，而不是绑定 command internals。
- 新增内置 Provider 只需要实现 trait 并在 composition 注册。
- 新增自定义 Translation Provider 复用 Provider Configuration Module。

权衡：

- OCR 和 Translation Coordinator 有少量重复，这是为了保留各自不同的激活模型。
- 自定义 Translation Provider 生命周期横跨凭证、配置、运行时注册和 Coordinator 状态；ADR 0005 记录了运行时重配置决策。
- 提到 Registry/Service 的历史文档不再作为当前架构指导。

## Related

- ADR 0002: Main Window Structure
- ADR 0004: Coordinator Consolidation
- ADR 0005: Runtime Provider Reconfiguration
- `CONTEXT.md`
- `ARCHITECTURE.md`
