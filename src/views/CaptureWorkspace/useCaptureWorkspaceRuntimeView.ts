import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import { createCaptureWorkspaceRuntime } from '../../application/capture-workspace/runtime';
import type { CaptureWorkspaceRuntime } from '../../application/capture-workspace/types';
import { prepareCaptureSurfaceForReveal } from './captureHostRuntime';
import {
  shouldPollCaptureHoverSelection,
  startCaptureHoverSelectionPolling,
} from './captureHoverPolling';
import { useCaptureMagnifierPixelSource } from './captureMagnifierRuntime';
import { useCaptureSelectionOverlay } from './captureSelectionOverlayRuntime';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import { ANNOTATION_COLORS, type AnnotationColor } from './annotationStyle';
import type {
  CaptureWorkspaceViewActions,
  CaptureWorkspaceViewRenderState,
} from './CaptureWorkspaceView';
import type { CaptureMode, LogicalRect } from './types';
import { useCaptureWorkspaceRuntime } from './runtimeContext';

const TOOLBAR_GAP = 14;
const TOOLBAR_SIZE = { width: 700, height: 42 };
const CAPTURE_HOVER_POLL_INTERVAL_MS = 16;

interface CaptureWorkspaceRuntimeViewOptions {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
  annotationColorPresets?: readonly AnnotationColor[];
  screenshotSavePath?: string;
}

export interface CaptureWorkspaceRuntimeView {
  renderState: CaptureWorkspaceViewRenderState;
  actions: CaptureWorkspaceViewActions;
}

