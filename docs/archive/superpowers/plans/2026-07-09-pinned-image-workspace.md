# Pinned Image Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deepen the Pinned Image Window into a Pinned Image Workspace module so pin state, keyboard/pointer dispatch, window effects, and rendering are locally testable while preserving current Snipaste-style behavior.

**Architecture:** Keep `src/components/PinnedImageWindow/index.tsx` as the composition shell. Move state/defaults into a workspace state module, move keyboard and pointer/wheel branching into runtime modules, and move JSX into a render-only view. Preserve existing `pinControls.ts` and `pinActions.ts` as low-level helpers; do not change backend commands or Tauri adapters.

**Tech Stack:** React 18, TypeScript, Vitest, Tauri frontend adapters, existing Pinned Image and ScreenshotSession helper modules.

---

## Scope

In scope:
- Shrink `PinnedImageWindow/index.tsx` from a state/effect/render owner into a composition shell.
- Add a pure Pinned Image Workspace state model.
- Add a workspace state hook that owns pin state, refs, image loading, sample canvas hydration, and derived values.
- Add keyboard runtime tests for all current shortcuts.
- Add pointer/wheel runtime tests for zoom, opacity, context menu, drag, thumbnail toggle, close, and reset behavior.
- Extract `PinnedImageWindowView.tsx` as render-only JSX.
- Update architecture docs with the new Pinned Image Workspace seam.

Out of scope:
- Changing any keyboard shortcuts, mouse gestures, menu labels, toolbar labels, visual styling, or pin window behavior.
- Changing backend pinned image commands, persistence, grouping, or Tauri capabilities.
- Reworking `pinControls.ts` or `pinActions.ts` beyond importing shared types if necessary.
- Adding new pin features.

## Preconditions

Do this in a feature branch:

```bash
git switch -c codex/pinned-image-workspace
```

Before editing, verify the worktree is clean:

```bash
git status --short
```

Expected: no output.

## Success Criteria

- `PinnedImageWindow/index.tsx` is materially smaller and mostly composes workspace state, runtime handlers, Tauri adapters, and `PinnedImageWindowView`.
- `PinnedImageWindowView.tsx` has no `useState`, `useEffect`, `useMemo`, `useCallback`, Tauri adapter imports, Zustand store imports, or direct window calls.
- Keyboard behavior is covered through `pinnedImageKeyboardRuntime.test.ts`.
- Pointer/wheel behavior is covered through `pinnedImagePointerRuntime.test.ts`.
- Existing `pinControls.test.ts`, `pinActions.test.ts`, and `pinWindowPermissions.test.ts` still pass.
- Full frontend verification passes:

```bash
npm test
npm run build
```

## File Structure

Create:
- `src/components/PinnedImageWindow/pinnedImageWorkspaceState.ts`
  - Pure state shape, defaults, reset patches, derived display/magnifier data, and view-model helpers.
- `src/components/PinnedImageWindow/pinnedImageWorkspaceState.test.ts`
  - Tests state defaults, reset patches, replacement patch, derived image frame size, magnifier view data, and hover toolbar visibility.
- `src/components/PinnedImageWindow/usePinnedImageWorkspaceState.ts`
  - React hook that owns workspace state, refs, image loading, sample canvas hydration, color sampling, and ref-backed patch helpers.
- `src/components/PinnedImageWindow/pinnedImageKeyboardRuntime.ts`
  - Pure keyboard dispatcher that maps `KeyboardEvent`-like input to injected actions.
- `src/components/PinnedImageWindow/pinnedImageKeyboardRuntime.test.ts`
  - Tests current keyboard shortcut priority and side effects through action spies.
- `src/components/PinnedImageWindow/pinnedImagePointerRuntime.ts`
  - Pure wheel/context-menu/pointer dispatcher helpers.
- `src/components/PinnedImageWindow/pinnedImagePointerRuntime.test.ts`
  - Tests wheel, context menu, image pointer move/leave, and pointer-down action mapping.
- `src/components/PinnedImageWindow/PinnedImageWindowView.tsx`
  - Render-only view for image, magnifier, context menu, hover toolbar, and error display.

Modify:
- `src/components/PinnedImageWindow/index.tsx`
  - Reduce to composition shell.
- `src/components/PinnedImageWindow/pinControls.ts`
  - Only if shared exported types/default helpers are needed.
