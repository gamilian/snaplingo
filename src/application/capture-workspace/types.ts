import type {
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from '../../domain/capture';
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
  readonly isRenderingOutput: boolean;
  readonly hasHydratedPixelSource: boolean;
  readonly error: string | null;
}

export interface CaptureWorkspaceRuntimeActions {
  startSession(mode: CaptureMode, sessionId?: string): Promise<void>;
  pointerDown(point: Point): void;
  pointerMove(point: Point): void;
  pointerUp(point: Point): Promise<void>;
  keyDown(event: { key: string }): Promise<void>;
  hydrateSnapshots(): Promise<void>;
}

export interface CaptureWorkspaceRuntime {
  readonly renderState: CaptureWorkspaceRenderState;
  readonly actions: CaptureWorkspaceRuntimeActions;
}

export interface CaptureWorkspaceRuntimePlatform {
  commands: CaptureWorkspaceCommandsPort;
  dismiss(): Promise<void>;
}
