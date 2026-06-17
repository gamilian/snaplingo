import { normalizeSelection } from './selection';
import type { AnnotationCommand, Point, TextAnnotationCommand } from './types';

export type AnnotationTool =
  | 'rectangle'
  | 'ellipse'
  | 'arrow'
  | 'line'
  | 'polyline'
  | 'pen'
  | 'highlight'
  | 'mosaic'
  | 'blur'
  | 'text'
  | 'eraser';
export type DrawingAnnotationTool = Exclude<AnnotationTool, 'text' | 'eraser'>;
export type AnnotationColor = [number, number, number, number];

export interface AnnotationStyle {
  color: AnnotationColor;
  strokeWidth: number;
  filled: boolean;
}

export interface AnnotationGestureDraft {
  tool: DrawingAnnotationTool;
  startPoint: Point;
  points?: Point[];
}

interface AnnotationToolShortcutEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey?: boolean;
}

interface AnnotationSizeShortcutOptions {
  editing?: boolean;
}

interface AnnotationSizeWheelEvent {
  deltaY: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
}

export type AnnotationSizeDirection = 'decrease' | 'increase';

export const ANNOTATION_COLORS: AnnotationColor[] = [
  [255, 77, 79, 255],
  [40, 167, 69, 255],
  [24, 144, 255, 255],
  [250, 219, 20, 255],
  [255, 255, 255, 255],
  [0, 0, 0, 255],
];

export const ANNOTATION_STROKE_WIDTHS = [2, 4, 6, 8];
export const DEFAULT_TEXT_FONT_SIZE = 24;
export const MIN_ANNOTATION_STROKE_WIDTH = 1;
export const MAX_ANNOTATION_STROKE_WIDTH = 8;
export const MIN_TEXT_FONT_SIZE = 12;
export const MAX_TEXT_FONT_SIZE = 48;

export const DEFAULT_ANNOTATION_STYLE: AnnotationStyle = {
  color: ANNOTATION_COLORS[0],
  strokeWidth: 2,
  filled: false,
};

const MIN_ANNOTATION_SIZE = 4;
const HIGHLIGHT_ALPHA = 96;

const ANNOTATION_TOOL_SHORTCUTS: Record<string, AnnotationTool> = {
  r: 'rectangle',
  o: 'ellipse',
  a: 'arrow',
  l: 'line',
  p: 'pen',
  h: 'highlight',
  m: 'mosaic',
  b: 'blur',
  t: 'text',
  e: 'eraser',
};

export function annotationToolFromShortcut(
  event: AnnotationToolShortcutEvent,
): AnnotationTool | null {
  if (event.metaKey || event.ctrlKey || event.altKey) return null;

  return ANNOTATION_TOOL_SHORTCUTS[event.key.toLowerCase()] ?? null;
}

export function nextAnnotationToolFromCycleShortcut(
  event: AnnotationToolShortcutEvent,
  currentTool: AnnotationTool | null,
): AnnotationTool | null {
  if (
    event.key !== 'Tab' ||
    event.metaKey ||
    event.ctrlKey ||
    event.altKey ||
    event.shiftKey
  ) {
    return null;
  }

  if (currentTool === 'line') return 'polyline';
  if (currentTool === 'polyline') return 'arrow';
  if (currentTool === 'arrow') return 'line';

  return null;
}

export function annotationColorFromShortcut(
  event: AnnotationToolShortcutEvent,
): AnnotationColor | null {
  if (event.metaKey || event.ctrlKey || event.altKey) return null;
  if (!/^[1-9]$/.test(event.key)) return null;

  const colorIndex = Number(event.key) - 1;
  return ANNOTATION_COLORS[colorIndex] ?? null;
}

export function isAnnotationFillToggleShortcut(
  event: AnnotationToolShortcutEvent,
) {
  return (
    event.key.toLowerCase() === 'f' &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey
  );
}

export function annotationSizeDirectionFromShortcut(
  event: AnnotationToolShortcutEvent,
  options: AnnotationSizeShortcutOptions = {},
): AnnotationSizeDirection | null {
  if (event.metaKey || event.ctrlKey || event.altKey) return null;
  if (options.editing && event.key === '1') return 'decrease';
  if (options.editing && event.key === '2') return 'increase';
  if (event.key === '[') return 'decrease';
  if (event.key === ']') return 'increase';

  return null;
}

export function annotationSizeDirectionFromWheel(
  event: AnnotationSizeWheelEvent,
): AnnotationSizeDirection | null {
  if (event.metaKey || event.ctrlKey || event.altKey || event.deltaY === 0) {
    return null;
  }

  return event.deltaY < 0 ? 'increase' : 'decrease';
}

function stepBoundedValue(
  value: number,
  direction: AnnotationSizeDirection,
  min: number,
  max: number,
) {
  const step = direction === 'increase' ? 1 : -1;
  return Math.min(Math.max(value + step, min), max);
}