- `src/components/PinnedImageWindow/pinControls.test.ts`
  - Only if types/default exports require test alignment; no behavior changes.
- `ARCHITECTURE.md`
  - Add the Pinned Image Workspace seam.
- `CONTEXT.md`
  - Add a short Pinned Image Workspace domain term.

Do not modify:
- `src/tauri/pinnedImage.ts`
- `src-tauri/src/commands/pinned_image_commands.rs`
- `src-tauri/src/application/services/pinned_image_service.rs`

---

## Task 1: Define Pinned Image Workspace State

**Files:**
- Create: `src/components/PinnedImageWindow/pinnedImageWorkspaceState.ts`
- Create: `src/components/PinnedImageWindow/pinnedImageWorkspaceState.test.ts`
- Modify: `src/components/PinnedImageWindow/index.tsx`

- [ ] **Step 1: Write failing state model tests**

Cover:
- initial workspace state matches current `PinnedImageWindow` defaults
- display reset clears zoom, thumbnail mode, and context menu while preserving opacity
- display + opacity reset clears zoom, opacity, thumbnail mode, and context menu
- replacing an image resets zoom, thumbnail mode, transform, and context menu
- frame size resolves thumbnail mode versus zoom mode
- hover toolbar visibility class matches forced-visible and hover-visible behavior

Suggested test skeleton:

```ts
import { describe, expect, it } from 'vitest';
import {
  createDefaultPinnedTransform,
  createDefaultPinnedVisualFilter,
  createInitialPinnedImageWorkspaceState,
  displayResetPatch,
  displayAndOpacityResetPatch,
  pinnedImageReplacementPatch,
  pinnedHoverToolbarVisibilityClassName,
  resolvePinnedImageFrameSize,
} from './pinnedImageWorkspaceState';

const image = {
  id: 'pin-1',
  image_base64: '',
  width: 1200,
  height: 800,
  source_text: 'hello',
};

describe('pinned image workspace state', () => {
  it('creates the same initial state currently owned by PinnedImageWindow', () => {
    expect(createInitialPinnedImageWorkspaceState()).toMatchObject({
      image: null,
      zoom: 1,
      opacity: 1,
      isThumbnailMode: false,
      transform: createDefaultPinnedTransform(),
      visualFilter: createDefaultPinnedVisualFilter(),
      contextMenuPosition: null,
      imagePointerPoint: null,
      viewportPointerPoint: null,
      cursorColor: null,
      colorSampleFormat: 'hex',
      isMagnifierRequested: false,
      sampleCanvasVersion: 0,
      isHoverToolbarForcedVisible: false,
      error: null,
    });
  });
});
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
npm test -- pinnedImageWorkspaceState.test.ts
```

Expected: FAIL because `pinnedImageWorkspaceState.ts` does not exist.

- [ ] **Step 3: Implement the pure state module**

Export at minimum:

