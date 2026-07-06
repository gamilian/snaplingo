import type { AnnotationHistory } from './annotationHistory';
import { emptyAnnotationHistory } from './annotationHistory';
import type { CaptureCandidate } from './captureCandidates';
import {
  getCandidateForPointerReleaseCompletion,
  getNextCandidateAtPoint,
} from './captureCandidates';
import type { SelectionArrowAction } from './captureActions';
import {
  constrainSelectionPoint,
  moveDraftSelectionByDelta,
  moveSelectionByDelta,
  normalizeSelection,
  nudgeDraftSelection,
  nudgeMovedSelection,
  nudgeResizedSelection,
  nudgeSelection,
  resizeSelectionByHandle,
  resizeSelectionBoundaryByArrow,
  snapMovedSelectionToRects,
  snapPointToRects,
  snapResizedSelectionToRects,
  type SelectionHandle,
} from './selection';
import type { AnnotationCommand, LogicalRect, Point } from './types';
import { nudgeVirtualPoint } from './virtualDesktop';

export interface CaptureDraftSelectionStartState {
  cursorPoint: Point;
  startPoint: Point;
  selection: null;
  hoverSelection: null;
  previewImageBase64: null;
  renderingOutput: false;
  status: 'selecting';
  activeAnnotationTool: null;
  annotationGesture: null;
  draftAnnotation: null;
  selectedAnnotationIndex: null;
  annotationMoveGesture: null;
  draftSelectionMoveGesture: null;
  textDraft: null;
  textDraftAnnotationIndex: null;
  annotationHistory: AnnotationHistory;
}

export interface CaptureDraftSelectionStartPlan {
  cursorPoint: Point;
  draftSelection: LogicalRect;
  nextState: CaptureDraftSelectionStartState;
}

export function planCaptureDraftSelectionStart({
  cursorPoint,
  anchorPoint,
}: {
  cursorPoint: Point;
  anchorPoint: Point;
}): CaptureDraftSelectionStartPlan {
  return {
    cursorPoint,
    draftSelection: {
      x: anchorPoint.x,
      y: anchorPoint.y,
      width: 0,
      height: 0,
    },
    nextState: {
      cursorPoint,
      startPoint: anchorPoint,
      selection: null,
      hoverSelection: null,
      previewImageBase64: null,
      renderingOutput: false,
      status: 'selecting',
      activeAnnotationTool: null,
      annotationGesture: null,
      draftAnnotation: null,
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      draftSelectionMoveGesture: null,
      textDraft: null,
      textDraftAnnotationIndex: null,
      annotationHistory: emptyAnnotationHistory(),
    },
  };
}

export interface CaptureDraftSelectionMoveGesture {
  startPoint: Point;
  startSelection: LogicalRect;
  startAnchorPoint: Point;
}

export interface CaptureDraftSelectionMoveShortcutStartPlan {
  draftSelectionMoveGesture: CaptureDraftSelectionMoveGesture;
}

export function planCaptureDraftSelectionMoveShortcutStart({
  cursorPoint,
  selection,
  anchorPoint,
}: {
  cursorPoint: Point;
  selection: LogicalRect;
  anchorPoint: Point;
}): CaptureDraftSelectionMoveShortcutStartPlan {
  return {
    draftSelectionMoveGesture: {
      startPoint: cursorPoint,
      startSelection: selection,
      startAnchorPoint: anchorPoint,
    },
  };
}

export interface CaptureDraftSelectionMovePlan {
  draftSelection: LogicalRect;
  anchorPoint: Point;
  previewImageBase64: null;
  renderingOutput: false;
}

export function planCaptureDraftSelectionMove({
  gesture,
  point,
  selectionBounds,
}: {
  gesture: CaptureDraftSelectionMoveGesture;
  point: Point;
  selectionBounds: LogicalRect;
}): CaptureDraftSelectionMovePlan {
  const result = moveDraftSelectionByDelta(
    gesture.startSelection,
    gesture.startAnchorPoint,
    {
      x: point.x - gesture.startPoint.x,
      y: point.y - gesture.startPoint.y,
    },
    selectionBounds,
  );

  return {
    draftSelection: result.selection,
    anchorPoint: result.anchorPoint,
    previewImageBase64: null,
    renderingOutput: false,
  };
}

