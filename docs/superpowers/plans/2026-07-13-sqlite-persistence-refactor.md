# SnapLingo SQLite 持久化重构方案

**目标：** 在产品尚未上线、无需兼容现有本地数据的前提下，将所有非敏感持久化数据统一到 `snaplingo.db`，将凭证保留在系统 Keychain，将截图和贴图文件保存在文件系统，并让 Rust 后端成为唯一业务数据源。

**技术栈：** Rust、Tauri 2、rusqlite、Serde、系统 Keychain、React、TypeScript、Zustand。

---

## 1. 前提与范围

### 前提

- 产品尚未上线，开发阶段产生的 `config.json`、`history.db` 和 localStorage 数据可以直接废弃。
- 不实现旧数据导入、兼容读取或一次性迁移。
- 从新的 SQLite `v1` 开始建立正式迁移机制，为上线后的数据库升级负责。
- 本次只重构持久化边界，不借机重写截图编辑器、Provider 运行时或其他无关业务逻辑。

### 包含范围

- 通用设置、截图设置、编辑器设置、颜色预设、快捷键、翻译设置和 Provider 非敏感配置。
- 翻译/OCR 历史、收藏、备注和标签。
- 截图收藏与需要跨重启恢复的贴图资产元数据。
- 数据库初始化、事务、版本迁移、默认值、恢复默认和多窗口同步。
- 清除前端重复持久化，让 Zustand 仅作为运行时缓存。

### 不包含范围

- 不迁移现有 `config.json`、`history.db` 或 WebView localStorage。
- 不把 API Key、Token、密码写入 SQLite。
- 不把截图原图作为大型 BLOB 写入 SQLite。
- 不引入 ORM、数据库连接池、云同步或配置审计历史。
- 不提前创建尚未接入业务的空表；资产表随对应功能阶段加入。

---

## 2. 目标架构

```text
React / Zustand（内存状态）
          │
          ▼
Tauri Commands + Events
          │
          ▼
Application Services
          │
          ├── Settings Repository ──┐
          ├── Provider Repository ──┤
          ├── History Repository ───┼── SQLite: snaplingo.db
          └── Asset Repository ─────┘
                    │
                    ├── Keychain：API Key / Token
                    └── assets/：截图和贴图文件
```

持久化目录：

```text
平台应用数据目录/
├── snaplingo.db
└── assets/
    ├── screenshots/
    ├── pinned/
    └── thumbnails/
```

核心边界：

| 数据 | 持久化介质 | 说明 |
|---|---|---|
| 通用、截图、编辑器、快捷键、翻译设置 | SQLite `settings` 表 | 按 namespace 保存类型化 JSON |
| Provider 非敏感配置 | SQLite `settings` 表 | 包括定义、顺序、启用状态和 Prompt 策略 |
| API Key、Token、密码 | 系统 Keychain | 使用稳定 Provider ID 作为凭证键的一部分 |
| 翻译/OCR 历史、收藏、备注、标签 | SQLite 关系表 | 支持分页、筛选、排序和稳定关联 |
| 截图、贴图、缩略图 | 文件系统 | SQLite 只保存相对路径及元数据 |
| 设置页标签、临时草稿、最近选区 | localStorage 或内存 | 必须是丢失后不影响核心数据的 UI 状态 |

---

## 3. SQLite 基础设计

### 3.1 数据库连接

创建一个共享数据库连接对象，由不同 Repository 共享，但不要让一个巨大的 `AppDatabase` 直接实现全部业务接口。

首版使用一个 `Mutex<rusqlite::Connection>`，所有事务保持短小。当前阶段不引入连接池；若未来搜索或并行读取成为性能瓶颈，再增加独立读取连接。