```ts
import type { PinnedImageView } from '../ScreenshotSession/types';
import type { ColorSample, ColorSampleFormat } from '../ScreenshotSession/colorSampler';
import {
  getPinnedDisplaySize,
  getPinnedThumbnailDisplaySize,
  type PinnedPoint,
  type PinnedTransform,
  type PinnedVisualFilter,
} from './pinControls';

export interface PinnedImageWorkspaceState {
  image: PinnedImageView | null;
  zoom: number;
  opacity: number;
  isThumbnailMode: boolean;
  transform: PinnedTransform;
  visualFilter: PinnedVisualFilter;
  contextMenuPosition: PinnedPoint | null;
  imagePointerPoint: PinnedPoint | null;
  viewportPointerPoint: PinnedPoint | null;
  cursorColor: ColorSample | null;
  colorSampleFormat: ColorSampleFormat;
  isMagnifierRequested: boolean;
  sampleCanvasVersion: number;
  isHoverToolbarForcedVisible: boolean;
  error: string | null;
}

export function createDefaultPinnedTransform(): PinnedTransform {
  return { rotation: 0, flipX: false, flipY: false };
}

export function createDefaultPinnedVisualFilter(): PinnedVisualFilter {
  return { grayscale: false, inverted: false };
}

export function createInitialPinnedImageWorkspaceState(): PinnedImageWorkspaceState {
  return {
    image: null,
    zoom: 1,
    opacity: 1,
    isThumbnailMode: false,
    transform: createDefaultPinnedTransform(),
    visualFilter: createDefaultPinnedVisualFilter(),
    contextMenuPosition: null,
    imagePointerPoint: null,
    viewportPointerPoint: null,
    cursorColor: null,
    colorSampleFormat: 'hex',
    isMagnifierRequested: false,
    sampleCanvasVersion: 0,
    isHoverToolbarForcedVisible: false,
    error: null,
  };
}

export function displayResetPatch(): Partial<PinnedImageWorkspaceState> {
  return {
    zoom: 1,
    isThumbnailMode: false,
    contextMenuPosition: null,
  };
}

export function displayAndOpacityResetPatch(): Partial<PinnedImageWorkspaceState> {
  return {
    ...displayResetPatch(),
    opacity: 1,
  };
}

export function pinnedImageReplacementPatch(
  image: PinnedImageView,
): Partial<PinnedImageWorkspaceState> {
  return {
    image,
    zoom: 1,
    isThumbnailMode: false,
    transform: createDefaultPinnedTransform(),
    contextMenuPosition: null,
  };
}

export function resolvePinnedImageFrameSize(state: Pick<
  PinnedImageWorkspaceState,
  'image' | 'isThumbnailMode' | 'zoom'
>) {
  if (!state.image) return null;
  return state.isThumbnailMode
    ? getPinnedThumbnailDisplaySize(state.image)
    : getPinnedDisplaySize(state.image, state.zoom);
}

export function pinnedHoverToolbarVisibilityClassName(forcedVisible: boolean) {
  return forcedVisible
    ? 'opacity-100'
    : 'opacity-0 focus-within:opacity-100 group-hover:opacity-100';
}
```

Keep this file pure: no React, no DOM, no Tauri adapters.

- [ ] **Step 4: Rewire default helpers in `index.tsx` only**

Import `createDefaultPinnedTransform`, `createDefaultPinnedVisualFilter`, and `resolvePinnedImageFrameSize` from the new module. Do not move event handlers yet.

- [ ] **Step 5: Run focused tests**

Run:

```bash
npm test -- pinnedImageWorkspaceState.test.ts pinControls.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/PinnedImageWindow/pinnedImageWorkspaceState.ts src/components/PinnedImageWindow/pinnedImageWorkspaceState.test.ts src/components/PinnedImageWindow/index.tsx
git commit -m "refactor(pin): define pinned image workspace state"
```

---

## Task 2: Move Workspace State Ownership Behind a Hook

**Files:**
- Create: `src/components/PinnedImageWindow/usePinnedImageWorkspaceState.ts`
- Modify: `src/components/PinnedImageWindow/index.tsx`
- Modify: `src/components/PinnedImageWindow/pinnedImageWorkspaceState.test.ts`

- [ ] **Step 1: Add failing tests for state patch helpers**

Add pure tests first; avoid brittle hook tests unless necessary. Cover:
- `applyPinnedImageWorkspacePatch` updates only specified fields
- `setErrorFromUnknown` formats `Error` and string-like values consistently
- sample canvas reset patch clears `cursorColor` and increments `sampleCanvasVersion`
- pointer leave patch clears image and viewport pointer points

Suggested exports:

```ts
export function applyPinnedImageWorkspacePatch(
  state: PinnedImageWorkspaceState,
  patch: Partial<PinnedImageWorkspaceState>,
): PinnedImageWorkspaceState;

export function sampleCanvasResetPatch(
  currentVersion: number,
): Partial<PinnedImageWorkspaceState>;

export function pointerLeavePatch(): Partial<PinnedImageWorkspaceState>;

export function errorMessageFromUnknown(error: unknown): string;
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
npm test -- pinnedImageWorkspaceState.test.ts
```

Expected: FAIL until helpers exist.

- [ ] **Step 3: Implement `usePinnedImageWorkspaceState.ts`**

The hook should own:
- `useState(createInitialPinnedImageWorkspaceState)`
- `imageFrameRef`
- `sampleCanvasRef`
- `applyPatch`
- field setters currently created by individual `useState` calls
- `loadImage(imageId, getPinnedImage)`
- `hydrateSampleCanvas()`
- `updateCursorColor(sampleCanvasColor, imageFrameSize)`

Use this shape:

