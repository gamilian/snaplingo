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
  createCaptureWorkspaceStateController,
} from './useCaptureWorkspaceState';
import type { CaptureSessionView, LogicalRect, Point } from './types';

type CaptureWorkspaceControllerOptions = {
  onRenderingOutputChange?: (isRendering: boolean) => void;
  onHoverSelectionSynced?: (nextHoverSelection: LogicalRect | null) => void;
};

function createCaptureWorkspaceControllerProbe(
  options?: CaptureWorkspaceControllerOptions,
) {
  let state = createInitialCaptureWorkspaceState();
  const refs: CaptureWorkspaceRefs = {
    startPointRef: createMutableRef<Point | null>(null),
    cursorPointRef: createMutableRef<Point | null>(null),
    draftSelectionRef: createMutableRef<LogicalRect | null>(null),
    hoverSelectionRef: createMutableRef<LogicalRect | null>(null),
  };
  const controller = createCaptureWorkspaceStateController({
    refs,
    setState(updateState) {
      state = updateState(state);
    },
    ...options,
  });

  return {
    act(action: (workspace: ReturnType<typeof getCurrent>) => void) {
      action(this.current);

      return this.current;
    },
    get current() {
      return getCurrent();
    },
  };

  function getCurrent() {
    return {
      ...state,
      ...controller,
      refs,
      startPointRef: refs.startPointRef,
      cursorPointRef: refs.cursorPointRef,
      draftSelectionRef: refs.draftSelectionRef,
      hoverSelectionRef: refs.hoverSelectionRef,
    };
  }
}

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
  it('syncs ref-backed state fields when applying controller patches', () => {
    const onRenderingOutputChange = vi.fn<(isRendering: boolean) => void>();
    const probe = createCaptureWorkspaceControllerProbe({
      onRenderingOutputChange,
    });
    const startPoint = { x: 10, y: 20 };
    const cursorPoint = { x: 30, y: 40 };
    const hoverSelection = { x: 12, y: 22, width: 90, height: 70 };

    const workspace = probe.act((currentWorkspace) => {
      currentWorkspace.applyPatch({
        cursorPoint,
        hoverSelection,
        isRenderingOutput: true,
        startPoint,
        status: 'selecting',
      });
    });

    expect(workspace.status).toBe('selecting');
    expect(workspace.startPoint).toBe(startPoint);
    expect(workspace.cursorPoint).toBe(cursorPoint);
    expect(workspace.hoverSelection).toBe(hoverSelection);
    expect(workspace.startPointRef.current).toBe(startPoint);
    expect(workspace.cursorPointRef.current).toBe(cursorPoint);
    expect(workspace.hoverSelectionRef.current).toBe(hoverSelection);
    expect(onRenderingOutputChange).toHaveBeenCalledOnce();
    expect(onRenderingOutputChange).toHaveBeenCalledWith(true);
  });

  it('clears refs and interaction state when resetting controller interaction', () => {
    const onRenderingOutputChange = vi.fn<(isRendering: boolean) => void>();
    const probe = createCaptureWorkspaceControllerProbe({
      onRenderingOutputChange,
    });
    const draftSelection = { x: 5, y: 10, width: 40, height: 30 };

    probe.act((workspace) => {
      workspace.applyPatch({
        cursorPoint: { x: 30, y: 40 },
        error: 'Previous error',
        hoverSelection: { x: 12, y: 22, width: 90, height: 70 },
        includeCapturedCursor: true,
        isRenderingOutput: true,
        selection: { x: 10, y: 20, width: 100, height: 80 },
        startPoint: { x: 10, y: 20 },
        status: 'preview',
      });
      workspace.setDraftSelectionWithRef(draftSelection);
    });
    onRenderingOutputChange.mockClear();

    const workspace = probe.act((currentWorkspace) => {
      currentWorkspace.resetInteraction();
    });

    expect(workspace.status).toBe('preview');
    expect(workspace.startPoint).toBeNull();
    expect(workspace.cursorPoint).toBeNull();
    expect(workspace.selection).toBeNull();
    expect(workspace.hoverSelection).toBeNull();
    expect(workspace.error).toBeNull();
    expect(workspace.isRenderingOutput).toBe(false);
    expect(workspace.includeCapturedCursor).toBe(false);
    expect(workspace.startPointRef.current).toBeNull();
    expect(workspace.cursorPointRef.current).toBeNull();
    expect(workspace.draftSelectionRef.current).toBeNull();
    expect(workspace.hoverSelectionRef.current).toBeNull();
    expect(onRenderingOutputChange).toHaveBeenCalledOnce();
    expect(onRenderingOutputChange).toHaveBeenCalledWith(false);
  });

  it('clears refs and session state when resetting a controller session', () => {
    const onRenderingOutputChange = vi.fn<(isRendering: boolean) => void>();
    const probe = createCaptureWorkspaceControllerProbe({
      onRenderingOutputChange,
    });
    const session = createCaptureSessionView({ id: 'dirty-session' });

    probe.act((workspace) => {
      workspace.applyPatch({
        cursorPoint: { x: 30, y: 40 },
        hoverSelection: { x: 12, y: 22, width: 90, height: 70 },
        isRenderingOutput: true,
        session,
        startPoint: { x: 10, y: 20 },
        status: 'preview',
      });
      workspace.setDraftSelectionWithRef({ x: 5, y: 10, width: 40, height: 30 });
    });
    onRenderingOutputChange.mockClear();

    const workspace = probe.act((currentWorkspace) => {
      currentWorkspace.resetSession();
    });

    expect(workspace.status).toBe('idle');
    expect(workspace.session).toBeNull();
    expect(workspace.startPoint).toBeNull();
    expect(workspace.cursorPoint).toBeNull();
    expect(workspace.hoverSelection).toBeNull();
    expect(workspace.startPointRef.current).toBeNull();
    expect(workspace.cursorPointRef.current).toBeNull();
    expect(workspace.draftSelectionRef.current).toBeNull();
    expect(workspace.hoverSelectionRef.current).toBeNull();
    expect(onRenderingOutputChange).toHaveBeenCalledOnce();
    expect(onRenderingOutputChange).toHaveBeenCalledWith(false);
  });

  it('syncs loaded session cursor and hover refs through the controller', () => {
    const probe = createCaptureWorkspaceControllerProbe();
    const session = createCaptureSessionView({ id: 'loaded-session' });
    const cursorPoint = { x: 50, y: 60 };
    const hoverSelection = { x: 45, y: 55, width: 75, height: 65 };

    const workspace = probe.act((currentWorkspace) => {
      currentWorkspace.applyLoadedSession({
        cursorPoint,
        hoverSelection,
        session,
      });
    });

    expect(workspace.status).toBe('selecting');
    expect(workspace.session).toBe(session);
    expect(workspace.cursorPoint).toBe(cursorPoint);
    expect(workspace.hoverSelection).toBe(hoverSelection);
    expect(workspace.cursorPointRef.current).toBe(cursorPoint);
    expect(workspace.hoverSelectionRef.current).toBe(hoverSelection);
  });

  it('clears refs and maps preview rendering output when resetting preview state', () => {
    const onRenderingOutputChange = vi.fn<(isRendering: boolean) => void>();
    const probe = createCaptureWorkspaceControllerProbe({
      onRenderingOutputChange,
    });

    probe.act((workspace) => {
      workspace.applyPatch({
        cursorPoint: { x: 30, y: 40 },
        hoverSelection: { x: 12, y: 22, width: 90, height: 70 },
        isRenderingOutput: true,
        selection: { x: 10, y: 20, width: 100, height: 80 },
        startPoint: { x: 10, y: 20 },
        status: 'preview',
      });
      workspace.setDraftSelectionWithRef({ x: 5, y: 10, width: 40, height: 30 });
    });
    onRenderingOutputChange.mockClear();

    const workspace = probe.act((currentWorkspace) => {
      currentWorkspace.resetPreview();
    });

    expect(workspace.status).toBe('selecting');
    expect(workspace.startPoint).toBeNull();
    expect(workspace.cursorPoint).toBeNull();
    expect(workspace.selection).toBeNull();
    expect(workspace.hoverSelection).toBeNull();
    expect(workspace.isRenderingOutput).toBe(false);
    expect(workspace.startPointRef.current).toBeNull();
    expect(workspace.cursorPointRef.current).toBeNull();
    expect(workspace.draftSelectionRef.current).toBeNull();
    expect(workspace.hoverSelectionRef.current).toBeNull();
    expect(onRenderingOutputChange).toHaveBeenCalledOnce();
    expect(onRenderingOutputChange).toHaveBeenCalledWith(false);
  });

  it('supports functional field setters from the controller', () => {
    const probe = createCaptureWorkspaceControllerProbe();

    const workspace = probe.act((currentWorkspace) => {
      currentWorkspace.setTextFontSize((fontSize) => fontSize + 2);
      currentWorkspace.setColorSampleFormat((format) =>
        format === 'hex' ? 'rgb' : 'hex',
      );
    });

    expect(workspace.textFontSize).toBe(DEFAULT_TEXT_FONT_SIZE + 2);
    expect(workspace.colorSampleFormat).toBe('rgb');
  });

  it('fires rendering output callbacks once through controller bulk and explicit actions', () => {
    const onRenderingOutputChange = vi.fn<(isRendering: boolean) => void>();
    const probe = createCaptureWorkspaceControllerProbe({
      onRenderingOutputChange,
    });

    probe.act((workspace) => {
      workspace.applyPatch({ isRenderingOutput: true });
    });
    expect(onRenderingOutputChange.mock.calls).toEqual([[true]]);
    onRenderingOutputChange.mockClear();

    probe.act((workspace) => {
      workspace.setRenderingOutput(false);
    });
    expect(onRenderingOutputChange.mock.calls).toEqual([[false]]);
    onRenderingOutputChange.mockClear();

    probe.act((workspace) => {
      workspace.resetPreview();
    });
    expect(onRenderingOutputChange.mock.calls).toEqual([[false]]);
    onRenderingOutputChange.mockClear();

    probe.act((workspace) => {
      workspace.resetInteraction();
    });
    expect(onRenderingOutputChange.mock.calls).toEqual([[false]]);
  });

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
