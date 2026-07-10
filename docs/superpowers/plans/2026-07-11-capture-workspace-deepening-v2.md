# Capture Workspace Deepening V2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox format for tracking.

**Goal:** Make one Capture Workspace Application runtime own selection, annotation, effect interpretation, and host workflow coordination behind a narrow View interface.

**Architecture:** Characterize the current multi-module workflow first, then collapse forwarding controllers into one runtime hook. Retain pure geometry, selection, annotation, and presentation modules.

**Tech Stack:** React, TypeScript, Vitest.

---

### Task 1: Characterize real workflow wiring

**Files:**
- Create: src/application/capture-workspace/runtime.test.ts
- Inspect: src/views/CaptureWorkspace/useCaptureWorkspaceController.ts
- Inspect: useCaptureWorkspaceHostController.ts, useCaptureWorkspaceEditorController.ts, useCaptureWorkspaceInputController.ts

- [ ] Write a failing test for pointer selection completion through effect execution.
- [ ] Write a failing test for keyboard confirm/cancel through the same runtime interface.
- [ ] Write a failing test for host hydration failure and rollback.
- [ ] Use fake Platform ports, not raw Tauri mocks.
- [ ] Run focused tests and verify RED because the unified runtime does not exist.
- [ ] Commit tests with message test: characterize capture workspace workflow.

### Task 2: Introduce the deep runtime interface

**Files:**
- Create: src/application/capture-workspace/runtime.ts
- Create: src/application/capture-workspace/types.ts
- Modify: src/application/capture-workspace/platformRuntime.ts

- [ ] Define one runtime result containing renderState and actions.
- [ ] Keep mutable refs and effect execution internal.
- [ ] Implement only enough composition to pass the characterization tests.
- [ ] Run focused tests and verify GREEN.
- [ ] Commit with message refactor: add capture workspace runtime.

### Task 3: Absorb host and input forwarding

**Files:**
- Modify: src/application/capture-workspace/runtime.ts
- Delete after migration: src/views/CaptureWorkspace/useCaptureWorkspaceHostController.ts
- Delete after migration: src/views/CaptureWorkspace/useCaptureWorkspaceInputController.ts
- Modify: captureWorkspaceHost.ts, captureHostRuntime.ts, captureWorkspacePointer.ts

- [ ] Move workflow orchestration into the runtime without moving pure algorithms.
- [ ] Replace wide deps objects with internal closures over runtime state.
- [ ] Run characterization and existing pointer/host tests after each move.
- [ ] Apply the deletion test before removing each forwarding module.
- [ ] Commit with message refactor: absorb capture host and input wiring.

### Task 4: Absorb editor transaction wiring

**Files:**
- Modify: src/application/capture-workspace/runtime.ts
- Delete after migration: src/views/CaptureWorkspace/useCaptureWorkspaceEditorController.ts
- Preserve: annotationGeometry.ts, annotationHistory.ts, annotationStyle.ts, textAnnotationDraft.ts

- [ ] Move annotation transaction ownership into the runtime.
- [ ] Keep annotation algorithms pure and independently tested.
- [ ] Run editor, history, style, and runtime tests.
- [ ] Commit with message refactor: centralize capture editor transactions.

### Task 5: Narrow the View seam

**Files:**
- Modify: src/views/CaptureWorkspace/index.tsx
- Modify: src/views/CaptureWorkspace/CaptureWorkspaceView.tsx
- Delete or rename: useCaptureWorkspaceController.ts

- [ ] Make the View consume only renderState and actions.
- [ ] Remove manually assembled host/editor/input property bags.
- [ ] Add a View test proving user actions call the runtime interface.
- [ ] Run all Capture Workspace tests.
- [ ] Commit with message refactor: narrow capture workspace view interface.

### Task 6: Verification

- [ ] Run npm test -- src/application/capture-workspace src/views/CaptureWorkspace.
- [ ] Run npm test.
- [ ] Run npm run build.
- [ ] Run the frontend architecture rules.

