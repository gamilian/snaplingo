import type {
  AnnotationSizeDirection,
  AnnotationStyle,
} from './annotationStyle';
import { startTextAnnotationDraft } from './textAnnotationDraft';
import { isFinishAnnotationGestureDoubleClick } from './captureActions';
import {
  planCaptureAnnotationErase,
  planCaptureAnnotationGestureMove,
  planCaptureAnnotationMove,
  planCaptureAnnotationMoveCommit,
  planCaptureAnnotationToolStart,
  planCaptureExistingAnnotationPointerDown,
  planCapturePolylineAnnotationContinue,
} from './captureEditorRuntime';
import {
  getCaptureSelectionLocalPoint,
  planCapturePointerWheelSizeAdjustment,
  planCapturePreviewPointerDown,
  planCaptureRootPointerDown,
} from './capturePointerInteractionRuntime';
import {
  planCapturePreviewSelectionMoveStart,
  planCaptureSelectionEditCommit,
  planCaptureSelectionEditMove,
  planCaptureSelectionResizeStart,
} from './captureSelectionRuntime';
import type { SelectionHandle } from './selection';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import type { AnnotationCommand, LogicalRect, Point } from './types';
import { viewportPointToVirtualPoint } from './virtualDesktop';

const MIN_SELECTION_SIZE = 10;
const EDGE_SNAP_THRESHOLD = 6;

type MaybePromiseVoid = Promise<void> | void;

interface MutableRefLike<Value> {
  current: Value;
}

export interface CaptureWorkspacePointerTarget {
  setPointerCapture(pointerId: number): void;
}

export interface CaptureWorkspacePointerEvent {
  clientX: number;
  clientY: number;
  pointerId: number;
  button: number;
  detail?: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  currentTarget: CaptureWorkspacePointerTarget;
  preventDefault(): void;
  stopPropagation(): void;
}

export interface CaptureWorkspaceWheelEvent {
  deltaY: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  preventDefault(): void;
}

export interface CaptureWorkspacePointerRefs {
  cursorPointRef: MutableRefLike<Point | null>;
  keyboardEditCursorPointRef: MutableRefLike<Point | null>;
}

export interface CaptureWorkspacePointerDerivedState {
  annotations: AnnotationCommand[];
  selectionBounds: LogicalRect | null;
  snapTargetRects: LogicalRect[];
  hasAnnotationEditingContext: boolean;
  shouldTrackMagnifierCursor: boolean;
}

export interface CaptureWorkspacePointerEditorActions {
  commitTextDraft(): void;
  commitAnnotationGestureAtPoint(
    localPoint: Point,
    constrainGesture: boolean,
  ): boolean | void;
  dismissCaptureLayer(): void;
  adjustAnnotationSize(direction: AnnotationSizeDirection): void;
  renderSelectionPreview(
    rect: LogicalRect,
    annotations?: AnnotationCommand[],
    includeCursor?: boolean,
  ): MaybePromiseVoid;
  setCursorPoint(point: Point): void;
  setSelection(selection: LogicalRect): void;
  scheduleSelectionOverlayPaint(): void;
  setPreviewImageBase64(imageBase64: string | null): void;
  setRenderingOutput(isRendering: boolean): void;
  setStatus(status: CaptureWorkspaceState['status']): void;
  setAnnotationGesture(
    gesture: CaptureWorkspaceState['annotationGesture'],
  ): void;
  setDraftAnnotation(annotation: AnnotationCommand | null): void;
  setSelectedAnnotationIndex(index: number | null): void;
  setAnnotationMoveGesture(
    gesture: CaptureWorkspaceState['annotationMoveGesture'],
  ): void;
  setTextDraft(draft: CaptureWorkspaceState['textDraft']): void;
  setTextDraftAnnotationIndex(index: number | null): void;
  setAnnotationHistory(
    history: CaptureWorkspaceState['annotationHistory'],
  ): void;
  setEditGesture(gesture: CaptureWorkspaceState['editGesture']): void;
  setAnnotationStyle(style: AnnotationStyle): void;
  setTextFontSize(fontSize: number): void;
}

