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

export interface PinnedTransform {
  rotation: number;
  flipX: boolean;
  flipY: boolean;
}

interface PinnedKeyboardEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
}

interface PinnedWheelEvent {
  metaKey: boolean;
  ctrlKey: boolean;
  altKey?: boolean;
}

interface PinnedPointerEvent {
  detail?: number;
  button: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
}

export type PinnedKeyboardZoomAction = 'zoom-in' | 'zoom-out' | 'reset';
export type PinnedKeyboardOpacityAction = 'decrease' | 'increase';
export type PinnedWheelAction = 'zoom' | 'opacity';
export type PinnedKeyboardTransformAction =
  | 'rotate-clockwise'
  | 'rotate-counterclockwise'
  | 'flip-horizontal'
  | 'flip-vertical';

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

export function getPinnedKeyboardOpacityAction(
  event: PinnedKeyboardEvent,
): PinnedKeyboardOpacityAction | null {
  if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) {
    return null;
  }
  if (event.key === '+' || event.key === '=') return 'increase';
  if (event.key === '-') return 'decrease';

  return null;
}

export function getPinnedKeyboardTransformAction(
  event: PinnedKeyboardEvent,
): PinnedKeyboardTransformAction | null {
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return null;
  if (event.key === '1') return 'rotate-clockwise';
  if (event.key === '2') return 'rotate-counterclockwise';
  if (event.key === '3') return 'flip-horizontal';
  if (event.key === '4') return 'flip-vertical';

  return null;
}

export function nextPinnedTransform(
  transform: PinnedTransform,
  action: PinnedKeyboardTransformAction,
): PinnedTransform {
  if (action === 'rotate-clockwise') {
    return { ...transform, rotation: (transform.rotation + 90) % 360 };
  }

  if (action === 'rotate-counterclockwise') {
    return { ...transform, rotation: (transform.rotation + 270) % 360 };
  }

  if (action === 'flip-horizontal') {
    return { ...transform, flipX: !transform.flipX };
  }

  return { ...transform, flipY: !transform.flipY };
}

export function getPinnedTransformStyle(transform: PinnedTransform) {
  const scaleX = transform.flipX ? -1 : 1;
  const scaleY = transform.flipY ? -1 : 1;

  return `rotate(${transform.rotation}deg) scale(${scaleX}, ${scaleY})`;
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

export function getPinnedWheelAction(
  event: PinnedWheelEvent,
): PinnedWheelAction | null {
  if (event.altKey) return null;
  if (event.metaKey || event.ctrlKey) return 'opacity';

  return 'zoom';
}

export function isResetPinnedImagePointer(event: PinnedPointerEvent) {
  const isUnmodifiedMiddleClick =
    event.button === 1 &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey;
  const isShiftLeftDoubleClick =
    (event.detail ?? 0) >= 2 &&
    event.button === 0 &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !!event.shiftKey;

  return isUnmodifiedMiddleClick || isShiftLeftDoubleClick;
}

export function isClosePinnedImageDoubleClick(event: PinnedPointerEvent) {
  return (
    (event.detail ?? 0) >= 2 &&
    event.button === 0 &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
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

export function getPinnedDisplaySizeForTransform(
  originalSize: PinnedSize,
  zoom: number,
  transform: PinnedTransform,
) {
  const size = getPinnedDisplaySize(originalSize, zoom);
  const normalizedRotation = ((transform.rotation % 360) + 360) % 360;

  if (normalizedRotation === 90 || normalizedRotation === 270) {
    return {
      width: size.height,
      height: size.width,
    };
  }

  return size;
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
