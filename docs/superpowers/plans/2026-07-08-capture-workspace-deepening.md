# Capture Workspace Deepening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deepen the frontend Capture Workspace seam so `ScreenshotSession/index.tsx` becomes a thin React view shell instead of owning state application, host workflow effects, keyboard dispatch, and pointer dispatch.

**Architecture:** Keep the existing capture domain modules (`captureSelectionRuntime`, `captureEditorRuntime`, `captureHostRuntime`, `captureActions`) and add one deeper Capture Workspace module around them. The new module should expose a small interface for workspace state, host actions, keyboard actions, and pointer actions while preserving the current Capture Mode behavior. This is not a visual redesign and must not change shortcut semantics, annotation behavior, magnifier behavior, or output behavior.

**Tech Stack:** React, TypeScript, Zustand settings store, Tauri frontend adapters, Vitest.

---

## Scope

In scope:
- frontend `ScreenshotSession` shell thinning
- workspace state/reset/patch locality
- host workflow actions: start, refresh, cancel, render preview, complete output
- keyboard dispatch currently owned by `ScreenshotSession/index.tsx`
- pointer dispatch currently owned by `ScreenshotSession/index.tsx`
- preserving current tests and adding focused tests for new interfaces

Out of scope:
- backend Capture Session Runtime changes
- changing screenshot defaults, hotkeys, or settings
- changing editor UI layout or toolbar design
- changing annotation geometry semantics
- changing Tauri command names

## File Structure

Create:
- `src/components/ScreenshotSession/captureWorkspaceState.ts`
  - Owns the frontend Capture Workspace snapshot shape and patch helpers.
- `src/components/ScreenshotSession/captureWorkspaceState.test.ts`
  - Tests reset, loaded-session application, preview reset, and manual-selection transition patches.
- `src/components/ScreenshotSession/useCaptureWorkspaceState.ts`
  - React hook that owns the workspace state and mutable refs currently scattered in `index.tsx`.
- `src/components/ScreenshotSession/captureWorkspaceHost.ts`
  - Deep host workflow module that binds workspace state to existing `captureHostRuntime` effects.
- `src/components/ScreenshotSession/captureWorkspaceHost.test.ts`
  - Tests host action ordering with injected clients and fake workspace adapter.
- `src/components/ScreenshotSession/captureWorkspaceKeyboard.ts`
  - Dispatches `KeyboardEvent` into workspace actions; depends on existing plan modules.
- `src/components/ScreenshotSession/captureWorkspaceKeyboard.test.ts`
  - Tests representative keyboard paths through a fake workspace adapter.
- `src/components/ScreenshotSession/captureWorkspacePointer.ts`
  - Dispatches root/preview pointer and wheel events into workspace actions; depends on existing pointer/selection/editor plan modules.
- `src/components/ScreenshotSession/captureWorkspacePointer.test.ts`
  - Tests representative pointer paths through a fake workspace adapter.
- `src/components/ScreenshotSession/CaptureWorkspaceView.tsx`
  - Pure-ish render module for the final JSX currently at the bottom of `index.tsx`.

Modify:
- `src/components/ScreenshotSession/index.tsx`
  - Use the new workspace state hook, host workflow module, keyboard dispatcher, pointer dispatcher, and view module.
- Existing tests as needed:
  - `src/components/ScreenshotSession/captureHostRuntime.test.ts`
  - `src/components/ScreenshotSession/captureEditorRuntime.test.ts`
  - `src/components/ScreenshotSession/capturePointerInteractionRuntime.test.ts`
  - `src/components/ScreenshotSession/captureKeyboardHostRuntime.test.ts`

Docs:
- Modify: `ARCHITECTURE.md`
  - Document Capture Workspace as the frontend owner of state/effect application.
- Modify: `CONTEXT.md`
  - Add or sharpen the Capture Workspace term.

## Task 1: Add a Capture Workspace state interface

