import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  type Dispatch,
  type SetStateAction,
} from 'react';

import type { CaptureWorkspacePlatformRuntime } from '../../application/capture-workspace/platformRuntime';
import {
  clearAnnotationHistory,
  removeAnnotationFromHistory,
  redoAnnotationHistory,
  undoAnnotationHistory,
} from './annotationHistory';
import type {
  AnnotationColor,
  AnnotationSizeDirection,
  AnnotationStyle,
  AnnotationTool,
} from './annotationStyle';
import {
  applyStyleToSelectedAnnotationHistory,
  commitCaptureEditorTextDraft,
  completeCaptureEditorGesture,
  getCaptureEditorDismissAction,
  planCaptureAnnotationColorSelection,
  planCaptureAnnotationFillToggle,
  planCaptureAnnotationSizeAdjustment,
  planCaptureAnnotationToolActivation,
  undoPolylineCaptureGesture,
} from './captureEditorRuntime';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import { colorSampleToClipboardText, type ColorSample } from './colorSampler';
import { updateTextAnnotationDraft } from './textAnnotationDraft';
import type { AnnotationCommand, LogicalRect, Point } from './types';

type WorkspaceSetter<Field extends keyof CaptureWorkspaceState> = Dispatch<
  SetStateAction<CaptureWorkspaceState[Field]>
>;

interface CaptureWorkspaceEditorSetters {
  setStatus: WorkspaceSetter<'status'>;
  setError: WorkspaceSetter<'error'>;
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
}

interface CaptureWorkspaceEditorDerivedState {
  annotations: AnnotationCommand[];
  canUndoAnnotation: boolean;
  canRedoAnnotation: boolean;
  isTextSizingActive: boolean;
  isFillModeActive: boolean;
}

interface CaptureWorkspaceEditorHostActions {
  renderSelectionPreview(
    rect: LogicalRect,
    annotations?: AnnotationCommand[],
  ): Promise<void>;
  cancelSession(): Promise<void>;
}

interface UseCaptureWorkspaceEditorControllerOptions {
  state: CaptureWorkspaceState;
  derived: CaptureWorkspaceEditorDerivedState;
  setters: CaptureWorkspaceEditorSetters;
  host: CaptureWorkspaceEditorHostActions;
  runtime: CaptureWorkspacePlatformRuntime;
}

