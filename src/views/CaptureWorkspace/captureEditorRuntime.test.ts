import { describe, expect, it } from 'vitest';

import {
  applyStyleToSelectedAnnotationHistory,
  commitCaptureEditorTextDraft,
  completeCaptureEditorGesture,
  createCapturePreviewResetState,
  deriveCaptureEditorToolbarState,
  getCaptureEditorDismissAction,
  getCaptureSelectedAnnotationBounds,
  moveSelectedAnnotationHistory,
  planCaptureAnnotationColorSelection,
  planCaptureAnnotationFillToggle,
  planCaptureAnnotationSizeAdjustment,
  planCaptureSelectedAnnotationKeyboardNudge,
  planCaptureAnnotationToolActivation,
  planCaptureAnnotationGestureMove,
  planCaptureExistingAnnotationPointerDown,
  planCaptureAnnotationMove,
  planCaptureAnnotationMoveCommit,
  planCaptureAnnotationToolStart,
  planCaptureManualSelectionTransition,
} from './captureEditorRuntime';
import { emptyAnnotationHistory, type AnnotationHistory } from './annotationHistory';
import type { AnnotationStyle } from './annotationStyle';
import type { AnnotationCommand } from './types';

const baseStyle: AnnotationStyle = {
  color: [255, 77, 79, 255],
  strokeWidth: 2,
  filled: false,
};

