import type { AnnotationCommand, LogicalRect, Point } from '../../domain/capture';
import { resizeSelectionByHandle, type SelectionHandle } from './selection';
import { constrainAnnotationGesturePoint, MAX_TEXT_RESIZE_FONT_SIZE, MIN_TEXT_RESIZE_FONT_SIZE } from './annotationStyle';

export type AnnotationHandle = SelectionHandle | 'arrow-start' | 'arrow-end' | 'control1' | 'control2' | 'radius';

export function arrowControlPoints(annotation: Extract<AnnotationCommand, { type: 'arrow' }>): [Point, Point] {
  const between = (t: number) => ({
    x: annotation.start.x + (annotation.end.x - annotation.start.x) * t,
    y: annotation.start.y + (annotation.end.y - annotation.start.y) * t,
  });
  return [annotation.control1 ?? between(1 / 3), annotation.control2 ?? between(2 / 3)];
}

export function arrowCurvePoints(annotation: Extract<AnnotationCommand, { type: 'arrow' }>): Point[] {
  const [c1, c2] = arrowControlPoints(annotation);
  const length = Math.hypot(c1.x - annotation.start.x, c1.y - annotation.start.y)
    + Math.hypot(c2.x - c1.x, c2.y - c1.y)
    + Math.hypot(annotation.end.x - c2.x, annotation.end.y - c2.y);
  const steps = Math.min(256, Math.max(16, Math.ceil(length / 8)));
  return Array.from({ length: steps + 1 }, (_, i) => {
    const t = i / steps;
    const u = 1 - t;
    return {
      x: u ** 3 * annotation.start.x + 3 * u ** 2 * t * c1.x + 3 * u * t ** 2 * c2.x + t ** 3 * annotation.end.x,
      y: u ** 3 * annotation.start.y + 3 * u ** 2 * t * c1.y + 3 * u * t ** 2 * c2.y + t ** 3 * annotation.end.y,
    };
  });
}

export function annotationSpecialHandles(annotation: AnnotationCommand): { handle: AnnotationHandle; point: Point }[] {
  if (annotation.type === 'arrow') {
    const [c1, c2] = arrowControlPoints(annotation);
    return [
      { handle: 'arrow-start', point: annotation.start },
      { handle: 'arrow-end', point: annotation.end },
      { handle: 'control1', point: c1 },
      { handle: 'control2', point: c2 },
    ];
  }
  if ('corner_radius' in annotation || annotation.type === 'rectangle' || annotation.type === 'blur' || annotation.type === 'spotlight' || annotation.type === 'region-highlight') {
    if (!('rect' in annotation)) return [];
    const radius = Math.min(annotation.corner_radius ?? 0, Math.min(annotation.rect.width, annotation.rect.height) / 2);
    return [{ handle: 'radius', point: { x: annotation.rect.x + annotation.rect.width - 8 - radius / 2, y: annotation.rect.y + 8 + radius / 2 } }];
  }
  return [];
}

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
  if ('rect' in annotation) {
    return annotation.rect;
  }

  if (annotation.type === 'line' || annotation.type === 'arrow') {
    return boundsFromPoints(
      annotation.type === 'arrow' ? arrowCurvePoints(annotation) : [annotation.start, annotation.end],
      annotation.stroke_width / 2,
    );
  }

  if (
    annotation.type === 'freehand' ||
    annotation.type === 'highlight' ||
    annotation.type === 'mosaic' ||
    annotation.type === 'eraser'
  ) {
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
    const annotation = annotations[index];
    if (annotation.type === 'arrow' && (annotation.control1 || annotation.control2)) {
      const curve = arrowCurvePoints(annotation);
      const hit = curve.slice(1).some((end, i) => {
        const start = curve[i];
        const dx = end.x - start.x;
        const dy = end.y - start.y;
        const lengthSquared = dx * dx + dy * dy;
        const t = lengthSquared ? Math.max(0, Math.min(1, ((point.x - start.x) * dx + (point.y - start.y) * dy) / lengthSquared)) : 0;
        return Math.hypot(point.x - start.x - dx * t, point.y - start.y - dy * t) <= tolerance + annotation.stroke_width / 2;
      });
      if (hit) return index;
    } else if (containsPoint(getAnnotationBounds(annotation), point, tolerance)) {
      return index;
    }
  }

  return null;
}