```ts
import { useCallback, useMemo, useRef, useState } from 'react';
import type { PinnedImageView } from '../ScreenshotSession/types';
import type { PinnedPoint } from './pinControls';
import {
  applyPinnedImageWorkspacePatch,
  createInitialPinnedImageWorkspaceState,
  errorMessageFromUnknown,
  pointerLeavePatch,
  sampleCanvasResetPatch,
  type PinnedImageWorkspaceState,
} from './pinnedImageWorkspaceState';

export interface UsePinnedImageWorkspaceStateOptions {
  getPinnedImage(imageId: string): Promise<PinnedImageView>;
}

export function usePinnedImageWorkspaceState({
  getPinnedImage,
}: UsePinnedImageWorkspaceStateOptions) {
  const [state, setState] = useState(createInitialPinnedImageWorkspaceState);
  const imageFrameRef = useRef<HTMLDivElement | null>(null);
  const sampleCanvasRef = useRef<HTMLCanvasElement | null>(null);

  const applyPatch = useCallback((patch: Partial<PinnedImageWorkspaceState>) => {
    setState((currentState) => applyPinnedImageWorkspacePatch(currentState, patch));
  }, []);

  return {
    ...state,
    imageFrameRef,
    sampleCanvasRef,
    applyPatch,
    setImage: (image: PinnedImageView | null) => applyPatch({ image }),
    setZoom: (zoom: number | ((current: number) => number)) =>
      setState((currentState) => ({
        ...currentState,
        zoom: typeof zoom === 'function' ? zoom(currentState.zoom) : zoom,
      })),
    // Add only the setters needed by index.tsx during this task.
  };
}
```

Do not move keyboard/pointer branching yet. Keep this hook as state ownership, not interaction policy.

- [ ] **Step 4: Rewire `index.tsx` state only**

Replace the many local `useState` calls with the hook. Keep current callbacks and JSX in `index.tsx`.

Important parity checks:
- image load still calls `getPinnedImage(imageId)`
- failed image load still sets `error`
- sample canvas hydration still resets `cursorColor` before loading
- color sampling still updates after image pointer movement
- `imageFrameRef` and `sampleCanvasRef` stay the same refs used by existing handlers

- [ ] **Step 5: Run focused tests and build**

Run:

```bash
npm test -- pinnedImageWorkspaceState.test.ts pinControls.test.ts pinActions.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/PinnedImageWindow/usePinnedImageWorkspaceState.ts src/components/PinnedImageWindow/pinnedImageWorkspaceState.ts src/components/PinnedImageWindow/pinnedImageWorkspaceState.test.ts src/components/PinnedImageWindow/index.tsx
git commit -m "refactor(pin): centralize pinned workspace state"
```

---

## Task 3: Extract Keyboard Runtime

**Files:**
- Create: `src/components/PinnedImageWindow/pinnedImageKeyboardRuntime.ts`
- Create: `src/components/PinnedImageWindow/pinnedImageKeyboardRuntime.test.ts`
- Modify: `src/components/PinnedImageWindow/index.tsx`

- [ ] **Step 1: Write failing keyboard runtime tests**

Cover priority and behavior:
- `Escape` hides forced toolbar before closing the pin
- `Shift+Escape` destroys the pin when toolbar is not being hidden
- plain `Escape` closes/hides the pin
- plain `Alt` requests magnifier
- with magnifier and cursor color, copy shortcut calls `copyCurrentColor`
- with magnifier and cursor color, format toggle flips `hex`/`rgb`
- `Cmd/Ctrl+C` copies image
- `Cmd/Ctrl+Shift+C` copies source text when available
- `Cmd/Ctrl+S` saves
- `Cmd/Ctrl+Shift+S` quick saves
- `Cmd/Ctrl+Shift+P` opens preferences
- `Cmd/Ctrl+W` hides current pin
- `Cmd/Ctrl+V` replaces from clipboard
- space toggles hover toolbar
- arrow keys move the window
- zoom, opacity, transform, and visual filter shortcuts call the right actions

Use an event stub:

```ts
function keyboardEvent(input: Partial<KeyboardEvent> & { key: string }) {
  return {
    key: input.key,
    metaKey: input.metaKey ?? false,
    ctrlKey: input.ctrlKey ?? false,
    altKey: input.altKey ?? false,
    shiftKey: input.shiftKey ?? false,
    preventDefault: vi.fn(),
  } as unknown as KeyboardEvent & { preventDefault: ReturnType<typeof vi.fn> };
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
npm test -- pinnedImageKeyboardRuntime.test.ts
```

Expected: FAIL because the runtime does not exist.

