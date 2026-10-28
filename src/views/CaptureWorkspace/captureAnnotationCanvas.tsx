import { annotationColorToCss } from '../../components/common/annotationColorPresentation';
import { useLayoutEffect, useRef, type RefObject } from 'react';

import { arrowHeadPoints } from '../../application/capture-workspace/annotationStyle';
import type {
  AnnotationCommand,
  CapturedCursorView,
  LogicalRect,
  MonitorSnapshotView,
  Point,
} from './types';

const WINDOWS_CAPTURE_TEXT_FONT_FAMILY =
  '"Segoe UI Variable Text", "Segoe UI", system-ui, "Microsoft YaHei", sans-serif';
const APPLE_CAPTURE_TEXT_FONT_FAMILY =
  '-apple-system, BlinkMacSystemFont, "PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif';

interface CaptureAnnotationCanvasProps {
  desktopRef: RefObject<HTMLDivElement>;
  monitors: readonly MonitorSnapshotView[];
  outputPixelSize: { width: number; height: number };
  capturedCursor?: CapturedCursorView | null;
  selection: LogicalRect;
  annotations: AnnotationCommand[];
  draftAnnotation: AnnotationCommand | null;
  selectionViewportRect: LogicalRect;
}

function drawPath(
  context: CanvasRenderingContext2D,
  points: Point[],
) {
  const first = points[0];
  if (!first) return;
  context.beginPath();
  context.moveTo(first.x, first.y);
  for (const point of points.slice(1)) context.lineTo(point.x, point.y);
  context.stroke();
}

function drawPixelatedRect(
  context: CanvasRenderingContext2D,
  source: HTMLCanvasElement,
  rect: LogicalRect,
  blockSize: number,
  buffer: HTMLCanvasElement,
) {
  const width = Math.max(1, Math.round(rect.width));
  const height = Math.max(1, Math.round(rect.height));
  const sampleWidth = Math.max(1, Math.ceil(width / Math.max(1, blockSize)));
  const sampleHeight = Math.max(1, Math.ceil(height / Math.max(1, blockSize)));
  if (buffer.width !== sampleWidth) buffer.width = sampleWidth;
  if (buffer.height !== sampleHeight) buffer.height = sampleHeight;
  const bufferContext = buffer.getContext('2d');
  if (!bufferContext) return;
  const pixelRatio = context.getTransform().a || 1;

  bufferContext.clearRect(0, 0, sampleWidth, sampleHeight);
  bufferContext.imageSmoothingEnabled = true;
  bufferContext.drawImage(
    source,
    rect.x * pixelRatio,
    rect.y * pixelRatio,
    width * pixelRatio,
    height * pixelRatio,
    0,
    0,
    sampleWidth,
    sampleHeight,
  );
  context.save();
  context.imageSmoothingEnabled = false;
  context.drawImage(
    buffer,
    0,
    0,
    sampleWidth,
    sampleHeight,
    rect.x,
    rect.y,
    width,
    height,
  );
  context.restore();
}

function brushBounds(points: Point[], diameter: number): LogicalRect | null {
  const first = points[0];
  if (!first) return null;
  const radius = diameter / 2;
  let minX = first.x;
  let maxX = first.x;
  let minY = first.y;
  let maxY = first.y;
  for (const point of points.slice(1)) {
    minX = Math.min(minX, point.x);
    maxX = Math.max(maxX, point.x);
    minY = Math.min(minY, point.y);
    maxY = Math.max(maxY, point.y);
  }
  return {
    x: minX - radius,
    y: minY - radius,
    width: Math.max(diameter, maxX - minX + diameter),
    height: Math.max(diameter, maxY - minY + diameter),
  };
}

function alignMosaicRectToGrid(
  rect: LogicalRect,
  blockSize: number,
  maxWidth: number,
  maxHeight: number,
): LogicalRect {
  const size = Math.max(1, blockSize);
  const x = Math.max(0, Math.floor(rect.x / size) * size);
  const y = Math.max(0, Math.floor(rect.y / size) * size);
  const right = Math.min(maxWidth, Math.ceil((rect.x + rect.width) / size) * size);
  const bottom = Math.min(maxHeight, Math.ceil((rect.y + rect.height) / size) * size);
  return {
    x,
    y,
    width: Math.max(1, right - x),
    height: Math.max(1, bottom - y),
  };
}

