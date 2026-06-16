import { useCallback, useEffect, useMemo, useState } from 'react';
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
import { parseCaptureLaunchPayload } from './windowMode';
import type {
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
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
const TOOLBAR_SIZE = { width: 168, height: 36 };
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
  const [status, setStatus] = useState<SessionStatus>('idle');
  const [mode, setMode] = useState<CaptureMode>('screenshot');
  const [session, setSession] = useState<CaptureSessionView | null>(null);
  const [startPoint, setStartPoint] = useState<Point | null>(null);
  const [selection, setSelection] = useState<LogicalRect | null>(null);
  const [editGesture, setEditGesture] = useState<EditGesture | null>(null);
  const [previewImageBase64, setPreviewImageBase64] = useState<string | null>(null);
  const [isRenderingOutput, setIsRenderingOutput] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasStartedInitialSession, setHasStartedInitialSession] = useState(false);

  const monitor = session?.monitors[0] ?? null;
  const isActive = status !== 'idle';
  const sizeLabel = selection
    ? `${Math.round(selection.width)} x ${Math.round(selection.height)}`
    : '';
  const selectionBounds = useMemo<LogicalRect | null>(() => {
    if (!monitor) return null;

    return {
      x: 0,
      y: 0,
      width: monitor.logical_bounds.width,
      height: monitor.logical_bounds.height,
    };
  }, [monitor]);
  const toolbarPosition = useMemo(() => {
    if (!selection || !selectionBounds || status !== 'preview') return null;

    return getToolbarPosition(selection, selectionBounds, TOOLBAR_SIZE, TOOLBAR_GAP);
  }, [selection, selectionBounds, status]);

  const resetSessionState = useCallback(() => {
    setStatus('idle');
    setSession(null);
    setStartPoint(null);
    setSelection(null);
    setEditGesture(null);
    setPreviewImageBase64(null);
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
    setSelection(null);
    setEditGesture(null);
    setPreviewImageBase64(null);
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
    if (status !== 'selecting' && status !== 'preview') return;

    const point = { x: event.clientX, y: event.clientY };
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
    if (editGesture) {
      setSelection(applyEditGesture(editGesture, {
        x: event.clientX,
        y: event.clientY,
      }));
      setPreviewImageBase64(null);
      setIsRenderingOutput(false);
      return;
    }

    if (!startPoint || status !== 'selecting') return;

    setSelection(normalizeSelection(startPoint, {
      x: event.clientX,
      y: event.clientY,
    }));
  };

  const handlePointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    if (editGesture) {
      const nextSelection = applyEditGesture(editGesture, {
        x: event.clientX,
        y: event.clientY,
      });
      setEditGesture(null);
      setSelection(nextSelection);
      setStatus('preview');
      void renderSelectionPreview(nextSelection);
      return;
    }

    if (!startPoint || status !== 'selecting') return;

    const nextSelection = normalizeSelection(startPoint, {
      x: event.clientX,
      y: event.clientY,
    });
    setStartPoint(null);

    if (
      nextSelection.width < MIN_SELECTION_SIZE ||
      nextSelection.height < MIN_SELECTION_SIZE
    ) {
      setSelection(null);
      return;
    }

    setSelection(nextSelection);
    setStatus('preview');
    void renderSelectionPreview(nextSelection);
  };

  const startMoveGesture = (event: React.PointerEvent<HTMLDivElement>) => {
    if (status !== 'preview' || !selection) return;

    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    setEditGesture({
      type: 'move',
      startPoint: { x: event.clientX, y: event.clientY },
      startSelection: selection,
    });
    setPreviewImageBase64(null);
  };

  const startResizeGesture = (
    handle: SelectionHandle,
    event: React.PointerEvent<HTMLButtonElement>,
  ) => {
    if (status !== 'preview' || !selection) return;

    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    setEditGesture({
      type: 'resize',
      handle,
      startPoint: { x: event.clientX, y: event.clientY },
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
      className="fixed inset-0 z-[9999] cursor-crosshair select-none overflow-hidden bg-black text-white"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
    >
      {monitor && (
        <img
          src={`data:image/png;base64,${monitor.image_base64}`}
          className="absolute inset-0 h-full w-full object-fill"
          draggable={false}
        />
      )}

      {status === 'loading' && (
        <div className="absolute inset-0 bg-black" aria-label="Loading capture" />
      )}

      {status === 'error' && (
        <div className="absolute left-4 top-4 max-w-md rounded bg-red-950/90 px-3 py-2 text-sm text-red-100 shadow-lg">
          {error}
        </div>
      )}

      {selection && (
        <>
          <DimMask rect={selection} />
          {previewImageBase64 && status === 'preview' && (
            <img
              src={`data:image/png;base64,${previewImageBase64}`}
              className="absolute object-fill"
              style={rectStyle(selection)}
              draggable={false}
            />
          )}
          <div
            className={`absolute border ${overlayClassName} bg-transparent ${
              status === 'preview' ? 'cursor-move' : ''
            }`}
            style={rectStyle(selection)}
            onPointerDown={startMoveGesture}
          />
          {status === 'preview' && (
            <div className="absolute pointer-events-none" style={rectStyle(selection)}>
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
              left: `${selection.x}px`,
              top: `${Math.max(0, selection.y - 24)}px`,
            }}
          >
            {sizeLabel}
          </div>
          {isRenderingOutput && (
            <div
              className="absolute h-1 bg-white/80"
              style={{
                left: `${selection.x}px`,
                top: `${selection.y + selection.height}px`,
                width: `${selection.width}px`,
              }}
            />
          )}
        </>
      )}
    </div>
  );
}