- [ ] **Step 3: Implement `pinnedImageKeyboardRuntime.ts`**

Export:

```ts
import type { ColorSample, ColorSampleFormat } from '../ScreenshotSession/colorSampler';
import type { PinnedImageWorkspaceState } from './pinnedImageWorkspaceState';
import type {
  PinnedKeyboardOpacityAction,
  PinnedKeyboardTransformAction,
  PinnedKeyboardVisualFilterAction,
  PinnedKeyboardZoomAction,
  PinnedPoint,
} from './pinControls';

export interface PinnedImageKeyboardActions {
  hideToolbar(): void;
  destroyCurrentPinnedImage(): Promise<void> | void;
  hideCurrentPinnedImage(): Promise<void> | void;
  setMagnifierRequested(value: boolean): void;
  copyCurrentColor(): Promise<void> | void;
  toggleColorSampleFormat(): void;
  copyPinnedSourceText(): Promise<void> | void;
  copyCurrentPinnedImage(): Promise<void> | void;
  savePinnedImageAs(): Promise<void> | void;
  quickSavePinnedImageToDirectory(): Promise<void> | void;
  openPreferencesWindow(): Promise<void> | void;
  replacePinnedFromClipboard(): Promise<void> | void;
  toggleHoverToolbar(): void;
  movePinnedWindowByKeyboard(delta: PinnedPoint): Promise<void> | void;
  applyOpacityAction(action: PinnedKeyboardOpacityAction): void;
  applyZoomAction(action: PinnedKeyboardZoomAction): void;
  applyTransformAction(action: PinnedKeyboardTransformAction): void;
  applyVisualFilterAction(action: PinnedKeyboardVisualFilterAction): void;
}

export interface PinnedImageKeyboardContext {
  state: Pick<
    PinnedImageWorkspaceState,
    'image' | 'isHoverToolbarForcedVisible' | 'isMagnifierRequested' | 'cursorColor'
  >;
  actions: PinnedImageKeyboardActions;
}

export function handlePinnedImageKeyDown(
  event: KeyboardEvent,
  context: PinnedImageKeyboardContext,
) {
  // Move the current branching from index.tsx here, preserving order.
}

export function handlePinnedImageKeyUp(
  event: KeyboardEvent,
  actions: Pick<PinnedImageKeyboardActions, 'setMagnifierRequested'>,
) {
  if (event.key === 'Alt') actions.setMagnifierRequested(false);
}
```

Do not call Tauri adapters from this runtime. All side effects go through `actions`.

- [ ] **Step 4: Rewire `index.tsx` keydown/keyup effects**

Replace both current keydown effects with one effect:

```ts
useEffect(() => {
  const handleKeyDown = (event: KeyboardEvent) => {
    handlePinnedImageKeyDown(event, {
      state: {
        image,
        isHoverToolbarForcedVisible,
        isMagnifierRequested,
        cursorColor,
      },
      actions: keyboardActions,
    });
  };
  const handleKeyUp = (event: KeyboardEvent) => {
    handlePinnedImageKeyUp(event, keyboardActions);
  };

  window.addEventListener('keydown', handleKeyDown);
  window.addEventListener('keyup', handleKeyUp);
  return () => {
    window.removeEventListener('keydown', handleKeyDown);
    window.removeEventListener('keyup', handleKeyUp);
  };
}, [keyboardActions, image, isHoverToolbarForcedVisible, isMagnifierRequested, cursorColor]);
```

Build `keyboardActions` in `index.tsx` from existing callbacks and setters.

- [ ] **Step 5: Verify keyboard-focused behavior**

Run:

```bash
npm test -- pinnedImageKeyboardRuntime.test.ts pinControls.test.ts pinActions.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/PinnedImageWindow/pinnedImageKeyboardRuntime.ts src/components/PinnedImageWindow/pinnedImageKeyboardRuntime.test.ts src/components/PinnedImageWindow/index.tsx
git commit -m "refactor(pin): extract pinned keyboard runtime"
```

---

## Task 4: Extract Pointer and Wheel Runtime

**Files:**
- Create: `src/components/PinnedImageWindow/pinnedImagePointerRuntime.ts`
- Create: `src/components/PinnedImageWindow/pinnedImagePointerRuntime.test.ts`
- Modify: `src/components/PinnedImageWindow/index.tsx`

- [ ] **Step 1: Write failing pointer runtime tests**

