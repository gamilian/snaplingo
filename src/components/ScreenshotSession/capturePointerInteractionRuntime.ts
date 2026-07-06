import type { AnnotationSizeDirection } from './annotationStyle';
import { annotationSizeDirectionFromWheel } from './annotationStyle';
import {
  getCancelCapturePointerAction,
  isCancelCapturePointer,
  isPinCapturePointer,
  type CancelCapturePointerAction,
} from './captureActions';
import type { LogicalRect, Point } from './types';

type CapturePointerStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';

interface CapturePointerEvent {
  button: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

interface CaptureWheelEvent {
  deltaY: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
}

export type CaptureRootPointerDownPlan =
  | {
      type: 'cancel-pointer';
      action: CancelCapturePointerAction;
    }
  | {
      type: 'start-draft-selection';
    }
  | {
      type: 'ignore';
    };

export interface CaptureRootPointerDownState {
  status: CapturePointerStatus;
  hasSelectionBounds: boolean;
  hasSelection: boolean;
  hasTextDraft: boolean;
  hasAnnotationGesture: boolean;
  hasDismissibleLayer: boolean;
}

export interface CapturePointerWheelState {
  status: CapturePointerStatus;
  hasTextDraft: boolean;
  hasAnnotationGesture: boolean;
  hasAnnotationMoveGesture: boolean;
  hasAnnotationEditingContext: boolean;
}

export type CapturePointerMoveAction =
  | 'move-annotation-gesture'
  | 'move-annotation'
  | 'move-draft-selection'
  | 'edit-selection'
  | 'update-draft-selection'
  | 'ignore';

export type CapturePointerUpAction =
  | 'commit-annotation-gesture'
  | 'commit-annotation-move'
  | 'commit-selection-edit'
  | 'commit-draft-selection'
  | 'ignore';

export interface CapturePointerMoveState {
  status: CapturePointerStatus;
  hasSelection: boolean;
  hasActiveStartPoint: boolean;
  hasEditGesture: boolean;
  hasAnnotationGesture: boolean;
  hasAnnotationMoveGesture: boolean;
  hasDraftSelectionMoveGesture: boolean;
}

export interface CapturePointerUpState {
  status: CapturePointerStatus;
  hasSelection: boolean;
  hasActiveStartPoint: boolean;
  hasEditGesture: boolean;
  hasAnnotationGesture: boolean;
  hasAnnotationMoveGesture: boolean;
}

export type CapturePreviewPointerDownPlan =
  | {
      type: 'pin-selection';
    }
  | {
      type: 'start-preview-interaction';
    }
  | {
      type: 'ignore';
    };

export interface CapturePreviewPointerDownState {
  status: CapturePointerStatus;
  hasSelection: boolean;
  hasSelectionBounds: boolean;
}

export function planCaptureRootPointerDown(
  event: CapturePointerEvent,
  state: CaptureRootPointerDownState,
): CaptureRootPointerDownPlan {
  if (isCancelCapturePointer(event)) {
    return {
      type: 'cancel-pointer',
      action: getCancelCapturePointerAction({
        status: state.status,
        hasSelection: state.hasSelection,
        hasTextDraft: state.hasTextDraft,
        hasAnnotationGesture: state.hasAnnotationGesture,
        hasDismissibleLayer: state.hasDismissibleLayer,
      }),
    };
  }

  if (
    (state.status === 'selecting' || state.status === 'preview') &&
    state.hasSelectionBounds
  ) {
    return {
      type: 'start-draft-selection',
    };
  }

  return {
    type: 'ignore',
  };
}

export function planCapturePreviewPointerDown(
  event: CapturePointerEvent,
  state: CapturePreviewPointerDownState,
): CapturePreviewPointerDownPlan {
  if (
    state.status !== 'preview' ||
    !state.hasSelection ||
    !state.hasSelectionBounds
  ) {
    return {
      type: 'ignore',
    };
  }

  if (isPinCapturePointer(event)) {
    return {
      type: 'pin-selection',
    };
  }

  return {
    type: 'start-preview-interaction',
  };
}

export function shouldSyncHoverSelectionOnPointerMove({
  hasActiveStartPoint,
  hasEditGesture,
  status,
}: Pick<
  CapturePointerMoveState,
  'hasActiveStartPoint' | 'hasEditGesture' | 'status'
>) {
  return !hasActiveStartPoint && !hasEditGesture && status === 'selecting';
}

export function getCapturePointerMoveAction({
  hasActiveStartPoint,
  hasAnnotationGesture,
  hasAnnotationMoveGesture,
  hasDraftSelectionMoveGesture,
  hasEditGesture,
  hasSelection,
  status,
}: CapturePointerMoveState): CapturePointerMoveAction {
  if (hasAnnotationGesture && hasSelection) return 'move-annotation-gesture';
  if (hasAnnotationMoveGesture && hasSelection) return 'move-annotation';
  if (hasDraftSelectionMoveGesture && status === 'selecting') {
    return 'move-draft-selection';
  }
  if (hasEditGesture) return 'edit-selection';
  if (hasActiveStartPoint && status === 'selecting') {
    return 'update-draft-selection';
  }

  return 'ignore';
}

export function getCapturePointerUpAction({
  hasActiveStartPoint,
  hasAnnotationGesture,
  hasAnnotationMoveGesture,
  hasEditGesture,
  hasSelection,
  status,
}: CapturePointerUpState): CapturePointerUpAction {
  if (hasAnnotationGesture && hasSelection) return 'commit-annotation-gesture';
  if (hasAnnotationMoveGesture && hasSelection) return 'commit-annotation-move';
  if (hasEditGesture) return 'commit-selection-edit';
  if (hasActiveStartPoint && status === 'selecting') {
    return 'commit-draft-selection';
  }

  return 'ignore';
}

export function getCaptureSelectionLocalPoint(
  point: Point,
  selection: LogicalRect,
): Point {
  return {
    x: clamp(point.x - selection.x, 0, selection.width),
    y: clamp(point.y - selection.y, 0, selection.height),
  };
}

export function planCapturePointerWheelSizeAdjustment(
  event: CaptureWheelEvent,
  state: CapturePointerWheelState,
): AnnotationSizeDirection | null {
  if (
    state.status !== 'preview' ||
    state.hasTextDraft ||
    state.hasAnnotationGesture ||
    state.hasAnnotationMoveGesture
  ) {
    return null;
  }

  return annotationSizeDirectionFromWheel(event, {
    editing: state.hasAnnotationEditingContext,
  });
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}
