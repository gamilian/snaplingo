import { describe, expect, it } from 'vitest';

import { emptyAnnotationHistory } from './annotationHistory';
import {
  DEFAULT_ANNOTATION_STYLE,
  DEFAULT_TEXT_FONT_SIZE,
} from './annotationStyle';
import { createInitialCaptureWorkspaceState } from './captureWorkspaceState';

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
      candidateDetectionMode: 'window',
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
});
