import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent,
  type WheelEvent,
} from 'react';

import { createCaptureWorkspaceRuntime } from '../../application/capture-workspace/runtime';
import type { AnnotationHistory } from './annotationHistory';
import {
  canToggleCapturedCursor,
  type SelectionHistoryStep,
} from './captureActions';
import {
  prepareCaptureSurfaceForReveal,
  restoreCaptureSelectionFromHistory,
  restoreLastSuccessfulCaptureSelection,
} from './captureHostRuntime';
import { planCaptureManualSelectionTransition } from './captureEditorRuntime';
import {
  shouldPollCaptureHoverSelection,
  startCaptureHoverSelectionPolling,
} from './captureHoverPolling';
import { useCaptureMagnifierPixelSource } from './captureMagnifierRuntime';
import { useCaptureSelectionOverlay } from './captureSelectionOverlayRuntime';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import {
  handleCaptureWorkspaceKeyDown,
  type CaptureWorkspaceKeyboardActions,
  type CaptureWorkspaceKeyboardDerivedState,
  type CaptureWorkspaceKeyboardRefs,
} from './captureWorkspaceKeyboard';
import { planManualSelectionCompletion } from './captureInteractionRuntime';
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
} from './captureWorkspacePointer';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import { useCaptureWorkspaceEditorController } from './useCaptureWorkspaceEditorController';
import { useCaptureWorkspaceState } from './useCaptureWorkspaceState';
import type { SelectionHandle } from './selection';
import { getCurrentMonitorBounds } from './virtualDesktop';
import type {
  AnnotationCommand,
  CaptureLaunch,
  CaptureMode,
  LogicalRect,
  Point,
} from './types';
import { useCaptureWorkspaceRuntime } from './runtimeContext';

