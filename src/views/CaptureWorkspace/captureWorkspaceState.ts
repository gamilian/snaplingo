import {
  emptyAnnotationHistory,
  type AnnotationHistory,
} from './annotationHistory';
import {
  DEFAULT_ANNOTATION_STYLE,
  DEFAULT_TEXT_FONT_SIZE,
  type AnnotationGestureDraft,
  type AnnotationStyle,
  type AnnotationTool,
} from './annotationStyle';
import type {
  ColorSample,
  ColorSampleFormat,
} from '../../application/image-inspection/colorSampler';
import {
  createCapturePreviewResetState,
  type CaptureAnnotationMoveGesture,
} from './captureEditorRuntime';
import type { LoadedCaptureHostSession } from './captureHostRuntime';
import type {
  CaptureDraftSelectionMoveGesture,
  CaptureSelectionEditGesture,
} from './captureSelectionRuntime';
import type { TextAnnotationDraft } from './textAnnotationDraft';
import type {
  AnnotationCommand,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from './types';

export type CaptureSessionStatus =
  | 'idle'
  | 'loading'
  | 'selecting'
  | 'preview'
  | 'error';
export type CaptureCandidateDetectionMode = 'window' | 'control';

export interface CaptureWorkspaceState {
  status: CaptureSessionStatus;
  mode: CaptureMode;
  session: CaptureSessionView | null;
  startPoint: Point | null;
  cursorPoint: Point | null;
  selection: LogicalRect | null;
  hoverSelection: LogicalRect | null;
  candidateDetectionMode: CaptureCandidateDetectionMode;
  editGesture: CaptureSelectionEditGesture | null;
  activeAnnotationTool: AnnotationTool | null;
  annotationGesture: AnnotationGestureDraft | null;
  draftAnnotation: AnnotationCommand | null;
  selectedAnnotationIndex: number | null;
  annotationMoveGesture: CaptureAnnotationMoveGesture | null;
  draftSelectionMoveGesture: CaptureDraftSelectionMoveGesture | null;
  textDraft: TextAnnotationDraft | null;
  textDraftAnnotationIndex: number | null;
  annotationStyle: AnnotationStyle;
  textFontSize: number;
  annotationHistory: AnnotationHistory;
  previewImageBase64: string | null;
  isAnnotationToolbarVisible: boolean;
  cursorColor: ColorSample | null;
  colorSampleFormat: ColorSampleFormat;
  isMagnifierRequested: boolean;
  isRenderingOutput: boolean;
  silentOcrHint: { status: 'loading' | 'success'; point: Point } | null;
  includeCapturedCursor: boolean;
  error: string | null;
}

export function createInitialCaptureWorkspaceState(): CaptureWorkspaceState {
  return {
    status: 'idle',
    mode: 'screenshot',
    session: null,
    startPoint: null,
    cursorPoint: null,
    selection: null,
    hoverSelection: null,
    candidateDetectionMode: 'window',
    editGesture: null,
    activeAnnotationTool: null,
    annotationGesture: null,
    draftAnnotation: null,
    selectedAnnotationIndex: null,
    annotationMoveGesture: null,
    draftSelectionMoveGesture: null,
    textDraft: null,
    textDraftAnnotationIndex: null,
    annotationStyle: DEFAULT_ANNOTATION_STYLE,
    textFontSize: DEFAULT_TEXT_FONT_SIZE,
    annotationHistory: emptyAnnotationHistory(),
    previewImageBase64: null,
    isAnnotationToolbarVisible: true,
    cursorColor: null,
    colorSampleFormat: 'hex',
    isMagnifierRequested: false,
    isRenderingOutput: false,
    silentOcrHint: null,
    includeCapturedCursor: false,
    error: null,
  };
}

export function resetCaptureInteractionStatePatch(): Partial<CaptureWorkspaceState> {
  return {
    startPoint: null,
    cursorPoint: null,
    selection: null,
    hoverSelection: null,
    candidateDetectionMode: 'window',
    editGesture: null,
    activeAnnotationTool: null,
    annotationGesture: null,
    draftAnnotation: null,
    selectedAnnotationIndex: null,
    annotationMoveGesture: null,
    draftSelectionMoveGesture: null,
    textDraft: null,
    textDraftAnnotationIndex: null,
    annotationHistory: emptyAnnotationHistory(),
    previewImageBase64: null,
    isAnnotationToolbarVisible: true,
    cursorColor: null,
    colorSampleFormat: 'hex',
    isMagnifierRequested: false,
    isRenderingOutput: false,
    silentOcrHint: null,
    includeCapturedCursor: false,
    error: null,
  };
}

export function loadedCaptureHostSessionPatch(
  loaded: LoadedCaptureHostSession,
): Partial<CaptureWorkspaceState> {
  return {
    session: loaded.session,
    status: 'selecting',
    cursorPoint: loaded.cursorPoint,
    hoverSelection: loaded.hoverSelection,
  };
}

export function previewResetPatch(): Partial<CaptureWorkspaceState> {
  const { renderingOutput, ...resetState } = createCapturePreviewResetState();

  return {
    ...resetState,
    isRenderingOutput: renderingOutput,
  };
}
