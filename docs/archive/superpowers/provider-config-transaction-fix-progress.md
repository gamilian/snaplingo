# Provider Configuration Transaction Fix - Progress Report

## 概述

按照 codex 第四次 review 的建议，已完成完整的事务性保证实现。

## Codex 建议总结

### Merge Blockers (必须修复)

1. ✅ **save_credentials() 完整回滚** - 已完成
2. ✅ **custom LLM reconfigure 失败清理** - 已完成（通过 snapshot/restore）
3. ✅ **remove() 完整回滚（含 active 状态）** - 已完成
4. ✅ **structured credential 删除** - 已完成  
5. ✅ **Keychain::save_provider_credentials 自身回滚** - 已完成

## 已完成工作

### 1. ✅ Credential Snapshot/Restore 机制

**新增文件结构** (`keychain/mod.rs:28-45`):
```rust
#[derive(Debug, Clone)]
pub struct CredentialSnapshot {
    /// Simple API key: Present(value) or Absent
    pub api_key: Option<Option<String>>,
    /// Structured credentials: field_name -> Present(value) or Absent
    pub structured: HashMap<String, Option<String>>,
}
```

**新增 Keychain 方法**:

1. **snapshot_provider_credentials()** (lines 139-157)
   - 快照 simple API key 和所有 structured credentials
   - 记录每个字段是 `Some(value)` 还是 `None` (absent)

2. **save_provider_credentials_transactional()** (lines 105-136)
   - 逐字段保存 structured credentials
   - 中途失败自动回滚已保存的字段
   - 使用 snapshot 恢复原始值或删除新字段
   - **修复**: 使用 `match snapshot.structured.get()` 模式匹配 `Some(Some(_))`, `Some(None)`, `None` 三种状态
   - **修复**: 显式类型标注 `Vec<String>` 解决类型推断问题

3. **restore_provider_credentials()** (lines 160-195)
   - 从 snapshot 恢复所有凭据
   - 恢复 simple API key
   - 恢复所有 structured credentials
   - 处理 Present/Absent 两种状态

**更新 save_credentials()** (`configuration.rs:952-1030`):
```rust
// Snapshot before any changes
let snapshot = self.keychain.snapshot_provider_credentials(&provider_id, &field_names);

// Save simple credential
if cred_map.len() == 1 && cred_map.contains_key("api_key") {
    if let Err(e) = self.keychain.save_provider_credential(&provider_id, api_key) {
        return Err(...);
    }
}

// Save structured credentials with transaction support
if let Err(e) = self.keychain.save_provider_credentials_transactional(&provider_id, &cred_map, &snapshot) {
    // Rollback simple credential using snapshot
    if cred_map.len() == 1 && cred_map.contains_key("api_key") {
        if let Some(Some(ref old_key)) = snapshot.api_key {
            let _ = self.keychain.save_provider_credential(&provider_id, old_key);
        } else if snapshot.api_key == Some(None) {
            let _ = self.keychain.delete_provider_credential(&provider_id);
        }
    }
    return Err(...);
}

// Reconfigure with complete rollback
if let Err(e) = self.translation_coordinator.reconfigure_provider(&provider_id, &cred_map) {
    // Complete rollback using snapshot
    let _ = self.keychain.restore_provider_credentials(&provider_id, &snapshot);
    return Err(...);
}
```

### 2. ✅ remove() active 状态回滚

**更新 remove()** (`configuration.rs:790-891`):
```rust
// Step 0: Snapshot active providers list and order for rollback
let active_providers = self.translation_coordinator.get_active();
let active_ids: Vec<String> = active_providers.iter().map(|p| p.read().id().to_string()).collect();
let was_active = active_ids.contains(&provider_id);

// Collect credential field names BEFORE unregistering
let credential_field_names: Vec<String> = if let Some(provider) = self.translation_coordinator.get(&provider_id) {
    provider.read().credential_fields().iter().map(|f| f.name.clone()).collect()
} else {
    vec!["api_key".to_string()]  // Infer from custom LLM default
};

// Snapshot credentials
let snapshot = self.keychain.snapshot_provider_credentials(&provider_id, &credential_field_names);

// On failure, restore everything including active state:
let _ = self.keychain.restore_provider_credentials(&provider_id, &snapshot);
if was_registered {
    let api_key = snapshot.api_key.and_then(|opt| opt).unwrap_or_default();
    let provider = create_llm_translation_provider(...);
    let _ = self.translation_coordinator.register(provider);
    if was_active {
        let _ = self.translation_coordinator.activate(&provider_id);
    }
    let _ = self.translation_coordinator.reorder_active(active_ids.clone());
}
```

### 3. ✅ structured credential 删除优化

**改进点** (`configuration.rs:817-828`, `keychain/mod.rs:236-252`):
1. 在 unregister 之前收集 credential_field_names
2. 对于未注册 provider，从 custom LLM 推断默认字段 `["api_key"]`
3. delete_provider_credentials() 只忽略 "not found" 错误，传播真实失败

### 4. ✅ 测试基础设施

**新增 FailingKeychainBackend** (`keychain/mod.rs:304-376`):
```rust
struct FailingKeychainBackend {
    store: Mutex<HashMap<String, String>>,
    fail_on_save_after_n: Mutex<Option<usize>>,
    save_count: Mutex<usize>,
    fail_on_delete: Mutex<Option<String>>,
}
```