Cover:
- wheel with no image does nothing and does not prevent default
- zoom wheel clears context menu and calls zoom action
- Cmd/Ctrl wheel clears context menu and applies opacity action
- Alt wheel is ignored because `getPinnedWheelAction` returns `null`
- context menu prevent-defaults and clamps to viewport
- pointer move maps viewport point into image-local point
- pointer leave clears both pointer points
- double click toggles thumbnail mode
- middle click resets size and opacity
- plain left pointer starts drag
- double click without modifiers hides current pin

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
npm test -- pinnedImagePointerRuntime.test.ts
```

Expected: FAIL because the runtime does not exist.

- [ ] **Step 3: Implement `pinnedImagePointerRuntime.ts`**

Export:

```ts
import {
  getPinnedContextMenuPosition,
  getPinnedImagePointFromPointer,
  getPinnedWheelAction,
  isClosePinnedImageDoubleClick,
  isResetPinnedImagePointer,
  isTogglePinnedThumbnailModeDoubleClick,
  type PinnedFrameRect,
  type PinnedPoint,
  type PinnedSize,
} from './pinControls';

export interface PinnedImagePointerActions {
  clearContextMenu(): void;
  adjustPinnedZoom(wheelDirection: number): void;
  adjustPinnedOpacity(wheelDirection: number): void;
  setContextMenuPosition(point: PinnedPoint): void;
  setViewportPointerPoint(point: PinnedPoint | null): void;
  setImagePointerPoint(point: PinnedPoint | null): void;
  togglePinnedThumbnailMode(): void;
  hideCurrentPinnedImage(): Promise<void> | void;
  resetPinnedSizeAndOpacity(): void;
  startPinnedWindowDrag(): Promise<void> | void;
}

export function handlePinnedImageWheel(
  event: Pick<WheelEvent, 'deltaY' | 'metaKey' | 'ctrlKey' | 'altKey' | 'preventDefault'>,
  hasImage: boolean,
  actions: Pick<
    PinnedImagePointerActions,
    'clearContextMenu' | 'adjustPinnedZoom' | 'adjustPinnedOpacity'
  >,
) {
  // Move current wheel branching here.
}

export function nextPinnedContextMenuPosition(
  point: PinnedPoint,
  menuSize: PinnedSize,
  viewportSize: PinnedSize,
) {
  return getPinnedContextMenuPosition(point, menuSize, viewportSize);
}

export function handlePinnedImagePointerMove(
  point: PinnedPoint,
  frame: PinnedFrameRect | null,
  actions: Pick<PinnedImagePointerActions, 'setViewportPointerPoint' | 'setImagePointerPoint'>,
) {
  if (!frame) return;
  actions.setViewportPointerPoint(point);
  actions.setImagePointerPoint(getPinnedImagePointFromPointer(point, frame));
}

export function handlePinnedImagePointerLeave(
  actions: Pick<PinnedImagePointerActions, 'setViewportPointerPoint' | 'setImagePointerPoint'>,
) {
  actions.setViewportPointerPoint(null);
  actions.setImagePointerPoint(null);
}

export function handlePinnedImagePointerDown(
  event: PointerEvent,
  actions: Pick<
    PinnedImagePointerActions,
    'togglePinnedThumbnailMode' | 'hideCurrentPinnedImage' | 'resetPinnedSizeAndOpacity' | 'startPinnedWindowDrag'
  >,
) {
  // Move current image onPointerDown branching here.
}
```

Keep this runtime pure. It should not read `window.innerWidth`, refs, or Tauri windows directly.

- [ ] **Step 4: Rewire pointer/wheel handlers in `index.tsx`**

`index.tsx` should:
- pass `Boolean(image)` to `handlePinnedImageWheel`
- pass viewport size `{ width: window.innerWidth, height: window.innerHeight }` to context-menu helper
- read `imageFrameRef.current?.getBoundingClientRect()` before calling pointer move runtime
- keep async side effects inside injected action callbacks

- [ ] **Step 5: Verify pointer-focused behavior**

Run:

```bash
npm test -- pinnedImagePointerRuntime.test.ts pinControls.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/PinnedImageWindow/pinnedImagePointerRuntime.ts src/components/PinnedImageWindow/pinnedImagePointerRuntime.test.ts src/components/PinnedImageWindow/index.tsx
git commit -m "refactor(pin): extract pinned pointer runtime"
```

---

## Task 5: Extract Render-Only View

**Files:**
- Create: `src/components/PinnedImageWindow/PinnedImageWindowView.tsx`
- Modify: `src/components/PinnedImageWindow/index.tsx`

- [ ] **Step 1: Add a view extraction guard**

Before extraction, run:

```bash
rg "useState|useEffect|useMemo|useCallback|getCurrentApp|getWebviewWindowByLabel|getPinnedImage|writeClipboardText" src/components/PinnedImageWindow/index.tsx
```

Expected: matches exist in `index.tsx`.

After extraction, this command should have no matches in `PinnedImageWindowView.tsx`.

- [ ] **Step 2: Create `PinnedImageWindowView.tsx`**

Move only JSX and style assembly into the view. Export a prop interface like:

```ts
import type { PointerEvent, Ref, WheelEvent } from 'react';
import type { PinnedHoverToolbarAction } from './pinActions';
import type { PinnedPoint, PinnedSize, PinnedTransform, PinnedVisualFilter } from './pinControls';
import type { PinnedImageView } from '../ScreenshotSession/types';
import type { ColorSample } from '../ScreenshotSession/colorSampler';

