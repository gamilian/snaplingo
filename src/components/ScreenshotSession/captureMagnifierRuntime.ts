import { useEffect, useRef, useState } from 'react';
import { sampleCanvasColor, type ColorSample } from './colorSampler';
import type { CaptureSessionView, MonitorSnapshotView, Point } from './types';

interface CaptureMagnifierHydrationState {
  hasSession: boolean;
  hasHydratedPixelSource: boolean;
  isMagnifierRequested: boolean;
}

interface UseCaptureMagnifierPixelSourceOptions {
  session: CaptureSessionView | null;
  hasHydratedPixelSource: boolean;
  isMagnifierRequested: boolean;
  isMagnifierShown: boolean;
  cursorMonitor: MonitorSnapshotView | null;
  cursorInMonitorPoint: Point | null;
  setCursorColor: (color: ColorSample | null) => void;
  ensureCaptureSnapshotsHydrated: (sessionId: string) => Promise<unknown>;
}

export function shouldHydrateCaptureMagnifierPixels({
  hasHydratedPixelSource,
  hasSession,
  isMagnifierRequested,
}: CaptureMagnifierHydrationState) {
  return hasSession && !hasHydratedPixelSource && isMagnifierRequested;
}

export function useCaptureMagnifierPixelSource({
  cursorInMonitorPoint,
  cursorMonitor,
  ensureCaptureSnapshotsHydrated,
  hasHydratedPixelSource,
  isMagnifierRequested,
  isMagnifierShown,
  session,
  setCursorColor,
}: UseCaptureMagnifierPixelSourceOptions) {
  const sampleCanvasByMonitorRef = useRef<Map<string, HTMLCanvasElement>>(new Map());
  const [sampleCanvasVersion, setSampleCanvasVersion] = useState(0);

  useEffect(() => {
    sampleCanvasByMonitorRef.current = new Map();
    setCursorColor(null);
    setSampleCanvasVersion((version) => version + 1);
  }, [session?.id, setCursorColor]);

  useEffect(() => {
    if (
      !shouldHydrateCaptureMagnifierPixels({
        hasSession: Boolean(session),
        hasHydratedPixelSource,
        isMagnifierRequested,
      }) ||
      !session
    ) {
      return;
    }

    void ensureCaptureSnapshotsHydrated(session.id).catch((err) => {
      console.warn('Failed to hydrate capture pixels for magnifier:', err);
    });
  }, [
    ensureCaptureSnapshotsHydrated,
    hasHydratedPixelSource,
    isMagnifierRequested,
    session,
  ]);

  useEffect(() => {
    if (!session || !isMagnifierShown) return;

    let disposed = false;
    session.monitors.forEach((monitor) => {
      if (!monitor.image_base64) return;
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
  }, [isMagnifierShown, session]);

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
  }, [cursorInMonitorPoint, cursorMonitor, sampleCanvasVersion, setCursorColor]);
}
