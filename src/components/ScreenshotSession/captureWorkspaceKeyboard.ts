import {
  annotationColorFromShortcut,
  annotationSizeDirectionFromShortcut,
  annotationToolFromShortcut,
  isAnnotationFillToggleShortcut,
  nextAnnotationToolFromCycleShortcut,
  type AnnotationColor,
  type AnnotationGestureDraft,
  type AnnotationSizeDirection,
  type AnnotationTool,
} from './annotationStyle';
import type { AnnotationHistory } from './annotationHistory';
import {
  canToggleCapturedCursor,
  getCandidateCycleDirectionFromShortcut,
  getCaptureKeyboardToolbarAction,
  getCursorNudgeDeltaFromShortcut,
  getHoverSelectionCompletionActionFromShortcut,
  getPreviewCaptureCompletionActionFromShortcut,
  getSelectionArrowActionFromShortcut,
  getSelectionHistoryStepFromShortcut,
  getUndoRedoActionFromShortcut,
  isClearAnnotationsShortcut,
  isDeleteSelectedAnnotationShortcut,
  isMagnifierShortcut,
  isMoveDraftSelectionShortcut,
  isRefreshCaptureShortcut,
  isSelectAllCaptureShortcut,
  isToggleCapturedCursorShortcut,
  isUndoAnnotationGesturePointShortcut,
  shouldRestoreLastSelectionFromShortcut,
  type HoverSelectionCompletionAction,
  type PreviewCaptureCompletionAction,
  type SelectionHistoryStep,
} from './captureActions';
import {
  planCaptureAnnotationToolActivation,
  planCaptureSelectedAnnotationKeyboardNudge,
  type CaptureAnnotationMoveGesture,
} from './captureEditorRuntime';
import {
  planCaptureDraftSelectionKeyboardNudge,
  planCaptureDraftSelectionMoveShortcutStart,
  planCaptureHoverSelectionCycle,
  planCaptureSelectionArrowPreview,
  planCaptureSelectionCursorKeyboardNudge,
  planCaptureSelectionEditKeyboardNudge,
  type CaptureDraftSelectionMoveGesture,
  type CaptureSelectionEditGesture,
} from './captureSelectionRuntime';
import type { CaptureCandidate } from './captureCandidates';
import {
  isColorSampleCopyShortcut,
  isColorSampleFormatToggleShortcut,
  type ColorSample,
  type ColorSampleFormat,
} from './colorSampler';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import type { AnnotationCommand, ArrowKey, LogicalRect, Point } from './types';

const MIN_SELECTION_SIZE = 10;
const KEYBOARD_NUDGE_STEP = 1;
const KEYBOARD_FAST_NUDGE_STEP = 10;
const ARROW_KEYS: ArrowKey[] = [
  'ArrowUp',
  'ArrowRight',
  'ArrowDown',
  'ArrowLeft',
];

type StateUpdater<Value> = Value | ((currentValue: Value) => Value);

interface MutableRefLike<Value> {
  current: Value;
}

export interface CaptureWorkspaceKeyboardRefs {
  startPointRef: MutableRefLike<Point | null>;
  cursorPointRef: MutableRefLike<Point | null>;
  draftSelectionRef: MutableRefLike<LogicalRect | null>;
  hoverSelectionRef: MutableRefLike<LogicalRect | null>;
  keyboardDraftCursorPointRef: MutableRefLike<Point | null>;
  keyboardEditCursorPointRef: MutableRefLike<Point | null>;
}

export interface CaptureWorkspaceKeyboardDerivedState {
  annotations: AnnotationCommand[];
  captureCandidates: CaptureCandidate[];
  selectionBounds: LogicalRect | null;
  hasAnnotationEditingContext: boolean;
  isAnnotationToolbarVisible: boolean;
  isMagnifierShown: boolean;
  isFillModeActive: boolean;
  cursorColor: ColorSample | null;
}

