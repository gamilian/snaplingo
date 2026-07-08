import type { MutableRefObject } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { emptyAnnotationHistory } from './annotationHistory';
import {
  DEFAULT_ANNOTATION_STYLE,
  DEFAULT_TEXT_FONT_SIZE,
} from './annotationStyle';
import { createCapturePreviewResetState } from './captureEditorRuntime';
import {
  type CaptureWorkspaceState,
  createInitialCaptureWorkspaceState,
  loadedCaptureHostSessionPatch,
  previewResetPatch,
  resetCaptureInteractionStatePatch,
} from './captureWorkspaceState';
import {
  type CaptureWorkspaceRefs,
  applyCaptureWorkspaceStatePatch,
  createCaptureWorkspaceStateActions,
} from './useCaptureWorkspaceState';
import type { CaptureSessionView, LogicalRect, Point } from './types';

function createMutableRef<T>(current: T): MutableRefObject<T> {
  return { current };
}

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
  it('applies a workspace state patch without changing unspecified fields', () => {
    const initialState = createInitialCaptureWorkspaceState();

    const patchedState = applyCaptureWorkspaceStatePatch(initialState, {
      status: 'loading',
      error: 'Loading capture session',
    });

    expect(patchedState).toEqual({
      ...initialState,
      status: 'loading',
      error: 'Loading capture session',
    });
    expect(initialState.status).toBe('idle');
    expect(initialState.error).toBeNull();
  });

  it('applies reset helper patches to the same workspace state reset paths', () => {
    const dirtySession = createCaptureSessionView({ id: 'dirty-session' });
    const dirtyState: CaptureWorkspaceState = {
      ...createInitialCaptureWorkspaceState(),
      status: 'preview',
      session: dirtySession,
      startPoint: { x: 10, y: 20 },
      cursorPoint: { x: 30, y: 40 },
      selection: { x: 10, y: 20, width: 100, height: 80 },
      hoverSelection: { x: 12, y: 22, width: 90, height: 70 },
      previewImageBase64: 'preview-image',
      isRenderingOutput: true,
      includeCapturedCursor: true,
      error: 'Previous error',
    };
    const loadedSession = createCaptureSessionView({ id: 'loaded-session' });
    const loadedPatch = loadedCaptureHostSessionPatch({
      session: loadedSession,
      cursorPoint: { x: 50, y: 60 },
      hoverSelection: { x: 45, y: 55, width: 75, height: 65 },
    });

    expect(
      applyCaptureWorkspaceStatePatch(dirtyState, resetCaptureInteractionStatePatch()),
    ).toEqual({
      ...dirtyState,
      ...resetCaptureInteractionStatePatch(),
    });
    expect(
      applyCaptureWorkspaceStatePatch(dirtyState, {
        status: 'idle',
        session: null,
        ...resetCaptureInteractionStatePatch(),
      }),
    ).toEqual({
      ...dirtyState,
      status: 'idle',
      session: null,
      ...resetCaptureInteractionStatePatch(),
    });
    expect(applyCaptureWorkspaceStatePatch(dirtyState, loadedPatch)).toEqual({
      ...dirtyState,
      ...loadedPatch,
    });
    expect(applyCaptureWorkspaceStatePatch(dirtyState, previewResetPatch())).toEqual({
      ...dirtyState,
      ...previewResetPatch(),
    });
  });

  it('keeps ref-backed values synchronized through explicit workspace actions', () => {
    const refs: CaptureWorkspaceRefs = {
      startPointRef: createMutableRef<Point | null>(null),
      cursorPointRef: createMutableRef<Point | null>(null),
      draftSelectionRef: createMutableRef<LogicalRect | null>(null),
      hoverSelectionRef: createMutableRef<LogicalRect | null>(null),
    };
    const applyPatch = vi.fn<(patch: Partial<CaptureWorkspaceState>) => void>();
    const onRenderingOutputChange = vi.fn<(isRendering: boolean) => void>();
    const onHoverSelectionSynced = vi.fn<
      (nextHoverSelection: LogicalRect | null) => void
    >();
    const startPoint = { x: 10, y: 20 };
    const cursorPoint = { x: 30, y: 40 };
    const draftSelection = { x: 8, y: 18, width: 80, height: 60 };
    const hoverSelection = { x: 5, y: 15, width: 70, height: 50 };

    const actions = createCaptureWorkspaceStateActions({
      refs,
      applyPatch,
      onRenderingOutputChange,
      onHoverSelectionSynced,
    });
    actions.setStartPointWithRef(startPoint);
    actions.setCursorPointWithRef(cursorPoint);
    actions.setDraftSelectionWithRef(draftSelection);
    actions.syncHoverSelection(hoverSelection);
    actions.syncHoverSelection({ ...hoverSelection });
    actions.setRenderingOutput(true);

    expect(refs.startPointRef.current).toBe(startPoint);
    expect(refs.cursorPointRef.current).toBe(cursorPoint);
    expect(refs.draftSelectionRef.current).toBe(draftSelection);
    expect(refs.hoverSelectionRef.current).toBe(hoverSelection);
    expect(applyPatch).toHaveBeenCalledTimes(4);
    expect(applyPatch).toHaveBeenNthCalledWith(1, { startPoint });
    expect(applyPatch).toHaveBeenNthCalledWith(2, { cursorPoint });
    expect(applyPatch).toHaveBeenNthCalledWith(3, { hoverSelection });
    expect(applyPatch).toHaveBeenNthCalledWith(4, { isRenderingOutput: true });
    expect(onHoverSelectionSynced).toHaveBeenCalledOnce();
    expect(onHoverSelectionSynced).toHaveBeenCalledWith(hoverSelection);
    expect(onRenderingOutputChange).toHaveBeenCalledOnce();
    expect(onRenderingOutputChange).toHaveBeenCalledWith(true);
  });

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
