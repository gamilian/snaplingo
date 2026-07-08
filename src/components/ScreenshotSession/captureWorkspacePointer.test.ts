import { describe, expect, it, vi } from 'vitest';

import type { CaptureCandidate } from './captureCandidates';
import {
  createInitialCaptureWorkspaceState,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import {
  handleCaptureWorkspacePointerDown,
  handleCaptureWorkspacePointerMove,
  handleCaptureWorkspacePointerUp,
  handleCaptureWorkspacePreviewPointerDown,
  handleCaptureWorkspaceResizePointerDown,
  handleCaptureWorkspaceWheel,
  type CaptureWorkspacePointerActions,
  type CaptureWorkspacePointerContext,
  type CaptureWorkspacePointerDerivedState,
  type CaptureWorkspacePointerEvent,
  type CaptureWorkspacePointerRefs,
  type CaptureWorkspaceWheelEvent,
} from './captureWorkspacePointer';
import type { LogicalRect, Point } from './types';

function createRef<Value>(current: Value) {
  return { current };
}

function createPointerEvent(
  overrides: Partial<CaptureWorkspacePointerEvent> = {},
  calls?: string[],
): CaptureWorkspacePointerEvent {
  const currentTarget = {
    setPointerCapture: vi.fn((pointerId: number) => {
      calls?.push(`setPointerCapture:${pointerId}`);
    }),
  };

  return {
    clientX: 0,
    clientY: 0,
    pointerId: 7,
    button: 0,
    detail: 1,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    preventDefault: vi.fn(() => {
      calls?.push('preventDefault');
    }),
    stopPropagation: vi.fn(() => {
      calls?.push('stopPropagation');
    }),
    currentTarget,
    ...overrides,
  };
}

function createWheelEvent(
  overrides: Partial<CaptureWorkspaceWheelEvent> = {},
): CaptureWorkspaceWheelEvent {
  return {
    deltaY: -1,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    preventDefault: vi.fn(),
    ...overrides,
  };
}

function createRefs(
  overrides: Partial<CaptureWorkspacePointerRefs> = {},
): CaptureWorkspacePointerRefs {
  return {
    startPointRef: createRef<Point | null>(null),
    cursorPointRef: createRef<Point | null>(null),
    draftSelectionRef: createRef<LogicalRect | null>(null),
    hoverSelectionRef: createRef<LogicalRect | null>(null),
    keyboardDraftCursorPointRef: createRef<Point | null>(null),
    keyboardEditCursorPointRef: createRef<Point | null>(null),
    ...overrides,
  };
}

function createActions(
  refs: CaptureWorkspacePointerRefs,
  overrides: Partial<CaptureWorkspacePointerActions> = {},
): CaptureWorkspacePointerActions {
  return {
    commitTextDraft: vi.fn(),
    commitAnnotationGestureAtPoint: vi.fn(),
    dismissCaptureLayer: vi.fn(),
    resetPreviewSelection: vi.fn(),
    cancelSession: vi.fn(),
    setCursorPoint: vi.fn(),
    setStartPointWithRef: vi.fn((point: Point | null) => {
      refs.startPointRef.current = point;
    }),
    setSelection: vi.fn(),
    setHoverSelection: vi.fn(),
    scheduleSelectionOverlayPaint: vi.fn(),
    setPreviewImageBase64: vi.fn(),
    setRenderingOutput: vi.fn(),
    setStatus: vi.fn(),
    setActiveAnnotationTool: vi.fn(),
    setAnnotationGesture: vi.fn(),
    setDraftAnnotation: vi.fn(),
    setSelectedAnnotationIndex: vi.fn(),
    setAnnotationMoveGesture: vi.fn(),
    setDraftSelectionMoveGesture: vi.fn(),
    setTextDraft: vi.fn(),
    setTextDraftAnnotationIndex: vi.fn(),
    setAnnotationHistory: vi.fn(),
    syncHoverSelection: vi.fn((selection: LogicalRect | null) => {
      refs.hoverSelectionRef.current = selection;
    }),
    renderSelectionPreview: vi.fn(),
    completeManualSelection: vi.fn(),
    pinSelection: vi.fn(),
    setEditGesture: vi.fn(),
    setAnnotationStyle: vi.fn(),
    setTextFontSize: vi.fn(),
    copySelection: vi.fn(),
    adjustAnnotationSize: vi.fn(),
    ...overrides,
  };
}

function createContext({
  state: stateOverrides = {},
  refs: refOverrides = {},
  derived: derivedOverrides = {},
  actions: actionOverrides = {},
}: {
  state?: Partial<CaptureWorkspaceState>;
  refs?: Partial<CaptureWorkspacePointerRefs>;
  derived?: Partial<CaptureWorkspacePointerDerivedState>;
  actions?: Partial<CaptureWorkspacePointerActions>;
} = {}) {
  const state: CaptureWorkspaceState = {
    ...createInitialCaptureWorkspaceState(),
    ...stateOverrides,
  };
  const refs = createRefs(refOverrides);
  const actions = createActions(refs, actionOverrides);
  const derived: CaptureWorkspacePointerDerivedState = {
    annotations: state.annotationHistory.annotations,
    captureCandidates: [],
    selectionBounds: null,
    snapTargetRects: [],
    hasAnnotationEditingContext:
      state.activeAnnotationTool !== null ||
      state.selectedAnnotationIndex !== null,
    shouldTrackMagnifierCursor: false,
    ...derivedOverrides,
  };
  const context: CaptureWorkspacePointerContext = {
    state,
    refs,
    derived,
    actions,
  };

  return { actions, context, derived, refs, state };
}

const selectionBounds: LogicalRect = { x: 0, y: 0, width: 500, height: 400 };
const selection: LogicalRect = { x: 50, y: 60, width: 120, height: 90 };

describe('capture workspace pointer dispatch', () => {
  it('starts a draft selection from root pointer down and captures the pointer', () => {
    const { actions, context, refs } = createContext({
      state: { status: 'selecting' },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ clientX: 20, clientY: 30 });

    handleCaptureWorkspacePointerDown(event, context);

    expect(event.currentTarget.setPointerCapture).toHaveBeenCalledWith(7);
    expect(refs.cursorPointRef.current).toEqual({ x: 20, y: 30 });
    expect(refs.startPointRef.current).toEqual({ x: 20, y: 30 });
    expect(refs.draftSelectionRef.current).toEqual({
      x: 20,
      y: 30,
      width: 0,
      height: 0,
    });
    expect(actions.setCursorPoint).toHaveBeenCalledWith({ x: 20, y: 30 });
    expect(actions.setSelection).toHaveBeenCalledWith(null);
    expect(actions.scheduleSelectionOverlayPaint).toHaveBeenCalledWith(
      { x: 20, y: 30, width: 0, height: 0 },
      null,
    );
    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(event.stopPropagation).not.toHaveBeenCalled();
  });

  it('updates the hover candidate on pointer move when not drafting', () => {
    const hoverCandidate: CaptureCandidate = {
      id: 'window-1',
      kind: 'window',
      rect: { x: 10, y: 10, width: 120, height: 80 },
      priority: 10,
    };
    const { actions, context, refs } = createContext({
      state: { status: 'selecting' },
      derived: {
        selectionBounds,
        captureCandidates: [hoverCandidate],
      },
    });
    const event = createPointerEvent({ clientX: 24, clientY: 32 });

    handleCaptureWorkspacePointerMove(event, context);

    expect(refs.cursorPointRef.current).toEqual({ x: 24, y: 32 });
    expect(actions.syncHoverSelection).toHaveBeenCalledWith(hoverCandidate.rect);
    expect(actions.scheduleSelectionOverlayPaint).toHaveBeenCalledWith();
    expect(actions.setCursorPoint).not.toHaveBeenCalled();
  });

  it('updates the draft selection on pointer move while dragging', () => {
    const startPoint = { x: 10, y: 12 };
    const { actions, context, refs } = createContext({
      state: { status: 'selecting', startPoint },
      refs: { startPointRef: createRef<Point | null>(startPoint) },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ clientX: 42, clientY: 52 });

    handleCaptureWorkspacePointerMove(event, context);

    expect(refs.cursorPointRef.current).toEqual({ x: 42, y: 52 });
    expect(refs.keyboardDraftCursorPointRef.current).toBeNull();
    expect(refs.draftSelectionRef.current).toEqual({
      x: 10,
      y: 12,
      width: 32,
      height: 40,
    });
    expect(actions.scheduleSelectionOverlayPaint).toHaveBeenLastCalledWith(
      { x: 10, y: 12, width: 32, height: 40 },
      null,
    );
  });

  it('commits a draft selection through manual completion on pointer up', () => {
    const startPoint = { x: 10, y: 12 };
    const { actions, context, refs } = createContext({
      state: { status: 'selecting', startPoint },
      refs: {
        startPointRef: createRef<Point | null>(startPoint),
        draftSelectionRef: createRef<LogicalRect | null>({
          x: 10,
          y: 12,
          width: 32,
          height: 40,
        }),
      },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ clientX: 42, clientY: 52 });

    handleCaptureWorkspacePointerUp(event, context);

    expect(actions.setCursorPoint).toHaveBeenCalledWith({ x: 42, y: 52 });
    expect(actions.setDraftSelectionMoveGesture).toHaveBeenCalledWith(null);
    expect(actions.setStartPointWithRef).toHaveBeenCalledWith(null);
    expect(refs.startPointRef.current).toBeNull();
    expect(refs.draftSelectionRef.current).toBeNull();
    expect(actions.completeManualSelection).toHaveBeenCalledWith({
      x: 10,
      y: 12,
      width: 32,
      height: 40,
    });
  });

  it('starts an annotation tool gesture from preview pointer down', () => {
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        activeAnnotationTool: 'rectangle',
      },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ clientX: 82, clientY: 96 });

    handleCaptureWorkspacePreviewPointerDown(event, context);

    expect(event.stopPropagation).toHaveBeenCalledOnce();
    expect(event.currentTarget.setPointerCapture).toHaveBeenCalledWith(7);
    expect(actions.setCursorPoint).toHaveBeenCalledWith({ x: 82, y: 96 });
    expect(actions.setSelectedAnnotationIndex).toHaveBeenCalledWith(null);
    expect(actions.setAnnotationGesture).toHaveBeenCalledWith(
      expect.objectContaining({
        tool: 'rectangle',
        startPoint: { x: 32, y: 36 },
      }),
    );
    expect(actions.setDraftAnnotation).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'rectangle' }),
    );
  });

  it('starts a selection edit gesture from resize handle pointer down', () => {
    const { actions, context } = createContext({
      state: { status: 'preview', selection },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ clientX: 170, clientY: 150 });

    handleCaptureWorkspaceResizePointerDown('se', event, context);

    expect(event.stopPropagation).toHaveBeenCalledOnce();
    expect(event.currentTarget.setPointerCapture).toHaveBeenCalledWith(7);
    expect(actions.setCursorPoint).toHaveBeenCalledWith({ x: 170, y: 150 });
    expect(actions.setEditGesture).toHaveBeenCalledWith({
      type: 'resize',
      handle: 'se',
      startPoint: { x: 170, y: 150 },
      startSelection: selection,
    });
    expect(actions.setPreviewImageBase64).toHaveBeenCalledWith(null);
  });

  it('adjusts annotation size from wheel input only when the state allows it', () => {
    const allowed = createContext({
      state: { status: 'preview' },
      derived: { hasAnnotationEditingContext: true },
    });
    const allowedEvent = createWheelEvent({ deltaY: -4 });

    handleCaptureWorkspaceWheel(allowedEvent, allowed.context);

    expect(allowedEvent.preventDefault).toHaveBeenCalledOnce();
    expect(allowed.actions.adjustAnnotationSize).toHaveBeenCalledWith(
      'increase',
    );

    const blocked = createContext({
      state: { status: 'selecting' },
      derived: { hasAnnotationEditingContext: true },
    });
    const blockedEvent = createWheelEvent({ deltaY: -4 });

    handleCaptureWorkspaceWheel(blockedEvent, blocked.context);

    expect(blockedEvent.preventDefault).not.toHaveBeenCalled();
    expect(blocked.actions.adjustAnnotationSize).not.toHaveBeenCalled();
  });
});
