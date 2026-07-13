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
  getCaptureKeyboardToolbarAction,
  getCursorNudgeDeltaFromShortcut,
  getSelectionArrowActionFromShortcut,
  getUndoRedoActionFromShortcut,
  isClearAnnotationsShortcut,
  isDeleteSelectedAnnotationShortcut,
  isMagnifierShortcut,
} from './captureActions';
import {
  planCaptureAnnotationToolActivation,
  planCaptureSelectedAnnotationKeyboardNudge,
  type CaptureAnnotationMoveGesture,
} from './captureEditorRuntime';
import {
  planCaptureSelectionArrowPreview,
  planCaptureSelectionEditKeyboardNudge,
  type CaptureSelectionEditGesture,
} from './captureSelectionRuntime';
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
  keyboardEditCursorPointRef: MutableRefLike<Point | null>;
}

export interface CaptureWorkspaceKeyboardDerivedState {
  annotations: AnnotationCommand[];
  selectionBounds: LogicalRect | null;
  hasAnnotationEditingContext: boolean;
  isAnnotationToolbarVisible: boolean;
  isMagnifierShown: boolean;
  isFillModeActive: boolean;
  cursorColor: ColorSample | null;
}

export interface CaptureWorkspaceKeyboardEditorActions {
  dismissCaptureLayer(): void;
  renderSelectionPreview(
    rect: LogicalRect,
    annotations?: AnnotationCommand[],
    includeCursor?: boolean,
  ): Promise<void> | void;
  setIsMagnifierRequested(value: boolean): void;
  clearAnnotations(): void;
  undoAnnotation(): void;
  redoAnnotation(): void;
  deleteSelectedAnnotation(): void;
  copyCurrentColor(): Promise<void> | void;
  setColorSampleFormat(
    updater: (format: ColorSampleFormat) => ColorSampleFormat,
  ): void;
  setCursorPoint(point: Point): void;
  setSelection(selection: LogicalRect): void;
  setPreviewImageBase64(imageBase64: string | null): void;
  setRenderingOutput(isRendering: boolean): void;
  setEditGesture(gesture: CaptureSelectionEditGesture): void;
  setIsAnnotationToolbarVisible(updater: StateUpdater<boolean>): void;
  adjustAnnotationSize(direction: AnnotationSizeDirection): void;
  toggleAnnotationFill(): void;
  setActiveAnnotationTool(tool: AnnotationTool | null): void;
  setSelectedAnnotationIndex(index: number | null): void;
  setAnnotationGesture(gesture: AnnotationGestureDraft | null): void;
  setAnnotationMoveGesture(gesture: CaptureAnnotationMoveGesture | null): void;
  setDraftAnnotation(annotation: AnnotationCommand | null): void;
  selectAnnotationColor(color: AnnotationColor): void;
  toggleAnnotationTool(tool: AnnotationTool): void;
  setAnnotationHistory(history: AnnotationHistory): void;
}

export interface CaptureWorkspaceKeyboardEditorContext {
  state: CaptureWorkspaceState;
  refs: CaptureWorkspaceKeyboardRefs;
  derived: CaptureWorkspaceKeyboardDerivedState;
  actions: CaptureWorkspaceKeyboardEditorActions;
}

