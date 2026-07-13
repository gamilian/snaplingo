import type {
  AnnotationColor,
  AnnotationGestureDraft,
  AnnotationSizeDirection,
  AnnotationStyle,
  AnnotationTool,
  DrawingAnnotationTool,
} from './annotationStyle';
import {
  appendAnnotationPoint,
  annotationFromGesture,
  annotationFromGestureDraft,
  applyAnnotationStyle,
  completeAnnotationGesture,
  isPointStrokeAnnotationTool,
  nextAnnotationStrokeWidth,
  nextTextFontSize,
} from './annotationStyle';
import type { AnnotationHistory } from './annotationHistory';
import {
  addAnnotationToHistory,
  emptyAnnotationHistory,
  replaceAnnotationInHistory,
} from './annotationHistory';
import type {
  CaptureRuntimeEffect,
  ManualSelectionCompletionPlan,
} from './captureInteractionRuntime';
import {
  constrainAnnotationMoveDelta,
  getAnnotationBounds,
  getAnnotationKeyboardNudgeDelta,
  hitTestAnnotations,
  moveAnnotationByDelta,
  resizeRectAnnotation,
} from './annotationGeometry';
import type { SelectionHandle } from './selection';
import type { AnnotationCommand, LogicalRect, Point } from './types';
import {
  commitTextAnnotationDraft,
  startTextAnnotationDraftFromAnnotation,
  type TextAnnotationDraft,
} from './textAnnotationDraft';

export interface CaptureEditorToolbarState {
  annotationStyle: AnnotationStyle;
  textFontSize: number;
}

export function deriveCaptureEditorToolbarState(
  currentState: CaptureEditorToolbarState,
  annotation: AnnotationCommand,
): CaptureEditorToolbarState {
  if (annotation.type === 'mosaic') {
    return {
      ...currentState,
      annotationStyle: {
        ...currentState.annotationStyle,
        strokeWidth: Math.max(1, Math.round(annotation.stroke_width / 5)),
      },
    };
  }

  if (annotation.type === 'eraser') {
    return {
      ...currentState,
      annotationStyle: {
        ...currentState.annotationStyle,
        strokeWidth: Math.max(1, Math.round(annotation.stroke_width / 4)),
      },
    };
  }

  if (annotation.type === 'text') {
    return {
      textFontSize: annotation.font_size,
      annotationStyle: {
        ...currentState.annotationStyle,
        color: annotation.color,
      },
    };
  }

  return {
    textFontSize: currentState.textFontSize,
    annotationStyle: {
      color: annotation.color,
      strokeWidth: annotation.stroke_width,
      filled:
        annotation.type === 'rectangle' || annotation.type === 'ellipse'
          ? annotation.filled
          : false,
    },
  };
}

export interface ApplyStyleToSelectedAnnotationHistoryOptions {
  annotationHistory: AnnotationHistory;
  annotations: AnnotationCommand[];
  selectedAnnotationIndex: number | null;
  textDraftActive: boolean;
  nextStyle: AnnotationStyle;
  nextTextFontSize: number;
}

export function applyStyleToSelectedAnnotationHistory({
  annotationHistory,
  annotations,
  selectedAnnotationIndex,
  textDraftActive,
  nextStyle,
  nextTextFontSize,
}: ApplyStyleToSelectedAnnotationHistoryOptions) {
  if (
    textDraftActive ||
    selectedAnnotationIndex === null ||
    !annotations[selectedAnnotationIndex]
  ) {
    return annotationHistory;
  }

  return replaceAnnotationInHistory(
    annotationHistory,
    selectedAnnotationIndex,
    applyAnnotationStyle(
      annotations[selectedAnnotationIndex],
      nextStyle,
      nextTextFontSize,
    ),
  );
}

export interface MoveSelectedAnnotationHistoryOptions {
  annotationHistory: AnnotationHistory;
  annotations: AnnotationCommand[];
  selectedAnnotationIndex: number | null;
  delta: Point;
}

export function moveSelectedAnnotationHistory({
  annotationHistory,
  annotations,
  selectedAnnotationIndex,
  delta,
}: MoveSelectedAnnotationHistoryOptions) {
  if (
    selectedAnnotationIndex === null ||
    !annotations[selectedAnnotationIndex]
  ) {
    return annotationHistory;
  }

  return replaceAnnotationInHistory(
    annotationHistory,
    selectedAnnotationIndex,
    moveAnnotationByDelta(annotations[selectedAnnotationIndex], delta),
  );
}

export interface CaptureSelectedAnnotationBoundsOptions {
  annotations: AnnotationCommand[];
  selectedAnnotationIndex: number | null;
  annotationMoveGesture: CaptureAnnotationMoveGesture | null;
}

