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
  candidates: CaptureCandidateView[];
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
}

export type CaptureMode = 'screenshot' | 'screenshot-ocr' | 'screenshot-translate';

export interface CaptureLaunch {
  mode: CaptureMode;
  sessionId?: string;
}

export interface Point {
  x: number;
  y: number;
}

export interface OcrResult {
  text: string;
  confidence: number | null;
}