export interface CaptureWorkspacePointerEditorContext {
  state: CaptureWorkspaceState;
  refs: CaptureWorkspacePointerRefs;
  derived: CaptureWorkspacePointerDerivedState;
  actions: CaptureWorkspacePointerEditorActions;
}

export function handleCaptureWorkspaceEditorPointerDown(
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerEditorContext,
) {
  const {
    state: {
      activeAnnotationTool,
      annotationGesture,
      annotationMoveGesture,
      draftSelectionMoveGesture,
      selectedAnnotationIndex,
      selection,
      status,
      textDraft,
    },
    derived: { selectionBounds },
    actions,
  } = context;
  if (status !== 'preview') return;

  const plan = planCaptureRootPointerDown(event, {
    status,
    hasSelectionBounds: Boolean(selectionBounds),
    hasSelection: selection !== null,
    hasTextDraft: textDraft !== null,
    hasAnnotationGesture: annotationGesture !== null,
    hasDismissibleLayer:
      textDraft !== null ||
      annotationMoveGesture !== null ||
      draftSelectionMoveGesture !== null ||
      selectedAnnotationIndex !== null ||
      activeAnnotationTool !== null ||
      annotationGesture !== null,
  });
  if (plan.type !== 'cancel-pointer') return;
  if (
    plan.action !== 'finish-edit' &&
    plan.action !== 'finish-annotation' &&
    plan.action !== 'dismiss-layer'
  ) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  if (plan.action === 'finish-edit') {
    actions.commitTextDraft();
  } else if (plan.action === 'finish-annotation') {
    if (selection && selectionBounds && annotationGesture) {
      const point = getCaptureWorkspacePointerPoint(event, selectionBounds);
      actions.commitAnnotationGestureAtPoint(
        getCaptureSelectionLocalPoint(point, selection),
        event.shiftKey,
      );
    } else {
      actions.dismissCaptureLayer();
    }
  } else {
    actions.dismissCaptureLayer();
  }
}

export function handleCaptureWorkspaceEditorPointerMove(
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerEditorContext,
) {
  const {
    state: {
      annotationGesture,
      annotationMoveGesture,
      annotationStyle,
      editGesture,
      selection,
      status,
    },
    refs,
    derived: {
      selectionBounds,
      shouldTrackMagnifierCursor,
      snapTargetRects,
    },
    actions,
  } = context;
  if (status !== 'preview' || !selectionBounds) return;

  const point = getCaptureWorkspacePointerPoint(event, selectionBounds);
  refs.cursorPointRef.current = point;
  if (shouldTrackMagnifierCursor) actions.setCursorPoint(point);
  actions.scheduleSelectionOverlayPaint();

  if (annotationGesture && selection) {
    const move = planCaptureAnnotationGestureMove({
      gesture: annotationGesture,
      localPoint: getCaptureSelectionLocalPoint(point, selection),
      annotationStyle,
      constrainGesture: event.shiftKey,
    });
    if (move.annotationGesture !== annotationGesture) {
      actions.setAnnotationGesture(move.annotationGesture);
    }
    actions.setDraftAnnotation(move.draftAnnotation);
    return;
  }

  if (annotationMoveGesture && selection) {
    const move = planCaptureAnnotationMove({
      startAnnotation: annotationMoveGesture.startAnnotation,
      startPoint: annotationMoveGesture.startPoint,
      localPoint: getCaptureSelectionLocalPoint(point, selection),
      constrainMove: event.shiftKey,
    });
    actions.setPreviewImageBase64(move.previewImageBase64);
    actions.setDraftAnnotation(move.draftAnnotation);
    return;
  }

  if (editGesture) {
    const move = planCaptureSelectionEditMove({
      gesture: editGesture,
      point,
      selectionBounds,
      snapTargetRects,
      edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
      minSelectionSize: MIN_SELECTION_SIZE,
      preserveAspect: event.shiftKey,
    });
    refs.keyboardEditCursorPointRef.current = move.keyboardEditCursorPoint;
    actions.setSelection(move.selection);
    actions.setPreviewImageBase64(move.previewImageBase64);
    actions.setRenderingOutput(move.renderingOutput);
  }
}