export function nextAnnotationStrokeWidth(
  value: number,
  direction: AnnotationSizeDirection,
) {
  return stepBoundedValue(
    value,
    direction,
    MIN_ANNOTATION_STROKE_WIDTH,
    MAX_ANNOTATION_STROKE_WIDTH,
  );
}

export function nextTextFontSize(
  value: number,
  direction: AnnotationSizeDirection,
) {
  return stepBoundedValue(value, direction, MIN_TEXT_FONT_SIZE, MAX_TEXT_FONT_SIZE);
}

function constrainToSquare(startPoint: Point, currentPoint: Point): Point {
  const deltaX = currentPoint.x - startPoint.x;
  const deltaY = currentPoint.y - startPoint.y;
  const size = Math.min(Math.abs(deltaX), Math.abs(deltaY));

  return {
    x: startPoint.x + Math.sign(deltaX) * size,
    y: startPoint.y + Math.sign(deltaY) * size,
  };
}

function constrainToStraightLine(startPoint: Point, currentPoint: Point): Point {
  const deltaX = currentPoint.x - startPoint.x;
  const deltaY = currentPoint.y - startPoint.y;
  const absX = Math.abs(deltaX);
  const absY = Math.abs(deltaY);
  if (absX === 0 || absY === 0) return currentPoint;

  const ratio = absX / absY;
  if (ratio >= 2) return { x: currentPoint.x, y: startPoint.y };
  if (ratio <= 0.5) return { x: startPoint.x, y: currentPoint.y };

  const size = Math.min(absX, absY);
  return {
    x: startPoint.x + Math.sign(deltaX) * size,
    y: startPoint.y + Math.sign(deltaY) * size,
  };
}

export function constrainAnnotationGesturePoint(
  tool: DrawingAnnotationTool,
  startPoint: Point,
  currentPoint: Point,
): Point {
  if (
    tool === 'rectangle' ||
    tool === 'ellipse' ||
    tool === 'mosaic' ||
    tool === 'blur'
  ) {
    return constrainToSquare(startPoint, currentPoint);
  }

  if (
    tool === 'line' ||
    tool === 'polyline' ||
    tool === 'arrow' ||
    tool === 'pen' ||
    tool === 'highlight'
  ) {
    return constrainToStraightLine(startPoint, currentPoint);
  }

  return currentPoint;
}

export function appendAnnotationPoint(points: Point[], point: Point) {
  const previousPoint = points[points.length - 1];
  if (previousPoint && previousPoint.x === point.x && previousPoint.y === point.y) {
    return points;
  }

  return [...points, point];
}

export function isPointStrokeAnnotationTool(tool: AnnotationTool) {
  return tool === 'pen' || tool === 'highlight';
}

function annotationGesturePoint(
  gesture: AnnotationGestureDraft,
  currentPoint: Point,
  constrainGesture: boolean,
) {
  const constraintStartPoint =
    gesture.tool === 'polyline'
      ? gesture.points?.[gesture.points.length - 1] ?? gesture.startPoint
      : gesture.startPoint;

  return constrainGesture
    ? constrainAnnotationGesturePoint(
        gesture.tool,
        constraintStartPoint,
        currentPoint,
      )
    : currentPoint;
}

export function appendAnnotationGesturePoint(
  gesture: AnnotationGestureDraft,
  currentPoint: Point,
  constrainGesture = false,
) {
  return appendAnnotationPoint(
    gesture.points ?? [gesture.startPoint],
    annotationGesturePoint(gesture, currentPoint, constrainGesture),
  );
}

export function undoAnnotationGesturePoint(
  gesture: AnnotationGestureDraft,
): AnnotationGestureDraft | null {
  if (gesture.tool !== 'polyline') return null;

  const points = gesture.points ?? [gesture.startPoint];
  if (points.length <= 1) return null;

  return {
    ...gesture,
    points: points.slice(0, -1),
  };
}

function annotationGesturePoints(
  gesture: AnnotationGestureDraft,
  currentPoint: Point,
  constrainGesture: boolean,
) {
  const gesturePoint = annotationGesturePoint(
    gesture,
    currentPoint,
    constrainGesture,
  );
  if (gesture.tool === 'polyline') {
    return appendAnnotationGesturePoint(gesture, currentPoint, constrainGesture);
  }

  if (!isPointStrokeAnnotationTool(gesture.tool)) return undefined;
  if (constrainGesture) return [gesture.startPoint, gesturePoint];

  return appendAnnotationPoint(gesture.points ?? [gesture.startPoint], currentPoint);
}

