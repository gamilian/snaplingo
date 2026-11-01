import {
  ANNOTATION_COLORS,
  type AnnotationColor,
} from '../../domain/annotationColor';
import type {
  AnnotationCommand,
  ArrowKey,
  LogicalRect,
  Point,
} from '../../domain/capture';
import {
  clearAnnotationHistory,
  redoAnnotationHistory,
  removeAnnotationFromHistory,
  undoAnnotationHistory,
} from './annotationHistory';
import {
  annotationColorFromShortcut,
  annotationSizeDirectionFromShortcut,
  annotationToolFromShortcut,
  isAnnotationFillToggleShortcut,
  nextAnnotationToolFromCycleShortcut,
  type AnnotationSizeDirection,
  type AnnotationStyle,
  type AnnotationTool,
} from './annotationStyle';
import {
  canToggleCapturedCursor,
  getCaptureKeyboardToolbarAction,
  getCursorNudgeDeltaFromShortcut,
  getSelectionArrowActionFromShortcut,
  getUndoRedoActionFromShortcut,
  isClearAnnotationsShortcut,
  isDeleteSelectedAnnotationShortcut,
  isMagnifierShortcut,
} from './captureActions';
import {
  applyStyleToSelectedAnnotationHistory,
  commitCaptureEditorTextDraft,
  completeCaptureEditorGesture,
  getCaptureEditorDismissAction,
  planCaptureAnnotationColorSelection,
  planCaptureAnnotationFillToggle,
  planCaptureAnnotationSizeAdjustment,
  planCaptureAnnotationToolActivation,
  planCaptureAnnotationGestureMove,
  planCaptureAnnotationMove,
  planCaptureAnnotationMoveCommit,
  planCaptureAnnotationToolStart,
  planCaptureExistingAnnotationPointerDown,
  planCaptureSelectedAnnotationKeyboardNudge,
} from './captureEditorRuntime';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
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
  planCaptureSelectionArrowPreview,
  planCaptureSelectionEditKeyboardNudge,
} from './captureSelectionRuntime';
import {
  startTextAnnotationDraft,
  updateTextAnnotationDraft,
} from './textAnnotationDraft';
import type { SelectionHandle } from './selection';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import type {
  CaptureWorkspaceKeyInput,
  CaptureWorkspaceKeyboardEvent,
  CaptureWorkspacePointerInput,
  CaptureWorkspaceWheelInput,
} from './captureWorkspaceRuntimeTypes';
import {
  colorSampleToClipboardText,
  isColorSampleCopyShortcut,
  isColorSampleFormatToggleShortcut,
} from '../image-inspection/colorSampler';

const MIN_SELECTION_SIZE = 10;
const EDGE_SNAP_THRESHOLD = 6;
const KEYBOARD_NUDGE_STEP = 1;
const KEYBOARD_FAST_NUDGE_STEP = 10;
const ARROW_KEYS: ArrowKey[] = [
  'ArrowUp',
  'ArrowRight',
  'ArrowDown',
  'ArrowLeft',
];

interface EditorPointerInput {
  point: Point;
  button: number;
  detail?: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  preventDefault(): void;
  stopPropagation(): void;
}
interface EditorWheelInput {
  deltaY: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  preventDefault(): void;
}

