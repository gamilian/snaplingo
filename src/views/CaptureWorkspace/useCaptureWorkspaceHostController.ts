import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MutableRefObject,
} from 'react';

import type { CaptureWorkspacePlatformRuntime } from '../../application/capture-workspace/platformRuntime';
import type { AnnotationHistory } from './annotationHistory';
import {
  canToggleCapturedCursor,
  copyCaptureSelection,
  printCaptureSelection,
  quickSaveCaptureSelection,
  saveCaptureSelection,
  type HoverSelectionCompletionAction,
  type PreviewCaptureCompletionAction,
  type SelectionHistoryStep,
  refreshCaptureSession,
} from './captureActions';
import {
  prepareCaptureSurfaceForReveal as prepareHostCaptureSurfaceForReveal,
  restoreCaptureSelectionFromHistory as restoreSelectionFromHostHistory,
  restoreLastSuccessfulCaptureSelection,
  type CaptureHostSessionStartPerfState,
  type CaptureHostSnapshotHydration,
} from './captureHostRuntime';
import {
  createCaptureWorkspaceHostActions,
  type CaptureWorkspaceHostAdapter,
} from './captureWorkspaceHost';
import type { CaptureSelectionOverlayFrame } from './captureSelectionOverlay';
import {
  resetCaptureInteractionStatePatch,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import { getCurrentMonitorBounds } from './virtualDesktop';
import type {
  AnnotationCommand,
  CaptureLaunch,
  CaptureMode,
  LogicalRect,
  Point,
} from './types';

interface CaptureWorkspaceHostStateController {
  state: CaptureWorkspaceState;
  applyPatch(patch: Partial<CaptureWorkspaceState>): void;
  resetInteraction(): void;
  resetSession(): void;
  resetPreview(): void;
  draftSelectionRef: MutableRefObject<LogicalRect | null>;
  startPointRef: MutableRefObject<Point | null>;
  cursorPointRef: MutableRefObject<Point | null>;
  hoverSelectionRef: MutableRefObject<LogicalRect | null>;
}

interface CaptureWorkspaceHostOverlay {
  reset(): void;
  getCurrentFrame(): CaptureSelectionOverlayFrame | null;
  paintFrame(frame: CaptureSelectionOverlayFrame | null): void;
}

interface CaptureWorkspaceHostDerivedState {
  areCaptureImagesReady: boolean;
  selectionBounds: LogicalRect | null;
}

interface UseCaptureWorkspaceHostControllerOptions {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
  screenshotSavePath?: string;
  minSelectionSize: number;
  workspace: CaptureWorkspaceHostStateController;
  derived: CaptureWorkspaceHostDerivedState;
  overlay: CaptureWorkspaceHostOverlay;
  setHydratedCaptureSessionId(sessionId: string | null): void;
  commitTextDraftToHistory(): AnnotationHistory;
  runtime: CaptureWorkspacePlatformRuntime;
}

export function useCaptureWorkspaceHostController({
  commitTextDraftToHistory,
  derived,
  initialMode,
  initialSessionId,
  minSelectionSize,
  onInactive,
  overlay,
  runtime,
  screenshotSavePath,
  setHydratedCaptureSessionId,
  workspace,
}: UseCaptureWorkspaceHostControllerOptions) {
  const {
    applyPatch,
    cursorPointRef,
    draftSelectionRef,
    hoverSelectionRef,
    resetInteraction,
    resetPreview,
    resetSession,
    startPointRef,
    state,
  } = workspace;
  const { areCaptureImagesReady, selectionBounds } = derived;
  const {
    getCurrentFrame: getSelectionOverlayCurrentFrame,
    paintFrame: paintSelectionOverlayFrame,
    reset: resetSelectionOverlay,
  } = overlay;
  const isCancellingSessionRef = useRef(false);
  const captureSnapshotHydrationRef =
    useRef<CaptureHostSnapshotHydration | null>(null);
  const isCompletingCaptureRef = useRef(false);
  const hasRevealedCaptureWindowRef = useRef(false);
  const captureFrontendPerfRef =
    useRef<CaptureHostSessionStartPerfState | null>(null);
  const [hasStartedInitialSession, setHasStartedInitialSession] = useState(false);

  const workspaceStateRef = useRef(state);
  workspaceStateRef.current = state;
  const getState = useCallback(() => workspaceStateRef.current, []);
  const workspaceHostAdapter = useMemo<CaptureWorkspaceHostAdapter>(
    () => ({
      getState,
      patch: (next) => {
        workspaceStateRef.current = {
          ...workspaceStateRef.current,
          ...next,
        };
        applyPatch(next);
      },
      clearDraftSelectionRef: () => {
        draftSelectionRef.current = null;
      },
      resetInteraction: () => {
        workspaceStateRef.current = {
          ...workspaceStateRef.current,
          ...resetCaptureInteractionStatePatch(),
        };
        resetInteraction();
      },
      resetSession: () => {
        workspaceStateRef.current = {
          ...workspaceStateRef.current,
          status: 'idle',
          session: null,
          ...resetCaptureInteractionStatePatch(),
        };
        resetSession();
      },
    }),
    [applyPatch, draftSelectionRef, getState, resetInteraction, resetSession],
  );

  const resetCaptureImageReadiness = useCallback(() => {
    captureSnapshotHydrationRef.current = null;
    setHydratedCaptureSessionId(null);
  }, [setHydratedCaptureSessionId]);

  const markCaptureFrontendPerf = useCallback(
    (event: string, sessionId?: string | null) => {
      const perf = captureFrontendPerfRef.current;
      if (!perf) return;

      void runtime.commands.logCaptureFrontendPerf({
        event,
        mode: perf.mode,
        sessionId: sessionId ?? perf.sessionId,
        elapsedMs: performance.now() - perf.startMs,
      }).catch(() => undefined);
    },
    [runtime],
  );

  const hostActions = useMemo(
    () =>
      createCaptureWorkspaceHostActions({
        workspace: workspaceHostAdapter,
        getScreenshotSavePath: () => screenshotSavePath,
        getOnInactive: () => onInactive,
        getSelection: () => workspaceHostAdapter.getState().selection,
        getAnnotations: () =>
          workspaceHostAdapter.getState().annotationHistory.annotations,
        getShouldIncludeCapturedCursor: () => {
          const currentState = workspaceHostAdapter.getState();
          return (
            currentState.includeCapturedCursor &&
            canToggleCapturedCursor(currentState.session)
          );
        },
        commitTextDraftToHistory,
        isCompletingCapture: () => isCompletingCaptureRef.current,
        setCompletingCapture: (isCompleting) => {
          isCompletingCaptureRef.current = isCompleting;
        },
        isCancellingSession: () => isCancellingSessionRef.current,
        setCancellingSession: (isCancelling) => {
          isCancellingSessionRef.current = isCancelling;
        },
        setRevealed: (hasRevealed) => {
          hasRevealedCaptureWindowRef.current = hasRevealed;
        },
        now: () => performance.now(),
        getPerfState: () => captureFrontendPerfRef.current,
        setPerfState: (state) => {
          captureFrontendPerfRef.current = state;
        },
        markPerf: markCaptureFrontendPerf,
        storage: window.localStorage,
        logWarning: (message, err) => {
          console.warn(message, err);
        },
        getSnapshotHydration: () => captureSnapshotHydrationRef.current,
        setSnapshotHydration: (hydration) => {
          captureSnapshotHydrationRef.current = hydration;
        },
        clearHydratedSession: () => setHydratedCaptureSessionId(null),
        markHydratedSession: setHydratedCaptureSessionId,
        resetSelectionOverlay,
        resetCaptureImageReadiness,
        clients: {
          createCaptureSession: runtime.commands.createCaptureSession,
          getCaptureSession: runtime.commands.getCaptureSession,
          refreshCaptureSession: (sessionId) =>
            refreshCaptureSession(sessionId, runtime.commands),
          currentCaptureCursorPosition:
            runtime.commands.currentCaptureCursorPosition,
          hydrateCaptureSessionSnapshots:
            runtime.commands.hydrateCaptureSessionSnapshots,
          cancelCaptureSession: runtime.commands.cancelCaptureSession,
          renderPreview: runtime.commands.renderCaptureOutput,
          runtimeEffectClient: {
            copyCaptureSelection: (sessionId, rect, annotations, includeCursor) =>
              copyCaptureSelection(
                sessionId,
                rect,
                annotations,
                includeCursor,
                runtime.commands,
              ),
            saveCaptureSelection: (sessionId, rect, annotations, includeCursor) =>
              saveCaptureSelection(
                sessionId,
                rect,
                annotations,
                includeCursor,
                runtime.commands,
              ),
            quickSaveCaptureSelection: (
              sessionId,
              rect,
              annotations,
              directory,
              includeCursor,
            ) =>
              quickSaveCaptureSelection(
                sessionId,
                rect,
                annotations,
                directory,
                includeCursor,
                runtime.commands,
              ),
            printCaptureSelection: (
              sessionId,
              rect,
              annotations,
              printImage,
              includeCursor,
            ) =>
              printCaptureSelection(
                sessionId,
                rect,
                annotations,
                printImage,
                includeCursor,
                runtime.commands,
              ),
            outputCapture: runtime.commands.outputCapture,
            runCaptureOcr: runtime.commands.runCaptureOcr,
            renderCaptureOutput: runtime.commands.renderCaptureOutput,
            openCaptureOcrResultWindow:
              runtime.commands.openCaptureOcrResultWindow,
            openCaptureTranslationResultWindow:
              runtime.commands.openCaptureTranslationResultWindow,
            copyTextToClipboard: runtime.commands.copyTextToClipboard,
          },
        },
      }),
    [
      commitTextDraftToHistory,
      markCaptureFrontendPerf,
      onInactive,
      resetSelectionOverlay,
      resetCaptureImageReadiness,
      runtime,
      screenshotSavePath,
      setHydratedCaptureSessionId,
      workspaceHostAdapter,
    ],
  );

  const ensureCaptureSnapshotsHydrated = useCallback(
    (sessionId: string) => hostActions.ensureCaptureSnapshotsHydrated(sessionId),
    [hostActions],
  );
  const startSession = useCallback(
    async (nextMode: CaptureMode, sessionId?: string) => {
      await hostActions.startSession(nextMode, sessionId);
    },
    [hostActions],
  );
  const cancelSession = useCallback(
    async () => {
      await hostActions.cancelSession();
    },
    [hostActions],
  );
  const renderSelectionPreview = useCallback(
    async (
      rect: LogicalRect,
      annotations?: AnnotationCommand[],
      includeCursor?: boolean,
    ) => {
      await hostActions.renderSelectionPreview(rect, annotations, includeCursor);
    },
    [hostActions],
  );
  const completePreviewSelection = useCallback(
    async (
      action: PreviewCaptureCompletionAction,
      options: { commitTextDraft?: boolean; guardCompletion?: boolean } = {},
    ) => {
      await hostActions.completePreviewSelection(action, options);
    },
    [hostActions],
  );
  const completeCandidateSelection = useCallback(
    async (rect: LogicalRect, action: HoverSelectionCompletionAction) => {
      await hostActions.completeCandidateSelection(rect, action);
    },
    [hostActions],
  );
  const completeManualSelection = useCallback(
    async (rect: LogicalRect) => {
      await hostActions.completeManualSelection(rect, getState().mode);
    },
    [getState, hostActions],
  );
  const refreshSession = useCallback(
    async () => {
      await hostActions.refreshSession();
    },
    [hostActions],
  );
  const copySelection = useCallback(
    async () => {
      await completePreviewSelection('copy', { guardCompletion: true });
    },
    [completePreviewSelection],
  );
  const saveSelection = useCallback(
    async () => {
      await completePreviewSelection('save');
    },
    [completePreviewSelection],
  );
  const quickSaveSelection = useCallback(
    async () => {
      await completePreviewSelection('quick-save');
    },
    [completePreviewSelection],
  );
  const runOcrSelection = useCallback(
    async () => {
      await completePreviewSelection('ocr', { commitTextDraft: false });
    },
    [completePreviewSelection],
  );
  const pinSelection = useCallback(
    async () => {
      await completePreviewSelection('pin');
    },
    [completePreviewSelection],
  );

  const handleNativeCopyRequest = useCallback(() => {
    const currentState = getState();
    if (currentState.status === 'preview') {
      void copySelection();
      return;
    }

    const activeStartPoint =
      startPointRef.current ?? currentState.startPoint;
    const activeHoverSelection =
      hoverSelectionRef.current ?? currentState.hoverSelection;
    if (
      currentState.status === 'selecting' &&
      !currentState.textDraft &&
      activeStartPoint === null &&
      activeHoverSelection
    ) {
      void completeCandidateSelection(activeHoverSelection, 'copy');
    }
  }, [
    completeCandidateSelection,
    copySelection,
    getState,
    hoverSelectionRef,
    startPointRef,
  ]);

  const resetPreviewSelection = useCallback(() => {
    resetPreview();
    resetSelectionOverlay();
  }, [resetPreview, resetSelectionOverlay]);

  const selectFullCaptureArea = useCallback(() => {
    const currentState = getState();
    if (!currentState.session || !selectionBounds) return;

    const currentPoint =
      cursorPointRef.current ??
      currentState.cursorPoint ??
      currentState.session.captured_cursor?.logical_position ??
      null;
    void completeManualSelection(
      getCurrentMonitorBounds(currentState.session.monitors, currentPoint),
    );
  }, [completeManualSelection, cursorPointRef, getState, selectionBounds]);

  const restoreLastSelection = useCallback(() => {
    if (!selectionBounds) return;

    restoreLastSuccessfulCaptureSelection({
      storage: window.localStorage,
      selectionBounds,
      minSelectionSize,
      completeSelection: completeManualSelection,
    });
  }, [completeManualSelection, minSelectionSize, selectionBounds]);

  const restoreSelectionFromHistory = useCallback(
    (step: SelectionHistoryStep) => {
      if (!selectionBounds) return;

      restoreSelectionFromHostHistory({
        storage: window.localStorage,
        currentSelection: getState().selection,
        step,
        selectionBounds,
        minSelectionSize,
        completeSelection: completeManualSelection,
      });
    },
    [completeManualSelection, getState, minSelectionSize, selectionBounds],
  );

  const prepareCaptureSurfaceForReveal = useCallback(async () => {
    await prepareHostCaptureSurfaceForReveal({
      frame: getSelectionOverlayCurrentFrame(),
      paintSelectionOverlayFrame,
    });
  }, [getSelectionOverlayCurrentFrame, paintSelectionOverlayFrame]);

  useEffect(() => {
    if (!initialMode || hasStartedInitialSession) return;

    setHasStartedInitialSession(true);
    void startSession(initialMode, initialSessionId);
  }, [hasStartedInitialSession, initialMode, initialSessionId, startSession]);

  useEffect(() => {
    const perf = captureFrontendPerfRef.current;
    const session = state.session;
    if (
      !session ||
      !areCaptureImagesReady ||
      !perf ||
      perf.hasLoggedImagesReady ||
      perf.sessionId !== session.id
    ) {
      return;
    }

    perf.hasLoggedImagesReady = true;
    markCaptureFrontendPerf('images_ready', session.id);
  }, [
    areCaptureImagesReady,
    markCaptureFrontendPerf,
    state.session,
  ]);

  const markCaptureHostWindowRevealed = useCallback(
    (sessionId: string) => {
      markCaptureFrontendPerf('revealed', sessionId);
    },
    [markCaptureFrontendPerf],
  );
  const handleCaptureHostRevealError = useCallback(
    (err: unknown) => {
      applyPatch({
        error: err instanceof Error ? err.message : String(err),
        status: 'error',
      });
    },
    [applyPatch],
  );

  const hostWindowReveal = useMemo(
    () => ({
      status: state.status,
      sessionId: state.session?.id ?? null,
      hasCaptureImagesReady: areCaptureImagesReady,
      hasRevealedRef: hasRevealedCaptureWindowRef,
      prepareSurface: prepareCaptureSurfaceForReveal,
      onRevealedSession: markCaptureHostWindowRevealed,
      onError: handleCaptureHostRevealError,
    }),
    [
      areCaptureImagesReady,
      handleCaptureHostRevealError,
      markCaptureHostWindowRevealed,
      prepareCaptureSurfaceForReveal,
      state.session?.id,
      state.status,
    ],
  );

  const handleCaptureHotkeyLaunch = useCallback(
    (launch: CaptureLaunch) => {
      void startSession(launch.mode, launch.sessionId);
    },
    [startSession],
  );
  const hostSubscriptions = useMemo(
    () => ({
      isActive: state.status !== 'idle',
      onLaunch: handleCaptureHotkeyLaunch,
      onCancel: cancelSession,
      onCopy: handleNativeCopyRequest,
    }),
    [
      cancelSession,
      handleCaptureHotkeyLaunch,
      handleNativeCopyRequest,
      state.status,
    ],
  );

  const actions = useMemo(
    () => ({
      ensureCaptureSnapshotsHydrated,
      cancelSession,
      renderSelectionPreview,
      completePreviewSelection,
      completeCandidateSelection,
      completeManualSelection,
      refreshSession,
      copySelection,
      saveSelection,
      quickSaveSelection,
      runOcrSelection,
      pinSelection,
      resetPreviewSelection,
      selectFullCaptureArea,
      restoreLastSelection,
      restoreSelectionFromHistory,
    }),
    [
      cancelSession,
      completeCandidateSelection,
      completeManualSelection,
      completePreviewSelection,
      copySelection,
      ensureCaptureSnapshotsHydrated,
      pinSelection,
      quickSaveSelection,
      refreshSession,
      renderSelectionPreview,
      resetPreviewSelection,
      restoreLastSelection,
      restoreSelectionFromHistory,
      runOcrSelection,
      saveSelection,
      selectFullCaptureArea,
    ],
  );

  return {
    hostWindowReveal,
    hostSubscriptions,
    actions,
  };
}
