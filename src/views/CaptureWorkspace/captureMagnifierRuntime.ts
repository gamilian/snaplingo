import { useEffect, useRef, useState } from 'react';
import {
  sampleImageColor,
  type ColorSample,
} from '../../application/image-inspection/colorSampler';
import { shouldHydrateCaptureMagnifierPixels } from '../../application/capture-workspace/captureMagnifierState';
import type { CaptureSessionView, MonitorSnapshotView, Point } from './types';

interface UseCaptureMagnifierPixelSourceOptions {
  session: CaptureSessionView | null;
  isMagnifierRequested: boolean;
  isMagnifierShown: boolean;
  cursorMonitor: MonitorSnapshotView | null;
  cursorInMonitorPoint: Point | null;
  setCursorColor: (color: ColorSample | null) => void;
  ensureCaptureMonitorHydrated: (
    sessionId: string,
    monitorId: string,
  ) => Promise<unknown>;
}

export function useCaptureMagnifierPixelSource({
  cursorInMonitorPoint,
  cursorMonitor,
  ensureCaptureMonitorHydrated,
  isMagnifierRequested,
  isMagnifierShown,
  session,
  setCursorColor,
}: UseCaptureMagnifierPixelSourceOptions) {
  const sampleSourceByMonitorRef = useRef<
    Map<string, { image: HTMLImageElement; canvas: HTMLCanvasElement }>
  >(new Map());
  const [sampleSourceVersion, setSampleSourceVersion] = useState(0);

  useEffect(() => {
    sampleSourceByMonitorRef.current = new Map();
    setCursorColor(null);
    setSampleSourceVersion((version) => version + 1);
  }, [session?.id, setCursorColor]);

  useEffect(() => {
    if (
      !session ||
      !cursorMonitor ||
      !shouldHydrateCaptureMagnifierPixels({
        hasSession: true,
        hasCursorMonitorPixelSource: Boolean(cursorMonitor.image_base64),
        isMagnifierRequested,
      })
    ) {
      return;
    }

    void ensureCaptureMonitorHydrated(session.id, cursorMonitor.id).catch((err) => {
      console.warn('Failed to hydrate capture pixels for magnifier:', err);
    });
  }, [
    cursorMonitor,
    ensureCaptureMonitorHydrated,
    isMagnifierRequested,
    session,
  ]);

  useEffect(() => {
    if (!isMagnifierShown || !cursorMonitor?.image_base64) return;
    if (sampleSourceByMonitorRef.current.has(cursorMonitor.id)) return;

    let disposed = false;
    const monitorId = cursorMonitor.id;
    const image = new Image();
    image.decoding = 'async';
    image.onload = () => {
      if (disposed) return;

      const canvas = document.createElement('canvas');
      canvas.width = 1;
      canvas.height = 1;
      sampleSourceByMonitorRef.current.set(monitorId, { image, canvas });
      setSampleSourceVersion((version) => version + 1);
    };
    image.src = `data:image/png;base64,${cursorMonitor.image_base64}`;

    return () => {
      disposed = true;
    };
  }, [cursorMonitor?.id, cursorMonitor?.image_base64, isMagnifierShown]);

  useEffect(() => {
    if (!isMagnifierShown || !cursorInMonitorPoint || !cursorMonitor) {
      setCursorColor(null);
      return;
    }

    const source = sampleSourceByMonitorRef.current.get(cursorMonitor.id);
    if (!source) {
      setCursorColor(null);
      return;
    }

    setCursorColor(
      sampleImageColor(source.image, source.canvas, cursorInMonitorPoint, {
        width: cursorMonitor.logical_bounds.width,
        height: cursorMonitor.logical_bounds.height,
      }),
    );
  }, [
    cursorInMonitorPoint,
    cursorMonitor,
    isMagnifierShown,
    sampleSourceVersion,
    setCursorColor,
  ]);

  return cursorMonitor
    ? sampleSourceByMonitorRef.current.get(cursorMonitor.id)?.image ?? null
    : null;
}