export function createCaptureEditing({
  read,
  update,
  hydratedSessionId,
  renderSelectionPreview,
  cancelSession,
  copyText,
  annotationColorPresets,
}: {
  read: () => CaptureWorkspaceState;
  update: (next: Partial<CaptureWorkspaceState>) => void;
  hydratedSessionId: () => string | null;
  renderSelectionPreview: (
    rect: LogicalRect,
    annotations?: AnnotationCommand[],
    includeCursor?: boolean,
  ) => Promise<void>;
  cancelSession: () => void;
  copyText: (text: string) => Promise<void>;
  annotationColorPresets?: () => readonly AnnotationColor[];
}) {
  const patch = update;
  const keyboardCursor = { current: null as Point | null };
  const getEditorDerived = () => ({
    ...getCaptureWorkspaceDerivedState({
      state: read(),
      hydratedCaptureSessionId: hydratedSessionId(),
      toolbarGap: 0,
      toolbarSize: { width: 0, height: 0 },
    }),
    cursorColor: read().cursorColor,
    isAnnotationToolbarVisible: read().isAnnotationToolbarVisible,
  });

  const hasDismissibleLayer = () =>
    read().textDraft !== null ||
    read().annotationGesture !== null ||
    read().annotationMoveGesture !== null ||
    read().draftSelectionMoveGesture !== null ||
    read().selectedAnnotationIndex !== null ||
    read().activeAnnotationTool !== null;

  const renderEditorSelectionPreview = (
    rect: LogicalRect,
    _annotations: AnnotationCommand[] = read().annotationHistory.annotations,
    includeCursor = read().includeCapturedCursor &&
      canToggleCapturedCursor(read().session),
  ) => renderSelectionPreview(rect, [], includeCursor);

  const commitTextDraftToHistory = () => {
    const result = commitCaptureEditorTextDraft({
      annotationHistory: read().annotationHistory,
      selectedAnnotationIndex: read().selectedAnnotationIndex,
      textDraft: read().textDraft,
      annotationStyle: read().annotationStyle,
      textDraftAnnotationIndex: read().textDraftAnnotationIndex,
    });
    patch({
      annotationHistory: result.annotationHistory,
      selectedAnnotationIndex: result.selectedAnnotationIndex,
      textDraft: result.textDraft,
      textDraftAnnotationIndex: result.textDraftAnnotationIndex,
    });
    return result.annotationHistory;
  };

  const applySelectedAnnotationStyle = (
    nextStyle: AnnotationStyle,
    nextTextFontSize: number,
  ) => {
    const derived = getEditorDerived();
    const previousHistory = read().annotationHistory;
    const nextHistory = applyStyleToSelectedAnnotationHistory({
      annotationHistory: previousHistory,
      annotations: derived.annotations,
      selectedAnnotationIndex: read().selectedAnnotationIndex,
      textDraftActive: read().textDraft !== null,
      nextStyle,
      nextTextFontSize,
    });
    patch({
      annotationStyle: nextStyle,
      textFontSize: nextTextFontSize,
      annotationHistory: nextHistory,
    });
  };

  const adjustAnnotationSize = (direction: AnnotationSizeDirection) => {
    if (read().textDraft) return;
    const next = planCaptureAnnotationSizeAdjustment({
      annotationStyle: read().annotationStyle,
      textFontSize: read().textFontSize,
      direction,
      isTextSizingActive: getEditorDerived().isTextSizingActive,
    });
    applySelectedAnnotationStyle(next.annotationStyle, next.textFontSize);
  };

  const selectAnnotationColor = (color: AnnotationColor) => {
    if (read().textDraft) return;
    const next = planCaptureAnnotationColorSelection({
      annotationStyle: read().annotationStyle,
      textFontSize: read().textFontSize,
      color,
    });
    applySelectedAnnotationStyle(next.annotationStyle, next.textFontSize);
  };

  const toggleAnnotationFill = () => {
    if (read().textDraft || !getEditorDerived().isFillModeActive) return;
    const next = planCaptureAnnotationFillToggle({
      annotationStyle: read().annotationStyle,
      textFontSize: read().textFontSize,
    });
    applySelectedAnnotationStyle(next.annotationStyle, next.textFontSize);
  };

  const undoAnnotation = () => {
    const nextHistory = undoAnnotationHistory(read().annotationHistory);
    if (nextHistory === read().annotationHistory) return;
    patch({
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      annotationHistory: nextHistory,
    });
  };

  const redoAnnotation = () => {
    const nextHistory = redoAnnotationHistory(read().annotationHistory);
    if (nextHistory === read().annotationHistory) return;
    patch({
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      annotationHistory: nextHistory,
    });
  };

  const clearAnnotations = () => {
    const nextHistory = clearAnnotationHistory(read().annotationHistory);
    if (nextHistory === read().annotationHistory) return;
    patch({
      annotationGesture: null,
      draftAnnotation: null,
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      textDraft: null,
      textDraftAnnotationIndex: null,
      annotationHistory: nextHistory,
    });
  };

  const deleteSelectedAnnotation = () => {
    const state = read();
    if (state.selectedAnnotationIndex === null) return;
    const nextHistory = removeAnnotationFromHistory(
      state.annotationHistory,
      state.selectedAnnotationIndex,
    );
    if (nextHistory === read().annotationHistory) return;
    patch({
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      annotationHistory: nextHistory,
    });
  };

  const commitAnnotationGestureAtPoint = (
    localPoint: Point,
    constrainGesture: boolean,
  ) => {
    const previousHistory = read().annotationHistory;
    const result = completeCaptureEditorGesture({
      annotationHistory: previousHistory,
      selectedAnnotationIndex: read().selectedAnnotationIndex,
      annotationGesture: read().annotationGesture,
      localPoint,
      annotationStyle: read().annotationStyle,
      constrainGesture,
    });
    if (!result) return false;
    patch({
      annotationHistory: result.annotationHistory,
      selectedAnnotationIndex: result.selectedAnnotationIndex,
      annotationGesture: result.annotationGesture,
      draftAnnotation: result.draftAnnotation,
    });
    return true;
  };

  const dismissCaptureLayer = () => {
    const action = getCaptureEditorDismissAction({
      hasTextDraft: read().textDraft !== null,
      hasAnnotationMoveGesture: read().annotationMoveGesture !== null,
      hasDraftSelectionMoveGesture: read().draftSelectionMoveGesture !== null,
      hasSelectedAnnotation: read().selectedAnnotationIndex !== null,
      hasActiveAnnotationTool: read().activeAnnotationTool !== null,
      hasAnnotationGesture: read().annotationGesture !== null,
    });
    if (action === 'clear-text-draft') {
      patch({ textDraft: null, textDraftAnnotationIndex: null });
    } else if (action === 'revert-annotation-move') {
      patch({ annotationMoveGesture: null, draftAnnotation: null });
    } else if (action === 'clear-draft-selection-move') {
      patch({ draftSelectionMoveGesture: null });
    } else if (action === 'clear-selected-annotation') {
      patch({ selectedAnnotationIndex: null });
    } else if (action === 'clear-active-annotation-tool') {
      patch({
        activeAnnotationTool: null,
        annotationGesture: null,
        draftAnnotation: null,
      });
    } else {
      cancelSession();
    }
  };

  const toggleAnnotationTool = (nextTool: AnnotationTool) => {
    commitTextDraftToHistory();
    const activation = planCaptureAnnotationToolActivation({
      currentTool: read().activeAnnotationTool,
      nextTool,
      selectedAnnotationIndex: read().selectedAnnotationIndex,
      clearSelectedAnnotation: false,
      toggle: false,
    });
    patch(activation);
  };

  const commitTextDraft = () => {
    commitTextDraftToHistory();
    patch({ selectedAnnotationIndex: null });
  };
  const copyCurrentColor = async () => {
    const state = read();
    if (!state.cursorColor) return;
    try {
      await copyText(
        colorSampleToClipboardText(state.cursorColor, state.colorSampleFormat),
      );
    } catch (error) {
      patch({ error: error instanceof Error ? error.message : String(error) });
    }
  };
  function handleCaptureWorkspaceEditorPointerDown(event: EditorPointerInput) {
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
    } = { state: read(), derived: getEditorDerived() };
    if (status !== 'preview') return;

    if (textDraft && event.button === 0) {
      event.preventDefault();
      event.stopPropagation();
      commitTextDraft();
      return;
    }

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
      commitTextDraft();
    } else if (plan.action === 'finish-annotation') {
      if (selection && selectionBounds && annotationGesture) {
        const point = event.point;
        commitAnnotationGestureAtPoint(
          getCaptureSelectionLocalPoint(point, selection),
          event.shiftKey,
        );
      } else {
        dismissCaptureLayer();
      }
    } else {
      dismissCaptureLayer();
    }
  }

  function handleCaptureWorkspaceEditorPointerMove(event: EditorPointerInput) {
    const {
      state: {
        annotationGesture,
        annotationMoveGesture,
        annotationStyle,
        editGesture,
        selection,
        status,
      },
      derived: { selectionBounds, shouldTrackMagnifierCursor, snapTargetRects },
    } = { state: read(), derived: getEditorDerived() };
    if (status !== 'preview' || !selectionBounds) return;

    const point = event.point;
    if (shouldTrackMagnifierCursor) patch({ cursorPoint: point });

    if (annotationGesture && selection) {
      const move = planCaptureAnnotationGestureMove({
        gesture: annotationGesture,
        localPoint: getCaptureSelectionLocalPoint(point, selection),
        annotationStyle,
        constrainGesture: event.shiftKey,
      });
      if (move.annotationGesture !== annotationGesture) {
        patch({
          annotationGesture: move.annotationGesture,
          draftAnnotation: move.draftAnnotation,
        });
      } else {
        patch({ draftAnnotation: move.draftAnnotation });
      }
      return;
    }

    if (annotationMoveGesture && selection) {
      const move = planCaptureAnnotationMove({
        startAnnotation: annotationMoveGesture.startAnnotation,
        startPoint: annotationMoveGesture.startPoint,
        localPoint: getCaptureSelectionLocalPoint(point, selection),
        constrainMove: event.shiftKey,
        resizeHandle: annotationMoveGesture.resizeHandle,
        selectionBounds: {
          x: 0,
          y: 0,
          width: selection.width,
          height: selection.height,
        },
      });
      patch({ draftAnnotation: move.draftAnnotation });
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
      keyboardCursor.current = move.keyboardEditCursorPoint;
      patch({
        selection: move.selection,
        isRenderingOutput: move.renderingOutput,
      });
    }
  }

  function handleCaptureWorkspaceEditorPointerUp(event: EditorPointerInput) {
    const {
      state: {
        annotationGesture,
        annotationHistory,
        annotationMoveGesture,
        editGesture,
        selection,
        status,
      },
      derived: { annotations, selectionBounds, snapTargetRects },
    } = { state: read(), derived: getEditorDerived() };
    if (status !== 'preview' || !selectionBounds) return;

    const point = event.point;
    const editReleasePoint = keyboardCursor.current ?? point;
    keyboardCursor.current = null;
    patch({ cursorPoint: point });

    if (annotationGesture && selection) {
      commitAnnotationGestureAtPoint(
        getCaptureSelectionLocalPoint(point, selection),
        event.shiftKey,
      );
      return;
    }

    if (annotationMoveGesture && selection) {
      const commit = planCaptureAnnotationMoveCommit({
        annotationHistory,
        annotationIndex: annotationMoveGesture.annotationIndex,
        startAnnotation: annotationMoveGesture.startAnnotation,
        startPoint: annotationMoveGesture.startPoint,
        localPoint: getCaptureSelectionLocalPoint(point, selection),
        constrainMove: event.shiftKey,
        resizeHandle: annotationMoveGesture.resizeHandle,
        selectionBounds: {
          x: 0,
          y: 0,
          width: selection.width,
          height: selection.height,
        },
      });
      patch({
        annotationMoveGesture: commit.annotationMoveGesture,
        draftAnnotation: commit.draftAnnotation,
        annotationHistory: commit.annotationHistory,
      });
      if (commit.selectedAnnotationIndex !== undefined) {
        patch({ selectedAnnotationIndex: commit.selectedAnnotationIndex });
      }
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
      patch({
        editGesture: commit.editGesture,
        selection: commit.selection,
        status: commit.status,
      });
      void renderEditorSelectionPreview(
        commit.previewRender.rect,
        commit.previewRender.annotations,
      );
    }
  }

  function handleCaptureWorkspaceEditorPreviewPointerDown(
    event: EditorPointerInput,
  ) {
    const {
      state: {
        activeAnnotationTool,
        annotationStyle,
        textDraft,
        textFontSize,
        selection,
        status,
      },
      derived: { annotations, selectionBounds },
    } = { state: read(), derived: getEditorDerived() };
    const plan = planCapturePreviewPointerDown(event, {
      status,
      hasSelection: selection !== null,
      hasSelectionBounds: Boolean(selectionBounds),
    });
    if (
      plan.type !== 'start-preview-interaction' ||
      !selection ||
      !selectionBounds
    ) {
      return;
    }

    if (textDraft) {
      event.preventDefault();
      event.stopPropagation();
      commitTextDraft();
      return;
    }

    event.stopPropagation();
    const point = event.point;
    patch({ cursorPoint: point });
    if (activeAnnotationTool) {
      patch({ selectedAnnotationIndex: null });
      const localPoint = getCaptureSelectionLocalPoint(point, selection);
      if (activeAnnotationTool === 'text') {
        if (textDraft) return;
        patch({
          textDraft: startTextAnnotationDraft(localPoint, textFontSize),
          textDraftAnnotationIndex: null,
        });
        return;
      }

      const start = planCaptureAnnotationToolStart({
        tool: activeAnnotationTool,
        localPoint,
        annotationStyle,
      });
      patch({
        selectedAnnotationIndex: start.selectedAnnotationIndex,
        annotationGesture: start.annotationGesture,
        draftAnnotation: start.draftAnnotation,
      });
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
      patch({
        selectedAnnotationIndex: existing.selectedAnnotationIndex,
        annotationStyle: existing.toolbarState.annotationStyle,
        textFontSize: existing.toolbarState.textFontSize,
      });
      if (existing.type === 'edit-text-annotation') {
        patch({
          annotationMoveGesture: existing.annotationMoveGesture,
          draftAnnotation: existing.draftAnnotation,
          textDraft: existing.textDraft,
          textDraftAnnotationIndex: existing.textDraftAnnotationIndex,
        });
        return;
      }
      patch({ annotationMoveGesture: existing.annotationMoveGesture });
      return;
    }

    const moveStart = planCapturePreviewSelectionMoveStart({
      point,
      selection,
      hasTextDraft: textDraft !== null,
      isCopyDoubleClick: false,
    });
    if (moveStart.type !== 'move-selection') return;
    patch({
      selectedAnnotationIndex: moveStart.selectedAnnotationIndex,
      annotationMoveGesture: moveStart.annotationMoveGesture,
      editGesture: moveStart.editGesture,
    });
  }

  function handleCaptureWorkspaceEditorResizePointerDown(
    handle: SelectionHandle,
    event: EditorPointerInput,
  ) {
    const {
      state: { selection, status },
      derived: { selectionBounds },
    } = { state: read(), derived: getEditorDerived() };
    if (status !== 'preview' || !selection || !selectionBounds) return;

    event.stopPropagation();
    const start = planCaptureSelectionResizeStart({
      point: event.point,
      selection,
      handle,
    });
    patch({ cursorPoint: start.cursorPoint, editGesture: start.editGesture });
  }

  function handleCaptureWorkspaceEditorWheel(event: EditorWheelInput) {
    const {
      state: { annotationGesture, annotationMoveGesture, status, textDraft },
      derived: { hasAnnotationEditingContext },
    } = { state: read(), derived: getEditorDerived() };
    const direction = planCapturePointerWheelSizeAdjustment(event, {
      status,
      hasTextDraft: textDraft !== null,
      hasAnnotationGesture: annotationGesture !== null,
      hasAnnotationMoveGesture: annotationMoveGesture !== null,
      hasAnnotationEditingContext,
    });
    if (!direction) return;
    event.preventDefault();
    adjustAnnotationSize(direction);
  }

  function handleCaptureWorkspaceEditorKeyDown(
    event: CaptureWorkspaceKeyboardEvent,
  ) {
    const state = read();
    const derived = getEditorDerived();
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
    const { annotations, cursorColor, hasAnnotationEditingContext } = derived;

    if (isMagnifierShortcut(event)) {
      event.preventDefault();
      patch({ isMagnifierRequested: true });
      return;
    }
    if (
      !textDraft &&
      derived.isMagnifierShown &&
      cursorColor &&
      isColorSampleCopyShortcut(event)
    ) {
      event.preventDefault();
      void copyCurrentColor();
      return;
    }
    if (
      !textDraft &&
      derived.isMagnifierShown &&
      cursorColor &&
      !event.repeat &&
      isColorSampleFormatToggleShortcut(event)
    ) {
      event.preventDefault();
      patch({
        colorSampleFormat: read().colorSampleFormat === 'hex' ? 'rgb' : 'hex',
      });
      return;
    }
    if (status !== 'preview') {
      return;
    }

    const undoRedoAction = getUndoRedoActionFromShortcut(event);
    const cursorNudgeDelta = getCursorNudgeDeltaFromShortcut(event);
    const cycledAnnotationTool = nextAnnotationToolFromCycleShortcut(
      event,
      activeAnnotationTool,
    );

    if (event.key === 'Escape') {
      event.preventDefault();
      void cancelSession();
    } else if (isClearAnnotationsShortcut(event)) {
      event.preventDefault();
      clearAnnotations();
    } else if (undoRedoAction) {
      event.preventDefault();
      if (undoRedoAction === 'undo') undoAnnotation();
      else redoAnnotation();
    } else if (
      selectedAnnotationIndex !== null &&
      isDeleteSelectedAnnotationShortcut(event)
    ) {
      event.preventDefault();
      deleteSelectedAnnotation();
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
      keyboardCursor.current = editNudge.keyboardEditCursorPoint;
      patch({
        cursorPoint: editNudge.cursorPoint,
        selection: editNudge.selection,
        editGesture: editNudge.editGesture,
        isRenderingOutput: editNudge.renderingOutput,
      });
    } else if (
      !textDraft &&
      getCaptureKeyboardToolbarAction(
        event,
        derived.isAnnotationToolbarVisible,
      ) === 'toggle'
    ) {
      event.preventDefault();
      patch({ isAnnotationToolbarVisible: !read().isAnnotationToolbarVisible });
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
        adjustAnnotationSize(direction);
      }
    } else if (
      !textDraft &&
      derived.isFillModeActive &&
      !annotationGesture &&
      !annotationMoveGesture &&
      isAnnotationFillToggleShortcut(event)
    ) {
      event.preventDefault();
      toggleAnnotationFill();
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
      patch({
        activeAnnotationTool: activation.activeAnnotationTool,
        selectedAnnotationIndex: activation.selectedAnnotationIndex,
        annotationGesture: activation.annotationGesture,
        annotationMoveGesture: activation.annotationMoveGesture,
        draftAnnotation: activation.draftAnnotation,
      });
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
        patch({ annotationHistory: nudge.annotationHistory });
      }
    } else if (!textDraft && !annotationGesture && !annotationMoveGesture) {
      const color = annotationColorFromShortcut(
        event,
        annotationColorPresets?.() ?? ANNOTATION_COLORS,
      );
      if (color) {
        event.preventDefault();
        selectAnnotationColor(color);
        return;
      }
      const tool = annotationToolFromShortcut(event);
      if (tool) {
        event.preventDefault();
        toggleAnnotationTool(tool);
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
        patch({ selection: preview.selection });
        void renderEditorSelectionPreview(preview.previewRender.rect);
      }
    }
  }

  function isArrowKey(key: string): key is ArrowKey {
    return ARROW_KEYS.includes(key as ArrowKey);
  }

  const pointerEvent = (input: CaptureWorkspacePointerInput) => {
    const bounds = getEditorDerived().selectionBounds;
    return bounds ? createEditorPointerInput(input) : null;
  };
  return {
    hasDismissibleLayer,
    captureInteraction: () => keyboardCursor.current,
    restoreInteraction: (point: Point | null) => {
      keyboardCursor.current = point;
    },
    pointerDown(input: CaptureWorkspacePointerInput) {
      const event = pointerEvent(input);
      if (!event) return false;
      if (input.source === 'preview')
        handleCaptureWorkspaceEditorPreviewPointerDown(event.event);
      else handleCaptureWorkspaceEditorPointerDown(event.event);
      return event.handled();
    },
    pointerMove(input: CaptureWorkspacePointerInput) {
      const event = pointerEvent(input);
      if (!event) return false;
      handleCaptureWorkspaceEditorPointerMove(event.event);
      return true;
    },
    pointerUp(input: CaptureWorkspacePointerInput) {
      const event = pointerEvent(input);
      if (!event) return false;
      handleCaptureWorkspaceEditorPointerUp(event.event);
      return true;
    },
    cancelPointer() {
      const state = read();
      if (
        !state.annotationGesture &&
        !state.draftAnnotation &&
        !state.annotationMoveGesture &&
        !state.editGesture
      )
        return false;
      keyboardCursor.current = null;
      patch({
        annotationGesture: null,
        draftAnnotation: null,
        annotationMoveGesture: null,
        editGesture: null,
      });
      return true;
    },
    resizePointerDown(
      handle: SelectionHandle,
      input: CaptureWorkspacePointerInput,
    ) {
      const event = pointerEvent(input);
      if (!event) return false;
      handleCaptureWorkspaceEditorResizePointerDown(handle, event.event);
      return event.handled();
    },
    resizeAnnotationPointerDown(
      handle: SelectionHandle,
      input: CaptureWorkspacePointerInput,
    ) {
      const state = read();
      if (
        state.status !== 'preview' ||
        !state.selection ||
        state.selectedAnnotationIndex === null
      )
        return false;
      const annotation =
        state.annotationHistory.annotations[state.selectedAnnotationIndex];
      if (
        !annotation ||
        (annotation.type !== 'rectangle' && annotation.type !== 'ellipse')
      )
        return false;
      patch({
        annotationMoveGesture: {
          annotationIndex: state.selectedAnnotationIndex,
          startPoint: {
            x: input.point.x - state.selection.x,
            y: input.point.y - state.selection.y,
          },
          startAnnotation: annotation,
          resizeHandle: handle,
        },
        draftAnnotation: annotation,
      });
      return true;
    },
    wheel(input: CaptureWorkspaceWheelInput) {
      let handled = false;
      handleCaptureWorkspaceEditorWheel({
        deltaY: input.deltaY,
        metaKey: input.metaKey ?? false,
        ctrlKey: input.ctrlKey ?? false,
        altKey: input.altKey ?? false,
        preventDefault: () => {
          handled = true;
        },
      });
      return handled;
    },
    keyDown(input: CaptureWorkspaceKeyInput) {
      let handled = false;
      handleCaptureWorkspaceEditorKeyDown({
        key: input.key,
        metaKey: input.metaKey ?? false,
        ctrlKey: input.ctrlKey ?? false,
        altKey: input.altKey ?? false,
        shiftKey: input.shiftKey ?? false,
        repeat: input.repeat ?? false,
        preventDefault: () => {
          handled = true;
        },
      });
      return handled;
    },
    commitTextDraft,
    commitTextDraftToHistory,
    updateTextDraftText(text: string) {
      const state = read();
      if (state.textDraft)
        patch({ textDraft: updateTextAnnotationDraft(state.textDraft, text) });
    },
    discardTextDraft() {
      patch({ textDraft: null, textDraftAnnotationIndex: null });
    },
    selectMoveTool() {
      patch({ activeAnnotationTool: null });
    },
    toggleAnnotationTool,
    applySelectedAnnotationStyle,
    updateTextDraftFontSize(fontSize: number) {
      const state = read();
      patch({
        textFontSize: fontSize,
        textDraft: state.textDraft ? { ...state.textDraft, fontSize } : null,
      });
    },
    undoAnnotation,
    redoAnnotation,
  };
}
function createEditorPointerInput(input: CaptureWorkspacePointerInput) {
  let wasHandled = false;
  const markHandled = () => {
    wasHandled = true;
  };
  const event: EditorPointerInput = {
    point: input.point,
    button: input.button ?? 0,
    detail: input.detail ?? 1,
    metaKey: input.metaKey ?? false,
    ctrlKey: input.ctrlKey ?? false,
    altKey: input.altKey ?? false,
    shiftKey: input.shiftKey ?? false,
    preventDefault: markHandled,
    stopPropagation: markHandled,
  };
  return { event, handled: () => wasHandled };
}
