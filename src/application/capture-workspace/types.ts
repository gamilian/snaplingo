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
  status: CaptureWorkspaceRuntimeStatus;
  mode: CaptureMode;
  session: CaptureSessionView | null;
  sessionId: string | null;
  cursorPoint: Point | null;
  selection: LogicalRect | null;
  hoverSelection: LogicalRect | null;
  isRenderingOutput: boolean;
  hasHydratedPixelSource: boolean;
  error: string | null;
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
  actions: CaptureWorkspaceRuntimeActions;
}

export interface CaptureWorkspaceRuntimePlatform {
  commands: CaptureWorkspaceCommandsPort;
  dismiss(): Promise<void>;
}
