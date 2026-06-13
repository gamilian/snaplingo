# Phase 4: Capture Service - 进度跟踪

## 📋 执行状态

**开始时间:** 2026-06-13  
**策略:** macOS 优先实现，Windows/Linux 占位符  
**原因:** Token 预算限制 + 开发环境限制（仅 macOS）

---

## ✅ 已完成任务

**Phases 1-3:**
- ✅ Phase 1: Infrastructure Layer (100%)
- ✅ Phase 2: Translation Provider (100%)
- ✅ Phase 3: OCR Provider (100%)
- ✅ 所有问题已修复（Tesseract 文档等）

---

## 🎯 Phase 4 任务列表

### Task 1: Complete Screenshot Backend - macOS
- **状态:** ⏳ 待执行
- **优先级:** 高
- **文件:** `src-tauri/src/infrastructure/system/screenshot/macos.rs`
- **描述:** 使用 core-graphics 实现 macOS 截图

### Task 2: Screenshot Backend - Windows/Linux (Placeholder)
- **状态:** ⏳ 待执行
- **优先级:** 低（占位符）
- **文件:** windows.rs, linux.rs
- **描述:** 添加 TODO 注释，保持占位符结构

### Task 3: Complete Hotkey Backend - macOS
- **状态:** ⏳ 待执行
- **优先级:** 高
- **文件:** `src-tauri/src/infrastructure/system/hotkey/macos.rs`
- **描述:** 实现 macOS 全局热键注册

### Task 4: CaptureService
- **状态:** ⏳ 待执行
- **优先级:** 高
- **文件:** `src-tauri/src/application/services/capture_service.rs`
- **描述:** 截图服务，协调 Screenshot backend

### Task 5: HotkeyService
- **状态:** ⏳ 待执行
- **优先级:** 高
- **文件:** `src-tauri/src/application/services/hotkey_service.rs`
- **描述:** 热键服务，管理热键注册

### Task 6: Capture Commands
- **状态:** ⏳ 待执行
- **优先级:** 高
- **文件:** `src-tauri/src/commands/capture_commands.rs`
- **描述:** Tauri 命令接口

### Task 7: Update AppState
- **状态:** ⏳ 待执行
- **优先级:** 高
- **文件:** `src-tauri/src/lib.rs`
- **描述:** 集成服务到 AppState

### Task 8: Integration Test
- **状态:** ⏳ 待执行
- **优先级:** 中
- **文件:** `src-tauri/tests/capture_integration_test.rs`
- **描述:** 集成测试

### Task 9: Frontend Integration Documentation
- **状态:** ⏳ 待执行
- **优先级:** 中
- **文件:** 文档
- **描述:** 前端集成指南

### Task 10: Delete Old Modules
- **状态:** ⏳ 待执行
- **优先级:** 低
- **文件:** 删除旧代码
- **描述:** 清理旧的 capture/hotkey 模块

---

## 💰 Token 预算

- **当前使用:** 162k / 200k (81%)
- **剩余:** 38k
- **建议:** 完成关键任务后保存进度，新会话继续

---

## 🎯 当前会话目标

**优先完成（如果 token 允许）：**
1. Task 1: macOS Screenshot (关键)
2. Task 4: CaptureService (关键)
3. 保存进度文档

**延后到新会话：**
- Task 3: Hotkey 实现（复杂）
- Task 6-10: 其他任务

---

## 📝 实现注意事项

### macOS Screenshot 实现要点

**使用的 API:**
- `core_graphics::display::CGDisplay` - 屏幕捕获
- `image` crate - PNG 编码

**关键方法:**
- `capture_full_screen()` - 全屏截图
- `capture_region(x, y, width, height)` - 区域截图
- 返回 `Vec<u8>` PNG 数据

### CaptureService 设计

**职责:**
- 调用 Screenshot backend
- 保存到文件（使用 System Paths）
- 复制到剪贴板（可选）
- 返回截图数据

**依赖:**
- `Arc<dyn ScreenshotBackend>` - 平台截图实现
- `ConfigFile` - 配置
- `System Paths` - 保存路径

---

## 🚀 下次会话继续

**启动命令:**
```
继续执行 Phase 4: Capture Service
参考进度：docs/PHASE4_PROGRESS.md
从 Task 列表继续未完成任务
```

**检查清单:**
- [ ] 查看 PHASE4_PROGRESS.md
- [ ] 确认已完成任务
- [ ] 继续未完成任务
- [ ] 运行测试验证
- [ ] 提交代码

---

## 📊 整体进度

**已完成:**
- ✅ Phase 1: 100%
- ✅ Phase 2: 100%
- ✅ Phase 3: 100%
- ⏳ Phase 4: 0%

**预计剩余:**
- Phase 4: 2-3 小时（macOS 实现）
- Phase 5: 1-2 小时（清理集成）

**Total: 3-5 小时完成全部**
