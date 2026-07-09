import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent,
  type WheelEvent,
} from 'react';
import { writeClipboardText } from '../../tauri/clipboard';
import {
  cancelCaptureSession,
  createCaptureSession,
  currentCaptureCursorPosition,
  getCaptureSession,
  hydrateCaptureSessionSnapshots,
  logCaptureFrontendPerf,
} from '../../tauri/captureSession';
import {
  type SelectionHandle,
} from './selection';
import { colorSampleToClipboardText } from './colorSampler';
import {
  clearAnnotationHistory,
  removeAnnotationFromHistory,
  redoAnnotationHistory,
  undoAnnotationHistory,
} from './annotationHistory';
import {
  type AnnotationColor,
  type AnnotationStyle,
  type AnnotationSizeDirection,
  type AnnotationTool,
} from './annotationStyle';
import {
  updateTextAnnotationDraft,
} from './textAnnotationDraft';
import {
  canToggleCapturedCursor,
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
import {
  applyStyleToSelectedAnnotationHistory,
  commitCaptureEditorTextDraft,
  completeCaptureEditorGesture,
  getCaptureEditorDismissAction,
  planCaptureAnnotationColorSelection,
  planCaptureAnnotationToolActivation,
  planCaptureAnnotationFillToggle,
  planCaptureAnnotationSizeAdjustment,
  undoPolylineCaptureGesture,
} from './captureEditorRuntime';
import {
  shouldPollCaptureHoverSelection,
  startCaptureHoverSelectionPolling,
} from './captureHoverPolling';
import { useCaptureSelectionOverlay } from './captureSelectionOverlayRuntime';
import {
  useCaptureMagnifierPixelSource,
} from './captureMagnifierRuntime';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import {
  resetCaptureInteractionStatePatch,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import {
  handleCaptureWorkspaceKeyDown,
  type CaptureWorkspaceKeyboardActions,
  type CaptureWorkspaceKeyboardDerivedState,
  type CaptureWorkspaceKeyboardRefs,
} from './captureWorkspaceKeyboard';
import {
  handleCaptureWorkspacePointerDown,
  handleCaptureWorkspacePointerMove,
  handleCaptureWorkspacePointerUp,
  handleCaptureWorkspacePreviewPointerDown,
  handleCaptureWorkspaceResizePointerDown,
  handleCaptureWorkspaceWheel,
  type CaptureWorkspacePointerActions,
  type CaptureWorkspacePointerContext,
  type CaptureWorkspacePointerDerivedState,
  type CaptureWorkspacePointerRefs,
} from './captureWorkspacePointer';
import {
  getCurrentMonitorBounds,
} from './virtualDesktop';
import { useCaptureWorkspaceState } from './useCaptureWorkspaceState';
import type {
  AnnotationCommand,
  CaptureLaunch,
  CaptureMode,
  LogicalRect,
  Point,
} from './types';

const MIN_SELECTION_SIZE = 10;
const TOOLBAR_GAP = 14;
const TOOLBAR_SIZE = { width: 1220, height: 56 };
const CAPTURE_HOVER_POLL_INTERVAL_MS = 16;

function areRectsEqual(a: LogicalRect | null, b: LogicalRect | null) {
  if (a === b) return true;
  if (!a || !b) return false;

  return (
    a.x === b.x &&
    a.y === b.y &&
    a.width === b.width &&
    a.height === b.height
  );
}

interface CaptureWorkspaceControllerOptions {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
  screenshotSavePath?: string;
}

export function useCaptureWorkspaceController({
  initialMode,
  initialSessionId,
  onInactive,
  screenshotSavePath,
}: CaptureWorkspaceControllerOptions) {
  const textDraftInputRef = useRef<HTMLTextAreaElement | null>(null);
  const keyboardDraftCursorPointRef = useRef<Point | null>(null);
  const keyboardEditCursorPointRef = useRef<Point | null>(null);
  const isCancellingSessionRef = useRef(false);
  const captureSnapshotHydrationRef =
    useRef<CaptureHostSnapshotHydration | null>(null);
  const isCompletingCaptureRef = useRef(false);
  const hasRevealedCaptureWindowRef = useRef(false);
  const captureFrontendPerfRef =
    useRef<CaptureHostSessionStartPerfState | null>(null);
  const isRenderingOutputRef = useRef(false);
  const handleRenderingOutputChange = useCallback((nextIsRendering: boolean) => {
    isRenderingOutputRef.current = nextIsRendering;
  }, []);
  const {
    status,
    setStatus,
    mode,
    session,
    startPoint,
    cursorPoint,
    setCursorPoint,
    selection,
    setSelection,
    hoverSelection,
    setHoverSelection,
    editGesture,
    setEditGesture,
    activeAnnotationTool,
    setActiveAnnotationTool,
    annotationGesture,
    setAnnotationGesture,
    draftAnnotation,
    setDraftAnnotation,
    selectedAnnotationIndex,
    setSelectedAnnotationIndex,
    annotationMoveGesture,
    setAnnotationMoveGesture,
    draftSelectionMoveGesture,
    setDraftSelectionMoveGesture,
    textDraft,
    setTextDraft,
    textDraftAnnotationIndex,
    setTextDraftAnnotationIndex,
    annotationStyle,
    setAnnotationStyle,
    textFontSize,
    setTextFontSize,
    annotationHistory,
    setAnnotationHistory,
    previewImageBase64,
    setPreviewImageBase64,
    isAnnotationToolbarVisible,
    setIsAnnotationToolbarVisible,
    cursorColor,
    setCursorColor,
    colorSampleFormat,
    setColorSampleFormat,
    isMagnifierRequested,
    setIsMagnifierRequested,
    isRenderingOutput,
    setRenderingOutput,
    includeCapturedCursor,
    setIncludeCapturedCursor,
    error,
    setError,
    applyPatch,
    startPointRef,
    cursorPointRef,
    draftSelectionRef,
    hoverSelectionRef,
    setStartPointWithRef,
    syncHoverSelection: syncWorkspaceHoverSelection,
    resetInteraction,
    resetSession,
    resetPreview,
  } = useCaptureWorkspaceState({
    onRenderingOutputChange: handleRenderingOutputChange,
  });
  const [hasStartedInitialSession, setHasStartedInitialSession] = useState(false);
  const [hydratedCaptureSessionId, setHydratedCaptureSessionId] =
    useState<string | null>(null);

  const captureWorkspaceState: CaptureWorkspaceState = {
    status,
    mode,
    session,
    startPoint,
    cursorPoint,
    selection,
    hoverSelection,
    editGesture,
    activeAnnotationTool,
    annotationGesture,
    draftAnnotation,
    selectedAnnotationIndex,
    annotationMoveGesture,
    draftSelectionMoveGesture,
    textDraft,
    textDraftAnnotationIndex,
    annotationStyle,
    textFontSize,
    annotationHistory,
    previewImageBase64,
    isAnnotationToolbarVisible,
    cursorColor,
    colorSampleFormat,
    isMagnifierRequested,
    isRenderingOutput,
    includeCapturedCursor,
    error,
  };
  const captureWorkspaceStateRef =
    useRef<CaptureWorkspaceState>(captureWorkspaceState);
  captureWorkspaceStateRef.current = captureWorkspaceState;
  const getCurrentCaptureWorkspaceState = useCallback(
    () => captureWorkspaceStateRef.current,
    [],
  );
  const workspaceHostAdapter = useMemo<CaptureWorkspaceHostAdapter>(
    () => ({
      getState: getCurrentCaptureWorkspaceState,
      patch: (next) => {
        captureWorkspaceStateRef.current = {
          ...captureWorkspaceStateRef.current,
          ...next,
        };
        applyPatch(next);
      },
      clearDraftSelectionRef: () => {
        draftSelectionRef.current = null;
      },
      resetInteraction: () => {
        captureWorkspaceStateRef.current = {
          ...captureWorkspaceStateRef.current,
          ...resetCaptureInteractionStatePatch(),
        };
        resetInteraction();
      },
      resetSession: () => {
        captureWorkspaceStateRef.current = {
          ...captureWorkspaceStateRef.current,
          status: 'idle',
          session: null,
          ...resetCaptureInteractionStatePatch(),
        };
        resetSession();
      },
    }),
    [
      applyPatch,
      draftSelectionRef,
      getCurrentCaptureWorkspaceState,
      resetInteraction,
      resetSession,
    ],
  );

  const isActive = status !== 'idle';
  const derived = useMemo(
    () =>
      getCaptureWorkspaceDerivedState({
        state: captureWorkspaceState,
        hydratedCaptureSessionId,
        toolbarGap: TOOLBAR_GAP,
        toolbarSize: TOOLBAR_SIZE,
      }),
    [captureWorkspaceState, hydratedCaptureSessionId],
  );
  const {
    annotations,
    hasAnnotationEditingContext,
    canUndoAnnotation,
    canRedoAnnotation,
    isTextSizingActive,
    isFillModeActive,
    captureCandidates,
    areCaptureImagesReady,
    snapTargetRects,
    selectionBounds,
    viewportBounds,
    selectionViewportRect,
    cursorViewportPoint,
    cursorInMonitorPoint,
    cursorMonitor,
    hasHydratedPixelSource,
    isMagnifierShown,
    shouldTrackMagnifierCursor,
    selectedAnnotationBounds,
    toolbarPosition,
  } = derived;
  const {
    canvasRef: selectionOverlayCanvasRef,
    cssSize: selectionOverlayCssSize,
    pixelRatio: selectionOverlayPixelRatio,
    paintFrame: paintSelectionOverlayFrame,
    schedulePaint: scheduleSelectionOverlayPaint,
    reset: resetSelectionOverlay,
    getCurrentFrame: getSelectionOverlayCurrentFrame,
  } = useCaptureSelectionOverlay({
    status,
    selectionBounds,
    selection,
    viewportBounds,
    cursorPointRef,
    draftSelectionRef,
    hoverSelectionRef,
  });

  const syncHoverSelection = useCallback(
    (nextHoverSelection: LogicalRect | null) => {
      if (areRectsEqual(hoverSelectionRef.current, nextHoverSelection)) return;

      syncWorkspaceHoverSelection(nextHoverSelection);
      scheduleSelectionOverlayPaint(null, nextHoverSelection, null);
    },
    [hoverSelectionRef, scheduleSelectionOverlayPaint, syncWorkspaceHoverSelection],
  );

  useEffect(() => {
    if (!session || !selectionBounds) return;

    const canPoll = () =>
      shouldPollCaptureHoverSelection({
        status,
        hasSession: Boolean(session),
        hasSelectionBounds: Boolean(selectionBounds),
        hasActiveStartPoint: Boolean(startPointRef.current ?? startPoint),
        hasEditGesture: Boolean(editGesture),
      });

    return startCaptureHoverSelectionPolling({
      sessionId: session.id,
      candidates: captureCandidates,
      shouldTrackMagnifierCursor,
      intervalMs: CAPTURE_HOVER_POLL_INTERVAL_MS,
      canPoll,
      getCursorPosition: currentCaptureCursorPosition,
      setCursorPointRef: (point) => {
        cursorPointRef.current = point;
      },
      setCursorPoint,
      scheduleSelectionOverlayPaint,
      syncHoverSelection,
      setTimeout: window.setTimeout,
      clearTimeout: window.clearTimeout,
    });
  }, [
    captureCandidates,
    editGesture,
    selectionBounds,
    session,
    scheduleSelectionOverlayPaint,
    shouldTrackMagnifierCursor,
    startPoint,
    status,
    syncHoverSelection,
  ]);

  const resetCaptureImageReadiness = useCallback(() => {
    captureSnapshotHydrationRef.current = null;
    setHydratedCaptureSessionId(null);
  }, []);

  const commitTextDraftToHistory = useCallback(() => {
    const commitResult = commitCaptureEditorTextDraft({
      annotationHistory,
      selectedAnnotationIndex,
      textDraft,
      annotationStyle,
      textDraftAnnotationIndex,
    });

    setTextDraft(commitResult.textDraft);
    setTextDraftAnnotationIndex(commitResult.textDraftAnnotationIndex);
    if (commitResult.selectedAnnotationIndex !== selectedAnnotationIndex) {
      setSelectedAnnotationIndex(commitResult.selectedAnnotationIndex);
    }
    if (commitResult.annotationHistory !== annotationHistory) {
      setAnnotationHistory(commitResult.annotationHistory);
    }

    return commitResult.annotationHistory;
  }, [
    annotationHistory,
    annotationStyle,
    selectedAnnotationIndex,
    textDraft,
    textDraftAnnotationIndex,
  ]);
  const commitTextDraftToHistoryRef = useRef(commitTextDraftToHistory);
  commitTextDraftToHistoryRef.current = commitTextDraftToHistory;

  const markCaptureFrontendPerf = useCallback(
    (event: string, sessionId?: string | null) => {
      const perf = captureFrontendPerfRef.current;
      if (!perf) return;

      void logCaptureFrontendPerf({
        event,
        mode: perf.mode,
        sessionId: sessionId ?? perf.sessionId,
        elapsedMs: performance.now() - perf.startMs,
      }).catch(() => undefined);
    },
    [],
  );

  const captureHostActions = useMemo(
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
        commitTextDraftToHistory: () => commitTextDraftToHistoryRef.current(),
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
          createCaptureSession,
          getCaptureSession,
          refreshCaptureSession,
          currentCaptureCursorPosition,
          hydrateCaptureSessionSnapshots,
          cancelCaptureSession,
        },
      }),
    [
      markCaptureFrontendPerf,
      onInactive,
      resetCaptureImageReadiness,
      resetSelectionOverlay,
      screenshotSavePath,
      workspaceHostAdapter,
    ],
  );

  const ensureCaptureSnapshotsHydrated = useCallback(
    (sessionId: string) =>
      captureHostActions.ensureCaptureSnapshotsHydrated(sessionId),
    [captureHostActions],
  );

  const startSession = useCallback(
    async (nextMode: CaptureMode, sessionId?: string) => {
      await captureHostActions.startSession(nextMode, sessionId);
    },
    [captureHostActions],
  );

  const cancelSession = useCallback(async () => {
    await captureHostActions.cancelSession();
  }, [captureHostActions]);

  const renderSelectionPreview = useCallback(
    async (
      rect: LogicalRect,
      nextAnnotations?: AnnotationCommand[],
      includeCursor?: boolean,
    ) => {
      await captureHostActions.renderSelectionPreview(
        rect,
        nextAnnotations,
        includeCursor,
      );
    },
    [captureHostActions],
  );

  const completePreviewSelection = useCallback(async (
    action: PreviewCaptureCompletionAction,
    options: {
      commitTextDraft?: boolean;
      guardCompletion?: boolean;
    } = {},
  ) => {
    await captureHostActions.completePreviewSelection(action, options);
  }, [captureHostActions]);

  const copySelection = useCallback(
    () => completePreviewSelection('copy', { guardCompletion: true }),
    [completePreviewSelection],
  );

  const completeCandidateSelection = useCallback(async (
    rect: LogicalRect,
    action: HoverSelectionCompletionAction,
  ) => {
    await captureHostActions.completeCandidateSelection(rect, action);
  }, [captureHostActions]);

  const handleNativeCopyRequest = useCallback(() => {
    const currentState = getCurrentCaptureWorkspaceState();
    if (currentState.status === 'preview') {
      void copySelection();
      return;
    }

    const activeStartPoint = startPointRef.current ?? currentState.startPoint;
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
    getCurrentCaptureWorkspaceState,
    hoverSelectionRef,
    startPointRef,
  ]);

  const copyCurrentColor = useCallback(async () => {
    if (!cursorColor) return;

    try {
      await writeClipboardText(
        colorSampleToClipboardText(cursorColor, colorSampleFormat),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, [colorSampleFormat, cursorColor]);

  const saveSelection = useCallback(
    () => completePreviewSelection('save'),
    [completePreviewSelection],
  );

  const quickSaveSelection = useCallback(
    () => completePreviewSelection('quick-save'),
    [completePreviewSelection],
  );

  const runOcrSelection = useCallback(
    () => completePreviewSelection('ocr', { commitTextDraft: false }),
    [completePreviewSelection],
  );

  const pinSelection = useCallback(
    () => completePreviewSelection('pin'),
    [completePreviewSelection],
  );

  const refreshSession = useCallback(async () => {
    await captureHostActions.refreshSession();
  }, [captureHostActions]);

  const undoAnnotation = useCallback(() => {
    if (!selection || !canUndoAnnotation) return;

    const nextHistory = undoAnnotationHistory(annotationHistory);
    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setAnnotationHistory(nextHistory);
    void renderSelectionPreview(selection, nextHistory.annotations);
  }, [annotationHistory, canUndoAnnotation, renderSelectionPreview, selection]);

  const redoAnnotation = useCallback(() => {
    if (!selection || !canRedoAnnotation) return;

    const nextHistory = redoAnnotationHistory(annotationHistory);
    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setAnnotationHistory(nextHistory);
    void renderSelectionPreview(selection, nextHistory.annotations);
  }, [annotationHistory, canRedoAnnotation, renderSelectionPreview, selection]);

  const undoPolylineGesturePoint = useCallback(() => {
    if (!annotationGesture || annotationGesture.tool !== 'polyline' || !selection) {
      return false;
    }

    const nextDraft = undoPolylineCaptureGesture({
      gesture: annotationGesture,
      selection,
      cursorPoint,
      annotationStyle,
    });
    if (!nextDraft) {
      setAnnotationGesture(null);
      setDraftAnnotation(null);
      return true;
    }

    setAnnotationGesture(nextDraft.gesture);
    setDraftAnnotation(nextDraft.draftAnnotation);
    return true;
  }, [annotationGesture, annotationStyle, cursorPoint, selection]);

  const clearAnnotations = useCallback(() => {
    if (!selection) return;

    const nextHistory = clearAnnotationHistory(annotationHistory);
    if (nextHistory === annotationHistory) return;

    setActiveAnnotationTool(null);
    setAnnotationGesture(null);
    setDraftAnnotation(null);
    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setTextDraft(null);
    setTextDraftAnnotationIndex(null);
    setAnnotationHistory(nextHistory);
    void renderSelectionPreview(selection, nextHistory.annotations);
  }, [annotationHistory, renderSelectionPreview, selection]);

  const deleteSelectedAnnotation = useCallback(() => {
    if (!selection || selectedAnnotationIndex === null) return;

    const nextHistory = removeAnnotationFromHistory(
      annotationHistory,
      selectedAnnotationIndex,
    );
    if (nextHistory === annotationHistory) return;

    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setAnnotationHistory(nextHistory);
    void renderSelectionPreview(selection, nextHistory.annotations);
  }, [
    annotationHistory,
    renderSelectionPreview,
    selectedAnnotationIndex,
    selection,
  ]);

  const applySelectedAnnotationStyle = useCallback(
    (nextStyle: AnnotationStyle, nextTextFontSize: number) => {
      setAnnotationStyle(nextStyle);
      setTextFontSize(nextTextFontSize);

      if (!selection) return;

      const nextHistory = applyStyleToSelectedAnnotationHistory({
        annotationHistory,
        annotations,
        selectedAnnotationIndex,
        textDraftActive: textDraft !== null,
        nextStyle,
        nextTextFontSize,
      });
      if (nextHistory === annotationHistory) return;

      setAnnotationHistory(nextHistory);
      void renderSelectionPreview(selection, nextHistory.annotations);
    },
    [
      annotationHistory,
      annotations,
      renderSelectionPreview,
      selectedAnnotationIndex,
      selection,
      textDraft,
    ],
  );

  const adjustAnnotationSize = useCallback(
    (direction: AnnotationSizeDirection) => {
      if (textDraft) return;

      const nextToolbarState = planCaptureAnnotationSizeAdjustment({
        annotationStyle,
        textFontSize,
        direction,
        isTextSizingActive,
      });
      applySelectedAnnotationStyle(
        nextToolbarState.annotationStyle,
        nextToolbarState.textFontSize,
      );
    },
    [
      annotationStyle,
      applySelectedAnnotationStyle,
      isTextSizingActive,
      textDraft,
      textFontSize,
    ],
  );

  const selectAnnotationColor = useCallback(
    (color: AnnotationColor) => {
      if (textDraft) return;

      const nextToolbarState = planCaptureAnnotationColorSelection({
        annotationStyle,
        textFontSize,
        color,
      });
      applySelectedAnnotationStyle(
        nextToolbarState.annotationStyle,
        nextToolbarState.textFontSize,
      );
    },
    [
      annotationStyle,
      applySelectedAnnotationStyle,
      textDraft,
      textFontSize,
    ],
  );

  const toggleAnnotationFill = useCallback(() => {
    if (textDraft || !isFillModeActive) return;

    const nextToolbarState = planCaptureAnnotationFillToggle({
      annotationStyle,
      textFontSize,
    });
    applySelectedAnnotationStyle(
      nextToolbarState.annotationStyle,
      nextToolbarState.textFontSize,
    );
  }, [
    annotationStyle,
    applySelectedAnnotationStyle,
    isFillModeActive,
    mode,
    scheduleSelectionOverlayPaint,
    textDraft,
    textFontSize,
  ]);

  const commitTextDraft = useCallback(() => {
    const nextHistory = commitTextDraftToHistory();
    if (selection && nextHistory !== annotationHistory) {
      void renderSelectionPreview(selection, nextHistory.annotations);
    }
  }, [
    annotationHistory,
    commitTextDraftToHistory,
    renderSelectionPreview,
    selection,
  ]);

  const updateTextDraftText = useCallback((text: string) => {
    setTextDraft((draft) =>
      draft ? updateTextAnnotationDraft(draft, text) : draft,
    );
  }, []);

  const discardTextDraft = useCallback(() => {
    setTextDraft(null);
    setTextDraftAnnotationIndex(null);
    if (textDraftAnnotationIndex !== null && selection) {
      void renderSelectionPreview(selection, annotations);
    }
  }, [
    annotations,
    renderSelectionPreview,
    selection,
    textDraftAnnotationIndex,
  ]);

  const commitAnnotationGestureAtPoint = useCallback(
    (localPoint: Point, constrainGesture: boolean) => {
      if (!annotationGesture || !selection) return false;

      const commitResult = completeCaptureEditorGesture({
        annotationHistory,
        selectedAnnotationIndex,
        annotationGesture,
        localPoint,
        annotationStyle,
        constrainGesture,
      });
      if (!commitResult) return false;

      setAnnotationGesture(commitResult.annotationGesture);
      setDraftAnnotation(commitResult.draftAnnotation);
      if (commitResult.selectedAnnotationIndex !== selectedAnnotationIndex) {
        setSelectedAnnotationIndex(commitResult.selectedAnnotationIndex);
      }
      if (commitResult.annotationHistory !== annotationHistory) {
        setAnnotationHistory(commitResult.annotationHistory);
        void renderSelectionPreview(selection, commitResult.annotationHistory.annotations);
      }
      return true;
    },
    [
      annotationGesture,
      annotationHistory,
      annotationStyle,
      renderSelectionPreview,
      selectedAnnotationIndex,
      selection,
    ],
  );

  const dismissCaptureLayer = useCallback(() => {
    const dismissAction = getCaptureEditorDismissAction({
      hasTextDraft: textDraft !== null,
      hasAnnotationMoveGesture: annotationMoveGesture !== null,
      hasDraftSelectionMoveGesture: draftSelectionMoveGesture !== null,
      hasSelectedAnnotation: selectedAnnotationIndex !== null,
      hasActiveAnnotationTool: activeAnnotationTool !== null,
      hasAnnotationGesture: annotationGesture !== null,
    });

    if (dismissAction === 'clear-text-draft') {
      setTextDraft(null);
      setTextDraftAnnotationIndex(null);
    } else if (dismissAction === 'revert-annotation-move') {
      setAnnotationMoveGesture(null);
      setDraftAnnotation(null);
      if (selection) {
        void renderSelectionPreview(selection, annotations);
      }
    } else if (dismissAction === 'clear-draft-selection-move') {
      setDraftSelectionMoveGesture(null);
    } else if (dismissAction === 'clear-selected-annotation') {
      setSelectedAnnotationIndex(null);
    } else if (dismissAction === 'clear-active-annotation-tool') {
      setActiveAnnotationTool(null);
      setAnnotationGesture(null);
      setDraftAnnotation(null);
    } else {
      void cancelSession();
    }
  }, [
    activeAnnotationTool,
    annotationGesture,
    annotationMoveGesture,
    annotations,
    cancelSession,
    draftSelectionMoveGesture,
    renderSelectionPreview,
    selectedAnnotationIndex,
    selection,
    textDraft,
  ]);

  const resetPreviewSelection = useCallback(() => {
    resetPreview();
    resetSelectionOverlay();
  }, [resetPreview, resetSelectionOverlay]);

  const completeManualSelection = useCallback((rect: LogicalRect) => {
    void captureHostActions.completeManualSelection(rect, mode);
  }, [captureHostActions, mode]);

  const selectFullCaptureArea = useCallback(() => {
    if (!session || !selectionBounds) return;

    const currentPoint =
      cursorPointRef.current ??
      cursorPoint ??
      session.captured_cursor?.logical_position ??
      null;
    completeManualSelection(getCurrentMonitorBounds(session.monitors, currentPoint));
  }, [completeManualSelection, cursorPoint, selectionBounds, session]);

  const restoreLastSelection = useCallback(() => {
    if (!selectionBounds) return;

    restoreLastSuccessfulCaptureSelection({
      storage: window.localStorage,
      selectionBounds,
      minSelectionSize: MIN_SELECTION_SIZE,
      completeSelection: completeManualSelection,
    });
  }, [completeManualSelection, selectionBounds]);

  const restoreSelectionFromHistory = useCallback(
    (step: SelectionHistoryStep) => {
      if (!selectionBounds) return;

      restoreSelectionFromHostHistory({
        storage: window.localStorage,
        currentSelection: selection,
        step,
        selectionBounds,
        minSelectionSize: MIN_SELECTION_SIZE,
        completeSelection: completeManualSelection,
      });
    },
    [completeManualSelection, selection, selectionBounds],
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
    if (!session || !areCaptureImagesReady || !perf || perf.hasLoggedImagesReady) {
      return;
    }
    if (perf.sessionId !== session.id) return;

    perf.hasLoggedImagesReady = true;
    markCaptureFrontendPerf('images_ready', session.id);
  }, [areCaptureImagesReady, markCaptureFrontendPerf, session]);

  const markCaptureHostWindowRevealed = useCallback(
    (sessionId: string) => {
      markCaptureFrontendPerf('revealed', sessionId);
    },
    [markCaptureFrontendPerf],
  );

  const handleCaptureHostRevealError = useCallback((err: unknown) => {
    setError(err instanceof Error ? err.message : String(err));
    setStatus('error');
  }, []);

  const hostWindowReveal = useMemo(
    () => ({
      status,
      sessionId: session?.id ?? null,
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
      session?.id,
      status,
    ],
  );

  const handleCaptureHotkeyLaunch = useCallback((launch: CaptureLaunch) => {
    void startSession(launch.mode, launch.sessionId);
  }, [startSession]);

  const hostSubscriptions = useMemo(
    () => ({
      isActive,
      onLaunch: handleCaptureHotkeyLaunch,
      onCancel: cancelSession,
      onCopy: handleNativeCopyRequest,
    }),
    [
      cancelSession,
      handleCaptureHotkeyLaunch,
      handleNativeCopyRequest,
      isActive,
    ],
  );

  useCaptureMagnifierPixelSource({
    session,
    hasHydratedPixelSource,
    isMagnifierRequested,
    isMagnifierShown,
    cursorMonitor,
    cursorInMonitorPoint,
    setCursorColor,
    ensureCaptureSnapshotsHydrated,
  });

  const captureWorkspaceKeyboardRefs =
    useMemo<CaptureWorkspaceKeyboardRefs>(
      () => ({
        startPointRef,
        cursorPointRef,
        draftSelectionRef,
        hoverSelectionRef,
        keyboardDraftCursorPointRef,
        keyboardEditCursorPointRef,
      }),
      [
        cursorPointRef,
        draftSelectionRef,
        hoverSelectionRef,
        keyboardDraftCursorPointRef,
        keyboardEditCursorPointRef,
        startPointRef,
      ],
    );

  const captureWorkspacePointerRefs: CaptureWorkspacePointerRefs =
    captureWorkspaceKeyboardRefs;

  const captureWorkspaceKeyboardDerived =
    useMemo<CaptureWorkspaceKeyboardDerivedState>(
      () => ({
        annotations,
        captureCandidates,
        selectionBounds,
        hasAnnotationEditingContext,
        isAnnotationToolbarVisible,
        isMagnifierShown,
        isFillModeActive,
        cursorColor,
      }),
      [
        annotations,
        captureCandidates,
        cursorColor,
        hasAnnotationEditingContext,
        isAnnotationToolbarVisible,
        isFillModeActive,
        isMagnifierShown,
        selectionBounds,
      ],
    );

  const captureWorkspacePointerDerived =
    useMemo<CaptureWorkspacePointerDerivedState>(
      () => ({
        annotations,
        captureCandidates,
        selectionBounds,
        snapTargetRects,
        hasAnnotationEditingContext,
        shouldTrackMagnifierCursor,
      }),
      [
        annotations,
        captureCandidates,
        hasAnnotationEditingContext,
        selectionBounds,
        shouldTrackMagnifierCursor,
        snapTargetRects,
      ],
    );

  const captureWorkspaceKeyboardActions =
    useMemo<CaptureWorkspaceKeyboardActions>(
      () => ({
        dismissCaptureLayer,
        refreshSession,
        setIncludeCapturedCursor,
        clearPreviewImage: () => setPreviewImageBase64(null),
        renderSelectionPreview,
        setIsMagnifierRequested,
        clearAnnotations,
        undoPolylineGesturePoint,
        undoAnnotation,
        redoAnnotation,
        deleteSelectedAnnotation,
        copyCurrentColor,
        setColorSampleFormat,
        restoreSelectionFromHistory,
        restoreLastSelection,
        setCursorPoint,
        setSelection,
        scheduleSelectionOverlayPaint,
        setPreviewImageBase64,
        setRenderingOutput,
        setEditGesture,
        syncHoverSelection,
        selectFullCaptureArea,
        completeCandidateSelection,
        setIsAnnotationToolbarVisible,
        completePreviewSelection,
        adjustAnnotationSize,
        toggleAnnotationFill,
        setActiveAnnotationTool,
        setSelectedAnnotationIndex,
        setAnnotationGesture,
        setAnnotationMoveGesture,
        setDraftAnnotation,
        selectAnnotationColor,
        toggleAnnotationTool,
        setDraftSelectionMoveGesture,
        setAnnotationHistory,
      }),
      [
        adjustAnnotationSize,
        clearAnnotations,
        completeCandidateSelection,
        completePreviewSelection,
        copyCurrentColor,
        deleteSelectedAnnotation,
        dismissCaptureLayer,
        redoAnnotation,
        refreshSession,
        renderSelectionPreview,
        restoreLastSelection,
        restoreSelectionFromHistory,
        scheduleSelectionOverlayPaint,
        selectAnnotationColor,
        selectFullCaptureArea,
        setActiveAnnotationTool,
        setAnnotationGesture,
        setAnnotationHistory,
        setAnnotationMoveGesture,
        setColorSampleFormat,
        setCursorPoint,
        setDraftAnnotation,
        setDraftSelectionMoveGesture,
        setEditGesture,
        setIncludeCapturedCursor,
        setIsAnnotationToolbarVisible,
        setIsMagnifierRequested,
        setPreviewImageBase64,
        setRenderingOutput,
        setSelectedAnnotationIndex,
        setSelection,
        toggleAnnotationFill,
        toggleAnnotationTool,
        undoAnnotation,
        undoPolylineGesturePoint,
      ],
    );

  const captureWorkspacePointerActions =
    useMemo<CaptureWorkspacePointerActions>(
      () => ({
        commitTextDraft,
        commitAnnotationGestureAtPoint,
        dismissCaptureLayer,
        resetPreviewSelection,
        cancelSession,
        setCursorPoint,
        setStartPointWithRef,
        setSelection,
        setHoverSelection,
        scheduleSelectionOverlayPaint,
        setPreviewImageBase64,
        setRenderingOutput,
        setStatus,
        setActiveAnnotationTool,
        setAnnotationGesture,
        setDraftAnnotation,
        setSelectedAnnotationIndex,
        setAnnotationMoveGesture,
        setDraftSelectionMoveGesture,
        setTextDraft,
        setTextDraftAnnotationIndex,
        setAnnotationHistory,
        syncHoverSelection,
        renderSelectionPreview,
        completeManualSelection,
        pinSelection,
        setEditGesture,
        setAnnotationStyle,
        setTextFontSize,
        copySelection,
        adjustAnnotationSize,
      }),
      [
        adjustAnnotationSize,
        cancelSession,
        commitAnnotationGestureAtPoint,
        commitTextDraft,
        completeManualSelection,
        copySelection,
        dismissCaptureLayer,
        pinSelection,
        renderSelectionPreview,
        resetPreviewSelection,
        scheduleSelectionOverlayPaint,
        setActiveAnnotationTool,
        setAnnotationGesture,
        setAnnotationHistory,
        setAnnotationMoveGesture,
        setAnnotationStyle,
        setCursorPoint,
        setDraftAnnotation,
        setDraftSelectionMoveGesture,
        setEditGesture,
        setHoverSelection,
        setPreviewImageBase64,
        setRenderingOutput,
        setSelectedAnnotationIndex,
        setSelection,
        setStartPointWithRef,
        setStatus,
        setTextDraft,
        setTextDraftAnnotationIndex,
        setTextFontSize,
        syncHoverSelection,
      ],
    );

  const captureWorkspacePointerContext =
    useMemo<CaptureWorkspacePointerContext>(
      () => ({
        state: captureWorkspaceState,
        refs: captureWorkspacePointerRefs,
        derived: captureWorkspacePointerDerived,
        actions: captureWorkspacePointerActions,
      }),
      [
        captureWorkspacePointerActions,
        captureWorkspacePointerDerived,
        captureWorkspacePointerRefs,
        captureWorkspaceState,
      ],
    );

  const handleCaptureKeyboardKeyDown = useCallback((event: KeyboardEvent) => {
    handleCaptureWorkspaceKeyDown(event, {
      state: captureWorkspaceState,
      refs: captureWorkspaceKeyboardRefs,
      derived: captureWorkspaceKeyboardDerived,
      actions: captureWorkspaceKeyboardActions,
    });
  }, [
    captureWorkspaceKeyboardActions,
    captureWorkspaceKeyboardDerived,
    captureWorkspaceKeyboardRefs,
    captureWorkspaceState,
  ]);

  const releaseCaptureMagnifierRequest = useCallback(() => {
    setIsMagnifierRequested(false);
  }, []);

  const finishDraftSelectionMoveFromKeyboard = useCallback(() => {
    setDraftSelectionMoveGesture(null);
  }, []);

  const keyboardHostEvents = useMemo(
    () => ({
      isActive,
      status,
      isRenderingOutputRef,
      hasDraftSelectionMoveGesture: draftSelectionMoveGesture !== null,
      onKeyDown: handleCaptureKeyboardKeyDown,
      onReleaseMagnifierRequest: releaseCaptureMagnifierRequest,
      onFinishDraftSelectionMove: finishDraftSelectionMoveFromKeyboard,
      onCancelSession: cancelSession,
    }),
    [
      cancelSession,
      draftSelectionMoveGesture,
      finishDraftSelectionMoveFromKeyboard,
      handleCaptureKeyboardKeyDown,
      isActive,
      releaseCaptureMagnifierRequest,
      status,
    ],
  );

  useEffect(() => {
    if (!textDraft) return;

    requestAnimationFrame(() => {
      textDraftInputRef.current?.focus();
    });
  }, [textDraft]);

  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    handleCaptureWorkspacePointerDown(
      event,
      captureWorkspacePointerContext,
    );
  };

  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    handleCaptureWorkspacePointerMove(
      event,
      captureWorkspacePointerContext,
    );
  };

  const handlePointerUp = (event: PointerEvent<HTMLDivElement>) => {
    handleCaptureWorkspacePointerUp(event, captureWorkspacePointerContext);
  };

  const startMoveGesture = (event: PointerEvent<HTMLDivElement>) => {
    handleCaptureWorkspacePreviewPointerDown(
      event,
      captureWorkspacePointerContext,
    );
  };

  function toggleAnnotationTool(nextTool: AnnotationTool) {
    const nextHistory = commitTextDraftToHistory();
    if (selection && nextHistory !== annotationHistory) {
      void renderSelectionPreview(selection, nextHistory.annotations);
    }

    const toolActivation = planCaptureAnnotationToolActivation({
      currentTool: activeAnnotationTool,
      nextTool,
      selectedAnnotationIndex,
      clearSelectedAnnotation: false,
      toggle: true,
    });
    setActiveAnnotationTool(toolActivation.activeAnnotationTool);
    setSelectedAnnotationIndex(toolActivation.selectedAnnotationIndex);
    setAnnotationGesture(toolActivation.annotationGesture);
    setAnnotationMoveGesture(toolActivation.annotationMoveGesture);
    setDraftAnnotation(toolActivation.draftAnnotation);
  }

  const startResizeGesture = (
    handle: SelectionHandle,
    event: PointerEvent<HTMLButtonElement>,
  ) => {
    handleCaptureWorkspaceResizePointerDown(
      handle,
      event,
      captureWorkspacePointerContext,
    );
  };

  const handleWheel = (event: WheelEvent<HTMLDivElement>) => {
    handleCaptureWorkspaceWheel(event, captureWorkspacePointerContext);
  };

  const selectMoveTool = useCallback(() => {
    setActiveAnnotationTool(null);
  }, [setActiveAnnotationTool]);

  const updateTextDraftFontSize = useCallback((fontSize: number) => {
    setTextFontSize(fontSize);
    setTextDraft((draft) => (draft ? { ...draft, fontSize } : draft));
  }, [setTextDraft, setTextFontSize]);

  const magnifierSelection =
    selection ?? draftSelectionRef.current ?? hoverSelectionRef.current ?? hoverSelection;

  return {
    state: captureWorkspaceState,
    derived,
    hostWindowReveal,
    hostSubscriptions,
    keyboardHostEvents,
    viewProps: {
      isActive,
      status,
      viewportBounds,
      error,
      selection,
      selectionViewportRect,
      previewImageBase64,
      draftAnnotation,
      textDraft,
      textDraftInputRef,
      annotationStyle,
      selectedAnnotationBounds,
      activeAnnotationTool,
      toolbarPosition,
      toolbarWidth: TOOLBAR_SIZE.width,
      isAnnotationToolbarVisible,
      textFontSize,
      isTextSizingActive,
      isFillModeActive,
      isRenderingOutput,
      selectionOverlayCanvasRef,
      selectionOverlayCssSize,
      selectionOverlayPixelRatio,
      isMagnifierShown,
      cursorMonitor,
      cursorViewportPoint,
      cursorInMonitorPoint,
      magnifierSelection,
      cursorColor,
      colorSampleFormat,
      onRootPointerDown: handlePointerDown,
      onRootPointerMove: handlePointerMove,
      onRootPointerUp: handlePointerUp,
      onRootWheel: handleWheel,
      onPreviewPointerDown: startMoveGesture,
      onResizeHandlePointerDown: startResizeGesture,
      onCommitTextDraft: commitTextDraft,
      onTextDraftTextChange: updateTextDraftText,
      onDiscardTextDraft: discardTextDraft,
      onSelectMove: selectMoveTool,
      onToggleAnnotationTool: toggleAnnotationTool,
      onApplyAnnotationStyle: applySelectedAnnotationStyle,
      onTextDraftFontSizeChange: updateTextDraftFontSize,
      onCancel: cancelSession,
      onRunOcr: runOcrSelection,
      onCopy: copySelection,
      onSave: saveSelection,
      onQuickSave: quickSaveSelection,
    },
  };
}
