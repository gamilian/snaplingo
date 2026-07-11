const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;
const ZOOM_STEP = 0.1;
const MIN_OPACITY = 0.2;
const MAX_OPACITY = 1;
const OPACITY_STEP = 0.05;
const MAX_INITIAL_WIDTH = 900;
const MAX_INITIAL_HEIGHT = 700;
const MAX_THUMBNAIL_WIDTH = 220;
const MAX_THUMBNAIL_HEIGHT = 160;
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

export interface PinnedFrameRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface PinnedTransform {
  rotation: number;
  flipX: boolean;
  flipY: boolean;
}

export interface PinnedVisualFilter {
  grayscale: boolean;
  inverted: boolean;
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
export type PinnedKeyboardToolbarAction = 'toggle' | 'hide';
export type PinnedWheelAction = 'zoom' | 'opacity';
export type PinnedKeyboardTransformAction =
  | 'rotate-clockwise'
  | 'rotate-counterclockwise'
  | 'flip-horizontal'
  | 'flip-vertical';
export type PinnedKeyboardVisualFilterAction =
  | 'toggle-grayscale'
  | 'toggle-invert';

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

export function getPinnedKeyboardVisualFilterAction(
  event: PinnedKeyboardEvent,
): PinnedKeyboardVisualFilterAction | null {
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return null;
  if (event.key === '5') return 'toggle-grayscale';
  if (event.key === '6') return 'toggle-invert';

  return null;
}

export function getPinnedKeyboardMoveDelta(
  event: PinnedKeyboardEvent,
): PinnedPoint | null {
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return null;

  const deltaByKey: Record<string, PinnedPoint> = {
    ArrowUp: { x: 0, y: -1 },
    ArrowRight: { x: 1, y: 0 },
    ArrowDown: { x: 0, y: 1 },
    ArrowLeft: { x: -1, y: 0 },
  };

  return deltaByKey[event.key] ?? null;
}

export function isPinnedMagnifierShortcut(event: PinnedKeyboardEvent) {
  return (
    event.key === 'Alt' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.shiftKey
  );
}

export function getPinnedImagePointFromPointer(
  point: PinnedPoint,
  frame: PinnedFrameRect,
): PinnedPoint {
  return {
    x: Math.floor(clamp(point.x - frame.left, 0, frame.width - 1)),
    y: Math.floor(clamp(point.y - frame.top, 0, frame.height - 1)),
  };
}

export function getPinnedKeyboardToolbarAction(
  event: PinnedKeyboardEvent,
  toolbarVisible: boolean,
): PinnedKeyboardToolbarAction | null {
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return null;
  if (event.key === ' ') return 'toggle';
  if (event.key === 'Escape' && toolbarVisible) return 'hide';

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

export function nextPinnedVisualFilter(
  filter: PinnedVisualFilter,
  action: PinnedKeyboardVisualFilterAction,
): PinnedVisualFilter {
  if (action === 'toggle-grayscale') {
    return { ...filter, grayscale: !filter.grayscale };
  }

  return { ...filter, inverted: !filter.inverted };
}

export function getPinnedVisualFilterStyle(filter: PinnedVisualFilter) {
  const filters = [
    filter.grayscale ? 'grayscale(1)' : null,
    filter.inverted ? 'invert(1)' : null,
  ].filter(Boolean);

  return filters.length > 0 ? filters.join(' ') : 'none';
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
  return (
    event.button === 1 &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
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

export function isTogglePinnedThumbnailModeDoubleClick(
  event: PinnedPointerEvent,
) {
  return (
    (event.detail ?? 0) >= 2 &&
    event.button === 0 &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !!event.shiftKey
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

export function getPinnedThumbnailDisplaySize(originalSize: PinnedSize) {
  const originalWidth = Math.max(1, originalSize.width);
  const originalHeight = Math.max(1, originalSize.height);
  const thumbnailScale = Math.min(
    MAX_THUMBNAIL_WIDTH / originalWidth,
    MAX_THUMBNAIL_HEIGHT / originalHeight,
    1,
  );

  return {
    width: Math.max(MIN_WIDTH, Math.round(originalWidth * thumbnailScale)),
    height: Math.max(MIN_HEIGHT, Math.round(originalHeight * thumbnailScale)),
  };
}

export function getPinnedDisplaySizeForTransform(
  originalSize: PinnedSize,
  zoom: number,
  transform: PinnedTransform,
  thumbnailMode = false,
) {
  const size = thumbnailMode
    ? getPinnedThumbnailDisplaySize(originalSize)
    : getPinnedDisplaySize(originalSize, zoom);
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
