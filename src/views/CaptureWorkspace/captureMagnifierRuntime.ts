import { useEffect, useRef, useState } from 'react';
import { sampleImageColor, type ColorSample } from './colorSampler';
import {
  shouldAutoShowCaptureMagnifier,
  shouldShowMagnifier,
  shouldTrackCaptureCursorForMagnifier,
} from './magnifier';
import { getMonitorAtVirtualPoint } from './virtualDesktop';
import type {
  CaptureSessionView,
  LogicalRect,
  MonitorSnapshotView,
  Point,
} from './types';

interface CaptureMagnifierHydrationState {
  hasSession: boolean;
  hasCursorMonitorPixelSource: boolean;
  isMagnifierRequested: boolean;
}

interface CaptureMagnifierRequestState {
  enabled: boolean;
  requested: boolean;
  status: 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
}

interface CaptureMagnifierRuntimeStateOptions {
  session: CaptureSessionView | null;
  status: 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
  cursorPoint: Point | null;
  cursorViewportPoint: Point | null;
  viewportBounds: LogicalRect | null;
  isMagnifierRequested: boolean;
}

export interface CaptureMagnifierRuntimeState {
  hasHydratedPixelSource: boolean;
  cursorMonitor: MonitorSnapshotView | null;
  cursorInMonitorPoint: Point | null;
  shouldTrackMagnifierCursor: boolean;
  isMagnifierShown: boolean;
}

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

export function shouldHydrateCaptureMagnifierPixels({
  hasCursorMonitorPixelSource,
  hasSession,
  isMagnifierRequested,
}: CaptureMagnifierHydrationState) {
  return hasSession && !hasCursorMonitorPixelSource && isMagnifierRequested;
}

export function shouldRequestCaptureMagnifierPixels({
  enabled,
  requested,
  status,
}: CaptureMagnifierRequestState) {
  return requested || (enabled && status === 'selecting');
}

export function getCaptureMagnifierRuntimeState({
  cursorPoint,
  cursorViewportPoint,
  isMagnifierRequested,
  session,
  status,
  viewportBounds,
}: CaptureMagnifierRuntimeStateOptions): CaptureMagnifierRuntimeState {
  const hasHydratedPixelSource = Boolean(
    session?.monitors.some((monitor) => monitor.image_base64),
  );
  const cursorMonitor =
    session && cursorPoint
      ? getMonitorAtVirtualPoint(session.monitors, cursorPoint)
      : null;
  const cursorInMonitorPoint =
    cursorPoint && cursorMonitor
      ? {
          x: cursorPoint.x - cursorMonitor.logical_bounds.x,
          y: cursorPoint.y - cursorMonitor.logical_bounds.y,
        }
      : null;
  const hasMagnifierPixelSource = Boolean(cursorMonitor?.image_base64);
  const isMagnifierAutoRequested = shouldAutoShowCaptureMagnifier({
    status,
    hasHydratedPixels: hasMagnifierPixelSource,
  });

  return {
    hasHydratedPixelSource,
    cursorMonitor,
    cursorInMonitorPoint,
    shouldTrackMagnifierCursor: shouldTrackCaptureCursorForMagnifier({
      status,
      requested: isMagnifierRequested,
      hasHydratedPixels: hasHydratedPixelSource,
    }),
    isMagnifierShown: shouldShowMagnifier({
      requested: isMagnifierRequested,
      automatic: isMagnifierAutoRequested,
      hasCursorMonitor: hasMagnifierPixelSource,
      hasViewportCursor: Boolean(cursorViewportPoint),
      hasImageCursor: Boolean(cursorInMonitorPoint),
      hasViewportBounds: Boolean(viewportBounds),
    }),
  };
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