**Files:**
- Create: `src/components/ScreenshotSession/captureWorkspaceState.ts`
- Create: `src/components/ScreenshotSession/captureWorkspaceState.test.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`

- [ ] **Step 1: Add failing tests for workspace patches**

Cover:
- initial workspace state matches the existing `ScreenshotSession` defaults
- loaded host session patch sets `session`, `status`, `cursorPoint`, and `hoverSelection`
- full interaction reset clears selection, annotations, text draft, preview image, magnifier, cursor color, rendering flag, and error
- preview reset matches `createCapturePreviewResetState()`

- [ ] **Step 2: Run focused tests**

Run:

```bash
npm test -- captureWorkspaceState.test.ts
```

Expected: FAIL because the module does not exist.

- [ ] **Step 3: Implement `captureWorkspaceState.ts`**

Create a data-first interface:

```ts
export interface CaptureWorkspaceState {
  status: SessionStatus;
  mode: CaptureMode;
  session: CaptureSessionView | null;
  startPoint: Point | null;
  cursorPoint: Point | null;
  selection: LogicalRect | null;
  hoverSelection: LogicalRect | null;
  editGesture: CaptureSelectionEditGesture | null;
  activeAnnotationTool: AnnotationTool | null;
  annotationGesture: AnnotationGestureDraft | null;
  draftAnnotation: AnnotationCommand | null;
  selectedAnnotationIndex: number | null;
  annotationMoveGesture: CaptureAnnotationMoveGesture | null;
  draftSelectionMoveGesture: CaptureDraftSelectionMoveGesture | null;
  textDraft: TextAnnotationDraft | null;
  textDraftAnnotationIndex: number | null;
  annotationStyle: AnnotationStyle;
  textFontSize: number;
  annotationHistory: AnnotationHistory;
  previewImageBase64: string | null;
  isAnnotationToolbarVisible: boolean;
  cursorColor: ColorSample | null;
  colorSampleFormat: ColorSampleFormat;
  isMagnifierRequested: boolean;
  isRenderingOutput: boolean;
  includeCapturedCursor: boolean;
  error: string | null;
}
```

Expose small helpers:

```ts
export function createInitialCaptureWorkspaceState(): CaptureWorkspaceState;
export function resetCaptureInteractionStatePatch(): Partial<CaptureWorkspaceState>;
export function loadedCaptureHostSessionPatch(loaded: LoadedCaptureHostSession): Partial<CaptureWorkspaceState>;
export function previewResetPatch(): Partial<CaptureWorkspaceState>;
```

Do not move DOM refs into this pure file.

- [ ] **Step 4: Re-run focused tests**

Run:

```bash
npm test -- captureWorkspaceState.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/ScreenshotSession/captureWorkspaceState.ts src/components/ScreenshotSession/captureWorkspaceState.test.ts
git commit -m "refactor(capture): define workspace state interface"
```

## Task 2: Move workspace state ownership behind one hook

