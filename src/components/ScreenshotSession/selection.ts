import type { LogicalRect, Point } from './types';

export type SelectionHandle = 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w' | 'nw';
export type ArrowKey = 'ArrowUp' | 'ArrowRight' | 'ArrowDown' | 'ArrowLeft';

interface Size {
  width: number;
  height: number;
}

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

export function nudgeSelection(
  rect: LogicalRect,
  direction: ArrowKey,
  bounds: LogicalRect,
  step: number,
): LogicalRect {
  const deltaByDirection: Record<ArrowKey, Point> = {
    ArrowUp: { x: 0, y: -step },
    ArrowRight: { x: step, y: 0 },
    ArrowDown: { x: 0, y: step },
    ArrowLeft: { x: -step, y: 0 },
  };

  return moveSelectionByDelta(rect, deltaByDirection[direction], bounds);
}

export function getToolbarPosition(
  rect: LogicalRect,
  bounds: LogicalRect,
  toolbarSize: Size,
  gap: number,
): Point {
  const boundsRight = bounds.x + bounds.width;
  const boundsBottom = bounds.y + bounds.height;
  const preferredX = rect.x;
  const belowY = rect.y + rect.height + gap;
  const aboveY = rect.y - toolbarSize.height - gap;
  const hasRoomBelow = belowY + toolbarSize.height <= boundsBottom;

  return {
    x: clamp(preferredX, bounds.x, boundsRight - toolbarSize.width),
    y: hasRoomBelow ? belowY : clamp(aboveY, bounds.y, boundsBottom - toolbarSize.height),
  };
}
