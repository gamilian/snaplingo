# Capture Window Adapter Design

> Date: 2026-06-19
> Status: Approved for phase 1 implementation

## Goal

Move capture-window runtime behavior out of the Tauri Command Module and into an Infrastructure Adapter, without changing screenshot behavior.

## Background

The current screenshot flow works, but `src-tauri/src/commands/capture_session_commands.rs` has become shallow. It is a Tauri Command Module, yet its Interface includes window visibility, capture-window labels, transparent window options, workspace visibility, focus, bounds, and settle-delay rules.

This creates poor Locality for macOS screenshot issues. When the capture overlay flashes, hides the wrong window, fails to restore a window, or needs a transparent-window adjustment, the maintainer has to read screenshot policy and Tauri window mechanics together.

ADR-0003 already says platform adaptation belongs in Infrastructure. This phase applies that decision to the capture window.

## Scope

Create a new Infrastructure Module:

```text
src-tauri/src/infrastructure/system/capture_window/
```

This Module owns the concrete Tauri Adapter for:

- capture-window label and URL construction
- capture-window bounds from frozen monitor snapshots
- capture-window creation and reuse
- transparent, always-on-top, workspace, focus, shadow, and taskbar options
- planning which app windows to hide before taking the frozen snapshot
- restoring hidden app windows after cancel/open failure
- hide settle delay

The Command Module remains the Tauri command entry point. It should call the Adapter and keep only request/response coordination.

## Non-Goals

This phase does not change:

- Capture Session lifecycle policy
- image composition or annotation rendering
- OCR behavior
- copy/save/pin output behavior
- successful-selection history behavior
- frontend ScreenshotSession state
- pinned-image window behavior, except where type imports need to remain compiling

## Module Shape

```text
Commands
  capture_session_commands.rs
    -> open_capture_window_for_mode()
    -> cancel_capture_session()
    -> restore_capture_snapshot_windows_for_session()

Infrastructure
  system/capture_window/
    mod.rs
    backend.rs
    tauri.rs
```

`backend.rs` contains small platform/runtime-neutral planning helpers and request/result types.

`tauri.rs` contains the concrete Adapter that receives a `tauri::AppHandle` and performs WebviewWindow operations.

`mod.rs` exports the public Interface used by Commands.

## Behavior To Preserve

- Existing capture window label remains `capture`.
- Existing capture window URL format remains `index.html?window=capture&mode=<mode>&sessionId=<id>`.
- Existing mode normalization remains:
  - `screenshot`
  - `screenshot-ocr`
  - `screenshot-translate`
- Reusing an existing capture window still resizes, repositions, and emits `hotkey-triggered`.
- New capture windows remain undecorated, always-on-top, visible on all workspaces, transparent, initially hidden, skipped from taskbar, focused, and shadowless.
- Before taking the snapshot, only the capture window is hidden.
- Hidden capture windows are not restored through the generic restore path.
- A non-empty hidden-window list still gets a 100ms settle delay.
- Open failure still restores hidden windows and cancels the session.

## Testing

Move the existing pure tests for capture-window planning from `capture_session_commands.rs` into the new Infrastructure Module:

- capture window URL construction
- capture window bounds union
- labels to hide before snapshot
- labels to restore after snapshot
- settle delay

The concrete Tauri Adapter is not unit-tested in this phase. Its behavior is covered by preserving the public flow and running the existing Rust and frontend tests that exercise the screenshot flow.

## Risks

- Moving URL helpers can break frontend routing if the exact string changes.
- Moving label helpers can break restore behavior and reintroduce hidden-window bugs.
- Moving bounds calculation can break multi-monitor capture placement.

The implementation should be mechanical and test-first: move tests, make them fail due to missing Module, then move the minimal code needed to pass.

## Follow-Up

After this phase, deepen the Capture Session workflow Module so screenshot lifecycle policy no longer lives in Commands.
