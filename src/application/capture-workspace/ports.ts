import type {
  AnnotationCommand,
  CaptureCandidateView,
  CaptureLaunch,
  CaptureMode,
  CaptureSessionView,
  MonitorSnapshotView,
  LogicalRect,
  OcrResult,
  Point,
} from '../../domain/capture';

export type CaptureWorkspaceUnsubscribe = () => void;

export interface CaptureWorkspacePrintPort {
  printImage(imageBase64: string): void | Promise<void>;
}

export interface CaptureWorkspacePorts {
  commands: CaptureWorkspaceCommandsPort;
  clipboard: CaptureWorkspaceClipboardPort;
  events: CaptureWorkspaceEventsPort;
  window: CaptureWindowPort;
  print: CaptureWorkspacePrintPort;
}

export type CaptureHotkeyHandler = (
  launch: CaptureLaunch,
) => void | Promise<void>;

export interface CaptureWorkspaceEventsPort {
  subscribeHotkeyTriggered(
    handler: CaptureHotkeyHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
}

export interface CaptureWindowPort {
  // A failed session load can still reveal its error without session geometry.
  prepareForReveal(sessionId: string | null): Promise<void>;
  reveal(sessionId: string | null): Promise<void>;
  hide(): Promise<void>;
}

export type CaptureOutputAction =
  | { type: 'copy' }
  | {
      type: 'save';
      path: string;
      format: 'png' | 'jpg' | 'webp';
      quality: number;
      copyAfterSave: boolean;
    }
  | { type: 'pin' }
  | { type: 'favorite' };

export interface RenderCaptureOutputInput {
  sessionId: string;
  rect: LogicalRect;
  annotations: AnnotationCommand[];
  includeCursor?: boolean;
}

export interface OutputCaptureInput extends RenderCaptureOutputInput {
  action: CaptureOutputAction;
}

export interface CaptureWorkspaceCommandsPort {
  createCaptureSession(): Promise<CaptureSessionView>;
  getCaptureSession(sessionId: string): Promise<CaptureSessionView>;
  hydrateCaptureSessionSnapshots(sessionId: string): Promise<CaptureSessionView>;
  hydrateCaptureMonitorSnapshot(
    sessionId: string,
    monitorId: string,
  ): Promise<MonitorSnapshotView>;
  logCaptureFrontendPerf(input: {
    event: string;
    mode: CaptureMode;
    sessionId?: string | null;
    elapsedMs: number;
  }): Promise<void>;
  currentCaptureCursorPosition(sessionId: string): Promise<Point | null>;
  currentCaptureControlCandidate(
    sessionId: string,
    point: Point,
  ): Promise<CaptureCandidateView | null>;
  moveCaptureCursor(delta: Point): Promise<void>;
  cancelCaptureSession(sessionId: string): Promise<void>;
  restoreCaptureSnapshotWindowsForSession(sessionId: string): Promise<void>;
  renderCaptureOutput(input: RenderCaptureOutputInput): Promise<string>;
  defaultCaptureSavePath(options?: CaptureSavePathOptions): Promise<string | null>;
  quickCaptureSavePath(options?: CaptureSavePathOptions): Promise<string>;
  outputCapture(input: OutputCaptureInput): Promise<void>;
  prepareCaptureOcr(input: PrepareCaptureOcrInput): Promise<void>;
  completeCaptureOcr(sessionId: string): Promise<void>;
  runCaptureOcr(sessionId: string, rect: LogicalRect, language?: string): Promise<OcrResult>;
  openCaptureOcrResultWindow(text: string, imageBase64?: string, confidence?: number): Promise<void>;
  openCaptureTranslationResultWindow(
    text: string,
    detectedLanguage?: string | null,
  ): Promise<void>;
  copyTextToClipboard(text: string): Promise<void>;
}

export interface PrepareCaptureOcrInput {
  sessionId: string;
  rect: LogicalRect;
  annotations: AnnotationCommand[];
  target: 'ocr-window' | 'translation-window' | 'clipboard';
  language?: string;
}

export interface CaptureSavePathOptions {
  directory?: string;
  format?: 'png' | 'jpg' | 'webp';
  namingRule?: 'timestamp' | 'date' | 'counter' | 'custom';
  customFileName?: string;
}

export interface CaptureWorkspaceClipboardPort {
  writeText(text: string): Promise<void>;
}
