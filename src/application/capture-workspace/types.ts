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
import type {
  AnnotationHistory,
} from '../../views/CaptureWorkspace/annotationHistory';
import type {
  AnnotationGestureDraft,
  AnnotationStyle,
  AnnotationTool,
} from '../../views/CaptureWorkspace/annotationStyle';
import type {
  CaptureAnnotationMoveGesture,
} from '../../views/CaptureWorkspace/captureEditorRuntime';
import type {
  CaptureDraftSelectionMoveGesture,
  CaptureSelectionEditGesture,
} from '../../views/CaptureWorkspace/captureSelectionRuntime';
import type {
  ColorSample,
  ColorSampleFormat,
} from '../../views/CaptureWorkspace/colorSampler';
import type { SelectionHandle } from '../../views/CaptureWorkspace/selection';
import type {
  TextAnnotationDraft,
} from '../../views/CaptureWorkspace/textAnnotationDraft';
import type { CaptureCandidateDetectionMode } from '../../views/CaptureWorkspace/captureWorkspaceState';
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
  readonly candidateDetectionMode: CaptureCandidateDetectionMode;
  readonly previewImageBase64: string | null;
  readonly editGesture: CaptureSelectionEditGesture | null;
  readonly activeAnnotationTool: AnnotationTool | null;
  readonly annotationGesture: AnnotationGestureDraft | null;
  readonly draftAnnotation: AnnotationCommand | null;
  readonly selectedAnnotationIndex: number | null;
  readonly annotationMoveGesture: CaptureAnnotationMoveGesture | null;
  readonly draftSelectionMoveGesture: CaptureDraftSelectionMoveGesture | null;
  readonly textDraft: TextAnnotationDraft | null;
  readonly textDraftAnnotationIndex: number | null;
  readonly annotationStyle: AnnotationStyle;
  readonly textFontSize: number;
  readonly annotationHistory: AnnotationHistory;
  readonly isAnnotationToolbarVisible: boolean;
  readonly cursorColor: ColorSample | null;
  readonly colorSampleFormat: ColorSampleFormat;
  readonly isMagnifierRequested: boolean;
  readonly includeCapturedCursor: boolean;
  readonly isRenderingOutput: boolean;
  readonly silentOcrHint: {
    readonly status: 'loading' | 'success';
    readonly point: Point;
  } | null;
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
  pointerDown(input: Point | CaptureWorkspacePointerInput): boolean;
  pointerMove(input: Point | CaptureWorkspacePointerInput): boolean;
  pointerUp(input: Point | CaptureWorkspacePointerInput): Promise<boolean>;
  resizePointerDown(
    handle: SelectionHandle,
    input: Point | CaptureWorkspacePointerInput,
  ): boolean;
  resizeAnnotationPointerDown(
    handle: SelectionHandle,
    input: Point | CaptureWorkspacePointerInput,
  ): boolean;
  wheel(input: CaptureWorkspaceWheelInput): boolean;
  commitTextDraft(): void;
  updateTextDraftText(text: string): void;
  discardTextDraft(): void;
  selectMoveTool(): void;
  toggleAnnotationTool(tool: AnnotationTool): void;
  applySelectedAnnotationStyle(
    style: AnnotationStyle,
    textFontSize: number,
  ): void;
  updateTextDraftFontSize(fontSize: number): void;
  commitAnnotationSizeDefault(
    kind: 'stroke' | 'font',
    value: number,
  ): void;
  undoAnnotation(): void;
  redoAnnotation(): void;
  updateCursorColor(color: ColorSample | null): void;
  updatePolledCursor(point: Point): void;
  updatePolledHover(selection: LogicalRect | null): void;
  keyDown(event: CaptureWorkspaceKeyInput): boolean;
  hydrateSnapshots(): Promise<void>;
}

export interface CaptureWorkspacePointerInput {
  point: Point;
  button?: number;
  detail?: number;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
  source?: 'root' | 'preview';
}

export interface CaptureWorkspaceWheelInput {
  deltaY: number;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
}

export interface CaptureWorkspaceKeyInput {
  key: string;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
  repeat?: boolean;
}

export interface CaptureWorkspaceRuntime {
  readonly renderState: CaptureWorkspaceRenderState;
  readonly actions: CaptureWorkspaceRuntimeActions;
  subscribe(listener: () => void): () => void;
  dispose(): void;
}

export interface CaptureWorkspaceRuntimePlatform {
  commands: CaptureWorkspaceCommandsPort;
  clipboard: {
    copyText(text: string): Promise<void>;
  };
  onCancelRequested(handler: () => void | Promise<void>): Promise<() => void>;
  onCopyRequested(handler: () => void | Promise<void>): Promise<() => void>;
  onSaveRequested(handler: () => void | Promise<void>): Promise<() => void>;
  onUndoRequested(handler: () => void | Promise<void>): Promise<() => void>;
  onRedoRequested(handler: () => void | Promise<void>): Promise<() => void>;
  onHotkeyTriggered(
    handler: (launch: CaptureLaunch) => void | Promise<void>,
  ): Promise<() => void>;
  prepareForReveal(): Promise<void>;
  reveal(): Promise<void>;
  dismiss(): Promise<void>;
}