export function annotationFromGesture(
  tool: DrawingAnnotationTool,
  startPoint: Point,
  currentPoint: Point,
  style: AnnotationStyle,
  points?: Point[],
): AnnotationCommand {
  if (tool === 'rectangle') {
    return {
      type: 'rectangle',
      rect: normalizeSelection(startPoint, currentPoint),
      color: style.color,
      stroke_width: style.strokeWidth,
      filled: style.filled,
    };
  }

  if (tool === 'ellipse') {
    return {
      type: 'ellipse',
      rect: normalizeSelection(startPoint, currentPoint),
      color: style.color,
      stroke_width: style.strokeWidth,
      filled: style.filled,
    };
  }

  if (tool === 'pen') {
    return {
      type: 'freehand',
      points: points ?? [startPoint, currentPoint],
      color: style.color,
      stroke_width: style.strokeWidth,
    };
  }

  if (tool === 'highlight') {
    return {
      type: 'highlight',
      points: points ?? [startPoint, currentPoint],
      color: [style.color[0], style.color[1], style.color[2], HIGHLIGHT_ALPHA],
      stroke_width: style.strokeWidth,
    };
  }

  if (tool === 'polyline') {
    return {
      type: 'polyline',
      points: points ?? [startPoint, currentPoint],
      color: style.color,
      stroke_width: style.strokeWidth,
    };
  }

  if (tool === 'mosaic') {
    return {
      type: 'mosaic',
      rect: normalizeSelection(startPoint, currentPoint),
      block_size: style.strokeWidth,
    };
  }

  if (tool === 'blur') {
    return {
      type: 'blur',
      rect: normalizeSelection(startPoint, currentPoint),
      radius: style.strokeWidth,
    };
  }

  if (tool === 'line') {
    return {
      type: 'line',
      start: startPoint,
      end: currentPoint,
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

export function annotationFromGestureDraft(
  gesture: AnnotationGestureDraft,
  currentPoint: Point,
  style: AnnotationStyle,
  constrainGesture = false,
) {
  const gesturePoint = annotationGesturePoint(
    gesture,
    currentPoint,
    constrainGesture,
  );

  return annotationFromGesture(
    gesture.tool,
    gesture.startPoint,
    gesturePoint,
    style,
    annotationGesturePoints(gesture, currentPoint, constrainGesture),
  );
}

export function completeAnnotationGesture(
  gesture: AnnotationGestureDraft,
  currentPoint: Point,
  style: AnnotationStyle,
  constrainGesture = false,
) {
  const annotation = annotationFromGestureDraft(
    gesture,
    currentPoint,
    style,
    constrainGesture,
  );

  return isCommittedAnnotation(annotation) ? annotation : null;
}

export function annotationFromText(
  position: Point,
  text: string,
  style: AnnotationStyle,
  fontSize = DEFAULT_TEXT_FONT_SIZE,
): TextAnnotationCommand {
  return {
    type: 'text',
    position,
    text,
    color: style.color,
    font_size: fontSize,
  };
}

export function applyAnnotationStyle(
  annotation: AnnotationCommand,
  style: AnnotationStyle,
  fontSize = DEFAULT_TEXT_FONT_SIZE,
): AnnotationCommand {
  if (annotation.type === 'mosaic') {
    return {
      ...annotation,
      block_size: style.strokeWidth,
    };
  }

  if (annotation.type === 'blur') {
    return {
      ...annotation,
      radius: style.strokeWidth,
    };
  }

  if (annotation.type === 'text') {
    return {
      ...annotation,
      color: style.color,
      font_size: fontSize,
    };
  }

  if (annotation.type === 'highlight') {
    return {
      ...annotation,
      color: [style.color[0], style.color[1], style.color[2], HIGHLIGHT_ALPHA],
      stroke_width: style.strokeWidth,
    };
  }

  if (annotation.type === 'rectangle' || annotation.type === 'ellipse') {
    return {
      ...annotation,
      color: style.color,
      stroke_width: style.strokeWidth,
      filled: style.filled,
    };
  }

  return {
    ...annotation,
    color: style.color,
    stroke_width: style.strokeWidth,
  };
}

export function isCommittedAnnotation(annotation: AnnotationCommand) {
  if (annotation.type === 'text') {
    return annotation.text.trim().length > 0 && annotation.font_size > 0;
  }

  if (
    annotation.type === 'rectangle' ||
    annotation.type === 'ellipse' ||
    annotation.type === 'mosaic' ||
    annotation.type === 'blur'
  ) {
    return (
      annotation.rect.width >= MIN_ANNOTATION_SIZE &&
      annotation.rect.height >= MIN_ANNOTATION_SIZE
    );
  }

  if (
    annotation.type === 'freehand' ||
    annotation.type === 'highlight' ||
    annotation.type === 'polyline'
  ) {
    if (annotation.points.length < 2) return false;

    const pathLength = annotation.points.slice(1).reduce((total, point, index) => {
      const previousPoint = annotation.points[index];
      return total + Math.hypot(point.x - previousPoint.x, point.y - previousPoint.y);
    }, 0);

    return pathLength >= MIN_ANNOTATION_SIZE;
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