describe('captureEditorRuntime', () => {
  it('derives toolbar state from annotation-specific fields while preserving unrelated controls', () => {
    expect(
      deriveCaptureEditorToolbarState(
        {
          annotationStyle: {
            color: [24, 144, 255, 255],
            strokeWidth: 4,
            filled: true,
          },
          textFontSize: 26,
        },
        {
          type: 'text',
          position: { x: 10, y: 20 },
          text: 'Snap',
          color: [40, 167, 69, 255],
          font_size: 32,
        },
      ),
    ).toEqual({
      annotationStyle: {
        color: [40, 167, 69, 255],
        strokeWidth: 4,
        filled: true,
      },
      textFontSize: 32,
    });

    expect(
      deriveCaptureEditorToolbarState(
        {
          annotationStyle: {
            color: [24, 144, 255, 255],
            strokeWidth: 4,
            filled: true,
          },
          textFontSize: 26,
        },
        {
          type: 'mosaic',
          points: [{ x: 1, y: 2 }, { x: 20, y: 10 }],
          stroke_width: 20,
          block_size: 7,
        },
      ),
    ).toEqual({
      annotationStyle: {
        color: [24, 144, 255, 255],
        strokeWidth: 4,
        filled: true,
      },
      textFontSize: 26,
    });
  });

  it('applies a selected annotation style through history only when an editable selection exists', () => {
    const annotations: AnnotationCommand[] = [
      {
        type: 'rectangle',
        rect: { x: 1, y: 2, width: 10, height: 8 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
    ];
    const history = {
      ...emptyAnnotationHistory(),
      annotations,
    };

    expect(
      applyStyleToSelectedAnnotationHistory({
        annotationHistory: history,
        annotations: history.annotations,
        selectedAnnotationIndex: 0,
        textDraftActive: false,
        nextStyle: {
          color: [40, 167, 69, 255],
          strokeWidth: 5,
          filled: true,
        },
        nextTextFontSize: 32,
      }),
    ).toEqual({
      ...history,
      annotations: [
        {
          type: 'rectangle',
          rect: { x: 1, y: 2, width: 10, height: 8 },
          color: [40, 167, 69, 255],
          stroke_width: 5,
          filled: true,
        },
      ],
      undoneAnnotations: [],
      undoSnapshots: [history.annotations],
      redoSnapshots: [],
    });

    expect(
      applyStyleToSelectedAnnotationHistory({
        annotationHistory: history,
        annotations: history.annotations,
        selectedAnnotationIndex: 0,
        textDraftActive: true,
        nextStyle: baseStyle,
        nextTextFontSize: 24,
      }),
    ).toBe(history);
  });

  it('moves the selected annotation through history updates and ignores missing selections', () => {
    const movedAnnotations: AnnotationCommand[] = [
      {
        type: 'line',
        start: { x: 1, y: 2 },
        end: { x: 5, y: 6 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
      },
    ];
    const history = {
      ...emptyAnnotationHistory(),
      annotations: movedAnnotations,
    };

    expect(
      moveSelectedAnnotationHistory({
        annotationHistory: history,
        annotations: history.annotations,
        selectedAnnotationIndex: 0,
        delta: { x: 3, y: -1 },
      }),
    ).toEqual({
      ...history,
      annotations: [
        {
          type: 'line',
          start: { x: 4, y: 1 },
          end: { x: 8, y: 5 },
          color: [255, 77, 79, 255],
          stroke_width: 2,
        },
      ],
      undoneAnnotations: [],
      undoSnapshots: [history.annotations],
      redoSnapshots: [],
    });

    expect(
      moveSelectedAnnotationHistory({
        annotationHistory: history,
        annotations: history.annotations,
        selectedAnnotationIndex: null,
        delta: { x: 1, y: 1 },
      }),
    ).toBe(history);
  });

  it('derives selected annotation bounds only when selection editing is idle', () => {
    const annotations: AnnotationCommand[] = [
      {
        type: 'rectangle',
        rect: { x: 4, y: 6, width: 20, height: 10 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
    ];

    expect(
      getCaptureSelectedAnnotationBounds({
        annotations,
        selectedAnnotationIndex: 0,
        annotationMoveGesture: null,
      }),
    ).toEqual({ x: 4, y: 6, width: 20, height: 10 });

    expect(
      getCaptureSelectedAnnotationBounds({
        annotations,
        selectedAnnotationIndex: 0,
        annotationMoveGesture: {
          annotationIndex: 0,
          startPoint: { x: 1, y: 1 },
          startAnnotation: annotations[0],
        },
      }),
    ).toBeNull();

    expect(
      getCaptureSelectedAnnotationBounds({
        annotations,
        selectedAnnotationIndex: 1,
        annotationMoveGesture: null,
      }),
    ).toBeNull();
  });

  it('plans selected annotation keyboard nudges through history', () => {
    const annotations: AnnotationCommand[] = [
      {
        type: 'rectangle',
        rect: { x: 1, y: 2, width: 10, height: 8 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
    ];
    const history: AnnotationHistory = {
      ...emptyAnnotationHistory(),
      annotations,
    };

    expect(
      planCaptureSelectedAnnotationKeyboardNudge({
        annotationHistory: history,
        annotations,
        selectedAnnotationIndex: 0,
        key: 'ArrowRight',
        fast: true,
        keyboardNudgeStep: 1,
        keyboardFastNudgeStep: 10,
      }),
    ).toEqual({
      annotationHistory: {
        ...history,
        annotations: [
          {
            type: 'rectangle',
            rect: { x: 11, y: 2, width: 10, height: 8 },
            color: [255, 77, 79, 255],
            stroke_width: 2,
            filled: false,
          },
        ],
        undoneAnnotations: [],
        undoSnapshots: [history.annotations],
        redoSnapshots: [],
      },
      previewAnnotations: [
        {
          type: 'rectangle',
          rect: { x: 11, y: 2, width: 10, height: 8 },
          color: [255, 77, 79, 255],
          stroke_width: 2,
          filled: false,
        },
      ],
    });

    expect(
      planCaptureSelectedAnnotationKeyboardNudge({
        annotationHistory: history,
        annotations,
        selectedAnnotationIndex: null,
        key: 'ArrowRight',
        fast: false,
        keyboardNudgeStep: 1,
        keyboardFastNudgeStep: 10,
      }),
    ).toEqual({
      annotationHistory: history,
      previewAnnotations: null,
    });
  });

  it('prioritizes dismiss actions from text draft to session cancel', () => {
    expect(
      getCaptureEditorDismissAction({
        hasTextDraft: true,
        hasAnnotationMoveGesture: true,
        hasDraftSelectionMoveGesture: true,
        hasSelectedAnnotation: true,
        hasActiveAnnotationTool: true,
        hasAnnotationGesture: true,
      }),
    ).toBe('clear-text-draft');

    expect(
      getCaptureEditorDismissAction({
        hasTextDraft: false,
        hasAnnotationMoveGesture: true,
        hasDraftSelectionMoveGesture: false,
        hasSelectedAnnotation: false,
        hasActiveAnnotationTool: false,
        hasAnnotationGesture: false,
      }),
    ).toBe('revert-annotation-move');

    expect(
      getCaptureEditorDismissAction({
        hasTextDraft: false,
        hasAnnotationMoveGesture: false,
        hasDraftSelectionMoveGesture: false,
        hasSelectedAnnotation: false,
        hasActiveAnnotationTool: false,
        hasAnnotationGesture: false,
      }),
    ).toBe('cancel-session');
  });

  it('plans annotation tool activation for toggle and direct selection paths', () => {
    expect(
      planCaptureAnnotationToolActivation({
        currentTool: 'line',
        nextTool: 'line',
        selectedAnnotationIndex: 3,
        clearSelectedAnnotation: false,
        toggle: true,
      }),
    ).toEqual({
      activeAnnotationTool: null,
      selectedAnnotationIndex: 3,
      annotationGesture: null,
      annotationMoveGesture: null,
      draftAnnotation: null,
    });

    expect(
      planCaptureAnnotationToolActivation({
        currentTool: 'line',
        nextTool: 'arrow',
        selectedAnnotationIndex: 3,
        clearSelectedAnnotation: true,
        toggle: false,
      }),
    ).toEqual({
      activeAnnotationTool: 'arrow',
      selectedAnnotationIndex: null,
      annotationGesture: null,
      annotationMoveGesture: null,
      draftAnnotation: null,
    });
  });

  it('plans annotation style changes for size, color, and fill shortcuts', () => {
    expect(
      planCaptureAnnotationSizeAdjustment({
        annotationStyle: baseStyle,
        textFontSize: 24,
        direction: 'increase',
        isTextSizingActive: false,
      }),
    ).toEqual({
      annotationStyle: {
        ...baseStyle,
        strokeWidth: 3,
      },
      textFontSize: 24,
    });

    expect(
      planCaptureAnnotationSizeAdjustment({
        annotationStyle: baseStyle,
        textFontSize: 24,
        direction: 'decrease',
        isTextSizingActive: true,
      }),
    ).toEqual({
      annotationStyle: baseStyle,
      textFontSize: 23,
    });

    expect(
      planCaptureAnnotationColorSelection({
        annotationStyle: baseStyle,
        textFontSize: 24,
        color: [40, 167, 69, 255],
      }),
    ).toEqual({
      annotationStyle: {
        ...baseStyle,
        color: [40, 167, 69, 255],
      },
      textFontSize: 24,
    });

    expect(
      planCaptureAnnotationFillToggle({
        annotationStyle: baseStyle,
        textFontSize: 24,
      }),
    ).toEqual({
      annotationStyle: {
        ...baseStyle,
        filled: true,
      },
      textFontSize: 24,
    });
  });

  it('commits a text draft into annotation history and clears editor draft state', () => {
    const history: AnnotationHistory = {
      ...emptyAnnotationHistory(),
      annotations: [
        {
          type: 'rectangle',
          rect: { x: 1, y: 2, width: 10, height: 8 },
          color: [255, 77, 79, 255],
          stroke_width: 2,
          filled: false,
        },
      ],
    };

    expect(
      commitCaptureEditorTextDraft({
        annotationHistory: history,
        selectedAnnotationIndex: 0,
        textDraft: {
          position: { x: 20, y: 24 },
          text: 'SnapLingo',
          fontSize: 28,
        },
        annotationStyle: baseStyle,
        textDraftAnnotationIndex: null,
      }),
    ).toEqual({
      annotationHistory: {
        ...history,
        annotations: [
          history.annotations[0],
          {
            type: 'text',
            position: { x: 20, y: 24 },
            text: 'SnapLingo',
            color: [255, 77, 79, 255],
            font_size: 28,
          },
        ],
        undoneAnnotations: [],
        undoSnapshots: [history.annotations],
        redoSnapshots: [],
      },
      selectedAnnotationIndex: null,
      textDraft: null,
      textDraftAnnotationIndex: null,
    });
  });

  it('completes an annotation gesture into history and preserves selection when the gesture is too small to commit', () => {
    const history = emptyAnnotationHistory();

    expect(
      completeCaptureEditorGesture({
        annotationHistory: history,
        selectedAnnotationIndex: 2,
        annotationGesture: {
          tool: 'line',
          startPoint: { x: 10, y: 12 },
        },
        localPoint: { x: 18, y: 20 },
        annotationStyle: baseStyle,
        constrainGesture: false,
      }),
    ).toEqual({
      annotationHistory: {
        ...history,
        annotations: [
          {
            type: 'line',
            start: { x: 10, y: 12 },
            end: { x: 18, y: 20 },
            color: [255, 77, 79, 255],
            stroke_width: 2,
          },
        ],
        undoneAnnotations: [],
        undoSnapshots: [[]],
        redoSnapshots: [],
      },
      selectedAnnotationIndex: null,
      annotationGesture: null,
      draftAnnotation: null,
    });

    expect(
      completeCaptureEditorGesture({
        annotationHistory: history,
        selectedAnnotationIndex: null,
        annotationGesture: {
          tool: 'rectangle',
          startPoint: { x: 10, y: 12 },
        },
        localPoint: { x: 30, y: 28 },
        annotationStyle: baseStyle,
        constrainGesture: false,
      })?.selectedAnnotationIndex,
    ).toBe(0);

    expect(
      completeCaptureEditorGesture({
        annotationHistory: history,
        selectedAnnotationIndex: 2,
        annotationGesture: {
          tool: 'rectangle',
          startPoint: { x: 10, y: 12 },
        },
        localPoint: { x: 12, y: 14 },
        annotationStyle: baseStyle,
        constrainGesture: false,
      }),
    ).toEqual({
      annotationHistory: history,
      selectedAnnotationIndex: 2,
      annotationGesture: null,
      draftAnnotation: null,
    });
  });

  it('builds a preview reset state that clears selection and editor artifacts', () => {
    expect(createCapturePreviewResetState()).toEqual({
      startPoint: null,
      cursorPoint: null,
      selection: null,
      hoverSelection: null,
      editGesture: null,
      previewImageBase64: null,
      renderingOutput: false,
      activeAnnotationTool: null,
      annotationGesture: null,
      draftAnnotation: null,
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      draftSelectionMoveGesture: null,
      textDraft: null,
      textDraftAnnotationIndex: null,
      annotationHistory: emptyAnnotationHistory(),
      isMagnifierRequested: false,
      status: 'selecting',
    });
  });

  it('plans manual selection transitions for preview and effects flows', () => {
    const rect = { x: 10, y: 20, width: 200, height: 80 };

    expect(
      planCaptureManualSelectionTransition({
        rect,
        completion: { type: 'preview' },
      }),
    ).toEqual({
      type: 'preview',
      clearOverlay: true,
      nextState: {
        startPoint: null,
        selection: rect,
        hoverSelection: null,
        editGesture: null,
        activeAnnotationTool: null,
        annotationGesture: null,
        draftAnnotation: null,
        selectedAnnotationIndex: null,
        annotationMoveGesture: null,
        draftSelectionMoveGesture: null,
        textDraft: null,
        textDraftAnnotationIndex: null,
        annotationHistory: emptyAnnotationHistory(),
        isMagnifierRequested: false,
        isAnnotationToolbarVisible: true,
        status: 'preview',
      },
      previewRender: {
        rect,
        annotations: [],
      },
    });

    expect(
      planCaptureManualSelectionTransition({
        rect,
        completion: {
          type: 'effects',
          effects: [{ type: 'finish-session' }],
        },
      }),
    ).toEqual({
      type: 'effects',
      clearOverlay: true,
      nextState: {
        startPoint: null,
        selection: rect,
        hoverSelection: null,
        editGesture: null,
        activeAnnotationTool: null,
        annotationGesture: null,
        draftAnnotation: null,
        selectedAnnotationIndex: null,
        annotationMoveGesture: null,
        draftSelectionMoveGesture: null,
        textDraft: null,
        textDraftAnnotationIndex: null,
        annotationHistory: emptyAnnotationHistory(),
        isMagnifierRequested: false,
        isAnnotationToolbarVisible: false,
        status: 'selecting',
        renderingOutput: true,
        error: null,
      },
      effects: [{ type: 'finish-session' }],
    });
  });

  it('plans annotation gesture pointer moves while preserving constrained stroke semantics', () => {
    expect(
      planCaptureAnnotationGestureMove({
        gesture: {
          tool: 'pen',
          startPoint: { x: 0, y: 0 },
          points: [{ x: 0, y: 0 }],
        },
        localPoint: { x: 8, y: 5 },
        annotationStyle: baseStyle,
        constrainGesture: false,
      }),
    ).toEqual({
      annotationGesture: {
        tool: 'pen',
        startPoint: { x: 0, y: 0 },
        points: [{ x: 0, y: 0 }, { x: 8, y: 5 }],
      },
      draftAnnotation: {
        type: 'freehand',
        points: [{ x: 0, y: 0 }, { x: 8, y: 5 }],
        color: [255, 77, 79, 255],
        stroke_width: 2,
      },
    });

    expect(
      planCaptureAnnotationGestureMove({
        gesture: {
          tool: 'pen',
          startPoint: { x: 0, y: 0 },
          points: [{ x: 0, y: 0 }, { x: 8, y: 5 }],
        },
        localPoint: { x: 9, y: 4 },
        annotationStyle: baseStyle,
        constrainGesture: true,
      }),
    ).toEqual({
      annotationGesture: {
        tool: 'pen',
        startPoint: { x: 0, y: 0 },
        points: [{ x: 0, y: 0 }, { x: 8, y: 5 }],
      },
      draftAnnotation: {
        type: 'freehand',
        points: [{ x: 0, y: 0 }, { x: 9, y: 0 }],
        color: [255, 77, 79, 255],
        stroke_width: 2,
      },
    });
  });

  it('plans annotation tool starts with draft gestures and draft annotations', () => {
    expect(
      planCaptureAnnotationToolStart({
        tool: 'rectangle',
        localPoint: { x: 10, y: 20 },
        annotationStyle: baseStyle,
      }),
    ).toEqual({
      selectedAnnotationIndex: null,
      annotationGesture: {
        tool: 'rectangle',
        startPoint: { x: 10, y: 20 },
      },
      draftAnnotation: {
        type: 'rectangle',
        rect: { x: 10, y: 20, width: 0, height: 0 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
    });

    expect(
      planCaptureAnnotationToolStart({
        tool: 'pen',
        localPoint: { x: 4, y: 8 },
        annotationStyle: baseStyle,
      }),
    ).toEqual({
      selectedAnnotationIndex: null,
      annotationGesture: {
        tool: 'pen',
        startPoint: { x: 4, y: 8 },
        points: [{ x: 4, y: 8 }],
      },
      draftAnnotation: {
        type: 'freehand',
        points: [{ x: 4, y: 8 }],
        color: [255, 77, 79, 255],
        stroke_width: 2,
      },
    });
  });

  it('plans existing annotation pointer moves with optional axis constraint', () => {
    expect(
      planCaptureAnnotationMove({
        startAnnotation: {
          type: 'rectangle',
          rect: { x: 2, y: 4, width: 30, height: 18 },
          color: [255, 77, 79, 255],
          stroke_width: 2,
          filled: false,
        },
        startPoint: { x: 10, y: 10 },
        localPoint: { x: 14, y: 5 },
        constrainMove: false,
      }),
    ).toEqual({
      draftAnnotation: {
        type: 'rectangle',
        rect: { x: 6, y: -1, width: 30, height: 18 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
    });

    expect(
      planCaptureAnnotationMove({
        startAnnotation: {
          type: 'line',
          start: { x: 2, y: 4 },
          end: { x: 20, y: 18 },
          color: [255, 77, 79, 255],
          stroke_width: 2,
        },
        startPoint: { x: 10, y: 10 },
        localPoint: { x: 4, y: 28 },
        constrainMove: true,
      }),
    ).toEqual({
      draftAnnotation: {
        type: 'line',
        start: { x: 2, y: 22 },
        end: { x: 20, y: 36 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
      },
    });
  });

  it('plans existing annotation move commits and preserves no-op commits', () => {
    const annotations: AnnotationCommand[] = [
      {
        type: 'rectangle',
        rect: { x: 2, y: 4, width: 30, height: 18 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
    ];
    const history: AnnotationHistory = {
      ...emptyAnnotationHistory(),
      annotations,
    };

    expect(
      planCaptureAnnotationMoveCommit({
        annotationHistory: history,
        annotationIndex: 0,
        startAnnotation: annotations[0],
        startPoint: { x: 10, y: 10 },
        localPoint: { x: 14, y: 5 },
        constrainMove: false,
      }),
    ).toEqual({
      annotationMoveGesture: null,
      draftAnnotation: null,
      annotationHistory: {
        ...history,
        annotations: [
          {
            type: 'rectangle',
            rect: { x: 6, y: -1, width: 30, height: 18 },
            color: [255, 77, 79, 255],
            stroke_width: 2,
            filled: false,
          },
        ],
        undoneAnnotations: [],
        undoSnapshots: [history.annotations],
        redoSnapshots: [],
      },
      selectedAnnotationIndex: 0,
    });

    expect(
      planCaptureAnnotationMoveCommit({
        annotationHistory: history,
        annotationIndex: 0,
        startAnnotation: annotations[0],
        startPoint: { x: 10, y: 10 },
        localPoint: { x: 10, y: 10 },
        constrainMove: false,
      }),
    ).toEqual({
      annotationMoveGesture: null,
      draftAnnotation: null,
      annotationHistory: history,
      selectedAnnotationIndex: undefined,
    });
  });

  it('plans double-click editing of an existing text annotation', () => {
    const annotations: AnnotationCommand[] = [
      {
        type: 'rectangle',
        rect: { x: 2, y: 4, width: 30, height: 18 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
      {
        type: 'text',
        position: { x: 12, y: 16 },
        text: 'Snap',
        color: [40, 167, 69, 255],
        font_size: 32,
      },
    ];

    expect(
      planCaptureExistingAnnotationPointerDown({
        annotations,
        localPoint: { x: 14, y: 18 },
        pointerDetail: 2,
        toolbarState: {
          annotationStyle: baseStyle,
          textFontSize: 20,
        },
      }),
    ).toEqual({
      type: 'edit-text-annotation',
      selectedAnnotationIndex: 1,
      annotationMoveGesture: null,
      draftAnnotation: null,
      textDraft: {
        position: { x: 12, y: 16 },
        text: 'Snap',
        fontSize: 32,
      },
      textDraftAnnotationIndex: 1,
      toolbarState: {
        annotationStyle: {
          ...baseStyle,
          color: [40, 167, 69, 255],
        },
        textFontSize: 32,
      },
    });
  });

  it('plans moving an existing annotation when a non-text edit click hits it', () => {
    const annotations: AnnotationCommand[] = [
      {
        type: 'rectangle',
        rect: { x: 2, y: 4, width: 30, height: 18 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
    ];

    expect(
      planCaptureExistingAnnotationPointerDown({
        annotations,
        localPoint: { x: 14, y: 18 },
        pointerDetail: 1,
        toolbarState: {
          annotationStyle: baseStyle,
          textFontSize: 20,
        },
      }),
    ).toEqual({
      type: 'move-annotation',
      selectedAnnotationIndex: 0,
      annotationMoveGesture: {
        annotationIndex: 0,
        startPoint: { x: 14, y: 18 },
        startAnnotation: annotations[0],
      },
      toolbarState: {
        annotationStyle: baseStyle,
        textFontSize: 20,
      },
    });
  });
});
