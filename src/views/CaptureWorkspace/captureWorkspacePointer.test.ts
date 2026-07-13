import { describe, expect, it, vi } from 'vitest';

import {
  createInitialCaptureWorkspaceState,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import {
  handleCaptureWorkspaceEditorPointerDown,
  handleCaptureWorkspaceEditorPointerMove,
  handleCaptureWorkspaceEditorPointerUp,
  handleCaptureWorkspaceEditorPreviewPointerDown,
  handleCaptureWorkspaceEditorResizePointerDown,
  handleCaptureWorkspaceEditorWheel,
  type CaptureWorkspacePointerDerivedState,
  type CaptureWorkspacePointerEditorActions,
  type CaptureWorkspacePointerEditorContext,
  type CaptureWorkspacePointerEvent,
  type CaptureWorkspacePointerRefs,
  type CaptureWorkspaceWheelEvent,
} from './captureWorkspacePointer';
import type { LogicalRect, Point } from './types';

function createPointerEvent(
  overrides: Partial<CaptureWorkspacePointerEvent> = {},
): CaptureWorkspacePointerEvent {
  return {
    clientX: 0,
    clientY: 0,
    button: 0,
    detail: 1,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
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

function createActions(
  overrides: Partial<CaptureWorkspacePointerEditorActions> = {},
): CaptureWorkspacePointerEditorActions {
  return {
    commitTextDraft: vi.fn(),
    commitAnnotationGestureAtPoint: vi.fn(),
    dismissCaptureLayer: vi.fn(),
    adjustAnnotationSize: vi.fn(),
    renderSelectionPreview: vi.fn(),
    setCursorPoint: vi.fn(),
    setSelection: vi.fn(),
    scheduleSelectionOverlayPaint: vi.fn(),
    setPreviewImageBase64: vi.fn(),
    setRenderingOutput: vi.fn(),
    setStatus: vi.fn(),
    setAnnotationGesture: vi.fn(),
    setDraftAnnotation: vi.fn(),
    setSelectedAnnotationIndex: vi.fn(),
    setAnnotationMoveGesture: vi.fn(),
    setTextDraft: vi.fn(),
    setTextDraftAnnotationIndex: vi.fn(),
    setAnnotationHistory: vi.fn(),
    setEditGesture: vi.fn(),
    setAnnotationStyle: vi.fn(),
    setTextFontSize: vi.fn(),
    ...overrides,
  };
}

function createContext({
  state: stateOverrides = {},
  derived: derivedOverrides = {},
  actions: actionOverrides = {},
}: {
  state?: Partial<CaptureWorkspaceState>;
  derived?: Partial<CaptureWorkspacePointerDerivedState>;
  actions?: Partial<CaptureWorkspacePointerEditorActions>;
} = {}) {
  const state: CaptureWorkspaceState = {
    ...createInitialCaptureWorkspaceState(),
    ...stateOverrides,
  };
  const refs: CaptureWorkspacePointerRefs = {
    cursorPointRef: { current: null as Point | null },
    keyboardEditCursorPointRef: { current: null as Point | null },
  };
  const actions = createActions(actionOverrides);
  const derived: CaptureWorkspacePointerDerivedState = {
    annotations: state.annotationHistory.annotations,
    selectionBounds: null,
    snapTargetRects: [],
    hasAnnotationEditingContext:
      state.activeAnnotationTool !== null ||
      state.selectedAnnotationIndex !== null,
    shouldTrackMagnifierCursor: false,
    ...derivedOverrides,
  };
  const context: CaptureWorkspacePointerEditorContext = {
    state,
    refs,
    derived,
    actions,
  };
  return { actions, context, refs };
}

const selectionBounds: LogicalRect = { x: 0, y: 0, width: 500, height: 400 };
const selection: LogicalRect = { x: 50, y: 60, width: 120, height: 90 };

describe('capture workspace editor pointer dispatch', () => {
  it('dismisses a preview editor layer from a cancel pointer', () => {
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        selectedAnnotationIndex: 0,
      },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ button: 2 });

    handleCaptureWorkspaceEditorPointerDown(event, context);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(event.stopPropagation).toHaveBeenCalledOnce();
    expect(actions.dismissCaptureLayer).toHaveBeenCalledOnce();
  });

  it('starts an annotation gesture from preview pointer down', () => {
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        activeAnnotationTool: 'rectangle',
      },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ clientX: 82, clientY: 96 });

    handleCaptureWorkspaceEditorPreviewPointerDown(event, context);

    expect(event.stopPropagation).toHaveBeenCalledOnce();
    expect(actions.setCursorPoint).toHaveBeenCalledWith({ x: 82, y: 96 });
    expect(actions.setAnnotationGesture).toHaveBeenCalledWith(
      expect.objectContaining({
        tool: 'rectangle',
        startPoint: { x: 32, y: 36 },
      }),
    );
  });

  it('ignores a selection click outside the active textarea', () => {
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        textDraft: { position: { x: 10, y: 12 }, text: 'Snap', fontSize: 24 },
      },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ clientX: 110, clientY: 120 });

    handleCaptureWorkspaceEditorPreviewPointerDown(event, context);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(event.stopPropagation).toHaveBeenCalledOnce();
    expect(actions.commitTextDraft).not.toHaveBeenCalled();
    expect(actions.setAnnotationGesture).not.toHaveBeenCalled();
  });

  it('ignores a preview click outside the selection while a textarea is active', () => {
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        textDraft: { position: { x: 10, y: 12 }, text: 'Snap', fontSize: 24 },
      },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ clientX: 10, clientY: 12 });

    handleCaptureWorkspaceEditorPointerDown(event, context);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(event.stopPropagation).toHaveBeenCalledOnce();
    expect(actions.commitTextDraft).not.toHaveBeenCalled();
  });

  it('updates an active selection edit without entering selecting workflows', () => {
    const editGesture = {
      type: 'move' as const,
      startPoint: { x: 80, y: 90 },
      startSelection: selection,
    };
    const { actions, context, refs } = createContext({
      state: { status: 'preview', selection, editGesture },
      derived: { selectionBounds },
    });

    handleCaptureWorkspaceEditorPointerMove(
      createPointerEvent({ clientX: 90, clientY: 100 }),
      context,
    );

    expect(refs.cursorPointRef.current).toEqual({ x: 90, y: 100 });
    expect(actions.setSelection).toHaveBeenCalledWith({
      x: 60,
      y: 70,
      width: 120,
      height: 90,
    });
    expect(actions.setPreviewImageBase64).toHaveBeenCalledWith(null);
  });

  it('updates freehand gesture and draft atomically during pointer movement', () => {
    const annotationGesture = {
      tool: 'pen' as const,
      startPoint: { x: 10, y: 10 },
      points: [{ x: 10, y: 10 }],
    };
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        annotationGesture,
      },
      derived: { selectionBounds },
    });

    handleCaptureWorkspaceEditorPointerMove(
      createPointerEvent({ clientX: 80, clientY: 100 }),
      context,
    );

    expect(actions.setAnnotationGesture).toHaveBeenCalledWith(
      expect.objectContaining({
        tool: 'pen',
        points: [
          { x: 10, y: 10 },
          { x: 30, y: 40 },
        ],
      }),
      expect.objectContaining({ type: 'freehand' }),
    );
    expect(actions.setDraftAnnotation).not.toHaveBeenCalled();
    expect(actions.scheduleSelectionOverlayPaint).not.toHaveBeenCalled();
  });

  it('commits a selection edit and rerenders its preview', () => {
    const editGesture = {
      type: 'move' as const,
      startPoint: { x: 80, y: 90 },
      startSelection: selection,
    };
    const { actions, context } = createContext({
      state: { status: 'preview', selection, editGesture },
      derived: { selectionBounds },
    });

    handleCaptureWorkspaceEditorPointerUp(
      createPointerEvent({ clientX: 90, clientY: 100 }),
      context,
    );

    const moved = { x: 60, y: 70, width: 120, height: 90 };
    expect(actions.setEditGesture).toHaveBeenCalledWith(null);
    expect(actions.setSelection).toHaveBeenCalledWith(moved);
    expect(actions.renderSelectionPreview).toHaveBeenCalledWith(moved, []);
  });

  it('starts a selection edit from a resize handle', () => {
    const { actions, context } = createContext({
      state: { status: 'preview', selection },
      derived: { selectionBounds },
    });
    const event = createPointerEvent({ clientX: 170, clientY: 150 });

    handleCaptureWorkspaceEditorResizePointerDown('se', event, context);

    expect(actions.setEditGesture).toHaveBeenCalledWith({
      type: 'resize',
      handle: 'se',
      startPoint: { x: 170, y: 150 },
      startSelection: selection,
    });
  });

  it('adjusts annotation size from wheel input only in editor state', () => {
    const allowed = createContext({
      state: { status: 'preview' },
      derived: { hasAnnotationEditingContext: true },
    });
    const allowedEvent = createWheelEvent({ deltaY: -4 });

    handleCaptureWorkspaceEditorWheel(allowedEvent, allowed.context);

    expect(allowedEvent.preventDefault).toHaveBeenCalledOnce();
    expect(allowed.actions.adjustAnnotationSize).toHaveBeenCalledWith(
      'increase',
    );

    const blocked = createContext({
      state: { status: 'selecting' },
      derived: { hasAnnotationEditingContext: true },
    });
    handleCaptureWorkspaceEditorWheel(createWheelEvent(), blocked.context);
    expect(blocked.actions.adjustAnnotationSize).not.toHaveBeenCalled();
  });
});
