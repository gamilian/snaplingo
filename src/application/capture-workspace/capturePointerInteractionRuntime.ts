import type { AnnotationSizeDirection } from './annotationStyle';
import { annotationSizeDirectionFromWheel } from './annotationStyle';
import {
  getCancelCapturePointerAction,
  isCancelCapturePointer,
  isPinCapturePointer,
  type CancelCapturePointerAction,
} from './captureActions';
import type { LogicalRect, Point } from '../../domain/capture';

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

  if (state.status === 'selecting' && state.hasSelectionBounds) {
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
