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

export interface CaptureSessionView {
  id: string;
  monitors: MonitorSnapshotView[];
}

export interface Point {
  x: number;
  y: number;
}

export interface OcrResult {
  text: string;
  confidence: number | null;
}
