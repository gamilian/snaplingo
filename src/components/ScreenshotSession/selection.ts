import type { LogicalRect, Point } from './types';

export type SelectionHandle = 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w' | 'nw';

export function normalizeSelection(start: Point, current: Point): LogicalRect {
  return {
    x: Math.min(start.x, current.x),
    y: Math.min(start.y, current.y),
    width: Math.abs(current.x - start.x),
    height: Math.abs(current.y - start.y),
  };
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

export function moveSelectionByDelta(
  rect: LogicalRect,
  delta: Point,
  bounds: LogicalRect,
): LogicalRect {
  return {
    ...rect,
    x: clamp(rect.x + delta.x, bounds.x, bounds.x + bounds.width - rect.width),
    y: clamp(rect.y + delta.y, bounds.y, bounds.y + bounds.height - rect.height),
  };
}

export function resizeSelectionByHandle(
  rect: LogicalRect,
  handle: SelectionHandle,
  delta: Point,
  bounds: LogicalRect,
  minSize: number,
): LogicalRect {
  const originalLeft = rect.x;
  const originalTop = rect.y;
  const originalRight = rect.x + rect.width;
  const originalBottom = rect.y + rect.height;
  const boundsRight = bounds.x + bounds.width;
  const boundsBottom = bounds.y + bounds.height;

  let left = originalLeft;
  let top = originalTop;
  let right = originalRight;
  let bottom = originalBottom;

  if (handle.includes('w')) {
    left = clamp(originalLeft + delta.x, bounds.x, originalRight - minSize);
  }
  if (handle.includes('e')) {
    right = clamp(originalRight + delta.x, originalLeft + minSize, boundsRight);
  }
  if (handle.includes('n')) {
    top = clamp(originalTop + delta.y, bounds.y, originalBottom - minSize);
  }
  if (handle.includes('s')) {
    bottom = clamp(originalBottom + delta.y, originalTop + minSize, boundsBottom);
  }

  return {
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  };
}
