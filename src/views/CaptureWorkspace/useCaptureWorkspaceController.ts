import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent,
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
import type { CaptureWorkspaceKeyboardRefs } from './captureWorkspaceKeyboard';
import { planManualSelectionCompletion } from './captureInteractionRuntime';
import {
  getCaptureWorkspacePointerPoint,
} from './captureWorkspacePointer';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import { useCaptureWorkspaceEditorController } from './useCaptureWorkspaceEditorController';
import { useCaptureWorkspaceState } from './useCaptureWorkspaceState';
import { getCurrentMonitorBounds } from './virtualDesktop';
import type {
  AnnotationCommand,
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
  const hostBridgeRef = useRef<{
    resetInteraction(): void;
    resetSession(): void;
    applyManualSelection(rect: LogicalRect, mode: CaptureMode): void;
    getAnnotations(): AnnotationCommand[];
    commitTextDraft(): AnnotationCommand[];
    shouldIncludeCursor(): boolean;
    hasTextDraft(): boolean;
    prepareSurface(): Promise<void>;
    getSnapTargetRects(): LogicalRect[];
  }>({
    resetInteraction: () => undefined,
    resetSession: () => undefined,
    applyManualSelection: (_rect: LogicalRect, _mode: CaptureMode) => undefined,
    getAnnotations: (): AnnotationCommand[] => [],
    commitTextDraft: (): AnnotationCommand[] => [],
    shouldIncludeCursor: () => false,
    hasTextDraft: () => false,
    prepareSurface: async () => undefined,
    getSnapTargetRects: () => [],
  });
  const keyboardBridgeRef = useRef<{
    onUnhandledKeyDown(event: KeyboardEvent): void;
    releaseMagnifierRequest(): void;
    hasDraftSelectionMoveGesture(): boolean;
    finishDraftSelectionMove(): void;
    hasDismissibleLayer(): boolean;
  }>({
    onUnhandledKeyDown: (_event: KeyboardEvent) => undefined,
    releaseMagnifierRequest: () => undefined,
    hasDraftSelectionMoveGesture: () => false,
    finishDraftSelectionMove: () => undefined,
    hasDismissibleLayer: () => false,
  });
  onInactiveRef.current = onInactive;
  screenshotSavePathRef.current = screenshotSavePath;
  const workflowRuntime = useMemo(
    () =>
      createCaptureWorkspaceRuntime({
        platform: runtime,
        onInactive: () => onInactiveRef.current?.(),
        screenshotSavePath: () => screenshotSavePathRef.current,
        storage: window.localStorage,
        host: {
          resetInteraction: () => hostBridgeRef.current.resetInteraction(),
          resetSession: () => hostBridgeRef.current.resetSession(),
          applyManualSelection: (rect, mode) =>
            hostBridgeRef.current.applyManualSelection(rect, mode),
          getAnnotations: () => hostBridgeRef.current.getAnnotations(),
          commitTextDraft: () => hostBridgeRef.current.commitTextDraft(),
          shouldIncludeCursor: () =>
            hostBridgeRef.current.shouldIncludeCursor(),
          hasTextDraft: () => hostBridgeRef.current.hasTextDraft(),
          prepareSurface: () => hostBridgeRef.current.prepareSurface(),
          getSnapTargetRects: () =>
            hostBridgeRef.current.getSnapTargetRects(),
        },
        keyboard: {
          target: window,
          onUnhandledKeyDown: (event) =>
            keyboardBridgeRef.current.onUnhandledKeyDown(event),
          releaseMagnifierRequest: () =>
            keyboardBridgeRef.current.releaseMagnifierRequest(),
          hasDraftSelectionMoveGesture: () =>
            keyboardBridgeRef.current.hasDraftSelectionMoveGesture(),
          finishDraftSelectionMove: () =>
            keyboardBridgeRef.current.finishDraftSelectionMove(),
          hasDismissibleLayer: () =>
            keyboardBridgeRef.current.hasDismissibleLayer(),
        },
      }),
    [runtime],
  );
  const keyboardDraftCursorPointRef = useRef<Point | null>(null);
  const keyboardEditCursorPointRef = useRef<Point | null>(null);
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
        if (next.status === 'idle' && !next.session) {
          workspace.resetSession();
          setHydratedCaptureSessionId(null);
          return;
        }
        workspace.applyPatch({
          status: next.status,
          mode: next.mode,
          session: next.session,
          startPoint: next.startPoint,
          cursorPoint: next.cursorPoint,
          selection: next.selection,
          hoverSelection: next.hoverSelection,
          previewImageBase64: next.previewImageBase64,
          includeCapturedCursor: next.includeCapturedCursor,
          isRenderingOutput: next.isRenderingOutput,
          error: next.error,
        });
        setHydratedCaptureSessionId(
          next.hasHydratedPixelSource ? next.sessionId : null,
        );
      }),
    [workflowRuntime, workspace.applyPatch, workspace.resetSession],
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
  const ensureCaptureSnapshotsHydrated = useCallback(
    async (_sessionId: string) => {
      await workflowRuntime.actions.hydrateSnapshots();
      return workflowRuntime.renderState.session!;
    },
    [workflowRuntime],
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
    (rect: LogicalRect) => workflowRuntime.actions.completeManualSelection(rect),
    [workflowRuntime],
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
    resetInteraction: () => {
      workspace.resetInteraction();
      overlay.reset();
      setHydratedCaptureSessionId(null);
    },
    resetSession: workspace.resetSession,
    applyManualSelection: (rect, mode) => {
      const transition = planCaptureManualSelectionTransition({
        rect,
        completion: planManualSelectionCompletion(mode),
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
    },
    getAnnotations: () => captureWorkspaceState.annotationHistory.annotations,
    commitTextDraft: () => commitTextDraftToHistory().annotations,
    shouldIncludeCursor: () =>
      captureWorkspaceState.includeCapturedCursor &&
      canToggleCapturedCursor(captureWorkspaceState.session),
    hasTextDraft: () => captureWorkspaceState.textDraft !== null,
    prepareSurface: prepareCaptureSurface,
    getSnapTargetRects: () => derived.snapTargetRects,
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

  const hostActions = useMemo(
    () => ({
      ensureCaptureSnapshotsHydrated,
      cancelSession,
      refreshSession: workflowRuntime.actions.refreshSession,
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
    [cancelSession, completeCandidateSelection, completeManualSelection, completePreviewSelection, copySelection, ensureCaptureSnapshotsHydrated, pinSelection, quickSaveSelection, renderSelectionPreview, resetPreviewSelection, restoreLastSelection, restoreSelectionFromHistory, runOcrSelection, saveSelection, selectFullCaptureArea, workflowRuntime.actions.refreshSession],
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
  const editorSetters = useMemo(
    () => ({
      setStatus: workspace.setStatus,
      setError: workspace.setError,
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
      workspace.setError,
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
  const editorController = useCaptureWorkspaceEditorController({
    state: captureWorkspaceState,
    derived: editorDerived,
    setters: editorSetters,
    host: editorHost,
    runtime,
    input: {
      refs: inputRefs,
      derived: inputDerived,
      setters: editorSetters,
      scheduleSelectionOverlayPaint: overlay.schedulePaint,
      syncHoverSelection,
    },
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

  const editorInput = editorController.input;
  keyboardBridgeRef.current = {
    onUnhandledKeyDown: editorInput.onUnhandledKeyDown,
    releaseMagnifierRequest: () =>
      editorSetters.setIsMagnifierRequested(false),
    hasDraftSelectionMoveGesture: () =>
      captureWorkspaceState.draftSelectionMoveGesture !== null,
    finishDraftSelectionMove: () =>
      editorSetters.setDraftSelectionMoveGesture(null),
    hasDismissibleLayer: () =>
      captureWorkspaceState.textDraft !== null ||
      captureWorkspaceState.annotationGesture !== null ||
      captureWorkspaceState.annotationMoveGesture !== null ||
      captureWorkspaceState.draftSelectionMoveGesture !== null ||
      captureWorkspaceState.selectedAnnotationIndex !== null ||
      captureWorkspaceState.activeAnnotationTool !== null,
  };
  const onRootPointerDown = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (derived.selectionBounds) {
        event.currentTarget.setPointerCapture(event.pointerId);
        const handled = workflowRuntime.actions.pointerDown({
          point: getCaptureWorkspacePointerPoint(event, derived.selectionBounds),
          button: event.button,
          shiftKey: event.shiftKey,
          source: 'root',
        });
        if (handled) return;
      }
      editorInput.onRootPointerDown(event);
    },
    [derived.selectionBounds, editorInput, workflowRuntime],
  );
  const onRootPointerMove = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (derived.selectionBounds) {
        const handled = workflowRuntime.actions.pointerMove({
          point: getCaptureWorkspacePointerPoint(event, derived.selectionBounds),
          button: event.button,
          shiftKey: event.shiftKey,
          source: 'root',
        });
        if (handled) return;
      }
      editorInput.onRootPointerMove(event);
    },
    [derived.selectionBounds, editorInput, workflowRuntime],
  );
  const onRootPointerUp = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (derived.selectionBounds) {
        void workflowRuntime.actions.pointerUp({
          point: getCaptureWorkspacePointerPoint(event, derived.selectionBounds),
          button: event.button,
          shiftKey: event.shiftKey,
          source: 'root',
        }).then((handled) => {
          if (!handled) editorInput.onRootPointerUp(event);
        });
        return;
      }
      editorInput.onRootPointerUp(event);
    },
    [derived.selectionBounds, editorInput, workflowRuntime],
  );
  const onPreviewPointerDown = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (
        derived.selectionBounds &&
        workflowRuntime.actions.pointerDown({
          point: getCaptureWorkspacePointerPoint(event, derived.selectionBounds),
          button: event.button,
          shiftKey: event.shiftKey,
          source: 'preview',
        })
      ) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }
      editorInput.onPreviewPointerDown(event);
    },
    [derived.selectionBounds, editorInput, workflowRuntime],
  );

  const magnifierSelection =
    workspace.selection ??
    workspace.draftSelectionRef.current ??
    workspace.hoverSelectionRef.current ??
    workspace.hoverSelection;

  return {
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
      onRootWheel: editorInput.onRootWheel,
      onPreviewPointerDown,
      onResizeHandlePointerDown: editorInput.onResizeHandlePointerDown,
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
