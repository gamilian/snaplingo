# ADR 0004: Coordinator Consolidation

## Status
Accepted (implemented)

## Context

在早期架构重构中，Application 层曾包含两个 Provider 模块：
- **Registry**: 管理 Providers 和激活状态
- **Service**: 协调 Provider 执行

这两个模块职责重叠，造成架构摩擦：

### 发现的问题

通过架构复审，发现以下架构问题：

1. **浅层抽象（Shallow Module）**
   - Service 是 Registry 的 1:1 传递包装器
   - `TranslationService.translate()` 只是锁定 Registry，获取 providers，调用它们
   - `OcrService.recognize()` 同样只是简单的转发调用
   - 删除测试失败：删除 Service 只是将复杂度转移到调用者，而非集中化

2. **职责分离不清晰**
   - Registry 管理状态但不执行
   - Service 执行但不拥有状态
   - 持久化逻辑原本在 Commands，后移到 Registry 后，Service 变得更加"无事可做"

3. **不必要的间接层**
   ```rust
   // 旧调用链
   Command → lock Service → lock Registry → get provider → call provider
   
   // 4 层锁定和转发，只为一次 provider 调用
   ```

### 架构评审结论

根据 deep module 原则：
- **deep module** = 小 interface + 大量 implementation
- **shallow module** = interface 复杂度接近 implementation 复杂度

Service 层是典型的浅层模块，增加了间接层但没有提供足够的抽象杠杆。

## Decision

**合并 Registry 和 Service 为 Coordinator**，将状态管理和执行协调集中在一个模块。

### 命名

采用 **Coordinator（协调器）** 而非保留 Registry：

**理由：**
1. **语义准确性**: "Coordinator" 准确描述职责（协调多个 Provider 并发执行）
2. **清晰的演进**: `Registry + Service → Coordinator` 反映架构变化
3. **扩展性**: Coordinator 可以自然承担更多协调职责（缓存、流式、批量等）

**命名方案：**
- `TranslationCoordinator`: 管理翻译 Providers，协调并发翻译
- `OcrCoordinator`: 管理 OCR Providers，执行单个识别

### 并发策略

采用**内部细粒度锁**设计：

```rust
pub struct TranslationCoordinator {
    providers: HashMap<String, Arc<dyn TranslationProvider>>, // 不可变，无锁
    active: Arc<Mutex<Vec<String>>>,  // 可变，细粒度锁
    config: Arc<ConfigFile>,
}
```

**理由：**
1. **真正的并发**: 多个 `translate()` 调用可以并发执行
2. **最小锁粒度**: 只在修改 `active` 列表时锁定
3. **符合 Rust 惯例**: 细粒度锁 + `&self` 方法

**替代方案（拒绝）:**
- ~~外部锁整个 Coordinator~~ - 阻止并发翻译调用
- ~~所有字段都用 Mutex~~ - 过度工程，providers 从不在运行时改变

### 初始化模式

采用 **Builder 模式**：

```rust
// 构建阶段：可变
let mut coordinator = TranslationCoordinator::new(config);
coordinator.register(Arc::new(GoogleProvider::new(...)))?;
coordinator.register(Arc::new(DeepLProvider::new(...)))?;
coordinator.restore_from_config()?;

// 使用阶段：不可变共享
let coordinator = Arc::new(coordinator);
```

**理由：**
1. **符合 Rust 所有权模型**: 构建时可变，使用时不可变
2. **条件注册灵活**: 根据 API key 存在与否注册 Provider
3. **最小锁粒度**: `providers` 初始化后不变，无需锁定

**替代方案（拒绝）:**
- ~~构造函数接受 Vec~~ - 条件注册困难，代码笨拙
- ~~register() 改为 &self~~ - 过度设计，providers 从不运行时改变

### 持久化位置

持久化逻辑保留在 Coordinator 内部（之前的决策，ADR 0003）。

这进一步证明了合并的必要性：如果 Registry 已经负责持久化，Service 的存在价值更低。

## Implementation Status

该决策已经实现。当前代码中：

- `src-tauri/src/application/providers/translation/coordinator.rs` 是 Translation Provider 的当前 module。
- `src-tauri/src/application/providers/ocr/coordinator.rs` 是 OCR Provider 的当前 module。
- 旧的 Registry/Service module 已删除。
- ADR 0003 已更新为当前 Provider 架构，并明确本 ADR 对旧设计的修正。

## Consequences

### 正面影响

1. **深化模块**
   - 接口变简单：`coordinator.translate()` 一次调用
   - 实现集中化：状态管理 + 执行协调 + 持久化在一起

2. **局部性提升**
   - 所有 Provider 相关逻辑在一个模块
   - 修改、调试、理解都在一个地方

3. **Commands 层简化**
   ```rust
   // 之前
   state.translation_service.translate(&request).await
   
   // 之后 - 相同！但不需要 lock().unwrap()
   state.translation_coordinator.translate(&request).await
   ```

4. **测试更清晰**
   - 测试 Coordinator 即可，无需 mock Service 层
   - 减少测试的间接层

5. **并发性能提升**
   - 细粒度锁允许真正的并发
   - 无不必要的锁竞争

### 负面影响

1. **文件重命名**
   - `registry.rs` → `coordinator.rs`
   - `service.rs` 删除
   - 导入语句更新

2. **历史追溯**
   - Git blame 需要 `--follow`
   - 旧的执行计划和历史报告可能提到 "Registry" 和 "Service"，不再作为当前架构文档保留

3. **测试迁移**
   - Registry 测试和 Service 测试已迁移为 Coordinator 测试
   - Mock 对象随 Coordinator interface 更新

4. **学习曲线**
   - 开发者需要理解 Coordinator 概念
   - 不过 CONTEXT.md 已有详细文档

### 迁移结果

1. `CONTEXT.md`、`ARCHITECTURE.md`、ADR 0003 已对齐 Coordinator 架构。
2. `TranslationCoordinator` 和 `OcrCoordinator` 已作为当前 Provider modules。
3. Commands 和 AppState 已改为使用 Coordinator。
4. 旧 Registry/Service 文件和历史快照文档已删除。

## Related

- ADR 0003: Provider Architecture - 当前 Provider module 结构
- ADR 0005: Runtime Provider Reconfiguration - Coordinator runtime reconfiguration interface

## References

- *A Philosophy of Software Design* by John Ousterhout - 深度模块原则
- Rust API Guidelines - 所有权和并发模式