export interface CaptureWorkspaceKeyboardActions {
  dismissCaptureLayer(): void;
  refreshSession(): Promise<void> | void;
  setIncludeCapturedCursor(value: boolean): void;
  clearPreviewImage(): void;
  renderSelectionPreview(
    rect: LogicalRect,
    annotations?: AnnotationCommand[],
    includeCursor?: boolean,
  ): Promise<void> | void;
  setIsMagnifierRequested(value: boolean): void;
  clearAnnotations(): void;
  undoPolylineGesturePoint(): boolean | void;
  undoAnnotation(): void;
  redoAnnotation(): void;
  deleteSelectedAnnotation(): void;
  copyCurrentColor(): Promise<void> | void;
  setColorSampleFormat(
    updater: (format: ColorSampleFormat) => ColorSampleFormat,
  ): void;
  restoreSelectionFromHistory(step: SelectionHistoryStep): void;
  restoreLastSelection(): void;
  setCursorPoint(point: Point): void;
  setSelection(selection: LogicalRect): void;
  scheduleSelectionOverlayPaint(
    selection?: LogicalRect | null,
    hoverSelection?: LogicalRect | null,
    cursorPoint?: Point | null,
  ): void;
  setPreviewImageBase64(imageBase64: string | null): void;
  setRenderingOutput(isRendering: boolean): void;
  setEditGesture(gesture: CaptureSelectionEditGesture): void;
  syncHoverSelection(selection: LogicalRect | null): void;
  selectFullCaptureArea(): void;
  completeCandidateSelection(
    rect: LogicalRect,
    action: HoverSelectionCompletionAction,
  ): Promise<void> | void;
  setIsAnnotationToolbarVisible(updater: StateUpdater<boolean>): void;
  completePreviewSelection(
    action: PreviewCaptureCompletionAction,
    options?: {
      commitTextDraft?: boolean;
      guardCompletion?: boolean;
    },
  ): Promise<void> | void;
  adjustAnnotationSize(direction: AnnotationSizeDirection): void;
  toggleAnnotationFill(): void;
  setActiveAnnotationTool(tool: AnnotationTool | null): void;
  setSelectedAnnotationIndex(index: number | null): void;
  setAnnotationGesture(gesture: AnnotationGestureDraft | null): void;
  setAnnotationMoveGesture(gesture: CaptureAnnotationMoveGesture | null): void;
  setDraftAnnotation(annotation: AnnotationCommand | null): void;
  selectAnnotationColor(color: AnnotationColor): void;
  toggleAnnotationTool(tool: AnnotationTool): void;
  setDraftSelectionMoveGesture(
    gesture: CaptureDraftSelectionMoveGesture | null,
  ): void;
  setAnnotationHistory(history: AnnotationHistory): void;
}

export interface CaptureWorkspaceKeyboardContext {
  state: CaptureWorkspaceState;
  refs: CaptureWorkspaceKeyboardRefs;
  derived: CaptureWorkspaceKeyboardDerivedState;
  actions: CaptureWorkspaceKeyboardActions;
}

function isArrowKey(key: string): key is ArrowKey {
  return ARROW_KEYS.includes(key as ArrowKey);
}

