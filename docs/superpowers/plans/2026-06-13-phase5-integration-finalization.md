# Phase 5: HistoryDb and Integration Finalization Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement SQLite history database, integrate with all services, add history UI, perform cross-platform testing, and finalize the refactor.

**Architecture:** Complete Infrastructure's HistoryDb, integrate into TranslationService and OcrService, add history queries, implement cleanup policies.

**Tech Stack:** Rust, rusqlite, existing infrastructure and application layers

**Duration:** 2 days

**Prerequisites:** Phase 1-4 completed (Infrastructure, Translation, OCR, Capture)

---

## File Structure

### New Files to Create

**Infrastructure - Storage:**
- `src-tauri/src/infrastructure/storage/history_db.rs`
- `src-tauri/src/infrastructure/storage/history_db_test.rs`

**Application - Services:**
- `src-tauri/src/application/services/history_cleanup.rs`

**Commands:**
- `src-tauri/src/commands/history_commands.rs`

**Tests:**
- `src-tauri/tests/history_integration_test.rs`
- `src-tauri/tests/cross_platform_test.rs`

### Files to Modify

- `src-tauri/src/application/providers/translation/service.rs` - Add history recording
- `src-tauri/src/application/providers/ocr/service.rs` - Add history recording
- `src-tauri/src/lib.rs` - Add HistoryDb to AppState
- `src-tauri/src/commands/mod.rs` - Add history commands
- `src-tauri/src/infrastructure/storage/mod.rs` - Export HistoryDb

### Files to Delete

- `src-tauri/src/history.rs` - Old history module
- `src-tauri/src/config/` - Old config module (fully replaced)
- `src-tauri/src/language/` - Keep for now, will be refactored later
- `src-tauri/src/hotkeys/` - Old hotkey module (replaced by HotkeyService)
- `src-tauri/src/utils/` - Review and migrate or delete

---

## Summary of Tasks

**Task 1:** HistoryDb Implementation - SQLite database for translation and OCR history
**Task 2:** Schema and Migrations - Database tables and migration system
**Task 3:** Integrate History into Services - Add recording to TranslationService and OcrService
**Task 4:** History Queries - Search, filter, pagination
**Task 5:** History Cleanup Service - Auto-cleanup policies (time/count based)
**Task 6:** History Commands - Tauri commands for history UI
**Task 7:** Update AppState - Add HistoryDb initialization
**Task 8:** Integration Tests - Test full history flow
**Task 9:** Cross-Platform Testing - Test on macOS/Windows/Linux
**Task 10:** Documentation and Cleanup - Update docs, remove old modules

**Estimated Time:** 2 days

---

## Phase 5 Completion Checklist

- [ ] HistoryDb implemented with SQLite
- [ ] Database schema and migrations working
- [ ] Translation history recording working
- [ ] OCR history recording working
- [ ] History query commands working (search, filter, pagination)
- [ ] History cleanup service implemented
- [ ] Auto-cleanup policies working (time-based and count-based)
- [ ] History commands implemented
- [ ] AppState updated with HistoryDb
- [ ] Integration tests pass
- [ ] Cross-platform tests pass (macOS/Windows/Linux)
- [ ] Old modules deleted (history.rs, config/, hotkeys/)
- [ ] Documentation updated
- [ ] All tests pass
- [ ] Application runs successfully on all platforms

**Architecture Refactor Complete!**

---

## Final Verification Checklist

### Architecture Quality
- [ ] Dependency direction is correct (Commands → Application → Domain, Infrastructure)
- [ ] Provider vertical slices are independent (translation/ocr/tts)
- [ ] Platform differences isolated in Infrastructure
- [ ] Dependency injection implemented (HttpClient, KeychainBackend, ScreenshotBackend)
- [ ] Test coverage > 80%

### Functionality
- [ ] Translation: Google/DeepL/Baidu working, multi-select, concurrent
- [ ] OCR: Tesseract/Baidu working, single-select
- [ ] Screenshot: Capture, edit, save, copy working
- [ ] History: Translation and OCR history queryable
- [ ] Config: Stored in JSON and Keychain
- [ ] Hotkeys: Global hotkeys registered

### Cross-Platform
- [ ] macOS: All features working
- [ ] Windows: All features working (if available)
- [ ] Linux: All features working (if available)

### Performance
- [ ] Translation response < 2s (network normal)
- [ ] OCR recognition < 3s (Tesseract local)
- [ ] Screenshot trigger < 100ms
- [ ] History query < 100ms

**Total Estimated Time for All Phases:** 11-15 days (2-3 weeks)