export function useCaptureWorkspaceEditorController({
  derived,
  host,
  runtime,
  setters,
  state,
}: UseCaptureWorkspaceEditorControllerOptions) {
  const textDraftInputRef = useRef<HTMLTextAreaElement | null>(null);

  const commitTextDraftToHistory = useCallback(() => {
    const commitResult = commitCaptureEditorTextDraft({
      annotationHistory: state.annotationHistory,
      selectedAnnotationIndex: state.selectedAnnotationIndex,
      textDraft: state.textDraft,
      annotationStyle: state.annotationStyle,
      textDraftAnnotationIndex: state.textDraftAnnotationIndex,
    });

    setters.setTextDraft(commitResult.textDraft);
    setters.setTextDraftAnnotationIndex(commitResult.textDraftAnnotationIndex);
    if (
      commitResult.selectedAnnotationIndex !== state.selectedAnnotationIndex
    ) {
      setters.setSelectedAnnotationIndex(commitResult.selectedAnnotationIndex);
    }
    if (commitResult.annotationHistory !== state.annotationHistory) {
      setters.setAnnotationHistory(commitResult.annotationHistory);
    }

    return commitResult.annotationHistory;
  }, [
    setters,
    state.annotationHistory,
    state.annotationStyle,
    state.selectedAnnotationIndex,
    state.textDraft,
    state.textDraftAnnotationIndex,
  ]);

  const undoAnnotation = useCallback(() => {
    if (!state.selection || !derived.canUndoAnnotation) return;

    const nextHistory = undoAnnotationHistory(state.annotationHistory);
    setters.setSelectedAnnotationIndex(null);
    setters.setAnnotationMoveGesture(null);
    setters.setAnnotationHistory(nextHistory);
    void host.renderSelectionPreview(state.selection, nextHistory.annotations);
  }, [derived.canUndoAnnotation, host, setters, state.annotationHistory, state.selection]);

  const redoAnnotation = useCallback(() => {
    if (!state.selection || !derived.canRedoAnnotation) return;

    const nextHistory = redoAnnotationHistory(state.annotationHistory);
    setters.setSelectedAnnotationIndex(null);
    setters.setAnnotationMoveGesture(null);
    setters.setAnnotationHistory(nextHistory);
    void host.renderSelectionPreview(state.selection, nextHistory.annotations);
  }, [derived.canRedoAnnotation, host, setters, state.annotationHistory, state.selection]);

  const undoPolylineGesturePoint = useCallback(() => {
    if (
      !state.annotationGesture ||
      state.annotationGesture.tool !== 'polyline' ||
      !state.selection
    ) {
      return false;
    }

    const nextDraft = undoPolylineCaptureGesture({
      gesture: state.annotationGesture,
      selection: state.selection,
      cursorPoint: state.cursorPoint,
      annotationStyle: state.annotationStyle,
    });
    if (!nextDraft) {
      setters.setAnnotationGesture(null);
      setters.setDraftAnnotation(null);
      return true;
    }

    setters.setAnnotationGesture(nextDraft.gesture);
    setters.setDraftAnnotation(nextDraft.draftAnnotation);
    return true;
  }, [
    setters,
    state.annotationGesture,
    state.annotationStyle,
    state.cursorPoint,
    state.selection,
  ]);

  const clearAnnotations = useCallback(() => {
    if (!state.selection) return;

    const nextHistory = clearAnnotationHistory(state.annotationHistory);
    if (nextHistory === state.annotationHistory) return;

    setters.setActiveAnnotationTool(null);
    setters.setAnnotationGesture(null);
    setters.setDraftAnnotation(null);
    setters.setSelectedAnnotationIndex(null);
    setters.setAnnotationMoveGesture(null);
    setters.setTextDraft(null);
    setters.setTextDraftAnnotationIndex(null);
    setters.setAnnotationHistory(nextHistory);
    void host.renderSelectionPreview(state.selection, nextHistory.annotations);
  }, [host, setters, state.annotationHistory, state.selection]);

  const deleteSelectedAnnotation = useCallback(() => {
    if (!state.selection || state.selectedAnnotationIndex === null) return;

    const nextHistory = removeAnnotationFromHistory(
      state.annotationHistory,
      state.selectedAnnotationIndex,
    );
    if (nextHistory === state.annotationHistory) return;

    setters.setSelectedAnnotationIndex(null);
    setters.setAnnotationMoveGesture(null);
    setters.setAnnotationHistory(nextHistory);
    void host.renderSelectionPreview(state.selection, nextHistory.annotations);
  }, [
    host,
    setters,
    state.annotationHistory,
    state.selectedAnnotationIndex,
    state.selection,
  ]);

  const applySelectedAnnotationStyle = useCallback(
    (nextStyle: AnnotationStyle, nextTextFontSize: number) => {
      setters.setAnnotationStyle(nextStyle);
      setters.setTextFontSize(nextTextFontSize);

      if (!state.selection) return;

      const nextHistory = applyStyleToSelectedAnnotationHistory({
        annotationHistory: state.annotationHistory,
        annotations: derived.annotations,
        selectedAnnotationIndex: state.selectedAnnotationIndex,
        textDraftActive: state.textDraft !== null,
        nextStyle,
        nextTextFontSize,
      });
      if (nextHistory === state.annotationHistory) return;

      setters.setAnnotationHistory(nextHistory);
      void host.renderSelectionPreview(state.selection, nextHistory.annotations);
    },
    [
      derived.annotations,
      host,
      setters,
      state.annotationHistory,
      state.selectedAnnotationIndex,
      state.selection,
      state.textDraft,
    ],
  );

  const adjustAnnotationSize = useCallback(
    (direction: AnnotationSizeDirection) => {
      if (state.textDraft) return;

      const nextToolbarState = planCaptureAnnotationSizeAdjustment({
        annotationStyle: state.annotationStyle,
        textFontSize: state.textFontSize,
        direction,
        isTextSizingActive: derived.isTextSizingActive,
      });
      applySelectedAnnotationStyle(
        nextToolbarState.annotationStyle,
        nextToolbarState.textFontSize,
      );
    },
    [
      applySelectedAnnotationStyle,
      derived.isTextSizingActive,
      state.annotationStyle,
      state.textDraft,
      state.textFontSize,
    ],
  );

  const selectAnnotationColor = useCallback(
    (color: AnnotationColor) => {
      if (state.textDraft) return;

      const nextToolbarState = planCaptureAnnotationColorSelection({
        annotationStyle: state.annotationStyle,
        textFontSize: state.textFontSize,
        color,
      });
      applySelectedAnnotationStyle(
        nextToolbarState.annotationStyle,
        nextToolbarState.textFontSize,
      );
    },
    [
      applySelectedAnnotationStyle,
      state.annotationStyle,
      state.textDraft,
      state.textFontSize,
    ],
  );

  const toggleAnnotationFill = useCallback(() => {
    if (state.textDraft || !derived.isFillModeActive) return;

    const nextToolbarState = planCaptureAnnotationFillToggle({
      annotationStyle: state.annotationStyle,
      textFontSize: state.textFontSize,
    });
    applySelectedAnnotationStyle(
      nextToolbarState.annotationStyle,
      nextToolbarState.textFontSize,
    );
  }, [
    applySelectedAnnotationStyle,
    derived.isFillModeActive,
    state.annotationStyle,
    state.textDraft,
    state.textFontSize,
  ]);

  const commitTextDraft = useCallback(() => {
    const nextHistory = commitTextDraftToHistory();
    if (state.selection && nextHistory !== state.annotationHistory) {
      void host.renderSelectionPreview(
        state.selection,
        nextHistory.annotations,
      );
    }
  }, [commitTextDraftToHistory, host, state.annotationHistory, state.selection]);

  const updateTextDraftText = useCallback(
    (text: string) => {
      setters.setTextDraft((draft) =>
        draft ? updateTextAnnotationDraft(draft, text) : draft,
      );
    },
    [setters],
  );

  const discardTextDraft = useCallback(() => {
    setters.setTextDraft(null);
    setters.setTextDraftAnnotationIndex(null);
    if (state.textDraftAnnotationIndex !== null && state.selection) {
      void host.renderSelectionPreview(state.selection, derived.annotations);
    }
  }, [
    derived.annotations,
    host,
    setters,
    state.selection,
    state.textDraftAnnotationIndex,
  ]);

  const commitAnnotationGestureAtPoint = useCallback(
    (localPoint: Point, constrainGesture: boolean) => {
      if (!state.annotationGesture || !state.selection) return false;

      const commitResult = completeCaptureEditorGesture({
        annotationHistory: state.annotationHistory,
        selectedAnnotationIndex: state.selectedAnnotationIndex,
        annotationGesture: state.annotationGesture,
        localPoint,
        annotationStyle: state.annotationStyle,
        constrainGesture,
      });
      if (!commitResult) return false;

      setters.setAnnotationGesture(commitResult.annotationGesture);
      setters.setDraftAnnotation(commitResult.draftAnnotation);
      if (
        commitResult.selectedAnnotationIndex !== state.selectedAnnotationIndex
      ) {
        setters.setSelectedAnnotationIndex(commitResult.selectedAnnotationIndex);
      }
      if (commitResult.annotationHistory !== state.annotationHistory) {
        setters.setAnnotationHistory(commitResult.annotationHistory);
        void host.renderSelectionPreview(
          state.selection,
          commitResult.annotationHistory.annotations,
        );
      }
      return true;
    },
    [
      host,
      setters,
      state.annotationGesture,
      state.annotationHistory,
      state.annotationStyle,
      state.selectedAnnotationIndex,
      state.selection,
    ],
  );

  const dismissCaptureLayer = useCallback(() => {
    const dismissAction = getCaptureEditorDismissAction({
      hasTextDraft: state.textDraft !== null,
      hasAnnotationMoveGesture: state.annotationMoveGesture !== null,
      hasDraftSelectionMoveGesture: state.draftSelectionMoveGesture !== null,
      hasSelectedAnnotation: state.selectedAnnotationIndex !== null,
      hasActiveAnnotationTool: state.activeAnnotationTool !== null,
      hasAnnotationGesture: state.annotationGesture !== null,
    });

    if (dismissAction === 'clear-text-draft') {
      setters.setTextDraft(null);
      setters.setTextDraftAnnotationIndex(null);
    } else if (dismissAction === 'revert-annotation-move') {
      setters.setAnnotationMoveGesture(null);
      setters.setDraftAnnotation(null);
      if (state.selection) {
        void host.renderSelectionPreview(state.selection, derived.annotations);
      }
    } else if (dismissAction === 'clear-draft-selection-move') {
      setters.setDraftSelectionMoveGesture(null);
    } else if (dismissAction === 'clear-selected-annotation') {
      setters.setSelectedAnnotationIndex(null);
    } else if (dismissAction === 'clear-active-annotation-tool') {
      setters.setActiveAnnotationTool(null);
      setters.setAnnotationGesture(null);
      setters.setDraftAnnotation(null);
    } else {
      void host.cancelSession();
    }
  }, [derived.annotations, host, setters, state]);

  const toggleAnnotationTool = useCallback(
    (nextTool: AnnotationTool) => {
      const nextHistory = commitTextDraftToHistory();
      if (state.selection && nextHistory !== state.annotationHistory) {
        void host.renderSelectionPreview(
          state.selection,
          nextHistory.annotations,
        );
      }

      const toolActivation = planCaptureAnnotationToolActivation({
        currentTool: state.activeAnnotationTool,
        nextTool,
        selectedAnnotationIndex: state.selectedAnnotationIndex,
        clearSelectedAnnotation: false,
        toggle: true,
      });
      setters.setActiveAnnotationTool(toolActivation.activeAnnotationTool);
      setters.setSelectedAnnotationIndex(toolActivation.selectedAnnotationIndex);
      setters.setAnnotationGesture(toolActivation.annotationGesture);
      setters.setAnnotationMoveGesture(toolActivation.annotationMoveGesture);
      setters.setDraftAnnotation(toolActivation.draftAnnotation);
    },
    [commitTextDraftToHistory, host, setters, state],
  );

  const selectMoveTool = useCallback(() => {
    setters.setActiveAnnotationTool(null);
  }, [setters]);

  const updateTextDraftFontSize = useCallback(
    (fontSize: number) => {
      setters.setTextFontSize(fontSize);
      setters.setTextDraft((draft) =>
        draft ? { ...draft, fontSize } : draft,
      );
    },
    [setters],
  );

  const copyCurrentColor = useCallback(
    async (cursorColor: ColorSample | null) => {
      if (!cursorColor) return;

      try {
        await runtime.clipboard.copyText(
          colorSampleToClipboardText(cursorColor, state.colorSampleFormat),
        );
      } catch (err) {
        setters.setError(err instanceof Error ? err.message : String(err));
        setters.setStatus('error');
      }
    },
    [setters, state.colorSampleFormat],
  );

  useEffect(() => {
    if (!state.textDraft) return;

    requestAnimationFrame(() => {
      textDraftInputRef.current?.focus();
    });
  }, [state.textDraft]);

  const actions = useMemo(
    () => ({
      undoAnnotation,
      redoAnnotation,
      undoPolylineGesturePoint,
      clearAnnotations,
      deleteSelectedAnnotation,
      applySelectedAnnotationStyle,
      adjustAnnotationSize,
      selectAnnotationColor,
      toggleAnnotationFill,
      commitTextDraft,
      updateTextDraftText,
      discardTextDraft,
      commitAnnotationGestureAtPoint,
      dismissCaptureLayer,
      toggleAnnotationTool,
      selectMoveTool,
      updateTextDraftFontSize,
      copyCurrentColor,
    }),
    [
      adjustAnnotationSize,
      applySelectedAnnotationStyle,
      clearAnnotations,
      commitAnnotationGestureAtPoint,
      commitTextDraft,
      copyCurrentColor,
      deleteSelectedAnnotation,
      discardTextDraft,
      dismissCaptureLayer,
      redoAnnotation,
      selectAnnotationColor,
      selectMoveTool,
      toggleAnnotationFill,
      toggleAnnotationTool,
      undoAnnotation,
      undoPolylineGesturePoint,
      updateTextDraftFontSize,
      updateTextDraftText,
    ],
  );

  return {
    textDraftInputRef,
    commitTextDraftToHistory,
    actions,
  };
}
