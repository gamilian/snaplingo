import type {
  AnnotationSizeDirection,
  AnnotationStyle,
} from './annotationStyle';
import { startTextAnnotationDraft } from './textAnnotationDraft';
import type { CaptureCandidate } from './captureCandidates';
import { getBestCandidateAtPoint } from './captureCandidates';
import {
  isCopyCaptureDoubleClick,
  isFinishAnnotationGestureDoubleClick,
} from './captureActions';
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
  getCapturePointerMoveAction,
  getCapturePointerUpAction,
  getCaptureSelectionLocalPoint,
  planCapturePointerWheelSizeAdjustment,
  planCapturePreviewPointerDown,
  planCaptureRootPointerDown,
  shouldSyncHoverSelectionOnPointerMove,
} from './capturePointerInteractionRuntime';
import {
  planCaptureDraftSelectionCommit,
  planCaptureDraftSelectionMove,
  planCaptureDraftSelectionPointerMove,
  planCaptureDraftSelectionStart,
  planCapturePreviewSelectionMoveStart,
  planCaptureSelectionEditCommit,
  planCaptureSelectionEditMove,
  planCaptureSelectionResizeStart,
} from './captureSelectionRuntime';
import type { SelectionHandle } from './selection';
import { snapPointToRects } from './selection';
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
  startPointRef: MutableRefLike<Point | null>;
  cursorPointRef: MutableRefLike<Point | null>;
  draftSelectionRef: MutableRefLike<LogicalRect | null>;
  hoverSelectionRef: MutableRefLike<LogicalRect | null>;
  keyboardDraftCursorPointRef: MutableRefLike<Point | null>;
  keyboardEditCursorPointRef: MutableRefLike<Point | null>;
}

export interface CaptureWorkspacePointerDerivedState {
  annotations: AnnotationCommand[];
  captureCandidates: CaptureCandidate[];
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
}

export interface CaptureWorkspacePointerHostActions {
  resetPreviewSelection(): void;
  cancelSession(): MaybePromiseVoid;
  renderSelectionPreview(
    rect: LogicalRect,
    annotations?: AnnotationCommand[],
    includeCursor?: boolean,
  ): MaybePromiseVoid;
  completeManualSelection(rect: LogicalRect): void;
  pinSelection(): MaybePromiseVoid;
  copySelection(): MaybePromiseVoid;
}

export interface CaptureWorkspacePointerStateActions {
  setCursorPoint(point: Point): void;
  setStartPointWithRef(point: Point | null): void;
  setSelection(selection: LogicalRect | null): void;
  setHoverSelection(selection: LogicalRect | null): void;
  scheduleSelectionOverlayPaint(
    selection?: LogicalRect | null,
    hoverSelection?: LogicalRect | null,
    cursorPoint?: Point | null,
  ): void;
  setPreviewImageBase64(imageBase64: string | null): void;
  setRenderingOutput(isRendering: boolean): void;
  setStatus(status: CaptureWorkspaceState['status']): void;
  setActiveAnnotationTool(
    tool: CaptureWorkspaceState['activeAnnotationTool'],
  ): void;
  setAnnotationGesture(
    gesture: CaptureWorkspaceState['annotationGesture'],
  ): void;
  setDraftAnnotation(annotation: AnnotationCommand | null): void;
  setSelectedAnnotationIndex(index: number | null): void;
  setAnnotationMoveGesture(
    gesture: CaptureWorkspaceState['annotationMoveGesture'],
  ): void;
  setDraftSelectionMoveGesture(
    gesture: CaptureWorkspaceState['draftSelectionMoveGesture'],
  ): void;
  setTextDraft(draft: CaptureWorkspaceState['textDraft']): void;
  setTextDraftAnnotationIndex(index: number | null): void;
  setAnnotationHistory(history: CaptureWorkspaceState['annotationHistory']): void;
  syncHoverSelection(selection: LogicalRect | null): void;
  setEditGesture(gesture: CaptureWorkspaceState['editGesture']): void;
  setAnnotationStyle(style: AnnotationStyle): void;
  setTextFontSize(fontSize: number): void;
}

