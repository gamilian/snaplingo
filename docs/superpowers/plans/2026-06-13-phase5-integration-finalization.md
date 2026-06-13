# Phase 5: HistoryDb and Integration Finalization Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement SQLite history database, integrate with all services, add history queries, perform cross-platform testing, and finalize the refactor.

**Architecture:** Complete Infrastructure's HistoryDb, integrate into TranslationService and OcrService, add history queries, implement cleanup policies.

**Duration:** 2 days

**Prerequisites:** Phase 1-4 completed

---

## Task 1: HistoryDb Schema Design

Define SQLite schema:
- translation_history table
- ocr_history table
- Indexes for queries

---

## Task 2: HistoryDb Implementation

**Files:**
- Create: `src-tauri/src/infrastructure/storage/history_db.rs`

Implement:
- Database creation and migrations
- add_translation_entry(request, results)
- add_ocr_entry(result)
- query methods with pagination

---

## Task 3: Integrate History into TranslationService

**Files:**
- Modify: `src-tauri/src/application/providers/translation/service.rs`

Add history recording after successful translation.

---

## Task 4: Integrate History into OcrService

**Files:**
- Modify: `src-tauri/src/application/providers/ocr/service.rs`

Add history recording after successful OCR.

---

## Task 5: History Query Commands

**Files:**
- Create: `src-tauri/src/commands/history_commands.rs`

Implement:
- query_translation_history(filter, pagination)
- query_ocr_history(filter, pagination)
- delete_history_entry(id)
- clear_history(type)

---

## Task 6: History Cleanup Service

**Files:**
- Create: `src-tauri/src/application/services/history_cleanup.rs`

Implement auto-cleanup:
- By age (default 30 days)
- By count (default 1000 entries)

---

## Task 7: Update AppState with HistoryDb

**Files:**
- Modify: `src-tauri/src/lib.rs`

Add history_db to AppState and pass to services.

---

## Task 8: Integration Tests

Test complete history flow.

---

## Task 9: Cross-Platform Testing

Test on macOS/Windows/Linux (if available).

---

## Task 10: Final Cleanup and Documentation

Delete old modules:
- src-tauri/src/history.rs
- src-tauri/src/config/
- src-tauri/src/utils/

Update README and documentation.

---

## Phase 5 Completion Checklist

- [ ] HistoryDb implemented
- [ ] Translation history recording working
- [ ] OCR history recording working
- [ ] History queries working
- [ ] Cleanup policies working
- [ ] History commands implemented
- [ ] AppState updated
- [ ] Integration tests pass
- [ ] Cross-platform tests pass
- [ ] Old modules deleted
- [ ] Documentation updated

**Architecture Refactor Complete!**

**Total Duration:** 13-17 days (adjusted from initial 11-15 days)