export function handleCaptureWorkspaceEditorKeyDown(
  event: KeyboardEvent,
  context: CaptureWorkspaceKeyboardEditorContext,
) {
  const { state, refs, derived, actions } = context;
  const {
    activeAnnotationTool,
    annotationGesture,
    annotationHistory,
    annotationMoveGesture,
    cursorPoint,
    editGesture,
    selectedAnnotationIndex,
    selection,
    status,
    textDraft,
  } = state;
  if (status !== 'preview') {
    if (isMagnifierShortcut(event)) {
      event.preventDefault();
      actions.setIsMagnifierRequested(true);
    }
    return;
  }

  const { annotations, cursorColor, hasAnnotationEditingContext } = derived;
  const undoRedoAction = getUndoRedoActionFromShortcut(event);
  const cursorNudgeDelta = getCursorNudgeDeltaFromShortcut(event);
  const cycledAnnotationTool = nextAnnotationToolFromCycleShortcut(
    event,
    activeAnnotationTool,
  );

  if (event.key === 'Escape') {
    event.preventDefault();
    actions.dismissCaptureLayer();
  } else if (isMagnifierShortcut(event)) {
    event.preventDefault();
    actions.setIsMagnifierRequested(true);
  } else if (isClearAnnotationsShortcut(event)) {
    event.preventDefault();
    actions.clearAnnotations();
  } else if (undoRedoAction) {
    event.preventDefault();
    if (undoRedoAction === 'undo') actions.undoAnnotation();
    else actions.redoAnnotation();
  } else if (
    selectedAnnotationIndex !== null &&
    isDeleteSelectedAnnotationShortcut(event)
  ) {
    event.preventDefault();
    actions.deleteSelectedAnnotation();
  } else if (
    !textDraft &&
    derived.isMagnifierShown &&
    cursorColor &&
    isColorSampleCopyShortcut(event)
  ) {
    event.preventDefault();
    void actions.copyCurrentColor();
  } else if (
    !textDraft &&
    derived.isMagnifierShown &&
    cursorColor &&
    !event.repeat &&
    isColorSampleFormatToggleShortcut(event)
  ) {
    event.preventDefault();
    actions.setColorSampleFormat((format) =>
      format === 'hex' ? 'rgb' : 'hex',
    );
  } else if (
    !textDraft &&
    editGesture &&
    selection &&
    cursorPoint &&
    derived.selectionBounds &&
    cursorNudgeDelta
  ) {
    event.preventDefault();
    const editNudge = planCaptureSelectionEditKeyboardNudge({
      gesture: editGesture,
      selection,
      cursorPoint,
      delta: cursorNudgeDelta,
      selectionBounds: derived.selectionBounds,
      minSelectionSize: MIN_SELECTION_SIZE,
      preserveAspect: event.shiftKey,
    });
    refs.keyboardEditCursorPointRef.current =
      editNudge.keyboardEditCursorPoint;
    actions.setCursorPoint(editNudge.cursorPoint);
    actions.setSelection(editNudge.selection);
    actions.setEditGesture(editNudge.editGesture);
    actions.setPreviewImageBase64(editNudge.previewImageBase64);
    actions.setRenderingOutput(editNudge.renderingOutput);
  } else if (
    !textDraft &&
    getCaptureKeyboardToolbarAction(event, derived.isAnnotationToolbarVisible) ===
      'toggle'
  ) {
    event.preventDefault();
    actions.setIsAnnotationToolbarVisible((visible) => !visible);
  } else if (
    !textDraft &&
    (event.key === '[' ||
      event.key === ']' ||
      (hasAnnotationEditingContext &&
        (event.key === '1' || event.key === '2')))
  ) {
    const direction = annotationSizeDirectionFromShortcut(event, {
      editing: hasAnnotationEditingContext,
    });
    if (direction) {
      event.preventDefault();
      actions.adjustAnnotationSize(direction);
    }
  } else if (
    !textDraft &&
    derived.isFillModeActive &&
    !annotationGesture &&
    !annotationMoveGesture &&
    isAnnotationFillToggleShortcut(event)
  ) {
    event.preventDefault();
    actions.toggleAnnotationFill();
  } else if (
    !textDraft &&
    cycledAnnotationTool &&
    !annotationGesture &&
    !annotationMoveGesture
  ) {
    event.preventDefault();
    const activation = planCaptureAnnotationToolActivation({
      currentTool: activeAnnotationTool,
      nextTool: cycledAnnotationTool,
      selectedAnnotationIndex,
      clearSelectedAnnotation: true,
      toggle: false,
    });
    actions.setActiveAnnotationTool(activation.activeAnnotationTool);
    actions.setSelectedAnnotationIndex(activation.selectedAnnotationIndex);
    actions.setAnnotationGesture(activation.annotationGesture);
    actions.setAnnotationMoveGesture(activation.annotationMoveGesture);
    actions.setDraftAnnotation(activation.draftAnnotation);
  } else if (
    !textDraft &&
    !annotationGesture &&
    !annotationMoveGesture &&
    selectedAnnotationIndex !== null &&
    isArrowKey(event.key)
  ) {
    event.preventDefault();
    const nudge = planCaptureSelectedAnnotationKeyboardNudge({
      annotationHistory,
      annotations,
      selectedAnnotationIndex,
      key: event.key,
      fast: event.shiftKey,
      keyboardNudgeStep: KEYBOARD_NUDGE_STEP,
      keyboardFastNudgeStep: KEYBOARD_FAST_NUDGE_STEP,
    });
    if (nudge.previewAnnotations && selection) {
      actions.setAnnotationHistory(nudge.annotationHistory);
    }
  } else if (!textDraft && !annotationGesture && !annotationMoveGesture) {
    const color = annotationColorFromShortcut(event);
    if (color) {
      event.preventDefault();
      actions.selectAnnotationColor(color);
      return;
    }
    const tool = annotationToolFromShortcut(event);
    if (tool) {
      event.preventDefault();
      actions.toggleAnnotationTool(tool);
      return;
    }
    const selectionArrowAction = getSelectionArrowActionFromShortcut(event, {
      editing: false,
    });
    if (selection && derived.selectionBounds && selectionArrowAction) {
      event.preventDefault();
      const preview = planCaptureSelectionArrowPreview({
        selection,
        selectionBounds: derived.selectionBounds,
        selectionArrowAction,
        minSelectionSize: MIN_SELECTION_SIZE,
        keyboardNudgeStep: KEYBOARD_NUDGE_STEP,
      });
      actions.setSelection(preview.selection);
      actions.setPreviewImageBase64(preview.previewImageBase64);
      void actions.renderSelectionPreview(preview.previewRender.rect);
    }
  }
}

function isArrowKey(key: string): key is ArrowKey {
  return ARROW_KEYS.includes(key as ArrowKey);
}
