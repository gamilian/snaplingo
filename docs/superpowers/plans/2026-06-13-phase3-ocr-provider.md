# Phase 3: OCR Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add OCR capability with provider pattern (Tesseract local + Baidu OCR remote), single-select registry.

**Architecture:** Vertical slice for OCR Provider. Similar to Translation but with single-select registry. Includes Trait, Registry, Service, and two implementations.

**Tech Stack:** Rust, tesseract-rs (local), async-trait, existing infrastructure

**Duration:** 3-4 days

**Prerequisites:** Phase 1 (Infrastructure) and Phase 2 (Translation) completed

---

## File Structure

### New Files to Create

**Application - OCR Provider:**
- `src-tauri/src/application/providers/ocr/mod.rs`
- `src-tauri/src/application/providers/ocr/trait_def.rs`
- `src-tauri/src/application/providers/ocr/registry.rs`
- `src-tauri/src/application/providers/ocr/service.rs`
- `src-tauri/src/application/providers/ocr/impls/mod.rs`
- `src-tauri/src/application/providers/ocr/impls/tesseract.rs`
- `src-tauri/src/application/providers/ocr/impls/baidu_ocr.rs`

**Commands:**
- `src-tauri/src/commands/ocr_commands.rs`

**Tests:**
- `src-tauri/src/application/providers/ocr/registry_test.rs`
- `src-tauri/src/application/providers/ocr/service_test.rs`
- `src-tauri/tests/ocr_integration_test.rs`

### Files to Modify

- `src-tauri/src/lib.rs` - Add OCR components to AppState
- `src-tauri/src/commands/mod.rs` - Add OCR commands
- `src-tauri/Cargo.toml` - Add tesseract dependencies

### Files to Delete

- `src-tauri/src/ocr/` - Old OCR module (after migration verification)

---

## Summary of Tasks

**Task 1:** OcrProvider Trait - Define trait with recognize method
**Task 2:** OcrRegistry (Single-Select) - Registry allowing only one active provider
**Task 3:** OcrService - Business logic with history recording
**Task 4:** Tesseract Provider - Local OCR using tesseract-rs
**Task 5:** Baidu OCR Provider - Remote OCR using Baidu API
**Task 6:** OCR Commands - Tauri commands for OCR operations
**Task 7:** Update AppState - Integrate OCR components
**Task 8:** Integration Test - End-to-end OCR test
**Task 9:** Frontend Integration - Connect to React UI
**Task 10:** Delete Old OCR Module - Remove legacy code

**Estimated Time:** 3-4 days

---

## Phase 3 Completion Checklist

- [ ] OcrProvider trait defined
- [ ] OcrRegistry (single-select) implemented with tests
- [ ] OcrService implemented with tests
- [ ] Tesseract provider implemented (local)
- [ ] Baidu OCR provider implemented (remote)
- [ ] OCR commands implemented
- [ ] AppState updated with OCR components
- [ ] Integration test passes
- [ ] Frontend connected and verified
- [ ] Old ocr module deleted
- [ ] All tests pass
- [ ] Application runs successfully

**Next Phase:** Phase 4 - Capture Service (screenshot functionality)

**Full implementation details:** Similar structure to Phase 2, adapted for OCR's single-select pattern and image processing requirements.