export interface CaptureDraftSelectionPointerMovePlan {
  keyboardDraftCursorPoint: null;
  draftSelection: LogicalRect;
}

export function planCaptureDraftSelectionPointerMove({
  anchorPoint,
  point,
  snapTargetRects,
  edgeSnapThreshold,
  constrainSelection,
}: {
  anchorPoint: Point;
  point: Point;
  snapTargetRects: LogicalRect[];
  edgeSnapThreshold: number;
  constrainSelection: boolean;
}): CaptureDraftSelectionPointerMovePlan {
  const currentPoint = snapPointToRects(point, snapTargetRects, edgeSnapThreshold);
  const constrainedPoint = constrainSelection
    ? constrainSelectionPoint(anchorPoint, currentPoint)
    : currentPoint;

  return {
    keyboardDraftCursorPoint: null,
    draftSelection: normalizeSelection(anchorPoint, constrainedPoint),
  };
}

export interface CaptureDraftSelectionKeyboardNudgePlan {
  keyboardDraftCursorPoint: Point;
  cursorPoint: Point;
  selection: LogicalRect;
  previewImageBase64: null;
  renderingOutput: false;
}

export function planCaptureDraftSelectionKeyboardNudge({
  anchorPoint,
  cursorPoint,
  delta,
  selectionBounds,
}: {
  anchorPoint: Point;
  cursorPoint: Point;
  delta: Point;
  selectionBounds: LogicalRect;
}): CaptureDraftSelectionKeyboardNudgePlan {
  const result = nudgeDraftSelection(
    anchorPoint,
    cursorPoint,
    delta,
    selectionBounds,
  );

  return {
    keyboardDraftCursorPoint: result.cursorPoint,
    cursorPoint: result.cursorPoint,
    selection: result.selection,
    previewImageBase64: null,
    renderingOutput: false,
  };
}

interface CaptureDraftSelectionCommitBasePlan {
  startPoint: null;
  draftSelection: null;
  overlayHoverSelection: LogicalRect | null;
}

export type CaptureDraftSelectionCommitPlan =
  | (CaptureDraftSelectionCommitBasePlan & {
      type: 'complete-selection';
      selection: LogicalRect;
    })
  | (CaptureDraftSelectionCommitBasePlan & {
      type: 'clear-selection';
      overlayHoverSelection: null;
      selection: null;
    });

export function planCaptureDraftSelectionCommit({
  anchorPoint,
  releasePoint,
  snapTargetRects,
  edgeSnapThreshold,
  constrainSelection,
  captureCandidates,
  activeHoverSelection,
  minSelectionSize,
}: {
  anchorPoint: Point;
  releasePoint: Point;
  snapTargetRects: LogicalRect[];
  edgeSnapThreshold: number;
  constrainSelection: boolean;
  captureCandidates: CaptureCandidate[];
  activeHoverSelection: LogicalRect | null;
  minSelectionSize: number;
}): CaptureDraftSelectionCommitPlan {
  const currentPoint = snapPointToRects(
    releasePoint,
    snapTargetRects,
    edgeSnapThreshold,
  );
  const nextSelection = normalizeSelection(
    anchorPoint,
    constrainSelection
      ? constrainSelectionPoint(anchorPoint, currentPoint)
      : currentPoint,
  );
  const candidateForPointerCompletion =
    getCandidateForPointerReleaseCompletion(
      captureCandidates,
      releasePoint,
      activeHoverSelection,
      nextSelection,
      minSelectionSize,
    )?.rect ?? null;

  if (
    nextSelection.width >= minSelectionSize &&
    nextSelection.height >= minSelectionSize
  ) {
    return {
      type: 'complete-selection',
      startPoint: null,
      draftSelection: null,
      overlayHoverSelection: activeHoverSelection,
      selection: nextSelection,
    };
  }

  if (candidateForPointerCompletion) {
    return {
      type: 'complete-selection',
      startPoint: null,
      draftSelection: null,
      overlayHoverSelection: activeHoverSelection,
      selection: candidateForPointerCompletion,
    };
  }

  return {
    type: 'clear-selection',
    startPoint: null,
    draftSelection: null,
    overlayHoverSelection: null,
    selection: null,
  };
}

export type CaptureSelectionEditGesture =
  | {
      type: 'move';
      startPoint: Point;
      startSelection: LogicalRect;
    }
  | {
      type: 'resize';
      handle: SelectionHandle;
      startPoint: Point;
      startSelection: LogicalRect;
    };