export function moveAnnotationByDelta(
  annotation: AnnotationCommand,
  delta: Point,
): AnnotationCommand {
  if ('rect' in annotation) {
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
      ...(annotation.type === 'arrow' && annotation.control1 ? { control1: movePointByDelta(annotation.control1, delta) } : {}),
      ...(annotation.type === 'arrow' && annotation.control2 ? { control2: movePointByDelta(annotation.control2, delta) } : {}),
    };
  }

  if (
    annotation.type === 'freehand' ||
    annotation.type === 'highlight' ||
    annotation.type === 'mosaic' ||
    annotation.type === 'eraser'
  ) {
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

export function resizeAnnotationByHandle(
  annotation: AnnotationCommand,
  handle: AnnotationHandle,
  delta: Point,
  bounds: LogicalRect,
  preserveAspect = false,
): AnnotationCommand {
  if (delta.x === 0 && delta.y === 0) return annotation;
  if (handle === 'radius' && 'rect' in annotation && annotation.type !== 'ellipse') {
    return {
      ...annotation,
      corner_radius: Math.max(0, Math.min(
        Math.min(annotation.rect.width, annotation.rect.height) / 2,
        (annotation.corner_radius ?? 0) + (delta.y - delta.x) / 2,
      )),
    };
  }
  if (annotation.type === 'arrow') {
    const [control1, control2] = arrowControlPoints(annotation);
    if (handle === 'control1' || handle === 'control2') {
      return { ...annotation, control1, control2, [handle]: movePointByDelta(handle === 'control1' ? control1 : control2, delta) };
    }
    if (handle === 'arrow-start' || handle === 'arrow-end') {
      const moving = handle === 'arrow-start' ? annotation.start : annotation.end;
      const anchor = handle === 'arrow-start' ? annotation.end : annotation.start;
      const next = movePointByDelta(moving, delta);
      const target = preserveAspect ? constrainAnnotationGesturePoint('arrow', anchor, next) : next;
      const dx = moving.x - anchor.x;
      const dy = moving.y - anchor.y;
      const denominator = dx * dx + dy * dy;
      const real = denominator > 0 ? ((target.x - anchor.x) * dx + (target.y - anchor.y) * dy) / denominator : 1;
      const imaginary = denominator > 0 ? ((target.y - anchor.y) * dx - (target.x - anchor.x) * dy) / denominator : 0;
      const transform = (p: Point) => ({ x: anchor.x + real * (p.x - anchor.x) - imaginary * (p.y - anchor.y), y: anchor.y + imaginary * (p.x - anchor.x) + real * (p.y - anchor.y) });
      return { ...annotation, [handle === 'arrow-start' ? 'start' : 'end']: target, control1: transform(control1), control2: transform(control2) };
    }
    return annotation;
  }
  if (handle === 'control1' || handle === 'control2' || handle === 'arrow-start' || handle === 'arrow-end' || handle === 'radius') return annotation;
  if ('rect' in annotation) {
    return { ...annotation, rect: resizeSelectionByHandle(annotation.rect, handle, delta, bounds, 4, preserveAspect) };
  }
  if (annotation.type === 'text') {
    const original = getAnnotationBounds(annotation);
    const target = resizeSelectionByHandle(original, handle, delta, bounds, 4, preserveAspect);
    const scale = target.height / original.height;
    const fontSize = Math.max(MIN_TEXT_RESIZE_FONT_SIZE, Math.min(MAX_TEXT_RESIZE_FONT_SIZE, Math.round(annotation.font_size * scale)));
    return { ...annotation, font_size: fontSize, position: { x: target.x, y: target.y + fontSize * TEXT_LINE_HEIGHT * annotation.text.split('\n').length } };
  }
  return annotation;
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

export function constrainAnnotationMoveDelta(delta: Point): Point {
  const absX = Math.abs(delta.x);
  const absY = Math.abs(delta.y);
  if (absX > absY) return { x: delta.x, y: 0 };
  if (absY > absX) return { x: 0, y: delta.y };

  return delta;
}