const MIN_SELECTION_SIZE = 10;
const TOOLBAR_GAP = 14;
const TOOLBAR_SIZE = { width: 640, height: 42 };
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
  const runtime = useCaptureWorkspaceRuntime();
  const onInactiveRef = useRef(onInactive);
  const screenshotSavePathRef = useRef(screenshotSavePath);
  onInactiveRef.current = onInactive;
  screenshotSavePathRef.current = screenshotSavePath;
  const workflowRuntime = useMemo(
    () =>
      createCaptureWorkspaceRuntime({
        platform: runtime,
        onInactive: () => onInactiveRef.current?.(),
        screenshotSavePath: () => screenshotSavePathRef.current,
        storage: window.localStorage,
      }),
    [runtime],
  );
  const keyboardDraftCursorPointRef = useRef<Point | null>(null);
  const keyboardEditCursorPointRef = useRef<Point | null>(null);
  const captureFrontendPerfRef = useRef<{
    mode: CaptureMode;
    sessionId: string | null;
    startMs: number;
    hasLoggedImagesReady: boolean;
  } | null>(null);
  const isRenderingOutputRef = useRef(false);
  const handleRenderingOutputChange = useCallback(
    (isRenderingOutput: boolean) => {
      isRenderingOutputRef.current = isRenderingOutput;
    },
    [],
  );
  const workspace = useCaptureWorkspaceState({
    onRenderingOutputChange: handleRenderingOutputChange,
  });
  const [hydratedCaptureSessionId, setHydratedCaptureSessionId] =
    useState<string | null>(null);

  useEffect(
    () =>
      workflowRuntime.subscribe(() => {
        const next = workflowRuntime.renderState;
        const perf = captureFrontendPerfRef.current;
        if (perf && next.sessionId && perf.sessionId !== next.sessionId) {
          perf.sessionId = next.sessionId;
          void runtime.commands.logCaptureFrontendPerf({
            event: 'session_loaded',
            mode: perf.mode,
            sessionId: next.sessionId,
            elapsedMs: performance.now() - perf.startMs,
          }).catch(() => undefined);
        }
        if (next.status === 'idle' && !next.session) {
          workspace.resetSession();
          setHydratedCaptureSessionId(null);
          return;
        }
        workspace.applyPatch({
          status: next.status,
          mode: next.mode,
          session: next.session,
          cursorPoint: next.cursorPoint,
          selection: next.selection,
          hoverSelection: next.hoverSelection,
          previewImageBase64: next.previewImageBase64,
          isRenderingOutput: next.isRenderingOutput,
          error: next.error,
        });
        setHydratedCaptureSessionId(
          next.hasHydratedPixelSource ? next.sessionId : null,
        );
      }),
    [runtime.commands, workflowRuntime, workspace.applyPatch, workspace.resetSession],
  );

  const captureWorkspaceState: CaptureWorkspaceState = {
    status: workspace.status,
    mode: workspace.mode,
    session: workspace.session,
    startPoint: workspace.startPoint,
    cursorPoint: workspace.cursorPoint,
    selection: workspace.selection,
    hoverSelection: workspace.hoverSelection,
    editGesture: workspace.editGesture,
    activeAnnotationTool: workspace.activeAnnotationTool,
    annotationGesture: workspace.annotationGesture,
    draftAnnotation: workspace.draftAnnotation,
    selectedAnnotationIndex: workspace.selectedAnnotationIndex,
    annotationMoveGesture: workspace.annotationMoveGesture,
    draftSelectionMoveGesture: workspace.draftSelectionMoveGesture,
    textDraft: workspace.textDraft,
    textDraftAnnotationIndex: workspace.textDraftAnnotationIndex,
    annotationStyle: workspace.annotationStyle,
    textFontSize: workspace.textFontSize,
    annotationHistory: workspace.annotationHistory,
    previewImageBase64: workspace.previewImageBase64,
    isAnnotationToolbarVisible: workspace.isAnnotationToolbarVisible,
    cursorColor: workspace.cursorColor,
    colorSampleFormat: workspace.colorSampleFormat,
    isMagnifierRequested: workspace.isMagnifierRequested,
    isRenderingOutput: workspace.isRenderingOutput,
    includeCapturedCursor: workspace.includeCapturedCursor,
    error: workspace.error,
  };
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
  const overlay = useCaptureSelectionOverlay({
    status: workspace.status,
    selectionBounds: derived.selectionBounds,
    selection: workspace.selection,
    viewportBounds: derived.viewportBounds,
    cursorPointRef: workspace.cursorPointRef,
    draftSelectionRef: workspace.draftSelectionRef,
    hoverSelectionRef: workspace.hoverSelectionRef,
  });

  const syncHoverSelection = useCallback(
    (nextHoverSelection: LogicalRect | null) => {
      if (
        areRectsEqual(
          workspace.hoverSelectionRef.current,
          nextHoverSelection,
        )
      ) {
        return;
      }

      workspace.syncHoverSelection(nextHoverSelection);
      overlay.schedulePaint(null, nextHoverSelection, null);
    },
    [overlay.schedulePaint, workspace.hoverSelectionRef, workspace.syncHoverSelection],
  );

  useEffect(() => {
    if (!workspace.session || !derived.selectionBounds) return;

    const canPoll = () =>
      shouldPollCaptureHoverSelection({
        status: workspace.status,
        hasSession: Boolean(workspace.session),
        hasSelectionBounds: Boolean(derived.selectionBounds),
        hasActiveStartPoint: Boolean(
          workspace.startPointRef.current ?? workspace.startPoint,
        ),
        hasEditGesture: Boolean(workspace.editGesture),
      });

    return startCaptureHoverSelectionPolling({
      sessionId: workspace.session.id,
      candidates: derived.captureCandidates,
      shouldTrackMagnifierCursor: derived.shouldTrackMagnifierCursor,
      intervalMs: CAPTURE_HOVER_POLL_INTERVAL_MS,
      canPoll,
      getCursorPosition: runtime.commands.currentCaptureCursorPosition,
      setCursorPointRef: (point) => {
        workspace.cursorPointRef.current = point;
      },
      setCursorPoint: workspace.setCursorPoint,
      scheduleSelectionOverlayPaint: overlay.schedulePaint,
      syncHoverSelection,
      setTimeout: window.setTimeout,
      clearTimeout: window.clearTimeout,
    });
  }, [
    derived.captureCandidates,
    derived.selectionBounds,
    derived.shouldTrackMagnifierCursor,
    overlay.schedulePaint,
    syncHoverSelection,
    workspace.cursorPointRef,
    workspace.editGesture,
    workspace.session,
    workspace.setCursorPoint,
    workspace.startPoint,
    workspace.startPointRef,
    workspace.status,
  ]);

  const commitTextDraftToHistoryRef = useRef<() => AnnotationHistory>(
    () => workspace.annotationHistory,
  );
  const commitTextDraftToHistory = useCallback(
    () => commitTextDraftToHistoryRef.current(),
    [],
  );
  const hasStartedInitialSessionRef = useRef(false);
  const hasRevealedCaptureWindowRef = useRef(false);
  const startSession = useCallback(
    (mode: CaptureMode, sessionId?: string) => {
      captureFrontendPerfRef.current = {
        mode,
        sessionId: null,
        startMs: performance.now(),
        hasLoggedImagesReady: false,
      };
      void runtime.commands.logCaptureFrontendPerf({
        event: 'start_session',
        mode,
        sessionId: null,
        elapsedMs: 0,
      }).catch(() => undefined);
      hasRevealedCaptureWindowRef.current = false;
      workspace.resetInteraction();
      overlay.reset();
      setHydratedCaptureSessionId(null);
      return workflowRuntime.actions.startSession(mode, sessionId);
    },
    [overlay.reset, runtime.commands, workflowRuntime, workspace.resetInteraction],
  );

  const ensureCaptureSnapshotsHydrated = useCallback(
    async (_sessionId: string) => {
      await workflowRuntime.actions.hydrateSnapshots();
      const perf = captureFrontendPerfRef.current;
      if (perf) {
        void runtime.commands.logCaptureFrontendPerf({
          event: 'snapshots_hydrated',
          mode: perf.mode,
          sessionId: workflowRuntime.renderState.sessionId,
          elapsedMs: performance.now() - perf.startMs,
        }).catch(() => undefined);
      }
      return workflowRuntime.renderState.session!;
    },
    [runtime.commands, workflowRuntime],
  );
  const cancelSession = workflowRuntime.actions.cancelSession;
  const renderSelectionPreview = useCallback(
    (
      rect: LogicalRect,
      annotations: AnnotationCommand[] = captureWorkspaceState.annotationHistory
        .annotations,
      includeCursor =
        captureWorkspaceState.includeCapturedCursor &&
        canToggleCapturedCursor(captureWorkspaceState.session),
    ) =>
      workflowRuntime.actions.renderSelectionPreview(
        rect,
        annotations,
        includeCursor,
      ),
    [captureWorkspaceState, workflowRuntime],
  );
  const completePreviewSelection = useCallback(
    async (
      action: Parameters<
        typeof workflowRuntime.actions.completePreviewSelection
      >[0],
      options: { commitTextDraft?: boolean; guardCompletion?: boolean } = {},
    ) => {
      if (!captureWorkspaceState.selection) return;
      if (options.guardCompletion && isRenderingOutputRef.current) return;

      const history =
        options.commitTextDraft === false
          ? captureWorkspaceState.annotationHistory
          : commitTextDraftToHistory();
      await workflowRuntime.actions.completePreviewSelection(
        action,
        captureWorkspaceState.selection,
        history.annotations,
        captureWorkspaceState.includeCapturedCursor &&
          canToggleCapturedCursor(captureWorkspaceState.session),
      );
    },
    [captureWorkspaceState, commitTextDraftToHistory, workflowRuntime],
  );
  const completeCandidateSelection = useCallback(
    (rect: LogicalRect, action: Parameters<typeof workflowRuntime.actions.completeCandidateSelection>[1]) =>
      workflowRuntime.actions.completeCandidateSelection(rect, action),
    [workflowRuntime],
  );
  const completeManualSelection = useCallback(
    (rect: LogicalRect) => {
      const transition = planCaptureManualSelectionTransition({
        rect,
        completion: planManualSelectionCompletion(captureWorkspaceState.mode),
      });
      workspace.draftSelectionRef.current = null;
      if (transition.clearOverlay) overlay.reset();
      if (transition.type === 'effects') {
        const { renderingOutput, ...nextState } = transition.nextState;
        workspace.applyPatch({
          ...nextState,
          isRenderingOutput: renderingOutput,
        });
      } else {
        workspace.applyPatch(transition.nextState);
      }
      return workflowRuntime.actions.completeManualSelection(rect);
    },
    [captureWorkspaceState.mode, overlay.reset, workflowRuntime, workspace.applyPatch, workspace.draftSelectionRef],
  );
  const resetPreviewSelection = useCallback(() => {
    workflowRuntime.actions.resetPreview();
    workspace.resetPreview();
    overlay.reset();
  }, [overlay.reset, workflowRuntime, workspace.resetPreview]);
  const copySelection = useCallback(
    () => completePreviewSelection('copy', { guardCompletion: true }),
    [completePreviewSelection],
  );
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
  const selectFullCaptureArea = useCallback(() => {
    if (!captureWorkspaceState.session || !derived.selectionBounds) return;
    const currentPoint =
      workspace.cursorPointRef.current ??
      captureWorkspaceState.cursorPoint ??
      captureWorkspaceState.session.captured_cursor?.logical_position ??
      null;
    void completeManualSelection(
      getCurrentMonitorBounds(captureWorkspaceState.session.monitors, currentPoint),
    );
  }, [captureWorkspaceState, completeManualSelection, derived.selectionBounds, workspace.cursorPointRef]);
  const restoreLastSelection = useCallback(() => {
    if (!derived.selectionBounds) return;
    restoreLastSuccessfulCaptureSelection({
      storage: window.localStorage,
      selectionBounds: derived.selectionBounds,
      minSelectionSize: MIN_SELECTION_SIZE,
      completeSelection: completeManualSelection,
    });
  }, [completeManualSelection, derived.selectionBounds]);
  const restoreSelectionFromHistory = useCallback(
    (step: SelectionHistoryStep) => {
      if (!derived.selectionBounds) return;
      restoreCaptureSelectionFromHistory({
        storage: window.localStorage,
        currentSelection: captureWorkspaceState.selection,
        step,
        selectionBounds: derived.selectionBounds,
        minSelectionSize: MIN_SELECTION_SIZE,
        completeSelection: completeManualSelection,
      });
    },
    [captureWorkspaceState.selection, completeManualSelection, derived.selectionBounds],
  );

  useEffect(() => {
    if (!initialMode || hasStartedInitialSessionRef.current) return;
    hasStartedInitialSessionRef.current = true;
    void startSession(initialMode, initialSessionId);
  }, [initialMode, initialSessionId, startSession]);

  useEffect(() => {
    const perf = captureFrontendPerfRef.current;
    if (
      !perf ||
      !derived.areCaptureImagesReady ||
      perf.hasLoggedImagesReady ||
      perf.sessionId !== captureWorkspaceState.session?.id
    ) {
      return;
    }

    perf.hasLoggedImagesReady = true;
    void runtime.commands.logCaptureFrontendPerf({
      event: 'images_ready',
      mode: perf.mode,
      sessionId: perf.sessionId,
      elapsedMs: performance.now() - perf.startMs,
    }).catch(() => undefined);
  }, [captureWorkspaceState.session?.id, derived.areCaptureImagesReady, runtime.commands]);

  const prepareCaptureSurface = useCallback(
    () =>
      prepareCaptureSurfaceForReveal({
        frame: overlay.getCurrentFrame(),
        paintSelectionOverlayFrame: overlay.paintFrame,
      }),
    [overlay.getCurrentFrame, overlay.paintFrame],
  );
  const hostWindowReveal = useMemo(
    () => ({
      status: captureWorkspaceState.status,
      sessionId: captureWorkspaceState.session?.id ?? null,
      hasCaptureImagesReady: derived.areCaptureImagesReady,
      hasRevealedRef: hasRevealedCaptureWindowRef,
      prepareSurface: prepareCaptureSurface,
      onRevealedSession: (sessionId: string) => {
        const perf = captureFrontendPerfRef.current;
        if (!perf) return;
        void runtime.commands.logCaptureFrontendPerf({
          event: 'revealed',
          mode: perf.mode,
          sessionId,
          elapsedMs: performance.now() - perf.startMs,
        }).catch(() => undefined);
      },
      onError: (error: unknown) => {
        workspace.applyPatch({
          status: 'error',
          error: error instanceof Error ? error.message : String(error),
        });
      },
    }),
    [captureWorkspaceState.session?.id, captureWorkspaceState.status, derived.areCaptureImagesReady, prepareCaptureSurface, runtime.commands, workspace.applyPatch],
  );
  const handleNativeCopyRequest = useCallback(() => {
    if (captureWorkspaceState.status === 'preview') {
      void copySelection();
      return;
    }
    const activeStartPoint =
      workspace.startPointRef.current ?? captureWorkspaceState.startPoint;
    const activeHoverSelection =
      workspace.hoverSelectionRef.current ?? captureWorkspaceState.hoverSelection;
    if (
      captureWorkspaceState.status === 'selecting' &&
      !captureWorkspaceState.textDraft &&
      activeStartPoint === null &&
      activeHoverSelection
    ) {
      void completeCandidateSelection(activeHoverSelection, 'copy');
    }
  }, [captureWorkspaceState, completeCandidateSelection, copySelection, workspace.hoverSelectionRef, workspace.startPointRef]);
  const hostSubscriptions = useMemo(
    () => ({
      isActive: captureWorkspaceState.status !== 'idle',
      onLaunch: (launch: CaptureLaunch) => {
        void startSession(launch.mode, launch.sessionId);
      },
      onCancel: cancelSession,
      onCopy: handleNativeCopyRequest,
    }),
    [cancelSession, captureWorkspaceState.status, handleNativeCopyRequest, startSession],
  );

  const hostActions = useMemo(
    () => ({
      ensureCaptureSnapshotsHydrated,
      cancelSession,
      refreshSession: async () => {
        hasRevealedCaptureWindowRef.current = false;
        workspace.resetInteraction();
        overlay.reset();
        setHydratedCaptureSessionId(null);
        await workflowRuntime.actions.refreshSession();
      },
      renderSelectionPreview,
      completeCandidateSelection,
      completePreviewSelection,
      completeManualSelection,
      pinSelection,
      copySelection,
      saveSelection,
      quickSaveSelection,
      runOcrSelection,
      resetPreviewSelection,
      selectFullCaptureArea,
      restoreLastSelection,
      restoreSelectionFromHistory,
    }),
    [cancelSession, completeCandidateSelection, completeManualSelection, completePreviewSelection, copySelection, ensureCaptureSnapshotsHydrated, overlay.reset, pinSelection, quickSaveSelection, renderSelectionPreview, resetPreviewSelection, restoreLastSelection, restoreSelectionFromHistory, runOcrSelection, saveSelection, selectFullCaptureArea, workflowRuntime, workspace.resetInteraction],
  );

  const editorSetters = useMemo(
    () => ({
      setStatus: workspace.setStatus,
      setError: workspace.setError,
      setActiveAnnotationTool: workspace.setActiveAnnotationTool,
      setAnnotationGesture: workspace.setAnnotationGesture,
      setDraftAnnotation: workspace.setDraftAnnotation,
      setSelectedAnnotationIndex: workspace.setSelectedAnnotationIndex,
      setAnnotationMoveGesture: workspace.setAnnotationMoveGesture,
      setDraftSelectionMoveGesture: workspace.setDraftSelectionMoveGesture,
      setTextDraft: workspace.setTextDraft,
      setTextDraftAnnotationIndex: workspace.setTextDraftAnnotationIndex,
      setAnnotationStyle: workspace.setAnnotationStyle,
      setTextFontSize: workspace.setTextFontSize,
      setAnnotationHistory: workspace.setAnnotationHistory,
    }),
    [
      workspace.setActiveAnnotationTool,
      workspace.setAnnotationGesture,
      workspace.setAnnotationHistory,
      workspace.setAnnotationMoveGesture,
      workspace.setAnnotationStyle,
      workspace.setDraftAnnotation,
      workspace.setDraftSelectionMoveGesture,
      workspace.setError,
      workspace.setSelectedAnnotationIndex,
      workspace.setStatus,
      workspace.setTextDraft,
      workspace.setTextDraftAnnotationIndex,
      workspace.setTextFontSize,
    ],
  );
  const editorDerived = useMemo(
    () => ({
      annotations: derived.annotations,
      canUndoAnnotation: derived.canUndoAnnotation,
      canRedoAnnotation: derived.canRedoAnnotation,
      isTextSizingActive: derived.isTextSizingActive,
      isFillModeActive: derived.isFillModeActive,
    }),
    [
      derived.annotations,
      derived.canRedoAnnotation,
      derived.canUndoAnnotation,
      derived.isFillModeActive,
      derived.isTextSizingActive,
    ],
  );
  const editorHost = useMemo(
    () => ({
      renderSelectionPreview,
      cancelSession,
    }),
    [cancelSession, renderSelectionPreview],
  );
  const editorController = useCaptureWorkspaceEditorController({
    state: captureWorkspaceState,
    derived: editorDerived,
    setters: editorSetters,
    host: editorHost,
    runtime,
  });
  commitTextDraftToHistoryRef.current =
    editorController.commitTextDraftToHistory;

  useCaptureMagnifierPixelSource({
    session: workspace.session,
    hasHydratedPixelSource: derived.hasHydratedPixelSource,
    isMagnifierRequested: workspace.isMagnifierRequested,
    isMagnifierShown: derived.isMagnifierShown,
    cursorMonitor: derived.cursorMonitor,
    cursorInMonitorPoint: derived.cursorInMonitorPoint,
    setCursorColor: workspace.setCursorColor,
    ensureCaptureSnapshotsHydrated,
  });

  const inputRefs = useMemo<CaptureWorkspaceKeyboardRefs>(
    () => ({
      startPointRef: workspace.startPointRef,
      cursorPointRef: workspace.cursorPointRef,
      draftSelectionRef: workspace.draftSelectionRef,
      hoverSelectionRef: workspace.hoverSelectionRef,
      keyboardDraftCursorPointRef,
      keyboardEditCursorPointRef,
    }),
    [
      workspace.cursorPointRef,
      workspace.draftSelectionRef,
      workspace.hoverSelectionRef,
      workspace.startPointRef,
    ],
  );
  const inputDerived = useMemo(
    () => ({
      annotations: derived.annotations,
      captureCandidates: derived.captureCandidates,
      selectionBounds: derived.selectionBounds,
      snapTargetRects: derived.snapTargetRects,
      hasAnnotationEditingContext: derived.hasAnnotationEditingContext,
      isAnnotationToolbarVisible: workspace.isAnnotationToolbarVisible,
      isMagnifierShown: derived.isMagnifierShown,
      isFillModeActive: derived.isFillModeActive,
      cursorColor: workspace.cursorColor,
      shouldTrackMagnifierCursor: derived.shouldTrackMagnifierCursor,
    }),
    [
      derived.annotations,
      derived.captureCandidates,
      derived.hasAnnotationEditingContext,
      derived.isFillModeActive,
      derived.isMagnifierShown,
      derived.selectionBounds,
      derived.shouldTrackMagnifierCursor,
      derived.snapTargetRects,
      workspace.cursorColor,
      workspace.isAnnotationToolbarVisible,
    ],
  );
  const inputSetters = useMemo(
    () => ({
      setStatus: workspace.setStatus,
      setCursorPoint: workspace.setCursorPoint,
      setSelection: workspace.setSelection,
      setHoverSelection: workspace.setHoverSelection,
      setEditGesture: workspace.setEditGesture,
      setActiveAnnotationTool: workspace.setActiveAnnotationTool,
      setAnnotationGesture: workspace.setAnnotationGesture,
      setDraftAnnotation: workspace.setDraftAnnotation,
      setSelectedAnnotationIndex: workspace.setSelectedAnnotationIndex,
      setAnnotationMoveGesture: workspace.setAnnotationMoveGesture,
      setDraftSelectionMoveGesture: workspace.setDraftSelectionMoveGesture,
      setTextDraft: workspace.setTextDraft,
      setTextDraftAnnotationIndex: workspace.setTextDraftAnnotationIndex,
      setAnnotationStyle: workspace.setAnnotationStyle,
      setTextFontSize: workspace.setTextFontSize,
      setAnnotationHistory: workspace.setAnnotationHistory,
      setPreviewImageBase64: workspace.setPreviewImageBase64,
      setIsAnnotationToolbarVisible: workspace.setIsAnnotationToolbarVisible,
      setColorSampleFormat: workspace.setColorSampleFormat,
      setIsMagnifierRequested: workspace.setIsMagnifierRequested,
      setIncludeCapturedCursor: workspace.setIncludeCapturedCursor,
      setRenderingOutput: workspace.setRenderingOutput,
      setStartPointWithRef: workspace.setStartPointWithRef,
    }),
    [
      workspace.setActiveAnnotationTool,
      workspace.setAnnotationGesture,
      workspace.setAnnotationHistory,
      workspace.setAnnotationMoveGesture,
      workspace.setAnnotationStyle,
      workspace.setColorSampleFormat,
      workspace.setCursorPoint,
      workspace.setDraftAnnotation,
      workspace.setDraftSelectionMoveGesture,
      workspace.setEditGesture,
      workspace.setHoverSelection,
      workspace.setIncludeCapturedCursor,
      workspace.setIsAnnotationToolbarVisible,
      workspace.setIsMagnifierRequested,
      workspace.setPreviewImageBase64,
      workspace.setRenderingOutput,
      workspace.setSelectedAnnotationIndex,
      workspace.setSelection,
      workspace.setStartPointWithRef,
      workspace.setStatus,
      workspace.setTextDraft,
      workspace.setTextDraftAnnotationIndex,
      workspace.setTextFontSize,
    ],
  );
  const keyboardDerived = useMemo<CaptureWorkspaceKeyboardDerivedState>(
    () => ({
      annotations: inputDerived.annotations,
      captureCandidates: inputDerived.captureCandidates,
      selectionBounds: inputDerived.selectionBounds,
      hasAnnotationEditingContext: inputDerived.hasAnnotationEditingContext,
      isAnnotationToolbarVisible: inputDerived.isAnnotationToolbarVisible,
      isMagnifierShown: inputDerived.isMagnifierShown,
      isFillModeActive: inputDerived.isFillModeActive,
      cursorColor: inputDerived.cursorColor,
    }),
    [inputDerived],
  );
  const pointerDerived = useMemo<CaptureWorkspacePointerDerivedState>(
    () => ({
      annotations: inputDerived.annotations,
      captureCandidates: inputDerived.captureCandidates,
      selectionBounds: inputDerived.selectionBounds,
      snapTargetRects: inputDerived.snapTargetRects,
      hasAnnotationEditingContext: inputDerived.hasAnnotationEditingContext,
      shouldTrackMagnifierCursor: inputDerived.shouldTrackMagnifierCursor,
    }),
    [inputDerived],
  );
  const keyboardActions = useMemo<CaptureWorkspaceKeyboardActions>(
    () => ({
      dismissCaptureLayer: editorController.actions.dismissCaptureLayer,
      refreshSession: hostActions.refreshSession,
      setIncludeCapturedCursor: inputSetters.setIncludeCapturedCursor,
      clearPreviewImage: () => inputSetters.setPreviewImageBase64(null),
      renderSelectionPreview: hostActions.renderSelectionPreview,
      setIsMagnifierRequested: inputSetters.setIsMagnifierRequested,
      clearAnnotations: editorController.actions.clearAnnotations,
      undoPolylineGesturePoint:
        editorController.actions.undoPolylineGesturePoint,
      undoAnnotation: editorController.actions.undoAnnotation,
      redoAnnotation: editorController.actions.redoAnnotation,
      deleteSelectedAnnotation: editorController.actions.deleteSelectedAnnotation,
      copyCurrentColor: () =>
        editorController.actions.copyCurrentColor(inputDerived.cursorColor),
      setColorSampleFormat: inputSetters.setColorSampleFormat,
      restoreSelectionFromHistory: hostActions.restoreSelectionFromHistory,
      restoreLastSelection: hostActions.restoreLastSelection,
      setCursorPoint: inputSetters.setCursorPoint,
      setSelection: inputSetters.setSelection,
      scheduleSelectionOverlayPaint: overlay.schedulePaint,
      setPreviewImageBase64: inputSetters.setPreviewImageBase64,
      setRenderingOutput: inputSetters.setRenderingOutput,
      setEditGesture: inputSetters.setEditGesture,
      syncHoverSelection,
      selectFullCaptureArea: hostActions.selectFullCaptureArea,
      completeCandidateSelection: hostActions.completeCandidateSelection,
      setIsAnnotationToolbarVisible:
        inputSetters.setIsAnnotationToolbarVisible,
      completePreviewSelection: hostActions.completePreviewSelection,
      adjustAnnotationSize: editorController.actions.adjustAnnotationSize,
      toggleAnnotationFill: editorController.actions.toggleAnnotationFill,
      setActiveAnnotationTool: inputSetters.setActiveAnnotationTool,
      setSelectedAnnotationIndex: inputSetters.setSelectedAnnotationIndex,
      setAnnotationGesture: inputSetters.setAnnotationGesture,
      setAnnotationMoveGesture: inputSetters.setAnnotationMoveGesture,
      setDraftAnnotation: inputSetters.setDraftAnnotation,
      selectAnnotationColor: editorController.actions.selectAnnotationColor,
      toggleAnnotationTool: editorController.actions.toggleAnnotationTool,
      setDraftSelectionMoveGesture:
        inputSetters.setDraftSelectionMoveGesture,
      setAnnotationHistory: inputSetters.setAnnotationHistory,
    }),
    [editorController.actions, hostActions, inputDerived.cursorColor, inputSetters, overlay.schedulePaint, syncHoverSelection],
  );
  const pointerActions = useMemo<CaptureWorkspacePointerActions>(
    () => ({
      commitTextDraft: editorController.actions.commitTextDraft,
      commitAnnotationGestureAtPoint:
        editorController.actions.commitAnnotationGestureAtPoint,
      dismissCaptureLayer: editorController.actions.dismissCaptureLayer,
      resetPreviewSelection: hostActions.resetPreviewSelection,
      cancelSession: hostActions.cancelSession,
      setCursorPoint: inputSetters.setCursorPoint,
      setStartPointWithRef: inputSetters.setStartPointWithRef,
      setSelection: inputSetters.setSelection,
      setHoverSelection: inputSetters.setHoverSelection,
      scheduleSelectionOverlayPaint: overlay.schedulePaint,
      setPreviewImageBase64: inputSetters.setPreviewImageBase64,
      setRenderingOutput: inputSetters.setRenderingOutput,
      setStatus: inputSetters.setStatus,
      setActiveAnnotationTool: inputSetters.setActiveAnnotationTool,
      setAnnotationGesture: inputSetters.setAnnotationGesture,
      setDraftAnnotation: inputSetters.setDraftAnnotation,
      setSelectedAnnotationIndex: inputSetters.setSelectedAnnotationIndex,
      setAnnotationMoveGesture: inputSetters.setAnnotationMoveGesture,
      setDraftSelectionMoveGesture:
        inputSetters.setDraftSelectionMoveGesture,
      setTextDraft: inputSetters.setTextDraft,
      setTextDraftAnnotationIndex: inputSetters.setTextDraftAnnotationIndex,
      setAnnotationHistory: inputSetters.setAnnotationHistory,
      syncHoverSelection,
      renderSelectionPreview: hostActions.renderSelectionPreview,
      completeManualSelection: (rect) => {
        void hostActions.completeManualSelection(rect);
      },
      pinSelection: hostActions.pinSelection,
      setEditGesture: inputSetters.setEditGesture,
      setAnnotationStyle: inputSetters.setAnnotationStyle,
      setTextFontSize: inputSetters.setTextFontSize,
      copySelection: hostActions.copySelection,
      adjustAnnotationSize: editorController.actions.adjustAnnotationSize,
    }),
    [editorController.actions, hostActions, inputSetters, overlay.schedulePaint, syncHoverSelection],
  );
  const pointerContext = useMemo<CaptureWorkspacePointerContext>(
    () => ({
      state: captureWorkspaceState,
      refs: inputRefs,
      derived: pointerDerived,
      actions: pointerActions,
    }),
    [captureWorkspaceState, inputRefs, pointerActions, pointerDerived],
  );
  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      handleCaptureWorkspaceKeyDown(event, {
        state: captureWorkspaceState,
        refs: inputRefs,
        derived: keyboardDerived,
        actions: keyboardActions,
      });
    },
    [captureWorkspaceState, inputRefs, keyboardActions, keyboardDerived],
  );
  const keyboardHostEvents = useMemo(
    () => ({
      isActive: captureWorkspaceState.status !== 'idle',
      status: captureWorkspaceState.status,
      isRenderingOutputRef,
      hasDraftSelectionMoveGesture:
        captureWorkspaceState.draftSelectionMoveGesture !== null,
      onKeyDown,
      onReleaseMagnifierRequest: () =>
        inputSetters.setIsMagnifierRequested(false),
      onFinishDraftSelectionMove: () =>
        inputSetters.setDraftSelectionMoveGesture(null),
      onCancelSession: hostActions.cancelSession,
    }),
    [captureWorkspaceState.draftSelectionMoveGesture, captureWorkspaceState.status, hostActions.cancelSession, inputSetters, onKeyDown],
  );
  const onRootPointerDown = useCallback(
    (event: PointerEvent<HTMLDivElement>) =>
      handleCaptureWorkspacePointerDown(event, pointerContext),
    [pointerContext],
  );
  const onRootPointerMove = useCallback(
    (event: PointerEvent<HTMLDivElement>) =>
      handleCaptureWorkspacePointerMove(event, pointerContext),
    [pointerContext],
  );
  const onRootPointerUp = useCallback(
    (event: PointerEvent<HTMLDivElement>) =>
      handleCaptureWorkspacePointerUp(event, pointerContext),
    [pointerContext],
  );
  const onPreviewPointerDown = useCallback(
    (event: PointerEvent<HTMLDivElement>) =>
      handleCaptureWorkspacePreviewPointerDown(event, pointerContext),
    [pointerContext],
  );
  const onResizeHandlePointerDown = useCallback(
    (handle: SelectionHandle, event: PointerEvent<HTMLButtonElement>) =>
      handleCaptureWorkspaceResizePointerDown(handle, event, pointerContext),
    [pointerContext],
  );
  const onRootWheel = useCallback(
    (event: WheelEvent<HTMLDivElement>) =>
      handleCaptureWorkspaceWheel(event, pointerContext),
    [pointerContext],
  );

  const magnifierSelection =
    workspace.selection ??
    workspace.draftSelectionRef.current ??
    workspace.hoverSelectionRef.current ??
    workspace.hoverSelection;

  return {
    hostWindowReveal,
    hostSubscriptions,
    keyboardHostEvents,
    viewProps: {
      isActive: workspace.status !== 'idle',
      status: workspace.status,
      viewportBounds: derived.viewportBounds,
      error: workspace.error,
      selection: workspace.selection,
      selectionViewportRect: derived.selectionViewportRect,
      previewImageBase64: workspace.previewImageBase64,
      draftAnnotation: workspace.draftAnnotation,
      textDraft: workspace.textDraft,
      textDraftInputRef: editorController.textDraftInputRef,
      annotationStyle: workspace.annotationStyle,
      selectedAnnotationBounds: derived.selectedAnnotationBounds,
      activeAnnotationTool: workspace.activeAnnotationTool,
      toolbarPosition: derived.toolbarPosition,
      toolbarWidth: TOOLBAR_SIZE.width,
      isAnnotationToolbarVisible: workspace.isAnnotationToolbarVisible,
      textFontSize: workspace.textFontSize,
      isTextSizingActive: derived.isTextSizingActive,
      isFillModeActive: derived.isFillModeActive,
      isRenderingOutput: workspace.isRenderingOutput,
      selectionOverlayCanvasRef: overlay.canvasRef,
      selectionOverlayCssSize: overlay.cssSize,
      selectionOverlayPixelRatio: overlay.pixelRatio,
      isMagnifierShown: derived.isMagnifierShown,
      cursorMonitor: derived.cursorMonitor,
      cursorViewportPoint: derived.cursorViewportPoint,
      cursorInMonitorPoint: derived.cursorInMonitorPoint,
      magnifierSelection,
      cursorColor: workspace.cursorColor,
      colorSampleFormat: workspace.colorSampleFormat,
      onRootPointerDown,
      onRootPointerMove,
      onRootPointerUp,
      onRootWheel,
      onPreviewPointerDown,
      onResizeHandlePointerDown,
      onCommitTextDraft: editorController.actions.commitTextDraft,
      onTextDraftTextChange: editorController.actions.updateTextDraftText,
      onDiscardTextDraft: editorController.actions.discardTextDraft,
      onSelectMove: editorController.actions.selectMoveTool,
      onToggleAnnotationTool: editorController.actions.toggleAnnotationTool,
      onApplyAnnotationStyle:
        editorController.actions.applySelectedAnnotationStyle,
      onTextDraftFontSizeChange:
        editorController.actions.updateTextDraftFontSize,
      onCancel: hostActions.cancelSession,
      onRunOcr: hostActions.runOcrSelection,
      onCopy: hostActions.copySelection,
      onSave: hostActions.saveSelection,
      onQuickSave: hostActions.quickSaveSelection,
    },
  };
}
