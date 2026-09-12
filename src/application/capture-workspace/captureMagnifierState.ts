import {
  shouldAutoShowCaptureMagnifier,
  shouldShowMagnifier,
  shouldTrackCaptureCursorForMagnifier,
} from '../image-inspection/magnifier';
import { getMonitorAtVirtualPoint } from './virtualDesktop';
import { captureMonitorImageSource } from './captureSnapshot';
import type {
  CaptureSessionView,
  LogicalRect,
  MonitorSnapshotView,
  Point,
} from '../../domain/capture';

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
    session?.monitors.some((monitor) => captureMonitorImageSource(monitor)),
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
  const hasMagnifierPixelSource = Boolean(captureMonitorImageSource(cursorMonitor));
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