export interface PinnedImageWindowViewProps {
  error: string | null;
  image: PinnedImageView | null;
  imageFrameRef: Ref<HTMLDivElement>;
  imageFrameSize: PinnedSize | null;
  opacity: number;
  transform: PinnedTransform;
  visualFilter: PinnedVisualFilter;
  isMagnifierShown: boolean;
  magnifierPosition: PinnedPoint | null;
  magnifierImageStyle: React.CSSProperties | null;
  cursorColor: ColorSample | null;
  colorText: string;
  contextMenuPosition: PinnedPoint | null;
  zoom: number;
  hoverToolbarActions: PinnedHoverToolbarAction[];
  hoverToolbarVisibilityClassName: string;
  onWheel(event: WheelEvent<HTMLDivElement>): void;
  onContextMenu(event: React.MouseEvent<HTMLDivElement>): void;
  onDismissContextMenu(): void;
  onImagePointerMove(event: PointerEvent<HTMLDivElement>): void;
  onImagePointerLeave(): void;
  onImagePointerDown(event: PointerEvent<HTMLImageElement>): void;
  onResetPinnedSize(): void;
  onSetPinnedOpacityPreset(opacity: number): void;
  onMovePinnedToAnotherGroup(): void | Promise<void>;
  onHideCurrentPinnedImage(): void | Promise<void>;
  onHideCurrentPinnedImageGroup(): void | Promise<void>;
  onCopyCurrentPinnedImage(): void | Promise<void>;
  onSavePinnedImageAs(): void | Promise<void>;
  onDestroyCurrentPinnedImage(): void | Promise<void>;
  onDestroyCurrentPinnedImageGroup(): void | Promise<void>;
  onRunHoverToolbarAction(actionId: PinnedHoverToolbarAction['id']): void;
}
```

The view may import presentation helpers from `pinControls.ts` and magnifier helpers only if they are pure style helpers. Prefer passing derived values from `index.tsx` if a helper needs `window`, refs, or state decisions.

- [ ] **Step 3: Rewire `index.tsx` to render the view**

`index.tsx` should compute:
- `imageFrameSize`
- `isMagnifierShown`
- `magnifierPosition`
- `magnifierImageStyle`
- `colorText`
- `hoverToolbarVisibilityClassName`
- all action handlers

Then return:

```tsx
return (
  <PinnedImageWindowView
    error={error}
    image={image}
    imageFrameRef={imageFrameRef}
    imageFrameSize={imageFrameSize}
    // pass the remaining props explicitly
  />
);
```

- [ ] **Step 4: Verify view is render-only**

Run:

```bash
rg "useState|useEffect|useMemo|useCallback|getCurrentApp|getWebviewWindowByLabel|getPinnedImage|writeClipboardText" src/components/PinnedImageWindow/PinnedImageWindowView.tsx
```

Expected: no matches.

- [ ] **Step 5: Verify frontend**

Run:

```bash
npm test -- pinnedImageWorkspaceState.test.ts pinnedImageKeyboardRuntime.test.ts pinnedImagePointerRuntime.test.ts pinControls.test.ts pinActions.test.ts pinWindowPermissions.test.ts
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/PinnedImageWindow/PinnedImageWindowView.tsx src/components/PinnedImageWindow/index.tsx
git commit -m "refactor(pin): split pinned image window view"
```

---

## Task 6: Architecture Docs and Residue Cleanup

**Files:**
- Modify: `ARCHITECTURE.md`
- Modify: `CONTEXT.md`
- Modify: `src/components/PinnedImageWindow/index.tsx` only for residue cleanup if needed

- [ ] **Step 1: Run architecture residue searches**

Run:

```bash
wc -l src/components/PinnedImageWindow/index.tsx src/components/PinnedImageWindow/PinnedImageWindowView.tsx
rg "useState|useEffect|useMemo|useCallback" src/components/PinnedImageWindow/PinnedImageWindowView.tsx
rg "getPinnedKeyboard|isCopyPinned|isSavePinned|isQuickSavePinned|isClosePinned|isReplacePinned|isPinnedMagnifier" src/components/PinnedImageWindow/index.tsx
rg "onPointerDown=|onWheel=|onContextMenu=|onPointerMove=" src/components/PinnedImageWindow/index.tsx
```

Expected:
- `index.tsx` is materially smaller than 925 lines.
- `PinnedImageWindowView.tsx` has no hook ownership.
- Keyboard helper references are concentrated in `pinnedImageKeyboardRuntime.ts`.
- Pointer/wheel branching is concentrated in `pinnedImagePointerRuntime.ts`.

- [ ] **Step 2: Update `CONTEXT.md`**

Add a concise term:

```md
### Pinned Image Workspace（贴图工作区）
`src/components/PinnedImageWindow/*` 中的前端贴图窗口 interaction module。