**Files:**
- Create: `src/components/ScreenshotSession/useCaptureWorkspaceState.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: `src/components/ScreenshotSession/captureWorkspaceState.test.ts`

- [ ] **Step 1: Add failing tests for state adapter behavior**

Cover pure helpers first, then use hook only where necessary:
- applying a patch updates only specified fields
- reset helpers produce the same state currently set in `index.tsx`
- ref-backed values (`startPointRef`, `cursorPointRef`, `draftSelectionRef`, `hoverSelectionRef`) stay synchronized when using explicit setter helpers

- [ ] **Step 2: Run focused tests**

Run:

```bash
npm test -- captureWorkspaceState.test.ts
```

Expected: FAIL until patch/ref helpers exist.

- [ ] **Step 3: Implement `useCaptureWorkspaceState.ts`**

The hook should own:
- all `useState` values for capture workspace state
- refs currently used to avoid stale pointer/keyboard values
- explicit actions such as `applyPatch`, `resetInteraction`, `resetSession`, `applyLoadedSession`, `setStartPointWithRef`, `syncHoverSelection`

Keep derived values (`annotations`, `selectedAnnotation`, `canUndoAnnotation`) in `index.tsx` for this task. Do not move keyboard or pointer logic yet.

- [ ] **Step 4: Rewire only reset/load paths in `index.tsx`**

Replace direct setter clusters in:
- `resetCaptureInteractionState`
- `resetSessionState`
- `applyLoadedCaptureHostSession`
- `resetPreviewSelection`

Do not change pointer or keyboard code yet.

- [ ] **Step 5: Run focused frontend tests**

Run:

```bash
npm test -- captureWorkspaceState.test.ts captureHostRuntime.test.ts captureEditorRuntime.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/ScreenshotSession/useCaptureWorkspaceState.ts src/components/ScreenshotSession/index.tsx src/components/ScreenshotSession/captureWorkspaceState.test.ts
git commit -m "refactor(capture): centralize workspace state ownership"
```

## Task 3: Extract host workflow actions from the React shell

**Files:**
- Create: `src/components/ScreenshotSession/captureWorkspaceHost.ts`
- Create: `src/components/ScreenshotSession/captureWorkspaceHost.test.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`

- [ ] **Step 1: Add failing host workflow tests**

Cover:
- `startSession` sets loading state, resets interaction state, loads session, applies loaded session, and records perf steps
- `refreshSession` refuses to run without a current session id
- completion flow guards duplicate copy completions
- preview rendering sets rendering true, clears preview/error, writes base64, and resets rendering false
- host errors set `status: "error"` and keep the error message

- [ ] **Step 2: Run focused tests**

Run:

```bash
npm test -- captureWorkspaceHost.test.ts
```

Expected: FAIL because the module does not exist.

- [ ] **Step 3: Implement `captureWorkspaceHost.ts`**

Create a small interface that binds existing deep modules:

```ts
export interface CaptureWorkspaceHostAdapter {
  getState(): CaptureWorkspaceState;
  patch(next: Partial<CaptureWorkspaceState>): void;
  resetInteraction(): void;
  resetSession(): void;
  refs: CaptureWorkspaceRefs;
}
```

Expose actions:

```ts
export function createCaptureWorkspaceHostActions(deps: CaptureWorkspaceHostDeps): CaptureWorkspaceHostActions;
```

Internally reuse:
- `runCaptureHostSessionStart`
- `runCaptureHostSessionRefresh`
- `ensureCaptureHostSnapshotsHydrated`
- `runCaptureHostCompletionFlow`
- `runCaptureHostPreviewRender`
- `runCaptureRuntimeEffects`
- `runCaptureCompletionAction`
- `finishCaptureSession`
- `cancelCaptureSessionFlow`

- [ ] **Step 4: Rewire host functions in `index.tsx`**

Replace local implementations of:
- `ensureCaptureSnapshotsHydrated`
- `finishCurrentCaptureSession`
- `cancelSession`
- `startSession`
- `recordSuccessfulSelection`
- `runCaptureRuntimeEffects`
- `runCaptureCompletionAction`
- `renderSelectionPreview`
- `completePreviewSelection`
- `completeCandidateSelection`
- `refreshSession`

`index.tsx` should call host actions instead of owning these workflows.

- [ ] **Step 5: Re-run focused tests and build**

Run:

```bash
npm test -- captureWorkspaceHost.test.ts captureHostRuntime.test.ts captureSessionLifecycle.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/ScreenshotSession/captureWorkspaceHost.ts src/components/ScreenshotSession/captureWorkspaceHost.test.ts src/components/ScreenshotSession/index.tsx
git commit -m "refactor(capture): extract workspace host actions"
```

## Task 4: Extract keyboard dispatch from `ScreenshotSession`

**Files:**
- Create: `src/components/ScreenshotSession/captureWorkspaceKeyboard.ts`
- Create: `src/components/ScreenshotSession/captureWorkspaceKeyboard.test.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`

- [ ] **Step 1: Add failing keyboard dispatcher tests**

Use a fake workspace adapter and fake host/editor actions. Cover:
- Escape chooses the same dismiss action sequence
- F5 refreshes only in `selecting` or `preview`
- cursor include toggle rerenders preview when already in preview
- selected annotation arrow nudge updates annotation history and preview
- hover selection completion calls candidate completion
- color sample copy and format toggle still respect magnifier state

- [ ] **Step 2: Run focused tests**

Run:

```bash
npm test -- captureWorkspaceKeyboard.test.ts
```

Expected: FAIL because the dispatcher does not exist.

- [ ] **Step 3: Implement `captureWorkspaceKeyboard.ts`**

Move the body of `handleCaptureKeyboardKeyDown` into a dispatcher interface:

```ts
export interface CaptureWorkspaceKeyboardContext {
  state: CaptureWorkspaceState;
  refs: CaptureWorkspaceRefs;
  derived: CaptureWorkspaceDerivedState;
  actions: CaptureWorkspaceKeyboardActions;
}

