import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useSettingsStore } from '../../stores/settingsStore';
import {
  copyTextToClipboard,
  createCaptureSession,
  currentCaptureCursorPosition,
  getCaptureSession,
  logCaptureFrontendPerf,
  openCaptureOcrResultWindow,
  openCaptureTranslationResultWindow,
  outputCapture,
  renderCaptureOutput,
  runCaptureOcr,
} from '../../tauri/captureSession';
import {
  getToolbarPosition,
  constrainSelectionPoint,
  moveDraftSelectionByDelta,
  moveSelectionByDelta,
  normalizeSelection,
  nudgeDraftSelection,
  nudgeMovedSelection,
  nudgeResizedSelection,
  nudgeSelection,
  resizeSelectionBoundaryByArrow,
  resizeSelectionByHandle,
  restoreSelectionWithinBounds,
  snapMovedSelectionToRects,
  snapPointToRects,
  snapResizedSelectionToRects,
  type SelectionHandle,
} from './selection';
import {
  getMagnifierImageStyle,
  getMagnifierPosition,
  shouldShowMagnifier,
} from './magnifier';
import {
  colorSampleToClipboardText,
  type ColorSample,
  type ColorSampleFormat,
  isColorSampleCopyShortcut,
  isColorSampleFormatToggleShortcut,
  sampleCanvasColor,
} from './colorSampler';
import {
  buildCaptureCandidates,
  getBestCandidateAtPoint,
  getCandidateForPointerReleaseCompletion,
  getNextCandidateAtPoint,
} from './captureCandidates';
import {
  addAnnotationToHistory,
  clearAnnotationHistory,
  emptyAnnotationHistory,
  removeAnnotationFromHistory,
  replaceAnnotationInHistory,
  redoAnnotationHistory,
  undoAnnotationHistory,
} from './annotationHistory';
import { eraseAnnotationAtPoint } from './annotationEraser';
import {
  constrainAnnotationMoveDelta,
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
  appendAnnotationGesturePoint,
  appendAnnotationPoint,
  annotationColorFromShortcut,
  annotationFromGestureDraft,
  annotationSizeDirectionFromShortcut,
  annotationSizeDirectionFromWheel,
  annotationToolFromShortcut,
  annotationColorToCss,
  completeAnnotationGesture,
  annotationFromGesture,
  arrowHeadPoints,
  isAnnotationFillToggleShortcut,
  isPointStrokeAnnotationTool,
  nextAnnotationToolFromCycleShortcut,
  nextAnnotationStrokeWidth,
  nextTextFontSize,
  undoAnnotationGesturePoint,
  type AnnotationColor,
  type AnnotationGestureDraft,
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
  canToggleCapturedCursor,
  copyCaptureSelection,
  type CaptureCompletionAction,
  type HoverSelectionCompletionAction,
  getCaptureKeyboardToolbarAction,
  getCandidateCycleDirectionFromShortcut,
  getCancelCapturePointerAction,
  getCursorNudgeDeltaFromShortcut,
  getHoverSelectionCompletionActionFromShortcut,
  getSaveCapturePointerAction,
  getSelectionArrowActionFromShortcut,
  getSelectionHistoryStepFromShortcut,
  getUndoRedoActionFromShortcut,
  isCancelCapturePointer,
  isClearAnnotationsShortcut,
  isCopyCaptureDoubleClick,
  isCopyCaptureKeyboardShortcut,
  isDeleteSelectedAnnotationShortcut,
  isFinishAnnotationGestureDoubleClick,
  isMoveDraftSelectionShortcut,
  isMagnifierShortcut,
  isPinCapturePointer,
  isPinCaptureShortcut,
  isPrintCaptureShortcut,
  isQuickSaveCaptureShortcut,
  isRefreshCaptureShortcut,
  isSaveCaptureShortcut,
  isSelectAllCaptureShortcut,
  isToggleCapturedCursorShortcut,
  isUndoAnnotationGesturePointShortcut,
  printCaptureSelection,
  quickSaveCaptureSelection,
  refreshCaptureSession,
  saveCaptureSelection,
  shouldCancelCaptureOnBlur,
  shouldRestoreLastSelectionFromShortcut,
} from './captureActions';
import {
  shouldRecordSuccessfulCaptureCompletion,
} from './captureInteractionModel';
import {
  planManualSelectionCompletion,
  planCandidateSelectionCompletion,
  type CaptureRuntimeEffect,
} from './captureInteractionRuntime';
import {
  closeInactiveCaptureSession,
  finishCaptureSession,
} from './captureSessionLifecycle';
import {
  subscribeCaptureCancelRequests,
} from './captureCancelRequest';
import {
  getInitialHoverSelection,
  getPolledHoverSelection,
  shouldPollCaptureHoverSelection,
} from './captureHoverPolling';
import {
  getCaptureEditorCommandButtonClassName,
  getCaptureEditorDividerClassName,
  getCaptureEditorIconButtonClassName,
  getCaptureEditorSelectionClassName,
  getCaptureSelectionOverlayCanvasClassName,
  getCaptureEditorToolbarClassName,
  getCaptureRootClassName,
  shouldShowCaptureLoadingMask,
} from './capturePresentation';
import {
  drawCaptureSelectionOverlayFrame,
  getCaptureSelectionOverlayFrame,
  type CaptureSelectionOverlayFrame,
} from './captureSelectionOverlay';
import {
  revealCaptureWindow,
  revealCaptureWindowForSession,
  shouldRevealCaptureWindow,
  waitForCaptureSurfacePaint,
} from './captureWindowVisibility';
import { printBase64PngImage } from './capturePrint';
import {
  getSelectionHistoryEntry,
  loadCaptureSelectionHistory,
  loadLastCaptureSelection,
  saveLastCaptureSelection,
} from './selectionMemory';
import { parseCaptureLaunchPayload } from './windowMode';
import {
  getMonitorAtVirtualPoint,
  getCurrentMonitorBounds,
  getMonitorViewportRect,
  getVirtualDesktopBounds,
  nudgeVirtualPoint,
  viewportPointToVirtualPoint,
  virtualPointToViewportPoint,
  virtualRectToViewportRect,
} from './virtualDesktop';
import type {
  AnnotationCommand,
  CaptureMode,
  CaptureSessionView,
  ArrowKey,
  LogicalRect,
  MonitorSnapshotView,
  Point,
} from './types';

const captureWindow = getCurrentWebviewWindow();

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
type CaptureImageReadiness = {
  sessionId: string | null;
  monitorIds: Set<string>;
};
type CaptureFrontendPerfState = {
  mode: CaptureMode;
  sessionId: string | null;
  startMs: number;
  hasLoggedImagesReady: boolean;
};

const MIN_SELECTION_SIZE = 10;
const EDGE_SNAP_THRESHOLD = 6;
const KEYBOARD_NUDGE_STEP = 1;
const KEYBOARD_FAST_NUDGE_STEP = 10;
const TOOLBAR_GAP = 14;
const TOOLBAR_SIZE = { width: 1220, height: 56 };
const MAGNIFIER_GAP = 14;
const MAGNIFIER_SIZE = { width: 120, height: 96 };
const MAGNIFIER_ZOOM = 4;
const CAPTURE_HOVER_POLL_INTERVAL_MS = 16;
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

function svgPolylinePoints(points: Point[]) {
  return points.map((point) => `${point.x},${point.y}`).join(' ');
}

function PointerIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden="true">
      <path
        d="M7 4.8v13.9l3.2-3.5 2.4 4.9 2.3-1.1-2.4-4.8h5.1L7 4.8Z"
        fill="currentColor"
      />
    </svg>
  );
}

function RectangleIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden="true">
      <rect
        x="5"
        y="8"
        width="14"
        height="8"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      />
    </svg>
  );
}

function EllipseIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden="true">
      <circle
        cx="12"
        cy="12"
        r="7"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      />
    </svg>
  );
}

function ArrowIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden="true">
      <path
        d="M5 12h13m-5-5 5 5-5 5"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
      />
    </svg>
  );
}

function LineIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden="true">
      <path
        d="M5 12h14"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="2"
      />
    </svg>
  );
}

function PenIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden="true">
      <path
        d="M5 17.5 15.7 6.8l2.5 2.5L7.5 20H5v-2.5Z"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="2"
      />
      <path
        d="m14.5 8 1.5-1.5a1.8 1.8 0 0 1 2.5 2.5L17 10.5"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="2"
      />
    </svg>
  );
}

function TextIcon() {
  return (
    <span className="text-lg font-semibold leading-none" aria-hidden="true">
      T
    </span>
  );
}

function MosaicIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden="true">
      <path
        d="M5 5h14v14H5V5Zm4 0v14M15 5v14M5 9h14M5 15h14"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
    </svg>
  );
}

function BlurIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden="true">
      <circle
        cx="12"
        cy="12"
        r="7"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      />
      <path d="M12 5a7 7 0 0 1 0 14V5Z" fill="currentColor" opacity="0.45" />
    </svg>
  );
}

function EraserIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden="true">
      <path
        d="m5 15 8.6-8.6a2 2 0 0 1 2.8 0l2.2 2.2a2 2 0 0 1 0 2.8L12 18H8l-3-3Z"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="2"
      />
      <path
        d="M12 18h7"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="2"
      />
    </svg>
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
  colorFormat,
}: {
  imageBase64: string;
  viewportCursor: Point;
  imageCursor: Point;
  viewportBounds: LogicalRect;
  imageSize: { width: number; height: number };
  selection: LogicalRect | null;
  color: ColorSample | null;
  colorFormat: ColorSampleFormat;
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
  const colorText = color ? colorSampleToClipboardText(color, colorFormat) : '';

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
              <span>{colorText}</span>
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
  onInactive?: () => void | Promise<void>;
}