export function useCaptureWorkspaceRuntimeView({
  initialMode,
  initialSessionId,
  onInactive,
  annotationColorPresets,
  screenshotSavePath,
}: CaptureWorkspaceRuntimeViewOptions): CaptureWorkspaceRuntimeView {
  const platformRuntime = useCaptureWorkspaceRuntime();
  const onInactiveRef = useRef(onInactive);
  const annotationColorPresetsRef = useRef(annotationColorPresets);
  const screenshotSavePathRef = useRef(screenshotSavePath);
  const hostBridgeRef = useRef<{
    reset(): void;
    prepareSurface(): Promise<void>;
    scheduleSelectionOverlayPaint(): void;
  }>({
    reset: () => undefined,
    prepareSurface: async () => undefined,
    scheduleSelectionOverlayPaint: () => undefined,
  });
  onInactiveRef.current = onInactive;
  annotationColorPresetsRef.current = annotationColorPresets;
  screenshotSavePathRef.current = screenshotSavePath;

  const [runtimeRevision, setRuntimeRevision] = useState(0);
  const disposedRuntimeRef = useRef<CaptureWorkspaceRuntime | null>(null);
  const workflowRuntime = useMemo(
    () =>
      createCaptureWorkspaceRuntime({
        platform: platformRuntime,
        onInactive: () => onInactiveRef.current?.(),
        annotationColorPresets: () =>
          annotationColorPresetsRef.current ?? ANNOTATION_COLORS,
        screenshotSavePath: () => screenshotSavePathRef.current,
        storage: window.localStorage,
        host: {
          resetInteraction: () => hostBridgeRef.current.reset(),
          resetSession: () => hostBridgeRef.current.reset(),
          prepareSurface: () => hostBridgeRef.current.prepareSurface(),
          scheduleSelectionOverlayPaint: () =>
            hostBridgeRef.current.scheduleSelectionOverlayPaint(),
        },
        keyboard: { target: window },
      }),
    [platformRuntime, runtimeRevision],
  );
  const [runtimeRenderState, setRuntimeRenderState] = useState(
    () => workflowRuntime.renderState,
  );

  useEffect(() => {
    setRuntimeRenderState(workflowRuntime.renderState);
    return workflowRuntime.subscribe(() => {
      setRuntimeRenderState(workflowRuntime.renderState);
    });
  }, [workflowRuntime]);

  const derived = useMemo(
    () =>
      getCaptureWorkspaceDerivedState({
        state: runtimeRenderState,
        hydratedCaptureSessionId: runtimeRenderState.hasHydratedPixelSource
          ? runtimeRenderState.sessionId
          : null,
        toolbarGap: TOOLBAR_GAP,
        toolbarSize: TOOLBAR_SIZE,
      }),
    [runtimeRenderState],
  );
  const cursorPointRef = useRef(runtimeRenderState.cursorPoint);
  const draftSelectionRef = useRef<LogicalRect | null>(null);
  const hoverSelectionRef = useRef(runtimeRenderState.hoverSelection);
  cursorPointRef.current = runtimeRenderState.cursorPoint;
  draftSelectionRef.current =
    runtimeRenderState.status === 'selecting' && runtimeRenderState.startPoint
      ? runtimeRenderState.selection
      : null;
  hoverSelectionRef.current = runtimeRenderState.hoverSelection;

  const overlay = useCaptureSelectionOverlay({
    status: runtimeRenderState.status,
    selectionBounds: derived.selectionBounds,
    selection:
      runtimeRenderState.status === 'preview'
        ? runtimeRenderState.selection
        : null,
    viewportBounds: derived.viewportBounds,
    cursorPointRef,
    draftSelectionRef,
    hoverSelectionRef,
  });

  useEffect(() => {
    if (!runtimeRenderState.session || !derived.selectionBounds) return;

    return startCaptureHoverSelectionPolling({
      sessionId: runtimeRenderState.session.id,
      candidates: derived.captureCandidates,
      shouldTrackMagnifierCursor: derived.shouldTrackMagnifierCursor,
      intervalMs: CAPTURE_HOVER_POLL_INTERVAL_MS,
      canPoll: () =>
        shouldPollCaptureHoverSelection({
          status: runtimeRenderState.status,
          hasSession: true,
          hasSelectionBounds: true,
          hasActiveStartPoint: runtimeRenderState.startPoint !== null,
          hasEditGesture: runtimeRenderState.editGesture !== null,
        }),
      getCursorPosition: platformRuntime.commands.currentCaptureCursorPosition,
      setCursorPointRef: workflowRuntime.actions.updatePolledCursor,
      setCursorPoint: () => undefined,
      scheduleSelectionOverlayPaint: overlay.schedulePaint,
      syncHoverSelection: workflowRuntime.actions.updatePolledHover,
      setTimeout: window.setTimeout,
      clearTimeout: window.clearTimeout,
    });
  }, [
    derived.captureCandidates,
    derived.selectionBounds,
    derived.shouldTrackMagnifierCursor,
    overlay.schedulePaint,
    platformRuntime.commands.currentCaptureCursorPosition,
    runtimeRenderState.editGesture,
    runtimeRenderState.session,
    runtimeRenderState.startPoint,
    runtimeRenderState.status,
    workflowRuntime,
  ]);

  const initialSessionRuntimeRef = useRef<CaptureWorkspaceRuntime | null>(null);
  useEffect(() => {
    if (!initialMode || initialSessionRuntimeRef.current === workflowRuntime) {
      return;
    }
    initialSessionRuntimeRef.current = workflowRuntime;
    void workflowRuntime.actions.startSession(initialMode, initialSessionId);
  }, [initialMode, initialSessionId, workflowRuntime]);

  const prepareCaptureSurface = useCallback(
    () =>
      prepareCaptureSurfaceForReveal({
        frame: overlay.getCurrentFrame(),
        paintSelectionOverlayFrame: overlay.paintFrame,
      }),
    [overlay.getCurrentFrame, overlay.paintFrame],
  );
  hostBridgeRef.current = {
    reset: overlay.reset,
    prepareSurface: prepareCaptureSurface,
    scheduleSelectionOverlayPaint: overlay.schedulePaint,
  };

  useEffect(() => {
    if (disposedRuntimeRef.current === workflowRuntime) {
      setRuntimeRevision((revision) => revision + 1);
      return;
    }

    void workflowRuntime.actions.connectHost().catch(() => undefined);
    return () => {
      disposedRuntimeRef.current = workflowRuntime;
      workflowRuntime.dispose();
    };
  }, [workflowRuntime]);

  useEffect(() => {
    void workflowRuntime.actions.updateHostReadiness(
      derived.areCaptureImagesReady,
    );
  }, [
    derived.areCaptureImagesReady,
    runtimeRenderState.sessionId,
    runtimeRenderState.status,
    workflowRuntime,
  ]);

  const ensureCaptureSnapshotsHydrated = useCallback(
    async () => {
      await workflowRuntime.actions.hydrateSnapshots();
    },
    [workflowRuntime],
  );
  useCaptureMagnifierPixelSource({
    session: runtimeRenderState.session,
    hasHydratedPixelSource: runtimeRenderState.hasHydratedPixelSource,
    isMagnifierRequested: runtimeRenderState.isMagnifierRequested,
    isMagnifierShown: derived.isMagnifierShown,
    cursorMonitor: derived.cursorMonitor,
    cursorInMonitorPoint: derived.cursorInMonitorPoint,
    setCursorColor: workflowRuntime.actions.updateCursorColor,
    ensureCaptureSnapshotsHydrated,
  });

  const textDraftInputRef = useRef<HTMLTextAreaElement | null>(null);
  useEffect(() => {
    if (!runtimeRenderState.textDraft) return;
    requestAnimationFrame(() => textDraftInputRef.current?.focus());
  }, [runtimeRenderState.textDraft]);

  const renderState = useMemo<CaptureWorkspaceViewRenderState>(
    () => ({
      status: runtimeRenderState.status,
      error: runtimeRenderState.error,
      viewportBounds: derived.viewportBounds,
      selectionBounds: derived.selectionBounds,
      isRenderingOutput: runtimeRenderState.isRenderingOutput,
      editor: {
        selection: runtimeRenderState.selection,
        selectionViewportRect: derived.selectionViewportRect,
        previewImageBase64: runtimeRenderState.previewImageBase64,
        annotations: derived.annotations.filter(
          (_, index) =>
            index !== runtimeRenderState.annotationMoveGesture?.annotationIndex &&
            index !== runtimeRenderState.textDraftAnnotationIndex,
        ),
        draftAnnotation: runtimeRenderState.draftAnnotation,
        textDraft: runtimeRenderState.textDraft,
        annotationStyle: runtimeRenderState.annotationStyle,
        selectedAnnotationBounds: derived.selectedAnnotationBounds,
        activeAnnotationTool: runtimeRenderState.activeAnnotationTool,
      },
      toolbar: {
        position: derived.toolbarPosition,
        width: TOOLBAR_SIZE.width,
        isVisible: runtimeRenderState.isAnnotationToolbarVisible,
        textFontSize: runtimeRenderState.textFontSize,
        isTextSizingActive: derived.isTextSizingActive,
        isFillModeActive: derived.isFillModeActive,
        canUndo: derived.canUndoAnnotation,
        canRedo: derived.canRedoAnnotation,
      },
      dom: {
        textDraftInputRef,
        selectionOverlay: {
          canvasRef: overlay.canvasRef,
          cssSize: overlay.cssSize,
          pixelRatio: overlay.pixelRatio,
        },
      },
      magnifier: {
        isShown: derived.isMagnifierShown,
        cursorMonitor: derived.cursorMonitor,
        cursorViewportPoint: derived.cursorViewportPoint,
        cursorInMonitorPoint: derived.cursorInMonitorPoint,
        selection:
          runtimeRenderState.selection ?? runtimeRenderState.hoverSelection,
        cursorColor: runtimeRenderState.cursorColor,
        colorSampleFormat: runtimeRenderState.colorSampleFormat,
      },
    }),
    [
      derived,
      overlay.canvasRef,
      overlay.cssSize,
      overlay.pixelRatio,
      runtimeRenderState,
    ],
  );
  const actions = useMemo<CaptureWorkspaceViewActions>(
    () => ({
      pointerDown: workflowRuntime.actions.pointerDown,
      pointerMove: workflowRuntime.actions.pointerMove,
      pointerUp: workflowRuntime.actions.pointerUp,
      resizePointerDown: workflowRuntime.actions.resizePointerDown,
      resizeAnnotationPointerDown:
        workflowRuntime.actions.resizeAnnotationPointerDown,
      wheel: workflowRuntime.actions.wheel,
      commitTextDraft: workflowRuntime.actions.commitTextDraft,
      updateTextDraftText: workflowRuntime.actions.updateTextDraftText,
      discardTextDraft: workflowRuntime.actions.discardTextDraft,
      selectMoveTool: workflowRuntime.actions.selectMoveTool,
      toggleAnnotationTool: workflowRuntime.actions.toggleAnnotationTool,
      applySelectedAnnotationStyle:
        workflowRuntime.actions.applySelectedAnnotationStyle,
      updateTextDraftFontSize: workflowRuntime.actions.updateTextDraftFontSize,
      undoAnnotation: workflowRuntime.actions.undoAnnotation,
      redoAnnotation: workflowRuntime.actions.redoAnnotation,
      cancelSession: workflowRuntime.actions.cancelSession,
      completePreviewSelection:
        workflowRuntime.actions.completePreviewSelection,
    }),
    [workflowRuntime],
  );

  return { renderState, actions };
}