export function getCaptureSelectedAnnotationBounds({
  annotationMoveGesture,
  annotations,
  selectedAnnotationIndex,
}: CaptureSelectedAnnotationBoundsOptions): LogicalRect | null {
  if (
    annotationMoveGesture ||
    selectedAnnotationIndex === null ||
    !annotations[selectedAnnotationIndex]
  ) {
    return null;
  }

  return getAnnotationBounds(annotations[selectedAnnotationIndex]);
}

export interface CaptureSelectedAnnotationKeyboardNudgePlan {
  annotationHistory: AnnotationHistory;
  previewAnnotations: AnnotationCommand[] | null;
}

export function planCaptureSelectedAnnotationKeyboardNudge({
  annotationHistory,
  annotations,
  selectedAnnotationIndex,
  key,
  fast,
  keyboardNudgeStep,
  keyboardFastNudgeStep,
}: {
  annotationHistory: AnnotationHistory;
  annotations: AnnotationCommand[];
  selectedAnnotationIndex: number | null;
  key: string;
  fast: boolean;
  keyboardNudgeStep: number;
  keyboardFastNudgeStep: number;
}): CaptureSelectedAnnotationKeyboardNudgePlan {
  const step = fast ? keyboardFastNudgeStep : keyboardNudgeStep;
  const delta = getAnnotationKeyboardNudgeDelta(key, step);
  if (!delta) {
    return {
      annotationHistory,
      previewAnnotations: null,
    };
  }

  const nextHistory = moveSelectedAnnotationHistory({
    annotationHistory,
    annotations,
    selectedAnnotationIndex,
    delta,
  });

  return {
    annotationHistory: nextHistory,
    previewAnnotations:
      nextHistory === annotationHistory ? null : nextHistory.annotations,
  };
}

export type CaptureEditorDismissAction =
  | 'clear-text-draft'
  | 'revert-annotation-move'
  | 'clear-draft-selection-move'
  | 'clear-selected-annotation'
  | 'clear-active-annotation-tool'
  | 'cancel-session';

export interface CaptureEditorDismissState {
  hasTextDraft: boolean;
  hasAnnotationMoveGesture: boolean;
  hasDraftSelectionMoveGesture: boolean;
  hasSelectedAnnotation: boolean;
  hasActiveAnnotationTool: boolean;
  hasAnnotationGesture: boolean;
}

export function getCaptureEditorDismissAction(
  state: CaptureEditorDismissState,
): CaptureEditorDismissAction {
  if (state.hasTextDraft) return 'clear-text-draft';
  if (state.hasAnnotationMoveGesture) return 'revert-annotation-move';
  if (state.hasDraftSelectionMoveGesture) return 'clear-draft-selection-move';
  if (state.hasSelectedAnnotation) return 'clear-selected-annotation';
  if (state.hasActiveAnnotationTool || state.hasAnnotationGesture) {
    return 'clear-active-annotation-tool';
  }

  return 'cancel-session';
}

export interface CaptureAnnotationToolActivationPlan {
  activeAnnotationTool: AnnotationTool | null;
  selectedAnnotationIndex: number | null;
  annotationGesture: null;
  annotationMoveGesture: null;
  draftAnnotation: null;
}

export function planCaptureAnnotationToolActivation({
  currentTool,
  nextTool,
  selectedAnnotationIndex,
  clearSelectedAnnotation,
  toggle,
}: {
  currentTool: AnnotationTool | null;
  nextTool: AnnotationTool;
  selectedAnnotationIndex: number | null;
  clearSelectedAnnotation: boolean;
  toggle: boolean;
}): CaptureAnnotationToolActivationPlan {
  return {
    activeAnnotationTool: toggle && currentTool === nextTool ? null : nextTool,
    selectedAnnotationIndex: clearSelectedAnnotation ? null : selectedAnnotationIndex,
    annotationGesture: null,
    annotationMoveGesture: null,
    draftAnnotation: null,
  };
}

export function planCaptureAnnotationSizeAdjustment({
  annotationStyle,
  textFontSize,
  direction,
  isTextSizingActive,
}: CaptureEditorToolbarState & {
  direction: AnnotationSizeDirection;
  isTextSizingActive: boolean;
}): CaptureEditorToolbarState {
  if (isTextSizingActive) {
    return {
      annotationStyle,
      textFontSize: nextTextFontSize(textFontSize, direction),
    };
  }

  return {
    annotationStyle: {
      ...annotationStyle,
      strokeWidth: nextAnnotationStrokeWidth(
        annotationStyle.strokeWidth,
        direction,
      ),
    },
    textFontSize,
  };
}

