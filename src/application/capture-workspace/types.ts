import type {
  AnnotationCommand,
  CaptureLaunch,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from '../../domain/capture';
import type {
  HoverSelectionCompletionAction,
  PreviewCaptureCompletionAction,
} from '../../views/CaptureWorkspace/captureActions';
import type { CaptureWorkspaceCommandsPort } from './ports';

export type CaptureWorkspaceRuntimeStatus =
  | 'idle'
  | 'loading'
  | 'selecting'
  | 'preview'
  | 'error';

export interface CaptureWorkspaceRenderState {
  readonly status: CaptureWorkspaceRuntimeStatus;
  readonly mode: CaptureMode;
  readonly session: CaptureSessionView | null;
  readonly sessionId: string | null;
  readonly cursorPoint: Point | null;
  readonly startPoint: Point | null;
  readonly selection: LogicalRect | null;
  readonly hoverSelection: LogicalRect | null;
  readonly previewImageBase64: string | null;
  readonly isRenderingOutput: boolean;
  readonly hasHydratedPixelSource: boolean;
  readonly error: string | null;
}

export interface CaptureWorkspaceRuntimeActions {
  connectHost(): Promise<() => void>;
  updateHostReadiness(imagesReady: boolean): Promise<void>;
  startSession(mode: CaptureMode, sessionId?: string): Promise<void>;
  refreshSession(): Promise<void>;
  cancelSession(): Promise<void>;
  renderSelectionPreview(
    rect: LogicalRect,
    annotations?: AnnotationCommand[],
    includeCursor?: boolean,
  ): Promise<void>;
  completeCandidateSelection(
    rect: LogicalRect,
    action?: HoverSelectionCompletionAction,
  ): Promise<void>;
  completeManualSelection(rect: LogicalRect): Promise<void>;
  completePreviewSelection(
    action: PreviewCaptureCompletionAction,
    rect: LogicalRect,
    annotations?: AnnotationCommand[],
    includeCursor?: boolean,
  ): Promise<void>;
  resetPreview(): void;
  pointerDown(input: Point | CaptureWorkspacePointerInput): void;
  pointerMove(input: Point | CaptureWorkspacePointerInput): void;
  pointerUp(input: Point | CaptureWorkspacePointerInput): Promise<void>;
  keyDown(event: { key: string }): Promise<boolean>;
  hydrateSnapshots(): Promise<void>;
}

export interface CaptureWorkspacePointerInput {
  point: Point;
  button?: number;
  shiftKey?: boolean;
}

export interface CaptureWorkspaceRuntime {
  readonly renderState: CaptureWorkspaceRenderState;
  readonly actions: CaptureWorkspaceRuntimeActions;
  subscribe(listener: () => void): () => void;
}

export interface CaptureWorkspaceRuntimePlatform {
  commands: CaptureWorkspaceCommandsPort;
  onCancelRequested(handler: () => void | Promise<void>): Promise<() => void>;
  onCopyRequested(handler: () => void | Promise<void>): Promise<() => void>;
  onHotkeyTriggered(
    handler: (launch: CaptureLaunch) => void | Promise<void>,
  ): Promise<() => void>;
  prepareForReveal(): Promise<void>;
  reveal(): Promise<void>;
  dismiss(): Promise<void>;
}
