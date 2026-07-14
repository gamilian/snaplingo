import type {
  AnnotationCommand,
  CaptureLaunch,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  OcrResult,
  Point,
} from '../../domain/capture';

export type CaptureWorkspaceUnsubscribe = () => void;

export type CaptureWorkspaceRequestHandler = () => void | Promise<void>;
export type CaptureHotkeyHandler = (
  launch: CaptureLaunch,
) => void | Promise<void>;

export interface CaptureWorkspaceEventsPort {
  subscribeCaptureCancel(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  subscribeCaptureCopy(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  subscribeCaptureSave(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  subscribeCaptureUndo(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  subscribeCaptureRedo(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  subscribeHotkeyTriggered(
    handler: CaptureHotkeyHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
}

export interface CaptureWindowPort {
  prepareForReveal(): Promise<void>;
  reveal(): Promise<void>;
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
  logCaptureFrontendPerf(input: {
    event: string;
    mode: CaptureMode;
    sessionId?: string | null;
    elapsedMs: number;
  }): Promise<void>;
  currentCaptureCursorPosition(sessionId: string): Promise<Point | null>;
  cancelCaptureSession(sessionId: string): Promise<void>;
  restoreCaptureSnapshotWindowsForSession(sessionId: string): Promise<void>;
  renderCaptureOutput(input: RenderCaptureOutputInput): Promise<string>;
  defaultCaptureSavePath(options?: CaptureSavePathOptions): Promise<string | null>;
  quickCaptureSavePath(options?: CaptureSavePathOptions): Promise<string>;
  outputCapture(input: OutputCaptureInput): Promise<void>;
  runCaptureOcr(sessionId: string, rect: LogicalRect): Promise<OcrResult>;
  openCaptureOcrResultWindow(text: string, imageBase64?: string): Promise<void>;
  openCaptureTranslationResultWindow(text: string): Promise<void>;
  copyTextToClipboard(text: string): Promise<void>;
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