function appendBrushPath(
  context: CanvasRenderingContext2D,
  points: Point[],
  diameter: number,
) {
  const first = points[0];
  if (!first) return;
  const radius = diameter / 2;
  context.beginPath();
  const addCircle = (point: Point) => {
    context.moveTo(point.x + radius, point.y);
    context.arc(point.x, point.y, radius, 0, Math.PI * 2);
  };
  addCircle(first);
  for (let index = 1; index < points.length; index += 1) {
    const previous = points[index - 1];
    const point = points[index];
    const steps = Math.max(
      1,
      Math.ceil(Math.hypot(point.x - previous.x, point.y - previous.y) / radius),
    );
    for (let step = 1; step <= steps; step += 1) {
      addCircle({
        x: previous.x + ((point.x - previous.x) * step) / steps,
        y: previous.y + ((point.y - previous.y) * step) / steps,
      });
    }
  }
}

function drawPixelatedBrush(
  context: CanvasRenderingContext2D,
  source: HTMLCanvasElement,
  points: Point[],
  diameter: number,
  blockSize: number,
  buffer: HTMLCanvasElement,
) {
  const bounds = brushBounds(points, diameter);
  if (!bounds) return;
  const pixelRatio = context.getTransform().a || 1;
  const rect = alignMosaicRectToGrid(
    bounds,
    blockSize,
    source.width / pixelRatio,
    source.height / pixelRatio,
  );
  context.save();
  appendBrushPath(context, points, diameter);
  context.clip();
  drawPixelatedRect(context, source, rect, blockSize, buffer);
  context.restore();
}

function eraseBrush(
  context: CanvasRenderingContext2D,
  points: Point[],
  diameter: number,
) {
  context.save();
  context.globalCompositeOperation = 'destination-out';
  appendBrushPath(context, points, diameter);
  context.fill();
  context.restore();
}

export function drawCaptureAnnotation(
  context: CanvasRenderingContext2D,
  annotation: AnnotationCommand,
  buffers?: {
    mosaic: HTMLCanvasElement;
    source: HTMLCanvasElement;
  },
) {
  if (annotation.type === 'mosaic') {
    drawPixelatedBrush(
      context,
      buffers?.source ?? context.canvas,
      annotation.points,
      annotation.stroke_width,
      annotation.block_size,
      buffers?.mosaic ?? document.createElement('canvas'),
    );
    return;
  }
  if (annotation.type === 'eraser') {
    eraseBrush(context, annotation.points, annotation.stroke_width);
    return;
  }

  context.save();
  if (annotation.type === 'rectangle' || annotation.type === 'ellipse') {
    context.lineWidth = annotation.stroke_width;
    context.strokeStyle = annotationColorToCss(annotation.color);
    context.fillStyle = annotationColorToCss(annotation.color);
    context.beginPath();
    if (annotation.type === 'rectangle') {
      context.rect(
        annotation.rect.x,
        annotation.rect.y,
        annotation.rect.width,
        annotation.rect.height,
      );
    } else {
      context.ellipse(
        annotation.rect.x + annotation.rect.width / 2,
        annotation.rect.y + annotation.rect.height / 2,
        annotation.rect.width / 2,
        annotation.rect.height / 2,
        0,
        0,
        Math.PI * 2,
      );
    }
    if (annotation.filled) context.fill();
    context.stroke();
  } else if (annotation.type === 'line' || annotation.type === 'arrow') {
    context.lineCap = 'round';
    context.lineWidth = annotation.stroke_width;
    context.strokeStyle = annotationColorToCss(annotation.color);
    drawPath(context, [annotation.start, annotation.end]);
    if (annotation.type === 'arrow') {
      const arrowPoints = arrowHeadPoints(
        annotation.start,
        annotation.end,
        annotation.stroke_width,
      );
      if (arrowPoints) {
        const points = arrowPoints.split(' ').map((point) => {
          const [x, y] = point.split(',').map(Number);
          return { x, y };
        });
        context.fillStyle = annotationColorToCss(annotation.color);
        context.beginPath();
        context.moveTo(points[0].x, points[0].y);
        for (const point of points.slice(1)) context.lineTo(point.x, point.y);
        context.closePath();
        context.fill();
      }
    }
  } else if (
    annotation.type === 'freehand' ||
    annotation.type === 'highlight'
  ) {
    context.lineCap = 'round';
    context.lineJoin = 'round';
    context.lineWidth = annotation.stroke_width;
    context.strokeStyle = annotationColorToCss(annotation.color);
    drawPath(context, annotation.points);
  } else if (annotation.type === 'text') {
    context.fillStyle = annotationColorToCss(annotation.color);
    const isWindows = typeof navigator !== 'undefined' && /^Win/i.test(navigator.platform);
    const fontFamily = isWindows
      ? WINDOWS_CAPTURE_TEXT_FONT_FAMILY
      : APPLE_CAPTURE_TEXT_FONT_FAMILY;
    context.font = `${annotation.font_size}px ${fontFamily}`;
    context.textBaseline = 'alphabetic';
    annotation.text.split('\n').forEach((line, index) => {
      context.fillText(
        line,
        annotation.position.x,
        annotation.position.y + index * annotation.font_size * 1.2,
      );
    });
  }
  context.restore();
}