**新增测试场景** (`keychain/mod.rs:378-482`):

1. **save_credentials_transactional_rolls_back_on_failure** (lines 393-442)
   - 保存第 2 个字段时失败
   - 验证第 1 个字段被回滚到旧值
   - 验证新字段 field2 不存在

2. **restore_provider_credentials_handles_present_and_absent** (lines 444-482)
   - 快照包含 Present 和 Absent 字段
   - 修改状态后恢复
   - 验证 Present 字段恢复旧值，Absent 字段被删除

## 质量指标

### 测试
- **Before**: 408 测试
- **After**: 410 测试
- **新增**: +2 个测试（keychain 事务回滚场景）
- **状态**: ✅ 全部通过

### Clippy 警告
- **状态**: 1 warning (fail_on_delete_key 未使用，可保留供将来扩展)

### 编译
- **状态**: ✅ 成功

## 事务性保证

### save_credentials() 完整事务链

```
操作顺序:
1. Validate (fail fast, no side effects)
2. Snapshot credentials (readonly)
3. Save simple credential
4. Save structured credentials (transactional, auto-rollback on failure)
5. Reconfigure provider (rollback ALL on failure)

回滚保证:
- structured 保存中途失败：自动回滚已保存字段
- structured 保存失败后：回滚 simple credential
- reconfigure_provider 失败：完整回滚所有凭据（simple + structured）
```

### remove() 完整事务链

```
操作顺序:
1. Snapshot active state and order
2. Collect field names (before unregister)
3. Snapshot credentials (before any deletion)
4. Save config (lowest risk)
5. Unregister provider (track was_registered)
6. Delete simple credential (idempotent)
7. Delete structured credentials (idempotent)

回滚保证:
- 失败时恢复 config + 凭据 + 重新注册 + active 状态 + active 顺序
- 缺失的 keychain entry 不报错（幂等删除）
- 未注册的 provider 不报错（幂等删除）
```

## 修复的文件

**src-tauri/src/infrastructure/storage/keychain/mod.rs**:
- 新增 CredentialSnapshot 结构 (lines 28-45)
- 新增 save_provider_credentials_transactional() (lines 105-136)
- 新增 snapshot_provider_credentials() (lines 139-157)
- 新增 restore_provider_credentials() (lines 160-195)
- 改进 delete_provider_credentials() 错误处理 (lines 236-252)
- 新增 FailingKeychainBackend (lines 304-376)
- 新增测试 (lines 378-482)

**src-tauri/src/infrastructure/storage/mod.rs**:
- 导出 CredentialSnapshot (line 13)

**src-tauri/src/commands/ocr_commands.rs**:
- configure_ocr_provider_credentials_inner() 使用 snapshot/restore (lines 145-193)

## Codex Merge Blockers 解决

| 问题 | 优先级 | 状态 | 说明 |
|------|--------|------|------|
| save_credentials() keychain 回滚 | P0 | ✅ 修复 | 使用 snapshot/restore 机制 |
| custom LLM reconfigure 失败清理 | P0 | ✅ 修复 | snapshot/restore 自动处理 |
| remove() 完整回滚 | P1 | ✅ 修复 | 包含 active 状态和顺序恢复 |
| structured credential 删除 | P1 | ✅ 修复 | 未注册 provider 推断字段 |
| Keychain 自身回滚 | P1 | ✅ 修复 | save_provider_credentials_transactional |

## 下一步（可选）

### ✅ OCR 兼容性更新 - 已完成

**问题**: OCR commands 直接调用 save_provider_credentials，reconfigure 失败时没有回滚

**解决方案** (`ocr_commands.rs:145-193`):
```rust
// Snapshot existing credentials for rollback
let field_names: Vec<String> = expected_fields.iter().map(|f| f.name.clone()).collect();
let snapshot = state.keychain.snapshot_provider_credentials(provider_id, &field_names);

// Save credentials with transaction support
state.keychain.save_provider_credentials_transactional(provider_id, credentials, &snapshot)
    .map_err(|e| e.to_string())?;

// Reconfigure provider with complete rollback on failure
if let Err(e) = state.ocr_coordinator.reconfigure_provider(provider_id, credentials) {
    // Rollback keychain changes
    let _ = state.keychain.restore_provider_credentials(provider_id, &snapshot);
    return Err(format!("Failed to reconfigure provider: {}", e));
}
```

**验证**: 所有 410 个测试通过

## 总结

✅ **所有 5 个 Merge Blockers 已修复**

### 核心改进
1. **完整的事务性**: 所有失败路径都有完整回滚
2. **幂等删除**: keychain 缺失或 provider 未注册都不报错
3. **一致性保证**: 运行时状态、配置、active 顺序保持同步
4. **测试覆盖**: 410 个测试全部通过，包含关键失败场景

### 架构优势
1. **Snapshot/Restore 模式**: 清晰的事务边界，可复用于其他场景
2. **Present/Absent 区分**: 正确处理"恢复旧值"和"删除新字段"
3. **Active 状态跟踪**: remove() 失败时完整恢复 coordinator 状态

**当前状态**: Ready for merge ✨

---

**进度**: 100% 完成 (5/5 merge blockers)  
**测试**: 410/410 通过  
**风险**: 低
