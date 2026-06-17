const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;
const ZOOM_STEP = 0.1;
const MIN_OPACITY = 0.2;
const MAX_OPACITY = 1;
const OPACITY_STEP = 0.05;
const MAX_INITIAL_WIDTH = 900;
const MAX_INITIAL_HEIGHT = 700;
const MIN_WIDTH = 80;
const MIN_HEIGHT = 60;

export interface PinnedSize {
  width: number;
  height: number;
}

export interface PinnedPoint {
  x: number;
  y: number;
}

interface PinnedKeyboardEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
}

export type PinnedKeyboardZoomAction = 'zoom-in' | 'zoom-out' | 'reset';

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function roundToTwoDecimals(value: number) {
  return Math.round(value * 100) / 100;
}

export function getPinnedZoomFromWheel(currentZoom: number, wheelDirection: number) {
  const nextZoom =
    currentZoom + (wheelDirection < 0 ? ZOOM_STEP : -ZOOM_STEP);

  return roundToTwoDecimals(clamp(nextZoom, MIN_ZOOM, MAX_ZOOM));
}

export function getPinnedKeyboardZoomAction(
  event: PinnedKeyboardEvent,
): PinnedKeyboardZoomAction | null {
  if (event.metaKey || event.ctrlKey) return null;
  if (event.key === '+' || event.key === '=') return 'zoom-in';
  if (event.key === '-') return 'zoom-out';
  if (event.key === '0') return 'reset';

  return null;
}

export function getPinnedOpacityFromWheel(
  currentOpacity: number,
  wheelDirection: number,
) {
  const nextOpacity =
    currentOpacity + (wheelDirection < 0 ? OPACITY_STEP : -OPACITY_STEP);

  return roundToTwoDecimals(clamp(nextOpacity, MIN_OPACITY, MAX_OPACITY));
}

export function getPinnedOpacityPreset(opacity: number) {
  return roundToTwoDecimals(clamp(opacity, MIN_OPACITY, MAX_OPACITY));
}

export function getPinnedDisplaySize(originalSize: PinnedSize, zoom: number) {
  const originalWidth = Math.max(1, originalSize.width);
  const originalHeight = Math.max(1, originalSize.height);
  const initialScale = Math.min(
    MAX_INITIAL_WIDTH / originalWidth,
    MAX_INITIAL_HEIGHT / originalHeight,
    1,
  );

  return {
    width: Math.max(MIN_WIDTH, Math.round(originalWidth * initialScale * zoom)),
    height: Math.max(MIN_HEIGHT, Math.round(originalHeight * initialScale * zoom)),
  };
}

export function getPinnedContextMenuPosition(
  point: PinnedPoint,
  menuSize: PinnedSize,
  viewportSize: PinnedSize,
): PinnedPoint {
  return {
    x: clamp(point.x, 0, Math.max(0, viewportSize.width - menuSize.width)),
    y: clamp(point.y, 0, Math.max(0, viewportSize.height - menuSize.height)),
  };
}