export function handleCaptureWorkspaceEditorPointerUp(
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerEditorContext,
) {
  const {
    state: {
      annotationGesture,
      annotationHistory,
      annotationMoveGesture,
      editGesture,
      selection,
      status,
    },
    refs,
    derived: { annotations, selectionBounds, snapTargetRects },
    actions,
  } = context;
  if (status !== 'preview' || !selectionBounds) return;

  const point = getCaptureWorkspacePointerPoint(event, selectionBounds);
  refs.cursorPointRef.current = point;
  const editReleasePoint = refs.keyboardEditCursorPointRef.current ?? point;
  refs.keyboardEditCursorPointRef.current = null;
  actions.setCursorPoint(point);

  if (annotationGesture && selection) {
    if (annotationGesture.tool === 'polyline') return;
    actions.commitAnnotationGestureAtPoint(
      getCaptureSelectionLocalPoint(point, selection),
      event.shiftKey,
    );
    return;
  }

  if (annotationMoveGesture && selection) {
    const commit = planCaptureAnnotationMoveCommit({
      annotationHistory,
      annotations,
      annotationIndex: annotationMoveGesture.annotationIndex,
      startAnnotation: annotationMoveGesture.startAnnotation,
      startPoint: annotationMoveGesture.startPoint,
      localPoint: getCaptureSelectionLocalPoint(point, selection),
      constrainMove: event.shiftKey,
    });
    actions.setAnnotationMoveGesture(commit.annotationMoveGesture);
    actions.setDraftAnnotation(commit.draftAnnotation);
    actions.setAnnotationHistory(commit.annotationHistory);
    if (commit.selectedAnnotationIndex !== undefined) {
      actions.setSelectedAnnotationIndex(commit.selectedAnnotationIndex);
    }
    void actions.renderSelectionPreview(selection, commit.previewAnnotations);
    return;
  }

  if (editGesture) {
    const commit = planCaptureSelectionEditCommit({
      gesture: editGesture,
      point: editReleasePoint,
      selectionBounds,
      snapTargetRects,
      edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
      minSelectionSize: MIN_SELECTION_SIZE,
      preserveAspect: event.shiftKey,
      annotations,
    });
    actions.setEditGesture(commit.editGesture);
    actions.setSelection(commit.selection);
    actions.setStatus(commit.status);
    void actions.renderSelectionPreview(
      commit.previewRender.rect,
      commit.previewRender.annotations,
    );
  }
}

