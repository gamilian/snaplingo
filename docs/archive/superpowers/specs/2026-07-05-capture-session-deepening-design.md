# Capture Session Deepening Design

## Goal

Refactor Capture Session around two deeper module seams:

- backend: `CaptureSessionRuntime` becomes the workflow module that owns capture startup choreography
- frontend: a new host runtime module becomes the workflow module that owns effect interpretation, host events, reveal timing, and selection persistence

The `commands` layer and `ScreenshotSession` React shell should stay thin. They should cross one clear seam each instead of owning ordered workflow knowledge directly.

## Scope

This refactor is intentionally limited to the Capture Session path.

In scope:

- backend capture startup choreography
- frontend capture host/runtime choreography
- moving side-effect ordering out of command and React shell modules
- tests that lock the new seam shapes and preserve current behavior

Out of scope:

- Provider refactors
- Hotkey architecture refactors outside Capture Session wiring
- annotation subsystem redesign
- Capture Mode semantics changes
- UI redesign

## Current Friction

### Backend

`src-tauri/src/commands/capture_session_commands.rs` currently knows too much about ordered workflow behavior:

- begin/end capture presentation
- create session from visible desktop
- open capture overlay on the main thread
- recover from partial failures
- restore hidden windows
- cancel failed sessions

That makes the command seam shallow. The command module is no longer only a Tauri adapter.

At the same time, `src-tauri/src/application/services/capture_session_runtime.rs` is still mostly a forwarder around render/output/OCR operations. The module name is right, but its interface has not yet absorbed the full Capture Session choreography.

### Frontend

`src/components/ScreenshotSession/index.tsx` is too broad:

- React state and refs
- JSX rendering
- `CaptureRuntimeEffect` interpretation
- host event subscription
- reveal timing
- selection persistence
- preview/output/OCR side effects

`captureInteractionRuntime.ts` already provides a useful pure decision seam, but the implementation still leaks into the React shell. The shell still decides how to execute runtime effects and when to subscribe to host events.

## Target Architecture

## Backend Seam

### Command Module

`src-tauri/src/commands/capture_session_commands.rs` remains a backend Tauri command seam.

Responsibilities:

- decode command inputs
- call `CaptureSessionRuntime`
- convert backend errors to command return types

Non-responsibilities:

- capture startup ordering
- presentation lifecycle orchestration
- window recovery choreography
- failure rollback strategy

### Capture Session Runtime

`src-tauri/src/application/services/capture_session_runtime.rs` becomes the deep backend module for Capture Session workflow.

Responsibilities:

- begin capture presentation
- create capture session from visible desktop
- open capture overlay for the session
- handle main-thread dispatch for capture window operations
- rollback partial failures
- restore hidden windows when startup fails
- continue to own render/output/OCR workflow entrypoints

The key design rule is:

```text
commands cross one Capture Session Runtime seam
Capture Session Runtime crosses infrastructure adapters
```

Infrastructure adapters such as `capture_window` and `pinned_window` remain adapters. They should know how to interact with the platform, not when a workflow should call them.

## Frontend Seam

### ScreenshotSession React Shell

`src/components/ScreenshotSession/index.tsx` remains the UI shell.

Responsibilities:

- React state
- refs
- JSX
- wiring user interaction into pure decision modules and host runtime calls

Non-responsibilities:

- directly interpreting `CaptureRuntimeEffect`
- directly subscribing to host events
- directly coordinating reveal timing
- directly reading/writing selection persistence

### Capture Host Runtime

Add a new deep frontend module under `src/components/ScreenshotSession/`, named:

`captureHostRuntime.ts`

Responsibilities:

- execute `CaptureRuntimeEffect`
- subscribe to `hotkey-triggered`
- subscribe to native cancel/copy requests
- coordinate reveal timing
- coordinate selection persistence and restoration
- hide adapter details such as Tauri event wiring, output calls, and clipboard paths behind one interface

### Pure Runtime Stays Pure

`src/components/ScreenshotSession/captureInteractionRuntime.ts` stays a pure decision module.

It should continue deciding:

- which effects to run
- which target receives OCR results
- whether a completion records selection history

It should not know:

- Tauri
- `window`
- `localStorage`
- event subscriptions
- concrete runtime adapter functions

## Data Flow

### Backend Capture Startup

```text
Tauri command
→ CaptureSessionRuntime.open_capture_for_mode(...)
→ begin capture presentation
→ create session from visible desktop
→ open capture overlay for session
→ return launch success
```

