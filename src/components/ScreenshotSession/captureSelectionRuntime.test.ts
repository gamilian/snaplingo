import { describe, expect, it } from 'vitest';

import {
  planCaptureDraftSelectionMoveShortcutStart,
  planCaptureDraftSelectionKeyboardNudge,
  applyCaptureSelectionEditGesture,
  planCaptureHoverSelectionCycle,
  planCaptureSelectionArrowPreview,
  planCaptureSelectionCursorKeyboardNudge,
  planCaptureDraftSelectionCommit,
  planCaptureDraftSelectionMove,
  planCaptureDraftSelectionPointerMove,
  planCaptureDraftSelectionStart,
  planCaptureSelectionEditKeyboardNudge,
  planCapturePreviewSelectionMoveStart,
  planCaptureSelectionEditCommit,
  planCaptureSelectionEditMove,
  planCaptureSelectionResizeStart,
} from './captureSelectionRuntime';
import { emptyAnnotationHistory } from './annotationHistory';
import type { AnnotationCommand } from './types';

describe('captureSelectionRuntime', () => {
  it('plans state for starting a fresh draft selection from a snapped anchor point', () => {
    expect(
      planCaptureDraftSelectionStart({
        cursorPoint: { x: 42, y: 64 },
        anchorPoint: { x: 40, y: 60 },
      }),
    ).toEqual({
      cursorPoint: { x: 42, y: 64 },
      draftSelection: { x: 40, y: 60, width: 0, height: 0 },
      nextState: {
        cursorPoint: { x: 42, y: 64 },
        startPoint: { x: 40, y: 60 },
        selection: null,
        hoverSelection: null,
        previewImageBase64: null,
        renderingOutput: false,
        status: 'selecting',
        activeAnnotationTool: null,
        annotationGesture: null,
        draftAnnotation: null,
        selectedAnnotationIndex: null,
        annotationMoveGesture: null,
        draftSelectionMoveGesture: null,
        textDraft: null,
        textDraftAnnotationIndex: null,
        annotationHistory: emptyAnnotationHistory(),
      },
    });
  });

  it('plans draft selection pointer moves while clamping movement to the capture bounds', () => {
    expect(
      planCaptureDraftSelectionMove({
        gesture: {
          startPoint: { x: 100, y: 90 },
          startSelection: { x: 80, y: 70, width: 50, height: 40 },
          startAnchorPoint: { x: 80, y: 70 },
        },
        point: { x: 140, y: 115 },
        selectionBounds: { x: 0, y: 0, width: 160, height: 120 },
      }),
    ).toEqual({
      draftSelection: { x: 110, y: 80, width: 50, height: 40 },
      anchorPoint: { x: 110, y: 80 },
      previewImageBase64: null,
      renderingOutput: false,
    });
  });

  it('plans keyboard nudges for a draft selection', () => {
    expect(
      planCaptureDraftSelectionKeyboardNudge({
        anchorPoint: { x: 10, y: 10 },
        cursorPoint: { x: 18, y: 16 },
        delta: { x: 4, y: -3 },
        selectionBounds: { x: 0, y: 0, width: 40, height: 30 },
      }),
    ).toEqual({
      keyboardDraftCursorPoint: { x: 22, y: 13 },
      cursorPoint: { x: 22, y: 13 },
      selection: { x: 10, y: 10, width: 12, height: 3 },
      previewImageBase64: null,
      renderingOutput: false,
    });
  });

  it('plans fresh draft selection pointer moves with snapping and square constraint', () => {
    const baseOptions = {
      anchorPoint: { x: 10, y: 10 },
      point: { x: 58, y: 40 },
      snapTargetRects: [{ x: 60, y: 40, width: 30, height: 20 }],
      edgeSnapThreshold: 6,
    };

    expect(
      planCaptureDraftSelectionPointerMove({
        ...baseOptions,
        constrainSelection: false,
      }),
    ).toEqual({
      keyboardDraftCursorPoint: null,
      draftSelection: { x: 10, y: 10, width: 50, height: 30 },
    });

    expect(
      planCaptureDraftSelectionPointerMove({
        ...baseOptions,
        constrainSelection: true,
      }),
    ).toEqual({
      keyboardDraftCursorPoint: null,
      draftSelection: { x: 10, y: 10, width: 30, height: 30 },
    });
  });

  it('plans draft selection release completion for drawn selections and hover candidates', () => {
    const baseOptions = {
      anchorPoint: { x: 10, y: 10 },
      snapTargetRects: [{ x: 60, y: 40, width: 30, height: 20 }],
      edgeSnapThreshold: 6,
      constrainSelection: false,
      captureCandidates: [
        {
          id: 'window:1',
          kind: 'window' as const,
          rect: { x: 0, y: 0, width: 50, height: 50 },
          priority: 10,
        },
      ],
      activeHoverSelection: { x: 0, y: 0, width: 50, height: 50 },
      minSelectionSize: 10,
    };

    expect(
      planCaptureDraftSelectionCommit({
        ...baseOptions,
        releasePoint: { x: 58, y: 40 },
      }),
    ).toEqual({
      type: 'complete-selection',
      startPoint: null,
      draftSelection: null,
      overlayHoverSelection: { x: 0, y: 0, width: 50, height: 50 },
      selection: { x: 10, y: 10, width: 50, height: 30 },
    });

    expect(
      planCaptureDraftSelectionCommit({
        ...baseOptions,
        releasePoint: { x: 12, y: 12 },
      }),
    ).toEqual({
      type: 'complete-selection',
      startPoint: null,
      draftSelection: null,
      overlayHoverSelection: { x: 0, y: 0, width: 50, height: 50 },
      selection: { x: 0, y: 0, width: 50, height: 50 },
    });
  });

  it('plans draft selection release cleanup when neither selection nor hover candidate is usable', () => {
    expect(
      planCaptureDraftSelectionCommit({
        anchorPoint: { x: 10, y: 10 },
        releasePoint: { x: 12, y: 12 },
        snapTargetRects: [],
        edgeSnapThreshold: 6,
        constrainSelection: false,
        captureCandidates: [],
        activeHoverSelection: null,
        minSelectionSize: 10,
      }),
    ).toEqual({
      type: 'clear-selection',
      startPoint: null,
      draftSelection: null,
      overlayHoverSelection: null,
      selection: null,
    });
  });

  it('plans hover candidate cycling and starting draft-selection move gestures', () => {
    const captureCandidates = [
      {
        id: 'window:1',
        kind: 'window' as const,
        rect: { x: 0, y: 0, width: 50, height: 50 },
        priority: 10,
      },
      {
        id: 'window:2',
        kind: 'window' as const,
        rect: { x: 10, y: 10, width: 80, height: 70 },
        priority: 20,
      },
    ];

    expect(
      planCaptureHoverSelectionCycle({
        captureCandidates,
        cursorPoint: { x: 20, y: 20 },
        hoverSelection: captureCandidates[0].rect,
        direction: 1,
      }),
    ).toEqual({
      hoverSelection: captureCandidates[1].rect,
    });

    expect(
      planCaptureDraftSelectionMoveShortcutStart({
        cursorPoint: { x: 30, y: 40 },
        selection: { x: 20, y: 30, width: 40, height: 20 },
        anchorPoint: { x: 20, y: 30 },
      }),
    ).toEqual({
      draftSelectionMoveGesture: {
        startPoint: { x: 30, y: 40 },
        startSelection: { x: 20, y: 30, width: 40, height: 20 },
        startAnchorPoint: { x: 20, y: 30 },
      },
    });
  });

  it('applies selection edit gestures with snapping and aspect-lock rules', () => {
    const selectionBounds = { x: 0, y: 0, width: 120, height: 90 };
    const snapTargetRects = [{ x: 60, y: 20, width: 30, height: 30 }];

    expect(
      applyCaptureSelectionEditGesture({
        gesture: {
          type: 'move',
          startPoint: { x: 0, y: 0 },
          startSelection: { x: 10, y: 10, width: 20, height: 20 },
        },
        point: { x: 28, y: 4 },
        selectionBounds,
        snapTargetRects,
        edgeSnapThreshold: 6,
        minSelectionSize: 10,
      }),
    ).toEqual({ x: 40, y: 20, width: 20, height: 20 });

    expect(
      applyCaptureSelectionEditGesture({
        gesture: {
          type: 'resize',
          handle: 'se',
          startPoint: { x: 40, y: 30 },
          startSelection: { x: 10, y: 10, width: 30, height: 20 },
        },
        point: { x: 57, y: 36 },
        selectionBounds,
        snapTargetRects,
        edgeSnapThreshold: 6,
        minSelectionSize: 10,
      }),
    ).toEqual({ x: 10, y: 10, width: 50, height: 26 });

    expect(
      applyCaptureSelectionEditGesture({
        gesture: {
          type: 'resize',
          handle: 'se',
          startPoint: { x: 50, y: 30 },
          startSelection: { x: 10, y: 10, width: 40, height: 20 },
        },
        point: { x: 70, y: 35 },
        selectionBounds,
        snapTargetRects,
        edgeSnapThreshold: 6,
        minSelectionSize: 10,
        preserveAspect: true,
      }),
    ).toEqual({ x: 10, y: 10, width: 60, height: 30 });
  });

  it('plans selection edit pointer moves and commits through one edit gesture seam', () => {
    const gesture = {
      type: 'move' as const,
      startPoint: { x: 10, y: 10 },
      startSelection: { x: 20, y: 30, width: 40, height: 20 },
    };
    const editOptions = {
      gesture,
      point: { x: 15, y: 25 },
      selectionBounds: { x: 0, y: 0, width: 120, height: 90 },
      snapTargetRects: [],
      edgeSnapThreshold: 6,
      minSelectionSize: 10,
      preserveAspect: false,
    };
    const annotations: AnnotationCommand[] = [
      {
        type: 'line',
        start: { x: 1, y: 2 },
        end: { x: 8, y: 9 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
      },
    ];

    expect(
      planCaptureSelectionEditMove(editOptions),
    ).toEqual({
      keyboardEditCursorPoint: null,
      selection: { x: 25, y: 45, width: 40, height: 20 },
      previewImageBase64: null,
      renderingOutput: false,
    });

    expect(
      planCaptureSelectionEditCommit({
        ...editOptions,
        annotations,
      }),
    ).toEqual({
      editGesture: null,
      selection: { x: 25, y: 45, width: 40, height: 20 },
      status: 'preview',
      previewRender: {
        rect: { x: 25, y: 45, width: 40, height: 20 },
        annotations,
      },
    });
  });

  it('plans keyboard nudges for selection edit gestures in move and resize modes', () => {
    const selectionBounds = { x: 0, y: 0, width: 120, height: 90 };

    expect(
      planCaptureSelectionEditKeyboardNudge({
        gesture: {
          type: 'move',
          startPoint: { x: 10, y: 10 },
          startSelection: { x: 20, y: 30, width: 40, height: 20 },
        },
        selection: { x: 20, y: 30, width: 40, height: 20 },
        cursorPoint: { x: 10, y: 10 },
        delta: { x: 5, y: -3 },
        selectionBounds,
        minSelectionSize: 10,
        preserveAspect: false,
      }),
    ).toEqual({
      keyboardEditCursorPoint: { x: 15, y: 7 },
      cursorPoint: { x: 15, y: 7 },
      selection: { x: 25, y: 27, width: 40, height: 20 },
      editGesture: {
        type: 'move',
        startPoint: { x: 15, y: 7 },
        startSelection: { x: 25, y: 27, width: 40, height: 20 },
      },
      previewImageBase64: null,
      renderingOutput: false,
    });

    expect(
      planCaptureSelectionEditKeyboardNudge({
        gesture: {
          type: 'resize',
          handle: 'se',
          startPoint: { x: 60, y: 50 },
          startSelection: { x: 20, y: 30, width: 40, height: 20 },
        },
        selection: { x: 20, y: 30, width: 40, height: 20 },
        cursorPoint: { x: 60, y: 50 },
        delta: { x: 4, y: 6 },
        selectionBounds,
        minSelectionSize: 10,
        preserveAspect: false,
      }),
    ).toEqual({
      keyboardEditCursorPoint: { x: 64, y: 56 },
      cursorPoint: { x: 64, y: 56 },
      selection: { x: 20, y: 30, width: 44, height: 26 },
      editGesture: {
        type: 'resize',
        handle: 'se',
        startPoint: { x: 64, y: 56 },
        startSelection: { x: 20, y: 30, width: 44, height: 26 },
      },
      previewImageBase64: null,
      renderingOutput: false,
    });
  });

  it('plans keyboard nudges for the floating cursor and preview selection arrow adjustments', () => {
    expect(
      planCaptureSelectionCursorKeyboardNudge({
        cursorPoint: { x: 10, y: 10 },
        delta: { x: -3, y: 5 },
        selectionBounds: { x: 0, y: 0, width: 40, height: 30 },
      }),
    ).toEqual({
      cursorPoint: { x: 7, y: 15 },
    });

    expect(
      planCaptureSelectionArrowPreview({
        selection: { x: 20, y: 30, width: 40, height: 20 },
        selectionBounds: { x: 0, y: 0, width: 120, height: 90 },
        selectionArrowAction: {
          mode: 'expand',
          direction: 'ArrowRight',
        },
        minSelectionSize: 10,
        keyboardNudgeStep: 1,
      }),
    ).toEqual({
      selection: { x: 20, y: 30, width: 41, height: 20 },
      previewImageBase64: null,
      previewRender: {
        rect: { x: 20, y: 30, width: 41, height: 20 },
      },
    });
  });

  it('plans preview selection move starts without stealing copy double-clicks', () => {
    const selection = { x: 20, y: 30, width: 40, height: 20 };

    expect(
      planCapturePreviewSelectionMoveStart({
        point: { x: 10, y: 12 },
        selection,
        hasTextDraft: false,
        isCopyDoubleClick: true,
      }),
    ).toEqual({
      type: 'copy-selection',
    });

    expect(
      planCapturePreviewSelectionMoveStart({
        point: { x: 10, y: 12 },
        selection,
        hasTextDraft: true,
        isCopyDoubleClick: true,
      }),
    ).toEqual({
      type: 'move-selection',
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      editGesture: {
        type: 'move',
        startPoint: { x: 10, y: 12 },
        startSelection: selection,
      },
      previewImageBase64: null,
    });
  });

  it('plans preview selection resize starts through the edit gesture seam', () => {
    const selection = { x: 20, y: 30, width: 40, height: 20 };

    expect(
      planCaptureSelectionResizeStart({
        point: { x: 60, y: 50 },
        selection,
        handle: 'se',
      }),
    ).toEqual({
      cursorPoint: { x: 60, y: 50 },
      editGesture: {
        type: 'resize',
        handle: 'se',
        startPoint: { x: 60, y: 50 },
        startSelection: selection,
      },
      previewImageBase64: null,
    });
  });
});
