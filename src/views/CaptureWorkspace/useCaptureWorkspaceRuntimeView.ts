import { shouldRequestCaptureMagnifierPixels } from '../../application/capture-workspace/captureMagnifierState';
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import {
  createCaptureWorkspaceRuntime,
  type CaptureScreenshotPreferences,
} from '../../application/capture-workspace/runtime';
import type { CaptureWorkspaceRuntime } from '../../application/capture-workspace/captureWorkspaceRuntimeTypes';
import type { OcrSettings } from '../../application/settings/ports';
import { prepareCaptureSurfaceForReveal } from './captureHostRuntime';
import {
  useCaptureMagnifierPixelSource,
} from './captureMagnifierRuntime';
import { useCaptureSelectionOverlay } from './captureSelectionOverlayRuntime';
import { captureInterfaceScale } from '../../application/capture-workspace/interfaceScale';
import { getCaptureWorkspaceDerivedState } from '../../application/capture-workspace/captureWorkspaceDerived';
import { ANNOTATION_COLORS, type AnnotationColor } from '../../application/capture-workspace/annotationStyle';
import type {
  CaptureWorkspaceViewActions,
  CaptureWorkspaceViewRenderState,
} from './CaptureWorkspaceView';
import type { CaptureMode, LogicalRect } from './types';
import { virtualPointToViewportPoint } from '../../application/capture-workspace/virtualDesktop';
import { useCaptureWorkspacePorts } from './runtimeContext';

const TOOLBAR_GAP = 14;
const TOOLBAR_SIZE = { width: 700, height: 40 };

