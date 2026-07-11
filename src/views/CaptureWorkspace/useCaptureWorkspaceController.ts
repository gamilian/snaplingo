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
import { prepareCaptureSurfaceForReveal } from './captureHostRuntime';
import {
  shouldPollCaptureHoverSelection,
  startCaptureHoverSelectionPolling,
} from './captureHoverPolling';
import { useCaptureMagnifierPixelSource } from './captureMagnifierRuntime';
import { useCaptureSelectionOverlay } from './captureSelectionOverlayRuntime';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import { getCaptureWorkspacePointerPoint } from './captureWorkspacePointer';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import { useCaptureWorkspaceState } from './useCaptureWorkspaceState';
import type { CaptureMode } from './types';
import { useCaptureWorkspaceRuntime } from './runtimeContext';

const TOOLBAR_GAP = 14;
const TOOLBAR_SIZE = { width: 640, height: 42 };
const CAPTURE_HOVER_POLL_INTERVAL_MS = 16;

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
    prepareSurface(): Promise<void>;
    scheduleSelectionOverlayPaint(): void;
  }>({
    resetInteraction: () => undefined,
    resetSession: () => undefined,
    prepareSurface: async () => undefined,
    scheduleSelectionOverlayPaint: () => undefined,
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
          prepareSurface: () => hostBridgeRef.current.prepareSurface(),
          scheduleSelectionOverlayPaint: () =>
            hostBridgeRef.current.scheduleSelectionOverlayPaint(),
        },
        keyboard: {
          target: window,
        },
      }),
    [runtime],
  );
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
          editGesture: next.editGesture,
          activeAnnotationTool: next.activeAnnotationTool,
          annotationGesture: next.annotationGesture,
          draftAnnotation: next.draftAnnotation,
          selectedAnnotationIndex: next.selectedAnnotationIndex,
          annotationMoveGesture: next.annotationMoveGesture,
          draftSelectionMoveGesture: next.draftSelectionMoveGesture,
          textDraft: next.textDraft,
          textDraftAnnotationIndex: next.textDraftAnnotationIndex,
          annotationStyle: next.annotationStyle,
          textFontSize: next.textFontSize,
          annotationHistory: next.annotationHistory,
          isAnnotationToolbarVisible: next.isAnnotationToolbarVisible,
          cursorColor: next.cursorColor,
          colorSampleFormat: next.colorSampleFormat,
          isMagnifierRequested: next.isMagnifierRequested,
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
    workspace.editGesture,
    workspace.session,
    workspace.startPoint,
    workspace.startPointRef,
    workspace.status,
    workflowRuntime,
  ]);

  const hasStartedInitialSessionRef = useRef(false);
  const ensureCaptureSnapshotsHydrated = useCallback(
    async (_sessionId: string) => {
      await workflowRuntime.actions.hydrateSnapshots();
      return workflowRuntime.renderState.session!;
    },
    [workflowRuntime],
  );
  const cancelSession = workflowRuntime.actions.cancelSession;
  const completePreviewSelection = useCallback(
    async (
      action: Parameters<
        typeof workflowRuntime.actions.completePreviewSelection
      >[0],
      guardCompletion = false,
    ) => {
      if (!captureWorkspaceState.selection) return;
      if (guardCompletion && isRenderingOutputRef.current) return;

      await workflowRuntime.actions.completePreviewSelection(
        action,
        captureWorkspaceState.selection,
      );
    },
    [captureWorkspaceState.selection, workflowRuntime],
  );
  const copySelection = useCallback(
    () => completePreviewSelection('copy', true),
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
    () => completePreviewSelection('ocr'),
    [completePreviewSelection],
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

  const textDraftInputRef = useRef<HTMLTextAreaElement | null>(null);
  useEffect(() => {
    if (!workspace.textDraft) return;
    requestAnimationFrame(() => textDraftInputRef.current?.focus());
  }, [workspace.textDraft]);

  useCaptureMagnifierPixelSource({
    session: workspace.session,
    hasHydratedPixelSource: derived.hasHydratedPixelSource,
    isMagnifierRequested: workspace.isMagnifierRequested,
    isMagnifierShown: derived.isMagnifierShown,
    cursorMonitor: derived.cursorMonitor,
    cursorInMonitorPoint: derived.cursorInMonitorPoint,
    setCursorColor: workflowRuntime.actions.updateCursorColor,
    ensureCaptureSnapshotsHydrated,
  });

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
        if (handled) {
          event.preventDefault();
          return;
        }
      }
    },
    [derived.selectionBounds, workflowRuntime],
  );
  const onRootPointerMove = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (derived.selectionBounds) {
        workflowRuntime.actions.pointerMove({
          point: getCaptureWorkspacePointerPoint(event, derived.selectionBounds),
          button: event.button,
          shiftKey: event.shiftKey,
          source: 'root',
        });
      }
    },
    [derived.selectionBounds, workflowRuntime],
  );
  const onRootPointerUp = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (derived.selectionBounds) {
        void workflowRuntime.actions.pointerUp({
          point: getCaptureWorkspacePointerPoint(event, derived.selectionBounds),
          button: event.button,
          shiftKey: event.shiftKey,
          source: 'root',
        });
      }
    },
    [derived.selectionBounds, workflowRuntime],
  );
  const onPreviewPointerDown = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (
        derived.selectionBounds &&
        workflowRuntime.actions.pointerDown({
          point: getCaptureWorkspacePointerPoint(event, derived.selectionBounds),
          button: event.button,
          detail: event.detail,
          metaKey: event.metaKey,
          ctrlKey: event.ctrlKey,
          altKey: event.altKey,
          shiftKey: event.shiftKey,
          source: 'preview',
        })
      ) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }
    },
    [derived.selectionBounds, workflowRuntime],
  );
  const onResizeHandlePointerDown = useCallback(
    (
      handle: Parameters<typeof workflowRuntime.actions.resizePointerDown>[0],
      event: PointerEvent<HTMLButtonElement>,
    ) => {
      if (!derived.selectionBounds) return;
      if (
        workflowRuntime.actions.resizePointerDown(handle, {
          point: getCaptureWorkspacePointerPoint(event, derived.selectionBounds),
          button: event.button,
          shiftKey: event.shiftKey,
          source: 'preview',
        })
      ) {
        event.preventDefault();
        event.stopPropagation();
      }
    },
    [derived.selectionBounds, workflowRuntime],
  );
  const onRootWheel = useCallback(
    (event: WheelEvent<HTMLDivElement>) => {
      if (
        workflowRuntime.actions.wheel({
          deltaY: event.deltaY,
          metaKey: event.metaKey,
          ctrlKey: event.ctrlKey,
          altKey: event.altKey,
        })
      ) {
        event.preventDefault();
      }
    },
    [workflowRuntime],
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
      textDraftInputRef,
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
      onCommitTextDraft: workflowRuntime.actions.commitTextDraft,
      onTextDraftTextChange: workflowRuntime.actions.updateTextDraftText,
      onDiscardTextDraft: workflowRuntime.actions.discardTextDraft,
      onSelectMove: workflowRuntime.actions.selectMoveTool,
      onToggleAnnotationTool: workflowRuntime.actions.toggleAnnotationTool,
      onApplyAnnotationStyle:
        workflowRuntime.actions.applySelectedAnnotationStyle,
      onTextDraftFontSizeChange:
        workflowRuntime.actions.updateTextDraftFontSize,
      onCancel: cancelSession,
      onRunOcr: runOcrSelection,
      onCopy: copySelection,
      onSave: saveSelection,
      onQuickSave: quickSaveSelection,
    },
  };
}
