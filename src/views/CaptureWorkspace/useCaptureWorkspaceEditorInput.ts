import {
  useCallback,
  useMemo,
  type Dispatch,
  type PointerEvent,
  type SetStateAction,
  type WheelEvent,
} from 'react';

import type { SelectionHistoryStep } from './captureActions';
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

interface EditorInputSetters {
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

interface EditorInputDerived {
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

interface EditorInputHost {
  cancelSession(): Promise<void>;
  refreshSession(): Promise<void>;
  renderSelectionPreview(rect: LogicalRect, annotations?: AnnotationCommand[], includeCursor?: boolean): Promise<void>;
  completeCandidateSelection(rect: LogicalRect, action: Parameters<CaptureWorkspaceKeyboardActions['completeCandidateSelection']>[1]): Promise<void>;
  completePreviewSelection: CaptureWorkspaceKeyboardActions['completePreviewSelection'];
  completeManualSelection(rect: LogicalRect): Promise<void>;
  pinSelection(): Promise<void>;
  copySelection(): Promise<void>;
  resetPreviewSelection(): void;
  selectFullCaptureArea(): void;
  restoreLastSelection(): void;
  restoreSelectionFromHistory(step: SelectionHistoryStep): void;
}

interface EditorInputActions {
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
  commitAnnotationGestureAtPoint(localPoint: Point, constrainGesture: boolean): boolean;
}

export function useCaptureWorkspaceEditorInput({
  state,
  refs,
  derived,
  setters,
  host,
  editor,
  scheduleSelectionOverlayPaint,
  syncHoverSelection,
}: {
  state: CaptureWorkspaceState;
  refs: CaptureWorkspaceKeyboardRefs;
  derived: EditorInputDerived;
  setters: EditorInputSetters;
  host: EditorInputHost;
  editor: EditorInputActions;
  scheduleSelectionOverlayPaint(draft?: LogicalRect | null, hover?: LogicalRect | null, active?: LogicalRect | null): void;
  syncHoverSelection(selection: LogicalRect | null): void;
}) {
  const keyboardDerived = useMemo<CaptureWorkspaceKeyboardDerivedState>(() => ({
    annotations: derived.annotations,
    captureCandidates: derived.captureCandidates,
    selectionBounds: derived.selectionBounds,
    hasAnnotationEditingContext: derived.hasAnnotationEditingContext,
    isAnnotationToolbarVisible: derived.isAnnotationToolbarVisible,
    isMagnifierShown: derived.isMagnifierShown,
    isFillModeActive: derived.isFillModeActive,
    cursorColor: derived.cursorColor,
  }), [derived]);
  const pointerDerived = useMemo<CaptureWorkspacePointerDerivedState>(() => ({
    annotations: derived.annotations,
    captureCandidates: derived.captureCandidates,
    selectionBounds: derived.selectionBounds,
    snapTargetRects: derived.snapTargetRects,
    hasAnnotationEditingContext: derived.hasAnnotationEditingContext,
    shouldTrackMagnifierCursor: derived.shouldTrackMagnifierCursor,
  }), [derived]);
  const keyboardActions = useMemo<CaptureWorkspaceKeyboardActions>(() => ({
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
  }), [derived.cursorColor, editor, host, scheduleSelectionOverlayPaint, setters, syncHoverSelection]);
  const pointerActions = useMemo<CaptureWorkspacePointerActions>(() => ({
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
    completeManualSelection: (rect) => void host.completeManualSelection(rect),
    pinSelection: host.pinSelection,
    setEditGesture: setters.setEditGesture,
    setAnnotationStyle: setters.setAnnotationStyle,
    setTextFontSize: setters.setTextFontSize,
    copySelection: host.copySelection,
    adjustAnnotationSize: editor.adjustAnnotationSize,
  }), [editor, host, scheduleSelectionOverlayPaint, setters, syncHoverSelection]);
  const context = useMemo<CaptureWorkspacePointerContext>(() => ({ state, refs, derived: pointerDerived, actions: pointerActions }), [pointerActions, pointerDerived, refs, state]);
  const onUnhandledKeyDown = useCallback((event: KeyboardEvent) => {
    handleCaptureWorkspaceKeyDown(event, { state, refs, derived: keyboardDerived, actions: keyboardActions });
  }, [keyboardActions, keyboardDerived, refs, state]);

  return {
    onUnhandledKeyDown,
    onRootPointerDown: useCallback((event: PointerEvent<HTMLDivElement>) => handleCaptureWorkspacePointerDown(event, context), [context]),
    onRootPointerMove: useCallback((event: PointerEvent<HTMLDivElement>) => handleCaptureWorkspacePointerMove(event, context), [context]),
    onRootPointerUp: useCallback((event: PointerEvent<HTMLDivElement>) => handleCaptureWorkspacePointerUp(event, context), [context]),
    onPreviewPointerDown: useCallback((event: PointerEvent<HTMLDivElement>) => handleCaptureWorkspacePreviewPointerDown(event, context), [context]),
    onResizeHandlePointerDown: useCallback((handle: SelectionHandle, event: PointerEvent<HTMLButtonElement>) => handleCaptureWorkspaceResizePointerDown(handle, event, context), [context]),
    onRootWheel: useCallback((event: WheelEvent<HTMLDivElement>) => handleCaptureWorkspaceWheel(event, context), [context]),
  };
}
