# Frozen Pixel Capture Refactor Design

## Goal

Refactor the screenshot flow around one simple rule: freeze the current screen pixels in the backend before the capture overlay is shown. Settings, main, pinned-image, and other app windows are treated as normal windows: if they are visible on screen at the moment pixels are frozen, they are captured; if they are hidden or covered, they are not.

This replaces the current capture startup complexity that mixes layout sessions, snapshot hydration, delayed overlay reveal, and special handling for SnapLingo-owned windows.

## Product Semantics

- Settings Window is configuration UI only. It does not participate in screenshot workflow decisions.
- Main Window, Settings Window, and Pinned Image Windows are ordinary capture subjects.
- Capture Overlay is not a capture subject. It must not be visible until screen pixels have been frozen.
- Screenshot output uses the frozen pixels from the trigger moment, not the live desktop at selection completion time.
- Selection UI may use the live desktop behind a transparent overlay. The backend-frozen pixels remain the source of truth for preview, copy, save, pin, OCR, and translation.

## Current Friction

The current flow has accumulated special cases:

- Backend creates layout sessions and snapshot caches.
- Frontend waits for `hydrate_capture_session_snapshots` before revealing the overlay.
- Capture window visibility is coordinated across backend and frontend.
- Older paths still exist (`ScreenshotWorkflow`, `screenshot.html`, `screenshot_window_commands.rs`).
- Some historical documentation suggests hiding or restoring SnapLingo windows, even though the intended behavior is now to capture whatever is actually visible.

This makes the module interfaces shallow: callers need to know too much about hidden windows, snapshot hydration timing, overlay reveal timing, and session cleanup.

## Target Architecture

### Capture Session Service

Owns frozen screenshot sessions.

Responsibilities:

- `create_frozen_session()` captures monitor pixels into backend memory before returning.
- Captures monitor layout, scale factors, optional cursor snapshot, and window candidates.
- Stores frozen monitor PNG/raw pixel data under a `CaptureSessionId`.
- Exposes `get_session_metadata()` without monitor image base64 for fast frontend startup.
- Exposes render/OCR/output operations that use the frozen pixels.

Non-responsibilities:

- Opening, hiding, revealing, or focusing Tauri windows.
- Knowing whether Settings Window or Main Window is visible.
- Encoding full-screen monitor images for frontend reveal.

### Capture Overlay Host

New backend module around the `capture` Tauri webview.

Responsibilities:

- Create or reuse the `capture` webview.
- Keep it hidden until a frozen session exists.
- Reveal, hide, and configure native window behavior.
- Own macOS Space/fullscreen/window-level behavior.
- Own capture presentation lifecycle that is truly platform/window related.

Non-responsibilities:

- Freezing pixels.
- Rendering capture output.
- Deciding capture mode completion behavior.

Likely location:

```text
src-tauri/src/application/services/capture_overlay_host.rs
src-tauri/src/infrastructure/system/capture_window/*
```

Application layer should own the workflow-facing interface. Infrastructure remains the adapter for Tauri/AppKit calls.

### Capture Runtime

Deep module that coordinates screenshot startup.

Responsibilities:

```text
start_capture(mode)
→ create_frozen_session()
→ open_capture_overlay(mode, session_id, virtual_desktop_bounds)
```

It should be the only backend module that understands the ordered requirement: frozen pixels first, overlay second.

Likely interface:

```rust
pub async fn start_capture(&self, mode: CaptureMode) -> Result<CaptureLaunchView>;
```

The command layer should call this one interface instead of manually sequencing session creation, cache hydration, overlay opening, restoration, and cleanup.

### Frontend Capture Overlay

`ScreenshotSession` becomes a UI runtime for interaction, not a screenshot-loading runtime.

Responsibilities:

- Read `mode` and `sessionId`.
- Load session metadata only.
- Render transparent canvas selection overlay.
- Manage selection, annotations, shortcuts, and completion actions.
- Call `render_capture_output`, `output_capture`, or `run_capture_ocr` after selection.

Non-responsibilities:

- Waiting for full monitor screenshots before reveal.
- Deciding whether Settings Window or Main Window should be hidden.
- Treating full-screen monitor base64 as required startup state.

Magnifier and color sampling may still need pixels. They should request lazy pixel hydration after overlay reveal, or degrade gracefully until pixels are available.

### Settings Runtime

Settings remains a configuration module.

Responsibilities:

- Persist settings.
- Call runtime config update adapters when values change.
- Keep hotkey display values separate from backend accelerator values.

Non-responsibilities:

- Triggering screenshot-specific behavior because the Settings Window is open.
- Knowing whether capture sessions are active.

## Data Flow

### Screenshot / Edit

```text
Global hotkey
→ HotkeyRuntime dispatches CaptureMode::Screenshot
→ CaptureRuntime.start_capture(mode)
→ CaptureSessionService freezes pixels
→ CaptureOverlayHost opens/reveals capture overlay
→ ScreenshotSession loads metadata
→ User selects rect and edits annotations
→ render_capture_output(session_id, rect, annotations)
→ output_capture(copy/save/pin)
→ cancel/end session
```

### Screenshot Copy

```text
Global hotkey
→ frozen session
→ overlay reveal
→ user selects rect
→ output_capture(copy)
→ end session
```

### Screenshot OCR / Translation

```text
Global hotkey
→ frozen session
→ overlay reveal
→ user selects rect
→ run_capture_ocr(session_id, rect)
→ OCR result window or translation result window
→ end session
```

OCR uses the original frozen pixels without annotations.

## Window Rules

Capture subject rules:

- `main`: ordinary window.
- Settings Window: ordinary window.
- `pin-*`: ordinary windows.
- Other apps: ordinary windows.
- `capture`: never a subject; reveal only after frozen pixels exist.

This removes hidden-window bookkeeping for business windows.

The only acceptable `capture` window special cases are:

- It may be hidden before taking a new frozen screenshot if an old overlay is still visible.
- It may need native window-level and Space handling on macOS.
- It may need an Escape/cancel integration while active.

## Relation To Existing Lazy Capture Plan

`docs/superpowers/plans/2026-06-27-lazy-capture-overlay.md` describes a different direction: show a transparent overlay immediately and freeze pixels after selection.

This design intentionally chooses the opposite ordering:

```text
freeze pixels first
show overlay second
```

Reason: it preserves trigger-time screenshot semantics and removes the risk that the overlay itself becomes part of the captured pixels. It may add a small startup delay, but reveal only waits for backend pixel capture into memory, not for frontend full-screen base64 hydration.

## Migration Plan

### Phase 1: Lock Behavior With Tests

Add tests that define the new rules:

- Main, Settings, and Pinned Image windows are not hidden before capture.
- Existing `capture` overlay is the only app window planned for hiding before a new freeze.
- Capture overlay reveal happens only after session creation succeeds.
- Frontend reveal no longer requires full monitor image hydration.
- Output/OCR still uses frozen session pixels.

### Phase 2: Introduce Capture Overlay Host

Move capture window open/reuse/reveal/hide behavior out of `capture_session_commands.rs`.

Expected result:

- Commands no longer know Tauri/AppKit window details.
- Capture window behavior is testable through a small interface.
- Platform-specific behavior remains in `infrastructure/system/capture_window`.

### Phase 3: Simplify Session Creation

Replace layout-first startup with frozen-session-first startup.

Expected result:

- `open_capture_window_for_mode` or its replacement awaits frozen pixels before opening/revealing overlay.
- `get_capture_session` can return metadata without monitor image payload.
- `hydrate_capture_session_snapshots` is no longer required for overlay reveal.

### Phase 4: Simplify Frontend Reveal

Update `ScreenshotSession`:

- Start session from metadata.
- Reveal when metadata is loaded and selection canvas is ready.
- Remove `areCaptureImagesReady` as a reveal precondition.
- Keep `render_capture_output` as the source for preview image.

Magnifier/color sampling can keep lazy hydration behind a separate optional path.

### Phase 5: Remove Legacy Screenshot Path

After current behavior is covered:

- Remove or quarantine `ScreenshotWorkflow`.
- Remove or quarantine `ScreenshotCapture`.
- Remove or quarantine `ScreenshotEditor`.
- Remove or quarantine `screenshot_window_commands.rs`.
- Remove `public/screenshot.html` if no longer used.
- Remove stale command registrations for legacy screenshot commands.

### Phase 6: Update Documentation

Update:

- `CONTEXT.md` Capture Session language.
- `ARCHITECTURE.md` capture module map.
- `docs/SCREENSHOT_SHORTCUT_CAPTURE_PITFALLS.md`.
- Any docs that still say SnapLingo windows should be hidden before capture.

## Risks

### Startup Delay

Overlay appears after backend pixel capture completes. This is intentional. The delay should be bounded because the reveal path must not wait for frontend base64 hydration or preview rendering.

Mitigation:

- Log capture freeze duration separately from PNG/base64/render durations.
- Keep frontend startup metadata-only.

### Live View vs Frozen Output

The transparent overlay shows the live desktop behind it, while output uses trigger-time frozen pixels. If the desktop changes after trigger, the user may see a mismatch.

Mitigation:

- This is acceptable screenshot-tool semantics.
- Preview after selection must show frozen output so the user sees the true result before final copy/save if in Screenshot Mode.

### macOS Fullscreen And Space Behavior

Fullscreen apps and Spaces still require native behavior.

Mitigation:

- Keep macOS window-level, nonactivating panel, all-spaces behavior in the capture window adapter.
- Do not try to solve platform behavior in React.

### Existing In-Progress Work

The repository currently has a lazy capture overlay plan and related in-progress files. This refactor supersedes that direction unless the product chooses "freeze after selection" semantics.

Mitigation:

- Decide explicitly before implementation whether to abandon or adapt the lazy overlay work.

## Success Criteria

- Screenshot hotkey captures exactly what was visible before the capture overlay appears.
- Settings Window is captured only when it is visibly on screen at freeze time.
- Pinned Image Windows are captured only when they are visibly on screen at freeze time.
- Capture overlay never appears in screenshot output.
- `screenshot`, `screenshot-copy`, `screenshot-ocr`, `silent-screenshot-ocr`, and `screenshot-translate` keep their user-visible behavior.
- Overlay reveal no longer depends on full monitor image base64 hydration.
- Command layer becomes thin: one call into Capture Runtime for startup.
- Legacy screenshot path is removed or clearly marked unused.

## Verification Matrix

Automated:

- Frontend tests for reveal preconditions and capture completion flows.
- Backend tests for capture startup ordering.
- Backend tests for window hide planning: only `capture` is eligible.
- Rendering tests for multi-monitor and high-DPI selection composition.
- OCR tests proving recognition uses unannotated frozen pixels.

Manual:

- macOS normal desktop.
- macOS full-screen app / separate Space.
- Multi-monitor with different scale factors.
- Settings Window visible, hidden, and partially covered.
- Pinned image visible and hidden.
- Permission denied path: every trigger surfaces a visible error.
- Rapid repeated hotkeys: no duplicate active sessions.
