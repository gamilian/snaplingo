import { normalizeSelection } from './selection';
import type { AnnotationCommand, Point } from './types';

export type AnnotationTool = 'rectangle' | 'arrow';
export type AnnotationColor = [number, number, number, number];

export interface AnnotationStyle {
  color: AnnotationColor;
  strokeWidth: number;
}

export const ANNOTATION_COLORS: AnnotationColor[] = [
  [255, 77, 79, 255],
  [40, 167, 69, 255],
  [24, 144, 255, 255],
  [250, 219, 20, 255],
  [255, 255, 255, 255],
  [0, 0, 0, 255],
];

export const ANNOTATION_STROKE_WIDTHS = [2, 4, 6, 8];

export const DEFAULT_ANNOTATION_STYLE: AnnotationStyle = {
  color: ANNOTATION_COLORS[0],
  strokeWidth: 2,
};

const MIN_ANNOTATION_SIZE = 4;

export function annotationFromGesture(
  tool: AnnotationTool,
  startPoint: Point,
  currentPoint: Point,
  style: AnnotationStyle,
): AnnotationCommand {
  if (tool === 'rectangle') {
    return {
      type: 'rectangle',
      rect: normalizeSelection(startPoint, currentPoint),
      color: style.color,
      stroke_width: style.strokeWidth,
    };
  }

  return {
    type: 'arrow',
    start: startPoint,
    end: currentPoint,
    color: style.color,
    stroke_width: style.strokeWidth,
  };
}

export function isCommittedAnnotation(annotation: AnnotationCommand) {
  if (annotation.type === 'rectangle') {
    return (
      annotation.rect.width >= MIN_ANNOTATION_SIZE &&
      annotation.rect.height >= MIN_ANNOTATION_SIZE
    );
  }

  return (
    Math.hypot(
      annotation.end.x - annotation.start.x,
      annotation.end.y - annotation.start.y,
    ) >= MIN_ANNOTATION_SIZE
  );
}

export function arrowHeadPoints(start: Point, end: Point, strokeWidth: number) {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const length = Math.hypot(dx, dy);
  if (length === 0) return null;

  const headLength = Math.min(Math.max(6, strokeWidth * 6), length * 0.8);
  const lineAngle = Math.atan2(dy, dx);
  const headAngle = (35 * Math.PI) / 180;
  const wingA = {
    x: end.x + Math.cos(lineAngle + Math.PI - headAngle) * headLength,
    y: end.y + Math.sin(lineAngle + Math.PI - headAngle) * headLength,
  };
  const wingB = {
    x: end.x + Math.cos(lineAngle + Math.PI + headAngle) * headLength,
    y: end.y + Math.sin(lineAngle + Math.PI + headAngle) * headLength,
  };

  return `${end.x},${end.y} ${wingA.x},${wingA.y} ${wingB.x},${wingB.y}`;
}

export function annotationColorToCss(color: AnnotationColor) {
  return `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${color[3] / 255})`;
}
