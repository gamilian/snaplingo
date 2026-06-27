export interface LogicalRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface PhysicalRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MonitorSnapshotView {
  id: string;
  logical_bounds: LogicalRect;
  physical_bounds: PhysicalRect;
  scale_factor: number;
  image_base64: string;
}

export interface CapturedCursorView {
  logical_position: Point;
  hotspot: Point;
  image_width: number;
  image_height: number;
  scale_factor: number;
  image_base64: string;
}

export interface CaptureSessionView {
  id: string;
  monitors: MonitorSnapshotView[];
  candidates: CaptureCandidateView[];
  captured_cursor?: CapturedCursorView | null;
}

export type CaptureCandidateKind = 'monitor' | 'window' | 'control';

export interface CaptureCandidateView {
  id: string;
  kind: CaptureCandidateKind;
  rect: LogicalRect;
  priority: number;
}

export interface PinnedImageView {
  id: string;
  image_base64: string;
  width: number;
  height: number;
  source_text?: string | null;
}

export type CaptureMode =
  | 'screenshot'
  | 'screenshot-copy'
  | 'screenshot-ocr'
  | 'silent-screenshot-ocr'
  | 'screenshot-translate';

export interface CaptureLaunch {
  mode: CaptureMode;
  sessionId?: string;
}

export interface Point {
  x: number;
  y: number;
}

export type ArrowKey = 'ArrowUp' | 'ArrowRight' | 'ArrowDown' | 'ArrowLeft';

export interface OcrResult {
  text: string;
  confidence: number | null;
}

export interface RectangleAnnotationCommand {
  type: 'rectangle';
  rect: LogicalRect;
  color: [number, number, number, number];
  stroke_width: number;
  filled: boolean;
}

export interface EllipseAnnotationCommand {
  type: 'ellipse';
  rect: LogicalRect;
  color: [number, number, number, number];
  stroke_width: number;
  filled: boolean;
}

export interface ArrowAnnotationCommand {
  type: 'arrow';
  start: Point;
  end: Point;
  color: [number, number, number, number];
  stroke_width: number;
}

export interface LineAnnotationCommand {
  type: 'line';
  start: Point;
  end: Point;
  color: [number, number, number, number];
  stroke_width: number;
}

export interface PolylineAnnotationCommand {
  type: 'polyline';
  points: Point[];
  color: [number, number, number, number];
  stroke_width: number;
}

export interface FreehandAnnotationCommand {
  type: 'freehand';
  points: Point[];
  color: [number, number, number, number];
  stroke_width: number;
}

export interface HighlightAnnotationCommand {
  type: 'highlight';
  points: Point[];
  color: [number, number, number, number];
  stroke_width: number;
}

export interface MosaicAnnotationCommand {
  type: 'mosaic';
  rect: LogicalRect;
  block_size: number;
}

export interface BlurAnnotationCommand {
  type: 'blur';
  rect: LogicalRect;
  radius: number;
}

export interface TextAnnotationCommand {
  type: 'text';
  position: Point;
  text: string;
  color: [number, number, number, number];
  font_size: number;
}

export type AnnotationCommand =
  | RectangleAnnotationCommand
  | EllipseAnnotationCommand
  | ArrowAnnotationCommand
  | LineAnnotationCommand
  | PolylineAnnotationCommand
  | FreehandAnnotationCommand
  | HighlightAnnotationCommand
  | MosaicAnnotationCommand
  | BlurAnnotationCommand
  | TextAnnotationCommand;