export function planCaptureAnnotationColorSelection({
  annotationStyle,
  textFontSize,
  color,
}: CaptureEditorToolbarState & {
  color: AnnotationColor;
}): CaptureEditorToolbarState {
  return {
    annotationStyle: {
      ...annotationStyle,
      color,
    },
    textFontSize,
  };
}

export function planCaptureAnnotationFillToggle({
  annotationStyle,
  textFontSize,
}: CaptureEditorToolbarState): CaptureEditorToolbarState {
  return {
    annotationStyle: {
      ...annotationStyle,
      filled: !annotationStyle.filled,
    },
    textFontSize,
  };
}

export interface CommitCaptureEditorTextDraftOptions {
  annotationHistory: AnnotationHistory;
  selectedAnnotationIndex: number | null;
  textDraft: TextAnnotationDraft | null;
  annotationStyle: AnnotationStyle;
  textDraftAnnotationIndex: number | null;
}

export interface CommitCaptureEditorTextDraftResult {
  annotationHistory: AnnotationHistory;
  selectedAnnotationIndex: number | null;
  textDraft: null;
  textDraftAnnotationIndex: null;
}

export function commitCaptureEditorTextDraft({
  annotationHistory,
  selectedAnnotationIndex,
  textDraft,
  annotationStyle,
  textDraftAnnotationIndex,
}: CommitCaptureEditorTextDraftOptions): CommitCaptureEditorTextDraftResult {
  if (!textDraft) {
    return {
      annotationHistory,
      selectedAnnotationIndex,
      textDraft: null,
      textDraftAnnotationIndex: null,
    };
  }

  const nextHistory = commitTextAnnotationDraft(
    annotationHistory,
    textDraft,
    annotationStyle,
    textDraftAnnotationIndex ?? undefined,
  );

  return {
    annotationHistory: nextHistory,
    selectedAnnotationIndex:
      nextHistory !== annotationHistory ? null : selectedAnnotationIndex,
    textDraft: null,
    textDraftAnnotationIndex: null,
  };
}

export interface CompleteCaptureEditorGestureOptions {
  annotationHistory: AnnotationHistory;
  selectedAnnotationIndex: number | null;
  annotationGesture: AnnotationGestureDraft | null;
  localPoint: Point;
  annotationStyle: AnnotationStyle;
  constrainGesture: boolean;
}

export interface CompleteCaptureEditorGestureResult {
  annotationHistory: AnnotationHistory;
  selectedAnnotationIndex: number | null;
  annotationGesture: null;
  draftAnnotation: null;
}

export function completeCaptureEditorGesture({
  annotationHistory,
  selectedAnnotationIndex,
  annotationGesture,
  localPoint,
  annotationStyle,
  constrainGesture,
}: CompleteCaptureEditorGestureOptions): CompleteCaptureEditorGestureResult | null {
  if (!annotationGesture) return null;

  const nextAnnotation = completeAnnotationGesture(
    annotationGesture,
    localPoint,
    annotationStyle,
    constrainGesture,
  );

  if (!nextAnnotation) {
    return {
      annotationHistory,
      selectedAnnotationIndex,
      annotationGesture: null,
      draftAnnotation: null,
    };
  }

  const nextHistory = addAnnotationToHistory(annotationHistory, nextAnnotation);
  const nextSelectedAnnotationIndex =
    nextAnnotation.type === 'rectangle' ||
    nextAnnotation.type === 'ellipse'
      ? nextHistory.annotations.length - 1
      : null;

  return {
    annotationHistory: nextHistory,
    selectedAnnotationIndex: nextSelectedAnnotationIndex,
    annotationGesture: null,
    draftAnnotation: null,
  };
}

export interface CapturePreviewResetState {
  startPoint: null;
  cursorPoint: null;
  selection: null;
  hoverSelection: null;
  editGesture: null;
  previewImageBase64: null;
  renderingOutput: false;
  activeAnnotationTool: null;
  annotationGesture: null;
  draftAnnotation: null;
  selectedAnnotationIndex: null;
  annotationMoveGesture: null;
  draftSelectionMoveGesture: null;
  textDraft: null;
  textDraftAnnotationIndex: null;
  annotationHistory: AnnotationHistory;
  isMagnifierRequested: false;
  status: 'selecting';
}

