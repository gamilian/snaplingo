import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  getToolbarPosition,
  constrainSelectionPoint,
  moveDraftSelectionByDelta,
  moveSelectionByDelta,
  normalizeSelection,
  nudgeSelection,
  resizeSelectionByHandle,
  snapMovedSelectionToRects,
  snapPointToRects,
  snapResizedSelectionToRects,
  type ArrowKey,
  type SelectionHandle,
} from './selection';
import {
  getMagnifierImageStyle,
  getMagnifierPosition,
} from './magnifier';
import {
  colorSampleToClipboardText,
  type ColorSample,
  isColorSampleCopyShortcut,
  sampleCanvasColor,
} from './colorSampler';
import {
  buildCaptureCandidates,
  getBestCandidateAtPoint,
} from './captureCandidates';
import {
  addAnnotationToHistory,
  emptyAnnotationHistory,
  removeAnnotationFromHistory,
  replaceAnnotationInHistory,
  redoAnnotationHistory,
  undoAnnotationHistory,
} from './annotationHistory';
import {
  getAnnotationKeyboardNudgeDelta,
  getAnnotationBounds,
  hitTestAnnotations,
  moveAnnotationByDelta,
} from './annotationGeometry';
import {
  ANNOTATION_COLORS,
  DEFAULT_TEXT_FONT_SIZE,
  DEFAULT_ANNOTATION_STYLE,
  MAX_ANNOTATION_STROKE_WIDTH,
  MAX_TEXT_FONT_SIZE,
  MIN_ANNOTATION_STROKE_WIDTH,
  MIN_TEXT_FONT_SIZE,
  applyAnnotationStyle,
  annotationColorFromShortcut,
  annotationSizeDirectionFromShortcut,
  annotationToolFromShortcut,
  annotationColorToCss,
  constrainAnnotationGesturePoint,
  annotationFromGesture,
  arrowHeadPoints,
  isCommittedAnnotation,
  nextAnnotationStrokeWidth,
  nextTextFontSize,
  type AnnotationColor,
  type AnnotationStyle,
  type AnnotationSizeDirection,
  type AnnotationTool,
} from './annotationStyle';
import {
  commitTextAnnotationDraft,
  startTextAnnotationDraftFromAnnotation,
  startTextAnnotationDraft,
  updateTextAnnotationDraft,
  type TextAnnotationDraft,
} from './textAnnotationDraft';
import {
  isCancelCapturePointer,
  isCopyCaptureDoubleClick,
  isPinCaptureShortcut,
  isSaveCaptureShortcut,
  isToggleToolbarShortcut,
  saveCaptureSelection,
} from './captureActions';
import { parseCaptureLaunchPayload } from './windowMode';
import {
  getMonitorAtVirtualPoint,
  getMonitorViewportRect,
  getVirtualDesktopBounds,
  viewportPointToVirtualPoint,
  virtualPointToViewportPoint,
  virtualRectToViewportRect,
} from './virtualDesktop';
import type {
  AnnotationCommand,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  MonitorSnapshotView,
  OcrResult,
  Point,
} from './types';

type SessionStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
type EditGesture =
  | {
      type: 'move';
      startPoint: Point;
      startSelection: LogicalRect;
    }
  | {
      type: 'resize';
      handle: SelectionHandle;
      startPoint: Point;
      startSelection: LogicalRect;
    };
type AnnotationGesture = {
  tool: AnnotationTool;
  startPoint: Point;
  points?: Point[];
};
type AnnotationMoveGesture = {
  annotationIndex: number;
  startPoint: Point;
  startAnnotation: AnnotationCommand;
};
type DraftSelectionMoveGesture = {
  startPoint: Point;
  startSelection: LogicalRect;
  startAnchorPoint: Point;
};

const MIN_SELECTION_SIZE = 10;
const EDGE_SNAP_THRESHOLD = 6;
const KEYBOARD_NUDGE_STEP = 1;
const KEYBOARD_FAST_NUDGE_STEP = 10;
const TOOLBAR_GAP = 8;
const TOOLBAR_SIZE = { width: 1110, height: 36 };
const MAGNIFIER_GAP = 14;
const MAGNIFIER_SIZE = { width: 120, height: 96 };
const MAGNIFIER_ZOOM = 4;
const ARROW_KEYS: ArrowKey[] = ['ArrowUp', 'ArrowRight', 'ArrowDown', 'ArrowLeft'];
const SELECTION_HANDLES: SelectionHandle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

const handleClassNames: Record<SelectionHandle, string> = {
  nw: '-left-1.5 -top-1.5 cursor-nwse-resize',
  n: 'left-1/2 -top-1.5 -translate-x-1/2 cursor-ns-resize',
  ne: '-right-1.5 -top-1.5 cursor-nesw-resize',
  e: '-right-1.5 top-1/2 -translate-y-1/2 cursor-ew-resize',
  se: '-bottom-1.5 -right-1.5 cursor-nwse-resize',
  s: '-bottom-1.5 left-1/2 -translate-x-1/2 cursor-ns-resize',
  sw: '-bottom-1.5 -left-1.5 cursor-nesw-resize',
  w: '-left-1.5 top-1/2 -translate-y-1/2 cursor-ew-resize',
};