export function handleCaptureWorkspaceEditorPreviewPointerDown(
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerEditorContext,
) {
  const {
    state: {
      activeAnnotationTool,
      annotationGesture,
      annotationHistory,
      annotationStyle,
      textDraft,
      textFontSize,
      selection,
      status,
    },
    derived: { annotations, selectionBounds },
    actions,
  } = context;
  const plan = planCapturePreviewPointerDown(event, {
    status,
    hasSelection: selection !== null,
    hasSelectionBounds: Boolean(selectionBounds),
  });
  if (plan.type !== 'start-preview-interaction' || !selection || !selectionBounds) {
    return;
  }

  event.stopPropagation();
  event.currentTarget.setPointerCapture(event.pointerId);
  const point = getCaptureWorkspacePointerPoint(event, selectionBounds);
  actions.setCursorPoint(point);
  if (activeAnnotationTool) {
    actions.setSelectedAnnotationIndex(null);
    const localPoint = getCaptureSelectionLocalPoint(point, selection);
    if (annotationGesture?.tool === 'polyline') {
      if (isFinishAnnotationGestureDoubleClick(event)) {
        actions.commitAnnotationGestureAtPoint(localPoint, false);
        return;
      }
      const continued = planCapturePolylineAnnotationContinue({
        gesture: annotationGesture,
        localPoint,
        annotationStyle,
        constrainGesture: event.shiftKey,
      });
      actions.setAnnotationGesture(continued.annotationGesture);
      actions.setDraftAnnotation(continued.draftAnnotation);
      return;
    }

    if (activeAnnotationTool === 'text') {
      if (textDraft) return;
      actions.setTextDraft(startTextAnnotationDraft(localPoint, textFontSize));
      actions.setTextDraftAnnotationIndex(null);
      return;
    }

    if (activeAnnotationTool === 'eraser') {
      const erase = planCaptureAnnotationErase({
        annotationHistory,
        localPoint,
      });
      actions.setAnnotationMoveGesture(erase.annotationMoveGesture);
      actions.setDraftAnnotation(erase.draftAnnotation);
      if (erase.previewAnnotations) {
        actions.setAnnotationHistory(erase.annotationHistory);
        void actions.renderSelectionPreview(selection, erase.previewAnnotations);
      }
      return;
    }

    const start = planCaptureAnnotationToolStart({
      tool: activeAnnotationTool,
      localPoint,
      annotationStyle,
    });
    actions.setSelectedAnnotationIndex(start.selectedAnnotationIndex);
    actions.setAnnotationGesture(start.annotationGesture);
    actions.setDraftAnnotation(start.draftAnnotation);
    return;
  }

  const localPoint = getCaptureSelectionLocalPoint(point, selection);
  const existing = planCaptureExistingAnnotationPointerDown({
    annotations,
    localPoint,
    pointerDetail: event.detail ?? 0,
    toolbarState: { annotationStyle, textFontSize },
  });
  if (existing) {
    actions.setSelectedAnnotationIndex(existing.selectedAnnotationIndex);
    actions.setAnnotationStyle(existing.toolbarState.annotationStyle);
    actions.setTextFontSize(existing.toolbarState.textFontSize);
    if (existing.type === 'edit-text-annotation') {
      actions.setAnnotationMoveGesture(existing.annotationMoveGesture);
      actions.setDraftAnnotation(existing.draftAnnotation);
      actions.setTextDraft(existing.textDraft);
      actions.setTextDraftAnnotationIndex(existing.textDraftAnnotationIndex);
      actions.setPreviewImageBase64(existing.previewImageBase64);
      void actions.renderSelectionPreview(
        selection,
        existing.previewAnnotations,
      );
      return;
    }
    actions.setAnnotationMoveGesture(existing.annotationMoveGesture);
    return;
  }

  const moveStart = planCapturePreviewSelectionMoveStart({
    point,
    selection,
    hasTextDraft: textDraft !== null,
    isCopyDoubleClick: false,
  });
  if (moveStart.type !== 'move-selection') return;
  actions.setSelectedAnnotationIndex(moveStart.selectedAnnotationIndex);
  actions.setAnnotationMoveGesture(moveStart.annotationMoveGesture);
  actions.setEditGesture(moveStart.editGesture);
  actions.setPreviewImageBase64(moveStart.previewImageBase64);
}

export function handleCaptureWorkspaceEditorResizePointerDown(
  handle: SelectionHandle,
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerEditorContext,
) {
  const {
    state: { selection, status },
    derived: { selectionBounds },
    actions,
  } = context;
  if (status !== 'preview' || !selection || !selectionBounds) return;

  event.stopPropagation();
  event.currentTarget.setPointerCapture(event.pointerId);
  const start = planCaptureSelectionResizeStart({
    point: getCaptureWorkspacePointerPoint(event, selectionBounds),
    selection,
    handle,
  });
  actions.setCursorPoint(start.cursorPoint);
  actions.setEditGesture(start.editGesture);
  actions.setPreviewImageBase64(start.previewImageBase64);
}

export function handleCaptureWorkspaceEditorWheel(
  event: CaptureWorkspaceWheelEvent,
  context: CaptureWorkspacePointerEditorContext,
) {
  const {
    state: { annotationGesture, annotationMoveGesture, status, textDraft },
    derived: { hasAnnotationEditingContext },
    actions,
  } = context;
  const direction = planCapturePointerWheelSizeAdjustment(event, {
    status,
    hasTextDraft: textDraft !== null,
    hasAnnotationGesture: annotationGesture !== null,
    hasAnnotationMoveGesture: annotationMoveGesture !== null,
    hasAnnotationEditingContext,
  });
  if (!direction) return;
  event.preventDefault();
  actions.adjustAnnotationSize(direction);
}

export function getCaptureWorkspacePointerPoint(
  event: CaptureWorkspacePointerEvent,
  selectionBounds: LogicalRect,
): Point {
  return viewportPointToVirtualPoint(
    { x: event.clientX, y: event.clientY },
    selectionBounds,
  );
}
