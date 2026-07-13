import { describe, expect, it, vi } from 'vitest';

import { emptyAnnotationHistory } from './annotationHistory';
import type { ColorSampleFormat } from './colorSampler';
import {
  createInitialCaptureWorkspaceState,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import {
  handleCaptureWorkspaceEditorKeyDown,
  type CaptureWorkspaceKeyboardDerivedState,
  type CaptureWorkspaceKeyboardEditorActions,
  type CaptureWorkspaceKeyboardEditorContext,
  type CaptureWorkspaceKeyboardRefs,
} from './captureWorkspaceKeyboard';
import type { AnnotationCommand, LogicalRect, Point } from './types';

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
    preventDefault: vi.fn(() => calls?.push('preventDefault')),
    ...overrides,
  } as unknown as KeyboardEvent;
}

function createActions(
  overrides: Partial<CaptureWorkspaceKeyboardEditorActions> = {},
): CaptureWorkspaceKeyboardEditorActions {
  return {
    dismissCaptureLayer: vi.fn(),
    renderSelectionPreview: vi.fn(),
    setIsMagnifierRequested: vi.fn(),
    clearAnnotations: vi.fn(),
    undoAnnotation: vi.fn(),
    redoAnnotation: vi.fn(),
    deleteSelectedAnnotation: vi.fn(),
    copyCurrentColor: vi.fn(),
    setColorSampleFormat: vi.fn(),
    setCursorPoint: vi.fn(),
    setSelection: vi.fn(),
    setPreviewImageBase64: vi.fn(),
    setRenderingOutput: vi.fn(),
    setEditGesture: vi.fn(),
    setIsAnnotationToolbarVisible: vi.fn(),
    adjustAnnotationSize: vi.fn(),
    toggleAnnotationFill: vi.fn(),
    setActiveAnnotationTool: vi.fn(),
    setSelectedAnnotationIndex: vi.fn(),
    setAnnotationGesture: vi.fn(),
    setAnnotationMoveGesture: vi.fn(),
    setDraftAnnotation: vi.fn(),
    selectAnnotationColor: vi.fn(),
    toggleAnnotationTool: vi.fn(),
    setAnnotationHistory: vi.fn(),
    ...overrides,
  };
}

function createContext({
  state: stateOverrides = {},
  derived: derivedOverrides = {},
  actions: actionOverrides = {},
}: {
  state?: Partial<CaptureWorkspaceState>;
  derived?: Partial<CaptureWorkspaceKeyboardDerivedState>;
  actions?: Partial<CaptureWorkspaceKeyboardEditorActions>;
} = {}) {
  const state: CaptureWorkspaceState = {
    ...createInitialCaptureWorkspaceState(),
    ...stateOverrides,
  };
  const refs: CaptureWorkspaceKeyboardRefs = {
    keyboardEditCursorPointRef: { current: null as Point | null },
  };
  const actions = createActions(actionOverrides);
  const derived: CaptureWorkspaceKeyboardDerivedState = {
    annotations: state.annotationHistory.annotations,
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
  const context: CaptureWorkspaceKeyboardEditorContext = {
    state,
    refs,
    derived,
    actions,
  };
  return { actions, context };
}

const selection: LogicalRect = { x: 10, y: 20, width: 120, height: 80 };

describe('handleCaptureWorkspaceEditorKeyDown', () => {
  it('prevents Escape before dismissing the preview editor layer', () => {
    const calls: string[] = [];
    const { actions, context } = createContext({
      state: { status: 'preview' },
      actions: {
        dismissCaptureLayer: vi.fn(() => calls.push('dismissCaptureLayer')),
      },
    });
    const event = createKeyboardEvent('Escape', {}, calls);

    handleCaptureWorkspaceEditorKeyDown(event, context);

    expect(actions.dismissCaptureLayer).toHaveBeenCalledOnce();
    expect(calls).toEqual(['preventDefault', 'dismissCaptureLayer']);
  });

  it('updates annotation history and rerenders a selected annotation nudge', () => {
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
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        selectedAnnotationIndex: 0,
        annotationHistory: {
          ...emptyAnnotationHistory(),
          annotations: [arrow],
        },
      },
      derived: {
        annotations: [arrow],
        hasAnnotationEditingContext: true,
      },
    });

    handleCaptureWorkspaceEditorKeyDown(
      createKeyboardEvent('ArrowRight'),
      context,
    );

    expect(actions.setAnnotationHistory).toHaveBeenCalledWith({
      annotations: [movedArrow],
      undoneAnnotations: [],
      undoSnapshots: [[arrow]],
      redoSnapshots: [],
    });
    expect(actions.renderSelectionPreview).not.toHaveBeenCalled();
  });

  it('prioritizes selected annotation nudges over selection movement', () => {
    const arrow: AnnotationCommand = {
      type: 'arrow',
      start: { x: 3, y: 4 },
      end: { x: 20, y: 24 },
      color: [255, 77, 79, 255],
      stroke_width: 2,
    };
    const { actions, context } = createContext({
      state: {
        status: 'preview',
        selection,
        selectedAnnotationIndex: 0,
        annotationHistory: {
          ...emptyAnnotationHistory(),
          annotations: [arrow],
        },
      },
      derived: {
        annotations: [arrow],
        hasAnnotationEditingContext: true,
        selectionBounds: { x: 0, y: 0, width: 300, height: 200 },
      },
    });

    handleCaptureWorkspaceEditorKeyDown(
      createKeyboardEvent('ArrowRight'),
      context,
    );

    expect(actions.setAnnotationHistory).toHaveBeenCalledOnce();
    expect(actions.setSelection).not.toHaveBeenCalled();
    expect(actions.setPreviewImageBase64).not.toHaveBeenCalled();
  });

  it('copies the sampled color only while the magnifier is shown', () => {
    const cursorColor = { hex: '#112233', red: 17, green: 34, blue: 51 };
    const hidden = createContext({
      state: { status: 'preview', cursorColor },
      derived: { cursorColor, isMagnifierShown: false },
    });
    handleCaptureWorkspaceEditorKeyDown(
      createKeyboardEvent('c'),
      hidden.context,
    );
    expect(hidden.actions.copyCurrentColor).not.toHaveBeenCalled();

    const shown = createContext({
      state: { status: 'preview', cursorColor },
      derived: { cursorColor, isMagnifierShown: true },
    });
    handleCaptureWorkspaceEditorKeyDown(
      createKeyboardEvent('c'),
      shown.context,
    );
    expect(shown.actions.copyCurrentColor).toHaveBeenCalledOnce();
  });

  it('toggles sampled color format only while the magnifier is shown', () => {
    const cursorColor = { hex: '#112233', red: 17, green: 34, blue: 51 };
    const shown = createContext({
      state: { status: 'preview', cursorColor },
      derived: { cursorColor, isMagnifierShown: true },
    });

    handleCaptureWorkspaceEditorKeyDown(
      createKeyboardEvent('Shift', { shiftKey: true }),
      shown.context,
    );

    const [formatUpdater] = vi.mocked(
      shown.actions.setColorSampleFormat,
    ).mock.calls[0];
    expect(formatUpdater('hex' satisfies ColorSampleFormat)).toBe('rgb');
    expect(formatUpdater('rgb' satisfies ColorSampleFormat)).toBe('hex');
  });
});