数据库打开后执行：

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
```

数据库核心只负责：

- 打开连接。
- 设置 PRAGMA。
- 执行迁移。
- 提供受控的 `with_connection` 和 `with_transaction`。
- 将 rusqlite 错误转换为应用错误。

### 3.2 数据库版本

使用两层版本：

- `PRAGMA user_version` 管理 SQL 表结构。
- `payload_version` 管理各配置 namespace 的 JSON 结构。

从 `v1` 开始维护顺序迁移函数：

```text
v0 → v1：创建 settings 和首版历史表
v1 → v2：未来新增资产表
v2 → v3：未来修改历史结构
```

迁移要求：

- 每个版本只负责一步变更。
- 一次启动可以连续执行多个版本。
- 每一步都在事务中执行。
- 事务成功后才更新 `user_version`。
- 未知的更高版本必须拒绝打开，不能删除或重建数据库。

---

## 4. 配置数据模型

### 4.1 Settings 表

```sql
CREATE TABLE settings (
    namespace       TEXT PRIMARY KEY,
    payload_version INTEGER NOT NULL,
    payload_json    TEXT NOT NULL CHECK(json_valid(payload_json)),
    revision        INTEGER NOT NULL DEFAULT 1,
    updated_at      INTEGER NOT NULL
);
```

首版 namespace：

| Namespace | 内容 |
|---|---|
| `general` | 语言、主题、开机启动 |
| `capture` | 保存路径、格式、质量、命名规则、自动复制 |
| `editor` | 默认工具、粗细、字号、预设颜色、马赛克参数 |
| `hotkeys` | 截图、翻译、OCR 等全部快捷键 |
| `translation` | 默认源语言、目标语言和翻译行为 |
| `providers` | Provider 定义、顺序、启用状态和 Prompt 策略，不包含凭证 |

不要把每个设置开关设计成 SQL 字段。设置项通常整组加载且不需要 SQL 查询，按 namespace 保存类型化 JSON 可以避免每增加一个开关就修改表结构。

### 4.2 默认值与恢复默认

- 默认值只定义在 Rust Domain 类型中。
- namespace 不存在时返回对应 Rust 默认值，不主动写入数据库。
- 首次修改时创建 namespace 行。
- 恢复整个 namespace 时删除该行，并返回 Rust 默认值。
- 恢复单个字段时由后端把该字段设置为当前 Rust 默认值，再原子写回。
- 动态默认值，例如默认截图目录，由 Application Service 计算，不写死在 SQL migration 中。

### 4.3 更新与并发

禁止保留“前端读取整个快照 → 修改一个字段 → 保存整个快照”的持久化流程。

更新分为两类：

1. 整组替换：请求携带 `expected_revision`，只允许更新匹配的版本。
2. 具体操作：例如新增颜色、删除颜色、修改单个快捷键，由 Repository 在事务中读取、修改并写回。

成功写入后：

- `revision` 增加 1。
- 返回最新类型化配置。
- 事务提交后发送 Tauri 变更事件。
- 其他 WebView 根据 revision 更新或重新加载。

过期写不能静默覆盖，应返回冲突错误及当前 revision。

---

## 5. 历史、收藏和标签模型

使用统一历史主键，避免翻译历史和 OCR 历史各自使用相同自增 ID 后发生删除或收藏关联冲突。

建议结构：

```text
history_records
├── id
├── kind: translation | ocr
├── created_at
├── favorite
└── note

translation_history
└── history_id → history_records.id

ocr_history
└── history_id → history_records.id

tags
history_tags
```

设计原则：

- `history_records` 保存所有历史共有字段。
- 子表保存翻译或 OCR 专属字段。
- 不需要单独查询的翻译 Provider 结果数组可以继续保存为 JSON。
- 收藏和备注直接属于历史记录，不再由前端 localStorage 维护。
- 标签名称唯一，关联表使用联合主键防止重复关联。
- 所有外键明确设置删除策略并开启 `foreign_keys`。
- 初版普通搜索可以继续使用 `LIKE`；数据规模证明需要后再增加 FTS5。

---

## 6. 图片和贴图资产

图片文件不进入 settings JSON，也不作为大型 BLOB 存入 SQLite。

建议元数据表：

```text
assets
├── id
├── kind
├── relative_path
├── content_hash
├── mime_type
├── width
├── height
└── created_at

