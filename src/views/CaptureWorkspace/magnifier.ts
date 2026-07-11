import type { LogicalRect, Point } from './types';

interface Size {
  width: number;
  height: number;
}

interface MagnifierVisibilityState {
  requested: boolean;
  automatic: boolean;
  hasCursorMonitor: boolean;
  hasViewportCursor: boolean;
  hasImageCursor: boolean;
  hasViewportBounds: boolean;
}

interface AutoMagnifierState {
  status: 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
  hasHydratedPixels: boolean;
}

interface CursorTrackingState extends AutoMagnifierState {
  requested: boolean;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

export function shouldShowMagnifier(state: MagnifierVisibilityState) {
  return (
    (state.requested || state.automatic) &&
    state.hasCursorMonitor &&
    state.hasViewportCursor &&
    state.hasImageCursor &&
    state.hasViewportBounds
  );
}

export function shouldAutoShowCaptureMagnifier(state: AutoMagnifierState) {
  return state.status === 'selecting' && state.hasHydratedPixels;
}

export function shouldTrackCaptureCursorForMagnifier(state: CursorTrackingState) {
  return (
    state.requested ||
    state.status === 'preview' ||
    shouldAutoShowCaptureMagnifier(state)
  );
}

export function getMagnifierPosition(
  cursor: Point,
  bounds: LogicalRect,
  size: Size,
  gap: number,
): Point {
  const boundsRight = bounds.x + bounds.width;
  const boundsBottom = bounds.y + bounds.height;
  const rightX = cursor.x + gap;
  const leftX = cursor.x - size.width - gap;
  const belowY = cursor.y + gap;
  const aboveY = cursor.y - size.height - gap;

  return {
    x: clamp(rightX + size.width <= boundsRight ? rightX : leftX, bounds.x, boundsRight - size.width),
    y: clamp(belowY + size.height <= boundsBottom ? belowY : aboveY, bounds.y, boundsBottom - size.height),
  };
}

export function getMagnifierImageStyle(
  imageBase64: string,
  cursor: Point,
  imageSize: Size,
  lensSize: Size,
  zoom: number,
) {
  const backgroundWidth = Math.round(imageSize.width * zoom);
  const backgroundHeight = Math.round(imageSize.height * zoom);
  const backgroundX = Math.round(lensSize.width / 2 - cursor.x * zoom);
  const backgroundY = Math.round(lensSize.height / 2 - cursor.y * zoom);

  return {
    backgroundImage: `url(data:image/png;base64,${imageBase64})`,
    backgroundSize: `${backgroundWidth}px ${backgroundHeight}px`,
    backgroundPosition: `${backgroundX}px ${backgroundY}px`,
    imageRendering: 'pixelated' as const,
  };
}
