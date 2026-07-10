import {
  useCallback,
  useMemo,
  type Dispatch,
  type PointerEvent,
  type RefObject,
  type SetStateAction,
  type WheelEvent,
} from 'react';

import type {
  HoverSelectionCompletionAction,
  PreviewCaptureCompletionAction,
  SelectionHistoryStep,
} from './captureActions';
import type { AnnotationColor, AnnotationSizeDirection, AnnotationTool } from './annotationStyle';
import type { ColorSample } from './colorSampler';
import {
  handleCaptureWorkspaceKeyDown,
  type CaptureWorkspaceKeyboardActions,
  type CaptureWorkspaceKeyboardDerivedState,
  type CaptureWorkspaceKeyboardRefs,
} from './captureWorkspaceKeyboard';
import {
  handleCaptureWorkspacePointerDown,
  handleCaptureWorkspacePointerMove,
  handleCaptureWorkspacePointerUp,
  handleCaptureWorkspacePreviewPointerDown,
  handleCaptureWorkspaceResizePointerDown,
  handleCaptureWorkspaceWheel,
  type CaptureWorkspacePointerActions,
  type CaptureWorkspacePointerContext,
  type CaptureWorkspacePointerDerivedState,
} from './captureWorkspacePointer';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import type { SelectionHandle } from './selection';
import type { AnnotationCommand, LogicalRect, Point } from './types';

type WorkspaceSetter<Field extends keyof CaptureWorkspaceState> = Dispatch<
  SetStateAction<CaptureWorkspaceState[Field]>
>;

interface CaptureWorkspaceInputSetters {
  setStatus: WorkspaceSetter<'status'>;
  setCursorPoint: WorkspaceSetter<'cursorPoint'>;
  setSelection: WorkspaceSetter<'selection'>;
  setHoverSelection: WorkspaceSetter<'hoverSelection'>;
  setEditGesture: WorkspaceSetter<'editGesture'>;
  setActiveAnnotationTool: WorkspaceSetter<'activeAnnotationTool'>;
  setAnnotationGesture: WorkspaceSetter<'annotationGesture'>;
  setDraftAnnotation: WorkspaceSetter<'draftAnnotation'>;
  setSelectedAnnotationIndex: WorkspaceSetter<'selectedAnnotationIndex'>;
  setAnnotationMoveGesture: WorkspaceSetter<'annotationMoveGesture'>;
  setDraftSelectionMoveGesture: WorkspaceSetter<'draftSelectionMoveGesture'>;
  setTextDraft: WorkspaceSetter<'textDraft'>;
  setTextDraftAnnotationIndex: WorkspaceSetter<'textDraftAnnotationIndex'>;
  setAnnotationStyle: WorkspaceSetter<'annotationStyle'>;
  setTextFontSize: WorkspaceSetter<'textFontSize'>;
  setAnnotationHistory: WorkspaceSetter<'annotationHistory'>;
  setPreviewImageBase64: WorkspaceSetter<'previewImageBase64'>;
  setIsAnnotationToolbarVisible: WorkspaceSetter<'isAnnotationToolbarVisible'>;
  setColorSampleFormat: WorkspaceSetter<'colorSampleFormat'>;
  setIsMagnifierRequested: WorkspaceSetter<'isMagnifierRequested'>;
  setIncludeCapturedCursor: WorkspaceSetter<'includeCapturedCursor'>;
  setRenderingOutput(isRendering: boolean): void;
  setStartPointWithRef(point: Point | null): void;
}

interface CaptureWorkspaceInputDerivedState {
  annotations: AnnotationCommand[];
  captureCandidates: CaptureWorkspaceKeyboardDerivedState['captureCandidates'];
  selectionBounds: LogicalRect | null;
  snapTargetRects: LogicalRect[];
  hasAnnotationEditingContext: boolean;
  isAnnotationToolbarVisible: boolean;
  isMagnifierShown: boolean;
  isFillModeActive: boolean;
  cursorColor: ColorSample | null;
  shouldTrackMagnifierCursor: boolean;
}

interface CaptureWorkspaceInputHostActions {
  cancelSession(): Promise<void>;
  refreshSession(): Promise<void>;
  renderSelectionPreview(
    rect: LogicalRect,
    annotations?: AnnotationCommand[],
    includeCursor?: boolean,
  ): Promise<void>;
  completeCandidateSelection(
    rect: LogicalRect,
    action: HoverSelectionCompletionAction,
  ): Promise<void>;
  completePreviewSelection(
    action: PreviewCaptureCompletionAction,
    options?: { commitTextDraft?: boolean; guardCompletion?: boolean },
  ): Promise<void>;
  completeManualSelection(rect: LogicalRect): Promise<void>;
  pinSelection(): Promise<void>;
  copySelection(): Promise<void>;
  resetPreviewSelection(): void;
  selectFullCaptureArea(): void;
  restoreLastSelection(): void;
  restoreSelectionFromHistory(step: SelectionHistoryStep): void;
}