**职责：**
- 拥有贴图窗口 state、image frame refs、sample canvas hydration 和颜色取样状态
- 通过 keyboard / pointer runtime 统一处理 Snipaste 风格快捷键、滚轮、拖拽、右键菜单和双击动作
- 通过 render-only view 展示图片、放大镜、右键菜单和 hover toolbar
- 不负责后端贴图状态、文件保存、剪贴板图片写入；这些仍走 `src/tauri/pinnedImage.ts` 和 `pinActions.ts`
```

- [ ] **Step 3: Update `ARCHITECTURE.md`**

In the frontend runtime seam list, add:

```md
- `src/components/PinnedImageWindow/*` 是 Pinned Image Workspace seam，拥有贴图窗口 state、keyboard/pointer dispatch、window action composition 和 render-only view；`pinActions.ts` 保持为前端 Tauri Adapter action helper。
```

In the directory tree, sharpen:

```md
└─ PinnedImageWindow/           # 贴图窗口 workspace + render-only view
```

- [ ] **Step 4: Full verification**

Run:

```bash
npm test
npm run build
```

Expected: PASS.

- [ ] **Step 5: Final diff review**

Run:

```bash
git diff --stat
git diff -- src/components/PinnedImageWindow docs/superpowers/plans/2026-07-09-pinned-image-workspace.md ARCHITECTURE.md CONTEXT.md
```

Confirm every changed line traces to:
- Pinned Image Workspace state
- keyboard runtime
- pointer runtime
- render-only view extraction
- architecture docs

- [ ] **Step 6: Commit**

```bash
git add src/components/PinnedImageWindow ARCHITECTURE.md CONTEXT.md
git commit -m "docs(pin): document pinned image workspace seam"
```

---

## Final Verification and Handoff

- [ ] **Step 1: Verify branch commits**

Run:

```bash
git log --oneline --decorate -n 8
git status --short
```

Expected:
- Recent commits show the task commits above.
- `git status --short` has no unrelated changes.

- [ ] **Step 2: Report residual risks**

Mention:
- This is a behavior-preserving frontend refactor, so manual smoke testing is still useful for real Tauri window drag/resize behavior.
- Automated tests cover runtime decision logic; `npm run build` covers TS/React integration.

- [ ] **Step 3: Optional manual smoke test**

If running the app is practical:

```bash
npm run tauri:dev
```

Manually check:
- create a pin from screenshot flow
- wheel zoom
- Cmd/Ctrl wheel opacity
- drag window
- right-click menu
- hover toolbar copy/save/close
- Alt magnifier and color copy
- Cmd/Ctrl+C, Cmd/Ctrl+S, Cmd/Ctrl+Shift+S, Cmd/Ctrl+W

Do not block merge solely on manual smoke if automated verification passes and no local GUI is available.
