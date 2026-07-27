interface Point {
  x: number;
  y: number;
}

interface LogicalRect extends Point {
  width: number;
  height: number;
}

interface Size {
  width: number;
  height: number;
}

interface Rect extends Point, Size {}

export interface MagnifierCanvasBlit {
  source: Rect;
  destination: Rect;
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

export function normalizeMagnifierZoom(zoom: number) {
  return clamp(Math.round(zoom), 4, 20);
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

export function getMagnifierCanvasBlit(
  cursor: Point,
  logicalSize: Size,
  pixelSize: Size,
  lensSize: Size,
  zoom: number,
): MagnifierCanvasBlit | null {
  if (
    logicalSize.width <= 0 ||
    logicalSize.height <= 0 ||
    pixelSize.width <= 0 ||
    pixelSize.height <= 0 ||
    lensSize.width <= 0 ||
    lensSize.height <= 0 ||
    zoom <= 0
  ) {
    return null;
  }

  const centerPixelX = clamp(
    Math.floor((cursor.x / logicalSize.width) * pixelSize.width),
    0,
    pixelSize.width - 1,
  );
  const centerPixelY = clamp(
    Math.floor((cursor.y / logicalSize.height) * pixelSize.height),
    0,
    pixelSize.height - 1,
  );
  const sampleWidth = Math.min(
    pixelSize.width,
    Math.max(1, Math.round(lensSize.width / zoom)),
  );
  const sampleHeight = Math.min(
    pixelSize.height,
    Math.max(1, Math.round(lensSize.height / zoom)),
  );
  const desiredX = centerPixelX - Math.floor(sampleWidth / 2);
  const desiredY = centerPixelY - Math.floor(sampleHeight / 2);
  const sourceX = clamp(desiredX, 0, pixelSize.width);
  const sourceY = clamp(desiredY, 0, pixelSize.height);
  const sourceRight = clamp(
    desiredX + sampleWidth,
    0,
    pixelSize.width,
  );
  const sourceBottom = clamp(
    desiredY + sampleHeight,
    0,
    pixelSize.height,
  );
  const sourceWidth = sourceRight - sourceX;
  const sourceHeight = sourceBottom - sourceY;
  if (sourceWidth <= 0 || sourceHeight <= 0) return null;

  return {
    source: {
      x: sourceX,
      y: sourceY,
      width: sourceWidth,
      height: sourceHeight,
    },
    destination: {
      x: (sourceX - desiredX) * zoom,
      y: (sourceY - desiredY) * zoom,
      width: sourceWidth * zoom,
      height: sourceHeight * zoom,
    },
  };
}
