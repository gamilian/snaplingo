import { describe, expect, it } from 'vitest';
import {
  getCaptureSelectionLocalPoint,
  getCapturePointerMoveAction,
  getCapturePointerUpAction,
  planCapturePointerWheelSizeAdjustment,
  planCapturePreviewPointerDown,
  planCaptureRootPointerDown,
  shouldSyncHoverSelectionOnPointerMove,
} from './capturePointerInteractionRuntime';

const primaryPointer = {
  button: 0,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
};

const secondaryPointer = {
  ...primaryPointer,
  button: 2,
};

const middlePointer = {
  ...primaryPointer,
  button: 1,
};

describe('capturePointerInteractionRuntime', () => {
  it('plans secondary-button pointer down as the current cancel-layer action', () => {
    expect(
      planCaptureRootPointerDown(secondaryPointer, {
        status: 'preview',
        hasSelectionBounds: true,
        hasSelection: true,
        hasTextDraft: true,
        hasAnnotationGesture: false,
        hasDismissibleLayer: true,
      }),
    ).toEqual({
      type: 'cancel-pointer',
      action: 'finish-edit',
    });

    expect(
      planCaptureRootPointerDown(secondaryPointer, {
        status: 'preview',
        hasSelectionBounds: true,
        hasSelection: true,
        hasTextDraft: false,
        hasAnnotationGesture: false,
        hasDismissibleLayer: false,
      }),
    ).toEqual({
      type: 'cancel-pointer',
      action: 'reset-selection',
    });
  });

  it('plans primary pointer down as draft selection only while capture interaction is active', () => {
    expect(
      planCaptureRootPointerDown(primaryPointer, {
        status: 'selecting',
        hasSelectionBounds: true,
        hasSelection: false,
        hasTextDraft: false,
        hasAnnotationGesture: false,
        hasDismissibleLayer: false,
      }),
    ).toEqual({
      type: 'start-draft-selection',
    });

    expect(
      planCaptureRootPointerDown(primaryPointer, {
        status: 'idle',
        hasSelectionBounds: true,
        hasSelection: false,
        hasTextDraft: false,
        hasAnnotationGesture: false,
        hasDismissibleLayer: false,
      }),
    ).toEqual({
      type: 'ignore',
    });
  });

  it('plans preview selection pointer down as pin, interaction, or ignore', () => {
    expect(
      planCapturePreviewPointerDown(middlePointer, {
        status: 'preview',
        hasSelection: true,
        hasSelectionBounds: true,
      }),
    ).toEqual({
      type: 'pin-selection',
    });

    expect(
      planCapturePreviewPointerDown(primaryPointer, {
        status: 'preview',
        hasSelection: true,
        hasSelectionBounds: true,
      }),
    ).toEqual({
      type: 'start-preview-interaction',
    });

    expect(
      planCapturePreviewPointerDown(primaryPointer, {
        status: 'selecting',
        hasSelection: true,
        hasSelectionBounds: true,
      }),
    ).toEqual({
      type: 'ignore',
    });
  });

  it('converts a virtual point into a selection-local clamped point', () => {
    expect(
      getCaptureSelectionLocalPoint(
        { x: 120, y: 40 },
        { x: 100, y: 50, width: 80, height: 60 },
      ),
    ).toEqual({ x: 20, y: 0 });

    expect(
      getCaptureSelectionLocalPoint(
        { x: 250, y: 140 },
        { x: 100, y: 50, width: 80, height: 60 },
      ),
    ).toEqual({ x: 80, y: 60 });
  });

  it('plans pointer move interaction by active capture gesture priority', () => {
    expect(
      getCapturePointerMoveAction({
        status: 'preview',
        hasSelection: true,
        hasActiveStartPoint: true,
        hasEditGesture: true,
        hasAnnotationGesture: true,
        hasAnnotationMoveGesture: true,
        hasDraftSelectionMoveGesture: true,
      }),
    ).toBe('move-annotation-gesture');

    expect(
      getCapturePointerMoveAction({
        status: 'selecting',
        hasSelection: false,
        hasActiveStartPoint: true,
        hasEditGesture: false,
        hasAnnotationGesture: false,
        hasAnnotationMoveGesture: false,
        hasDraftSelectionMoveGesture: true,
      }),
    ).toBe('move-draft-selection');

    expect(
      getCapturePointerMoveAction({
        status: 'preview',
        hasSelection: true,
        hasActiveStartPoint: true,
        hasEditGesture: false,
        hasAnnotationGesture: false,
        hasAnnotationMoveGesture: false,
        hasDraftSelectionMoveGesture: false,
      }),
    ).toBe('ignore');
  });

  it('syncs hover selection only when selecting without an active gesture', () => {
    expect(
      shouldSyncHoverSelectionOnPointerMove({
        status: 'selecting',
        hasActiveStartPoint: false,
        hasEditGesture: false,
      }),
    ).toBe(true);

    expect(
      shouldSyncHoverSelectionOnPointerMove({
        status: 'selecting',
        hasActiveStartPoint: true,
        hasEditGesture: false,
      }),
    ).toBe(false);

    expect(
      shouldSyncHoverSelectionOnPointerMove({
        status: 'preview',
        hasActiveStartPoint: false,
        hasEditGesture: false,
      }),
    ).toBe(false);
  });

  it('plans pointer up interaction by active capture gesture priority', () => {
    expect(
      getCapturePointerUpAction({
        status: 'preview',
        hasSelection: true,
        hasActiveStartPoint: true,
        hasEditGesture: true,
        hasAnnotationGesture: true,
        hasAnnotationMoveGesture: true,
      }),
    ).toBe('commit-annotation-gesture');

    expect(
      getCapturePointerUpAction({
        status: 'preview',
        hasSelection: true,
        hasActiveStartPoint: false,
        hasEditGesture: true,
        hasAnnotationGesture: false,
        hasAnnotationMoveGesture: false,
      }),
    ).toBe('commit-selection-edit');

    expect(
      getCapturePointerUpAction({
        status: 'selecting',
        hasSelection: false,
        hasActiveStartPoint: true,
        hasEditGesture: false,
        hasAnnotationGesture: false,
        hasAnnotationMoveGesture: false,
      }),
    ).toBe('commit-draft-selection');

    expect(
      getCapturePointerUpAction({
        status: 'preview',
        hasSelection: false,
        hasActiveStartPoint: true,
        hasEditGesture: false,
        hasAnnotationGesture: false,
        hasAnnotationMoveGesture: false,
      }),
    ).toBe('ignore');
  });

  it('plans wheel-driven annotation size adjustment only for editable preview state', () => {
    expect(
      planCapturePointerWheelSizeAdjustment(
        {
          deltaY: -1,
          metaKey: false,
          ctrlKey: false,
          altKey: false,
        },
        {
          status: 'preview',
          hasTextDraft: false,
          hasAnnotationGesture: false,
          hasAnnotationMoveGesture: false,
          hasAnnotationEditingContext: true,
        },
      ),
    ).toBe('increase');

    expect(
      planCapturePointerWheelSizeAdjustment(
        {
          deltaY: -1,
          metaKey: false,
          ctrlKey: false,
          altKey: false,
        },
        {
          status: 'preview',
          hasTextDraft: true,
          hasAnnotationGesture: false,
          hasAnnotationMoveGesture: false,
          hasAnnotationEditingContext: true,
        },
      ),
    ).toBeNull();

    expect(
      planCapturePointerWheelSizeAdjustment(
        {
          deltaY: -1,
          metaKey: true,
          ctrlKey: false,
          altKey: false,
        },
        {
          status: 'preview',
          hasTextDraft: false,
          hasAnnotationGesture: false,
          hasAnnotationMoveGesture: false,
          hasAnnotationEditingContext: true,
        },
      ),
    ).toBeNull();
  });
});