export function drawFrozenCaptureSelection(
  context: CanvasRenderingContext2D,
  selection: LogicalRect,
  monitors: readonly MonitorSnapshotView[],
  images: ReadonlyMap<string, HTMLImageElement>,
  capturedCursor?: CapturedCursorView | null,
  cursorImage?: HTMLImageElement | null,
) {
  context.clearRect(0, 0, selection.width, selection.height);
  for (const monitor of monitors) {
    const bounds = monitor.logical_bounds;
    const image = images.get(monitor.id);
    if (
      !image ||
      bounds.x >= selection.x + selection.width ||
      bounds.y >= selection.y + selection.height ||
      bounds.x + bounds.width <= selection.x ||
      bounds.y + bounds.height <= selection.y
    ) continue;
    context.drawImage(
      image,
      bounds.x - selection.x,
      bounds.y - selection.y,
      bounds.width,
      bounds.height,
    );
  }
  if (capturedCursor && cursorImage) {
    context.drawImage(
      cursorImage,
      capturedCursor.logical_position.x - capturedCursor.hotspot.x - selection.x,
      capturedCursor.logical_position.y - capturedCursor.hotspot.y - selection.y,
      capturedCursor.image_width / Math.max(1, capturedCursor.scale_factor),
      capturedCursor.image_height / Math.max(1, capturedCursor.scale_factor),
    );
  }
}

export function getCaptureAnnotationCanvasBackingSize(
  selection: LogicalRect,
  localPixelRatio: number,
  outputPixelSize: { width: number; height: number },
) {
  const localScale = Number.isFinite(localPixelRatio)
    ? Math.max(1, localPixelRatio)
    : 1;
  const outputScale = Math.max(
    outputPixelSize.width / selection.width,
    outputPixelSize.height / selection.height,
  );
  const pixelRatio = Math.max(localScale, outputScale);

  return {
    pixelRatio,
    width: Math.max(1, outputPixelSize.width, Math.ceil(selection.width * pixelRatio)),
    height: Math.max(1, outputPixelSize.height, Math.ceil(selection.height * pixelRatio)),
  };
}