export default function ScreenshotSession({
  initialMode,
  initialSessionId,
  onInactive,
}: ScreenshotSessionProps) {
  const screenshotSavePath = useSettingsStore((state) => state.screenshotSavePath);
  const sampleCanvasByMonitorRef = useRef<Map<string, HTMLCanvasElement>>(new Map());
  const selectionOverlayCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const selectionOverlayAnimationFrameRef = useRef<number | null>(null);
  const selectionOverlayFrameRef = useRef<CaptureSelectionOverlayFrame | null>(null);
  const startPointRef = useRef<Point | null>(null);
  const cursorPointRef = useRef<Point | null>(null);
  const draftSelectionRef = useRef<LogicalRect | null>(null);
  const hoverSelectionRef = useRef<LogicalRect | null>(null);
  const textDraftInputRef = useRef<HTMLTextAreaElement | null>(null);
  const keyboardDraftCursorPointRef = useRef<Point | null>(null);
  const keyboardEditCursorPointRef = useRef<Point | null>(null);
  const isCancellingSessionRef = useRef(false);
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
    useState<AnnotationGestureDraft | null>(null);
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
  const [isAnnotationToolbarVisible, setIsAnnotationToolbarVisible] = useState(true);
  const [cursorColor, setCursorColor] = useState<ColorSample | null>(null);
  const [colorSampleFormat, setColorSampleFormat] =
    useState<ColorSampleFormat>('hex');
  const [isMagnifierRequested, setIsMagnifierRequested] = useState(false);
  const [sampleCanvasVersion, setSampleCanvasVersion] = useState(0);
  const [isRenderingOutput, setIsRenderingOutput] = useState(false);
  const [includeCapturedCursor, setIncludeCapturedCursor] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasStartedInitialSession, setHasStartedInitialSession] = useState(false);
  const [captureImageReadiness, setCaptureImageReadiness] =
    useState<CaptureImageReadiness>(() => ({
      sessionId: null,
      monitorIds: new Set(),
    }));
  const hasRevealedCaptureWindowRef = useRef(false);
  const captureFrontendPerfRef = useRef<CaptureFrontendPerfState | null>(null);

  const isActive = status !== 'idle';
  const annotations = annotationHistory.annotations;
  const shouldIncludeCapturedCursor =
    includeCapturedCursor && canToggleCapturedCursor(session);
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
    const imageMonitors = session.monitors.filter((monitor) => monitor.image_base64);
    if (imageMonitors.length === 0) return true;
    if (captureImageReadiness.sessionId !== session.id) return false;

    return imageMonitors.every((monitor) =>
      captureImageReadiness.monitorIds.has(monitor.id),
    );
  }, [captureImageReadiness, session]);
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
  const isMagnifierShown = shouldShowMagnifier({
    requested: isMagnifierRequested,
    automatic: false,
    hasCursorMonitor: Boolean(cursorMonitor?.image_base64),
    hasViewportCursor: Boolean(cursorViewportPoint),
    hasImageCursor: Boolean(cursorInMonitorPoint),
    hasViewportBounds: Boolean(viewportBounds),
  });

  const setStartPointWithRef = useCallback((point: Point | null) => {
    startPointRef.current = point;
    setStartPoint(point);
  }, []);

  const paintSelectionOverlayFrame = useCallback(
    (frame: CaptureSelectionOverlayFrame | null) => {
      const canvas = selectionOverlayCanvasRef.current;
      const context = canvas?.getContext('2d');
      if (!canvas || !context) return;

      drawCaptureSelectionOverlayFrame(
        context,
        { width: canvas.width, height: canvas.height },
        frame,
      );
    },
    [],
  );

  const scheduleSelectionOverlayPaint = useCallback(
    (
      draftSelection: LogicalRect | null = draftSelectionRef.current,
      hoverSelection: LogicalRect | null = hoverSelectionRef.current,
      activeSelection: LogicalRect | null = selection,
    ) => {
      selectionOverlayFrameRef.current = getCaptureSelectionOverlayFrame({
        status,
        selectionBounds,
        selection: activeSelection,
        draftSelection,
        hoverSelection,
      });

      if (selectionOverlayAnimationFrameRef.current !== null) return;

      selectionOverlayAnimationFrameRef.current = window.requestAnimationFrame(() => {
        selectionOverlayAnimationFrameRef.current = null;
        paintSelectionOverlayFrame(selectionOverlayFrameRef.current);
      });
    },
    [paintSelectionOverlayFrame, selection, selectionBounds, status],
  );

  useEffect(() => {
    scheduleSelectionOverlayPaint();
  }, [scheduleSelectionOverlayPaint, selection, viewportBounds]);

  const syncHoverSelection = useCallback(
    (nextHoverSelection: LogicalRect | null) => {
      if (areRectsEqual(hoverSelectionRef.current, nextHoverSelection)) return;

      hoverSelectionRef.current = nextHoverSelection;
      setHoverSelection(nextHoverSelection);
      scheduleSelectionOverlayPaint(null, nextHoverSelection, null);
    },
    [scheduleSelectionOverlayPaint],
  );

  useEffect(() => {
    if (!session || !selectionBounds) return;

    let disposed = false;
    let timeoutId: number | null = null;

    const canPoll = () =>
      shouldPollCaptureHoverSelection({
        status,
        hasSession: Boolean(session),
        hasSelectionBounds: Boolean(selectionBounds),
        hasActiveStartPoint: Boolean(startPointRef.current ?? startPoint),
        hasEditGesture: Boolean(editGesture),
      });

    const scheduleNextPoll = () => {
      if (disposed) return;

      timeoutId = window.setTimeout(poll, CAPTURE_HOVER_POLL_INTERVAL_MS);
    };

    const poll = async () => {
      if (!canPoll()) return;

      try {
        const point = await currentCaptureCursorPosition(session.id);
        if (disposed || !canPoll()) return;

        if (!point) {
          syncHoverSelection(null);
          scheduleNextPoll();
          return;
        }

        cursorPointRef.current = point;
        if (isMagnifierRequested) {
          setCursorPoint(point);
        }
        syncHoverSelection(getPolledHoverSelection(captureCandidates, point));
        scheduleNextPoll();
      } catch {
        syncHoverSelection(null);
      }
    };

    if (canPoll()) {
      timeoutId = window.setTimeout(poll, 0);
    }

    return () => {
      disposed = true;
      if (timeoutId !== null) {
        window.clearTimeout(timeoutId);
      }
    };
  }, [
    captureCandidates,
    editGesture,
    isMagnifierRequested,
    selectionBounds,
    session,
    startPoint,
    status,
    syncHoverSelection,
  ]);

  useEffect(() => {
    return () => {
      if (selectionOverlayAnimationFrameRef.current !== null) {
        window.cancelAnimationFrame(selectionOverlayAnimationFrameRef.current);
      }
    };
  }, []);

  const resetCaptureImageReadiness = useCallback(() => {
    setCaptureImageReadiness({
      sessionId: null,
      monitorIds: new Set(),
    });
  }, []);

  const markCaptureImageReady = useCallback((sessionId: string, monitorId: string) => {
    setCaptureImageReadiness((previous) => {
      const monitorIds =
        previous.sessionId === sessionId ? previous.monitorIds : new Set<string>();
      if (monitorIds.has(monitorId)) return previous;

      const nextMonitorIds = new Set(monitorIds);
      nextMonitorIds.add(monitorId);

      return {
        sessionId,
        monitorIds: nextMonitorIds,
      };
    });
  }, []);

  const resetCaptureInteractionState = useCallback(() => {
    startPointRef.current = null;
    cursorPointRef.current = null;
    draftSelectionRef.current = null;
    hoverSelectionRef.current = null;
    selectionOverlayFrameRef.current = null;
    paintSelectionOverlayFrame(null);
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
    setIsAnnotationToolbarVisible(true);
    setCursorColor(null);
    setColorSampleFormat('hex');
    setIsMagnifierRequested(false);
    setSampleCanvasVersion(0);
    setIsRenderingOutput(false);
    setIncludeCapturedCursor(false);
    setError(null);
    resetCaptureImageReadiness();
  }, [paintSelectionOverlayFrame, resetCaptureImageReadiness]);

  const resetSessionState = useCallback(() => {
    setStatus('idle');
    setSession(null);
    resetCaptureInteractionState();
  }, [resetCaptureInteractionState]);

  const primeInitialHoverSelection = useCallback((
    nextSession: CaptureSessionView,
    initialCursorPosition?: Point | null,
  ) => {
    const cursorPosition =
      nextSession.captured_cursor?.logical_position ?? initialCursorPosition ?? null;
    const initialHoverSelection = getInitialHoverSelection(
      buildCaptureCandidates(nextSession.monitors, nextSession.candidates),
      cursorPosition
        ? {
            logical_position: cursorPosition,
            hotspot: { x: 0, y: 0 },
            image_width: 0,
            image_height: 0,
            scale_factor: 1,
            image_base64: '',
          }
        : null,
    );

    cursorPointRef.current = cursorPosition;
    hoverSelectionRef.current = initialHoverSelection;
    setHoverSelection(initialHoverSelection);
  }, []);

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

  const finishCurrentCaptureSession = useCallback(
    async (sessionId: string) => {
      await finishCaptureSession({
        sessionId,
        onInactive,
        resetSessionState,
      });
    },
    [onInactive, resetSessionState],
  );

  const cancelSession = useCallback(async () => {
    if (isCancellingSessionRef.current) return;
    isCancellingSessionRef.current = true;

    const sessionId = session?.id;

    try {
      if (sessionId) {
        await finishCaptureSession({
          sessionId,
          onInactive,
          resetSessionState,
        });
      } else {
        await closeInactiveCaptureSession({ onInactive, resetSessionState });
      }
    } catch (err) {
      isCancellingSessionRef.current = false;
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, [onInactive, resetSessionState, session?.id]);

  const startSession = useCallback(async (nextMode: CaptureMode, sessionId?: string) => {
    isCancellingSessionRef.current = false;
    hasRevealedCaptureWindowRef.current = false;
    setStatus('loading');
    setMode(nextMode);
    resetCaptureInteractionState();
    captureFrontendPerfRef.current = {
      mode: nextMode,
      sessionId: sessionId ?? null,
      startMs: performance.now(),
      hasLoggedImagesReady: false,
    };
    markCaptureFrontendPerf('start_session', sessionId);

    try {
      const nextSession = sessionId
        ? await getCaptureSession(sessionId)
        : await createCaptureSession();
      if (captureFrontendPerfRef.current) {
        captureFrontendPerfRef.current.sessionId = nextSession.id;
      }
      markCaptureFrontendPerf('session_loaded', nextSession.id);
      const initialCursorPosition = nextSession.captured_cursor
        ? null
        : await currentCaptureCursorPosition(nextSession.id).catch(() => null);
      primeInitialHoverSelection(nextSession, initialCursorPosition);
      setSession(nextSession);
      setStatus('selecting');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, [
    markCaptureFrontendPerf,
    primeInitialHoverSelection,
    resetCaptureInteractionState,
  ]);

  const recordLastSelection = useCallback((rect: LogicalRect) => {
    try {
      saveLastCaptureSelection(window.localStorage, rect);
    } catch (err) {
      console.warn('Failed to remember capture selection:', err);
    }
  }, []);

  const recordSuccessfulSelection = useCallback(
    (
      action: CaptureCompletionAction,
      rect: LogicalRect,
    ) => {
      if (!shouldRecordSuccessfulCaptureCompletion(action)) return;
      recordLastSelection(rect);
    },
    [recordLastSelection],
  );

  const executeCaptureRuntimeEffect = useCallback(
    async (effect: CaptureRuntimeEffect, rect: LogicalRect) => {
      if (!session) return;

      if (effect.type === 'output-capture') {
        if (effect.action === 'copy') {
          await copyCaptureSelection(
            session.id,
            rect,
            [],
            shouldIncludeCapturedCursor,
          );
        } else if (effect.action === 'save') {
          await saveCaptureSelection(
            session.id,
            rect,
            [],
            shouldIncludeCapturedCursor,
          );
        } else if (effect.action === 'quick-save') {
          await quickSaveCaptureSelection(
            session.id,
            rect,
            [],
            screenshotSavePath,
            shouldIncludeCapturedCursor,
          );
        } else if (effect.action === 'print') {
          await printCaptureSelection(
            session.id,
            rect,
            [],
            printBase64PngImage,
            shouldIncludeCapturedCursor,
          );
        } else if (effect.action === 'pin') {
          await outputCapture({
            sessionId: session.id,
            rect,
            annotations: [],
            ...(shouldIncludeCapturedCursor ? { includeCursor: true } : {}),
            action: { type: 'pin' },
          });
        }
        return;
      }

      if (effect.type === 'run-ocr') {
        const ocrResult = await runCaptureOcr(session.id, rect);
        if (effect.resultWindow === 'translation') {
          await openCaptureTranslationResultWindow(ocrResult.text);
        } else if (effect.resultWindow === 'ocr') {
          await openCaptureOcrResultWindow(ocrResult.text);
        } else {
          await copyTextToClipboard(ocrResult.text);
        }
        return;
      }

      if (effect.type === 'record-selection') {
        recordSuccessfulSelection(effect.action, rect);
        return;
      }

      await finishCurrentCaptureSession(session.id);
    },
    [
      finishCurrentCaptureSession,
      recordSuccessfulSelection,
      screenshotSavePath,
      session,
      shouldIncludeCapturedCursor,
    ],
  );

  const renderSelectionPreview = useCallback(
    async (
      rect: LogicalRect,
      nextAnnotations: AnnotationCommand[] = annotations,
      includeCursor = shouldIncludeCapturedCursor,
    ) => {
      if (!session) return;

      setIsRenderingOutput(true);
      setPreviewImageBase64(null);
      setError(null);

      try {
        const base64 = await renderCaptureOutput({
          sessionId: session.id,
          rect,
          annotations: nextAnnotations,
          ...(includeCursor ? { includeCursor } : {}),
        });
        setPreviewImageBase64(base64);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setStatus('error');
      } finally {
        setIsRenderingOutput(false);
      }
    },
    [
      annotations,
      session,
      shouldIncludeCapturedCursor,
    ],
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
      await copyCaptureSelection(
        session.id,
        selection,
        outputHistory.annotations,
        shouldIncludeCapturedCursor,
      );
      recordSuccessfulSelection('copy', selection);
      await finishCurrentCaptureSession(session.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [
    commitTextDraftToHistory,
    finishCurrentCaptureSession,
    recordSuccessfulSelection,
    selection,
    session,
    shouldIncludeCapturedCursor,
  ]);

  const completeCandidateSelection = useCallback(async (
    rect: LogicalRect,
    action: HoverSelectionCompletionAction,
  ) => {
    if (!session) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      for (const effect of planCandidateSelectionCompletion(action)) {
        await executeCaptureRuntimeEffect(effect, rect);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [
    executeCaptureRuntimeEffect,
    session,
  ]);

  const copyCurrentColor = useCallback(async () => {
    if (!cursorColor) return;

    try {
      await navigator.clipboard.writeText(
        colorSampleToClipboardText(cursorColor, colorSampleFormat),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, [colorSampleFormat, cursorColor]);

  const saveSelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      const outputHistory = commitTextDraftToHistory();
      await saveCaptureSelection(
        session.id,
        selection,
        outputHistory.annotations,
        shouldIncludeCapturedCursor,
      );
      recordSuccessfulSelection('save', selection);
      await finishCurrentCaptureSession(session.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [
    commitTextDraftToHistory,
    finishCurrentCaptureSession,
    recordSuccessfulSelection,
    selection,
    session,
    shouldIncludeCapturedCursor,
  ]);

  const quickSaveSelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      const outputHistory = commitTextDraftToHistory();
      await quickSaveCaptureSelection(
        session.id,
        selection,
        outputHistory.annotations,
        screenshotSavePath,
        shouldIncludeCapturedCursor,
      );
      recordSuccessfulSelection('quick-save', selection);
      await finishCurrentCaptureSession(session.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [
    commitTextDraftToHistory,
    finishCurrentCaptureSession,
    recordSuccessfulSelection,
    screenshotSavePath,
    selection,
    session,
    shouldIncludeCapturedCursor,
  ]);

  const runOcrSelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      const ocrResult = await runCaptureOcr(session.id, selection);
      await openCaptureOcrResultWindow(ocrResult.text);
      recordSuccessfulSelection('ocr', selection);
      await finishCurrentCaptureSession(session.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [
    finishCurrentCaptureSession,
    recordSuccessfulSelection,
    selection,
    session,
  ]);

  const pinSelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      const outputHistory = commitTextDraftToHistory();
      await outputCapture({
        sessionId: session.id,
        rect: selection,
        annotations: outputHistory.annotations,
        ...(shouldIncludeCapturedCursor ? { includeCursor: true } : {}),
        action: { type: 'pin' },
      });
      recordSuccessfulSelection('pin', selection);
      await finishCurrentCaptureSession(session.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [
    commitTextDraftToHistory,
    finishCurrentCaptureSession,
    recordSuccessfulSelection,
    selection,
    session,
    shouldIncludeCapturedCursor,
  ]);

  const printSelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      const outputHistory = commitTextDraftToHistory();
      await printCaptureSelection(
        session.id,
        selection,
        outputHistory.annotations,
        printBase64PngImage,
        shouldIncludeCapturedCursor,
      );
      recordSuccessfulSelection('print', selection);
      await finishCurrentCaptureSession(session.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    } finally {
      setIsRenderingOutput(false);
    }
  }, [
    commitTextDraftToHistory,
    finishCurrentCaptureSession,
    recordSuccessfulSelection,
    selection,
    session,
    shouldIncludeCapturedCursor,
  ]);

  const refreshSession = useCallback(async () => {
    if (!session) return;

    hasRevealedCaptureWindowRef.current = false;
    setStatus('loading');
    resetCaptureInteractionState();

    try {
      const nextSession = await refreshCaptureSession(session.id);
      const initialCursorPosition = nextSession.captured_cursor
        ? null
        : await currentCaptureCursorPosition(nextSession.id).catch(() => null);
      primeInitialHoverSelection(nextSession, initialCursorPosition);
      setSession(nextSession);
      setStatus('selecting');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, [primeInitialHoverSelection, resetCaptureInteractionState, session]);

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

    const nextGesture = undoAnnotationGesturePoint(annotationGesture);
    if (!nextGesture) {
      setAnnotationGesture(null);
      setDraftAnnotation(null);
      return true;
    }

    const fallbackPoint =
      nextGesture.points?.[nextGesture.points.length - 1] ?? nextGesture.startPoint;
    const localPoint = cursorPoint
      ? clampPointToRect(
          { x: cursorPoint.x - selection.x, y: cursorPoint.y - selection.y },
          selection,
        )
      : fallbackPoint;

    setAnnotationGesture(nextGesture);
    setDraftAnnotation(
      annotationFromGestureDraft(nextGesture, localPoint, annotationStyle),
    );
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
        filled:
          annotation.type === 'rectangle' || annotation.type === 'ellipse'
            ? annotation.filled
            : false,
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

  const toggleAnnotationFill = useCallback(() => {
    if (textDraft || !isFillModeActive) return;

    applySelectedAnnotationStyle(
      {
        ...annotationStyle,
        filled: !annotationStyle.filled,
      },
      textFontSize,
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

  const commitAnnotationGestureAtPoint = useCallback(
    (localPoint: Point, constrainGesture: boolean) => {
      if (!annotationGesture || !selection) return false;

      const nextAnnotation = completeAnnotationGesture(
        annotationGesture,
        localPoint,
        annotationStyle,
        constrainGesture,
      );
      setAnnotationGesture(null);
      setDraftAnnotation(null);
      if (!nextAnnotation) return true;

      const nextHistory = addAnnotationToHistory(annotationHistory, nextAnnotation);
      setSelectedAnnotationIndex(null);
      setAnnotationHistory(nextHistory);
      void renderSelectionPreview(selection, nextHistory.annotations);
      return true;
    },
    [
      annotationGesture,
      annotationHistory,
      annotationStyle,
      renderSelectionPreview,
      selection,
    ],
  );

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

  const resetPreviewSelection = useCallback(() => {
    startPointRef.current = null;
    cursorPointRef.current = null;
    draftSelectionRef.current = null;
    hoverSelectionRef.current = null;
    selectionOverlayFrameRef.current = null;
    paintSelectionOverlayFrame(null);
    setStartPoint(null);
    setCursorPoint(null);
    setSelection(null);
    setHoverSelection(null);
    setEditGesture(null);
    setPreviewImageBase64(null);
    setIsRenderingOutput(false);
    setActiveAnnotationTool(null);
    setAnnotationGesture(null);
    setDraftAnnotation(null);
    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setDraftSelectionMoveGesture(null);
    setTextDraft(null);
    setTextDraftAnnotationIndex(null);
    setAnnotationHistory(emptyAnnotationHistory());
    setIsMagnifierRequested(false);
    setStatus('selecting');
  }, [paintSelectionOverlayFrame]);

  const completeManualSelection = useCallback((rect: LogicalRect) => {
    const completion = planManualSelectionCompletion(mode);

    startPointRef.current = null;
    draftSelectionRef.current = null;
    hoverSelectionRef.current = null;
    if (completion.type !== 'preview') {
      selectionOverlayFrameRef.current = null;
      paintSelectionOverlayFrame(null);
    }
    setStartPoint(null);
    setSelection(rect);
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
    setIsMagnifierRequested(false);

    if (completion.type === 'preview') {
      setIsAnnotationToolbarVisible(true);
      setStatus('preview');
      void renderSelectionPreview(rect, []);
      return;
    }

    if (!session) return;

    setIsAnnotationToolbarVisible(false);
    setStatus('selecting');
    setIsRenderingOutput(true);
    setError(null);

    void (async () => {
      try {
        for (const effect of completion.effects) {
          await executeCaptureRuntimeEffect(effect, rect);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setStatus('error');
      } finally {
        setIsRenderingOutput(false);
      }
    })();
  }, [
    executeCaptureRuntimeEffect,
    mode,
    paintSelectionOverlayFrame,
    renderSelectionPreview,
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

    const savedSelection = loadLastCaptureSelection(window.localStorage);
    if (!savedSelection) return;

    const restoredSelection = restoreSelectionWithinBounds(
      savedSelection,
      selectionBounds,
      MIN_SELECTION_SIZE,
    );
    if (!restoredSelection) return;

    completeManualSelection(restoredSelection);
  }, [completeManualSelection, selectionBounds]);

  const restoreSelectionFromHistory = useCallback(
    (step: ReturnType<typeof getSelectionHistoryStepFromShortcut>) => {
      if (!step || !selectionBounds) return;

      const historySelection = getSelectionHistoryEntry(
        loadCaptureSelectionHistory(window.localStorage),
        selection,
        step,
      );
      if (!historySelection) return;

      const restoredSelection = restoreSelectionWithinBounds(
        historySelection,
        selectionBounds,
        MIN_SELECTION_SIZE,
      );
      if (!restoredSelection) return;

      completeManualSelection(restoredSelection);
    },
    [completeManualSelection, selection, selectionBounds],
  );

  const prepareCaptureSurfaceForReveal = useCallback(async () => {
    paintSelectionOverlayFrame(selectionOverlayFrameRef.current);
    await waitForCaptureSurfacePaint();
  }, [paintSelectionOverlayFrame]);

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

  useEffect(() => {
    if (
      !shouldRevealCaptureWindow({
        status,
        hasSession: Boolean(session),
        hasCaptureImagesReady: areCaptureImagesReady,
        hasRevealed: hasRevealedCaptureWindowRef.current,
      })
    ) {
      return;
    }

    hasRevealedCaptureWindowRef.current = true;
    if (!session) {
      void revealCaptureWindow(captureWindow)
        .catch((err) => {
          setError(err instanceof Error ? err.message : String(err));
          setStatus('error');
        });
      return;
    }

    void revealCaptureWindowForSession({
      window: captureWindow,
      sessionId: session.id,
      prepareSurface: prepareCaptureSurfaceForReveal,
    })
      .then(() => {
        markCaptureFrontendPerf('revealed', session.id);
      })
      .catch((err) => {
        setError(err instanceof Error ? err.message : String(err));
        setStatus('error');
      });
  }, [
    areCaptureImagesReady,
    markCaptureFrontendPerf,
    prepareCaptureSurfaceForReveal,
    session,
    status,
  ]);

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
    if (!isActive) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    subscribeCaptureCancelRequests(cancelSession)
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for native capture cancel requests:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [cancelSession, isActive]);

  useEffect(() => {
    sampleCanvasByMonitorRef.current = new Map();
    setCursorColor(null);
    setSampleCanvasVersion((version) => version + 1);
  }, [session?.id]);

  useEffect(() => {
    if (!session || !isMagnifierRequested) return;

    let disposed = false;
    session.monitors.forEach((monitor) => {
      if (sampleCanvasByMonitorRef.current.has(monitor.id)) return;

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
  }, [isMagnifierRequested, session]);

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
        const result = nudgeDraftSelection(
          activeStartPoint,
          activeCursorPoint,
          cursorNudgeDelta,
          selectionBounds,
        );
        keyboardDraftCursorPointRef.current = result.cursorPoint;
        cursorPointRef.current = result.cursorPoint;
        draftSelectionRef.current = result.selection;
        setCursorPoint(result.cursorPoint);
        setSelection(result.selection);
        scheduleSelectionOverlayPaint(result.selection, null);
        setPreviewImageBase64(null);
        setIsRenderingOutput(false);
      } else if (
        status === 'preview' &&
        !textDraft &&
        editGesture?.type === 'move' &&
        selection &&
        cursorPoint &&
        selectionBounds &&
        cursorNudgeDelta
      ) {
        event.preventDefault();
        const result = nudgeMovedSelection(
          selection,
          cursorPoint,
          cursorNudgeDelta,
          selectionBounds,
        );
        keyboardEditCursorPointRef.current = result.cursorPoint;
        setCursorPoint(result.cursorPoint);
        setSelection(result.selection);
        setEditGesture({
          ...editGesture,
          startPoint: result.cursorPoint,
          startSelection: result.selection,
        });
        setPreviewImageBase64(null);
        setIsRenderingOutput(false);
      } else if (
        status === 'preview' &&
        !textDraft &&
        editGesture?.type === 'resize' &&
        selection &&
        cursorPoint &&
        selectionBounds &&
        cursorNudgeDelta
      ) {
        event.preventDefault();
        const result = nudgeResizedSelection(
          selection,
          cursorPoint,
          editGesture.handle,
          cursorNudgeDelta,
          selectionBounds,
          MIN_SELECTION_SIZE,
          event.shiftKey,
        );
        keyboardEditCursorPointRef.current = result.cursorPoint;
        setCursorPoint(result.cursorPoint);
        setSelection(result.selection);
        setEditGesture({
          ...editGesture,
          startPoint: result.cursorPoint,
          startSelection: result.selection,
        });
        setPreviewImageBase64(null);
        setIsRenderingOutput(false);
      } else if (
        status === 'selecting' &&
        !textDraft &&
        activeCursorPoint &&
        selectionBounds &&
        cursorNudgeDelta
      ) {
        event.preventDefault();
        const nextCursorPoint = nudgeVirtualPoint(
          activeCursorPoint,
          cursorNudgeDelta,
          selectionBounds,
        );
        cursorPointRef.current = nextCursorPoint;
        setCursorPoint(nextCursorPoint);
      } else if (
        status === 'selecting' &&
        !textDraft &&
        activeCursorPoint &&
        candidateCycleDirection
      ) {
        event.preventDefault();
        const nextHoverSelection =
          getNextCandidateAtPoint(
            captureCandidates,
            activeCursorPoint,
            activeHoverSelection,
            candidateCycleDirection,
          )?.rect ?? null;
        syncHoverSelection(nextHoverSelection);
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
        isCopyCaptureKeyboardShortcut(event)
      ) {
        event.preventDefault();
        void copySelection();
      } else if (status === 'preview' && isQuickSaveCaptureShortcut(event)) {
        event.preventDefault();
        void quickSaveSelection();
      } else if (status === 'preview' && isSaveCaptureShortcut(event)) {
        event.preventDefault();
        void saveSelection();
      } else if (status === 'preview' && isPinCaptureShortcut(event)) {
        event.preventDefault();
        void pinSelection();
      } else if (status === 'preview' && isPrintCaptureShortcut(event)) {
        event.preventDefault();
        void printSelection();
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
        setActiveAnnotationTool(cycledAnnotationTool);
        setSelectedAnnotationIndex(null);
        setAnnotationGesture(null);
        setAnnotationMoveGesture(null);
        setDraftAnnotation(null);
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
        setDraftSelectionMoveGesture({
          startPoint: activeCursorPoint,
          startSelection: activeDraftSelection,
          startAnchorPoint: activeStartPoint,
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
      } else if (
        status === 'preview' &&
        selection &&
        selectionBounds &&
        selectionArrowAction
      ) {
        event.preventDefault();
        const nextSelection = selectionArrowAction.mode === 'expand'
          ? resizeSelectionBoundaryByArrow(
              selection,
              selectionArrowAction.direction,
              'expand',
              selectionBounds,
              MIN_SELECTION_SIZE,
            )
          : selectionArrowAction.mode === 'shrink'
            ? resizeSelectionBoundaryByArrow(
                selection,
                selectionArrowAction.direction,
                'shrink',
                selectionBounds,
                MIN_SELECTION_SIZE,
              )
            : nudgeSelection(
                selection,
                selectionArrowAction.direction,
                selectionBounds,
                KEYBOARD_NUDGE_STEP,
              );
        setSelection(nextSelection);
        setPreviewImageBase64(null);
        void renderSelectionPreview(nextSelection);
      }
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      if (event.key === 'Alt') {
        setIsMagnifierRequested(false);
      }

      if (event.key === ' ' && draftSelectionMoveGesture) {
        event.preventDefault();
        setDraftSelectionMoveGesture(null);
      }
    };

    const handleWindowBlur = () => {
      setIsMagnifierRequested(false);
      if (shouldCancelCaptureOnBlur({ status })) {
        void cancelSession();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    window.addEventListener('blur', handleWindowBlur);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      window.removeEventListener('blur', handleWindowBlur);
    };
  }, [
    adjustAnnotationSize,
    clearAnnotations,
    cancelSession,
    completeCandidateSelection,
    copyCurrentColor,
    copySelection,
    colorSampleFormat,
    activeAnnotationTool,
    annotationGesture,
    annotationMoveGesture,
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
    isActive,
    nudgeSelectedAnnotation,
    pinSelection,
    printSelection,
    quickSaveSelection,
    refreshSession,
    restoreLastSelection,
    restoreSelectionFromHistory,
    saveSelection,
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
      const action = getCancelCapturePointerAction({
        status,
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

      if (action === 'finish-edit') {
        commitTextDraft();
      } else if (action === 'finish-annotation') {
        if (selection && selectionBounds && annotationGesture) {
          const point = viewportPointToVirtualPoint(
            { x: event.clientX, y: event.clientY },
            selectionBounds,
          );
          const localPoint = clampPointToRect(
            { x: point.x - selection.x, y: point.y - selection.y },
            selection,
          );
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

    if ((status !== 'selecting' && status !== 'preview') || !selectionBounds) return;

    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    const snappedPoint = snapPointToRects(point, snapTargetRects, EDGE_SNAP_THRESHOLD);
    const draftSelection = normalizeSelection(snappedPoint, snappedPoint);
    cursorPointRef.current = point;
    draftSelectionRef.current = draftSelection;
    setCursorPoint(point);
    event.currentTarget.setPointerCapture(event.pointerId);
    setStartPointWithRef(snappedPoint);
    setSelection(null);
    setHoverSelection(null);
    scheduleSelectionOverlayPaint(draftSelection, null);
    setPreviewImageBase64(null);
    setIsRenderingOutput(false);
    setStatus('selecting');
    setActiveAnnotationTool(null);
    setAnnotationGesture(null);
    setDraftAnnotation(null);
    setSelectedAnnotationIndex(null);
    setAnnotationMoveGesture(null);
    setDraftSelectionMoveGesture(null);
    keyboardDraftCursorPointRef.current = null;
    keyboardEditCursorPointRef.current = null;
    setTextDraft(null);
    setTextDraftAnnotationIndex(null);
    setAnnotationHistory(emptyAnnotationHistory());
  };

  const applyEditGesture = useCallback(
    (gesture: EditGesture, point: Point, preserveAspect = false) => {
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

      const shouldPreserveAspect = preserveAspect && gesture.handle.length === 2;
      const resizedSelection = resizeSelectionByHandle(
        gesture.startSelection,
        gesture.handle,
        delta,
        selectionBounds,
        MIN_SELECTION_SIZE,
        shouldPreserveAspect,
      );
      if (shouldPreserveAspect) return resizedSelection;

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

    cursorPointRef.current = point;

    if (status === 'preview' || (status === 'selecting' && isMagnifierRequested)) {
      setCursorPoint(point);
    }

    const activeStartPoint = startPointRef.current ?? startPoint;

    if (!activeStartPoint && !editGesture && status === 'selecting') {
      const nextHoverCandidate = getBestCandidateAtPoint(captureCandidates, point);
      const nextHoverSelection = nextHoverCandidate?.rect ?? null;
      syncHoverSelection(nextHoverSelection);
    }

    if (annotationGesture && selection) {
      const localPoint = clampPointToRect(
        { x: point.x - selection.x, y: point.y - selection.y },
        selection,
      );
      const points = isPointStrokeAnnotationTool(annotationGesture.tool)
        ? appendAnnotationPoint(annotationGesture.points ?? [], localPoint)
        : undefined;
      if (points && !event.shiftKey) {
        setAnnotationGesture({
          ...annotationGesture,
          points,
        });
      }
      setDraftAnnotation(
        annotationFromGestureDraft(
          annotationGesture,
          localPoint,
          annotationStyle,
          event.shiftKey,
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
      const moveDelta = event.shiftKey ? constrainAnnotationMoveDelta(delta) : delta;
      setPreviewImageBase64(null);
      setDraftAnnotation(
        moveAnnotationByDelta(annotationMoveGesture.startAnnotation, moveDelta),
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
      draftSelectionRef.current = result.selection;
      startPointRef.current = result.anchorPoint;
      scheduleSelectionOverlayPaint(result.selection, null);
      setPreviewImageBase64(null);
      setIsRenderingOutput(false);
      return;
    }

    if (editGesture) {
      keyboardEditCursorPointRef.current = null;
      setSelection(applyEditGesture(editGesture, point, event.shiftKey));
      setPreviewImageBase64(null);
      setIsRenderingOutput(false);
      return;
    }

    if (!activeStartPoint || status !== 'selecting') return;

    keyboardDraftCursorPointRef.current = null;
    const currentPoint = snapPointToRects(point, snapTargetRects, EDGE_SNAP_THRESHOLD);
    const nextDraftSelection = normalizeSelection(
      activeStartPoint,
      event.shiftKey
        ? constrainSelectionPoint(activeStartPoint, currentPoint)
        : currentPoint,
    );
    draftSelectionRef.current = nextDraftSelection;
    scheduleSelectionOverlayPaint(nextDraftSelection, null);
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

    if (annotationGesture && selection) {
      const localPoint = clampPointToRect(
        { x: point.x - selection.x, y: point.y - selection.y },
        selection,
      );
      if (annotationGesture.tool === 'polyline') return;

      commitAnnotationGestureAtPoint(localPoint, event.shiftKey);
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
      const moveDelta = event.shiftKey ? constrainAnnotationMoveDelta(delta) : delta;
      const nextAnnotation = moveAnnotationByDelta(
        annotationMoveGesture.startAnnotation,
        moveDelta,
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
      const nextSelection = applyEditGesture(
        editGesture,
        editReleasePoint,
        event.shiftKey,
      );
      setEditGesture(null);
      setSelection(nextSelection);
      setStatus('preview');
      void renderSelectionPreview(nextSelection, annotations);
      return;
    }

    const activeStartPoint = startPointRef.current ?? startPoint;

    if (!activeStartPoint || status !== 'selecting') return;

    const currentPoint = snapPointToRects(
      selectionReleasePoint,
      snapTargetRects,
      EDGE_SNAP_THRESHOLD,
    );
    const nextSelection = normalizeSelection(
      activeStartPoint,
      event.shiftKey
        ? constrainSelectionPoint(activeStartPoint, currentPoint)
        : currentPoint,
    );
    setStartPointWithRef(null);
    draftSelectionRef.current = null;
    scheduleSelectionOverlayPaint(null, hoverSelectionRef.current);

    const activeHoverSelection = hoverSelectionRef.current ?? hoverSelection;
    const candidateForPointerCompletion =
      getCandidateForPointerReleaseCompletion(
        captureCandidates,
        selectionReleasePoint,
        activeHoverSelection,
        nextSelection,
        MIN_SELECTION_SIZE,
      )?.rect ?? null;

    if (
      nextSelection.width < MIN_SELECTION_SIZE ||
      nextSelection.height < MIN_SELECTION_SIZE
    ) {
      if (candidateForPointerCompletion) {
        completeManualSelection(candidateForPointerCompletion);
        return;
      }

      setSelection(null);
      scheduleSelectionOverlayPaint(null, null);
      return;
    }

    completeManualSelection(nextSelection);
  };

  const startMoveGesture = (event: React.PointerEvent<HTMLDivElement>) => {
    if (status !== 'preview' || !selection || !selectionBounds) return;

    if (isPinCapturePointer(event)) {
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
      const localPoint = clampPointToRect(
        { x: point.x - selection.x, y: point.y - selection.y },
        selection,
      );
      if (annotationGesture?.tool === 'polyline') {
        if (isFinishAnnotationGestureDoubleClick(event)) {
          commitAnnotationGestureAtPoint(localPoint, false);
          return;
        }

        const points = appendAnnotationGesturePoint(
          annotationGesture,
          localPoint,
          event.shiftKey,
        );
        const nextGesture = {
          ...annotationGesture,
          points,
        };
        setAnnotationGesture(nextGesture);
        setDraftAnnotation(
          annotationFromGestureDraft(
            nextGesture,
            localPoint,
            annotationStyle,
            event.shiftKey,
          ),
        );
        return;
      }

      if (activeAnnotationTool === 'text') {
        if (textDraft) return;
        setTextDraft(startTextAnnotationDraft(localPoint, textFontSize));
        setTextDraftAnnotationIndex(null);
        return;
      }

      if (activeAnnotationTool === 'eraser') {
        const nextHistory = eraseAnnotationAtPoint(annotationHistory, localPoint);
        setAnnotationMoveGesture(null);
        setDraftAnnotation(null);
        if (nextHistory !== annotationHistory) {
          setAnnotationHistory(nextHistory);
          void renderSelectionPreview(selection, nextHistory.annotations);
        }
        return;
      }

      const points =
        isPointStrokeAnnotationTool(activeAnnotationTool) ||
        activeAnnotationTool === 'polyline'
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

  const handleWheel = (event: React.WheelEvent<HTMLDivElement>) => {
    if (
      status !== 'preview' ||
      textDraft ||
      annotationGesture ||
      annotationMoveGesture
    ) {
      return;
    }

    const sizeDirection = annotationSizeDirectionFromWheel(event, {
      editing: hasAnnotationEditingContext,
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
      }}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onWheel={handleWheel}
      onContextMenu={(event) => event.preventDefault()}
    >
      {session &&
        selectionBounds &&
        session.monitors.filter((monitor) => monitor.image_base64).map((monitor) => (
          <img
            key={monitor.id}
            src={`data:image/png;base64,${monitor.image_base64}`}
            className="absolute object-fill"
            style={rectStyle(getMonitorViewportRect(monitor, selectionBounds))}
            onLoad={() => markCaptureImageReady(session.id, monitor.id)}
            onError={() => {
              setError('Failed to load capture snapshot');
              setStatus('error');
            }}
            draggable={false}
          />
        ))}

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
                backgroundColor: draftAnnotation.filled
                  ? annotationColorToCss(draftAnnotation.color)
                  : 'transparent',
              }}
            />
          )}
          {draftAnnotation?.type === 'ellipse' && (
            <svg
              className="pointer-events-none absolute overflow-visible"
              style={rectStyle(selectionViewportRect)}
              viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
              fill={
                draftAnnotation.filled
                  ? annotationColorToCss(draftAnnotation.color)
                  : 'none'
              }
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
            className={getCaptureEditorSelectionClassName(
              status,
              Boolean(activeAnnotationTool),
            )}
            style={rectStyle(selectionViewportRect)}
            onPointerDown={startMoveGesture}
          />
          {status === 'preview' && (
            <div
              className="absolute pointer-events-none"
              style={{ ...rectStyle(selectionViewportRect), zIndex: 2 }}
            >
              {SELECTION_HANDLES.map((handle) => (
                <button
                  key={handle}
                  className={`pointer-events-auto absolute h-3 w-3 rounded-full border border-[#5b7fff] bg-white shadow-[0_2px_7px_rgba(91,127,255,0.35)] ${handleClassNames[handle]}`}
                  aria-label={`Resize selection ${handle}`}
                  onPointerDown={(event) => startResizeGesture(handle, event)}
                />
              ))}
            </div>
          )}
          {toolbarPosition && isAnnotationToolbarVisible && (
            <div
              className={getCaptureEditorToolbarClassName()}
              style={{
                left: `${toolbarPosition.x}px`,
                top: `${toolbarPosition.y}px`,
                width: `${TOOLBAR_SIZE.width}px`,
                zIndex: 2,
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
                className={getCaptureEditorIconButtonClassName(!activeAnnotationTool)}
                disabled={isRenderingOutput}
                title="Select and move"
                aria-label="Select and move"
                onClick={() => setActiveAnnotationTool(null)}
              >
                <PointerIcon />
              </button>
              <button
                type="button"
                className={getCaptureEditorIconButtonClassName(
                  activeAnnotationTool === 'rectangle',
                )}
                disabled={isRenderingOutput}
                title="Rectangle"
                aria-label="Draw rectangle annotation"
                onClick={() => toggleAnnotationTool('rectangle')}
              >
                <RectangleIcon />
              </button>
              <button
                type="button"
                className={getCaptureEditorIconButtonClassName(
                  activeAnnotationTool === 'ellipse',
                )}
                disabled={isRenderingOutput}
                title="Ellipse"
                aria-label="Draw ellipse annotation"
                onClick={() => toggleAnnotationTool('ellipse')}
              >
                <EllipseIcon />
              </button>
              <button
                type="button"
                className={getCaptureEditorIconButtonClassName(
                  activeAnnotationTool === 'arrow',
                )}
                disabled={isRenderingOutput}
                title="Arrow"
                aria-label="Draw arrow annotation"
                onClick={() => toggleAnnotationTool('arrow')}
              >
                <ArrowIcon />
              </button>
              <button
                type="button"
                className={getCaptureEditorIconButtonClassName(
                  activeAnnotationTool === 'line',
                )}
                disabled={isRenderingOutput}
                title="Line"
                aria-label="Draw line annotation"
                onClick={() => toggleAnnotationTool('line')}
              >
                <LineIcon />
              </button>
              <button
                type="button"
                className={getCaptureEditorIconButtonClassName(
                  activeAnnotationTool === 'pen',
                )}
                disabled={isRenderingOutput}
                title="Pen"
                aria-label="Draw freehand annotation"
                onClick={() => toggleAnnotationTool('pen')}
              >
                <PenIcon />
              </button>
              <button
                type="button"
                className={getCaptureEditorIconButtonClassName(
                  activeAnnotationTool === 'text',
                )}
                disabled={isRenderingOutput}
                title="Text"
                aria-label="Add text annotation"
                onClick={() => toggleAnnotationTool('text')}
              >
                <TextIcon />
              </button>
              <button
                type="button"
                className={getCaptureEditorIconButtonClassName(
                  activeAnnotationTool === 'mosaic',
                )}
                disabled={isRenderingOutput}
                title="Mosaic"
                aria-label="Draw mosaic annotation"
                onClick={() => toggleAnnotationTool('mosaic')}
              >
                <MosaicIcon />
              </button>
              <button
                type="button"
                className={getCaptureEditorIconButtonClassName(
                  activeAnnotationTool === 'blur',
                )}
                disabled={isRenderingOutput}
                title="Blur"
                aria-label="Draw blur annotation"
                onClick={() => toggleAnnotationTool('blur')}
              >
                <BlurIcon />
              </button>
              <button
                type="button"
                className={getCaptureEditorIconButtonClassName(
                  activeAnnotationTool === 'eraser',
                )}
                disabled={isRenderingOutput}
                title="Eraser"
                aria-label="Erase annotation"
                onClick={() => toggleAnnotationTool('eraser')}
              >
                <EraserIcon />
              </button>
              <div className={getCaptureEditorDividerClassName()} />
              <button
                type="button"
                className="h-9 w-9 shrink-0 rounded-[10px] border border-slate-200 bg-[#5b7fff] shadow-[0_0_0_2px_rgba(91,127,255,0.15)] disabled:cursor-not-allowed disabled:opacity-40"
                style={{ backgroundColor: annotationColorToCss(annotationStyle.color) }}
                disabled={isRenderingOutput}
                title="Annotation color"
                aria-label="Annotation color"
                onClick={() => {
                  const currentIndex = ANNOTATION_COLORS.findIndex((color) =>
                    sameAnnotationColor(annotationStyle.color, color),
                  );
                  const nextColor =
                    ANNOTATION_COLORS[
                      (Math.max(0, currentIndex) + 1) % ANNOTATION_COLORS.length
                    ];
                  applySelectedAnnotationStyle(
                    {
                      ...annotationStyle,
                      color: nextColor,
                    },
                    textFontSize,
                  );
                }}
              />
              <input
                className="h-9 w-20 accent-[#5b7fff] disabled:opacity-40"
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
              <input
                className="h-5 w-5 accent-[#5b7fff] disabled:opacity-40"
                type="checkbox"
                checked={annotationStyle.filled}
                disabled={isRenderingOutput || !isFillModeActive}
                title="Fill shape"
                aria-label="Fill shape"
                onChange={(event) => {
                  applySelectedAnnotationStyle(
                    {
                      ...annotationStyle,
                      filled: event.currentTarget.checked,
                    },
                    textFontSize,
                  );
                }}
              />
              <div className={getCaptureEditorDividerClassName()} />
              <button
                type="button"
                className={getCaptureEditorCommandButtonClassName()}
                disabled={isRenderingOutput}
                title="Cancel"
                aria-label="Cancel capture"
                onClick={cancelSession}
              >
                取消 (Esc)
              </button>
              <button
                type="button"
                className={getCaptureEditorCommandButtonClassName()}
                disabled={isRenderingOutput}
                title="OCR"
                aria-label="Run OCR"
                onClick={runOcrSelection}
              >
                OCR
              </button>
              <button
                type="button"
                className={getCaptureEditorCommandButtonClassName()}
                disabled={isRenderingOutput}
                title="Copy"
                aria-label="Copy selection"
                onClick={copySelection}
              >
                复制 (⌘C)
              </button>
              <button
                type="button"
                className={getCaptureEditorCommandButtonClassName()}
                disabled={isRenderingOutput}
                title="Save"
                aria-label="Save selection"
                onClick={(event) => {
                  const action = getSaveCapturePointerAction(event);
                  if (action === 'quick-save') {
                    void quickSaveSelection();
                  } else {
                    void saveSelection();
                  }
                }}
              >
                保存
              </button>
              <button
                type="button"
                className={getCaptureEditorCommandButtonClassName('primary')}
                disabled={isRenderingOutput}
                title="Finish"
                aria-label="Finish capture"
                onClick={copySelection}
              >
                完成 (Enter)
              </button>
            </div>
          )}
          {isRenderingOutput && (
            <div
              className="absolute h-1 bg-white/80"
              style={{
                left: `${selectionViewportRect.x}px`,
                top: `${selectionViewportRect.y + selectionViewportRect.height}px`,
                width: `${selectionViewportRect.width}px`,
                zIndex: 2,
              }}
            />
          )}
        </>
      )}
      {viewportBounds && (
        <canvas
          ref={selectionOverlayCanvasRef}
          width={Math.max(0, Math.round(viewportBounds.width))}
          height={Math.max(0, Math.round(viewportBounds.height))}
          className={getCaptureSelectionOverlayCanvasClassName()}
          aria-hidden="true"
        />
      )}
      {isMagnifierShown &&
        cursorMonitor &&
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
