import { useEffect, useRef, useState, type RefObject } from 'react';
import {
  sampleImageColor,
  type ColorSample,
} from '../../application/image-inspection/colorSampler';
import { shouldHydrateCaptureMagnifierPixels } from '../../application/capture-workspace/captureMagnifierState';
import { captureMonitorImageSource } from '../../application/capture-workspace/captureSnapshot';
import type { CaptureSessionView, MonitorSnapshotView, Point } from './types';

interface UseCaptureMagnifierPixelSourceOptions {
  desktopRef: RefObject<HTMLDivElement>;
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
  desktopRef,
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
        hasCursorMonitorPixelSource: Boolean(captureMonitorImageSource(cursorMonitor)),
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
    const source = captureMonitorImageSource(cursorMonitor);
    if (!isMagnifierShown || !cursorMonitor || !source) return;
    if (sampleSourceByMonitorRef.current.has(cursorMonitor.id)) return;

    let disposed = false;
    const monitorId = cursorMonitor.id;
    const image = Array.from(
      desktopRef.current?.querySelectorAll<HTMLImageElement>('[data-capture-monitor]') ?? [],
    ).find((image) => image.dataset.captureMonitor === monitorId);
    if (!image) return;
    void image.decode().then(() => {
      if (disposed) return;

      const canvas = document.createElement('canvas');
      canvas.width = 1;
      canvas.height = 1;
      sampleSourceByMonitorRef.current.set(monitorId, { image, canvas });
      setSampleSourceVersion((version) => version + 1);
    }).catch(() => undefined);

    return () => {
      disposed = true;
    };
  }, [desktopRef, cursorMonitor?.id, cursorMonitor?.image_base64, cursorMonitor?.image_url, isMagnifierShown]);

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