function rectStyle(rect: LogicalRect) {
  return {
    left: `${rect.x}px`,
    top: `${rect.y}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
  };
}

function isArrowKey(key: string): key is ArrowKey {
  return ARROW_KEYS.includes(key as ArrowKey);
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function clampPointToRect(point: Point, rect: LogicalRect): Point {
  return {
    x: clamp(point.x, 0, rect.width),
    y: clamp(point.y, 0, rect.height),
  };
}

function annotationRectToViewportRect(
  rect: LogicalRect,
  selectionViewportRect: LogicalRect,
) {
  return {
    x: selectionViewportRect.x + rect.x,
    y: selectionViewportRect.y + rect.y,
    width: rect.width,
    height: rect.height,
  };
}

function sameAnnotationColor(a: AnnotationColor, b: AnnotationColor) {
  return a.every((channel, index) => channel === b[index]);
}

function appendAnnotationPoint(points: Point[], point: Point) {
  const previousPoint = points[points.length - 1];
  if (previousPoint && previousPoint.x === point.x && previousPoint.y === point.y) {
    return points;
  }

  return [...points, point];
}

function svgPolylinePoints(points: Point[]) {
  return points.map((point) => `${point.x},${point.y}`).join(' ');
}

function isPointStrokeAnnotationTool(tool: AnnotationTool) {
  return tool === 'pen' || tool === 'highlight';
}

function DimMask({ rect }: { rect: LogicalRect }) {
  const right = rect.x + rect.width;
  const bottom = rect.y + rect.height;

  return (
    <>
      <div
        className="absolute left-0 top-0 w-full bg-black/45"
        style={{ height: `${rect.y}px` }}
      />
      <div
        className="absolute left-0 bg-black/45"
        style={{
          top: `${bottom}px`,
          width: '100%',
          height: `calc(100% - ${bottom}px)`,
        }}
      />
      <div
        className="absolute left-0 bg-black/45"
        style={{
          top: `${rect.y}px`,
          width: `${rect.x}px`,
          height: `${rect.height}px`,
        }}
      />
      <div
        className="absolute bg-black/45"
        style={{
          left: `${right}px`,
          top: `${rect.y}px`,
          width: `calc(100% - ${right}px)`,
          height: `${rect.height}px`,
        }}
      />
    </>
  );
}

function Magnifier({
  imageBase64,
  viewportCursor,
  imageCursor,
  viewportBounds,
  imageSize,
  selection,
  color,
}: {
  imageBase64: string;
  viewportCursor: Point;
  imageCursor: Point;
  viewportBounds: LogicalRect;
  imageSize: { width: number; height: number };
  selection: LogicalRect | null;
  color: ColorSample | null;
}) {
  const position = getMagnifierPosition(
    viewportCursor,
    viewportBounds,
    MAGNIFIER_SIZE,
    MAGNIFIER_GAP,
  );
  const imageStyle = getMagnifierImageStyle(
    imageBase64,
    imageCursor,
    imageSize,
    MAGNIFIER_SIZE,
    MAGNIFIER_ZOOM,
  );
  const sizeText = selection
    ? `${Math.round(selection.width)} x ${Math.round(selection.height)}`
    : '';

  return (
    <div
      className="pointer-events-none absolute overflow-hidden rounded border border-white/70 bg-neutral-950 text-[10px] text-white shadow-2xl ring-1 ring-black/50"
      style={{
        left: `${position.x}px`,
        top: `${position.y}px`,
        width: `${MAGNIFIER_SIZE.width}px`,
      }}
    >
      <div
        className="relative border-b border-white/20"
        style={{
          ...imageStyle,
          width: `${MAGNIFIER_SIZE.width}px`,
          height: `${MAGNIFIER_SIZE.height}px`,
          backgroundRepeat: 'no-repeat',
        }}
      >
        <div className="absolute left-1/2 top-0 h-full w-px bg-red-400/90" />
        <div className="absolute left-0 top-1/2 h-px w-full bg-red-400/90" />
      </div>
      <div className="flex items-center justify-between px-1.5 py-1 font-mono leading-none">
        <span>
          {Math.round(imageCursor.x)}, {Math.round(imageCursor.y)}
        </span>
        <span className="flex items-center gap-1">
          {color && (
            <>
              <span
                className="h-2.5 w-2.5 border border-white/60"
                style={{ backgroundColor: color.hex }}
              />
              <span>{color.hex}</span>
            </>
          )}
          {sizeText && <span>{sizeText}</span>}
        </span>
      </div>
    </div>
  );
}

interface ScreenshotSessionProps {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void;
}

export default function ScreenshotSession({
  initialMode,
  initialSessionId,
  onInactive,
}: ScreenshotSessionProps) {
  const sampleCanvasByMonitorRef = useRef<Map<string, HTMLCanvasElement>>(new Map());
  const textDraftInputRef = useRef<HTMLTextAreaElement | null>(null);
  const [status, setStatus] = useState<SessionStatus>('idle');
  const [mode, setMode] = useState<CaptureMode>('screenshot');
  const [session, setSession] = useState<CaptureSessionView | null>(null);
  const [startPoint, setStartPoint] = useState<Point | null>(null);
  const [cursorPoint, setCursorPoint] = useState<Point | null>(null);
  const [selection, setSelection] = useState<LogicalRect | null>(null);
  const [hoverSelection, setHoverSelection] = useState<LogicalRect | null>(null);
  const [editGesture, setEditGesture] = useState<EditGesture | null>(null);
  const [activeAnnotationTool, setActiveAnnotationTool] =
    useState<AnnotationTool | null>(null);
  const [annotationGesture, setAnnotationGesture] =
    useState<AnnotationGesture | null>(null);
  const [draftAnnotation, setDraftAnnotation] = useState<AnnotationCommand | null>(null);
  const [selectedAnnotationIndex, setSelectedAnnotationIndex] = useState<number | null>(
    null,
  );
  const [annotationMoveGesture, setAnnotationMoveGesture] =
    useState<AnnotationMoveGesture | null>(null);
  const [draftSelectionMoveGesture, setDraftSelectionMoveGesture] =
    useState<DraftSelectionMoveGesture | null>(null);
  const [textDraft, setTextDraft] = useState<TextAnnotationDraft | null>(null);
  const [textDraftAnnotationIndex, setTextDraftAnnotationIndex] =
    useState<number | null>(null);
  const [annotationStyle, setAnnotationStyle] = useState<AnnotationStyle>(
    DEFAULT_ANNOTATION_STYLE,
  );
  const [textFontSize, setTextFontSize] = useState(DEFAULT_TEXT_FONT_SIZE);
  const [annotationHistory, setAnnotationHistory] = useState(emptyAnnotationHistory);
  const [previewImageBase64, setPreviewImageBase64] = useState<string | null>(null);
  const [cursorColor, setCursorColor] = useState<ColorSample | null>(null);
  const [sampleCanvasVersion, setSampleCanvasVersion] = useState(0);
  const [isRenderingOutput, setIsRenderingOutput] = useState(false);
  const [isToolbarHidden, setIsToolbarHidden] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasStartedInitialSession, setHasStartedInitialSession] = useState(false);

  const isActive = status !== 'idle';
  const annotations = annotationHistory.annotations;
  const selectedAnnotation =
    selectedAnnotationIndex === null ? null : annotations[selectedAnnotationIndex] ?? null;
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
  const sizeLabel = selection
    ? `${Math.round(selection.width)} x ${Math.round(selection.height)}`
    : '';
  const hoverSizeLabel = hoverSelection
    ? `${Math.round(hoverSelection.width)} x ${Math.round(hoverSelection.height)}`
    : '';
  const captureCandidates = useMemo(() => {
    if (!session) return [];

    return buildCaptureCandidates(session.monitors, session.candidates);
  }, [session]);
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
  const selectionViewportRect = useMemo<LogicalRect | null>(() => {
    if (!selection || !selectionBounds) return null;

    return virtualRectToViewportRect(selection, selectionBounds);
  }, [selection, selectionBounds]);
  const hoverSelectionViewportRect = useMemo<LogicalRect | null>(() => {
    if (!hoverSelection || !selectionBounds || selection) return null;

    return virtualRectToViewportRect(hoverSelection, selectionBounds);
  }, [hoverSelection, selection, selectionBounds]);
  const cursorViewportPoint = useMemo<Point | null>(() => {
    if (!cursorPoint || !selectionBounds) return null;

    return virtualPointToViewportPoint(cursorPoint, selectionBounds);
  }, [cursorPoint, selectionBounds]);
  const selectedAnnotationBounds = useMemo<LogicalRect | null>(() => {
    if (
      annotationMoveGesture ||
      selectedAnnotationIndex === null ||
      !annotations[selectedAnnotationIndex]
    ) {
      return null;
    }

    return getAnnotationBounds(annotations[selectedAnnotationIndex]);
  }, [annotationMoveGesture, annotations, selectedAnnotationIndex]);
  const cursorMonitor = useMemo<MonitorSnapshotView | null>(() => {
    if (!session || !cursorPoint) return null;

    return getMonitorAtVirtualPoint(session.monitors, cursorPoint);
  }, [cursorPoint, session]);
  const cursorInMonitorPoint = useMemo<Point | null>(() => {
    if (!cursorPoint || !cursorMonitor) return null;

    return {
      x: cursorPoint.x - cursorMonitor.logical_bounds.x,
      y: cursorPoint.y - cursorMonitor.logical_bounds.y,
    };
  }, [cursorMonitor, cursorPoint]);
  const toolbarPosition = useMemo(() => {
    if (!selectionViewportRect || !viewportBounds || status !== 'preview') return null;

    return getToolbarPosition(selectionViewportRect, viewportBounds, TOOLBAR_SIZE, TOOLBAR_GAP);
  }, [selectionViewportRect, status, viewportBounds]);

  const resetSessionState = useCallback(() => {
    setStatus('idle');
    setSession(null);
    setStartPoint(null);
    setCursorPoint(null);
    setSelection(null);
    setHoverSelection(null);
    setEditGesture(null);
    setActiveAnnotationTool(null);
    setAnnotationGesture(null);
    setDraftAnnotation(null);
    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setDraftSelectionMoveGesture(null);
    setTextDraft(null);
    setTextDraftAnnotationIndex(null);
    setAnnotationHistory(emptyAnnotationHistory());
    setPreviewImageBase64(null);
    setCursorColor(null);
    setSampleCanvasVersion(0);
    setIsRenderingOutput(false);
    setIsToolbarHidden(false);
    setError(null);
  }, []);

  const cancelSession = useCallback(async () => {
    const sessionId = session?.id;
    resetSessionState();

    if (sessionId) {
      try {
        await invoke('cancel_capture_session', { sessionId });
      } catch (err) {
        console.error('Failed to cancel capture session:', err);
      }
    }

    onInactive?.();
  }, [onInactive, resetSessionState, session?.id]);

  const startSession = useCallback(async (nextMode: CaptureMode, sessionId?: string) => {
    setStatus('loading');
    setMode(nextMode);
    setStartPoint(null);
    setCursorPoint(null);
    setSelection(null);
    setHoverSelection(null);
    setEditGesture(null);
    setActiveAnnotationTool(null);
    setAnnotationGesture(null);
    setDraftAnnotation(null);
    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setDraftSelectionMoveGesture(null);
    setTextDraft(null);
    setTextDraftAnnotationIndex(null);
    setAnnotationHistory(emptyAnnotationHistory());
    setPreviewImageBase64(null);
    setCursorColor(null);
    setSampleCanvasVersion(0);
    setIsRenderingOutput(false);
    setIsToolbarHidden(false);
    setError(null);

    try {
      const nextSession = sessionId
        ? await invoke<CaptureSessionView>('get_capture_session', { sessionId })
        : await invoke<CaptureSessionView>('create_capture_session');
      setSession(nextSession);
      setStatus('selecting');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, []);

  const renderSelectionPreview = useCallback(
    async (rect: LogicalRect, nextAnnotations: AnnotationCommand[] = annotations) => {
      if (!session) return;

      setIsRenderingOutput(true);
      setPreviewImageBase64(null);
      setError(null);

      try {
        const base64 = await invoke<string>('render_capture_output', {
          sessionId: session.id,
          rect,
          annotations: nextAnnotations,
        });
        setPreviewImageBase64(base64);

        if (mode === 'screenshot-ocr') {
          const ocrResult = await invoke<OcrResult>('run_capture_ocr', {
            sessionId: session.id,
            rect,
          });
          await invoke('open_result_window', { text: ocrResult.text });
          await invoke('cancel_capture_session', { sessionId: session.id });
          resetSessionState();
          onInactive?.();
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setStatus('error');
      } finally {
        setIsRenderingOutput(false);
      }
    },
    [annotations, mode, onInactive, resetSessionState, session],
  );

  const commitTextDraftToHistory = useCallback(() => {
    if (!textDraft) return annotationHistory;

    const nextHistory = commitTextAnnotationDraft(
      annotationHistory,
      textDraft,
      annotationStyle,
      textDraftAnnotationIndex ?? undefined,
    );
    setTextDraft(null);
    setTextDraftAnnotationIndex(null);
    if (nextHistory !== annotationHistory) {
      setSelectedAnnotationIndex(null);
      setAnnotationHistory(nextHistory);
    }

    return nextHistory;
  }, [annotationHistory, annotationStyle, textDraft, textDraftAnnotationIndex]);

  const copySelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      const outputHistory = commitTextDraftToHistory();
      await invoke('output_capture', {
        sessionId: session.id,
        rect: selection,
        annotations: outputHistory.annotations,
        action: { type: 'copy' },
      });
      await invoke('cancel_capture_session', { sessionId: session.id });
      resetSessionState();
      onInactive?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [commitTextDraftToHistory, onInactive, resetSessionState, selection, session]);

  const copyCurrentColor = useCallback(async () => {
    if (!cursorColor) return;

    try {
      await navigator.clipboard.writeText(colorSampleToClipboardText(cursorColor));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, [cursorColor]);

  const saveSelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      const outputHistory = commitTextDraftToHistory();
      await saveCaptureSelection(
        invoke,
        session.id,
        selection,
        outputHistory.annotations,
      );
      await invoke('cancel_capture_session', { sessionId: session.id });
      resetSessionState();
      onInactive?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [commitTextDraftToHistory, onInactive, resetSessionState, selection, session]);

  const runOcrSelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      const ocrResult = await invoke<OcrResult>('run_capture_ocr', {
        sessionId: session.id,
        rect: selection,
      });
      await invoke('open_result_window', { text: ocrResult.text });
      await invoke('cancel_capture_session', { sessionId: session.id });
      resetSessionState();
      onInactive?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [onInactive, resetSessionState, selection, session]);

  const pinSelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      const outputHistory = commitTextDraftToHistory();
      await invoke('output_capture', {
        sessionId: session.id,
        rect: selection,
        annotations: outputHistory.annotations,
        action: { type: 'pin' },
      });
      await invoke('cancel_capture_session', { sessionId: session.id });
      resetSessionState();
      onInactive?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [commitTextDraftToHistory, onInactive, resetSessionState, selection, session]);

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

  const nudgeSelectedAnnotation = useCallback(
    (delta: Point) => {
      if (
        !selection ||
        selectedAnnotationIndex === null ||
        !annotations[selectedAnnotationIndex]
      ) {
        return;
      }

      const nextAnnotation = moveAnnotationByDelta(
        annotations[selectedAnnotationIndex],
        delta,
      );
      const nextHistory = replaceAnnotationInHistory(
        annotationHistory,
        selectedAnnotationIndex,
        nextAnnotation,
      );
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
    ],
  );

  const syncToolbarStyleFromAnnotation = useCallback(
    (annotation: AnnotationCommand) => {
      if (annotation.type === 'mosaic') {
        setAnnotationStyle((style) => ({
          ...style,
          strokeWidth: annotation.block_size,
        }));
        return;
      }

      if (annotation.type === 'blur') {
        setAnnotationStyle((style) => ({
          ...style,
          strokeWidth: annotation.radius,
        }));
        return;
      }

      if (annotation.type === 'text') {
        setAnnotationStyle((style) => ({
          ...style,
          color: annotation.color,
        }));
        setTextFontSize(annotation.font_size);
        return;
      }

      setAnnotationStyle({
        color: annotation.color,
        strokeWidth: annotation.stroke_width,
      });
    },
    [],
  );

  const applySelectedAnnotationStyle = useCallback(
    (nextStyle: AnnotationStyle, nextTextFontSize: number) => {
      setAnnotationStyle(nextStyle);
      setTextFontSize(nextTextFontSize);

      if (
        !selection ||
        textDraft ||
        selectedAnnotationIndex === null ||
        !annotations[selectedAnnotationIndex]
      ) {
        return;
      }

      const nextAnnotation = applyAnnotationStyle(
        annotations[selectedAnnotationIndex],
        nextStyle,
        nextTextFontSize,
      );
      const nextHistory = replaceAnnotationInHistory(
        annotationHistory,
        selectedAnnotationIndex,
        nextAnnotation,
      );
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

      if (isTextSizingActive) {
        applySelectedAnnotationStyle(
          annotationStyle,
          nextTextFontSize(textFontSize, direction),
        );
        return;
      }

      applySelectedAnnotationStyle(
        {
          ...annotationStyle,
          strokeWidth: nextAnnotationStrokeWidth(
            annotationStyle.strokeWidth,
            direction,
          ),
        },
        textFontSize,
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

      applySelectedAnnotationStyle(
        {
          ...annotationStyle,
          color,
        },
        textFontSize,
      );
    },
    [
      annotationStyle,
      applySelectedAnnotationStyle,
      textDraft,
      textFontSize,
    ],
  );

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

  const dismissCaptureLayer = useCallback(() => {
    if (textDraft) {
      setTextDraft(null);
      setTextDraftAnnotationIndex(null);
    } else if (annotationMoveGesture) {
      setAnnotationMoveGesture(null);
      setDraftAnnotation(null);
      if (selection) {
        void renderSelectionPreview(selection, annotations);
      }
    } else if (draftSelectionMoveGesture) {
      setDraftSelectionMoveGesture(null);
    } else if (selectedAnnotationIndex !== null) {
      setSelectedAnnotationIndex(null);
    } else if (activeAnnotationTool || annotationGesture) {
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

  useEffect(() => {
    if (!initialMode || hasStartedInitialSession) return;

    setHasStartedInitialSession(true);
    void startSession(initialMode, initialSessionId);
  }, [hasStartedInitialSession, initialMode, initialSessionId, startSession]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    listen<unknown>('hotkey-triggered', (event) => {
      const launch = parseCaptureLaunchPayload(event.payload);
      if (launch) {
        void startSession(launch.mode, launch.sessionId);
      }
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for capture hotkeys:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [startSession]);

  useEffect(() => {
    sampleCanvasByMonitorRef.current = new Map();
    setCursorColor(null);
    setSampleCanvasVersion((version) => version + 1);

    if (!session) return;

    let disposed = false;
    session.monitors.forEach((monitor) => {
      const image = new Image();
      image.onload = () => {
        if (disposed) return;

        const canvas = document.createElement('canvas');
        canvas.width = image.naturalWidth;
        canvas.height = image.naturalHeight;
        canvas.getContext('2d')?.drawImage(image, 0, 0);
        sampleCanvasByMonitorRef.current.set(monitor.id, canvas);
        setSampleCanvasVersion((version) => version + 1);
      };
      image.src = `data:image/png;base64,${monitor.image_base64}`;
    });

    return () => {
      disposed = true;
    };
  }, [session]);

  useEffect(() => {
    if (!cursorInMonitorPoint || !cursorMonitor) {
      setCursorColor(null);
      return;
    }

    const canvas = sampleCanvasByMonitorRef.current.get(cursorMonitor.id);
    if (!canvas) {
      setCursorColor(null);
      return;
    }

    setCursorColor(
      sampleCanvasColor(canvas, cursorInMonitorPoint, {
        width: cursorMonitor.logical_bounds.width,
        height: cursorMonitor.logical_bounds.height,
      }),
    );
  }, [cursorInMonitorPoint, cursorMonitor, sampleCanvasVersion]);

  useEffect(() => {
    if (!isActive) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        dismissCaptureLayer();
      } else if (
        status === 'preview' &&
        (event.metaKey || event.ctrlKey) &&
        event.key.toLowerCase() === 'z'
      ) {
        event.preventDefault();
        if (event.shiftKey) {
          redoAnnotation();
        } else {
          undoAnnotation();
        }
      } else if (
        status === 'preview' &&
        (event.metaKey || event.ctrlKey) &&
        event.key.toLowerCase() === 'y'
      ) {
        event.preventDefault();
        redoAnnotation();
      } else if (
        status === 'preview' &&
        selectedAnnotationIndex !== null &&
        (event.key === 'Backspace' || event.key === 'Delete')
      ) {
        event.preventDefault();
        deleteSelectedAnnotation();
      } else if (
        !textDraft &&
        cursorColor &&
        isColorSampleCopyShortcut(event)
      ) {
        event.preventDefault();
        void copyCurrentColor();
      } else if (
        status === 'preview' &&
        (event.key === 'Enter' ||
          ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'c'))
      ) {
        event.preventDefault();
        void copySelection();
      } else if (status === 'preview' && isSaveCaptureShortcut(event)) {
        event.preventDefault();
        void saveSelection();
      } else if (status === 'preview' && isPinCaptureShortcut(event)) {
        event.preventDefault();
        void pinSelection();
      } else if (
        status === 'preview' &&
        !textDraft &&
        isToggleToolbarShortcut(event)
      ) {
        event.preventDefault();
        setIsToolbarHidden((isHidden) => !isHidden);
      } else if (
        status === 'preview' &&
        !textDraft &&
        (event.key === '[' || event.key === ']')
      ) {
        const sizeDirection = annotationSizeDirectionFromShortcut(event);
        if (sizeDirection) {
          event.preventDefault();
          adjustAnnotationSize(sizeDirection);
        }
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
        event.key === ' ' &&
        status === 'selecting' &&
        startPoint &&
        selection &&
        cursorPoint &&
        !draftSelectionMoveGesture
      ) {
        event.preventDefault();
        setDraftSelectionMoveGesture({
          startPoint: cursorPoint,
          startSelection: selection,
          startAnchorPoint: startPoint,
        });
      } else if (
        status === 'preview' &&
        !textDraft &&
        !annotationGesture &&
        !annotationMoveGesture &&
        selectedAnnotationIndex !== null &&
        isArrowKey(event.key)
      ) {
        event.preventDefault();
        const step = event.shiftKey ? KEYBOARD_FAST_NUDGE_STEP : KEYBOARD_NUDGE_STEP;
        const delta = getAnnotationKeyboardNudgeDelta(event.key, step);
        if (delta) {
          nudgeSelectedAnnotation(delta);
        }
      } else if (status === 'preview' && selection && selectionBounds && isArrowKey(event.key)) {
        event.preventDefault();
        const step = event.shiftKey ? KEYBOARD_FAST_NUDGE_STEP : KEYBOARD_NUDGE_STEP;
        const nextSelection = nudgeSelection(selection, event.key, selectionBounds, step);
        setSelection(nextSelection);
        setPreviewImageBase64(null);
        void renderSelectionPreview(nextSelection);
      }
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      if (event.key === ' ' && draftSelectionMoveGesture) {
        event.preventDefault();
        setDraftSelectionMoveGesture(null);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, [
    adjustAnnotationSize,
    copyCurrentColor,
    copySelection,
    annotationGesture,
    annotationMoveGesture,
    cursorPoint,
    draftSelectionMoveGesture,
    dismissCaptureLayer,
    textDraft,
    deleteSelectedAnnotation,
    redoAnnotation,
    isActive,
    nudgeSelectedAnnotation,
    pinSelection,
    saveSelection,
    selectAnnotationColor,
    selection,
    selectionBounds,
    selectedAnnotationIndex,
    startPoint,
    status,
    cursorColor,
    toggleAnnotationTool,
    undoAnnotation,
  ]);

  useEffect(() => {
    if (!textDraft) return;

    requestAnimationFrame(() => {
      textDraftInputRef.current?.focus();
    });
  }, [textDraft]);

  const handlePointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (isCancelCapturePointer(event)) {
      event.preventDefault();
      event.stopPropagation();
      dismissCaptureLayer();
      return;
    }

    if ((status !== 'selecting' && status !== 'preview') || !selectionBounds) return;

    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    const snappedPoint = snapPointToRects(point, snapTargetRects, EDGE_SNAP_THRESHOLD);
    setCursorPoint(point);
    event.currentTarget.setPointerCapture(event.pointerId);
    setStartPoint(snappedPoint);
    setSelection(normalizeSelection(snappedPoint, snappedPoint));
    setPreviewImageBase64(null);
    setIsRenderingOutput(false);
    setStatus('selecting');
    setActiveAnnotationTool(null);
    setAnnotationGesture(null);
    setDraftAnnotation(null);
    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setDraftSelectionMoveGesture(null);
    setTextDraft(null);
    setTextDraftAnnotationIndex(null);
    setAnnotationHistory(emptyAnnotationHistory());
    setIsToolbarHidden(false);
  };

  const applyEditGesture = useCallback(
    (gesture: EditGesture, point: Point) => {
      if (!selectionBounds) return gesture.startSelection;

      const delta = {
        x: point.x - gesture.startPoint.x,
        y: point.y - gesture.startPoint.y,
      };

      if (gesture.type === 'move') {
        const movedSelection = moveSelectionByDelta(
          gesture.startSelection,
          delta,
          selectionBounds,
        );

        return snapMovedSelectionToRects(
          movedSelection,
          snapTargetRects,
          selectionBounds,
          EDGE_SNAP_THRESHOLD,
        );
      }

      const resizedSelection = resizeSelectionByHandle(
        gesture.startSelection,
        gesture.handle,
        delta,
        selectionBounds,
        MIN_SELECTION_SIZE,
      );

      return snapResizedSelectionToRects(
        resizedSelection,
        gesture.handle,
        snapTargetRects,
        selectionBounds,
        MIN_SELECTION_SIZE,
        EDGE_SNAP_THRESHOLD,
      );
    },
    [selectionBounds, snapTargetRects],
  );

  const handlePointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!selectionBounds) return;

    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );

    if (status === 'selecting' || status === 'preview') {
      setCursorPoint(point);
    }

    if (!startPoint && !editGesture && status === 'selecting') {
      setHoverSelection(getBestCandidateAtPoint(captureCandidates, point)?.rect ?? null);
    }

    if (annotationGesture && selection) {
      const localPoint = clampPointToRect(
        { x: point.x - selection.x, y: point.y - selection.y },
        selection,
      );
      const points = isPointStrokeAnnotationTool(annotationGesture.tool)
        ? appendAnnotationPoint(annotationGesture.points ?? [], localPoint)
        : undefined;
      const gesturePoint = event.shiftKey
        ? constrainAnnotationGesturePoint(
            annotationGesture.tool,
            annotationGesture.startPoint,
            localPoint,
          )
        : localPoint;
      if (points) {
        setAnnotationGesture({
          ...annotationGesture,
          points,
        });
      }
      setDraftAnnotation(
        annotationFromGesture(
          annotationGesture.tool,
          annotationGesture.startPoint,
          gesturePoint,
          annotationStyle,
          points,
        ),
      );
      return;
    }

    if (annotationMoveGesture && selection) {
      const localPoint = clampPointToRect(
        { x: point.x - selection.x, y: point.y - selection.y },
        selection,
      );
      const delta = {
        x: localPoint.x - annotationMoveGesture.startPoint.x,
        y: localPoint.y - annotationMoveGesture.startPoint.y,
      };
      setPreviewImageBase64(null);
      setDraftAnnotation(
        moveAnnotationByDelta(annotationMoveGesture.startAnnotation, delta),
      );
      return;
    }

    if (draftSelectionMoveGesture && status === 'selecting') {
      const result = moveDraftSelectionByDelta(
        draftSelectionMoveGesture.startSelection,
        draftSelectionMoveGesture.startAnchorPoint,
        {
          x: point.x - draftSelectionMoveGesture.startPoint.x,
          y: point.y - draftSelectionMoveGesture.startPoint.y,
        },
        selectionBounds,
      );
      setSelection(result.selection);
      setStartPoint(result.anchorPoint);
      setPreviewImageBase64(null);
      setIsRenderingOutput(false);
      return;
    }

    if (editGesture) {
      setSelection(applyEditGesture(editGesture, point));
      setPreviewImageBase64(null);
      setIsRenderingOutput(false);
      return;
    }

    if (!startPoint || status !== 'selecting') return;

    const currentPoint = snapPointToRects(point, snapTargetRects, EDGE_SNAP_THRESHOLD);
    setSelection(
      normalizeSelection(
        startPoint,
        event.shiftKey ? constrainSelectionPoint(startPoint, currentPoint) : currentPoint,
      ),
    );
  };

  const handlePointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!selectionBounds) return;

    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    setCursorPoint(point);
    setDraftSelectionMoveGesture(null);

    if (annotationGesture && selection) {
      const localPoint = clampPointToRect(
        { x: point.x - selection.x, y: point.y - selection.y },
        selection,
      );
      const points = isPointStrokeAnnotationTool(annotationGesture.tool)
        ? appendAnnotationPoint(annotationGesture.points ?? [], localPoint)
        : undefined;
      const gesturePoint = event.shiftKey
        ? constrainAnnotationGesturePoint(
            annotationGesture.tool,
            annotationGesture.startPoint,
            localPoint,
          )
        : localPoint;
      const nextAnnotation = annotationFromGesture(
        annotationGesture.tool,
        annotationGesture.startPoint,
        gesturePoint,
        annotationStyle,
        points,
      );
      setAnnotationGesture(null);
      setDraftAnnotation(null);
      if (isCommittedAnnotation(nextAnnotation)) {
        const nextHistory = addAnnotationToHistory(annotationHistory, nextAnnotation);
        setSelectedAnnotationIndex(null);
        setAnnotationHistory(nextHistory);
        void renderSelectionPreview(selection, nextHistory.annotations);
      }
      return;
    }

    if (annotationMoveGesture && selection) {
      const localPoint = clampPointToRect(
        { x: point.x - selection.x, y: point.y - selection.y },
        selection,
      );
      const delta = {
        x: localPoint.x - annotationMoveGesture.startPoint.x,
        y: localPoint.y - annotationMoveGesture.startPoint.y,
      };
      const nextAnnotation = moveAnnotationByDelta(
        annotationMoveGesture.startAnnotation,
        delta,
      );
      const nextHistory = replaceAnnotationInHistory(
        annotationHistory,
        annotationMoveGesture.annotationIndex,
        nextAnnotation,
      );
      setAnnotationMoveGesture(null);
      setDraftAnnotation(null);
      setAnnotationHistory(nextHistory);
      if (nextHistory === annotationHistory) {
        void renderSelectionPreview(selection, annotations);
      } else {
        setSelectedAnnotationIndex(annotationMoveGesture.annotationIndex);
        void renderSelectionPreview(selection, nextHistory.annotations);
      }
      return;
    }

    if (editGesture) {
      const nextSelection = applyEditGesture(editGesture, point);
      setEditGesture(null);
      setSelection(nextSelection);
      setStatus('preview');
      void renderSelectionPreview(nextSelection, annotations);
      return;
    }

    if (!startPoint || status !== 'selecting') return;

    const currentPoint = snapPointToRects(point, snapTargetRects, EDGE_SNAP_THRESHOLD);
    const nextSelection = normalizeSelection(
      startPoint,
      event.shiftKey ? constrainSelectionPoint(startPoint, currentPoint) : currentPoint,
    );
    setStartPoint(null);

    if (
      nextSelection.width < MIN_SELECTION_SIZE ||
      nextSelection.height < MIN_SELECTION_SIZE
    ) {
      if (hoverSelection) {
        setSelection(hoverSelection);
        setHoverSelection(null);
        setStatus('preview');
        void renderSelectionPreview(hoverSelection, []);
        return;
      }

      setSelection(null);
      return;
    }

    setSelection(nextSelection);
    setHoverSelection(null);
    setStatus('preview');
    void renderSelectionPreview(nextSelection, []);
  };

  const startMoveGesture = (event: React.PointerEvent<HTMLDivElement>) => {
    if (status !== 'preview' || !selection || !selectionBounds) return;

    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    setCursorPoint(point);
    if (activeAnnotationTool) {
      setSelectedAnnotationIndex(null);
      const localPoint = clampPointToRect(
        { x: point.x - selection.x, y: point.y - selection.y },
        selection,
      );
      if (activeAnnotationTool === 'text') {
        if (textDraft) return;
        setTextDraft(startTextAnnotationDraft(localPoint, textFontSize));
        setTextDraftAnnotationIndex(null);
        return;
      }

      const points = isPointStrokeAnnotationTool(activeAnnotationTool)
        ? [localPoint]
        : undefined;
      setAnnotationGesture({
        tool: activeAnnotationTool,
        startPoint: localPoint,
        ...(points ? { points } : {}),
      });
      setDraftAnnotation(
        annotationFromGesture(
          activeAnnotationTool,
          localPoint,
          localPoint,
          annotationStyle,
          points,
        ),
      );
      return;
    }

    const localPoint = clampPointToRect(
      { x: point.x - selection.x, y: point.y - selection.y },
      selection,
    );
    const hitAnnotationIndex = hitTestAnnotations(annotations, localPoint);
    if (hitAnnotationIndex !== null) {
      const hitAnnotation = annotations[hitAnnotationIndex];
      if (event.detail >= 2 && hitAnnotation.type === 'text') {
        setSelectedAnnotationIndex(hitAnnotationIndex);
        setAnnotationMoveGesture(null);
        setDraftAnnotation(null);
        setTextDraft(startTextAnnotationDraftFromAnnotation(hitAnnotation));
        setTextDraftAnnotationIndex(hitAnnotationIndex);
        syncToolbarStyleFromAnnotation(hitAnnotation);
        setPreviewImageBase64(null);
        void renderSelectionPreview(
          selection,
          annotations.filter((_, index) => index !== hitAnnotationIndex),
        );
        return;
      }

      setSelectedAnnotationIndex(hitAnnotationIndex);
      syncToolbarStyleFromAnnotation(hitAnnotation);
      setAnnotationMoveGesture({
        annotationIndex: hitAnnotationIndex,
        startPoint: localPoint,
        startAnnotation: hitAnnotation,
      });
      return;
    }

    if (!textDraft && isCopyCaptureDoubleClick(event)) {
      event.preventDefault();
      void copySelection();
      return;
    }

    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setEditGesture({
      type: 'move',
      startPoint: point,
      startSelection: selection,
    });
    setPreviewImageBase64(null);
  };

  function toggleAnnotationTool(nextTool: AnnotationTool) {
    const nextHistory = commitTextDraftToHistory();
    if (selection && nextHistory !== annotationHistory) {
      void renderSelectionPreview(selection, nextHistory.annotations);
    }

    setActiveAnnotationTool((tool) => (tool === nextTool ? null : nextTool));
    setAnnotationGesture(null);
    setAnnotationMoveGesture(null);
    setDraftAnnotation(null);
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
    setCursorPoint(point);
    setEditGesture({
      type: 'resize',
      handle,
      startPoint: point,
      startSelection: selection,
    });
    setPreviewImageBase64(null);
  };

  const overlayClassName = useMemo(() => {
    if (mode === 'screenshot-ocr') return 'border-sky-300';
    if (mode === 'screenshot-translate') return 'border-emerald-300';
    return 'border-white';
  }, [mode]);

  if (!isActive) return null;

  return (
    <div
      className="fixed left-0 top-0 z-[9999] cursor-crosshair select-none overflow-hidden bg-black text-white"
      style={{
        width: `${viewportBounds?.width ?? window.innerWidth}px`,
        height: `${viewportBounds?.height ?? window.innerHeight}px`,
      }}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onContextMenu={(event) => event.preventDefault()}
    >
      {session &&
        selectionBounds &&
        session.monitors.map((monitor) => (
          <img
            key={monitor.id}
            src={`data:image/png;base64,${monitor.image_base64}`}
            className="absolute object-fill"
            style={rectStyle(getMonitorViewportRect(monitor, selectionBounds))}
            draggable={false}
          />
        ))}

      {status === 'loading' && (
        <div className="absolute inset-0 bg-black" aria-label="Loading capture" />
      )}

      {status === 'error' && (
        <div className="absolute left-4 top-4 max-w-md rounded bg-red-950/90 px-3 py-2 text-sm text-red-100 shadow-lg">
          {error}
        </div>
      )}

      {hoverSelectionViewportRect && status === 'selecting' && (
        <>
          <DimMask rect={hoverSelectionViewportRect} />
          <div
            className="pointer-events-none absolute border border-white/80 bg-white/5"
            style={rectStyle(hoverSelectionViewportRect)}
          />
          <div
            className="pointer-events-none absolute rounded bg-black/80 px-2 py-1 text-xs leading-none text-white shadow"
            style={{
              left: `${hoverSelectionViewportRect.x}px`,
              top: `${Math.max(0, hoverSelectionViewportRect.y - 24)}px`,
            }}
          >
            {hoverSizeLabel}
          </div>
        </>
      )}

      {selection && selectionViewportRect && (
        <>
          <DimMask rect={selectionViewportRect} />
          {previewImageBase64 && status === 'preview' && (
            <img
              src={`data:image/png;base64,${previewImageBase64}`}
              className="absolute object-fill"
              style={rectStyle(selectionViewportRect)}
              draggable={false}
            />
          )}
          {draftAnnotation?.type === 'rectangle' && (
            <div
              className="pointer-events-none absolute"
              style={{
                ...rectStyle(
                  annotationRectToViewportRect(
                    draftAnnotation.rect,
                    selectionViewportRect,
                  ),
                ),
                border: `${draftAnnotation.stroke_width}px solid ${annotationColorToCss(
                  draftAnnotation.color,
                )}`,
              }}
            />
          )}
          {draftAnnotation?.type === 'ellipse' && (
            <svg
              className="pointer-events-none absolute overflow-visible"
              style={rectStyle(selectionViewportRect)}
              viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
              fill="none"
            >
              <ellipse
                cx={draftAnnotation.rect.x + draftAnnotation.rect.width / 2}
                cy={draftAnnotation.rect.y + draftAnnotation.rect.height / 2}
                rx={draftAnnotation.rect.width / 2}
                ry={draftAnnotation.rect.height / 2}
                stroke={annotationColorToCss(draftAnnotation.color)}
                strokeWidth={draftAnnotation.stroke_width}
              />
            </svg>
          )}
          {draftAnnotation?.type === 'mosaic' && (
            <div
              className="pointer-events-none absolute border border-white/70 bg-black/35"
              style={{
                ...rectStyle(
                  annotationRectToViewportRect(
                    draftAnnotation.rect,
                    selectionViewportRect,
                  ),
                ),
                backgroundImage:
                  'linear-gradient(45deg, rgba(255,255,255,0.2) 25%, transparent 25%, transparent 75%, rgba(255,255,255,0.2) 75%), linear-gradient(45deg, rgba(255,255,255,0.2) 25%, transparent 25%, transparent 75%, rgba(255,255,255,0.2) 75%)',
                backgroundPosition: '0 0, 4px 4px',
                backgroundSize: '8px 8px',
              }}
            />
          )}
          {draftAnnotation?.type === 'blur' && (
            <div
              className="pointer-events-none absolute border border-white/70 bg-white/10"
              style={{
                ...rectStyle(
                  annotationRectToViewportRect(
                    draftAnnotation.rect,
                    selectionViewportRect,
                  ),
                ),
                backdropFilter: `blur(${draftAnnotation.radius}px)`,
              }}
            />
          )}
          {draftAnnotation?.type === 'line' && (
            <svg
              className="pointer-events-none absolute overflow-visible"
              style={rectStyle(selectionViewportRect)}
              viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
              fill="none"
            >
              <line
                x1={draftAnnotation.start.x}
                y1={draftAnnotation.start.y}
                x2={draftAnnotation.end.x}
                y2={draftAnnotation.end.y}
                stroke={annotationColorToCss(draftAnnotation.color)}
                strokeWidth={draftAnnotation.stroke_width}
                strokeLinecap="round"
              />
            </svg>
          )}
          {draftAnnotation?.type === 'arrow' && (
            <svg
              className="pointer-events-none absolute overflow-visible"
              style={rectStyle(selectionViewportRect)}
              viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
              fill="none"
            >
              <line
                x1={draftAnnotation.start.x}
                y1={draftAnnotation.start.y}
                x2={draftAnnotation.end.x}
                y2={draftAnnotation.end.y}
                stroke={annotationColorToCss(draftAnnotation.color)}
                strokeWidth={draftAnnotation.stroke_width}
                strokeLinecap="round"
              />
              {arrowHeadPoints(
                draftAnnotation.start,
                draftAnnotation.end,
                draftAnnotation.stroke_width,
              ) && (
                <polygon
                  points={
                    arrowHeadPoints(
                      draftAnnotation.start,
                      draftAnnotation.end,
                      draftAnnotation.stroke_width,
                    ) ?? ''
                  }
                  fill={annotationColorToCss(draftAnnotation.color)}
                />
              )}
            </svg>
          )}
          {draftAnnotation?.type === 'freehand' && (
            <svg
              className="pointer-events-none absolute overflow-visible"
              style={rectStyle(selectionViewportRect)}
              viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
              fill="none"
            >
              <polyline
                points={svgPolylinePoints(draftAnnotation.points)}
                stroke={annotationColorToCss(draftAnnotation.color)}
                strokeWidth={draftAnnotation.stroke_width}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          )}
          {draftAnnotation?.type === 'highlight' && (
            <svg
              className="pointer-events-none absolute overflow-visible"
              style={rectStyle(selectionViewportRect)}
              viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
              fill="none"
            >
              <polyline
                points={svgPolylinePoints(draftAnnotation.points)}
                stroke={annotationColorToCss(draftAnnotation.color)}
                strokeWidth={draftAnnotation.stroke_width}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          )}
          {draftAnnotation?.type === 'text' && (
            <div
              className="pointer-events-none absolute whitespace-pre"
              style={{
                left: `${selectionViewportRect.x + draftAnnotation.position.x}px`,
                top: `${selectionViewportRect.y + draftAnnotation.position.y - draftAnnotation.font_size}px`,
                color: annotationColorToCss(draftAnnotation.color),
                fontSize: `${draftAnnotation.font_size}px`,
                lineHeight: 1,
              }}
            >
              {draftAnnotation.text}
            </div>
          )}
          {textDraft && (
            <textarea
              ref={textDraftInputRef}
              data-screenshot-text-draft="true"
              className="absolute resize-none overflow-hidden border border-white/70 bg-black/15 px-1 py-0 text-left outline-none ring-1 ring-black/35"
              style={{
                left: `${selectionViewportRect.x + textDraft.position.x}px`,
                top: `${selectionViewportRect.y + textDraft.position.y - textDraft.fontSize}px`,
                width: `${Math.max(160, selectionViewportRect.width - textDraft.position.x)}px`,
                minHeight: `${Math.ceil(textDraft.fontSize * 1.35)}px`,
                color: annotationColorToCss(annotationStyle.color),
                fontSize: `${textDraft.fontSize}px`,
                lineHeight: 1.2,
                zIndex: 2,
              }}
              value={textDraft.text}
              onBlur={commitTextDraft}
              onChange={(event) => {
                const text = event.currentTarget.value;
                setTextDraft((draft) =>
                  draft ? updateTextAnnotationDraft(draft, text) : draft,
                );
              }}
              onKeyDown={(event) => {
                event.stopPropagation();
                if (event.key === 'Escape') {
                  event.preventDefault();
                  setTextDraft(null);
                  setTextDraftAnnotationIndex(null);
                  if (textDraftAnnotationIndex !== null && selection) {
                    void renderSelectionPreview(selection, annotations);
                  }
                } else if (
                  event.key === 'Enter' &&
                  (event.metaKey || event.ctrlKey)
                ) {
                  event.preventDefault();
                  event.currentTarget.blur();
                }
              }}
              onPointerDown={(event) => event.stopPropagation()}
            />
          )}
          {selectedAnnotationBounds && (
            <div
              className="pointer-events-none absolute border border-dashed border-white shadow-[0_0_0_1px_rgba(0,0,0,0.55)]"
              style={rectStyle(
                annotationRectToViewportRect(
                  selectedAnnotationBounds,
                  selectionViewportRect,
                ),
              )}
            />
          )}
          <div
            className={`absolute border ${overlayClassName} bg-transparent ${
              status === 'preview'
                ? activeAnnotationTool
                  ? 'cursor-crosshair'
                  : 'cursor-move'
                : ''
            }`}
            style={rectStyle(selectionViewportRect)}
            onPointerDown={startMoveGesture}
          />
          {status === 'preview' && (
            <div className="absolute pointer-events-none" style={rectStyle(selectionViewportRect)}>
              {SELECTION_HANDLES.map((handle) => (
                <button
                  key={handle}
                  className={`pointer-events-auto absolute h-3 w-3 rounded-full border border-black/70 bg-white shadow ${handleClassNames[handle]}`}
                  aria-label={`Resize selection ${handle}`}
                  onPointerDown={(event) => startResizeGesture(handle, event)}
                />
              ))}
            </div>
          )}
          {toolbarPosition && !isToolbarHidden && (
            <div
              className="absolute flex h-9 items-center gap-1 rounded bg-neutral-950/90 p-1 text-xs text-white shadow-lg ring-1 ring-white/15"
              style={{
                left: `${toolbarPosition.x}px`,
                top: `${toolbarPosition.y}px`,
                width: `${TOOLBAR_SIZE.width}px`,
              }}
              onPointerDown={(event) => {
                event.stopPropagation();
                const target = event.target as HTMLElement;
                if (textDraft && target.tagName !== 'INPUT') {
                  event.preventDefault();
                }
              }}
            >
              <button
                type="button"
                className="h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50"
                disabled={isRenderingOutput || !canUndoAnnotation}
                title="Undo"
                aria-label="Undo annotation"
                onClick={undoAnnotation}
              >
                Undo
              </button>
              <button
                type="button"
                className="h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50"
                disabled={isRenderingOutput || !canRedoAnnotation}
                title="Redo"
                aria-label="Redo annotation"
                onClick={redoAnnotation}
              >
                Redo
              </button>
              <button
                type="button"
                className={`h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50 ${
                  activeAnnotationTool === 'rectangle' ? 'bg-white/15' : ''
                }`}
                disabled={isRenderingOutput}
                title="Rectangle"
                aria-label="Draw rectangle annotation"
                onClick={() => toggleAnnotationTool('rectangle')}
              >
                Rect
              </button>
              <button
                type="button"
                className={`h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50 ${
                  activeAnnotationTool === 'ellipse' ? 'bg-white/15' : ''
                }`}
                disabled={isRenderingOutput}
                title="Ellipse"
                aria-label="Draw ellipse annotation"
                onClick={() => toggleAnnotationTool('ellipse')}
              >
                Ellipse
              </button>
              <button
                type="button"
                className={`h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50 ${
                  activeAnnotationTool === 'arrow' ? 'bg-white/15' : ''
                }`}
                disabled={isRenderingOutput}
                title="Arrow"
                aria-label="Draw arrow annotation"
                onClick={() => toggleAnnotationTool('arrow')}
              >
                Arrow
              </button>
              <button
                type="button"
                className={`h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50 ${
                  activeAnnotationTool === 'line' ? 'bg-white/15' : ''
                }`}
                disabled={isRenderingOutput}
                title="Line"
                aria-label="Draw line annotation"
                onClick={() => toggleAnnotationTool('line')}
              >
                Line
              </button>
              <button
                type="button"
                className={`h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50 ${
                  activeAnnotationTool === 'pen' ? 'bg-white/15' : ''
                }`}
                disabled={isRenderingOutput}
                title="Pen"
                aria-label="Draw freehand annotation"
                onClick={() => toggleAnnotationTool('pen')}
              >
                Pen
              </button>
              <button
                type="button"
                className={`h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50 ${
                  activeAnnotationTool === 'highlight' ? 'bg-white/15' : ''
                }`}
                disabled={isRenderingOutput}
                title="Highlight"
                aria-label="Draw highlight annotation"
                onClick={() => toggleAnnotationTool('highlight')}
              >
                Highlight
              </button>
              <button
                type="button"
                className={`h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50 ${
                  activeAnnotationTool === 'mosaic' ? 'bg-white/15' : ''
                }`}
                disabled={isRenderingOutput}
                title="Mosaic"
                aria-label="Draw mosaic annotation"
                onClick={() => toggleAnnotationTool('mosaic')}
              >
                Mosaic
              </button>
              <button
                type="button"
                className={`h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50 ${
                  activeAnnotationTool === 'blur' ? 'bg-white/15' : ''
                }`}
                disabled={isRenderingOutput}
                title="Blur"
                aria-label="Draw blur annotation"
                onClick={() => toggleAnnotationTool('blur')}
              >
                Blur
              </button>
              <button
                type="button"
                className={`h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50 ${
                  activeAnnotationTool === 'text' ? 'bg-white/15' : ''
                }`}
                disabled={isRenderingOutput}
                title="Text"
                aria-label="Add text annotation"
                onClick={() => toggleAnnotationTool('text')}
              >
                Text
              </button>
              <div className="flex h-7 items-center gap-1 px-1">
                {ANNOTATION_COLORS.map((color) => (
                  <button
                    key={color.join('-')}
                    type="button"
                    className={`h-5 w-5 rounded border border-white/40 ${
                      sameAnnotationColor(annotationStyle.color, color)
                        ? 'ring-2 ring-white'
                        : ''
                    }`}
                    style={{ backgroundColor: annotationColorToCss(color) }}
                    disabled={isRenderingOutput}
                    title="Annotation color"
                    aria-label="Annotation color"
                    onClick={() => {
                      const nextStyle = {
                        ...annotationStyle,
                        color,
                      };
                      applySelectedAnnotationStyle(nextStyle, textFontSize);
                    }}
                  />
                ))}
              </div>
              <input
                className="h-7 w-20 accent-white disabled:opacity-50"
                type="range"
                min={
                  isTextSizingActive
                    ? MIN_TEXT_FONT_SIZE
                    : MIN_ANNOTATION_STROKE_WIDTH
                }
                max={
                  isTextSizingActive
                    ? MAX_TEXT_FONT_SIZE
                    : MAX_ANNOTATION_STROKE_WIDTH
                }
                step={1}
                value={
                  isTextSizingActive
                    ? textFontSize
                    : annotationStyle.strokeWidth
                }
                disabled={isRenderingOutput}
                title={
                  isTextSizingActive
                    ? 'Text font size'
                    : 'Annotation stroke width'
                }
                aria-label={
                  isTextSizingActive
                    ? 'Text font size'
                    : 'Annotation stroke width'
                }
                onChange={(event) => {
                  const value = Number(event.currentTarget.value);
                  if (textDraft) {
                    setTextFontSize(value);
                    setTextDraft((draft) =>
                      draft ? { ...draft, fontSize: value } : draft,
                    );
                    return;
                  }

                  if (isTextSizingActive) {
                    applySelectedAnnotationStyle(annotationStyle, value);
                    return;
                  }

                  applySelectedAnnotationStyle(
                    {
                      ...annotationStyle,
                      strokeWidth: value,
                    },
                    textFontSize,
                  );
                }}
              />
              <button
                type="button"
                className="h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50"
                disabled={isRenderingOutput}
                title="Copy"
                aria-label="Copy selection"
                onClick={copySelection}
              >
                Copy
              </button>
              <button
                type="button"
                className="h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50"
                disabled={isRenderingOutput}
                title="OCR"
                aria-label="Run OCR"
                onClick={runOcrSelection}
              >
                OCR
              </button>
              <button
                type="button"
                className="h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50"
                disabled={isRenderingOutput}
                title="Save"
                aria-label="Save selection"
                onClick={saveSelection}
              >
                Save
              </button>
              <button
                type="button"
                className="h-7 flex-1 rounded px-2 text-center leading-7 hover:bg-white/15 disabled:opacity-50"
                disabled={isRenderingOutput}
                title="Pin"
                aria-label="Pin selection"
                onClick={pinSelection}
              >
                Pin
              </button>
              <button
                type="button"
                className="h-7 w-7 rounded text-center leading-7 hover:bg-white/15 disabled:opacity-50"
                disabled={isRenderingOutput}
                title="Cancel"
                aria-label="Cancel capture"
                onClick={cancelSession}
              >
                X
              </button>
            </div>
          )}
          <div
            className="absolute rounded bg-black/80 px-2 py-1 text-xs leading-none text-white shadow"
            style={{
              left: `${selectionViewportRect.x}px`,
              top: `${Math.max(0, selectionViewportRect.y - 24)}px`,
            }}
          >
            {sizeLabel}
          </div>
          {isRenderingOutput && (
            <div
              className="absolute h-1 bg-white/80"
              style={{
                left: `${selectionViewportRect.x}px`,
                top: `${selectionViewportRect.y + selectionViewportRect.height}px`,
                width: `${selectionViewportRect.width}px`,
              }}
            />
          )}
        </>
      )}
      {cursorMonitor &&
        cursorViewportPoint &&
        cursorInMonitorPoint &&
        viewportBounds && (
        <Magnifier
          imageBase64={cursorMonitor.image_base64}
          viewportCursor={cursorViewportPoint}
          imageCursor={cursorInMonitorPoint}
          viewportBounds={viewportBounds}
          imageSize={{
            width: cursorMonitor.logical_bounds.width,
            height: cursorMonitor.logical_bounds.height,
          }}
          selection={selection ?? hoverSelection}
          color={cursorColor}
        />
      )}
    </div>
  );
}