export function handleCaptureWorkspaceKeyDown(
  event: KeyboardEvent,
  context: CaptureWorkspaceKeyboardContext,
): void {
  const {
    state: {
      activeAnnotationTool,
      annotationGesture,
      annotationHistory,
      annotationMoveGesture,
      cursorPoint,
      draftSelectionMoveGesture,
      editGesture,
      hoverSelection,
      includeCapturedCursor,
      mode,
      selectedAnnotationIndex,
      selection,
      session,
      startPoint,
      status,
      textDraft,
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
      cursorColor,
      hasAnnotationEditingContext,
      isAnnotationToolbarVisible,
      isFillModeActive,
      isMagnifierShown,
      selectionBounds,
    },
    actions,
  } = context;

  const activeStartPoint = startPointRef.current ?? startPoint;
  const activeCursorPoint = cursorPointRef.current ?? cursorPoint;
  const activeDraftSelection = draftSelectionRef.current ?? selection;
  const activeHoverSelection = hoverSelectionRef.current ?? hoverSelection;
  const cursorNudgeDelta = getCursorNudgeDeltaFromShortcut(event);
  const candidateCycleDirection =
    getCandidateCycleDirectionFromShortcut(event);
  const hoverSelectionCompletionAction =
    getHoverSelectionCompletionActionFromShortcut(event, {
      drafting: activeStartPoint !== null,
      mode,
    });
  const selectionHistoryStep = getSelectionHistoryStepFromShortcut(event);
  const undoRedoAction = getUndoRedoActionFromShortcut(event);
  const previewCaptureCompletionAction =
    getPreviewCaptureCompletionActionFromShortcut(event);
  const toolbarAction = getCaptureKeyboardToolbarAction(
    event,
    isAnnotationToolbarVisible,
  );
  const selectionArrowAction = getSelectionArrowActionFromShortcut(event, {
    editing:
      hasAnnotationEditingContext ||
      annotationGesture !== null ||
      annotationMoveGesture !== null ||
      textDraft !== null,
  });
  const cycledAnnotationTool = nextAnnotationToolFromCycleShortcut(
    event,
    activeAnnotationTool,
  );

  if (event.key === 'Escape') {
    event.preventDefault();
    actions.dismissCaptureLayer();
  } else if (
    (status === 'selecting' || status === 'preview') &&
    isRefreshCaptureShortcut(event)
  ) {
    event.preventDefault();
    void actions.refreshSession();
  } else if (
    (status === 'selecting' || status === 'preview') &&
    !textDraft &&
    canToggleCapturedCursor(session) &&
    isToggleCapturedCursorShortcut(event)
  ) {
    event.preventDefault();
    const nextIncludeCursor = !includeCapturedCursor;
    actions.setIncludeCapturedCursor(nextIncludeCursor);
    if (status === 'preview' && selection) {
      actions.clearPreviewImage();
      void actions.renderSelectionPreview(
        selection,
        annotations,
        nextIncludeCursor,
      );
    }
  } else if (isMagnifierShortcut(event)) {
    event.preventDefault();
    actions.setIsMagnifierRequested(true);
  } else if (status === 'preview' && isClearAnnotationsShortcut(event)) {
    event.preventDefault();
    actions.clearAnnotations();
  } else if (
    status === 'preview' &&
    undoRedoAction === 'undo' &&
    annotationGesture?.tool === 'polyline'
  ) {
    event.preventDefault();
    actions.undoPolylineGesturePoint();
  } else if (status === 'preview' && undoRedoAction) {
    event.preventDefault();
    if (undoRedoAction === 'undo') {
      actions.undoAnnotation();
    } else {
      actions.redoAnnotation();
    }
  } else if (
    status === 'preview' &&
    annotationGesture?.tool === 'polyline' &&
    isUndoAnnotationGesturePointShortcut(event)
  ) {
    event.preventDefault();
    actions.undoPolylineGesturePoint();
  } else if (
    status === 'preview' &&
    selectedAnnotationIndex !== null &&
    isDeleteSelectedAnnotationShortcut(event)
  ) {
    event.preventDefault();
    actions.deleteSelectedAnnotation();
  } else if (
    !textDraft &&
    isMagnifierShown &&
    cursorColor &&
    isColorSampleCopyShortcut(event)
  ) {
    event.preventDefault();
    void actions.copyCurrentColor();
  } else if (
    !textDraft &&
    isMagnifierShown &&
    cursorColor &&
    !event.repeat &&
    isColorSampleFormatToggleShortcut(event)
  ) {
    event.preventDefault();
    actions.setColorSampleFormat((format) =>
      format === 'hex' ? 'rgb' : 'hex',
    );
  } else if (
    (status === 'selecting' || status === 'preview') &&
    !textDraft &&
    selectionHistoryStep
  ) {
    event.preventDefault();
    actions.restoreSelectionFromHistory(selectionHistoryStep);
  } else if (
    shouldRestoreLastSelectionFromShortcut(event, {
      status,
      editing:
        hasAnnotationEditingContext ||
        annotationGesture !== null ||
        annotationMoveGesture !== null ||
        textDraft !== null,
    })
  ) {
    event.preventDefault();
    actions.restoreLastSelection();
  } else if (
    status === 'selecting' &&
    !textDraft &&
    activeStartPoint &&
    activeDraftSelection &&
    activeCursorPoint &&
    selectionBounds &&
    cursorNudgeDelta
  ) {
    event.preventDefault();
    const draftNudge = planCaptureDraftSelectionKeyboardNudge({
      anchorPoint: activeStartPoint,
      cursorPoint: activeCursorPoint,
      delta: cursorNudgeDelta,
      selectionBounds,
    });
    keyboardDraftCursorPointRef.current = draftNudge.keyboardDraftCursorPoint;
    cursorPointRef.current = draftNudge.cursorPoint;
    draftSelectionRef.current = draftNudge.selection;
    actions.setCursorPoint(draftNudge.cursorPoint);
    actions.setSelection(draftNudge.selection);
    actions.scheduleSelectionOverlayPaint(draftNudge.selection, null);
    actions.setPreviewImageBase64(draftNudge.previewImageBase64);
    actions.setRenderingOutput(draftNudge.renderingOutput);
  } else if (
    status === 'preview' &&
    !textDraft &&
    editGesture &&
    selection &&
    cursorPoint &&
    selectionBounds &&
    cursorNudgeDelta
  ) {
    event.preventDefault();
    const editNudge = planCaptureSelectionEditKeyboardNudge({
      gesture: editGesture,
      selection,
      cursorPoint,
      delta: cursorNudgeDelta,
      selectionBounds,
      minSelectionSize: MIN_SELECTION_SIZE,
      preserveAspect: event.shiftKey,
    });
    keyboardEditCursorPointRef.current = editNudge.keyboardEditCursorPoint;
    actions.setCursorPoint(editNudge.cursorPoint);
    actions.setSelection(editNudge.selection);
    actions.setEditGesture(editNudge.editGesture);
    actions.setPreviewImageBase64(editNudge.previewImageBase64);
    actions.setRenderingOutput(editNudge.renderingOutput);
  } else if (
    status === 'selecting' &&
    !textDraft &&
    activeCursorPoint &&
    selectionBounds &&
    cursorNudgeDelta
  ) {
    event.preventDefault();
    const cursorNudge = planCaptureSelectionCursorKeyboardNudge({
      cursorPoint: activeCursorPoint,
      delta: cursorNudgeDelta,
      selectionBounds,
    });
    cursorPointRef.current = cursorNudge.cursorPoint;
    actions.setCursorPoint(cursorNudge.cursorPoint);
  } else if (
    status === 'selecting' &&
    !textDraft &&
    activeCursorPoint &&
    candidateCycleDirection
  ) {
    event.preventDefault();
    const hoverCycle = planCaptureHoverSelectionCycle({
      captureCandidates,
      cursorPoint: activeCursorPoint,
      hoverSelection: activeHoverSelection,
      direction: candidateCycleDirection,
    });
    actions.syncHoverSelection(hoverCycle.hoverSelection);
  } else if (
    (status === 'selecting' || status === 'preview') &&
    !textDraft &&
    isSelectAllCaptureShortcut(event)
  ) {
    event.preventDefault();
    actions.selectFullCaptureArea();
  } else if (
    status === 'selecting' &&
    activeHoverSelection &&
    hoverSelectionCompletionAction
  ) {
    event.preventDefault();
    void actions.completeCandidateSelection(
      activeHoverSelection,
      hoverSelectionCompletionAction,
    );
  } else if (
    status === 'preview' &&
    !textDraft &&
    toolbarAction === 'toggle'
  ) {
    event.preventDefault();
    actions.setIsAnnotationToolbarVisible((visible) => !visible);
  } else if (status === 'preview' && previewCaptureCompletionAction) {
    event.preventDefault();
    void actions.completePreviewSelection(previewCaptureCompletionAction, {
      guardCompletion: previewCaptureCompletionAction === 'copy',
    });
  } else if (
    status === 'preview' &&
    !textDraft &&
    (event.key === '[' ||
      event.key === ']' ||
      (hasAnnotationEditingContext && (event.key === '1' || event.key === '2')))
  ) {
    const sizeDirection = annotationSizeDirectionFromShortcut(event, {
      editing: hasAnnotationEditingContext,
    });
    if (sizeDirection) {
      event.preventDefault();
      actions.adjustAnnotationSize(sizeDirection);
    }
  } else if (
    status === 'preview' &&
    !textDraft &&
    isFillModeActive &&
    !annotationGesture &&
    !annotationMoveGesture &&
    isAnnotationFillToggleShortcut(event)
  ) {
    event.preventDefault();
    actions.toggleAnnotationFill();
  } else if (
    status === 'preview' &&
    !textDraft &&
    cycledAnnotationTool &&
    !annotationGesture &&
    !annotationMoveGesture
  ) {
    event.preventDefault();
    const toolActivation = planCaptureAnnotationToolActivation({
      currentTool: activeAnnotationTool,
      nextTool: cycledAnnotationTool,
      selectedAnnotationIndex,
      clearSelectedAnnotation: true,
      toggle: false,
    });
    actions.setActiveAnnotationTool(toolActivation.activeAnnotationTool);
    actions.setSelectedAnnotationIndex(toolActivation.selectedAnnotationIndex);
    actions.setAnnotationGesture(toolActivation.annotationGesture);
    actions.setAnnotationMoveGesture(toolActivation.annotationMoveGesture);
    actions.setDraftAnnotation(toolActivation.draftAnnotation);
  } else if (
    status === 'preview' &&
    !textDraft &&
    !annotationGesture &&
    !annotationMoveGesture &&
    selectedAnnotationIndex !== null &&
    isArrowKey(event.key)
  ) {
    // Selected annotations own Arrow keys before selection movement previews.
    event.preventDefault();
    const annotationNudge = planCaptureSelectedAnnotationKeyboardNudge({
      annotationHistory,
      annotations,
      selectedAnnotationIndex,
      key: event.key,
      fast: event.shiftKey,
      keyboardNudgeStep: KEYBOARD_NUDGE_STEP,
      keyboardFastNudgeStep: KEYBOARD_FAST_NUDGE_STEP,
    });
    if (annotationNudge.previewAnnotations && selection) {
      actions.setAnnotationHistory(annotationNudge.annotationHistory);
      void actions.renderSelectionPreview(
        selection,
        annotationNudge.previewAnnotations,
      );
    }
  } else if (
    status === 'preview' &&
    !textDraft &&
    !annotationGesture &&
    !annotationMoveGesture
  ) {
    const shortcutColor = annotationColorFromShortcut(event);
    if (shortcutColor) {
      event.preventDefault();
      actions.selectAnnotationColor(shortcutColor);
    } else {
      const shortcutTool = annotationToolFromShortcut(event);
      if (shortcutTool) {
        event.preventDefault();
        actions.toggleAnnotationTool(shortcutTool);
      }
    }
  } else if (
    isMoveDraftSelectionShortcut(event) &&
    status === 'selecting' &&
    activeStartPoint &&
    activeDraftSelection &&
    activeCursorPoint &&
    !draftSelectionMoveGesture
  ) {
    event.preventDefault();
    const draftSelectionMoveStart = planCaptureDraftSelectionMoveShortcutStart({
      cursorPoint: activeCursorPoint,
      selection: activeDraftSelection,
      anchorPoint: activeStartPoint,
    });
    actions.setDraftSelectionMoveGesture(
      draftSelectionMoveStart.draftSelectionMoveGesture,
    );
  } else if (
    status === 'preview' &&
    selection &&
    selectionBounds &&
    selectionArrowAction
  ) {
    event.preventDefault();
    const selectionArrowPreview = planCaptureSelectionArrowPreview({
      selection,
      selectionBounds,
      selectionArrowAction,
      minSelectionSize: MIN_SELECTION_SIZE,
      keyboardNudgeStep: KEYBOARD_NUDGE_STEP,
    });
    actions.setSelection(selectionArrowPreview.selection);
    actions.setPreviewImageBase64(selectionArrowPreview.previewImageBase64);
    void actions.renderSelectionPreview(selectionArrowPreview.previewRender.rect);
  }
}
