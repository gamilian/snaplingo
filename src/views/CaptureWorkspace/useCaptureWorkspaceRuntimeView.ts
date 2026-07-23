import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import {
  createCaptureWorkspaceRuntime,
  type CaptureScreenshotPreferences,
} from './captureWorkspaceRuntime';
import type { CaptureWorkspaceRuntime } from './captureWorkspaceRuntimeTypes';
import type { OcrSettings } from '../../application/settings/ports';
import { prepareCaptureSurfaceForReveal } from './captureHostRuntime';
import {
  shouldRequestCaptureMagnifierPixels,
  useCaptureMagnifierPixelSource,
} from './captureMagnifierRuntime';
import { useCaptureSelectionOverlay } from './captureSelectionOverlayRuntime';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import { ANNOTATION_COLORS, type AnnotationColor } from './annotationStyle';
import type {
  CaptureWorkspaceViewActions,
  CaptureWorkspaceViewRenderState,
} from './CaptureWorkspaceView';
import type { CaptureMode, LogicalRect } from './types';
import { virtualPointToViewportPoint } from './virtualDesktop';
import { useCaptureWorkspaceRuntime } from './runtimeContext';

const TOOLBAR_GAP = 14;
const TOOLBAR_SIZE = { width: 700, height: 42 };

interface CaptureWorkspaceRuntimeViewOptions {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
  annotationColorPresets?: readonly AnnotationColor[];
  screenshotPreferences?: CaptureScreenshotPreferences;
  persistScreenshotDefaults?: (
    input: Partial<
      Pick<CaptureScreenshotPreferences, 'defaultStrokeWidth' | 'defaultFontSize'>
    >,
  ) => void;
  ocrPreferences?: OcrSettings;
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
  screenshotPreferences,
  persistScreenshotDefaults,
  ocrPreferences,
}: CaptureWorkspaceRuntimeViewOptions): CaptureWorkspaceRuntimeView {
  const platformRuntime = useCaptureWorkspaceRuntime();
  const onInactiveRef = useRef(onInactive);
  const annotationColorPresetsRef = useRef(annotationColorPresets);
  const screenshotPreferencesRef = useRef(screenshotPreferences);
  const persistScreenshotDefaultsRef = useRef(persistScreenshotDefaults);
  const ocrPreferencesRef = useRef(ocrPreferences);
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
  screenshotPreferencesRef.current = screenshotPreferences;
  persistScreenshotDefaultsRef.current = persistScreenshotDefaults;
  ocrPreferencesRef.current = ocrPreferences;

  const [runtimeRevision, setRuntimeRevision] = useState(0);
  const disposedRuntimeRef = useRef<CaptureWorkspaceRuntime | null>(null);
  const workflowRuntime = useMemo(
    () =>
      createCaptureWorkspaceRuntime({
        platform: platformRuntime,
        onInactive: () => onInactiveRef.current?.(),
        annotationColorPresets: () =>
          annotationColorPresetsRef.current ?? ANNOTATION_COLORS,
        screenshotPreferences: () => screenshotPreferencesRef.current,
        persistScreenshotDefaults: (input) =>
          persistScreenshotDefaultsRef.current?.(input),
        ocrPreferences: () => ocrPreferencesRef.current,
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
  const draftSelectionRef = useRef<LogicalRect | null>(null);
  const hoverSelectionRef = useRef(runtimeRenderState.hoverSelection);
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
    draftSelectionRef,
    hoverSelectionRef,
    showSelectionSize: screenshotPreferences?.showSelectionSize ?? true,
    selectionBorderWidth: screenshotPreferences?.selectionBorderWidth,
    selectionBorderColor: screenshotPreferences?.selectionBorderColor,
    selectionMaskColor: screenshotPreferences?.selectionMaskColor,
  });

  const isMagnifierEnabled = screenshotPreferences?.showMagnifier ?? false;
  const isMagnifierRequested = shouldRequestCaptureMagnifierPixels({
    enabled: isMagnifierEnabled,
    requested: runtimeRenderState.isMagnifierRequested,
    status: runtimeRenderState.status,
  });
  const isMagnifierShown = isMagnifierRequested && derived.isMagnifierShown;

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

  const magnifierSourceImage = useCaptureMagnifierPixelSource({
    session: runtimeRenderState.session,
    isMagnifierRequested,
    isMagnifierShown,
    cursorMonitor: derived.cursorMonitor,
    cursorInMonitorPoint: derived.cursorInMonitorPoint,
    setCursorColor: workflowRuntime.actions.updateCursorColor,
    ensureCaptureMonitorHydrated: (_sessionId, monitorId) =>
      workflowRuntime.actions.hydrateMagnifierMonitor(monitorId),
  });

  const textDraftInputRef = useRef<HTMLTextAreaElement | null>(null);
  useEffect(() => {
    if (!runtimeRenderState.textDraft) return;
    requestAnimationFrame(() => textDraftInputRef.current?.focus());
  }, [runtimeRenderState.textDraft]);

  const renderState = useMemo<CaptureWorkspaceViewRenderState>(
    () => ({
      status: runtimeRenderState.status,
      candidateDetectionMode: runtimeRenderState.candidateDetectionMode,
      error: runtimeRenderState.error,
      viewportBounds: derived.viewportBounds,
      selectionBounds: derived.selectionBounds,
      isRenderingOutput: runtimeRenderState.isRenderingOutput,
      silentOcrHint:
        runtimeRenderState.silentOcrHint && derived.selectionBounds
          ? {
              status: runtimeRenderState.silentOcrHint.status,
              point: virtualPointToViewportPoint(
                runtimeRenderState.silentOcrHint.point,
                derived.selectionBounds,
              ),
            }
          : null,
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
        isShown: isMagnifierShown,
        cursorMonitor: derived.cursorMonitor,
        cursorViewportPoint: derived.cursorViewportPoint,
        cursorScreenPoint: runtimeRenderState.cursorPoint,
        cursorInMonitorPoint: derived.cursorInMonitorPoint,
        cursorColor: runtimeRenderState.cursorColor,
        colorSampleFormat: runtimeRenderState.colorSampleFormat,
        sourceImage: magnifierSourceImage,
        zoom: screenshotPreferences?.magnifierZoom ?? 12,
      },
    }),
    [
      derived,
      overlay.canvasRef,
      overlay.cssSize,
      overlay.pixelRatio,
      isMagnifierShown,
      magnifierSourceImage,
      runtimeRenderState,
      screenshotPreferences?.magnifierZoom,
    ],
  );
  const actions = useMemo<CaptureWorkspaceViewActions>(
    () => ({
      pointerDown: workflowRuntime.actions.pointerDown,
      pointerMove: workflowRuntime.actions.pointerMove,
      pointerUp: workflowRuntime.actions.pointerUp,
      pointerCancel: workflowRuntime.actions.pointerCancel,
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
      commitAnnotationSizeDefault:
        workflowRuntime.actions.commitAnnotationSizeDefault,
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