interface CaptureWorkspaceInputEditorActions {
  dismissCaptureLayer(): void;
  clearAnnotations(): void;
  undoPolylineGesturePoint(): boolean;
  undoAnnotation(): void;
  redoAnnotation(): void;
  deleteSelectedAnnotation(): void;
  copyCurrentColor(cursorColor: ColorSample | null): Promise<void>;
  adjustAnnotationSize(direction: AnnotationSizeDirection): void;
  toggleAnnotationFill(): void;
  selectAnnotationColor(color: AnnotationColor): void;
  toggleAnnotationTool(tool: AnnotationTool): void;
  commitTextDraft(): void;
  commitAnnotationGestureAtPoint(
    localPoint: Point,
    constrainGesture: boolean,
  ): boolean;
}

interface UseCaptureWorkspaceInputControllerOptions {
  state: CaptureWorkspaceState;
  refs: CaptureWorkspaceKeyboardRefs;
  derived: CaptureWorkspaceInputDerivedState;
  setters: CaptureWorkspaceInputSetters;
  host: CaptureWorkspaceInputHostActions;
  editor: CaptureWorkspaceInputEditorActions;
  isRenderingOutputRef: RefObject<boolean>;
  scheduleSelectionOverlayPaint(
    draftSelection?: LogicalRect | null,
    hoverSelection?: LogicalRect | null,
    activeSelection?: LogicalRect | null,
  ): void;
  syncHoverSelection(selection: LogicalRect | null): void;
}