pin_groups
pinned_items
```

文件写入流程：

1. 在目标目录写临时文件。
2. 完成编码后原子 rename 为正式文件。
3. 在事务中写入 SQLite 元数据。
4. 数据库写入失败时删除刚生成的文件。

文件删除流程：

1. 事务删除或标记数据库记录。
2. 提交成功后删除实际文件。
3. 删除文件失败时记录日志，由后续孤儿文件清理任务处理。

数据库保存相对路径，应用数据目录由系统路径服务统一解析。

---

## 7. Provider 与凭证边界

Provider namespace 可以保存：

- 自定义 Provider ID、名称、协议、Endpoint 和模型。
- Provider 显示顺序。
- 启用状态。
- 推理级别和 Prompt 策略。
- OCR Provider 选择。

不能保存：

- API Key。
- Token。
- 密码或其他凭证字段。

Provider ID 必须稳定，Keychain 键使用 Provider ID，例如：

```text
provider:{provider_id}:api_key
provider:{provider_id}:credential:{field_name}
```

SQLite 与 Keychain 无法组成真正的跨介质事务，因此 Provider 新增、更新和删除必须保留补偿逻辑：

- 写 Keychain 成功、SQLite 失败：恢复或删除刚写入的凭证。
- SQLite 更新成功、运行时重建失败：恢复 SQLite 与 Keychain 快照。
- 删除 Provider 失败：不能留下无法关联的凭证或半删除的 Provider。

---

## 8. Rust 模块重构

### 8.1 目标文件结构

新建：

```text
src-tauri/src/infrastructure/storage/
├── database/
│   ├── mod.rs
│   ├── connection.rs
│   ├── migrations.rs
│   ├── settings.rs
│   ├── providers.rs
│   ├── history.rs
│   └── assets.rs
└── keychain/
```

建议的具体 Repository：

- `SqliteSettingsRepository`
- `SqliteHotkeyRepository`
- `SqliteProviderRepository`
- `SqliteHistoryRepository`
- `SqliteAssetRepository`

各 Repository 共享同一个数据库连接对象，业务接口仍由 Application 层定义，Infrastructure 只负责实现。

### 8.2 需要删除或替换

- 删除 `src-tauri/src/infrastructure/storage/config_file.rs`。
- 删除 `config_file_test.rs`。
- 将 `history_db.rs` 拆入统一 database 模块。
- 删除 `get_config_path` 和 `get_history_db_path`。
- 新增唯一的 `get_database_path` 和资产目录函数。
- `composition.rs` 创建一次数据库实例并注入各 Repository。
- 测试中的 `ConfigFile::new_temp()` 改为共享的内存数据库测试夹具。
- 删除 Rust 端扫描 WebKit localStorage 的兼容逻辑。

### 8.3 Application Store 接口

现有接口偏向完整快照读写：

```text
load_settings
save_settings
load_hotkeys
save_hotkeys
```

目标接口应表达原子业务操作：

```text
load_settings
update_general_settings
update_capture_settings
update_editor_settings
replace_annotation_colors
add_annotation_color
update_annotation_color
delete_annotation_color
update_hotkey
reset_hotkey
reset_hotkey_category
reset_settings_namespace
```

Application Service 负责输入校验、默认值和规范化；Repository 负责事务、序列化、revision 和持久化。

---

## 9. 前端重构

Zustand Store 仅作为后端数据的运行时缓存。

需要删除：

- `settingsConfigStore.ts` 中的旧 localStorage 配置迁移。
- `providerStore.ts` 对 Provider 业务数据的 `persist`。
- `historyStore.ts` 对历史、收藏、备注和标签的 `persist`。
- 前端保存的快捷键默认快照。
- 任何与 Rust 后端重复的设置副本。

可以保留 localStorage：

- 设置页当前主标签和子标签。
- 尚未提交的临时草稿。
- 有数量上限的最近截图选区。
- 其他明确标记为可丢失的 UI 状态。

所有持久化操作遵循：

1. 前端调用 Runtime Port。
2. Tauri Adapter 调用后端命令。
3. 后端提交事务。
4. 后端返回最新数据。
5. Zustand 使用返回值替换缓存。

多窗口同步事件：

```text
settings-changed
hotkeys-changed
providers-changed
history-changed
```

事件至少携带 namespace 和 revision。只有事务成功提交后才能发出事件。

---

## 10. 分阶段实施计划

### Phase 1：数据库基础设施

- [ ] 新建统一 Database 模块。
- [ ] 建立 `v1` migration。
- [ ] 配置 PRAGMA。
- [ ] 提供内存数据库测试夹具。
- [ ] 修改 Composition，使一个数据库实例由多个 Repository 共享。
- [ ] 添加空数据库初始化、重复初始化和迁移回滚测试。

验证：

- 空数据库可以初始化到 `user_version = 1`。
- 重复打开不会重复建表或丢失数据。
- 失败迁移完整回滚。
- 多个 Repository 使用同一个数据库实例。

### Phase 2：设置、快捷键和 Provider 配置

- [ ] 实现 settings namespace 表读写。
- [ ] 实现 payload version 和 revision。
- [ ] 改造 Settings、Hotkey、Provider Store 接口为原子操作。
- [ ] 把恢复默认操作移到 Rust 后端。
- [ ] Provider 非敏感配置迁入 providers namespace。
- [ ] 保持 Keychain 和凭证补偿逻辑。
- [ ] 删除 ConfigFile、config.json 路径和相关测试。
- [ ] 删除全部旧 localStorage 配置迁移代码。

验证：

- 首次读取返回 Rust 默认值且数据库无冗余默认行。
- 修改后重新打开数据库仍能读取。
- 恢复整个 namespace 后对应数据库行消失。
- 不同 namespace 并发更新互不覆盖。
- 同一 namespace 的过期 revision 写入被拒绝。
- 数据库中搜索不到测试 API Key。

### Phase 3：前端单一数据源

- [ ] 移除设置、Provider 和快捷键的业务 localStorage 副本。
- [ ] 所有 Store 启动时从 Rust 后端 hydrate。
- [ ] Store 使用后端返回值更新状态。
- [ ] 接入配置变更事件和 revision 判断。
- [ ] 修复快捷键恢复默认流程，不再从用户快照推断默认值。

验证：

- 清空 localStorage 不影响任何核心配置。
- 多 WebView 修改设置后能够同步。
- 前端不会提交旧快照覆盖新值。
- UI 导航状态仍可按原有需求保留。

### Phase 4：历史、收藏、备注和标签

- [ ] 将独立 HistoryDatabase 合并到统一数据库。
- [ ] 建立统一历史主键和类型子表。
- [ ] 把收藏、备注和标签迁入 SQLite。
- [ ] 删除历史业务数据的 Zustand persist。
- [ ] 删除独立 `history.db` 路径和初始化逻辑。
- [ ] 修正删除接口，使其使用唯一 history ID。

验证：

- 翻译和 OCR 历史不会发生 ID 冲突或交叉删除。
- 收藏、备注和标签重新启动后仍然存在。
- 分页、搜索、删除和清空行为正确。
- 外键和级联删除符合定义。

### Phase 5：截图和贴图资产

- [ ] 在功能接入时新增 assets migration。
- [ ] 实现文件型 AssetStore。
- [ ] 截图收藏使用文件加 SQLite 元数据。
- [ ] 如果贴图要求跨重启，则保存分组、位置、透明度、层级和可见状态。
- [ ] 添加文件写入失败、数据库失败和孤儿文件清理测试。

验证：

- SQLite 中不包含原图大型 BLOB。
- 资产路径为相对路径。
- 元数据写入失败不会留下新文件。
- 删除资产不会留下有效数据库记录指向不存在文件。

### Phase 6：清理与完整验证

- [ ] 删除失效模块、导出、兼容函数和测试夹具。
- [ ] 更新 Infrastructure README 和架构依赖测试。
- [ ] 搜索并确认不存在 `ConfigFile`、`config.json`、独立 `history.db` 引用。
- [ ] 搜索并确认业务 Store 不再使用 Zustand persist。
- [ ] 检查 SQLite 内容不包含凭证。
- [ ] 运行 Rust、前端测试和前端构建，不启动开发版。

验证命令：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm test
npm run build
```