export function createCapturePreviewResetState(): CapturePreviewResetState {
  return {
    startPoint: null,
    cursorPoint: null,
    selection: null,
    hoverSelection: null,
    editGesture: null,
    previewImageBase64: null,
    renderingOutput: false,
    activeAnnotationTool: null,
    annotationGesture: null,
    draftAnnotation: null,
    selectedAnnotationIndex: null,
    annotationMoveGesture: null,
    draftSelectionMoveGesture: null,
    textDraft: null,
    textDraftAnnotationIndex: null,
    annotationHistory: emptyAnnotationHistory(),
    isMagnifierRequested: false,
    status: 'selecting',
  };
}

interface CaptureManualSelectionBaseState {
  startPoint: null;
  selection: LogicalRect;
  hoverSelection: null;
  editGesture: null;
  activeAnnotationTool: null;
  annotationGesture: null;
  draftAnnotation: null;
  selectedAnnotationIndex: null;
  annotationMoveGesture: null;
  draftSelectionMoveGesture: null;
  textDraft: null;
  textDraftAnnotationIndex: null;
  annotationHistory: AnnotationHistory;
  isMagnifierRequested: false;
}

interface CapturePreviewManualSelectionState
  extends CaptureManualSelectionBaseState {
  isAnnotationToolbarVisible: true;
  status: 'preview';
}

interface CaptureEffectsManualSelectionState
  extends CaptureManualSelectionBaseState {
  isAnnotationToolbarVisible: false;
  status: 'selecting';
  renderingOutput: true;
  error: null;
}

export type CaptureManualSelectionTransition =
  | {
      type: 'preview';
      clearOverlay: true;
      nextState: CapturePreviewManualSelectionState;
      previewRender: {
        rect: LogicalRect;
        annotations: AnnotationCommand[];
      };
    }
  | {
      type: 'effects';
      clearOverlay: true;
      nextState: CaptureEffectsManualSelectionState;
      effects: CaptureRuntimeEffect[];
    };

export function planCaptureManualSelectionTransition({
  rect,
  completion,
}: {
  rect: LogicalRect;
  completion: ManualSelectionCompletionPlan;
}): CaptureManualSelectionTransition {
  const baseState: CaptureManualSelectionBaseState = {
    startPoint: null,
    selection: rect,
    hoverSelection: null,
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
    isMagnifierRequested: false,
  };

  if (completion.type === 'preview') {
    return {
      type: 'preview',
      clearOverlay: true,
      nextState: {
        ...baseState,
        isAnnotationToolbarVisible: true,
        status: 'preview',
      },
      previewRender: {
        rect,
        annotations: [],
      },
    };
  }

  return {
    type: 'effects',
    clearOverlay: true,
    nextState: {
      ...baseState,
      isAnnotationToolbarVisible: false,
      status: 'selecting',
      renderingOutput: true,
      error: null,
    },
    effects: completion.effects,
  };
}

export interface CaptureAnnotationToolStartPlan {
  selectedAnnotationIndex: null;
  annotationGesture: AnnotationGestureDraft;
  draftAnnotation: AnnotationCommand;
}

export function planCaptureAnnotationToolStart({
  tool,
  localPoint,
  annotationStyle,
}: {
  tool: DrawingAnnotationTool;
  localPoint: Point;
  annotationStyle: AnnotationStyle;
}): CaptureAnnotationToolStartPlan {
  const points =
  isPointStrokeAnnotationTool(tool)
      ? [localPoint]
      : undefined;
  const annotationGesture: AnnotationGestureDraft = {
    tool,
    startPoint: localPoint,
    ...(points ? { points } : {}),
  };

  return {
    selectedAnnotationIndex: null,
    annotationGesture,
    draftAnnotation: annotationFromGesture(
      tool,
      localPoint,
      localPoint,
      annotationStyle,
      points,
    ),
  };
}

export interface CaptureAnnotationGestureMovePlan {
  annotationGesture: AnnotationGestureDraft;
  draftAnnotation: AnnotationCommand;
}

export function planCaptureAnnotationGestureMove({
  gesture,
  localPoint,
  annotationStyle,
  constrainGesture,
}: {
  gesture: AnnotationGestureDraft;
  localPoint: Point;
  annotationStyle: AnnotationStyle;
  constrainGesture: boolean;
}): CaptureAnnotationGestureMovePlan {
  const points = isPointStrokeAnnotationTool(gesture.tool)
    ? appendAnnotationPoint(gesture.points ?? [], localPoint)
    : undefined;
  const annotationGesture =
    points && !constrainGesture
      ? {
          ...gesture,
          points,
        }
      : gesture;

  return {
    annotationGesture,
    draftAnnotation: annotationFromGestureDraft(
      gesture,
      localPoint,
      annotationStyle,
      constrainGesture,
    ),
  };
}