interface CaptureWorkspaceRuntimeViewOptions {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
  annotationColorPresets?: readonly AnnotationColor[];
  persistAnnotationColors?: (colors: AnnotationColor[]) => void | Promise<unknown>;
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
  persistAnnotationColors,
  screenshotPreferences,
  persistScreenshotDefaults,
  ocrPreferences,
}: CaptureWorkspaceRuntimeViewOptions): CaptureWorkspaceRuntimeView {
  const platformPorts = useCaptureWorkspacePorts();
  const [fontFamilies, setFontFamilies] = useState<string[]>([]);
  const fontFamiliesRequestRef = useRef<Promise<string[]> | null>(null);
  const [nativeToolbar, setNativeToolbar] = useState(false);
  const toolbarOwnerRef = useRef<string | null>(null);
  const toolbarRequestRef = useRef(0);
  const toolbarWriteRef = useRef(Promise.resolve());
  const onInactiveRef = useRef(onInactive);
  const annotationColorPresetsRef = useRef(annotationColorPresets);
  const persistAnnotationColorsRef = useRef(persistAnnotationColors);
  const screenshotPreferencesRef = useRef(screenshotPreferences);
  const persistScreenshotDefaultsRef = useRef(persistScreenshotDefaults);
  const ocrPreferencesRef = useRef(ocrPreferences);
  const hostBridgeRef = useRef<{
    reset(): void;
    prepareSurface(windowPrepared: Promise<void>): Promise<void>;
    scheduleSelectionOverlayPaint(): void;
  }>({
    reset: () => undefined,
    prepareSurface: async () => undefined,
    scheduleSelectionOverlayPaint: () => undefined,
  });
  onInactiveRef.current = onInactive;
  annotationColorPresetsRef.current = annotationColorPresets;
  persistAnnotationColorsRef.current = persistAnnotationColors;
  screenshotPreferencesRef.current = screenshotPreferences;
  persistScreenshotDefaultsRef.current = persistScreenshotDefaults;
  ocrPreferencesRef.current = ocrPreferences;

  const runtimeLifetimeRef = useRef<{ runtime: CaptureWorkspaceRuntime } | null>(null);
  const workflowRuntime = useMemo(
    () =>
      createCaptureWorkspaceRuntime({
        platform: platformPorts,
        onInactive: () => (
          onInactiveRef.current
            ? onInactiveRef.current()
            : platformPorts.window.hide()
        ),
        annotationColorPresets: () =>
          annotationColorPresetsRef.current ?? ANNOTATION_COLORS,
        persistAnnotationColors: (colors) => persistAnnotationColorsRef.current?.(colors),
        screenshotPreferences: () => screenshotPreferencesRef.current,
        persistScreenshotDefaults: (input) =>
          persistScreenshotDefaultsRef.current?.(input),
        ocrPreferences: () => ocrPreferencesRef.current,
        storage: window.localStorage,
        host: {
          resetInteraction: () => hostBridgeRef.current.reset(),
          resetSession: () => hostBridgeRef.current.reset(),
          prepareSurface: (windowPrepared) => hostBridgeRef.current.prepareSurface(windowPrepared),
          scheduleSelectionOverlayPaint: () =>
            hostBridgeRef.current.scheduleSelectionOverlayPaint(),
        },
        keyboard: {
          target: window,
          isEditingText: (event) => {
            const target = event.target;
            return target instanceof HTMLElement && (
              target.isContentEditable ||
              target.closest('input, textarea, select') !== null
            );
          },
        },
      }),
    [platformPorts],
  );
  const [runtimeRenderState, setRuntimeRenderState] = useState(
    () => workflowRuntime.renderState,
  );
  const toolbarRef = useRef<HTMLDivElement | null>(null);
  const scaleAnchor = runtimeRenderState.selection ?? runtimeRenderState.cursorPoint;
  const scaleMonitor = runtimeRenderState.session?.monitors.find((monitor) => scaleAnchor && scaleAnchor.x >= monitor.logical_bounds.x && scaleAnchor.y >= monitor.logical_bounds.y && scaleAnchor.x < monitor.logical_bounds.x + monitor.logical_bounds.width && scaleAnchor.y < monitor.logical_bounds.y + monitor.logical_bounds.height)
    ?? runtimeRenderState.session?.monitors[0];
  const uiScale = captureInterfaceScale(screenshotPreferences?.captureUiScale, scaleMonitor?.logical_bounds);
  const [toolbarHeight, setToolbarHeight] = useState(TOOLBAR_SIZE.height);
  const [toolbarClientBounds, setToolbarClientBounds] = useState(() => ({
    x: 0,
    y: 0,
    width: document.documentElement.clientWidth,
    height: document.documentElement.clientHeight,
  }));

  useLayoutEffect(() => {
    const updateSize = () => {
      const { clientWidth: width, clientHeight: height } = document.documentElement;
      setToolbarClientBounds((previous) =>
        previous.width === width && previous.height === height
          ? previous
          : { x: 0, y: 0, width, height },
      );
      const toolbarHeight = toolbarRef.current?.getBoundingClientRect().height;
      if (toolbarHeight) setToolbarHeight(toolbarHeight);
    };
    updateSize();
    const observer = new ResizeObserver(updateSize);
    observer.observe(document.documentElement);
    if (toolbarRef.current) observer.observe(toolbarRef.current);
    window.addEventListener('resize', updateSize);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', updateSize);
    };
  }, [runtimeRenderState.status, runtimeRenderState.isAnnotationToolbarVisible, uiScale, nativeToolbar]);

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
        toolbarSize: { width: TOOLBAR_SIZE.width, height: toolbarHeight },
        toolbarClientBounds,
      }),
    [runtimeRenderState, toolbarClientBounds, toolbarHeight],
  );
  const toolbarDark = document.documentElement.dataset.theme === 'dark';
  const selectedType = derived.selectedAnnotation?.type;
  const roundEnabled = ['rectangle', 'region-highlight', 'blur', 'spotlight']
    .some(type => type === runtimeRenderState.activeAnnotationTool || type === selectedType);
  const blurMode = runtimeRenderState.activeAnnotationTool === 'blur' || selectedType === 'blur';
  const styling = useMemo(() => ({
    style: runtimeRenderState.annotationStyle,
    fontSize: runtimeRenderState.textFontSize,
    textMode: derived.isTextSizingActive,
    fillEnabled: derived.isFillModeActive,
    roundEnabled, blurMode,
    colorPresets: annotationColorPresets ?? ANNOTATION_COLORS,
    fontFamilies,
    error: runtimeRenderState.error,
  }), [runtimeRenderState.annotationStyle, runtimeRenderState.textFontSize,
    runtimeRenderState.error, derived.isTextSizingActive, derived.isFillModeActive,
    roundEnabled, blurMode, annotationColorPresets, fontFamilies]);
  const propertyRows = Number(styling.textMode) + Number(styling.roundEnabled) + Number(Boolean(styling.error));
  const nativeToolbarHeight = (
    (derived.toolbarSize.width / uiScale < 460 ? 100 : 68) +
    (propertyRows ? 8 + 36 * propertyRows : 0)
  ) * uiScale;
  useEffect(() => {
    const update = platformPorts.commands.updateCaptureEditorToolbar;
    if (!update) return;
    const request = ++toolbarRequestRef.current;
    const id = runtimeRenderState.sessionId;
    const position = derived.toolbarPosition;
    const presentation = id && position && runtimeRenderState.status === 'preview' &&
      runtimeRenderState.isAnnotationToolbarVisible ? {
        frame: { x: position.x, y: position.y,
          width: derived.toolbarSize.width, height: nativeToolbarHeight },
        uiScale,
        dark: toolbarDark,
        interfaceTheme: document.documentElement.dataset.interfaceTheme ?? 'native',
        visible: true,
        activeTool: runtimeRenderState.activeAnnotationTool,
        canUndo: derived.canUndoAnnotation,
        canRedo: derived.canRedoAnnotation,
        busy: runtimeRenderState.isRenderingOutput,
        styling,
      } : null;
    // Serialize writes and skip superseded frames, including session replacement.
    toolbarWriteRef.current = toolbarWriteRef.current.catch(() => undefined).then(async () => {
      if (request !== toolbarRequestRef.current) return;
      const owner = toolbarOwnerRef.current;
      if (owner && owner !== id) await update(owner, null);
      if (!id) {
        toolbarOwnerRef.current = null;
        setNativeToolbar(false);
        return;
      }
      const enabled = await update(id, presentation);
      if (!enabled && owner === id) await update(id, null);
      toolbarOwnerRef.current = enabled && presentation ? id : null;
      if (request === toolbarRequestRef.current) setNativeToolbar(enabled && !!presentation);
    }).catch(async () => {
      if (request !== toolbarRequestRef.current) return;
      setNativeToolbar(false);
      if (id) await update(id, null).catch(() => undefined);
      toolbarOwnerRef.current = null;
    });
  }, [platformPorts.commands, runtimeRenderState.sessionId, runtimeRenderState.status,
    runtimeRenderState.isAnnotationToolbarVisible, runtimeRenderState.activeAnnotationTool,
    runtimeRenderState.isRenderingOutput, derived.toolbarPosition?.x,
    derived.toolbarPosition?.y, derived.toolbarSize.width, derived.canUndoAnnotation,
    derived.canRedoAnnotation, uiScale, toolbarDark, nativeToolbarHeight, styling]);
  useEffect(() => () => {
    ++toolbarRequestRef.current;
    const update = platformPorts.commands.updateCaptureEditorToolbar;
    toolbarWriteRef.current = toolbarWriteRef.current.catch(() => undefined).then(async () => {
      const owner = toolbarOwnerRef.current;
      toolbarOwnerRef.current = null;
      if (owner && update) await update(owner, null).catch(() => undefined);
    });
  }, [platformPorts.commands]);
  useEffect(() => {
    if (!derived.isTextSizingActive || !platformPorts.commands.listAnnotationFontFamilies) return;
    let active = true;
    fontFamiliesRequestRef.current ??= platformPorts.commands.listAnnotationFontFamilies();
    void fontFamiliesRequestRef.current.then((families) => {
      if (active) setFontFamilies(families);
    }).catch((error) => {
      fontFamiliesRequestRef.current = null;
      console.warn('Failed to load annotation fonts:', error);
    });
    return () => { active = false; };
  }, [derived.isTextSizingActive, platformPorts.commands]);
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
    monitors: runtimeRenderState.session?.monitors ?? [],
    outputPixelScale: runtimeRenderState.session?.output_pixel_scale,
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
    uiScale,
  });

  const isMagnifierEnabled = screenshotPreferences?.showMagnifier ?? false;
  const isMagnifierRequested = shouldRequestCaptureMagnifierPixels({
    enabled: isMagnifierEnabled,
    requested: runtimeRenderState.isMagnifierRequested,
    status: runtimeRenderState.status,
  });
  const isMagnifierShown = isMagnifierRequested && derived.isMagnifierShown;

  const initialSessionRuntimeRef = useRef<CaptureWorkspaceRuntime | null>(null);
  const desktopRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!initialMode || initialSessionRuntimeRef.current === workflowRuntime) {
      return;
    }
    initialSessionRuntimeRef.current = workflowRuntime;
    void workflowRuntime.actions.startSession(initialMode, initialSessionId);
  }, [initialMode, initialSessionId, workflowRuntime]);

  useEffect(() => {
    if (!runtimeRenderState.sessionId) return;
    void workflowRuntime.actions.hydrateSnapshots().catch(() => undefined);
  }, [runtimeRenderState.sessionId, workflowRuntime]);

  const prepareCaptureSurface = useCallback(
    (windowPrepared: Promise<void>) =>
      prepareCaptureSurfaceForReveal({
        windowPrepared,
        images: runtimeRenderState.status === 'error'
          ? []
          : Array.from(desktopRef.current?.querySelectorAll('img') ?? []),
        frame: overlay.getCurrentFrame(),
        paintSelectionOverlayFrame: overlay.paintFrame,
      }),
    [overlay.getCurrentFrame, overlay.paintFrame, runtimeRenderState.status],
  );
  hostBridgeRef.current = {
    reset: overlay.reset,
    prepareSurface: prepareCaptureSurface,
    scheduleSelectionOverlayPaint: overlay.schedulePaint,
  };

  useEffect(() => {
    const lifetime = { runtime: workflowRuntime };
    runtimeLifetimeRef.current = lifetime;
    let closed = false;
    let disconnect: (() => void) | undefined;
    void workflowRuntime.actions.connectHost(() =>
      closed ? undefined : platformPorts.window.signalReady(),
    ).then((cleanup) => {
      if (closed) {
        cleanup();
        return;
      }
      disconnect = cleanup;
    });
    return () => {
      closed = true;
      disconnect?.();
      // StrictMode immediately reconnects the same view. Only a real unmount
      // should cancel the native session shared by those effect setups.
      queueMicrotask(() => {
        if (
          runtimeLifetimeRef.current === lifetime ||
          runtimeLifetimeRef.current?.runtime !== workflowRuntime
        ) workflowRuntime.dispose();
      });
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
    desktopRef,
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
      monitors: runtimeRenderState.session?.monitors ?? [],
      outputPixelScale: runtimeRenderState.session?.output_pixel_scale ?? null,
      capturedCursor: runtimeRenderState.session?.captured_cursor,
      includeCapturedCursor: runtimeRenderState.includeCapturedCursor,
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
        annotations: derived.annotations.filter(
          (_, index) =>
            index !== runtimeRenderState.annotationMoveGesture?.annotationIndex &&
            index !== runtimeRenderState.textDraftAnnotationIndex,
        ),
        draftAnnotation: runtimeRenderState.draftAnnotation,
        textDraft: runtimeRenderState.textDraft,
        annotationStyle: runtimeRenderState.annotationStyle,
        selectedAnnotationBounds: derived.selectedAnnotationBounds,
        selectedAnnotation: derived.selectedAnnotation,
        activeAnnotationTool: runtimeRenderState.activeAnnotationTool,
      },
      toolbar: {
        nativeControls: nativeToolbar,
        height: nativeToolbarHeight,
        fontFamilies,
        uiScale,
        position: derived.toolbarPosition,
        width: derived.toolbarSize.width,
        isVisible: runtimeRenderState.isAnnotationToolbarVisible,
        textFontSize: runtimeRenderState.textFontSize,
        isTextSizingActive: derived.isTextSizingActive,
        isFillModeActive: derived.isFillModeActive,
        canUndo: derived.canUndoAnnotation,
        canRedo: derived.canRedoAnnotation,
      },
      dom: {
        desktopRef,
        toolbarRef,
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
      fontFamilies,
      uiScale,
      nativeToolbar,
      nativeToolbarHeight,
    ],
  );
  const actions = useMemo<CaptureWorkspaceViewActions>(
    () => ({
      pointerDown: workflowRuntime.actions.pointerDown,
      beginScrollingCapture: workflowRuntime.actions.beginScrollingCapture,
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
      clearAnnotations: workflowRuntime.actions.clearAnnotations,
      cancelSession: workflowRuntime.actions.cancelSession,
      completePreviewSelection:
        workflowRuntime.actions.completePreviewSelection,
    }),
    [workflowRuntime],
  );

  return { renderState, actions };
}