export function useCaptureWorkspaceInputController({
  derived,
  editor,
  host,
  isRenderingOutputRef,
  refs,
  scheduleSelectionOverlayPaint,
  setters,
  state,
  syncHoverSelection,
}: UseCaptureWorkspaceInputControllerOptions) {
  const keyboardDerived = useMemo<CaptureWorkspaceKeyboardDerivedState>(
    () => ({
      annotations: derived.annotations,
      captureCandidates: derived.captureCandidates,
      selectionBounds: derived.selectionBounds,
      hasAnnotationEditingContext: derived.hasAnnotationEditingContext,
      isAnnotationToolbarVisible: derived.isAnnotationToolbarVisible,
      isMagnifierShown: derived.isMagnifierShown,
      isFillModeActive: derived.isFillModeActive,
      cursorColor: derived.cursorColor,
    }),
    [derived],
  );
  const pointerDerived = useMemo<CaptureWorkspacePointerDerivedState>(
    () => ({
      annotations: derived.annotations,
      captureCandidates: derived.captureCandidates,
      selectionBounds: derived.selectionBounds,
      snapTargetRects: derived.snapTargetRects,
      hasAnnotationEditingContext: derived.hasAnnotationEditingContext,
      shouldTrackMagnifierCursor: derived.shouldTrackMagnifierCursor,
    }),
    [derived],
  );

  const keyboardActions = useMemo<CaptureWorkspaceKeyboardActions>(
    () => ({
      dismissCaptureLayer: editor.dismissCaptureLayer,
      refreshSession: host.refreshSession,
      setIncludeCapturedCursor: setters.setIncludeCapturedCursor,
      clearPreviewImage: () => setters.setPreviewImageBase64(null),
      renderSelectionPreview: host.renderSelectionPreview,
      setIsMagnifierRequested: setters.setIsMagnifierRequested,
      clearAnnotations: editor.clearAnnotations,
      undoPolylineGesturePoint: editor.undoPolylineGesturePoint,
      undoAnnotation: editor.undoAnnotation,
      redoAnnotation: editor.redoAnnotation,
      deleteSelectedAnnotation: editor.deleteSelectedAnnotation,
      copyCurrentColor: () => editor.copyCurrentColor(derived.cursorColor),
      setColorSampleFormat: setters.setColorSampleFormat,
      restoreSelectionFromHistory: host.restoreSelectionFromHistory,
      restoreLastSelection: host.restoreLastSelection,
      setCursorPoint: setters.setCursorPoint,
      setSelection: setters.setSelection,
      scheduleSelectionOverlayPaint,
      setPreviewImageBase64: setters.setPreviewImageBase64,
      setRenderingOutput: setters.setRenderingOutput,
      setEditGesture: setters.setEditGesture,
      syncHoverSelection,
      selectFullCaptureArea: host.selectFullCaptureArea,
      completeCandidateSelection: host.completeCandidateSelection,
      setIsAnnotationToolbarVisible: setters.setIsAnnotationToolbarVisible,
      completePreviewSelection: host.completePreviewSelection,
      adjustAnnotationSize: editor.adjustAnnotationSize,
      toggleAnnotationFill: editor.toggleAnnotationFill,
      setActiveAnnotationTool: setters.setActiveAnnotationTool,
      setSelectedAnnotationIndex: setters.setSelectedAnnotationIndex,
      setAnnotationGesture: setters.setAnnotationGesture,
      setAnnotationMoveGesture: setters.setAnnotationMoveGesture,
      setDraftAnnotation: setters.setDraftAnnotation,
      selectAnnotationColor: editor.selectAnnotationColor,
      toggleAnnotationTool: editor.toggleAnnotationTool,
      setDraftSelectionMoveGesture: setters.setDraftSelectionMoveGesture,
      setAnnotationHistory: setters.setAnnotationHistory,
    }),
    [
      derived.cursorColor,
      editor,
      host,
      scheduleSelectionOverlayPaint,
      setters,
      syncHoverSelection,
    ],
  );

  const pointerActions = useMemo<CaptureWorkspacePointerActions>(
    () => ({
      commitTextDraft: editor.commitTextDraft,
      commitAnnotationGestureAtPoint: editor.commitAnnotationGestureAtPoint,
      dismissCaptureLayer: editor.dismissCaptureLayer,
      resetPreviewSelection: host.resetPreviewSelection,
      cancelSession: host.cancelSession,
      setCursorPoint: setters.setCursorPoint,
      setStartPointWithRef: setters.setStartPointWithRef,
      setSelection: setters.setSelection,
      setHoverSelection: setters.setHoverSelection,
      scheduleSelectionOverlayPaint,
      setPreviewImageBase64: setters.setPreviewImageBase64,
      setRenderingOutput: setters.setRenderingOutput,
      setStatus: setters.setStatus,
      setActiveAnnotationTool: setters.setActiveAnnotationTool,
      setAnnotationGesture: setters.setAnnotationGesture,
      setDraftAnnotation: setters.setDraftAnnotation,
      setSelectedAnnotationIndex: setters.setSelectedAnnotationIndex,
      setAnnotationMoveGesture: setters.setAnnotationMoveGesture,
      setDraftSelectionMoveGesture: setters.setDraftSelectionMoveGesture,
      setTextDraft: setters.setTextDraft,
      setTextDraftAnnotationIndex: setters.setTextDraftAnnotationIndex,
      setAnnotationHistory: setters.setAnnotationHistory,
      syncHoverSelection,
      renderSelectionPreview: host.renderSelectionPreview,
      completeManualSelection: (rect) => {
        void host.completeManualSelection(rect);
      },
      pinSelection: host.pinSelection,
      setEditGesture: setters.setEditGesture,
      setAnnotationStyle: setters.setAnnotationStyle,
      setTextFontSize: setters.setTextFontSize,
      copySelection: host.copySelection,
      adjustAnnotationSize: editor.adjustAnnotationSize,
    }),
    [
      editor,
      host,
      scheduleSelectionOverlayPaint,
      setters,
      syncHoverSelection,
    ],
  );
  const pointerContext = useMemo<CaptureWorkspacePointerContext>(
    () => ({
      state,
      refs,
      derived: pointerDerived,
      actions: pointerActions,
    }),
    [pointerActions, pointerDerived, refs, state],
  );

  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      handleCaptureWorkspaceKeyDown(event, {
        state,
        refs,
        derived: keyboardDerived,
        actions: keyboardActions,
      });
    },
    [keyboardActions, keyboardDerived, refs, state],
  );
  const onReleaseMagnifierRequest = useCallback(() => {
    setters.setIsMagnifierRequested(false);
  }, [setters]);
  const onFinishDraftSelectionMove = useCallback(() => {
    setters.setDraftSelectionMoveGesture(null);
  }, [setters]);
  const keyboardHostEvents = useMemo(
    () => ({
      isActive: state.status !== 'idle',
      status: state.status,
      isRenderingOutputRef,
      hasDraftSelectionMoveGesture: state.draftSelectionMoveGesture !== null,
      onKeyDown,
      onReleaseMagnifierRequest,
      onFinishDraftSelectionMove,
      onCancelSession: host.cancelSession,
    }),
    [
      host.cancelSession,
      isRenderingOutputRef,
      onFinishDraftSelectionMove,
      onKeyDown,
      onReleaseMagnifierRequest,
      state.draftSelectionMoveGesture,
      state.status,
    ],
  );

  const onRootPointerDown = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      handleCaptureWorkspacePointerDown(event, pointerContext);
    },
    [pointerContext],
  );
  const onRootPointerMove = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      handleCaptureWorkspacePointerMove(event, pointerContext);
    },
    [pointerContext],
  );
  const onRootPointerUp = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      handleCaptureWorkspacePointerUp(event, pointerContext);
    },
    [pointerContext],
  );
  const onPreviewPointerDown = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      handleCaptureWorkspacePreviewPointerDown(event, pointerContext);
    },
    [pointerContext],
  );
  const onResizeHandlePointerDown = useCallback(
    (
      handle: SelectionHandle,
      event: PointerEvent<HTMLButtonElement>,
    ) => {
      handleCaptureWorkspaceResizePointerDown(handle, event, pointerContext);
    },
    [pointerContext],
  );
  const onRootWheel = useCallback(
    (event: WheelEvent<HTMLDivElement>) => {
      handleCaptureWorkspaceWheel(event, pointerContext);
    },
    [pointerContext],
  );

  return {
    keyboardHostEvents,
    pointerHandlers: {
      onRootPointerDown,
      onRootPointerMove,
      onRootPointerUp,
      onRootWheel,
      onPreviewPointerDown,
      onResizeHandlePointerDown,
    },
  };
}
