import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import { createCaptureWorkspaceRuntime } from '../../application/capture-workspace/runtime';
import type { CaptureWorkspaceRuntimeActions } from '../../application/capture-workspace/types';
import { prepareCaptureSurfaceForReveal } from './captureHostRuntime';
import {
  shouldPollCaptureHoverSelection,
  startCaptureHoverSelectionPolling,
} from './captureHoverPolling';
import { useCaptureMagnifierPixelSource } from './captureMagnifierRuntime';
import { useCaptureSelectionOverlay } from './captureSelectionOverlayRuntime';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import type { CaptureWorkspaceViewRenderState } from './CaptureWorkspaceView';
import type { CaptureMode, LogicalRect } from './types';
import { useCaptureWorkspaceRuntime } from './runtimeContext';

const TOOLBAR_GAP = 14;
const TOOLBAR_SIZE = { width: 640, height: 42 };
const CAPTURE_HOVER_POLL_INTERVAL_MS = 16;

interface CaptureWorkspaceRuntimeViewOptions {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
  screenshotSavePath?: string;
}

export interface CaptureWorkspaceRuntimeView {
  renderState: CaptureWorkspaceViewRenderState;
  actions: CaptureWorkspaceRuntimeActions;
}

export function useCaptureWorkspaceRuntimeView({
  initialMode,
  initialSessionId,
  onInactive,
  screenshotSavePath,
}: CaptureWorkspaceRuntimeViewOptions): CaptureWorkspaceRuntimeView {
  const platformRuntime = useCaptureWorkspaceRuntime();
  const onInactiveRef = useRef(onInactive);
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
  screenshotSavePathRef.current = screenshotSavePath;

  const workflowRuntime = useMemo(
    () =>
      createCaptureWorkspaceRuntime({
        platform: platformRuntime,
        onInactive: () => onInactiveRef.current?.(),
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
    [platformRuntime],
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

  const hasStartedInitialSessionRef = useRef(false);
  useEffect(() => {
    if (!initialMode || hasStartedInitialSessionRef.current) return;
    hasStartedInitialSessionRef.current = true;
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
    let disposed = false;
    let disconnect: (() => void) | undefined;
    void workflowRuntime.actions.connectHost().then((nextDisconnect) => {
      if (disposed) nextDisconnect();
      else disconnect = nextDisconnect;
    });
    return () => {
      disposed = true;
      disconnect?.();
    };
  }, [workflowRuntime]);

  useEffect(() => {
    void workflowRuntime.actions.updateHostReadiness(
      derived.areCaptureImagesReady,
    );
  }, [derived.areCaptureImagesReady, workflowRuntime]);

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
      ...runtimeRenderState,
      ...derived,
      hasHydratedPixelSource: runtimeRenderState.hasHydratedPixelSource,
      toolbarWidth: TOOLBAR_SIZE.width,
      magnifierSelection:
        runtimeRenderState.selection ?? runtimeRenderState.hoverSelection,
      textDraftInputRef,
      selectionOverlayCanvasRef: overlay.canvasRef,
      selectionOverlayCssSize: overlay.cssSize,
      selectionOverlayPixelRatio: overlay.pixelRatio,
    }),
    [
      derived,
      overlay.canvasRef,
      overlay.cssSize,
      overlay.pixelRatio,
      runtimeRenderState,
    ],
  );

  return { renderState, actions: workflowRuntime.actions };
}
