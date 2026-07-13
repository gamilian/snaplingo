import { normalizeSelection } from './selection';
import type { AnnotationCommand, Point, TextAnnotationCommand } from './types';

export type AnnotationTool =
  | 'rectangle'
  | 'ellipse'
  | 'arrow'
  | 'line'
  | 'pen'
  | 'highlight'
  | 'mosaic'
  | 'text'
  | 'eraser';
export type DrawingAnnotationTool = Exclude<AnnotationTool, 'text'>;
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
  o: 'ellipse',
  a: 'arrow',
  l: 'line',
  p: 'pen',
  h: 'highlight',
  m: 'mosaic',
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

  if (currentTool === 'line') return 'arrow';
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
  if (!options.editing) return null;
  if (event.key === '1') return 'decrease';
  if (event.key === '2') return 'increase';
  if (event.key === '[') return 'decrease';
  if (event.key === ']') return 'increase';

  return null;
}

export function annotationSizeDirectionFromWheel(
  event: AnnotationSizeWheelEvent,
  options: AnnotationSizeShortcutOptions = {},
): AnnotationSizeDirection | null {
  if (
    !options.editing ||
    event.metaKey ||
    event.ctrlKey ||
    event.altKey ||
    event.deltaY === 0
  ) {
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
    tool === 'ellipse'
  ) {
    return constrainToSquare(startPoint, currentPoint);
  }

  if (
    tool === 'line' ||
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
  return (
    tool === 'pen' ||
    tool === 'highlight' ||
    tool === 'mosaic' ||
    tool === 'eraser'
  );
}

function annotationGesturePoint(
  gesture: AnnotationGestureDraft,
  currentPoint: Point,
  constrainGesture: boolean,
) {
  return constrainGesture
    ? constrainAnnotationGesturePoint(
      gesture.tool,
        gesture.startPoint,
        currentPoint,
      )
    : currentPoint;
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

  if (tool === 'mosaic') {
    return {
      type: 'mosaic',
      points: points ?? [startPoint, currentPoint],
      stroke_width: annotationBrushDiameter(tool, style.strokeWidth),
      block_size: mosaicBlockSize(style.strokeWidth),
    };
  }

  if (tool === 'eraser') {
    return {
      type: 'eraser',
      points: points ?? [startPoint, currentPoint],
      stroke_width: annotationBrushDiameter(tool, style.strokeWidth),
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
      stroke_width: annotationBrushDiameter('mosaic', style.strokeWidth),
      block_size: mosaicBlockSize(style.strokeWidth),
    };
  }

  if (annotation.type === 'eraser') {
    return {
      ...annotation,
      stroke_width: annotationBrushDiameter('eraser', style.strokeWidth),
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

function mosaicBlockSize(strokeWidth: number) {
  return Math.max(6, Math.round(strokeWidth * 3));
}

export function annotationBrushDiameter(
  tool: Extract<AnnotationTool, 'mosaic' | 'eraser'>,
  strokeWidth: number,
) {
  const scale = tool === 'mosaic' ? 5 : 4;
  const minimum = tool === 'mosaic' ? 18 : 14;
  return Math.max(minimum, Math.round(strokeWidth * scale));
}

export function isCommittedAnnotation(annotation: AnnotationCommand) {
  if (annotation.type === 'text') {
    return annotation.text.trim().length > 0 && annotation.font_size > 0;
  }

  if (
    annotation.type === 'rectangle' ||
    annotation.type === 'ellipse'
  ) {
    return (
      annotation.rect.width >= MIN_ANNOTATION_SIZE &&
      annotation.rect.height >= MIN_ANNOTATION_SIZE
    );
  }

  if (
    annotation.type === 'freehand' ||
    annotation.type === 'highlight'
  ) {
    if (annotation.points.length < 2) return false;

    const pathLength = annotation.points.slice(1).reduce((total, point, index) => {
      const previousPoint = annotation.points[index];
      return total + Math.hypot(point.x - previousPoint.x, point.y - previousPoint.y);
    }, 0);

    return pathLength >= MIN_ANNOTATION_SIZE;
  }

  if (annotation.type === 'mosaic' || annotation.type === 'eraser') {
    return annotation.points.length > 0;
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