export interface CaptureWorkspacePointerActions
  extends CaptureWorkspacePointerEditorActions,
    CaptureWorkspacePointerHostActions,
    CaptureWorkspacePointerStateActions {}

export interface CaptureWorkspacePointerContext {
  state: CaptureWorkspaceState;
  refs: CaptureWorkspacePointerRefs;
  derived: CaptureWorkspacePointerDerivedState;
  actions: CaptureWorkspacePointerActions;
}

export function handleCaptureWorkspacePointerDown(
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerContext,
): void {
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
    refs: {
      cursorPointRef,
      draftSelectionRef,
      keyboardDraftCursorPointRef,
      keyboardEditCursorPointRef,
    },
    derived: { selectionBounds, snapTargetRects },
    actions,
  } = context;

  const pointerDownPlan = planCaptureRootPointerDown(event, {
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

  if (pointerDownPlan.type === 'cancel-pointer') {
    event.preventDefault();
    event.stopPropagation();
    const { action } = pointerDownPlan;

    if (action === 'finish-edit') {
      actions.commitTextDraft();
    } else if (action === 'finish-annotation') {
      if (selection && selectionBounds && annotationGesture) {
        const point = getVirtualPoint(event, selectionBounds);
        const localPoint = getCaptureSelectionLocalPoint(point, selection);
        actions.commitAnnotationGestureAtPoint(localPoint, event.shiftKey);
      } else {
        actions.dismissCaptureLayer();
      }
    } else if (action === 'dismiss-layer') {
      actions.dismissCaptureLayer();
    } else if (action === 'reset-selection') {
      actions.resetPreviewSelection();
    } else {
      void actions.cancelSession();
    }
    return;
  }

  if (pointerDownPlan.type === 'ignore' || !selectionBounds) return;

  const point = getVirtualPoint(event, selectionBounds);
  const snappedPoint = snapPointToRects(
    point,
    snapTargetRects,
    EDGE_SNAP_THRESHOLD,
  );
  const draftStart = planCaptureDraftSelectionStart({
    cursorPoint: point,
    anchorPoint: snappedPoint,
  });
  cursorPointRef.current = draftStart.cursorPoint;
  draftSelectionRef.current = draftStart.draftSelection;
  actions.setCursorPoint(draftStart.nextState.cursorPoint);
  event.currentTarget.setPointerCapture(event.pointerId);
  actions.setStartPointWithRef(draftStart.nextState.startPoint);
  actions.setSelection(draftStart.nextState.selection);
  actions.setHoverSelection(draftStart.nextState.hoverSelection);
  actions.scheduleSelectionOverlayPaint(draftStart.draftSelection, null);
  actions.setPreviewImageBase64(draftStart.nextState.previewImageBase64);
  actions.setRenderingOutput(draftStart.nextState.renderingOutput);
  actions.setStatus(draftStart.nextState.status);
  actions.setActiveAnnotationTool(draftStart.nextState.activeAnnotationTool);
  actions.setAnnotationGesture(draftStart.nextState.annotationGesture);
  actions.setDraftAnnotation(draftStart.nextState.draftAnnotation);
  actions.setSelectedAnnotationIndex(
    draftStart.nextState.selectedAnnotationIndex,
  );
  actions.setAnnotationMoveGesture(
    draftStart.nextState.annotationMoveGesture,
  );
  actions.setDraftSelectionMoveGesture(
    draftStart.nextState.draftSelectionMoveGesture,
  );
  keyboardDraftCursorPointRef.current = null;
  keyboardEditCursorPointRef.current = null;
  actions.setTextDraft(draftStart.nextState.textDraft);
  actions.setTextDraftAnnotationIndex(
    draftStart.nextState.textDraftAnnotationIndex,
  );
  actions.setAnnotationHistory(draftStart.nextState.annotationHistory);
}

export function handleCaptureWorkspacePointerMove(
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerContext,
): void {
  const {
    state: {
      annotationGesture,
      annotationMoveGesture,
      annotationStyle,
      draftSelectionMoveGesture,
      editGesture,
      selection,
      startPoint,
      status,
    },
    refs: {
      cursorPointRef,
      draftSelectionRef,
      keyboardDraftCursorPointRef,
      keyboardEditCursorPointRef,
      startPointRef,
    },
    derived: {
      captureCandidates,
      selectionBounds,
      shouldTrackMagnifierCursor,
      snapTargetRects,
    },
    actions,
  } = context;

  if (!selectionBounds) return;

  const point = getVirtualPoint(event, selectionBounds);

  cursorPointRef.current = point;

  if (shouldTrackMagnifierCursor) {
    actions.setCursorPoint(point);
  }
  actions.scheduleSelectionOverlayPaint();

  const activeStartPoint = startPointRef.current ?? startPoint;
  const shouldSyncHoverSelection = shouldSyncHoverSelectionOnPointerMove({
    status,
    hasActiveStartPoint: activeStartPoint !== null,
    hasEditGesture: editGesture !== null,
  });
  const pointerMoveAction = getCapturePointerMoveAction({
    status,
    hasSelection: selection !== null,
    hasActiveStartPoint: activeStartPoint !== null,
    hasEditGesture: editGesture !== null,
    hasAnnotationGesture: annotationGesture !== null,
    hasAnnotationMoveGesture: annotationMoveGesture !== null,
    hasDraftSelectionMoveGesture: draftSelectionMoveGesture !== null,
  });

  if (shouldSyncHoverSelection) {
    const nextHoverCandidate = getBestCandidateAtPoint(captureCandidates, point);
    const nextHoverSelection = nextHoverCandidate?.rect ?? null;
    actions.syncHoverSelection(nextHoverSelection);
  }

  if (
    pointerMoveAction === 'move-annotation-gesture' &&
    annotationGesture &&
    selection
  ) {
    const localPoint = getCaptureSelectionLocalPoint(point, selection);
    const gestureMove = planCaptureAnnotationGestureMove({
      gesture: annotationGesture,
      localPoint,
      annotationStyle,
      constrainGesture: event.shiftKey,
    });
    if (gestureMove.annotationGesture !== annotationGesture) {
      actions.setAnnotationGesture(gestureMove.annotationGesture);
    }
    actions.setDraftAnnotation(gestureMove.draftAnnotation);
    return;
  }

  if (
    pointerMoveAction === 'move-annotation' &&
    annotationMoveGesture &&
    selection
  ) {
    const localPoint = getCaptureSelectionLocalPoint(point, selection);
    const annotationMove = planCaptureAnnotationMove({
      startAnnotation: annotationMoveGesture.startAnnotation,
      startPoint: annotationMoveGesture.startPoint,
      localPoint,
      constrainMove: event.shiftKey,
    });
    actions.setPreviewImageBase64(annotationMove.previewImageBase64);
    actions.setDraftAnnotation(annotationMove.draftAnnotation);
    return;
  }

  if (
    pointerMoveAction === 'move-draft-selection' &&
    draftSelectionMoveGesture
  ) {
    const draftSelectionMove = planCaptureDraftSelectionMove({
      gesture: draftSelectionMoveGesture,
      point,
      selectionBounds,
    });
    draftSelectionRef.current = draftSelectionMove.draftSelection;
    startPointRef.current = draftSelectionMove.anchorPoint;
    actions.scheduleSelectionOverlayPaint(
      draftSelectionMove.draftSelection,
      null,
    );
    actions.setPreviewImageBase64(draftSelectionMove.previewImageBase64);
    actions.setRenderingOutput(draftSelectionMove.renderingOutput);
    return;
  }

  if (pointerMoveAction === 'edit-selection' && editGesture) {
    const editMove = planCaptureSelectionEditMove({
      gesture: editGesture,
      point,
      selectionBounds,
      snapTargetRects,
      edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
      minSelectionSize: MIN_SELECTION_SIZE,
      preserveAspect: event.shiftKey,
    });
    keyboardEditCursorPointRef.current = editMove.keyboardEditCursorPoint;
    actions.setSelection(editMove.selection);
    actions.setPreviewImageBase64(editMove.previewImageBase64);
    actions.setRenderingOutput(editMove.renderingOutput);
    return;
  }

  if (pointerMoveAction !== 'update-draft-selection' || !activeStartPoint) {
    return;
  }

  const draftMove = planCaptureDraftSelectionPointerMove({
    anchorPoint: activeStartPoint,
    point,
    snapTargetRects,
    edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
    constrainSelection: event.shiftKey,
  });
  keyboardDraftCursorPointRef.current = draftMove.keyboardDraftCursorPoint;
  draftSelectionRef.current = draftMove.draftSelection;
  actions.scheduleSelectionOverlayPaint(draftMove.draftSelection, null);
}

export function handleCaptureWorkspacePointerUp(
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerContext,
): void {
  const {
    state: {
      annotationGesture,
      annotationHistory,
      annotationMoveGesture,
      editGesture,
      hoverSelection,
      selection,
      startPoint,
      status,
    },
    refs: {
      cursorPointRef,
      draftSelectionRef,
      hoverSelectionRef,
      keyboardDraftCursorPointRef,
      keyboardEditCursorPointRef,
      startPointRef,
    },
    derived: {
      annotations,
      captureCandidates,
      selectionBounds,
      snapTargetRects,
    },
    actions,
  } = context;

  if (!selectionBounds) return;

  const point = getVirtualPoint(event, selectionBounds);
  cursorPointRef.current = point;
  const selectionReleasePoint =
    keyboardDraftCursorPointRef.current ?? cursorPointRef.current ?? point;
  const editReleasePoint = keyboardEditCursorPointRef.current ?? point;
  actions.setCursorPoint(point);
  actions.setDraftSelectionMoveGesture(null);
  keyboardDraftCursorPointRef.current = null;
  keyboardEditCursorPointRef.current = null;
  const activeStartPoint = startPointRef.current ?? startPoint;
  const pointerUpAction = getCapturePointerUpAction({
    status,
    hasSelection: selection !== null,
    hasActiveStartPoint: activeStartPoint !== null,
    hasEditGesture: editGesture !== null,
    hasAnnotationGesture: annotationGesture !== null,
    hasAnnotationMoveGesture: annotationMoveGesture !== null,
  });

  if (
    pointerUpAction === 'commit-annotation-gesture' &&
    annotationGesture &&
    selection
  ) {
    const localPoint = getCaptureSelectionLocalPoint(point, selection);
    if (annotationGesture.tool === 'polyline') return;

    actions.commitAnnotationGestureAtPoint(localPoint, event.shiftKey);
    return;
  }

  if (
    pointerUpAction === 'commit-annotation-move' &&
    annotationMoveGesture &&
    selection
  ) {
    const localPoint = getCaptureSelectionLocalPoint(point, selection);
    const annotationMoveCommit = planCaptureAnnotationMoveCommit({
      annotationHistory,
      annotations,
      annotationIndex: annotationMoveGesture.annotationIndex,
      startAnnotation: annotationMoveGesture.startAnnotation,
      startPoint: annotationMoveGesture.startPoint,
      localPoint,
      constrainMove: event.shiftKey,
    });
    actions.setAnnotationMoveGesture(
      annotationMoveCommit.annotationMoveGesture,
    );
    actions.setDraftAnnotation(annotationMoveCommit.draftAnnotation);
    actions.setAnnotationHistory(annotationMoveCommit.annotationHistory);
    if (annotationMoveCommit.selectedAnnotationIndex !== undefined) {
      actions.setSelectedAnnotationIndex(
        annotationMoveCommit.selectedAnnotationIndex,
      );
    }
    void actions.renderSelectionPreview(
      selection,
      annotationMoveCommit.previewAnnotations,
    );
    return;
  }

  if (pointerUpAction === 'commit-selection-edit' && editGesture) {
    const editCommit = planCaptureSelectionEditCommit({
      gesture: editGesture,
      point: editReleasePoint,
      selectionBounds,
      snapTargetRects,
      edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
      minSelectionSize: MIN_SELECTION_SIZE,
      preserveAspect: event.shiftKey,
      annotations,
    });
    actions.setEditGesture(editCommit.editGesture);
    actions.setSelection(editCommit.selection);
    actions.setStatus(editCommit.status);
    void actions.renderSelectionPreview(
      editCommit.previewRender.rect,
      editCommit.previewRender.annotations,
    );
    return;
  }

  if (pointerUpAction !== 'commit-draft-selection' || !activeStartPoint) {
    return;
  }

  const activeHoverSelection = hoverSelectionRef.current ?? hoverSelection;
  const draftCommit = planCaptureDraftSelectionCommit({
    anchorPoint: activeStartPoint,
    releasePoint: selectionReleasePoint,
    snapTargetRects,
    edgeSnapThreshold: EDGE_SNAP_THRESHOLD,
    constrainSelection: event.shiftKey,
    captureCandidates,
    activeHoverSelection,
    minSelectionSize: MIN_SELECTION_SIZE,
  });
  actions.setStartPointWithRef(draftCommit.startPoint);
  draftSelectionRef.current = draftCommit.draftSelection;
  actions.scheduleSelectionOverlayPaint(
    null,
    draftCommit.overlayHoverSelection,
  );

  if (draftCommit.type === 'clear-selection') {
    actions.setSelection(draftCommit.selection);
    actions.scheduleSelectionOverlayPaint(null, null);
    return;
  }

  actions.completeManualSelection(draftCommit.selection);
}

export function handleCaptureWorkspacePreviewPointerDown(
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerContext,
): void {
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

  const previewPointerDownPlan = planCapturePreviewPointerDown(event, {
    status,
    hasSelection: selection !== null,
    hasSelectionBounds: Boolean(selectionBounds),
  });

  if (
    previewPointerDownPlan.type === 'ignore' ||
    !selection ||
    !selectionBounds
  ) {
    return;
  }

  if (previewPointerDownPlan.type === 'pin-selection') {
    event.preventDefault();
    event.stopPropagation();
    void actions.pinSelection();
    return;
  }

  event.stopPropagation();
  event.currentTarget.setPointerCapture(event.pointerId);
  const point = getVirtualPoint(event, selectionBounds);
  actions.setCursorPoint(point);
  if (activeAnnotationTool) {
    actions.setSelectedAnnotationIndex(null);
    const localPoint = getCaptureSelectionLocalPoint(point, selection);
    if (annotationGesture?.tool === 'polyline') {
      if (isFinishAnnotationGestureDoubleClick(event)) {
        actions.commitAnnotationGestureAtPoint(localPoint, false);
        return;
      }

      const polylineContinue = planCapturePolylineAnnotationContinue({
        gesture: annotationGesture,
        localPoint,
        annotationStyle,
        constrainGesture: event.shiftKey,
      });
      actions.setAnnotationGesture(polylineContinue.annotationGesture);
      actions.setDraftAnnotation(polylineContinue.draftAnnotation);
      return;
    }

    if (activeAnnotationTool === 'text') {
      if (textDraft) return;
      actions.setTextDraft(startTextAnnotationDraft(localPoint, textFontSize));
      actions.setTextDraftAnnotationIndex(null);
      return;
    }

    if (activeAnnotationTool === 'eraser') {
      const erasePlan = planCaptureAnnotationErase({
        annotationHistory,
        localPoint,
      });
      actions.setAnnotationMoveGesture(erasePlan.annotationMoveGesture);
      actions.setDraftAnnotation(erasePlan.draftAnnotation);
      if (erasePlan.previewAnnotations) {
        actions.setAnnotationHistory(erasePlan.annotationHistory);
        void actions.renderSelectionPreview(
          selection,
          erasePlan.previewAnnotations,
        );
      }
      return;
    }

    const toolStart = planCaptureAnnotationToolStart({
      tool: activeAnnotationTool,
      localPoint,
      annotationStyle,
    });
    actions.setSelectedAnnotationIndex(toolStart.selectedAnnotationIndex);
    actions.setAnnotationGesture(toolStart.annotationGesture);
    actions.setDraftAnnotation(toolStart.draftAnnotation);
    return;
  }

  const localPoint = getCaptureSelectionLocalPoint(point, selection);
  const existingAnnotationStart = planCaptureExistingAnnotationPointerDown({
    annotations,
    localPoint,
    pointerDetail: event.detail ?? 0,
    toolbarState: {
      annotationStyle,
      textFontSize,
    },
  });
  if (existingAnnotationStart) {
    actions.setSelectedAnnotationIndex(
      existingAnnotationStart.selectedAnnotationIndex,
    );
    actions.setAnnotationStyle(
      existingAnnotationStart.toolbarState.annotationStyle,
    );
    actions.setTextFontSize(existingAnnotationStart.toolbarState.textFontSize);

    if (existingAnnotationStart.type === 'edit-text-annotation') {
      actions.setAnnotationMoveGesture(
        existingAnnotationStart.annotationMoveGesture,
      );
      actions.setDraftAnnotation(existingAnnotationStart.draftAnnotation);
      actions.setTextDraft(existingAnnotationStart.textDraft);
      actions.setTextDraftAnnotationIndex(
        existingAnnotationStart.textDraftAnnotationIndex,
      );
      actions.setPreviewImageBase64(
        existingAnnotationStart.previewImageBase64,
      );
      void actions.renderSelectionPreview(
        selection,
        existingAnnotationStart.previewAnnotations,
      );
      return;
    }

    actions.setAnnotationMoveGesture(
      existingAnnotationStart.annotationMoveGesture,
    );
    return;
  }

  const selectionMoveStart = planCapturePreviewSelectionMoveStart({
    point,
    selection,
    hasTextDraft: textDraft !== null,
    isCopyDoubleClick: isCopyCaptureDoubleClick(event),
  });
  if (selectionMoveStart.type === 'copy-selection') {
    event.preventDefault();
    void actions.copySelection();
    return;
  }

  actions.setSelectedAnnotationIndex(selectionMoveStart.selectedAnnotationIndex);
  actions.setAnnotationMoveGesture(selectionMoveStart.annotationMoveGesture);
  actions.setEditGesture(selectionMoveStart.editGesture);
  actions.setPreviewImageBase64(selectionMoveStart.previewImageBase64);
}

export function handleCaptureWorkspaceResizePointerDown(
  handle: SelectionHandle,
  event: CaptureWorkspacePointerEvent,
  context: CaptureWorkspacePointerContext,
): void {
  const {
    state: { selection, status },
    derived: { selectionBounds },
    actions,
  } = context;

  if (status !== 'preview' || !selection || !selectionBounds) return;

  event.stopPropagation();
  event.currentTarget.setPointerCapture(event.pointerId);
  const point = getVirtualPoint(event, selectionBounds);
  const resizeStart = planCaptureSelectionResizeStart({
    point,
    selection,
    handle,
  });
  actions.setCursorPoint(resizeStart.cursorPoint);
  actions.setEditGesture(resizeStart.editGesture);
  actions.setPreviewImageBase64(resizeStart.previewImageBase64);
}

export function handleCaptureWorkspaceWheel(
  event: CaptureWorkspaceWheelEvent,
  context: CaptureWorkspacePointerContext,
): void {
  const {
    state: {
      annotationGesture,
      annotationMoveGesture,
      status,
      textDraft,
    },
    derived: { hasAnnotationEditingContext },
    actions,
  } = context;

  const sizeDirection = planCapturePointerWheelSizeAdjustment(event, {
    status,
    hasTextDraft: textDraft !== null,
    hasAnnotationGesture: annotationGesture !== null,
    hasAnnotationMoveGesture: annotationMoveGesture !== null,
    hasAnnotationEditingContext,
  });
  if (!sizeDirection) return;

  event.preventDefault();
  actions.adjustAnnotationSize(sizeDirection);
}

function getVirtualPoint(
  event: CaptureWorkspacePointerEvent,
  selectionBounds: LogicalRect,
): Point {
  return viewportPointToVirtualPoint(
    { x: event.clientX, y: event.clientY },
    selectionBounds,
  );
}
