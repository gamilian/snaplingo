import { describe, expect, it, vi } from 'vitest';

import { emptyAnnotationHistory } from './annotationHistory';
import type { ColorSampleFormat } from './colorSampler';
import {
  createInitialCaptureWorkspaceState,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import {
  handleCaptureWorkspaceKeyDown,
  type CaptureWorkspaceKeyboardActions,
  type CaptureWorkspaceKeyboardContext,
  type CaptureWorkspaceKeyboardDerivedState,
  type CaptureWorkspaceKeyboardRefs,
} from './captureWorkspaceKeyboard';
import type {
  AnnotationCommand,
  CaptureSessionView,
  LogicalRect,
  Point,
} from './types';

function createRef<Value>(current: Value) {
  return { current };
}

function createKeyboardEvent(
  key: string,
  overrides: Partial<KeyboardEvent> = {},
  calls?: string[],
): KeyboardEvent {
  return {
    key,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    repeat: false,
    preventDefault: vi.fn(() => {
      calls?.push('preventDefault');
    }),
    ...overrides,
  } as unknown as KeyboardEvent;
}

function createActions(
  overrides: Partial<CaptureWorkspaceKeyboardActions> = {},
): CaptureWorkspaceKeyboardActions {
  return {
    dismissCaptureLayer: vi.fn(),
    refreshSession: vi.fn(),
    setIncludeCapturedCursor: vi.fn(),
    clearPreviewImage: vi.fn(),
    renderSelectionPreview: vi.fn(),
    setIsMagnifierRequested: vi.fn(),
    clearAnnotations: vi.fn(),
    undoPolylineGesturePoint: vi.fn(),
    undoAnnotation: vi.fn(),
    redoAnnotation: vi.fn(),
    deleteSelectedAnnotation: vi.fn(),
    copyCurrentColor: vi.fn(),
    setColorSampleFormat: vi.fn(),
    restoreSelectionFromHistory: vi.fn(),
    restoreLastSelection: vi.fn(),
    setCursorPoint: vi.fn(),
    setSelection: vi.fn(),
    scheduleSelectionOverlayPaint: vi.fn(),
    setPreviewImageBase64: vi.fn(),
    setRenderingOutput: vi.fn(),
    setEditGesture: vi.fn(),
    syncHoverSelection: vi.fn(),
    selectFullCaptureArea: vi.fn(),
    completeCandidateSelection: vi.fn(),
    setIsAnnotationToolbarVisible: vi.fn(),
    completePreviewSelection: vi.fn(),
    adjustAnnotationSize: vi.fn(),
    toggleAnnotationFill: vi.fn(),
    setActiveAnnotationTool: vi.fn(),
    setSelectedAnnotationIndex: vi.fn(),
    setAnnotationGesture: vi.fn(),
    setAnnotationMoveGesture: vi.fn(),
    setDraftAnnotation: vi.fn(),
    selectAnnotationColor: vi.fn(),
    toggleAnnotationTool: vi.fn(),
    setDraftSelectionMoveGesture: vi.fn(),
    setAnnotationHistory: vi.fn(),
    ...overrides,
  };
}

function createRefs(
  overrides: Partial<CaptureWorkspaceKeyboardRefs> = {},
): CaptureWorkspaceKeyboardRefs {
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

function createContext({
  state: stateOverrides = {},
  refs: refOverrides = {},
  derived: derivedOverrides = {},
  actions: actionOverrides = {},
}: {
  state?: Partial<CaptureWorkspaceState>;
  refs?: Partial<CaptureWorkspaceKeyboardRefs>;
  derived?: Partial<CaptureWorkspaceKeyboardDerivedState>;
  actions?: Partial<CaptureWorkspaceKeyboardActions>;
} = {}) {
  const state: CaptureWorkspaceState = {
    ...createInitialCaptureWorkspaceState(),
    ...stateOverrides,
  };
  const refs = createRefs(refOverrides);
  const actions = createActions(actionOverrides);
  const derived: CaptureWorkspaceKeyboardDerivedState = {
    annotations: state.annotationHistory.annotations,
    captureCandidates: [],
    selectionBounds: null,
    hasAnnotationEditingContext:
      state.activeAnnotationTool !== null ||
      state.selectedAnnotationIndex !== null,
    isAnnotationToolbarVisible: state.isAnnotationToolbarVisible,
    isMagnifierShown: false,
    isFillModeActive: false,
    cursorColor: state.cursorColor,
    ...derivedOverrides,
  };
  const context: CaptureWorkspaceKeyboardContext = {
    state,
    refs,
    derived,
    actions,
  };

  return { actions, context, derived, refs, state };
}

const selection: LogicalRect = { x: 10, y: 20, width: 120, height: 80 };

const sessionWithCapturedCursor: CaptureSessionView = {
  id: 'session-1',
  monitors: [],
  candidates: [],
  captured_cursor: {
    logical_position: { x: 12, y: 24 },
    hotspot: { x: 1, y: 1 },
    image_width: 16,
    image_height: 16,
    scale_factor: 1,
    image_base64: 'cursor-image',
  },
};

describe('handleCaptureWorkspaceKeyDown', () => {
  it('prevents Escape before dispatching the capture dismiss action', () => {
    const calls: string[] = [];
    const { actions, context } = createContext({
      actions: {
        dismissCaptureLayer: vi.fn(() => {
          calls.push('dismissCaptureLayer');
        }),
      },
    });
    const event = createKeyboardEvent('Escape', {}, calls);

    handleCaptureWorkspaceKeyDown(event, context);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(actions.dismissCaptureLayer).toHaveBeenCalledOnce();
    expect(calls).toEqual(['preventDefault', 'dismissCaptureLayer']);
  });

  it('refreshes from F5 only while selecting or previewing', () => {
    for (const status of ['selecting', 'preview'] as const) {
      const { actions, context } = createContext({ state: { status } });
      const event = createKeyboardEvent('F5');

      handleCaptureWorkspaceKeyDown(event, context);

      expect(event.preventDefault).toHaveBeenCalledOnce();
      expect(actions.refreshSession).toHaveBeenCalledOnce();
    }

    for (const status of ['idle', 'loading', 'error'] as const) {
      const { actions, context } = createContext({ state: { status } });
      const event = createKeyboardEvent('F5');

      handleCaptureWorkspaceKeyDown(event, context);

      expect(event.preventDefault).not.toHaveBeenCalled();
      expect(actions.refreshSession).not.toHaveBeenCalled();
    }
  });

  it('rerenders the existing preview when the captured cursor is toggled', () => {
    const annotations: AnnotationCommand[] = [
      {
        type: 'rectangle',
        rect: { x: 2, y: 4, width: 12, height: 8 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
    ];
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        session: sessionWithCapturedCursor,
        selection,
        includeCapturedCursor: false,
        annotationHistory: {
          ...emptyAnnotationHistory(),
          annotations,
        },
      },
      derived: {
        annotations,
      },
    });
    const event = createKeyboardEvent('`');

    handleCaptureWorkspaceKeyDown(event, context);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(actions.setIncludeCapturedCursor).toHaveBeenCalledWith(true);
    expect(actions.clearPreviewImage).toHaveBeenCalledOnce();
    expect(actions.renderSelectionPreview).toHaveBeenCalledWith(
      selection,
      annotations,
      true,
    );
  });

  it('updates annotation history and rerenders preview when nudging the selected annotation', () => {
    const arrow: AnnotationCommand = {
      type: 'arrow',
      start: { x: 3, y: 4 },
      end: { x: 20, y: 24 },
      color: [255, 77, 79, 255],
      stroke_width: 2,
    };
    const movedArrow: AnnotationCommand = {
      ...arrow,
      start: { x: 4, y: 4 },
      end: { x: 21, y: 24 },
    };
    const annotationHistory = {
      ...emptyAnnotationHistory(),
      annotations: [arrow],
    };
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        selectedAnnotationIndex: 0,
        annotationHistory,
      },
      derived: {
        annotations: [arrow],
        hasAnnotationEditingContext: true,
      },
    });
    const event = createKeyboardEvent('ArrowRight');

    handleCaptureWorkspaceKeyDown(event, context);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(actions.setAnnotationHistory).toHaveBeenCalledWith({
      annotations: [movedArrow],
      undoneAnnotations: [],
      undoSnapshots: [[arrow]],
      redoSnapshots: [],
    });
    expect(actions.renderSelectionPreview).toHaveBeenCalledWith(selection, [
      movedArrow,
    ]);
  });

  it('prioritizes selected annotation arrow nudges over selection arrow preview movement', () => {
    const arrow: AnnotationCommand = {
      type: 'arrow',
      start: { x: 3, y: 4 },
      end: { x: 20, y: 24 },
      color: [255, 77, 79, 255],
      stroke_width: 2,
    };
    const movedArrow: AnnotationCommand = {
      ...arrow,
      start: { x: 4, y: 4 },
      end: { x: 21, y: 24 },
    };
    const movedAnnotations = [movedArrow];
    const annotationHistory = {
      ...emptyAnnotationHistory(),
      annotations: [arrow],
    };
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        selectedAnnotationIndex: 0,
        annotationHistory,
      },
      derived: {
        annotations: [arrow],
        hasAnnotationEditingContext: true,
        selectionBounds: { x: 0, y: 0, width: 300, height: 200 },
      },
    });
    const event = createKeyboardEvent('ArrowRight');

    handleCaptureWorkspaceKeyDown(event, context);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(actions.setAnnotationHistory).toHaveBeenCalledWith({
      annotations: movedAnnotations,
      undoneAnnotations: [],
      undoSnapshots: [[arrow]],
      redoSnapshots: [],
    });
    expect(actions.renderSelectionPreview).toHaveBeenCalledWith(
      selection,
      movedAnnotations,
    );
    expect(actions.setSelection).not.toHaveBeenCalled();
    expect(actions.setPreviewImageBase64).not.toHaveBeenCalled();
  });

  it('completes the active hover candidate from the hover-selection ref', () => {
    const hoverSelection: LogicalRect = { x: 40, y: 50, width: 60, height: 70 };
    const { actions, context } = createContext({
      state: {
        status: 'selecting',
        mode: 'screenshot',
      },
      refs: {
        hoverSelectionRef: createRef(hoverSelection),
      },
    });
    const event = createKeyboardEvent('Enter');

    handleCaptureWorkspaceKeyDown(event, context);

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(actions.completeCandidateSelection).toHaveBeenCalledWith(
      hoverSelection,
      'copy',
    );
  });

  it('copies the current color only while the magnifier is shown', () => {
    const cursorColor = { hex: '#112233', red: 17, green: 34, blue: 51 };
    const hidden = createContext({
      state: {
        status: 'preview',
        cursorColor,
      },
      derived: {
        cursorColor,
        isMagnifierShown: false,
      },
    });
    const hiddenEvent = createKeyboardEvent('c');

    handleCaptureWorkspaceKeyDown(hiddenEvent, hidden.context);

    expect(hiddenEvent.preventDefault).not.toHaveBeenCalled();
    expect(hidden.actions.copyCurrentColor).not.toHaveBeenCalled();

    const shown = createContext({
      state: {
        status: 'preview',
        cursorColor,
      },
      derived: {
        cursorColor,
        isMagnifierShown: true,
      },
    });
    const shownEvent = createKeyboardEvent('c');

    handleCaptureWorkspaceKeyDown(shownEvent, shown.context);

    expect(shownEvent.preventDefault).toHaveBeenCalledOnce();
    expect(shown.actions.copyCurrentColor).toHaveBeenCalledOnce();
  });

  it('toggles color sample format only while the magnifier is shown', () => {
    const cursorColor = { hex: '#112233', red: 17, green: 34, blue: 51 };
    const hidden = createContext({
      state: {
        status: 'preview',
        cursorColor,
      },
      derived: {
        cursorColor,
        isMagnifierShown: false,
      },
    });
    const hiddenEvent = createKeyboardEvent('Shift', { shiftKey: true });

    handleCaptureWorkspaceKeyDown(hiddenEvent, hidden.context);

    expect(hiddenEvent.preventDefault).not.toHaveBeenCalled();
    expect(hidden.actions.setColorSampleFormat).not.toHaveBeenCalled();

    const shown = createContext({
      state: {
        status: 'preview',
        cursorColor,
      },
      derived: {
        cursorColor,
        isMagnifierShown: true,
      },
    });
    const shownEvent = createKeyboardEvent('Shift', { shiftKey: true });

    handleCaptureWorkspaceKeyDown(shownEvent, shown.context);

    expect(shownEvent.preventDefault).toHaveBeenCalledOnce();
    expect(shown.actions.setColorSampleFormat).toHaveBeenCalledOnce();

    const [formatUpdater] = vi.mocked(
      shown.actions.setColorSampleFormat,
    ).mock.calls[0];
    expect(formatUpdater('hex' satisfies ColorSampleFormat)).toBe('rgb');
    expect(formatUpdater('rgb' satisfies ColorSampleFormat)).toBe('hex');
  });
});
