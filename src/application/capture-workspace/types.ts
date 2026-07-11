import type {
  AnnotationCommand,
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
  readonly selection: LogicalRect | null;
  readonly hoverSelection: LogicalRect | null;
  readonly previewImageBase64: string | null;
  readonly isRenderingOutput: boolean;
  readonly hasHydratedPixelSource: boolean;
  readonly error: string | null;
}

export interface CaptureWorkspaceRuntimeActions {
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
  pointerDown(point: Point): void;
  pointerMove(point: Point): void;
  pointerUp(point: Point): Promise<void>;
  keyDown(event: { key: string }): Promise<void>;
  hydrateSnapshots(): Promise<void>;
}

export interface CaptureWorkspaceRuntime {
  readonly renderState: CaptureWorkspaceRenderState;
  readonly actions: CaptureWorkspaceRuntimeActions;
  subscribe(listener: () => void): () => void;
}

export interface CaptureWorkspaceRuntimePlatform {
  commands: CaptureWorkspaceCommandsPort;
  dismiss(): Promise<void>;
}