export function CaptureAnnotationCanvas({
  desktopRef,
  monitors,
  outputPixelSize,
  capturedCursor,
  selection,
  annotations,
  draftAnnotation,
  selectionViewportRect,
}: CaptureAnnotationCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const draftCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const mosaicBufferRef = useRef<HTMLCanvasElement | null>(null);
  const sourceRef = useRef<HTMLCanvasElement | null>(null);
  const paintedDraftRef = useRef<{
    annotation: AnnotationCommand | null;
    monitors: readonly MonitorSnapshotView[];
    capturedCursor?: CapturedCursorView | null;
    pixelRatio: number;
    pixelWidth: number;
    pixelHeight: number;
    selection: LogicalRect;
  } | null>(null);
  const { pixelRatio, width: pixelWidth, height: pixelHeight } =
    getCaptureAnnotationCanvasBackingSize(
      selection,
      window.devicePixelRatio || 1,
      outputPixelSize,
    );
  const nonMosaicDraftAnnotation = draftAnnotation?.type === 'mosaic'
    ? null
    : draftAnnotation;

  useLayoutEffect(() => {
    const sourceContext = sourceRef.current?.getContext('2d');
    const desktop = desktopRef.current;
    if (!sourceContext || !desktop) return;
    sourceContext.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
    const images = new Map(
      Array.from(desktop.querySelectorAll<HTMLImageElement>('[data-capture-monitor]'))
        .map((image) => [image.dataset.captureMonitor!, image]),
    );
    drawFrozenCaptureSelection(
      sourceContext,
      selection,
      monitors,
      images,
      capturedCursor,
      desktop.querySelector<HTMLImageElement>('[data-capture-cursor]'),
    );
  }, [
    desktopRef,
    monitors,
    capturedCursor,
    selection,
    pixelRatio,
    pixelWidth,
    pixelHeight,
  ]);

  useLayoutEffect(() => {
    const context = canvasRef.current?.getContext('2d');
    if (!context) return;
    context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
    context.clearRect(0, 0, selection.width, selection.height);
    for (const annotation of annotations) {
      drawCaptureAnnotation(context, annotation, {
        mosaic: mosaicBufferRef.current ??
          (mosaicBufferRef.current = document.createElement('canvas')),
        source: sourceRef.current ?? context.canvas,
      });
    }
    if (nonMosaicDraftAnnotation) {
      drawCaptureAnnotation(context, nonMosaicDraftAnnotation, {
        mosaic: mosaicBufferRef.current ??
          (mosaicBufferRef.current = document.createElement('canvas')),
        source: sourceRef.current ?? context.canvas,
      });
    }
  }, [
    annotations,
    nonMosaicDraftAnnotation,
    selection,
    pixelRatio,
    pixelWidth,
    pixelHeight,
  ]);

  useLayoutEffect(() => {
    const context = draftCanvasRef.current?.getContext('2d');
    const source = sourceRef.current;
    if (!context || !source) return;
    const buffers = {
      mosaic: mosaicBufferRef.current ??
        (mosaicBufferRef.current = document.createElement('canvas')),
      source,
    };
    const previous = paintedDraftRef.current;
    const sameSurface = previous &&
      previous.monitors === monitors &&
      previous.capturedCursor === capturedCursor &&
      previous.pixelRatio === pixelRatio &&
      previous.pixelWidth === pixelWidth &&
      previous.pixelHeight === pixelHeight &&
      previous.selection === selection;
    const previousMosaic = previous?.annotation?.type === 'mosaic'
      ? previous.annotation
      : null;
    const nextMosaic = draftAnnotation?.type === 'mosaic' ? draftAnnotation : null;
    const isMosaicExtension = Boolean(
      sameSurface && previousMosaic && nextMosaic &&
      previousMosaic.stroke_width === nextMosaic.stroke_width &&
      previousMosaic.block_size === nextMosaic.block_size &&
      nextMosaic.points.length >= previousMosaic.points.length &&
      previousMosaic.points.every((point, index) => {
        const nextPoint = nextMosaic.points[index];
        return point.x === nextPoint?.x && point.y === nextPoint?.y;
      }),
    );

    if (isMosaicExtension && previousMosaic && nextMosaic) {
      const addedPoints = nextMosaic.points.slice(
        Math.max(0, previousMosaic.points.length - 1),
      );
      if (addedPoints.length > 1) {
        drawCaptureAnnotation(context, { ...nextMosaic, points: addedPoints }, buffers);
      }
    } else {
      context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
      context.clearRect(0, 0, selection.width, selection.height);
      if (nextMosaic) drawCaptureAnnotation(context, nextMosaic, buffers);
    }

    paintedDraftRef.current = {
      annotation: draftAnnotation,
      monitors,
      capturedCursor,
      pixelRatio,
      pixelWidth,
      pixelHeight,
      selection,
    };
  }, [
    draftAnnotation,
    monitors,
    capturedCursor,
    selection,
    pixelRatio,
    pixelWidth,
    pixelHeight,
  ]);

  const style = {
    left: `${selectionViewportRect.x}px`,
    top: `${selectionViewportRect.y}px`,
    width: `${selectionViewportRect.width}px`,
    height: `${selectionViewportRect.height}px`,
  };
  return (
    <>
      <canvas
        ref={sourceRef}
        data-capture-preview
        width={pixelWidth}
        height={pixelHeight}
        className="pointer-events-none absolute"
        style={style}
      />
      <canvas
        ref={canvasRef}
        data-capture-annotations
        width={pixelWidth}
        height={pixelHeight}
        className="pointer-events-none absolute"
        style={style}
      />
      <canvas
        ref={draftCanvasRef}
        data-capture-draft
        width={pixelWidth}
        height={pixelHeight}
        className="pointer-events-none absolute"
        style={style}
      />
    </>
  );
}
