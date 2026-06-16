import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  getToolbarPosition,
  moveSelectionByDelta,
  normalizeSelection,
  nudgeSelection,
  resizeSelectionByHandle,
  type ArrowKey,
  type SelectionHandle,
} from './selection';
import {
  getMagnifierImageStyle,
  getMagnifierPosition,
} from './magnifier';
import {
  type ColorSample,
  sampleCanvasColor,
} from './colorSampler';
import {
  buildMonitorCandidates,
  getBestCandidateAtPoint,
} from './captureCandidates';
import { saveCaptureSelection } from './captureActions';
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

const MIN_SELECTION_SIZE = 10;
const KEYBOARD_NUDGE_STEP = 1;
const KEYBOARD_FAST_NUDGE_STEP = 10;
const TOOLBAR_GAP = 8;
const TOOLBAR_SIZE = { width: 272, height: 36 };
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
  const [status, setStatus] = useState<SessionStatus>('idle');
  const [mode, setMode] = useState<CaptureMode>('screenshot');
  const [session, setSession] = useState<CaptureSessionView | null>(null);
  const [startPoint, setStartPoint] = useState<Point | null>(null);
  const [cursorPoint, setCursorPoint] = useState<Point | null>(null);
  const [selection, setSelection] = useState<LogicalRect | null>(null);
  const [hoverSelection, setHoverSelection] = useState<LogicalRect | null>(null);
  const [editGesture, setEditGesture] = useState<EditGesture | null>(null);
  const [previewImageBase64, setPreviewImageBase64] = useState<string | null>(null);
  const [cursorColor, setCursorColor] = useState<ColorSample | null>(null);
  const [sampleCanvasVersion, setSampleCanvasVersion] = useState(0);
  const [isRenderingOutput, setIsRenderingOutput] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasStartedInitialSession, setHasStartedInitialSession] = useState(false);

  const isActive = status !== 'idle';
  const sizeLabel = selection
    ? `${Math.round(selection.width)} x ${Math.round(selection.height)}`
    : '';
  const hoverSizeLabel = hoverSelection
    ? `${Math.round(hoverSelection.width)} x ${Math.round(hoverSelection.height)}`
    : '';
  const captureCandidates = useMemo(() => {
    if (!session) return [];

    return buildMonitorCandidates(session.monitors);
  }, [session]);
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
    setPreviewImageBase64(null);
    setCursorColor(null);
    setSampleCanvasVersion(0);
    setIsRenderingOutput(false);
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
    setPreviewImageBase64(null);
    setCursorColor(null);
    setSampleCanvasVersion(0);
    setIsRenderingOutput(false);
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
    async (rect: LogicalRect) => {
      if (!session) return;

      setIsRenderingOutput(true);
      setPreviewImageBase64(null);
      setError(null);

      try {
        const base64 = await invoke<string>('render_capture_output', {
          sessionId: session.id,
          rect,
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
    [mode, onInactive, resetSessionState, session],
  );

  const copySelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      await invoke('output_capture', {
        sessionId: session.id,
        rect: selection,
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
  }, [onInactive, resetSessionState, selection, session]);

  const saveSelection = useCallback(async () => {
    if (!session || !selection) return;

    setIsRenderingOutput(true);
    setError(null);

    try {
      await saveCaptureSelection(invoke, session.id, selection);
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
      await invoke('output_capture', {
        sessionId: session.id,
        rect: selection,
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
  }, [onInactive, resetSessionState, selection, session]);

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
        void cancelSession();
      } else if (
        status === 'preview' &&
        (event.key === 'Enter' ||
          ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'c'))
      ) {
        event.preventDefault();
        void copySelection();
      } else if (status === 'preview' && selection && selectionBounds && isArrowKey(event.key)) {
        event.preventDefault();
        const step = event.shiftKey ? KEYBOARD_FAST_NUDGE_STEP : KEYBOARD_NUDGE_STEP;
        const nextSelection = nudgeSelection(selection, event.key, selectionBounds, step);
        setSelection(nextSelection);
        setPreviewImageBase64(null);
        void renderSelectionPreview(nextSelection);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    cancelSession,
    copySelection,
    isActive,
    renderSelectionPreview,
    selection,
    selectionBounds,
    status,
  ]);

  const handlePointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if ((status !== 'selecting' && status !== 'preview') || !selectionBounds) return;

    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    setCursorPoint(point);
    event.currentTarget.setPointerCapture(event.pointerId);
    setStartPoint(point);
    setSelection(normalizeSelection(point, point));
    setPreviewImageBase64(null);
    setIsRenderingOutput(false);
    setStatus('selecting');
  };

  const applyEditGesture = useCallback(
    (gesture: EditGesture, point: Point) => {
      if (!selectionBounds) return gesture.startSelection;

      const delta = {
        x: point.x - gesture.startPoint.x,
        y: point.y - gesture.startPoint.y,
      };

      if (gesture.type === 'move') {
        return moveSelectionByDelta(gesture.startSelection, delta, selectionBounds);
      }

      return resizeSelectionByHandle(
        gesture.startSelection,
        gesture.handle,
        delta,
        selectionBounds,
        MIN_SELECTION_SIZE,
      );
    },
    [selectionBounds],
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

    if (editGesture) {
      setSelection(applyEditGesture(editGesture, point));
      setPreviewImageBase64(null);
      setIsRenderingOutput(false);
      return;
    }

    if (!startPoint || status !== 'selecting') return;

    setSelection(normalizeSelection(startPoint, point));
  };

  const handlePointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!selectionBounds) return;

    const point = viewportPointToVirtualPoint(
      { x: event.clientX, y: event.clientY },
      selectionBounds,
    );
    setCursorPoint(point);

    if (editGesture) {
      const nextSelection = applyEditGesture(editGesture, point);
      setEditGesture(null);
      setSelection(nextSelection);
      setStatus('preview');
      void renderSelectionPreview(nextSelection);
      return;
    }

    if (!startPoint || status !== 'selecting') return;

    const nextSelection = normalizeSelection(startPoint, point);
    setStartPoint(null);

    if (
      nextSelection.width < MIN_SELECTION_SIZE ||
      nextSelection.height < MIN_SELECTION_SIZE
    ) {
      if (hoverSelection) {
        setSelection(hoverSelection);
        setHoverSelection(null);
        setStatus('preview');
        void renderSelectionPreview(hoverSelection);
        return;
      }

      setSelection(null);
      return;
    }

    setSelection(nextSelection);
    setHoverSelection(null);
    setStatus('preview');
    void renderSelectionPreview(nextSelection);
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
    setEditGesture({
      type: 'move',
      startPoint: point,
      startSelection: selection,
    });
    setPreviewImageBase64(null);
  };

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
          <div
            className={`absolute border ${overlayClassName} bg-transparent ${
              status === 'preview' ? 'cursor-move' : ''
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
          {toolbarPosition && (
            <div
              className="absolute flex h-9 items-center gap-1 rounded bg-neutral-950/90 p-1 text-xs text-white shadow-lg ring-1 ring-white/15"
              style={{
                left: `${toolbarPosition.x}px`,
                top: `${toolbarPosition.y}px`,
                width: `${TOOLBAR_SIZE.width}px`,
              }}
              onPointerDown={(event) => event.stopPropagation()}
            >
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