export type CapturePreviewSelectionMoveStartPlan =
  | {
      type: 'copy-selection';
    }
  | {
      type: 'move-selection';
      selectedAnnotationIndex: null;
      annotationMoveGesture: null;
      editGesture: CaptureSelectionEditGesture;
      previewImageBase64: null;
    };

export function planCapturePreviewSelectionMoveStart({
  point,
  selection,
  hasTextDraft,
  isCopyDoubleClick,
}: {
  point: Point;
  selection: LogicalRect;
  hasTextDraft: boolean;
  isCopyDoubleClick: boolean;
}): CapturePreviewSelectionMoveStartPlan {
  if (!hasTextDraft && isCopyDoubleClick) {
    return {
      type: 'copy-selection',
    };
  }

  return {
    type: 'move-selection',
    selectedAnnotationIndex: null,
    annotationMoveGesture: null,
    editGesture: {
      type: 'move',
      startPoint: point,
      startSelection: selection,
    },
    previewImageBase64: null,
  };
}

export interface CaptureSelectionResizeStartPlan {
  cursorPoint: Point;
  editGesture: CaptureSelectionEditGesture;
  previewImageBase64: null;
}

export function planCaptureSelectionResizeStart({
  point,
  selection,
  handle,
}: {
  point: Point;
  selection: LogicalRect;
  handle: SelectionHandle;
}): CaptureSelectionResizeStartPlan {
  return {
    cursorPoint: point,
    editGesture: {
      type: 'resize',
      handle,
      startPoint: point,
      startSelection: selection,
    },
    previewImageBase64: null,
  };
}

export interface CaptureSelectionEditKeyboardNudgePlan {
  keyboardEditCursorPoint: Point;
  cursorPoint: Point;
  selection: LogicalRect;
  editGesture: CaptureSelectionEditGesture;
  previewImageBase64: null;
  renderingOutput: false;
}

export function planCaptureSelectionEditKeyboardNudge({
  gesture,
  selection,
  cursorPoint,
  delta,
  selectionBounds,
  minSelectionSize,
  preserveAspect = false,
}: {
  gesture: CaptureSelectionEditGesture;
  selection: LogicalRect;
  cursorPoint: Point;
  delta: Point;
  selectionBounds: LogicalRect;
  minSelectionSize: number;
  preserveAspect?: boolean;
}): CaptureSelectionEditKeyboardNudgePlan {
  const result =
    gesture.type === 'move'
      ? nudgeMovedSelection(selection, cursorPoint, delta, selectionBounds)
      : nudgeResizedSelection(
          selection,
          cursorPoint,
          gesture.handle,
          delta,
          selectionBounds,
          minSelectionSize,
          preserveAspect,
        );

  return {
    keyboardEditCursorPoint: result.cursorPoint,
    cursorPoint: result.cursorPoint,
    selection: result.selection,
    editGesture: {
      ...gesture,
      startPoint: result.cursorPoint,
      startSelection: result.selection,
    },
    previewImageBase64: null,
    renderingOutput: false,
  };
}

export interface CaptureSelectionCursorKeyboardNudgePlan {
  cursorPoint: Point;
}

export function planCaptureSelectionCursorKeyboardNudge({
  cursorPoint,
  delta,
  selectionBounds,
}: {
  cursorPoint: Point;
  delta: Point;
  selectionBounds: LogicalRect;
}): CaptureSelectionCursorKeyboardNudgePlan {
  return {
    cursorPoint: nudgeVirtualPoint(cursorPoint, delta, selectionBounds),
  };
}

export interface CaptureHoverSelectionCyclePlan {
  hoverSelection: LogicalRect | null;
}

export function planCaptureHoverSelectionCycle({
  captureCandidates,
  cursorPoint,
  hoverSelection,
  direction,
}: {
  captureCandidates: CaptureCandidate[];
  cursorPoint: Point;
  hoverSelection: LogicalRect | null;
  direction: 1 | -1;
}): CaptureHoverSelectionCyclePlan {
  return {
    hoverSelection:
      getNextCandidateAtPoint(
        captureCandidates,
        cursorPoint,
        hoverSelection,
        direction,
      )?.rect ?? null,
  };
}