export interface CaptureAnnotationMoveGesture {
  annotationIndex: number;
  startPoint: Point;
  startAnnotation: AnnotationCommand;
  resizeHandle?: SelectionHandle;
}

export type CaptureExistingAnnotationPointerDownPlan =
  | {
      type: 'edit-text-annotation';
      selectedAnnotationIndex: number;
      annotationMoveGesture: null;
      draftAnnotation: null;
      textDraft: TextAnnotationDraft;
      textDraftAnnotationIndex: number;
      toolbarState: CaptureEditorToolbarState;
    }
  | {
      type: 'move-annotation';
      selectedAnnotationIndex: number;
      annotationMoveGesture: CaptureAnnotationMoveGesture;
      toolbarState: CaptureEditorToolbarState;
    };

export function planCaptureExistingAnnotationPointerDown({
  annotations,
  localPoint,
  pointerDetail,
  toolbarState,
}: {
  annotations: AnnotationCommand[];
  localPoint: Point;
  pointerDetail: number;
  toolbarState: CaptureEditorToolbarState;
}): CaptureExistingAnnotationPointerDownPlan | null {
  const hitAnnotationIndex = hitTestAnnotations(annotations, localPoint);
  if (hitAnnotationIndex === null) return null;

  const hitAnnotation = annotations[hitAnnotationIndex];
  const nextToolbarState = deriveCaptureEditorToolbarState(
    toolbarState,
    hitAnnotation,
  );

  if (pointerDetail >= 2 && hitAnnotation.type === 'text') {
    return {
      type: 'edit-text-annotation',
      selectedAnnotationIndex: hitAnnotationIndex,
      annotationMoveGesture: null,
      draftAnnotation: null,
      textDraft: startTextAnnotationDraftFromAnnotation(hitAnnotation),
      textDraftAnnotationIndex: hitAnnotationIndex,
      toolbarState: nextToolbarState,
    };
  }

  return {
    type: 'move-annotation',
    selectedAnnotationIndex: hitAnnotationIndex,
    annotationMoveGesture: {
      annotationIndex: hitAnnotationIndex,
      startPoint: localPoint,
      startAnnotation: hitAnnotation,
    },
    toolbarState: nextToolbarState,
  };
}

export interface CaptureAnnotationMovePlan {
  draftAnnotation: AnnotationCommand;
}

export function planCaptureAnnotationMove({
  startAnnotation,
  startPoint,
  localPoint,
  constrainMove,
  resizeHandle,
  selectionBounds,
}: {
  startAnnotation: AnnotationCommand;
  startPoint: Point;
  localPoint: Point;
  constrainMove: boolean;
  resizeHandle?: SelectionHandle;
  selectionBounds?: LogicalRect;
}): CaptureAnnotationMovePlan {
  const delta = {
    x: localPoint.x - startPoint.x,
    y: localPoint.y - startPoint.y,
  };
  if (resizeHandle && selectionBounds) {
    return {
      draftAnnotation: resizeRectAnnotation(
        startAnnotation,
        resizeHandle,
        delta,
        selectionBounds,
        constrainMove,
      ),
    };
  }

  const moveDelta = constrainMove ? constrainAnnotationMoveDelta(delta) : delta;

  return {
    draftAnnotation: moveAnnotationByDelta(startAnnotation, moveDelta),
  };
}

export interface CaptureAnnotationMoveCommitPlan {
  annotationMoveGesture: null;
  draftAnnotation: null;
  annotationHistory: AnnotationHistory;
  selectedAnnotationIndex?: number;
}

export function planCaptureAnnotationMoveCommit({
  annotationHistory,
  annotationIndex,
  startAnnotation,
  startPoint,
  localPoint,
  constrainMove,
  resizeHandle,
  selectionBounds,
}: {
  annotationHistory: AnnotationHistory;
  annotationIndex: number;
  startAnnotation: AnnotationCommand;
  startPoint: Point;
  localPoint: Point;
  constrainMove: boolean;
  resizeHandle?: SelectionHandle;
  selectionBounds?: LogicalRect;
}): CaptureAnnotationMoveCommitPlan {
  const annotationMove = planCaptureAnnotationMove({
    startAnnotation,
    startPoint,
    localPoint,
    constrainMove,
    resizeHandle,
    selectionBounds,
  });
  const nextHistory = replaceAnnotationInHistory(
    annotationHistory,
    annotationIndex,
    annotationMove.draftAnnotation,
  );

  return {
    annotationMoveGesture: null,
    draftAnnotation: null,
    annotationHistory: nextHistory,
    selectedAnnotationIndex:
      nextHistory === annotationHistory ? undefined : annotationIndex,
  };
}
