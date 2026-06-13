# ADR 0004: Coordinator Consolidation

## Status
Proposed

## Context

在架构重构的 Phase 1-4 中，我们创建了清晰的四层架构（Commands → Application → Domain → Infrastructure）。Application 层包含两个模块：
- **Registry**: 管理 Providers 和激活状态
- **Service**: 协调 Provider 执行

这两个模块职责重叠，造成架构摩擦：

### 发现的问题

通过 `/improve-codebase-architecture` 技能分析，发现以下架构问题：

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
   // 当前调用链
   Command → lock Service → lock Registry → get provider → call provider
   
   // 4 层锁定和转发，只为一次 provider 调用
   ```

### 架构评审结论

根据《A Philosophy of Software Design》的深度原则：
- **深层模块** = 小接口 + 大量行为
- **浅层模块** = 接口复杂度 ≈ 实现复杂度

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
   - 旧的 ADR 和文档提到 "Registry" 和 "Service"

3. **测试迁移**
   - Registry 测试和 Service 测试需要合并
   - Mock 对象需要更新

4. **学习曲线**
   - 开发者需要理解 Coordinator 概念
   - 不过 CONTEXT.md 已有详细文档

### 迁移策略

1. **Phase 1**: 更新文档（CONTEXT.md、ADR）
2. **Phase 2**: 实现 Coordinator（基于 Registry 代码）
3. **Phase 3**: 迁移测试
4. **Phase 4**: 更新 Commands 和 AppState
5. **Phase 5**: 删除旧的 Registry/Service 文件

## Related

- ADR 0003: Provider Architecture - 定义了 Provider 模式和 Registry 职责
- `/improve-codebase-architecture` 架构审查 - 识别出这个深化机会

## References

- *A Philosophy of Software Design* by John Ousterhout - 深度模块原则
- Rust API Guidelines - 所有权和并发模式
- 架构审查报告: `/var/folders/.../architecture-review-zh-20260613-225804.html`
