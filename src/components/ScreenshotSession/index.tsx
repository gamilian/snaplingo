import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { normalizeSelection } from './selection';
import { parseCaptureLaunchPayload } from './windowMode';
import type {
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  OcrResult,
  Point,
} from './types';

type SessionStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';

const MIN_SELECTION_SIZE = 10;

function rectStyle(rect: LogicalRect) {
  return {
    left: `${rect.x}px`,
    top: `${rect.y}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
  };
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
  const [previewImageBase64, setPreviewImageBase64] = useState<string | null>(null);
  const [isRenderingOutput, setIsRenderingOutput] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasStartedInitialSession, setHasStartedInitialSession] = useState(false);

  const monitor = session?.monitors[0] ?? null;
  const isActive = status !== 'idle';
  const sizeLabel = selection
    ? `${Math.round(selection.width)} x ${Math.round(selection.height)}`
    : '';

  const resetSessionState = useCallback(() => {
    setStatus('idle');
    setSession(null);
    setStartPoint(null);
    setSelection(null);
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
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [cancelSession, isActive, status]);

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

  const handlePointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!startPoint || status !== 'selecting') return;

    setSelection(normalizeSelection(startPoint, {
      x: event.clientX,
      y: event.clientY,
    }));
  };

  const handlePointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
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
            className={`absolute border ${overlayClassName} bg-transparent`}
            style={rectStyle(selection)}
          />
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
