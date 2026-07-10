import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import type { AnnotationHistory } from './annotationHistory';
import {
  shouldPollCaptureHoverSelection,
  startCaptureHoverSelectionPolling,
} from './captureHoverPolling';
import { useCaptureMagnifierPixelSource } from './captureMagnifierRuntime';
import { useCaptureSelectionOverlay } from './captureSelectionOverlayRuntime';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import type { CaptureWorkspaceKeyboardRefs } from './captureWorkspaceKeyboard';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import { useCaptureWorkspaceEditorController } from './useCaptureWorkspaceEditorController';
import { useCaptureWorkspaceHostController } from './useCaptureWorkspaceHostController';
import { useCaptureWorkspaceInputController } from './useCaptureWorkspaceInputController';
import { useCaptureWorkspaceState } from './useCaptureWorkspaceState';
import { currentCaptureCursorPosition } from '../../tauri/captureSession';
import type { CaptureMode, LogicalRect, Point } from './types';

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
      getCursorPosition: currentCaptureCursorPosition,
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
  const hostWorkspace = useMemo(
    () => ({
      state: captureWorkspaceState,
      applyPatch: workspace.applyPatch,
      resetInteraction: workspace.resetInteraction,
      resetSession: workspace.resetSession,
      resetPreview: workspace.resetPreview,
      draftSelectionRef: workspace.draftSelectionRef,
      startPointRef: workspace.startPointRef,
      cursorPointRef: workspace.cursorPointRef,
      hoverSelectionRef: workspace.hoverSelectionRef,
    }),
    [
      captureWorkspaceState,
      workspace.applyPatch,
      workspace.cursorPointRef,
      workspace.draftSelectionRef,
      workspace.hoverSelectionRef,
      workspace.resetInteraction,
      workspace.resetPreview,
      workspace.resetSession,
      workspace.startPointRef,
    ],
  );
  const hostDerived = useMemo(
    () => ({
      areCaptureImagesReady: derived.areCaptureImagesReady,
      selectionBounds: derived.selectionBounds,
    }),
    [derived.areCaptureImagesReady, derived.selectionBounds],
  );
  const hostOverlay = useMemo(
    () => ({
      reset: overlay.reset,
      getCurrentFrame: overlay.getCurrentFrame,
      paintFrame: overlay.paintFrame,
    }),
    [overlay.getCurrentFrame, overlay.paintFrame, overlay.reset],
  );
  const hostController = useCaptureWorkspaceHostController({
    initialMode,
    initialSessionId,
    onInactive,
    screenshotSavePath,
    minSelectionSize: MIN_SELECTION_SIZE,
    workspace: hostWorkspace,
    derived: hostDerived,
    overlay: hostOverlay,
    setHydratedCaptureSessionId,
    commitTextDraftToHistory,
  });

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
      renderSelectionPreview:
        hostController.actions.renderSelectionPreview,
      cancelSession: hostController.actions.cancelSession,
    }),
    [
      hostController.actions.cancelSession,
      hostController.actions.renderSelectionPreview,
    ],
  );
  const editorController = useCaptureWorkspaceEditorController({
    state: captureWorkspaceState,
    derived: editorDerived,
    setters: editorSetters,
    host: editorHost,
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
    ensureCaptureSnapshotsHydrated:
      hostController.actions.ensureCaptureSnapshotsHydrated,
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
  const inputController = useCaptureWorkspaceInputController({
    state: captureWorkspaceState,
    refs: inputRefs,
    derived: inputDerived,
    setters: inputSetters,
    host: hostController.actions,
    editor: editorController.actions,
    isRenderingOutputRef,
    scheduleSelectionOverlayPaint: overlay.schedulePaint,
    syncHoverSelection,
  });

  const magnifierSelection =
    workspace.selection ??
    workspace.draftSelectionRef.current ??
    workspace.hoverSelectionRef.current ??
    workspace.hoverSelection;

  return {
    hostWindowReveal: hostController.hostWindowReveal,
    hostSubscriptions: hostController.hostSubscriptions,
    keyboardHostEvents: inputController.keyboardHostEvents,
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
      ...inputController.pointerHandlers,
      onCommitTextDraft: editorController.actions.commitTextDraft,
      onTextDraftTextChange: editorController.actions.updateTextDraftText,
      onDiscardTextDraft: editorController.actions.discardTextDraft,
      onSelectMove: editorController.actions.selectMoveTool,
      onToggleAnnotationTool: editorController.actions.toggleAnnotationTool,
      onApplyAnnotationStyle:
        editorController.actions.applySelectedAnnotationStyle,
      onTextDraftFontSizeChange:
        editorController.actions.updateTextDraftFontSize,
      onCancel: hostController.actions.cancelSession,
      onRunOcr: hostController.actions.runOcrSelection,
      onCopy: hostController.actions.copySelection,
      onSave: hostController.actions.saveSelection,
      onQuickSave: hostController.actions.quickSaveSelection,
    },
  };
}
