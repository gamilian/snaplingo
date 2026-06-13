# Phase 4: Capture Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement screenshot capture functionality (Screenshot Mode) with platform-adapted backends, save/load operations, and hotkey integration.

**Architecture:** Application service layer using Infrastructure's ScreenshotBackend. Includes capture, save, and hotkey management.

**Tech Stack:** Rust, core-graphics (macOS), Windows GDI, xcb (Linux), existing infrastructure

**Duration:** 2-3 days

**Prerequisites:** Phase 1 (Infrastructure with Screenshot backend placeholder) completed

---

## File Structure

### New Files to Create

**Application - Services:**
- `src-tauri/src/application/services/mod.rs`
- `src-tauri/src/application/services/capture_service.rs`
- `src-tauri/src/application/services/hotkey_service.rs`

**Commands:**
- `src-tauri/src/commands/capture_commands.rs`

**Tests:**
- `src-tauri/src/application/services/capture_service_test.rs`
- `src-tauri/tests/capture_integration_test.rs`

### Files to Modify

- `src-tauri/src/infrastructure/system/screenshot/macos.rs` - Complete implementation
- `src-tauri/src/infrastructure/system/screenshot/windows.rs` - Complete implementation  
- `src-tauri/src/infrastructure/system/screenshot/linux.rs` - Complete implementation
- `src-tauri/src/lib.rs` - Add CaptureService to AppState
- `src-tauri/src/commands/mod.rs` - Add capture commands

### Files to Delete

- `src-tauri/src/capture.rs` - Old capture module (after migration)

---

## Summary of Tasks

**Task 1:** Complete Screenshot Backend Implementations - Finish macOS/Windows/Linux screenshot capture
**Task 2:** CaptureService - Business logic for capture, save, filename generation
**Task 3:** HotkeyService - Global hotkey registration and management
**Task 4:** Capture Commands - Tauri commands for screenshot operations
**Task 5:** Update AppState - Integrate CaptureService and HotkeyService
**Task 6:** Integration Test - Test screenshot capture flow
**Task 7:** Frontend Canvas Editor - React component for screenshot editing
**Task 8:** Hotkey Registration - Connect global hotkeys to capture
**Task 9:** Save and Copy - Implement save to file and copy to clipboard
**Task 10:** Delete Old Capture Module - Remove legacy code

**Estimated Time:** 2-3 days

---

## Phase 4 Completion Checklist

- [ ] Screenshot backends fully implemented (macOS/Windows/Linux)
- [ ] CaptureService implemented with save/load
- [ ] HotkeyService implemented with platform abstraction
- [ ] Capture commands implemented
- [ ] AppState updated with Capture components
- [ ] Integration test passes
- [ ] Frontend Canvas editor working
- [ ] Global hotkeys registered and working
- [ ] Save to file working
- [ ] Copy to clipboard working
- [ ] Old capture module deleted
- [ ] All tests pass
- [ ] Application runs successfully

**Next Phase:** Phase 5 - HistoryDb and final integration

**Full implementation details:** Focus on completing Screenshot backend placeholders from Phase 1 and building application services on top.