Failure path:

```text
CaptureSessionRuntime.open_capture_for_mode(...)
→ partial failure after presentation/session start
→ restore hidden windows
→ cancel failed session
→ end capture presentation
→ return error
```

### Frontend Completion Flow

```text
React shell
→ captureInteractionRuntime builds effect plan
→ captureHostRuntime executes plan
→ adapters perform OCR/render/output/window actions
→ React shell updates state/UI
```

### Frontend Host Event Flow

```text
React shell mounts
→ captureHostRuntime subscribes host events
→ host event received
→ captureHostRuntime invokes shell callback
→ shell updates state or starts session
```

## Files

### Backend

Modify:

- `src-tauri/src/application/services/capture_session_runtime.rs`
- `src-tauri/src/commands/capture_session_commands.rs`
- `src-tauri/src/application/services/mod.rs`

Optional helper extraction is allowed only if it stays internal to the Capture Session Runtime implementation. This refactor should not create another shallow public seam.

### Frontend

Create:

- `src/components/ScreenshotSession/captureHostRuntime.ts`

Modify:

- `src/components/ScreenshotSession/index.tsx`

Reuse:

- `src/components/ScreenshotSession/captureInteractionRuntime.ts`
- `src/components/ScreenshotSession/captureWindowVisibility.ts`
- `src/components/ScreenshotSession/selectionMemory.ts`
- `src/components/ScreenshotSession/captureCancelRequest.ts`

## Migration Strategy

### Phase 1: Backend First

Deepen `CaptureSessionRuntime` before moving frontend host logic.

Expected result:

- command module no longer imports and sequences most capture window choreography directly
- startup rollback is concentrated in one backend module
- existing render/output/OCR methods remain intact

### Phase 2: Frontend Host Runtime

Move effect interpretation, host subscriptions, reveal timing, and selection persistence into `captureHostRuntime.ts`.

Expected result:

- `ScreenshotSession/index.tsx` becomes a thinner UI shell
- pure planning stays in `captureInteractionRuntime.ts`
- host/runtime side effects concentrate in one module

### Phase 3: Verification Sweep

After both phases:

- re-run focused Rust tests
- re-run focused frontend tests
- run full verification commands

## Error Handling

### Backend

When presentation begins and a later step fails, `CaptureSessionRuntime` owns the full recovery sequence:

- restore hidden windows
- cancel failed session
- end capture presentation

The command seam should not manually compose recovery strings or lifecycle ordering.

### Frontend

`captureHostRuntime.ts` should surface failures back to `ScreenshotSession/index.tsx` through narrow callbacks or Promise rejection.

The React shell remains responsible for:

- `setError(...)`
- `setStatus('error')`

But it should not own the lower-level host choreography that produced the failure.

The refactor should preserve current user-visible behavior:

- no intended UX changes
- no Capture Mode semantic changes
- no new error presentation behavior

## Testing

### Backend

Add or update focused tests to cover:

- capture startup success path
- capture startup failure rollback
- command seam behaving as a thin adapter
- render/output/OCR behavior remaining intact

### Frontend

Add or update focused tests to cover:

- `captureHostRuntime.ts` effect interpretation
- host subscription wiring
- reveal timing behavior
- selection persistence behavior
- `ScreenshotSession` behavior through the new host seam

## Success Criteria

### Backend

- `capture_session_commands.rs` no longer directly owns most startup choreography
- `CaptureSessionRuntime` becomes the main workflow seam below commands
- rollback logic is concentrated in `CaptureSessionRuntime`

### Frontend

- `ScreenshotSession/index.tsx` no longer directly interprets `CaptureRuntimeEffect`
- `ScreenshotSession/index.tsx` no longer directly subscribes to `hotkey-triggered`, native cancel, or native copy requests
- `ScreenshotSession/index.tsx` no longer directly owns selection persistence wiring

### Verification

Required before completion:

- focused Rust tests for capture workflow changes
- focused Vitest tests for frontend host/runtime changes
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npm test`
- `npm run build`

## Recommendation

Use this refactor as a seam-deepening change, not a broad cleanup pass.

Do not use it to:

- opportunistically redesign annotations
- refactor unrelated provider or hotkey architecture
- chase cosmetic file splits that do not improve locality

The value of this change comes from one thing: Capture Session behavior moving behind deeper interfaces on both backend and frontend.
