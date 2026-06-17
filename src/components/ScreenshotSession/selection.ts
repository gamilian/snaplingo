import type { ArrowKey, LogicalRect, Point } from './types';

export type SelectionHandle = 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w' | 'nw';

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

export function constrainSelectionPoint(start: Point, current: Point): Point {
  const deltaX = current.x - start.x;
  const deltaY = current.y - start.y;
  const size = Math.min(Math.abs(deltaX), Math.abs(deltaY));

  return {
    x: start.x + Math.sign(deltaX) * size,
    y: start.y + Math.sign(deltaY) * size,
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

export function restoreSelectionWithinBounds(
  rect: LogicalRect,
  bounds: LogicalRect,
  minSize: number,
): LogicalRect | null {
  if (
    rect.width < minSize ||
    rect.height < minSize ||
    rect.width > bounds.width ||
    rect.height > bounds.height
  ) {
    return null;
  }

  return moveSelectionByDelta(rect, { x: 0, y: 0 }, bounds);
}

export function moveDraftSelectionByDelta(
  rect: LogicalRect,
  anchorPoint: Point,
  delta: Point,
  bounds: LogicalRect,
) {
  const selection = moveSelectionByDelta(rect, delta, bounds);
  const appliedDelta = {
    x: selection.x - rect.x,
    y: selection.y - rect.y,
  };

  return {
    selection,
    anchorPoint: {
      x: anchorPoint.x + appliedDelta.x,
      y: anchorPoint.y + appliedDelta.y,
    },
  };
}

export function nudgeDraftSelection(
  anchorPoint: Point,
  cursorPoint: Point,
  delta: Point,
  bounds: LogicalRect,
): { cursorPoint: Point; selection: LogicalRect } {
  const nextCursorPoint = {
    x: clamp(cursorPoint.x + delta.x, bounds.x, bounds.x + bounds.width - 1),
    y: clamp(cursorPoint.y + delta.y, bounds.y, bounds.y + bounds.height - 1),
  };

  return {
    cursorPoint: nextCursorPoint,
    selection: normalizeSelection(anchorPoint, nextCursorPoint),
  };
}

export function nudgeMovedSelection(
  rect: LogicalRect,
  cursorPoint: Point,
  delta: Point,
  bounds: LogicalRect,
): { cursorPoint: Point; selection: LogicalRect } {
  const selection = moveSelectionByDelta(rect, delta, bounds);
  const appliedDelta = {
    x: selection.x - rect.x,
    y: selection.y - rect.y,
  };

  return {
    cursorPoint: {
      x: cursorPoint.x + appliedDelta.x,
      y: cursorPoint.y + appliedDelta.y,
    },
    selection,
  };
}

export function resizeSelectionByHandle(
  rect: LogicalRect,
  handle: SelectionHandle,
  delta: Point,
  bounds: LogicalRect,
  minSize: number,
  preserveAspect = false,
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

  if (preserveAspect && handle.length === 2) {
    const aspectRatio = rect.width / rect.height;
    const freeWidth = right - left;
    const freeHeight = bottom - top;
    const useWidth =
      Math.abs(freeWidth - rect.width) >=
      Math.abs((freeHeight - rect.height) * aspectRatio);
    let nextWidth = useWidth ? freeWidth : freeHeight * aspectRatio;
    let nextHeight = useWidth ? freeWidth / aspectRatio : freeHeight;
    const maxWidth = handle.includes('w')
      ? originalRight - bounds.x
      : boundsRight - originalLeft;
    const maxHeight = handle.includes('n')
      ? originalBottom - bounds.y
      : boundsBottom - originalTop;

    nextWidth = clamp(nextWidth, minSize, maxWidth);
    nextHeight = nextWidth / aspectRatio;
    if (nextHeight > maxHeight) {
      nextHeight = maxHeight;
      nextWidth = nextHeight * aspectRatio;
    }
    if (nextHeight < minSize) {
      nextHeight = minSize;
      nextWidth = nextHeight * aspectRatio;
    }

    if (handle.includes('w')) {
      left = originalRight - nextWidth;
      right = originalRight;
    } else {
      left = originalLeft;
      right = originalLeft + nextWidth;
    }

    if (handle.includes('n')) {
      top = originalBottom - nextHeight;
      bottom = originalBottom;
    } else {
      top = originalTop;
      bottom = originalTop + nextHeight;
    }
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

export function resizeSelectionBoundaryByArrow(
  rect: LogicalRect,
  direction: ArrowKey,
  mode: 'expand' | 'shrink',
  bounds: LogicalRect,
  minSize: number,
): LogicalRect {
  const step = mode === 'expand' ? 1 : -1;
  const handleByDirection: Record<ArrowKey, SelectionHandle> = {
    ArrowUp: 'n',
    ArrowRight: 'e',
    ArrowDown: 's',
    ArrowLeft: 'w',
  };
  const deltaByDirection: Record<ArrowKey, Point> = {
    ArrowUp: { x: 0, y: -step },
    ArrowRight: { x: step, y: 0 },
    ArrowDown: { x: 0, y: step },
    ArrowLeft: { x: -step, y: 0 },
  };

  return resizeSelectionByHandle(
    rect,
    handleByDirection[direction],
    deltaByDirection[direction],
    bounds,
    minSize,
  );
}

export function snapPointToRects(
  point: Point,
  targets: LogicalRect[],
  threshold: number,
): Point {
  return {
    x: snapValueToEdges(point.x, targetXEdges(targets), threshold),
    y: snapValueToEdges(point.y, targetYEdges(targets), threshold),
  };
}

export function snapMovedSelectionToRects(
  rect: LogicalRect,
  targets: LogicalRect[],
  bounds: LogicalRect,
  threshold: number,
): LogicalRect {
  const xOffset = nearestEdgeOffset(
    [rect.x, rect.x + rect.width],
    targetXEdges(targets),
    threshold,
  );
  const yOffset = nearestEdgeOffset(
    [rect.y, rect.y + rect.height],
    targetYEdges(targets),
    threshold,
  );

  return {
    ...rect,
    x: clamp(rect.x + xOffset, bounds.x, bounds.x + bounds.width - rect.width),
    y: clamp(rect.y + yOffset, bounds.y, bounds.y + bounds.height - rect.height),
  };
}

export function snapResizedSelectionToRects(
  rect: LogicalRect,
  handle: SelectionHandle,
  targets: LogicalRect[],
  bounds: LogicalRect,
  minSize: number,
  threshold: number,
): LogicalRect {
  let left = rect.x;
  let top = rect.y;
  let right = rect.x + rect.width;
  let bottom = rect.y + rect.height;
  const xEdges = targetXEdges(targets);
  const yEdges = targetYEdges(targets);

  if (handle.includes('w')) {
    left = clamp(snapValueToEdges(left, xEdges, threshold), bounds.x, right - minSize);
  }
  if (handle.includes('e')) {
    right = clamp(
      snapValueToEdges(right, xEdges, threshold),
      left + minSize,
      bounds.x + bounds.width,
    );
  }
  if (handle.includes('n')) {
    top = clamp(snapValueToEdges(top, yEdges, threshold), bounds.y, bottom - minSize);
  }
  if (handle.includes('s')) {
    bottom = clamp(
      snapValueToEdges(bottom, yEdges, threshold),
      top + minSize,
      bounds.y + bounds.height,
    );
  }

  return {
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  };
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
  const maxX = Math.max(bounds.x, boundsRight - toolbarSize.width);
  const maxY = Math.max(bounds.y, boundsBottom - toolbarSize.height);

  return {
    x: clamp(preferredX, bounds.x, maxX),
    y: hasRoomBelow ? belowY : clamp(aboveY, bounds.y, maxY),
  };
}

function targetXEdges(targets: LogicalRect[]) {
  return targets.flatMap((target) => [target.x, target.x + target.width]);
}

function targetYEdges(targets: LogicalRect[]) {
  return targets.flatMap((target) => [target.y, target.y + target.height]);
}

function snapValueToEdges(value: number, edges: number[], threshold: number) {
  const offset = nearestEdgeOffset([value], edges, threshold);
  return value + offset;
}

function nearestEdgeOffset(values: number[], edges: number[], threshold: number) {
  let bestOffset = 0;
  let bestDistance = threshold;

  values.forEach((value) => {
    edges.forEach((edge) => {
      const offset = edge - value;
      const distance = Math.abs(offset);
      if (distance <= bestDistance) {
        bestDistance = distance;
        bestOffset = offset;
      }
    });
  });

  return bestOffset;
}
