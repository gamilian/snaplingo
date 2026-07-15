import { describe, expect, it } from 'vitest';

import { emptyAnnotationHistory } from './annotationHistory';
import {
  DEFAULT_ANNOTATION_STYLE,
  DEFAULT_TEXT_FONT_SIZE,
} from './annotationStyle';
import { createCapturePreviewResetState } from './captureEditorRuntime';
import {
  createInitialCaptureWorkspaceState,
  loadedCaptureHostSessionPatch,
  previewResetPatch,
  resetCaptureInteractionStatePatch,
} from './captureWorkspaceState';
import type { CaptureSessionView } from './types';

function createCaptureSessionView(
  overrides: Partial<CaptureSessionView> = {},
): CaptureSessionView {
  return {
    id: 'session-1',
    monitors: [
      {
        id: 'monitor-1',
        logical_bounds: { x: 0, y: 0, width: 500, height: 300 },
        physical_bounds: { x: 0, y: 0, width: 1000, height: 600 },
        scale_factor: 2,
        image_base64: '',
      },
    ],
    candidates: [],
    captured_cursor: null,
    ...overrides,
  };
}

describe('captureWorkspaceState', () => {
  it('creates the initial workspace state from ScreenshotSession defaults', () => {
    expect(createInitialCaptureWorkspaceState()).toEqual({
      status: 'idle',
      mode: 'screenshot',
      session: null,
      startPoint: null,
      cursorPoint: null,
      selection: null,
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
      annotationStyle: DEFAULT_ANNOTATION_STYLE,
      textFontSize: DEFAULT_TEXT_FONT_SIZE,
      annotationHistory: emptyAnnotationHistory(),
      previewImageBase64: null,
      isAnnotationToolbarVisible: true,
      cursorColor: null,
      colorSampleFormat: 'hex',
      isMagnifierRequested: false,
      isRenderingOutput: false,
      silentOcrHint: null,
      includeCapturedCursor: false,
      error: null,
    });
  });

  it('creates a loaded host session patch for selecting with host cursor context', () => {
    const session = createCaptureSessionView({ id: 'loaded-session' });
    const cursorPoint = { x: 120, y: 80 };
    const hoverSelection = { x: 100, y: 70, width: 220, height: 140 };

    expect(
      loadedCaptureHostSessionPatch({
        session,
        cursorPoint,
        hoverSelection,
      }),
    ).toEqual({
      session,
      status: 'selecting',
      cursorPoint,
      hoverSelection,
    });
  });

  it('creates a full interaction reset patch matching ScreenshotSession reset intent', () => {
    expect(resetCaptureInteractionStatePatch()).toEqual({
      startPoint: null,
      cursorPoint: null,
      selection: null,
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
      previewImageBase64: null,
      isAnnotationToolbarVisible: true,
      cursorColor: null,
      colorSampleFormat: 'hex',
      isMagnifierRequested: false,
      isRenderingOutput: false,
      silentOcrHint: null,
      includeCapturedCursor: false,
      error: null,
    });
  });

  it('creates a preview reset patch from the editor reset state', () => {
    const { renderingOutput, ...resetState } = createCapturePreviewResetState();

    expect(previewResetPatch()).toEqual({
      ...resetState,
      isRenderingOutput: renderingOutput,
    });
  });
});