export interface CaptureSelectionArrowPreviewPlan {
  selection: LogicalRect;
  previewImageBase64: null;
  previewRender: {
    rect: LogicalRect;
  };
}

export function planCaptureSelectionArrowPreview({
  selection,
  selectionBounds,
  selectionArrowAction,
  minSelectionSize,
  keyboardNudgeStep,
}: {
  selection: LogicalRect;
  selectionBounds: LogicalRect;
  selectionArrowAction: SelectionArrowAction;
  minSelectionSize: number;
  keyboardNudgeStep: number;
}): CaptureSelectionArrowPreviewPlan {
  const nextSelection =
    selectionArrowAction.mode === 'expand'
      ? resizeSelectionBoundaryByArrow(
          selection,
          selectionArrowAction.direction,
          'expand',
          selectionBounds,
          minSelectionSize,
        )
      : selectionArrowAction.mode === 'shrink'
        ? resizeSelectionBoundaryByArrow(
            selection,
            selectionArrowAction.direction,
            'shrink',
            selectionBounds,
            minSelectionSize,
          )
        : nudgeSelection(
            selection,
            selectionArrowAction.direction,
            selectionBounds,
            keyboardNudgeStep,
          );

  return {
    selection: nextSelection,
    previewImageBase64: null,
    previewRender: {
      rect: nextSelection,
    },
  };
}

export function applyCaptureSelectionEditGesture({
  gesture,
  point,
  selectionBounds,
  snapTargetRects,
  edgeSnapThreshold,
  minSelectionSize,
  preserveAspect = false,
}: {
  gesture: CaptureSelectionEditGesture;
  point: Point;
  selectionBounds: LogicalRect;
  snapTargetRects: LogicalRect[];
  edgeSnapThreshold: number;
  minSelectionSize: number;
  preserveAspect?: boolean;
}): LogicalRect {
  const delta = {
    x: point.x - gesture.startPoint.x,
    y: point.y - gesture.startPoint.y,
  };

  if (gesture.type === 'move') {
    const movedSelection = moveSelectionByDelta(
      gesture.startSelection,
      delta,
      selectionBounds,
    );

    return snapMovedSelectionToRects(
      movedSelection,
      snapTargetRects,
      selectionBounds,
      edgeSnapThreshold,
    );
  }

  const shouldPreserveAspect = preserveAspect && gesture.handle.length === 2;
  const resizedSelection = resizeSelectionByHandle(
    gesture.startSelection,
    gesture.handle,
    delta,
    selectionBounds,
    minSelectionSize,
    shouldPreserveAspect,
  );
  if (shouldPreserveAspect) return resizedSelection;

  return snapResizedSelectionToRects(
    resizedSelection,
    gesture.handle,
    snapTargetRects,
    selectionBounds,
    minSelectionSize,
    edgeSnapThreshold,
  );
}

export interface CaptureSelectionEditPlanOptions {
  gesture: CaptureSelectionEditGesture;
  point: Point;
  selectionBounds: LogicalRect;
  snapTargetRects: LogicalRect[];
  edgeSnapThreshold: number;
  minSelectionSize: number;
  preserveAspect?: boolean;
}

export interface CaptureSelectionEditMovePlan {
  keyboardEditCursorPoint: null;
  selection: LogicalRect;
  previewImageBase64: null;
  renderingOutput: false;
}

export function planCaptureSelectionEditMove(
  options: CaptureSelectionEditPlanOptions,
): CaptureSelectionEditMovePlan {
  return {
    keyboardEditCursorPoint: null,
    selection: applyCaptureSelectionEditGesture(options),
    previewImageBase64: null,
    renderingOutput: false,
  };
}

export interface CaptureSelectionEditCommitPlan {
  editGesture: null;
  selection: LogicalRect;
  status: 'preview';
  previewRender: {
    rect: LogicalRect;
    annotations: AnnotationCommand[];
  };
}

export function planCaptureSelectionEditCommit({
  annotations,
  ...options
}: CaptureSelectionEditPlanOptions & {
  annotations: AnnotationCommand[];
}): CaptureSelectionEditCommitPlan {
  const nextSelection = applyCaptureSelectionEditGesture(options);

  return {
    editGesture: null,
    selection: nextSelection,
    status: 'preview',
    previewRender: {
      rect: nextSelection,
      annotations,
    },
  };
}
