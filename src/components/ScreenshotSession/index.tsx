import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSettingsConfigStore } from '../../stores/settingsConfigStore';
import { writeClipboardText } from '../../tauri/clipboard';
import { getCurrentAppWebviewWindow } from '../../tauri/window';
import {
  cancelCaptureSession,
  createCaptureSession,
  currentCaptureCursorPosition,
  getCaptureSession,
  hydrateCaptureSessionSnapshots,
  logCaptureFrontendPerf,
} from '../../tauri/captureSession';
import {
  getToolbarPosition,
  snapPointToRects,
  type SelectionHandle,
} from './selection';
import {
  colorSampleToClipboardText,
  isColorSampleCopyShortcut,
  isColorSampleFormatToggleShortcut,
} from './colorSampler';
import {
  buildCaptureCandidates,
  getBestCandidateAtPoint,
} from './captureCandidates';
import {
  clearAnnotationHistory,
  removeAnnotationFromHistory,
  redoAnnotationHistory,
  undoAnnotationHistory,
} from './annotationHistory';
import {
  annotationColorFromShortcut,
  annotationSizeDirectionFromShortcut,
  annotationToolFromShortcut,
  isAnnotationFillToggleShortcut,
  nextAnnotationToolFromCycleShortcut,
  type AnnotationColor,
  type AnnotationStyle,
  type AnnotationSizeDirection,
  type AnnotationTool,
} from './annotationStyle';
import {
  startTextAnnotationDraft,
  updateTextAnnotationDraft,
} from './textAnnotationDraft';
import {
  canToggleCapturedCursor,
  type HoverSelectionCompletionAction,
  type PreviewCaptureCompletionAction,
  getCaptureKeyboardToolbarAction,
  getCandidateCycleDirectionFromShortcut,
  getCursorNudgeDeltaFromShortcut,
  getHoverSelectionCompletionActionFromShortcut,
  getPreviewCaptureCompletionActionFromShortcut,
  getSelectionArrowActionFromShortcut,
  getSelectionHistoryStepFromShortcut,
  getUndoRedoActionFromShortcut,
  isClearAnnotationsShortcut,
  isCopyCaptureDoubleClick,
  isDeleteSelectedAnnotationShortcut,
  isFinishAnnotationGestureDoubleClick,
  isMoveDraftSelectionShortcut,
  isMagnifierShortcut,
  isRefreshCaptureShortcut,
  isSelectAllCaptureShortcut,
  isToggleCapturedCursorShortcut,
  isUndoAnnotationGesturePointShortcut,
  refreshCaptureSession,
  shouldRestoreLastSelectionFromShortcut,
} from './captureActions';
import {
  planManualSelectionCompletion,
  type CaptureRuntimeEffect,
} from './captureInteractionRuntime';
import {
  prepareCaptureSurfaceForReveal as prepareHostCaptureSurfaceForReveal,
  restoreCaptureSelectionFromHistory as restoreSelectionFromHostHistory,
  restoreLastSuccessfulCaptureSelection,
  runCaptureHostTransitionEffects,
  type CaptureHostSessionStartPerfState,
  type CaptureHostSnapshotHydration,
} from './captureHostRuntime';
import {
  createCaptureWorkspaceHostActions,
  type CaptureWorkspaceHostAdapter,
} from './captureWorkspaceHost';
import {
  useCaptureHostSubscriptions,
  useCaptureHostWindowReveal,
} from './captureHostRuntimeHooks';
import { useCaptureKeyboardHostEvents } from './captureKeyboardHostRuntimeHooks';
import {
  applyStyleToSelectedAnnotationHistory,
  commitCaptureEditorTextDraft,
  completeCaptureEditorGesture,
  getCaptureEditorDismissAction,
  getCaptureSelectedAnnotationBounds,
  planCaptureAnnotationColorSelection,
  planCaptureAnnotationToolActivation,
  planCaptureAnnotationFillToggle,
  planCaptureAnnotationErase,
  planCaptureAnnotationGestureMove,
  planCaptureSelectedAnnotationKeyboardNudge,
  planCaptureExistingAnnotationPointerDown,
  planCaptureAnnotationMove,
  planCaptureAnnotationMoveCommit,
  planCaptureAnnotationSizeAdjustment,
  planCaptureAnnotationToolStart,
  planCapturePolylineAnnotationContinue,
  planCaptureManualSelectionTransition,
  undoPolylineCaptureGesture,
} from './captureEditorRuntime';
import {
  planCaptureDraftSelectionMoveShortcutStart,
  planCaptureDraftSelectionKeyboardNudge,
  planCaptureDraftSelectionCommit,
  planCaptureDraftSelectionMove,
  planCaptureDraftSelectionPointerMove,
  planCaptureDraftSelectionStart,
  planCaptureHoverSelectionCycle,
  planCapturePreviewSelectionMoveStart,
  planCaptureSelectionArrowPreview,
  planCaptureSelectionCursorKeyboardNudge,
  planCaptureSelectionEditCommit,
  planCaptureSelectionEditKeyboardNudge,
  planCaptureSelectionEditMove,
  planCaptureSelectionResizeStart,
} from './captureSelectionRuntime';
import {
  getCapturePointerMoveAction,
  getCapturePointerUpAction,
  getCaptureSelectionLocalPoint,
  planCapturePointerWheelSizeAdjustment,
  planCapturePreviewPointerDown,
  planCaptureRootPointerDown,
  shouldSyncHoverSelectionOnPointerMove,
} from './capturePointerInteractionRuntime';
import {
  shouldPollCaptureHoverSelection,
  startCaptureHoverSelectionPolling,
} from './captureHoverPolling';
import { CaptureEditorToolbar } from './captureEditorToolbar';
import { CaptureMagnifierOverlay } from './captureMagnifierOverlay';
import {
  getCaptureEditorSelectionClassName,
  getCaptureRootCursorStyle,
  getCaptureRootClassName,
  shouldShowCaptureLoadingMask,
} from './capturePresentation';
import {
  CaptureDraftAnnotationOverlay,
  CapturePreviewImage,
  CaptureRenderingOutputBar,
  CaptureSelectedAnnotationBoundsOverlay,
  CaptureSelectionResizeHandles,
  CaptureTextDraftEditor,
  rectStyle,
} from './capturePreviewPresentation';
import {
  CaptureSelectionOverlayCanvas,
  useCaptureSelectionOverlay,
} from './captureSelectionOverlayRuntime';
import {
  getCaptureMagnifierRuntimeState,
  useCaptureMagnifierPixelSource,
} from './captureMagnifierRuntime';
import {
  resetCaptureInteractionStatePatch,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import {
  getCurrentMonitorBounds,
  getVirtualDesktopBounds,
  viewportPointToVirtualPoint,
  virtualPointToViewportPoint,
  virtualRectToViewportRect,
} from './virtualDesktop';
import { useCaptureWorkspaceState } from './useCaptureWorkspaceState';
import type {
  AnnotationCommand,
  CaptureLaunch,
  CaptureMode,
  ArrowKey,
  LogicalRect,
  Point,
} from './types';

const captureWindow = getCurrentAppWebviewWindow();

const MIN_SELECTION_SIZE = 10;
const EDGE_SNAP_THRESHOLD = 6;
const KEYBOARD_NUDGE_STEP = 1;
const KEYBOARD_FAST_NUDGE_STEP = 10;
const TOOLBAR_GAP = 14;
const TOOLBAR_SIZE = { width: 1220, height: 56 };
const CAPTURE_HOVER_POLL_INTERVAL_MS = 16;
const ARROW_KEYS: ArrowKey[] = ['ArrowUp', 'ArrowRight', 'ArrowDown', 'ArrowLeft'];

function isArrowKey(key: string): key is ArrowKey {
  return ARROW_KEYS.includes(key as ArrowKey);
}

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

interface ScreenshotSessionProps {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
}

export default function ScreenshotSession({
  initialMode,
  initialSessionId,
  onInactive,
}: ScreenshotSessionProps) {
  const screenshotSavePath = useSettingsConfigStore(
    (state) => state.screenshot?.savePath,
  );
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
    setStartPoint,
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
      getCurrentCaptureWorkspaceState,
      resetInteraction,
      resetSession,
    ],
  );

  const isActive = status !== 'idle';
  const annotations = annotationHistory.annotations;
  const selectedAnnotation =
    selectedAnnotationIndex === null ? null : annotations[selectedAnnotationIndex] ?? null;
  const hasAnnotationEditingContext =
    activeAnnotationTool !== null || selectedAnnotationIndex !== null;
  const canUndoAnnotation =
    annotationHistory.undoSnapshots !== undefined
      ? annotationHistory.undoSnapshots.length > 0
      : annotationHistory.annotations.length > 0;
  const canRedoAnnotation =
    annotationHistory.redoSnapshots !== undefined
      ? annotationHistory.redoSnapshots.length > 0
      : annotationHistory.undoneAnnotations.length > 0;
  const isTextSizingActive =
    activeAnnotationTool === 'text' ||
    Boolean(textDraft) ||
    selectedAnnotation?.type === 'text';
  const isFillModeActive =
    activeAnnotationTool === 'rectangle' ||
    activeAnnotationTool === 'ellipse' ||
    selectedAnnotation?.type === 'rectangle' ||
    selectedAnnotation?.type === 'ellipse';
  const captureCandidates = useMemo(() => {
    if (!session) return [];

    return buildCaptureCandidates(session.monitors, session.candidates);
  }, [session]);
  const areCaptureImagesReady = useMemo(() => {
    if (!session) return false;
    return hydratedCaptureSessionId === session.id;
  }, [hydratedCaptureSessionId, session]);
  const snapTargetRects = useMemo(
    () => captureCandidates.map((candidate) => candidate.rect),
    [captureCandidates],
  );
  const selectionBounds = useMemo<LogicalRect | null>(() => {
    if (!session) return null;

    return getVirtualDesktopBounds(session.monitors);
  }, [session]);
  const viewportBounds = useMemo<LogicalRect | null>(() => {
    if (!selectionBounds) return null;

    return {
      x: 0,
      y: 0,
      width: selectionBounds.width,
      height: selectionBounds.height,
    };
  }, [selectionBounds]);
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
  const selectionViewportRect = useMemo<LogicalRect | null>(() => {
    if (!selection || !selectionBounds) return null;

    return virtualRectToViewportRect(selection, selectionBounds);
  }, [selection, selectionBounds]);
  const cursorViewportPoint = useMemo<Point | null>(() => {
    if (!cursorPoint || !selectionBounds) return null;

    return virtualPointToViewportPoint(cursorPoint, selectionBounds);
  }, [cursorPoint, selectionBounds]);
  const {
    cursorInMonitorPoint,
    cursorMonitor,
    hasHydratedPixelSource,
    isMagnifierShown,
    shouldTrackMagnifierCursor,
  } = useMemo(
    () =>
      getCaptureMagnifierRuntimeState({
        session,
        status,
        cursorPoint,
        cursorViewportPoint,
        viewportBounds,
        isMagnifierRequested,
      }),
    [
      cursorPoint,
      cursorViewportPoint,
      isMagnifierRequested,
      session,
      status,
      viewportBounds,
    ],
  );
  const selectedAnnotationBounds = useMemo<LogicalRect | null>(() => {
    return getCaptureSelectedAnnotationBounds({
      annotations,
      selectedAnnotationIndex,
      annotationMoveGesture,
    });
  }, [annotationMoveGesture, annotations, selectedAnnotationIndex]);
  const toolbarPosition = useMemo(() => {
    if (!selectionViewportRect || !viewportBounds || status !== 'preview') return null;

    return getToolbarPosition(selectionViewportRect, viewportBounds, TOOLBAR_SIZE, TOOLBAR_GAP);
  }, [selectionViewportRect, status, viewportBounds]);

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

  const runCaptureRuntimeEffects = useCallback(
    async (
      effects: CaptureRuntimeEffect[],
      rect: LogicalRect,
      nextAnnotations: AnnotationCommand[] = [],
    ) => {
      await captureHostActions.runCaptureRuntimeEffects(
        effects,
        rect,
        nextAnnotations,
      );
    },
    [captureHostActions],
  );

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
    if (status === 'preview') {
      void copySelection();
      return;
    }

    const activeStartPoint = startPointRef.current ?? startPoint;
    const activeHoverSelection = hoverSelectionRef.current ?? hoverSelection;
    if (
      status === 'selecting' &&
      !textDraft &&
      activeStartPoint === null &&
      activeHoverSelection
    ) {
      void completeCandidateSelection(activeHoverSelection, 'copy');
    }
  }, [
    completeCandidateSelection,
    copySelection,
    hoverSelection,
    startPoint,
    status,
    textDraft,
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
    const completion = planManualSelectionCompletion(mode);
    const transition = planCaptureManualSelectionTransition({
      rect,
      completion,
    });

    startPointRef.current = null;
    draftSelectionRef.current = null;
    hoverSelectionRef.current = null;
    if (transition.clearOverlay) {
      resetSelectionOverlay();
    }
    setStartPoint(transition.nextState.startPoint);
    setSelection(transition.nextState.selection);
    setHoverSelection(transition.nextState.hoverSelection);
    setEditGesture(transition.nextState.editGesture);
    setActiveAnnotationTool(transition.nextState.activeAnnotationTool);
    setAnnotationGesture(transition.nextState.annotationGesture);
    setDraftAnnotation(transition.nextState.draftAnnotation);
    setSelectedAnnotationIndex(transition.nextState.selectedAnnotationIndex);
    setAnnotationMoveGesture(transition.nextState.annotationMoveGesture);
    setDraftSelectionMoveGesture(transition.nextState.draftSelectionMoveGesture);
    setTextDraft(transition.nextState.textDraft);
    setTextDraftAnnotationIndex(transition.nextState.textDraftAnnotationIndex);
    setAnnotationHistory(transition.nextState.annotationHistory);
    setIsMagnifierRequested(transition.nextState.isMagnifierRequested);
    setIsAnnotationToolbarVisible(
      transition.nextState.isAnnotationToolbarVisible,
    );
    setStatus(transition.nextState.status);

    if (transition.type === 'preview') {
      void renderSelectionPreview(
        transition.previewRender.rect,
        transition.previewRender.annotations,
      );
      return;
    }

    if (!session) return;

    void runCaptureHostTransitionEffects({
      rendering: transition.nextState.renderingOutput,
      error: transition.nextState.error,
      setRendering: setRenderingOutput,
      setError,
      runEffects: () => runCaptureRuntimeEffects(transition.effects, rect),
      onError: (err) => {
        setError(err instanceof Error ? err.message : String(err));
        setStatus('error');
      },
    });
  }, [
    mode,
    resetSelectionOverlay,
    renderSelectionPreview,
    runCaptureRuntimeEffects,
    session,
  ]);

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
    (step: ReturnType<typeof getSelectionHistoryStepFromShortcut>) => {
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

  useCaptureHostWindowReveal({
    status,
    sessionId: session?.id ?? null,
    hasCaptureImagesReady: areCaptureImagesReady,
    hasRevealedRef: hasRevealedCaptureWindowRef,
    window: captureWindow,
    prepareSurface: prepareCaptureSurfaceForReveal,
    onRevealedSession: markCaptureHostWindowRevealed,
    onError: handleCaptureHostRevealError,
  });

  const handleCaptureHotkeyLaunch = useCallback((launch: CaptureLaunch) => {
    void startSession(launch.mode, launch.sessionId);
  }, [startSession]);

  useCaptureHostSubscriptions({
    isActive,
    onLaunch: handleCaptureHotkeyLaunch,
    onCancel: cancelSession,
    onCopy: handleNativeCopyRequest,
  });

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

  const handleCaptureKeyboardKeyDown = useCallback((event: KeyboardEvent) => {
      const activeStartPoint = startPointRef.current ?? startPoint;
      const activeCursorPoint = cursorPointRef.current ?? cursorPoint;
      const activeDraftSelection = draftSelectionRef.current ?? selection;
      const activeHoverSelection = hoverSelectionRef.current ?? hoverSelection;
      const cursorNudgeDelta = getCursorNudgeDeltaFromShortcut(event);
      const candidateCycleDirection =
        getCandidateCycleDirectionFromShortcut(event);
      const hoverSelectionCompletionAction =
        getHoverSelectionCompletionActionFromShortcut(event, {
          drafting: activeStartPoint !== null,
          mode,
        });
      const selectionHistoryStep = getSelectionHistoryStepFromShortcut(event);
      const undoRedoAction = getUndoRedoActionFromShortcut(event);
      const previewCaptureCompletionAction =
        getPreviewCaptureCompletionActionFromShortcut(event);
      const toolbarAction = getCaptureKeyboardToolbarAction(
        event,
        isAnnotationToolbarVisible,
      );
      const selectionArrowAction = getSelectionArrowActionFromShortcut(event, {
        editing:
          hasAnnotationEditingContext ||
          annotationGesture !== null ||
          annotationMoveGesture !== null ||
          textDraft !== null,
      });
      const cycledAnnotationTool = nextAnnotationToolFromCycleShortcut(
        event,
        activeAnnotationTool,
      );

      if (event.key === 'Escape') {
        event.preventDefault();
        dismissCaptureLayer();
      } else if (
        (status === 'selecting' || status === 'preview') &&
        isRefreshCaptureShortcut(event)
      ) {
        event.preventDefault();
        void refreshSession();
      } else if (
        (status === 'selecting' || status === 'preview') &&
        !textDraft &&
        canToggleCapturedCursor(session) &&
        isToggleCapturedCursorShortcut(event)
      ) {
        event.preventDefault();
        const nextIncludeCursor = !includeCapturedCursor;
        setIncludeCapturedCursor(nextIncludeCursor);
        if (status === 'preview' && selection) {
          setPreviewImageBase64(null);
          void renderSelectionPreview(selection, annotations, nextIncludeCursor);
        }
      } else if (isMagnifierShortcut(event)) {
        event.preventDefault();
        setIsMagnifierRequested(true);
      } else if (
        status === 'preview' &&
        isClearAnnotationsShortcut(event)
      ) {
        event.preventDefault();
        clearAnnotations();
      } else if (
        status === 'preview' &&
        undoRedoAction === 'undo' &&
        annotationGesture?.tool === 'polyline'
      ) {
        event.preventDefault();
        undoPolylineGesturePoint();
      } else if (
        status === 'preview' &&
        undoRedoAction
      ) {
        event.preventDefault();
        if (undoRedoAction === 'undo') {
          undoAnnotation();
        } else {
          redoAnnotation();
        }
      } else if (
        status === 'preview' &&
        annotationGesture?.tool === 'polyline' &&
        isUndoAnnotationGesturePointShortcut(event)
      ) {
        event.preventDefault();
        undoPolylineGesturePoint();
      } else if (
        status === 'preview' &&
        selectedAnnotationIndex !== null &&
        isDeleteSelectedAnnotationShortcut(event)
      ) {
        event.preventDefault();
        deleteSelectedAnnotation();
      } else if (
        !textDraft &&
        isMagnifierShown &&
        cursorColor &&
        isColorSampleCopyShortcut(event)
      ) {
        event.preventDefault();
        void copyCurrentColor();
      } else if (
        !textDraft &&
        isMagnifierShown &&
        cursorColor &&
        !event.repeat &&
        isColorSampleFormatToggleShortcut(event)
      ) {
        event.preventDefault();
        setColorSampleFormat((format) => (format === 'hex' ? 'rgb' : 'hex'));
      } else if (
        (status === 'selecting' || status === 'preview') &&
        !textDraft &&
        selectionHistoryStep
      ) {
        event.preventDefault();
        restoreSelectionFromHistory(selectionHistoryStep);
      } else if (
        shouldRestoreLastSelectionFromShortcut(event, {
          status,
          editing:
            hasAnnotationEditingContext ||
            annotationGesture !== null ||
            annotationMoveGesture !== null ||
            textDraft !== null,
        })
      ) {
        event.preventDefault();
        restoreLastSelection();
      } else if (
        status === 'selecting' &&
        !textDraft &&
        activeStartPoint &&
        activeDraftSelection &&
        activeCursorPoint &&
        selectionBounds &&
        cursorNudgeDelta
      ) {
        event.preventDefault();
        const draftNudge = planCaptureDraftSelectionKeyboardNudge({
          anchorPoint: activeStartPoint,
          cursorPoint: activeCursorPoint,
          delta: cursorNudgeDelta,
          selectionBounds,
        });
        keyboardDraftCursorPointRef.current = draftNudge.keyboardDraftCursorPoint;
        cursorPointRef.current = draftNudge.cursorPoint;
        draftSelectionRef.current = draftNudge.selection;
        setCursorPoint(draftNudge.cursorPoint);
        setSelection(draftNudge.selection);
        scheduleSelectionOverlayPaint(draftNudge.selection, null);
        setPreviewImageBase64(draftNudge.previewImageBase64);
        setRenderingOutput(draftNudge.renderingOutput);
      } else if (
        status === 'preview' &&
        !textDraft &&
        editGesture &&
        selection &&
        cursorPoint &&
        selectionBounds &&
        cursorNudgeDelta
      ) {
        event.preventDefault();
        const editNudge = planCaptureSelectionEditKeyboardNudge({
          gesture: editGesture,
          selection,
          cursorPoint,
          delta: cursorNudgeDelta,
          selectionBounds,
          minSelectionSize: MIN_SELECTION_SIZE,
          preserveAspect: event.shiftKey,
        });
        keyboardEditCursorPointRef.current = editNudge.keyboardEditCursorPoint;
        setCursorPoint(editNudge.cursorPoint);
        setSelection(editNudge.selection);
        setEditGesture(editNudge.editGesture);
        setPreviewImageBase64(editNudge.previewImageBase64);
        setRenderingOutput(editNudge.renderingOutput);
      } else if (
        status === 'selecting' &&
        !textDraft &&
        activeCursorPoint &&
        selectionBounds &&
        cursorNudgeDelta
      ) {
        event.preventDefault();
        const cursorNudge = planCaptureSelectionCursorKeyboardNudge({
          cursorPoint: activeCursorPoint,
          delta: cursorNudgeDelta,
          selectionBounds,
        });
        cursorPointRef.current = cursorNudge.cursorPoint;
        setCursorPoint(cursorNudge.cursorPoint);
      } else if (
        status === 'selecting' &&
        !textDraft &&
        activeCursorPoint &&
        candidateCycleDirection
      ) {
        event.preventDefault();
        const hoverCycle = planCaptureHoverSelectionCycle({
          captureCandidates,
          cursorPoint: activeCursorPoint,
          hoverSelection: activeHoverSelection,
          direction: candidateCycleDirection,
        });
        syncHoverSelection(hoverCycle.hoverSelection);
      } else if (
        (status === 'selecting' || status === 'preview') &&
        !textDraft &&
        isSelectAllCaptureShortcut(event)
      ) {
        event.preventDefault();
        selectFullCaptureArea();
      } else if (
        status === 'selecting' &&
        activeHoverSelection &&
        hoverSelectionCompletionAction
      ) {
        event.preventDefault();
        void completeCandidateSelection(
          activeHoverSelection,
          hoverSelectionCompletionAction,
        );
      } else if (
        status === 'preview' &&
        !textDraft &&
        toolbarAction === 'toggle'
      ) {
        event.preventDefault();
        setIsAnnotationToolbarVisible((visible) => !visible);
      } else if (
        status === 'preview' &&
        previewCaptureCompletionAction
      ) {
        event.preventDefault();
        void completePreviewSelection(previewCaptureCompletionAction, {
          guardCompletion: previewCaptureCompletionAction === 'copy',
        });
      } else if (
        status === 'preview' &&
        !textDraft &&
        (event.key === '[' ||
          event.key === ']' ||
          (hasAnnotationEditingContext &&
            (event.key === '1' || event.key === '2')))
      ) {
        const sizeDirection = annotationSizeDirectionFromShortcut(event, {
          editing: hasAnnotationEditingContext,
        });
        if (sizeDirection) {
          event.preventDefault();
          adjustAnnotationSize(sizeDirection);
        }
      } else if (
        status === 'preview' &&
        !textDraft &&
        isFillModeActive &&
        !annotationGesture &&
        !annotationMoveGesture &&
        isAnnotationFillToggleShortcut(event)
      ) {
        event.preventDefault();
        toggleAnnotationFill();
      } else if (
        status === 'preview' &&
        !textDraft &&
        cycledAnnotationTool &&
        !annotationGesture &&
        !annotationMoveGesture
      ) {
        event.preventDefault();
        const toolActivation = planCaptureAnnotationToolActivation({
          currentTool: activeAnnotationTool,
          nextTool: cycledAnnotationTool,
          selectedAnnotationIndex,
          clearSelectedAnnotation: true,
          toggle: false,
        });
        setActiveAnnotationTool(toolActivation.activeAnnotationTool);
        setSelectedAnnotationIndex(toolActivation.selectedAnnotationIndex);
        setAnnotationGesture(toolActivation.annotationGesture);
        setAnnotationMoveGesture(toolActivation.annotationMoveGesture);
        setDraftAnnotation(toolActivation.draftAnnotation);
      } else if (
        status === 'preview' &&
        !textDraft &&
        !annotationGesture &&
        !annotationMoveGesture
      ) {
        const shortcutColor = annotationColorFromShortcut(event);
        if (shortcutColor) {
          event.preventDefault();
          selectAnnotationColor(shortcutColor);
        } else {
          const shortcutTool = annotationToolFromShortcut(event);
          if (shortcutTool) {
            event.preventDefault();
            toggleAnnotationTool(shortcutTool);
          }
        }
      } else if (
        isMoveDraftSelectionShortcut(event) &&
        status === 'selecting' &&
        activeStartPoint &&
        activeDraftSelection &&
        activeCursorPoint &&
        !draftSelectionMoveGesture
      ) {
        event.preventDefault();
        const draftSelectionMoveStart = planCaptureDraftSelectionMoveShortcutStart({
          cursorPoint: activeCursorPoint,
          selection: activeDraftSelection,
          anchorPoint: activeStartPoint,
        });
        setDraftSelectionMoveGesture(
          draftSelectionMoveStart.draftSelectionMoveGesture,
        );
      } else if (
        status === 'preview' &&
        !textDraft &&
        !annotationGesture &&
        !annotationMoveGesture &&
        selectedAnnotationIndex !== null &&
        isArrowKey(event.key)
      ) {
        event.preventDefault();
        const annotationNudge = planCaptureSelectedAnnotationKeyboardNudge({
          annotationHistory,
          annotations,
          selectedAnnotationIndex,
          key: event.key,
          fast: event.shiftKey,
          keyboardNudgeStep: KEYBOARD_NUDGE_STEP,
          keyboardFastNudgeStep: KEYBOARD_FAST_NUDGE_STEP,
        });
        if (annotationNudge.previewAnnotations && selection) {
          setAnnotationHistory(annotationNudge.annotationHistory);
          void renderSelectionPreview(selection, annotationNudge.previewAnnotations);
        }
      } else if (
        status === 'preview' &&
        selection &&
        selectionBounds &&
        selectionArrowAction
      ) {
        event.preventDefault();
        const selectionArrowPreview = planCaptureSelectionArrowPreview({
          selection,
          selectionBounds,
          selectionArrowAction,
          minSelectionSize: MIN_SELECTION_SIZE,
          keyboardNudgeStep: KEYBOARD_NUDGE_STEP,
        });
        setSelection(selectionArrowPreview.selection);
        setPreviewImageBase64(selectionArrowPreview.previewImageBase64);
        void renderSelectionPreview(selectionArrowPreview.previewRender.rect);
      }
  }, [
    adjustAnnotationSize,
    clearAnnotations,
    completeCandidateSelection,
    completePreviewSelection,
    copyCurrentColor,
    copySelection,
    colorSampleFormat,
    activeAnnotationTool,
    annotationHistory,
    annotationGesture,
    annotationMoveGesture,
    annotations,
    captureCandidates,
    cursorPoint,
    draftSelectionMoveGesture,
    dismissCaptureLayer,
    editGesture,
    hasAnnotationEditingContext,
    hoverSelection,
    includeCapturedCursor,
    isAnnotationToolbarVisible,
    isMagnifierShown,
    isFillModeActive,
    textDraft,
    deleteSelectedAnnotation,
    redoAnnotation,
    refreshSession,
    renderSelectionPreview,
    restoreLastSelection,
    restoreSelectionFromHistory,
    selectAnnotationColor,
    selection,
    selectionBounds,
    selectedAnnotationIndex,
    selectFullCaptureArea,
    startPoint,
    status,
    syncHoverSelection,
    cursorColor,
    toggleAnnotationTool,
    toggleAnnotationFill,
    undoAnnotation,
    undoPolylineGesturePoint,
  ]);

  const releaseCaptureMagnifierRequest = useCallback(() => {
    setIsMagnifierRequested(false);
  }, []);

  const finishDraftSelectionMoveFromKeyboard = useCallback(() => {
    setDraftSelectionMoveGesture(null);
  }, []);

  useCaptureKeyboardHostEvents({
    isActive,
    status,
    isRenderingOutputRef,
    hasDraftSelectionMoveGesture: draftSelectionMoveGesture !== null,
    onKeyDown: handleCaptureKeyboardKeyDown,
    onReleaseMagnifierRequest: releaseCaptureMagnifierRequest,
    onFinishDraftSelectionMove: finishDraftSelectionMoveFromKeyboard,
    onCancelSession: cancelSession,
  });

  useEffect(() => {
    if (!textDraft) return;

    requestAnimationFrame(() => {
      textDraftInputRef.current?.focus();
    });
  }, [textDraft]);

  const handlePointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    const pointerDownPlan = planCaptureRootPointerDown(event, {
      status,
      hasSelectionBounds: Boolean(selectionBounds),
      hasSelection: selection !== null,
      hasTextDraft: textDraft !== null,
      hasAnnotationGesture: annotationGesture !== null,
      hasDismissibleLayer:
        textDraft !== null ||
        annotationMoveGesture !== null ||
        draftSelectionMoveGesture !== null ||
        selectedAnnotationIndex !== null ||
        activeAnnotationTool !== null ||
        annotationGesture !== null,
    });

    if (pointerDownPlan.type === 'cancel-pointer') {
      event.preventDefault();
      event.stopPropagation();
      const { action } = pointerDownPlan;

      if (action === 'finish-edit') {
        commitTextDraft();
      } else if (action === 'finish-annotation') {
        if (selection && selectionBounds && annotationGesture) {
          const point = viewportPointToVirtualPoint(
            { x: event.clientX, y: event.clientY },
            selectionBounds,
          );
          const localPoint = getCaptureSelectionLocalPoint(point, selection);
          commitAnnotationGestureAtPoint(localPoint, event.shiftKey);
        } else {
          dismissCaptureLayer();
        }
      } else if (action === 'dismiss-layer') {
        dismissCaptureLayer();
      } else if (action === 'reset-selection') {
        resetPreviewSelection();
      } else {
        void cancelSession();
      }
      return;
    }

    if (pointerDownPlan.type === 'ignore' || !selectionBounds) return;

    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    const snappedPoint = snapPointToRects(point, snapTargetRects, EDGE_SNAP_THRESHOLD);
    const draftStart = planCaptureDraftSelectionStart({
      cursorPoint: point,
      anchorPoint: snappedPoint,
    });
    cursorPointRef.current = draftStart.cursorPoint;
    draftSelectionRef.current = draftStart.draftSelection;
    setCursorPoint(draftStart.nextState.cursorPoint);
    event.currentTarget.setPointerCapture(event.pointerId);
    setStartPointWithRef(draftStart.nextState.startPoint);
    setSelection(draftStart.nextState.selection);
    setHoverSelection(draftStart.nextState.hoverSelection);
    scheduleSelectionOverlayPaint(draftStart.draftSelection, null);
    setPreviewImageBase64(draftStart.nextState.previewImageBase64);
    setRenderingOutput(draftStart.nextState.renderingOutput);
    setStatus(draftStart.nextState.status);
    setActiveAnnotationTool(draftStart.nextState.activeAnnotationTool);
    setAnnotationGesture(draftStart.nextState.annotationGesture);
    setDraftAnnotation(draftStart.nextState.draftAnnotation);
    setSelectedAnnotationIndex(draftStart.nextState.selectedAnnotationIndex);
    setAnnotationMoveGesture(draftStart.nextState.annotationMoveGesture);
    setDraftSelectionMoveGesture(draftStart.nextState.draftSelectionMoveGesture);
    keyboardDraftCursorPointRef.current = null;
    keyboardEditCursorPointRef.current = null;
    setTextDraft(draftStart.nextState.textDraft);
    setTextDraftAnnotationIndex(draftStart.nextState.textDraftAnnotationIndex);
    setAnnotationHistory(draftStart.nextState.annotationHistory);
  };

  const handlePointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!selectionBounds) return;

    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );

    cursorPointRef.current = point;

    if (shouldTrackMagnifierCursor) {
      setCursorPoint(point);
    }
    scheduleSelectionOverlayPaint();

    const activeStartPoint = startPointRef.current ?? startPoint;
    const shouldSyncHoverSelection = shouldSyncHoverSelectionOnPointerMove({
      status,
      hasActiveStartPoint: activeStartPoint !== null,
      hasEditGesture: editGesture !== null,
    });
    const pointerMoveAction = getCapturePointerMoveAction({
      status,
      hasSelection: selection !== null,
      hasActiveStartPoint: activeStartPoint !== null,
      hasEditGesture: editGesture !== null,
      hasAnnotationGesture: annotationGesture !== null,
      hasAnnotationMoveGesture: annotationMoveGesture !== null,
      hasDraftSelectionMoveGesture: draftSelectionMoveGesture !== null,
    });

    if (shouldSyncHoverSelection) {
      const nextHoverCandidate = getBestCandidateAtPoint(captureCandidates, point);
      const nextHoverSelection = nextHoverCandidate?.rect ?? null;
      syncHoverSelection(nextHoverSelection);
    }

    if (pointerMoveAction === 'move-annotation-gesture' && annotationGesture && selection) {
      const localPoint = getCaptureSelectionLocalPoint(point, selection);
      const gestureMove = planCaptureAnnotationGestureMove({
        gesture: annotationGesture,
        localPoint,
        annotationStyle,
        constrainGesture: event.shiftKey,
      });
      if (gestureMove.annotationGesture !== annotationGesture) {
        setAnnotationGesture(gestureMove.annotationGesture);
      }
      setDraftAnnotation(gestureMove.draftAnnotation);
      return;
    }

    if (pointerMoveAction === 'move-annotation' && annotationMoveGesture && selection) {
      const localPoint = getCaptureSelectionLocalPoint(point, selection);
      const annotationMove = planCaptureAnnotationMove({
        startAnnotation: annotationMoveGesture.startAnnotation,
        startPoint: annotationMoveGesture.startPoint,
        localPoint,
        constrainMove: event.shiftKey,
      });
      setPreviewImageBase64(annotationMove.previewImageBase64);
      setDraftAnnotation(annotationMove.draftAnnotation);
      return;
    }

    if (pointerMoveAction === 'move-draft-selection' && draftSelectionMoveGesture) {
      const draftSelectionMove = planCaptureDraftSelectionMove({
        gesture: draftSelectionMoveGesture,
        point,
        selectionBounds,
      });
      draftSelectionRef.current = draftSelectionMove.draftSelection;
      startPointRef.current = draftSelectionMove.anchorPoint;
      scheduleSelectionOverlayPaint(draftSelectionMove.draftSelection, null);
      setPreviewImageBase64(draftSelectionMove.previewImageBase64);
      setRenderingOutput(draftSelectionMove.renderingOutput);
      return;
    }

    if (pointerMoveAction === 'edit-selection' && editGesture) {
      const editMove = planCaptureSelectionEditMove({
        gesture: editGesture,
        point,
        selectionBounds,
        snapTargetRects,
        edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
        minSelectionSize: MIN_SELECTION_SIZE,
        preserveAspect: event.shiftKey,
      });
      keyboardEditCursorPointRef.current = editMove.keyboardEditCursorPoint;
      setSelection(editMove.selection);
      setPreviewImageBase64(editMove.previewImageBase64);
      setRenderingOutput(editMove.renderingOutput);
      return;
    }

    if (pointerMoveAction !== 'update-draft-selection' || !activeStartPoint) return;

    const draftMove = planCaptureDraftSelectionPointerMove({
      anchorPoint: activeStartPoint,
      point,
      snapTargetRects,
      edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
      constrainSelection: event.shiftKey,
    });
    keyboardDraftCursorPointRef.current = draftMove.keyboardDraftCursorPoint;
    draftSelectionRef.current = draftMove.draftSelection;
    scheduleSelectionOverlayPaint(draftMove.draftSelection, null);
  };

  const handlePointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!selectionBounds) return;

    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    cursorPointRef.current = point;
    const selectionReleasePoint =
      keyboardDraftCursorPointRef.current ?? cursorPointRef.current ?? point;
    const editReleasePoint = keyboardEditCursorPointRef.current ?? point;
    setCursorPoint(point);
    setDraftSelectionMoveGesture(null);
    keyboardDraftCursorPointRef.current = null;
    keyboardEditCursorPointRef.current = null;
    const activeStartPoint = startPointRef.current ?? startPoint;
    const pointerUpAction = getCapturePointerUpAction({
      status,
      hasSelection: selection !== null,
      hasActiveStartPoint: activeStartPoint !== null,
      hasEditGesture: editGesture !== null,
      hasAnnotationGesture: annotationGesture !== null,
      hasAnnotationMoveGesture: annotationMoveGesture !== null,
    });

    if (pointerUpAction === 'commit-annotation-gesture' && annotationGesture && selection) {
      const localPoint = getCaptureSelectionLocalPoint(point, selection);
      if (annotationGesture.tool === 'polyline') return;

      commitAnnotationGestureAtPoint(localPoint, event.shiftKey);
      return;
    }

    if (pointerUpAction === 'commit-annotation-move' && annotationMoveGesture && selection) {
      const localPoint = getCaptureSelectionLocalPoint(point, selection);
      const annotationMoveCommit = planCaptureAnnotationMoveCommit({
        annotationHistory,
        annotations,
        annotationIndex: annotationMoveGesture.annotationIndex,
        startAnnotation: annotationMoveGesture.startAnnotation,
        startPoint: annotationMoveGesture.startPoint,
        localPoint,
        constrainMove: event.shiftKey,
      });
      setAnnotationMoveGesture(annotationMoveCommit.annotationMoveGesture);
      setDraftAnnotation(annotationMoveCommit.draftAnnotation);
      setAnnotationHistory(annotationMoveCommit.annotationHistory);
      if (annotationMoveCommit.selectedAnnotationIndex !== undefined) {
        setSelectedAnnotationIndex(annotationMoveCommit.selectedAnnotationIndex);
      }
      void renderSelectionPreview(
        selection,
        annotationMoveCommit.previewAnnotations,
      );
      return;
    }

    if (pointerUpAction === 'commit-selection-edit' && editGesture) {
      const editCommit = planCaptureSelectionEditCommit({
        gesture: editGesture,
        point: editReleasePoint,
        selectionBounds,
        snapTargetRects,
        edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
        minSelectionSize: MIN_SELECTION_SIZE,
        preserveAspect: event.shiftKey,
        annotations,
      });
      setEditGesture(editCommit.editGesture);
      setSelection(editCommit.selection);
      setStatus(editCommit.status);
      void renderSelectionPreview(
        editCommit.previewRender.rect,
        editCommit.previewRender.annotations,
      );
      return;
    }

    if (pointerUpAction !== 'commit-draft-selection' || !activeStartPoint) return;

    const activeHoverSelection = hoverSelectionRef.current ?? hoverSelection;
    const draftCommit = planCaptureDraftSelectionCommit({
      anchorPoint: activeStartPoint,
      releasePoint: selectionReleasePoint,
      snapTargetRects,
      edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
      constrainSelection: event.shiftKey,
      captureCandidates,
      activeHoverSelection,
      minSelectionSize: MIN_SELECTION_SIZE,
    });
    setStartPointWithRef(draftCommit.startPoint);
    draftSelectionRef.current = draftCommit.draftSelection;
    scheduleSelectionOverlayPaint(null, draftCommit.overlayHoverSelection);

    if (draftCommit.type === 'clear-selection') {
      setSelection(draftCommit.selection);
      scheduleSelectionOverlayPaint(null, null);
      return;
    }

    completeManualSelection(draftCommit.selection);
  };

  const startMoveGesture = (event: React.PointerEvent<HTMLDivElement>) => {
    const previewPointerDownPlan = planCapturePreviewPointerDown(event, {
      status,
      hasSelection: selection !== null,
      hasSelectionBounds: Boolean(selectionBounds),
    });

    if (
      previewPointerDownPlan.type === 'ignore' ||
      !selection ||
      !selectionBounds
    ) {
      return;
    }

    if (previewPointerDownPlan.type === 'pin-selection') {
      event.preventDefault();
      event.stopPropagation();
      void pinSelection();
      return;
    }

    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    setCursorPoint(point);
    if (activeAnnotationTool) {
      setSelectedAnnotationIndex(null);
      const localPoint = getCaptureSelectionLocalPoint(point, selection);
      if (annotationGesture?.tool === 'polyline') {
        if (isFinishAnnotationGestureDoubleClick(event)) {
          commitAnnotationGestureAtPoint(localPoint, false);
          return;
        }

        const polylineContinue = planCapturePolylineAnnotationContinue({
          gesture: annotationGesture,
          localPoint,
          annotationStyle,
          constrainGesture: event.shiftKey,
        });
        setAnnotationGesture(polylineContinue.annotationGesture);
        setDraftAnnotation(polylineContinue.draftAnnotation);
        return;
      }

      if (activeAnnotationTool === 'text') {
        if (textDraft) return;
        setTextDraft(startTextAnnotationDraft(localPoint, textFontSize));
        setTextDraftAnnotationIndex(null);
        return;
      }

      if (activeAnnotationTool === 'eraser') {
        const erasePlan = planCaptureAnnotationErase({
          annotationHistory,
          localPoint,
        });
        setAnnotationMoveGesture(erasePlan.annotationMoveGesture);
        setDraftAnnotation(erasePlan.draftAnnotation);
        if (erasePlan.previewAnnotations) {
          setAnnotationHistory(erasePlan.annotationHistory);
          void renderSelectionPreview(selection, erasePlan.previewAnnotations);
        }
        return;
      }

      const toolStart = planCaptureAnnotationToolStart({
        tool: activeAnnotationTool,
        localPoint,
        annotationStyle,
      });
      setSelectedAnnotationIndex(toolStart.selectedAnnotationIndex);
      setAnnotationGesture(toolStart.annotationGesture);
      setDraftAnnotation(toolStart.draftAnnotation);
      return;
    }

    const localPoint = getCaptureSelectionLocalPoint(point, selection);
    const existingAnnotationStart = planCaptureExistingAnnotationPointerDown({
      annotations,
      localPoint,
      pointerDetail: event.detail,
      toolbarState: {
        annotationStyle,
        textFontSize,
      },
    });
    if (existingAnnotationStart) {
      setSelectedAnnotationIndex(existingAnnotationStart.selectedAnnotationIndex);
      setAnnotationStyle(existingAnnotationStart.toolbarState.annotationStyle);
      setTextFontSize(existingAnnotationStart.toolbarState.textFontSize);

      if (existingAnnotationStart.type === 'edit-text-annotation') {
        setAnnotationMoveGesture(existingAnnotationStart.annotationMoveGesture);
        setDraftAnnotation(existingAnnotationStart.draftAnnotation);
        setTextDraft(existingAnnotationStart.textDraft);
        setTextDraftAnnotationIndex(existingAnnotationStart.textDraftAnnotationIndex);
        setPreviewImageBase64(existingAnnotationStart.previewImageBase64);
        void renderSelectionPreview(
          selection,
          existingAnnotationStart.previewAnnotations,
        );
        return;
      }

      setAnnotationMoveGesture(existingAnnotationStart.annotationMoveGesture);
      return;
    }

    const selectionMoveStart = planCapturePreviewSelectionMoveStart({
      point,
      selection,
      hasTextDraft: textDraft !== null,
      isCopyDoubleClick: isCopyCaptureDoubleClick(event),
    });
    if (selectionMoveStart.type === 'copy-selection') {
      event.preventDefault();
      void copySelection();
      return;
    }

    setSelectedAnnotationIndex(selectionMoveStart.selectedAnnotationIndex);
    setAnnotationMoveGesture(selectionMoveStart.annotationMoveGesture);
    setEditGesture(selectionMoveStart.editGesture);
    setPreviewImageBase64(selectionMoveStart.previewImageBase64);
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
    event: React.PointerEvent<HTMLButtonElement>,
  ) => {
    if (status !== 'preview' || !selection || !selectionBounds) return;

    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    const resizeStart = planCaptureSelectionResizeStart({
      point,
      selection,
      handle,
    });
    setCursorPoint(resizeStart.cursorPoint);
    setEditGesture(resizeStart.editGesture);
    setPreviewImageBase64(resizeStart.previewImageBase64);
  };

  const handleWheel = (event: React.WheelEvent<HTMLDivElement>) => {
    const sizeDirection = planCapturePointerWheelSizeAdjustment(event, {
      status,
      hasTextDraft: textDraft !== null,
      hasAnnotationGesture: annotationGesture !== null,
      hasAnnotationMoveGesture: annotationMoveGesture !== null,
      hasAnnotationEditingContext,
    });
    if (!sizeDirection) return;

    event.preventDefault();
    adjustAnnotationSize(sizeDirection);
  };

  if (!isActive) return null;

  return (
    <div
      className={getCaptureRootClassName(status)}
      style={{
        width: `${viewportBounds?.width ?? window.innerWidth}px`,
        height: `${viewportBounds?.height ?? window.innerHeight}px`,
        cursor: getCaptureRootCursorStyle(status),
      }}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onWheel={handleWheel}
      onContextMenu={(event) => event.preventDefault()}
    >
      {shouldShowCaptureLoadingMask(status) && (
        <div className="absolute inset-0 bg-black" aria-label="Loading capture" />
      )}

      {status === 'error' && (
        <div className="absolute left-4 top-4 max-w-md rounded bg-red-950/90 px-3 py-2 text-sm text-red-100 shadow-lg">
          {error}
        </div>
      )}

      {status === 'preview' && selection && selectionViewportRect && (
        <>
          <CapturePreviewImage
            imageBase64={previewImageBase64}
            selectionViewportRect={selectionViewportRect}
          />
          <CaptureDraftAnnotationOverlay
            draftAnnotation={draftAnnotation}
            selectionViewportRect={selectionViewportRect}
          />
          {textDraft && (
            <CaptureTextDraftEditor
              inputRef={textDraftInputRef}
              textDraft={textDraft}
              selectionViewportRect={selectionViewportRect}
              annotationStyle={annotationStyle}
              onCommit={commitTextDraft}
              onTextChange={updateTextDraftText}
              onDiscard={discardTextDraft}
            />
          )}
          <CaptureSelectedAnnotationBoundsOverlay
            selectedAnnotationBounds={selectedAnnotationBounds}
            selectionViewportRect={selectionViewportRect}
          />
          <div
            className={getCaptureEditorSelectionClassName(
              status,
              Boolean(activeAnnotationTool),
            )}
            style={rectStyle(selectionViewportRect)}
            onPointerDown={startMoveGesture}
          />
          {status === 'preview' && (
            <CaptureSelectionResizeHandles
              selectionViewportRect={selectionViewportRect}
              onResizeHandlePointerDown={startResizeGesture}
            />
          )}
          {toolbarPosition && isAnnotationToolbarVisible && (
            <CaptureEditorToolbar
              position={toolbarPosition}
              width={TOOLBAR_SIZE.width}
              activeAnnotationTool={activeAnnotationTool}
              annotationStyle={annotationStyle}
              textFontSize={textFontSize}
              textDraftActive={textDraft !== null}
              isTextSizingActive={isTextSizingActive}
              isFillModeActive={isFillModeActive}
              isRenderingOutput={isRenderingOutput}
              onSelectMove={() => setActiveAnnotationTool(null)}
              onToggleAnnotationTool={toggleAnnotationTool}
              onApplyAnnotationStyle={applySelectedAnnotationStyle}
              onTextDraftFontSizeChange={(fontSize) => {
                setTextFontSize(fontSize);
                setTextDraft((draft) =>
                  draft ? { ...draft, fontSize } : draft,
                );
              }}
              onCancel={cancelSession}
              onRunOcr={runOcrSelection}
              onCopy={copySelection}
              onSave={saveSelection}
              onQuickSave={quickSaveSelection}
            />
          )}
          <CaptureRenderingOutputBar
            isRenderingOutput={isRenderingOutput}
            selectionViewportRect={selectionViewportRect}
          />
        </>
      )}
      <CaptureSelectionOverlayCanvas
        canvasRef={selectionOverlayCanvasRef}
        cssSize={selectionOverlayCssSize}
        pixelRatio={selectionOverlayPixelRatio}
      />
      {isMagnifierShown &&
        cursorMonitor &&
        cursorViewportPoint &&
        cursorInMonitorPoint &&
        viewportBounds && (
        <CaptureMagnifierOverlay
          imageBase64={cursorMonitor.image_base64}
          viewportCursor={cursorViewportPoint}
          imageCursor={cursorInMonitorPoint}
          viewportBounds={viewportBounds}
          imageSize={{
            width: cursorMonitor.logical_bounds.width,
            height: cursorMonitor.logical_bounds.height,
          }}
          selection={
            selection ??
            draftSelectionRef.current ??
            hoverSelectionRef.current ??
            hoverSelection
          }
          color={cursorColor}
          colorFormat={colorSampleFormat}
        />
      )}
    </div>
  );
}
