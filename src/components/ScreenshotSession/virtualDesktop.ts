import type { LogicalRect, MonitorSnapshotView, Point } from './types';

export function getVirtualDesktopBounds(
  monitors: MonitorSnapshotView[],
): LogicalRect {
  if (monitors.length === 0) {
    return { x: 0, y: 0, width: 0, height: 0 };
  }

  const left = Math.min(...monitors.map((monitor) => monitor.logical_bounds.x));
  const top = Math.min(...monitors.map((monitor) => monitor.logical_bounds.y));
  const right = Math.max(
    ...monitors.map(
      (monitor) => monitor.logical_bounds.x + monitor.logical_bounds.width,
    ),
  );
  const bottom = Math.max(
    ...monitors.map(
      (monitor) => monitor.logical_bounds.y + monitor.logical_bounds.height,
    ),
  );

  return {
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  };
}

export function virtualPointToViewportPoint(
  point: Point,
  bounds: LogicalRect,
): Point {
  return {
    x: point.x - bounds.x,
    y: point.y - bounds.y,
  };
}

export function viewportPointToVirtualPoint(
  point: Point,
  bounds: LogicalRect,
): Point {
  return {
    x: point.x + bounds.x,
    y: point.y + bounds.y,
  };
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

export function nudgeVirtualPoint(
  point: Point,
  delta: Point,
  bounds: LogicalRect,
): Point {
  return {
    x: clamp(point.x + delta.x, bounds.x, bounds.x + bounds.width - 1),
    y: clamp(point.y + delta.y, bounds.y, bounds.y + bounds.height - 1),
  };
}

export function virtualRectToViewportRect(
  rect: LogicalRect,
  bounds: LogicalRect,
): LogicalRect {
  const point = virtualPointToViewportPoint(rect, bounds);

  return {
    ...point,
    width: rect.width,
    height: rect.height,
  };
}

export function getMonitorViewportRect(
  monitor: MonitorSnapshotView,
  bounds: LogicalRect,
): LogicalRect {
  return virtualRectToViewportRect(monitor.logical_bounds, bounds);
}

export function getMonitorAtVirtualPoint(
  monitors: MonitorSnapshotView[],
  point: Point,
): MonitorSnapshotView | null {
  return (
    monitors.find((monitor) => {
      const bounds = monitor.logical_bounds;
      return (
        point.x >= bounds.x &&
        point.x < bounds.x + bounds.width &&
        point.y >= bounds.y &&
        point.y < bounds.y + bounds.height
      );
    }) ?? null
  );
}

export function getCurrentMonitorBounds(
  monitors: MonitorSnapshotView[],
  point: Point | null,
): LogicalRect {
  const monitor = point ? getMonitorAtVirtualPoint(monitors, point) : null;

  return monitor?.logical_bounds ?? getVirtualDesktopBounds(monitors);
}