export function handleCaptureWorkspaceKeyDown(
  event: KeyboardEvent,
  context: CaptureWorkspaceKeyboardContext,
): void;
```

The dispatcher may still call existing `plan*` modules. It must not import React.

- [ ] **Step 4: Rewire `index.tsx` keyboard callback**

`index.tsx` should build a context object and delegate to `handleCaptureWorkspaceKeyDown(...)`.

Keep `useCaptureKeyboardHostEvents(...)` in `index.tsx` for this task.

- [ ] **Step 5: Verify keyboard-focused behavior**

Run:

```bash
npm test -- captureWorkspaceKeyboard.test.ts captureActions.test.ts captureKeyboardHostRuntime.test.ts captureEditorRuntime.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/ScreenshotSession/captureWorkspaceKeyboard.ts src/components/ScreenshotSession/captureWorkspaceKeyboard.test.ts src/components/ScreenshotSession/index.tsx
git commit -m "refactor(capture): extract workspace keyboard dispatch"
```

## Task 5: Extract pointer and wheel dispatch from `ScreenshotSession`

**Files:**
- Create: `src/components/ScreenshotSession/captureWorkspacePointer.ts`
- Create: `src/components/ScreenshotSession/captureWorkspacePointer.test.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`

- [ ] **Step 1: Add failing pointer dispatcher tests**

Use fake pointer events or event-like objects. Cover:
- root pointer down starts a draft selection and captures the pointer
- pointer move updates hover candidate when not drafting
- pointer move updates draft selection while dragging
- pointer up commits draft selection through manual selection completion
- preview pointer down starts annotation tool gesture
- resize handle pointer down starts selection edit gesture
- wheel adjusts annotation size only when the current state allows it

- [ ] **Step 2: Run focused tests**

Run:

```bash
npm test -- captureWorkspacePointer.test.ts
```

Expected: FAIL because the dispatcher does not exist.

- [ ] **Step 3: Implement `captureWorkspacePointer.ts`**

Move logic from:
- `handlePointerDown`
- `handlePointerMove`
- `handlePointerUp`
- `startMoveGesture`
- `startResizeGesture`
- `handleWheel`

Expose:

```ts
export function handleCaptureWorkspacePointerDown(...): void;
export function handleCaptureWorkspacePointerMove(...): void;
export function handleCaptureWorkspacePointerUp(...): void;
export function handleCaptureWorkspacePreviewPointerDown(...): void;
export function handleCaptureWorkspaceResizePointerDown(...): void;
export function handleCaptureWorkspaceWheel(...): void;
```

The module may depend on existing pointer, selection, editor, virtual desktop, and candidate plan modules. It must not import React.

- [ ] **Step 4: Rewire `index.tsx` pointer handlers**

`index.tsx` should only adapt React event objects into dispatcher calls and pass workspace/host/editor actions.

- [ ] **Step 5: Verify pointer-focused behavior**

Run:

```bash
npm test -- captureWorkspacePointer.test.ts capturePointerInteractionRuntime.test.ts captureSelectionRuntime.test.ts captureEditorRuntime.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/ScreenshotSession/captureWorkspacePointer.ts src/components/ScreenshotSession/captureWorkspacePointer.test.ts src/components/ScreenshotSession/index.tsx
git commit -m "refactor(capture): extract workspace pointer dispatch"
```

## Task 6: Split the render module from the orchestration shell

**Files:**
- Create: `src/components/ScreenshotSession/CaptureWorkspaceView.tsx`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Add or modify tests only if markup behavior needs locking

- [ ] **Step 1: Add a smoke render test if a stable one exists**

Prefer testing presentation helpers rather than brittle full markup. If no stable test surface exists, skip adding a JSX snapshot.

Run:

```bash
npm test -- capturePresentation.test.ts capturePreviewPresentation
```

Expected: existing tests pass before extraction.

- [ ] **Step 2: Move the final JSX into `CaptureWorkspaceView.tsx`**

The view receives props:
- workspace state needed for rendering
- derived geometry
- pointer handlers
- toolbar handlers
- cancel/copy/save/quick-save/OCR handlers

The view must not start sessions, read localStorage, call Tauri adapters, or mutate refs directly.

- [ ] **Step 3: Reduce `index.tsx` to composition**

After this task, `index.tsx` should mostly:
- read settings
- initialize workspace state
- compute derived geometry
- create host/keyboard/pointer actions
- wire hooks
- render `<CaptureWorkspaceView />`

- [ ] **Step 4: Verify frontend**

Run:

```bash
npm test -- ScreenshotSession captureWorkspace captureHostRuntime.test.ts captureEditorRuntime.test.ts captureActions.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/ScreenshotSession/CaptureWorkspaceView.tsx src/components/ScreenshotSession/index.tsx
git commit -m "refactor(capture): split workspace view from orchestration"
```

## Task 7: Architecture residue check and docs

**Files:**
- Modify: `ARCHITECTURE.md`
- Modify: `CONTEXT.md`

- [ ] **Step 1: Run architecture residue searches**

Run:

```bash
wc -l src/components/ScreenshotSession/index.tsx
rg "set[A-Z][A-Za-z]+\\(" src/components/ScreenshotSession/index.tsx
rg "window\\.localStorage|runCaptureHostSessionStart|runCaptureHostCompletionFlow|planCaptureRootPointerDown|planCaptureWorkspaceKeyDown" src/components/ScreenshotSession/index.tsx
```

Expected:
- `index.tsx` is materially smaller than 2461 lines
- direct setter clusters are gone or limited to local composition
- workflow and dispatch logic lives behind workspace modules

- [ ] **Step 2: Update docs**

In `CONTEXT.md`, add `Capture Workspace`:
- owns frontend capture state/effect application
- bridges pure plan modules and React view
- keeps Tauri/native calls behind host adapters

In `ARCHITECTURE.md`, update the runtime seam list:
- `captureInteractionRuntime.ts` remains pure plan logic
- `captureWorkspace*` modules own state/effect dispatch
- `ScreenshotSession/index.tsx` is the composition shell

- [ ] **Step 3: Run full verification**

Run:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
cargo fmt --manifest-path src-tauri/Cargo.toml --check
git diff --check
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add ARCHITECTURE.md CONTEXT.md
git commit -m "docs(capture): document workspace seam"
```

## Notes

- Do not introduce a generic state machine framework. The current plan modules are already useful; deepen the workspace seam around them.
- Do not move backend capture behavior in this plan. Backend `CaptureSessionRuntime` is already a separate deep module.
- Do not add new UI behavior while extracting. Every task should preserve current Capture Mode semantics.
- Use TDD for every new workspace module. Each new module must have a narrower test surface than the original React shell.
- If a task starts requiring broad visual rewrites, stop and split it into a separate design plan.