---

## 11. 文件级触点

后端主要修改：

- `src-tauri/src/lib.rs`
- `src-tauri/src/composition.rs`
- `src-tauri/src/composition/history_runtime.rs`
- `src-tauri/src/infrastructure/system/paths.rs`
- `src-tauri/src/infrastructure/storage/mod.rs`
- `src-tauri/src/infrastructure/storage/config_file.rs`
- `src-tauri/src/infrastructure/storage/history_db.rs`
- `src-tauri/src/application/settings/store.rs`
- `src-tauri/src/application/settings/configuration.rs`
- `src-tauri/src/application/hotkeys/store.rs`
- `src-tauri/src/application/hotkeys/configuration.rs`
- `src-tauri/src/application/providers/config_store.rs`
- `src-tauri/src/application/providers/configuration.rs`
- `src-tauri/src/application/history/repository.rs`
- `src-tauri/src/commands/settings_commands.rs`
- `src-tauri/src/commands/hotkey_commands.rs`

前端主要修改：

- `src/stores/settingsConfigStore.ts`
- `src/stores/hotkeyConfigStore.ts`
- `src/stores/providerStore.ts`
- `src/stores/historyStore.ts`
- `src/application/settings/ports.ts`
- `src/application/settings/runtime.ts`
- `src/platform/tauri/settings.ts`
- `src/platform/tauri/appEvents.ts`

