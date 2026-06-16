import type { AnnotationCommand, LogicalRect, Point } from './types';

const TEXT_WIDTH_FACTOR = 0.6;
const TEXT_LINE_HEIGHT = 1.2;

function boundsFromPoints(points: Point[], padding = 0): LogicalRect {
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  const minX = Math.min(...xs) - padding;
  const minY = Math.min(...ys) - padding;
  const maxX = Math.max(...xs) + padding;
  const maxY = Math.max(...ys) + padding;

  return {
    x: minX,
    y: minY,
    width: maxX - minX,
    height: maxY - minY,
  };
}

function textBounds(annotation: Extract<AnnotationCommand, { type: 'text' }>) {
  const lines = annotation.text.split('\n');
  const maxLineLength = Math.max(...lines.map((line) => line.length), 1);
  const height = annotation.font_size * TEXT_LINE_HEIGHT * lines.length;

  return {
    x: annotation.position.x,
    y: annotation.position.y - height,
    width: maxLineLength * annotation.font_size * TEXT_WIDTH_FACTOR,
    height,
  };
}

export function getAnnotationBounds(annotation: AnnotationCommand): LogicalRect {
  if (
    annotation.type === 'rectangle' ||
    annotation.type === 'ellipse' ||
    annotation.type === 'mosaic' ||
    annotation.type === 'blur'
  ) {
    return annotation.rect;
  }

  if (annotation.type === 'line' || annotation.type === 'arrow') {
    return boundsFromPoints(
      [annotation.start, annotation.end],
      annotation.stroke_width / 2,
    );
  }

  if (annotation.type === 'freehand' || annotation.type === 'highlight') {
    return boundsFromPoints(annotation.points, annotation.stroke_width / 2);
  }

  return textBounds(annotation);
}

function containsPoint(rect: LogicalRect, point: Point, tolerance: number) {
  return (
    point.x >= rect.x - tolerance &&
    point.x <= rect.x + rect.width + tolerance &&
    point.y >= rect.y - tolerance &&
    point.y <= rect.y + rect.height + tolerance
  );
}

function movePointByDelta(point: Point, delta: Point): Point {
  return {
    x: point.x + delta.x,
    y: point.y + delta.y,
  };
}

export function hitTestAnnotations(
  annotations: AnnotationCommand[],
  point: Point,
  tolerance = 6,
) {
  for (let index = annotations.length - 1; index >= 0; index -= 1) {
    if (containsPoint(getAnnotationBounds(annotations[index]), point, tolerance)) {
      return index;
    }
  }

  return null;
}

export function moveAnnotationByDelta(
  annotation: AnnotationCommand,
  delta: Point,
): AnnotationCommand {
  if (
    annotation.type === 'rectangle' ||
    annotation.type === 'ellipse' ||
    annotation.type === 'mosaic' ||
    annotation.type === 'blur'
  ) {
    return {
      ...annotation,
      rect: {
        ...annotation.rect,
        x: annotation.rect.x + delta.x,
        y: annotation.rect.y + delta.y,
      },
    };
  }

  if (annotation.type === 'line' || annotation.type === 'arrow') {
    return {
      ...annotation,
      start: movePointByDelta(annotation.start, delta),
      end: movePointByDelta(annotation.end, delta),
    };
  }

  if (annotation.type === 'freehand' || annotation.type === 'highlight') {
    return {
      ...annotation,
      points: annotation.points.map((point) => movePointByDelta(point, delta)),
    };
  }

  return {
    ...annotation,
    position: movePointByDelta(annotation.position, delta),
  };
}

export function getAnnotationKeyboardNudgeDelta(
  key: string,
  step: number,
): Point | null {
  if (key === 'ArrowLeft') return { x: -step, y: 0 };
  if (key === 'ArrowRight') return { x: step, y: 0 };
  if (key === 'ArrowUp') return { x: 0, y: -step };
  if (key === 'ArrowDown') return { x: 0, y: step };

  return null;
}