注意：当前工作区已有截图编辑器相关未提交修改。实施时应按 Phase 拆分小批次变更，不修改与持久化无关的截图渲染和交互文件。

---

## 12. 最终验收标准

重构完成必须满足：

- [ ] 只有一个 `snaplingo.db` 承载非敏感持久化数据。
- [ ] 不再创建或读取 `config.json` 和独立 `history.db`。
- [ ] 不包含旧配置或 localStorage 数据迁移逻辑。
- [ ] 默认值完全由 Rust Domain 定义。
- [ ] 恢复默认由 Rust 后端执行。
- [ ] 多窗口更新不会静默覆盖新配置。
- [ ] API Key、Token 和密码无法在 SQLite 中检索到。
- [ ] 图片文件不作为大型 BLOB 存储。
- [ ] 清空 localStorage 不影响核心配置和业务记录。
- [ ] 设置、快捷键、Provider、历史和收藏通过重启持久化测试。
- [ ] 所有 migration、Repository、IPC 和前端 Store 测试通过。
- [ ] `cargo test`、`npm test`、`npm run build` 全部通过。

推荐执行顺序：**数据库基础 → 设置/快捷键/Provider → 前端单一数据源 → 历史 → 资产**。每个 Phase 独立完成并通过验证后再进入下一阶段。

---

## 13. 实施状态（2026-07-13）

已完成：

- 一个共享的 `snaplingo.db`、SQLite PRAGMA、`user_version = 1` migration、内存测试数据库和 migration 回滚测试。
- 设置、快捷键、Provider 非敏感配置迁入 SQLite；凭证继续留在 Keychain。
- 删除 `ConfigFile`、`config.json`、独立 `history.db` 及其路径和兼容读取逻辑。
- 前端设置、Provider、历史业务 Store 不再持久化业务副本；`settingsStore` 只保留可丢失的设置页导航状态。
- 设置、快捷键、Provider、历史的跨 WebView 变更事件与后端重新加载。
- 快捷键默认值和单项/整组恢复由 Rust 后端负责，并同步更新全局快捷键注册。
- 翻译/OCR 历史使用统一主键，收藏、备注、标签写入 SQLite；自动产生的新历史在提交后广播变更。
- 自定义 Provider 凭证边界测试直接检查 SQLite payload，确认 API Key 只写入 Keychain。
- Infrastructure README 和架构依赖测试已更新。

当前实现的有意收敛：

- 设置沿用一个类型化 `settings` 快照 namespace；更新命令在后端按具体业务操作串行执行，避免前端直接保存整份旧快照。
- 跨窗口事件作为失效通知，接收方重新读取后端权威状态；目前不向前端暴露 revision 冲突协议。
- 当前产品没有“截图收藏”入口，贴图也明确是进程内临时状态，没有跨重启位置、透明度、层级或可见状态需求。因此不预建 assets 空表；出现实际持久化入口时再新增下一版 migration 和文件型 AssetStore。

验证结果：

- `cargo test --manifest-path src-tauri/Cargo.toml`：448 个单元测试及全部集成测试通过。
- `npm run build`：通过。
- `npm test`：732/732 通过；旧的 `capture-workspace/productionWiring.test.ts` 结构断言已更新为当前颜色预设接口契约。
