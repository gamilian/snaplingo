import type {
  AnnotationCommand,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from '../../domain/capture';
import type {
  HoverSelectionCompletionAction,
} from '../../views/CaptureWorkspace/captureActions';
import {
  canToggleCapturedCursor,
  getCandidateCycleDirectionFromShortcut,
  getCursorNudgeDeltaFromShortcut,
  getHoverSelectionCompletionActionFromShortcut,
  getPreviewCaptureCompletionActionFromShortcut,
  getSelectionHistoryStepFromShortcut,
  isRefreshCaptureShortcut,
  isRestoreLastSelectionShortcut,
  isSelectAllCaptureShortcut,
  isToggleCapturedCursorShortcut,
} from '../../views/CaptureWorkspace/captureActions';
import {
  getCaptureKeyboardKeyUpAction,
  planCaptureKeyboardBlur,
} from '../../views/CaptureWorkspace/captureKeyboardHostRuntime';
import {
  buildCaptureCandidates,
  getBestCandidateAtPoint,
} from '../../views/CaptureWorkspace/captureCandidates';
import {
  getPrimaryCaptureCompletionActionForMode,
  planCandidateSelectionCompletion,
  planManualSelectionCompletion,
  type CaptureRuntimeEffect,
} from '../../views/CaptureWorkspace/captureInteractionRuntime';
import {
  recordSuccessfulCaptureSelection,
  restoreCaptureSelectionFromHistory,
  restoreLastSuccessfulCaptureSelection,
  type CaptureSelectionStorage,
} from '../../views/CaptureWorkspace/captureHostRuntime';
import { printBase64PngImage } from '../../views/CaptureWorkspace/capturePrint';
import {
  clearAnnotationHistory,
  emptyAnnotationHistory,
  redoAnnotationHistory,
  removeAnnotationFromHistory,
  undoAnnotationHistory,
} from '../../views/CaptureWorkspace/annotationHistory';
import {
  type AnnotationColor,
  type AnnotationSizeDirection,
  type AnnotationStyle,
  type AnnotationTool,
} from '../../views/CaptureWorkspace/annotationStyle';
import {
  applyStyleToSelectedAnnotationHistory,
  commitCaptureEditorTextDraft,
  completeCaptureEditorGesture,
  getCaptureEditorDismissAction,
  planCaptureAnnotationColorSelection,
  planCaptureAnnotationFillToggle,
  planCaptureAnnotationSizeAdjustment,
  planCaptureAnnotationToolActivation,
  planCaptureManualSelectionTransition,
  undoPolylineCaptureGesture,
} from '../../views/CaptureWorkspace/captureEditorRuntime';
import { getCaptureWorkspaceDerivedState } from '../../views/CaptureWorkspace/captureWorkspaceDerived';
import {
  handleCaptureWorkspaceEditorKeyDown,
  type CaptureWorkspaceKeyboardEditorActions,
  type CaptureWorkspaceKeyboardEditorContext,
} from '../../views/CaptureWorkspace/captureWorkspaceKeyboard';
import {
  handleCaptureWorkspaceEditorPointerDown,
  handleCaptureWorkspaceEditorPointerMove,
  handleCaptureWorkspaceEditorPointerUp,
  handleCaptureWorkspaceEditorPreviewPointerDown,
  handleCaptureWorkspaceEditorResizePointerDown,
  handleCaptureWorkspaceEditorWheel,
  type CaptureWorkspacePointerEditorActions,
  type CaptureWorkspacePointerEditorContext,
  type CaptureWorkspacePointerEvent,
} from '../../views/CaptureWorkspace/captureWorkspacePointer';
import {
  createInitialCaptureWorkspaceState,
  type CaptureWorkspaceState,
} from '../../views/CaptureWorkspace/captureWorkspaceState';
import { colorSampleToClipboardText } from '../../views/CaptureWorkspace/colorSampler';
import { updateTextAnnotationDraft } from '../../views/CaptureWorkspace/textAnnotationDraft';
import {
  normalizeSelection,
  snapPointToRects,
} from '../../views/CaptureWorkspace/selection';
import type { SelectionHandle } from '../../views/CaptureWorkspace/selection';
import {
  planCaptureDraftSelectionCommit,
  planCaptureDraftSelectionKeyboardNudge,
  planCaptureDraftSelectionPointerMove,
  planCaptureDraftSelectionStart,
  planCaptureHoverSelectionCycle,
  planCaptureSelectionCursorKeyboardNudge,
} from '../../views/CaptureWorkspace/captureSelectionRuntime';
import { shouldRevealCaptureWindow } from '../../views/CaptureWorkspace/captureWindowVisibility';
import {
  getCurrentMonitorBounds,
  getVirtualDesktopBounds,
} from '../../views/CaptureWorkspace/virtualDesktop';
import { normalizeOcrText } from '../../utils/ocrTextProcessing';
import type {
  CaptureWorkspaceRuntime,
  CaptureWorkspacePointerInput,
  CaptureWorkspaceKeyInput,
  CaptureWorkspaceRuntimePlatform,
} from './types';

const MIN_SELECTION_SIZE = 10;

type RuntimeState = CaptureWorkspaceState;

interface SnapshotHydration {
  generation: number;
  sessionId: string;
  promise: Promise<void>;
}

interface PreviewRenderRequest {
  revision: number;
  generation: number;
  sessionId: string;
  rect: LogicalRect;
  annotations: AnnotationCommand[];
  includeCursor: boolean;
}

interface PreviewRenderScheduler {
  generation: number;
  sessionId: string;
  revision: number;
  queuedPreview: PreviewRenderRequest | null;
  drain: Promise<void> | null;
}

interface TerminalOutputOperation {
  id: number;
  generation: number;
  sessionId: string;
}

interface CaptureWorkspaceRuntimeHost {
  resetInteraction(): void;
  resetSession(): void;
  prepareSurface(): void | Promise<void>;
  scheduleSelectionOverlayPaint?(): void;
}

interface CaptureWorkspaceRuntimeKeyboard {
  target: {
    addEventListener(
      type: 'keydown',
      listener: (event: KeyboardEvent) => void,
    ): void;
    addEventListener(
      type: 'keyup',
      listener: (event: KeyboardEvent) => void,
    ): void;
    addEventListener(type: 'blur', listener: () => void): void;
    removeEventListener(
      type: 'keydown',
      listener: (event: KeyboardEvent) => void,
    ): void;
    removeEventListener(
      type: 'keyup',
      listener: (event: KeyboardEvent) => void,
    ): void;
    removeEventListener(type: 'blur', listener: () => void): void;
  };
}

interface CaptureFrontendPerfState {
  mode: CaptureMode;
  sessionId: string | null;
  startMs: number;
  hasLoggedImagesReady: boolean;
}

export function createCaptureWorkspaceRuntime({
  platform,
  host,
  keyboard,
  onInactive,
  screenshotSavePath,
  storage,
}: {
  platform: CaptureWorkspaceRuntimePlatform;
  host?: CaptureWorkspaceRuntimeHost;
  keyboard?: CaptureWorkspaceRuntimeKeyboard;
  onInactive?: () => void | Promise<void>;
  screenshotSavePath?: () => string | undefined;
  storage?: CaptureSelectionStorage;
}): CaptureWorkspaceRuntime {
  let state = createInitialState();
  let generation = 0;
  let hydratedSessionId: string | null = null;
  let snapshotHydration: SnapshotHydration | null = null;
  const listeners = new Set<() => void>();
  let hasRevealed = false;
  let revealAttempt: { key: string; promise: Promise<void> } | null = null;
  let perfState: CaptureFrontendPerfState | null = null;
  let hasKeyboardAdjustedDraft = false;
  const cursorPointRef = { current: null as Point | null };
  const keyboardEditCursorPointRef = { current: null as Point | null };
  let previewScheduler: PreviewRenderScheduler | null = null;
  let terminalOutputSequence = 0;
  let terminalOutputOperation: TerminalOutputOperation | null = null;

  const markPerf = (event: string, sessionId?: string | null) => {
    const perf = perfState;
    if (!perf) return;

    void platform.commands.logCaptureFrontendPerf({
      event,
      mode: perf.mode,
      sessionId: sessionId ?? perf.sessionId,
      elapsedMs: performance.now() - perf.startMs,
    }).catch(() => undefined);
  };

  const patch = (next: Partial<RuntimeState>) => {
    state = { ...state, ...next };
    if ('cursorPoint' in next) cursorPointRef.current = next.cursorPoint ?? null;
    listeners.forEach((listener) => listener());
  };

  const launch = (operation: () => Promise<unknown>) => {
    const actionGeneration = generation;
    const reportFailure = (error: unknown) => {
      if (generation === actionGeneration) {
        patch({ status: 'error', error: errorMessage(error) });
      }
    };

    try {
      void operation().catch(reportFailure);
    } catch (error) {
      reportFailure(error);
    }
  };

  const detachPreviewScheduler = () => {
    if (previewScheduler) {
      previewScheduler.queuedPreview = null;
      previewScheduler.revision += 1;
      previewScheduler = null;
    }
  };

  const hasCurrentTerminalOutput = () =>
    terminalOutputOperation?.generation === generation &&
    terminalOutputOperation.sessionId === state.session?.id;

  const resetSession = () => {
    detachPreviewScheduler();
    terminalOutputOperation = null;
    state = createInitialState(state.mode, state);
    cursorPointRef.current = null;
    keyboardEditCursorPointRef.current = null;
    hasKeyboardAdjustedDraft = false;
    hydratedSessionId = null;
    snapshotHydration = null;
    host?.resetSession();
    listeners.forEach((listener) => listener());
  };

  const createNativeSessionCancellation = (sessionId: string) => {
    let cancellation: Promise<void> | null = null;

    return () => {
      cancellation ??= platform.commands.cancelCaptureSession(sessionId);
      return cancellation;
    };
  };

  const finishSession = async (
    actionGeneration: number,
    sessionId: string,
    cancelNativeSession: () => Promise<void>,
  ) => {
    if (generation !== actionGeneration) {
      if (state.session?.id !== sessionId) await cancelNativeSession();
      return;
    }

    await (onInactive ? onInactive() : platform.dismiss());
    if (generation !== actionGeneration) {
      if (state.session?.id !== sessionId) await cancelNativeSession();
      return;
    }

    resetSession();
    await cancelNativeSession();
  };

  const executeEffect = async (
    effect: CaptureRuntimeEffect,
    sessionId: string,
    rect: LogicalRect,
    annotations: AnnotationCommand[],
    includeCursor: boolean,
    actionGeneration: number,
    cancelNativeSession: () => Promise<void>,
  ) => {
    const cancelIfStale = async () => {
      if (
        generation === actionGeneration &&
        state.session?.id === sessionId
      ) {
        return false;
      }

      if (state.session?.id !== sessionId) await cancelNativeSession();
      return true;
    };

    if (effect.type === 'output-capture') {
      if (effect.action === 'print') {
        const imageBase64 = await platform.commands.renderCaptureOutput({
          sessionId,
          rect,
          annotations,
          ...(includeCursor ? { includeCursor: true } : {}),
        });
        if (await cancelIfStale()) return;
        await printBase64PngImage(imageBase64);
        return;
      }

      const action =
        effect.action === 'save'
          ? {
              type: 'save' as const,
              path: await platform.commands.defaultCaptureSavePath(),
            }
          : effect.action === 'quick-save'
            ? {
                type: 'save' as const,
                path: await platform.commands.quickCaptureSavePath(
                  screenshotSavePath?.(),
                ),
              }
            : effect.action === 'pin'
              ? { type: 'pin' as const }
              : { type: 'copy' as const };
      if (await cancelIfStale()) return;
      await platform.commands.outputCapture({
        sessionId,
        rect,
        annotations,
        ...(includeCursor ? { includeCursor: true } : {}),
        action,
      });
      return;
    }

    if (effect.type === 'run-ocr') {
      const result = await platform.commands.runCaptureOcr(sessionId, rect);
      if (await cancelIfStale()) return;
      const text = normalizeOcrText(result.text);

      if (effect.target === 'translation-window') {
        await platform.commands.openCaptureTranslationResultWindow(text);
        return;
      }

      if (effect.target === 'ocr-window') {
        const imageBase64 = await platform.commands.renderCaptureOutput({
          sessionId,
          rect,
          annotations,
        });
        if (await cancelIfStale()) return;
        await platform.commands.openCaptureOcrResultWindow(text, imageBase64);
        return;
      }

      await platform.commands.copyTextToClipboard(text);
      return;
    }

    if (effect.type === 'record-selection') {
      if (storage) {
        recordSuccessfulCaptureSelection(storage, effect.action, rect);
      }
      return;
    }

    if (effect.type === 'finish-session') {
      await finishSession(actionGeneration, sessionId, cancelNativeSession);
      return;
    }

  };

  const runCompletionEffects = async (
    rect: LogicalRect,
    effects: CaptureRuntimeEffect[],
    annotations: AnnotationCommand[] = [],
    includeCursor = false,
  ) => {
    const session = state.session;
    if (!session) return;
    if (
      terminalOutputOperation?.generation === generation &&
      terminalOutputOperation.sessionId === session.id
    ) {
      return;
    }
    const actionGeneration = generation;
    const operation: TerminalOutputOperation = {
      id: ++terminalOutputSequence,
      generation: actionGeneration,
      sessionId: session.id,
    };
    terminalOutputOperation = operation;
    const cancelNativeSession = createNativeSessionCancellation(session.id);

    patch({
      selection: rect,
      hoverSelection: null,
      isRenderingOutput: true,
      error: null,
    });

    try {
      for (const effect of effects) {
        await executeEffect(
          effect,
          session.id,
          rect,
          annotations,
          includeCursor,
          actionGeneration,
          cancelNativeSession,
        );
        if (
          generation !== actionGeneration ||
          state.session?.id !== session.id
        ) {
          if (state.session?.id !== session.id) await cancelNativeSession();
          return;
        }
      }
    } catch (error) {
      if (generation === actionGeneration && state.session?.id === session.id) {
        patch({ status: 'error', error: errorMessage(error) });
      } else if (state.session?.id !== session.id) {
        await cancelNativeSession().catch(() => undefined);
      }
    } finally {
      if (terminalOutputOperation === operation) {
        terminalOutputOperation = null;
      }
      if (
        terminalOutputOperation === null &&
        generation === actionGeneration &&
        state.session?.id === session.id
      ) {
        patch({ isRenderingOutput: Boolean(previewScheduler?.drain) });
      }
    }
  };

  const renderSelectionPreview = async (
    rect: LogicalRect,
    annotations: AnnotationCommand[] = [],
    includeCursor = false,
  ) => {
    const session = state.session;
    if (!session) return;
    let scheduler = previewScheduler;
    if (
      !scheduler ||
      scheduler.generation !== generation ||
      scheduler.sessionId !== session.id
    ) {
      scheduler = {
        generation,
        sessionId: session.id,
        revision: 0,
        queuedPreview: null,
        drain: null,
      };
      previewScheduler = scheduler;
    }
    const request: PreviewRenderRequest = {
      revision: ++scheduler.revision,
      generation,
      sessionId: session.id,
      rect,
      annotations,
      includeCursor,
    };
    scheduler.queuedPreview = request;

    patch({
      status: 'preview',
      selection: rect,
      hoverSelection: null,
      previewImageBase64: null,
      isRenderingOutput: true,
      error: null,
    });

    if (!scheduler.drain) {
      const work = (async () => {
        while (scheduler.queuedPreview) {
          const current = scheduler.queuedPreview;
          scheduler.queuedPreview = null;
          const cancelNativeSession = createNativeSessionCancellation(
            current.sessionId,
          );
          if (
            generation !== current.generation ||
            state.session?.id !== current.sessionId
          ) {
            if (state.session?.id !== current.sessionId) {
              await cancelNativeSession().catch(() => undefined);
            }
            continue;
          }

          try {
            const previewImageBase64 =
              await platform.commands.renderCaptureOutput({
                sessionId: current.sessionId,
                rect: current.rect,
                annotations: current.annotations,
                ...(current.includeCursor ? { includeCursor: true } : {}),
              });
            if (
              generation === current.generation &&
              state.session?.id === current.sessionId &&
              scheduler.revision === current.revision
            ) {
              patch({ previewImageBase64 });
            } else if (state.session?.id !== current.sessionId) {
              await cancelNativeSession().catch(() => undefined);
            }
          } catch (error) {
            if (
              generation === current.generation &&
              state.session?.id === current.sessionId &&
              scheduler.revision === current.revision
            ) {
              patch({ status: 'error', error: errorMessage(error) });
            } else if (state.session?.id !== current.sessionId) {
              await cancelNativeSession().catch(() => undefined);
            }
          }
        }
      })();
      let drain!: Promise<void>;
      drain = work.finally(() => {
        if (scheduler.drain === drain) {
          scheduler.drain = null;
        }
        if (previewScheduler === scheduler) {
          patch({ isRenderingOutput: hasCurrentTerminalOutput() });
        }
      });
      scheduler.drain = drain;
    }
    await scheduler.drain;
  };

  const getEditorDerived = () =>
    getCaptureWorkspaceDerivedState({
      state,
      hydratedCaptureSessionId: hydratedSessionId,
      toolbarGap: 0,
      toolbarSize: { width: 0, height: 0 },
    });

  const hasDismissibleLayer = () =>
    state.textDraft !== null ||
    state.annotationGesture !== null ||
    state.annotationMoveGesture !== null ||
    state.draftSelectionMoveGesture !== null ||
    state.selectedAnnotationIndex !== null ||
    state.activeAnnotationTool !== null;

  const renderEditorSelectionPreview = (
    rect: LogicalRect,
    annotations: AnnotationCommand[] = state.annotationHistory.annotations,
    includeCursor =
      state.includeCapturedCursor && canToggleCapturedCursor(state.session),
  ) => renderSelectionPreview(rect, annotations, includeCursor);

  const renderCurrentSelection = (annotations: AnnotationCommand[]) => {
    const selection = state.selection;
    if (!selection) return;
    launch(() => renderEditorSelectionPreview(selection, annotations));
  };

  const commitTextDraftToHistory = () => {
    const result = commitCaptureEditorTextDraft({
      annotationHistory: state.annotationHistory,
      selectedAnnotationIndex: state.selectedAnnotationIndex,
      textDraft: state.textDraft,
      annotationStyle: state.annotationStyle,
      textDraftAnnotationIndex: state.textDraftAnnotationIndex,
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
    const previousHistory = state.annotationHistory;
    const nextHistory = applyStyleToSelectedAnnotationHistory({
      annotationHistory: previousHistory,
      annotations: derived.annotations,
      selectedAnnotationIndex: state.selectedAnnotationIndex,
      textDraftActive: state.textDraft !== null,
      nextStyle,
      nextTextFontSize,
    });
    patch({
      annotationStyle: nextStyle,
      textFontSize: nextTextFontSize,
      annotationHistory: nextHistory,
    });
    if (nextHistory !== previousHistory) {
      renderCurrentSelection(nextHistory.annotations);
    }
  };

  const adjustAnnotationSize = (direction: AnnotationSizeDirection) => {
    if (state.textDraft) return;
    const next = planCaptureAnnotationSizeAdjustment({
      annotationStyle: state.annotationStyle,
      textFontSize: state.textFontSize,
      direction,
      isTextSizingActive: getEditorDerived().isTextSizingActive,
    });
    applySelectedAnnotationStyle(next.annotationStyle, next.textFontSize);
  };

  const selectAnnotationColor = (color: AnnotationColor) => {
    if (state.textDraft) return;
    const next = planCaptureAnnotationColorSelection({
      annotationStyle: state.annotationStyle,
      textFontSize: state.textFontSize,
      color,
    });
    applySelectedAnnotationStyle(next.annotationStyle, next.textFontSize);
  };

  const toggleAnnotationFill = () => {
    if (state.textDraft || !getEditorDerived().isFillModeActive) return;
    const next = planCaptureAnnotationFillToggle({
      annotationStyle: state.annotationStyle,
      textFontSize: state.textFontSize,
    });
    applySelectedAnnotationStyle(next.annotationStyle, next.textFontSize);
  };

  const undoAnnotation = () => {
    const nextHistory = undoAnnotationHistory(state.annotationHistory);
    if (nextHistory === state.annotationHistory) return;
    patch({
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      annotationHistory: nextHistory,
    });
    renderCurrentSelection(nextHistory.annotations);
  };

  const redoAnnotation = () => {
    const nextHistory = redoAnnotationHistory(state.annotationHistory);
    if (nextHistory === state.annotationHistory) return;
    patch({
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      annotationHistory: nextHistory,
    });
    renderCurrentSelection(nextHistory.annotations);
  };

  const undoPolylineGesturePoint = () => {
    if (
      !state.annotationGesture ||
      state.annotationGesture.tool !== 'polyline' ||
      !state.selection
    ) {
      return false;
    }
    const next = undoPolylineCaptureGesture({
      gesture: state.annotationGesture,
      selection: state.selection,
      cursorPoint: state.cursorPoint,
      annotationStyle: state.annotationStyle,
    });
    patch(
      next
        ? {
            annotationGesture: next.gesture,
            draftAnnotation: next.draftAnnotation,
          }
        : { annotationGesture: null, draftAnnotation: null },
    );
    return true;
  };

  const clearAnnotations = () => {
    const nextHistory = clearAnnotationHistory(state.annotationHistory);
    if (nextHistory === state.annotationHistory) return;
    patch({
      activeAnnotationTool: null,
      annotationGesture: null,
      draftAnnotation: null,
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      textDraft: null,
      textDraftAnnotationIndex: null,
      annotationHistory: nextHistory,
    });
    renderCurrentSelection(nextHistory.annotations);
  };

  const deleteSelectedAnnotation = () => {
    if (state.selectedAnnotationIndex === null) return;
    const nextHistory = removeAnnotationFromHistory(
      state.annotationHistory,
      state.selectedAnnotationIndex,
    );
    if (nextHistory === state.annotationHistory) return;
    patch({
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      annotationHistory: nextHistory,
    });
    renderCurrentSelection(nextHistory.annotations);
  };

  const commitAnnotationGestureAtPoint = (
    localPoint: Point,
    constrainGesture: boolean,
  ) => {
    const previousHistory = state.annotationHistory;
    const result = completeCaptureEditorGesture({
      annotationHistory: previousHistory,
      selectedAnnotationIndex: state.selectedAnnotationIndex,
      annotationGesture: state.annotationGesture,
      localPoint,
      annotationStyle: state.annotationStyle,
      constrainGesture,
    });
    if (!result) return false;
    patch({
      annotationHistory: result.annotationHistory,
      selectedAnnotationIndex: result.selectedAnnotationIndex,
      annotationGesture: result.annotationGesture,
      draftAnnotation: result.draftAnnotation,
    });
    if (result.annotationHistory !== previousHistory) {
      renderCurrentSelection(result.annotationHistory.annotations);
    }
    return true;
  };

  const dismissCaptureLayer = () => {
    const action = getCaptureEditorDismissAction({
      hasTextDraft: state.textDraft !== null,
      hasAnnotationMoveGesture: state.annotationMoveGesture !== null,
      hasDraftSelectionMoveGesture: state.draftSelectionMoveGesture !== null,
      hasSelectedAnnotation: state.selectedAnnotationIndex !== null,
      hasActiveAnnotationTool: state.activeAnnotationTool !== null,
      hasAnnotationGesture: state.annotationGesture !== null,
    });
    if (action === 'clear-text-draft') {
      patch({ textDraft: null, textDraftAnnotationIndex: null });
    } else if (action === 'revert-annotation-move') {
      patch({ annotationMoveGesture: null, draftAnnotation: null });
      renderCurrentSelection(state.annotationHistory.annotations);
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
      launch(cancelSession);
    }
  };

  const toggleAnnotationTool = (nextTool: AnnotationTool) => {
    const previousHistory = state.annotationHistory;
    const nextHistory = commitTextDraftToHistory();
    if (nextHistory !== previousHistory) {
      renderCurrentSelection(nextHistory.annotations);
    }
    const activation = planCaptureAnnotationToolActivation({
      currentTool: state.activeAnnotationTool,
      nextTool,
      selectedAnnotationIndex: state.selectedAnnotationIndex,
      clearSelectedAnnotation: false,
      toggle: true,
    });
    patch(activation);
  };

  const editorActions: CaptureWorkspacePointerEditorActions &
    CaptureWorkspaceKeyboardEditorActions = {
    commitTextDraft() {
      const previousHistory = state.annotationHistory;
      const nextHistory = commitTextDraftToHistory();
      if (nextHistory !== previousHistory) {
        renderCurrentSelection(nextHistory.annotations);
      }
    },
    commitAnnotationGestureAtPoint,
    dismissCaptureLayer,
    adjustAnnotationSize,
    renderSelectionPreview: renderEditorSelectionPreview,
    setCursorPoint: (cursorPoint) => patch({ cursorPoint }),
    setSelection: (selection) => patch({ selection }),
    scheduleSelectionOverlayPaint: () => host?.scheduleSelectionOverlayPaint?.(),
    setPreviewImageBase64: (previewImageBase64) => patch({ previewImageBase64 }),
    setRenderingOutput: (isRenderingOutput) => patch({ isRenderingOutput }),
    setStatus: (status) => patch({ status }),
    setAnnotationGesture: (annotationGesture) => patch({ annotationGesture }),
    setDraftAnnotation: (draftAnnotation) => patch({ draftAnnotation }),
    setSelectedAnnotationIndex: (selectedAnnotationIndex) =>
      patch({ selectedAnnotationIndex }),
    setAnnotationMoveGesture: (annotationMoveGesture) =>
      patch({ annotationMoveGesture }),
    setTextDraft: (textDraft) => patch({ textDraft }),
    setTextDraftAnnotationIndex: (textDraftAnnotationIndex) =>
      patch({ textDraftAnnotationIndex }),
    setAnnotationHistory: (annotationHistory) => patch({ annotationHistory }),
    setEditGesture: (editGesture) => patch({ editGesture }),
    setAnnotationStyle: (annotationStyle) => patch({ annotationStyle }),
    setTextFontSize: (textFontSize) => patch({ textFontSize }),
    setIsMagnifierRequested: (isMagnifierRequested) =>
      patch({ isMagnifierRequested }),
    clearAnnotations,
    undoPolylineGesturePoint,
    undoAnnotation,
    redoAnnotation,
    deleteSelectedAnnotation,
    async copyCurrentColor() {
      if (!state.cursorColor) return;
      try {
        await platform.clipboard.copyText(
          colorSampleToClipboardText(
            state.cursorColor,
            state.colorSampleFormat,
          ),
        );
      } catch (error) {
        patch({ status: 'error', error: errorMessage(error) });
      }
    },
    setColorSampleFormat: (updater) =>
      patch({ colorSampleFormat: updater(state.colorSampleFormat) }),
    setIsAnnotationToolbarVisible: (updater) =>
      patch({
        isAnnotationToolbarVisible:
          typeof updater === 'function'
            ? updater(state.isAnnotationToolbarVisible)
            : updater,
      }),
    toggleAnnotationFill,
    setActiveAnnotationTool: (activeAnnotationTool) =>
      patch({ activeAnnotationTool }),
    selectAnnotationColor,
    toggleAnnotationTool,
  };

  const editorContext = (): CaptureWorkspacePointerEditorContext => {
    const derived = getEditorDerived();
    return {
      state,
      refs: { cursorPointRef, keyboardEditCursorPointRef },
      derived: {
        annotations: derived.annotations,
        selectionBounds: derived.selectionBounds,
        snapTargetRects: derived.snapTargetRects,
        hasAnnotationEditingContext: derived.hasAnnotationEditingContext,
        shouldTrackMagnifierCursor: derived.shouldTrackMagnifierCursor,
      },
      actions: editorActions,
    };
  };

  const keyboardEditorContext = (): CaptureWorkspaceKeyboardEditorContext => {
    const derived = getEditorDerived();
    return {
      state,
      refs: { keyboardEditCursorPointRef },
      derived: {
        annotations: derived.annotations,
        selectionBounds: derived.selectionBounds,
        hasAnnotationEditingContext: derived.hasAnnotationEditingContext,
        isAnnotationToolbarVisible: state.isAnnotationToolbarVisible,
        isMagnifierShown: derived.isMagnifierShown,
        isFillModeActive: derived.isFillModeActive,
        cursorColor: state.cursorColor,
      },
      actions: editorActions,
    };
  };

  const completeCandidateSelection = async (
    rect: LogicalRect,
    action: HoverSelectionCompletionAction =
      getPrimaryCaptureCompletionActionForMode(
        state.mode,
      ) as HoverSelectionCompletionAction,
  ) => {
    await runCompletionEffects(
      rect,
      planCandidateSelectionCompletion(action),
    );
  };

  const completeManualSelection = async (rect: LogicalRect) => {
    const completion = planManualSelectionCompletion(state.mode);
    const transition = planCaptureManualSelectionTransition({ rect, completion });
    if (transition.type === 'preview') {
      patch({
        ...transition.nextState,
        isRenderingOutput: false,
      });
      await renderSelectionPreview(rect);
      return;
    }

    const { renderingOutput: _renderingOutput, ...nextState } =
      transition.nextState;
    patch({ ...nextState, isRenderingOutput: false });
    await runCompletionEffects(rect, transition.effects);
  };

  const restoreLastSelection = () => {
    if (!storage || !state.session) return false;
    let restored = false;
    restoreLastSuccessfulCaptureSelection({
      storage,
      selectionBounds: getVirtualDesktopBounds(state.session.monitors),
      minSelectionSize: MIN_SELECTION_SIZE,
      completeSelection: (rect) => {
        restored = true;
        launch(() => completeManualSelection(rect));
      },
    });
    return restored;
  };

  const restoreSelectionHistory = (step: 'previous' | 'next') => {
    if (!storage || !state.session) return false;
    let restored = false;
    restoreCaptureSelectionFromHistory({
      storage,
      currentSelection: state.selection,
      step,
      selectionBounds: getVirtualDesktopBounds(state.session.monitors),
      minSelectionSize: MIN_SELECTION_SIZE,
      completeSelection: (rect) => {
        restored = true;
        launch(() => completeManualSelection(rect));
      },
    });
    return restored;
  };

  const cancelSession = async () => {
    const actionGeneration = ++generation;
    const sessionId = state.session?.id;
    if (!sessionId) {
      try {
        await (onInactive ? onInactive() : platform.dismiss());
        if (generation === actionGeneration) resetSession();
      } catch (error) {
        if (generation === actionGeneration) {
          patch({ status: 'error', error: errorMessage(error) });
        }
      }
      return;
    }
    const cancelNativeSession = createNativeSessionCancellation(sessionId);

    try {
      await finishSession(actionGeneration, sessionId, cancelNativeSession);
    } catch (error) {
      if (generation === actionGeneration) {
        patch({ status: 'error', error: errorMessage(error) });
      } else {
        await cancelNativeSession().catch(() => undefined);
      }
    }
  };

  const actions: CaptureWorkspaceRuntime['actions'] = {
    async connectHost() {
      const unlisten: Array<() => void> = [];
      const disposeAll = () => {
        for (const dispose of unlisten.splice(0).reverse()) {
          try {
            dispose();
          } catch {
            // Keep releasing the remaining host subscriptions.
          }
        }
      };
      const handleKeyDown = (event: KeyboardEvent) => {
        if (actions.keyDown(event)) {
          event.preventDefault();
        }
      };
      const handleKeyUp = (event: KeyboardEvent) => {
        const action = getCaptureKeyboardKeyUpAction(event, {
          hasDraftSelectionMoveGesture:
            state.draftSelectionMoveGesture !== null,
        });
        if (action === 'release-magnifier-request') {
          patch({ isMagnifierRequested: false });
        } else if (action === 'finish-draft-selection-move') {
          event.preventDefault();
          patch({ draftSelectionMoveGesture: null });
        }
      };
      const handleBlur = () => {
        const plan = planCaptureKeyboardBlur({
          status: state.status,
          isRenderingOutput: state.isRenderingOutput,
        });
        if (plan.releaseMagnifierRequest) {
          patch({ isMagnifierRequested: false });
        }
        if (plan.cancelSession) {
          launch(cancelSession);
        }
      };

      if (keyboard) {
        keyboard.target.addEventListener('keydown', handleKeyDown);
        keyboard.target.addEventListener('keyup', handleKeyUp);
        keyboard.target.addEventListener('blur', handleBlur);
        unlisten.push(() => {
          keyboard.target.removeEventListener('keydown', handleKeyDown);
          keyboard.target.removeEventListener('keyup', handleKeyUp);
          keyboard.target.removeEventListener('blur', handleBlur);
        });
      }

      try {
        unlisten.push(
          await platform.onHotkeyTriggered((launch) =>
            actions.startSession(launch.mode, launch.sessionId),
          ),
        );
        unlisten.push(await platform.onCancelRequested(() => cancelSession()));
        unlisten.push(
          await platform.onCopyRequested(async () => {
            if (state.status === 'preview' && state.selection) {
              await runCompletionEffects(
                state.selection,
                planCandidateSelectionCompletion('copy'),
                commitTextDraftToHistory().annotations,
                state.includeCapturedCursor &&
                  canToggleCapturedCursor(state.session),
              );
              return;
            }

            if (
              state.status === 'selecting' &&
              !state.startPoint &&
              state.hoverSelection &&
              !state.textDraft
            ) {
              await completeCandidateSelection(state.hoverSelection, 'copy');
            }
          }),
        );
      } catch (error) {
        disposeAll();
        patch({ status: 'error', error: errorMessage(error) });
        return () => undefined;
      }

      return disposeAll;
    },

    async updateHostReadiness(imagesReady) {
      const sessionId = state.session?.id ?? null;
      const readinessGeneration = generation;
      const revealKey = `${readinessGeneration}:${sessionId ?? ''}`;
      if (
        !shouldRevealCaptureWindow({
          status: state.status,
          hasSession: Boolean(sessionId),
          hasCaptureImagesReady: imagesReady,
          hasRevealed,
        })
      ) {
        return;
      }

      if (!revealAttempt || revealAttempt.key !== revealKey) {
        const isCurrent = () =>
          generation === readinessGeneration && state.session?.id === sessionId;
        const promise = (async () => {
          try {
            await platform.prepareForReveal();
            if (!isCurrent()) return;
            await host?.prepareSurface();
            if (!isCurrent()) return;
            await platform.reveal();
            if (!isCurrent()) return;
            hasRevealed = true;
            if (sessionId) markPerf('revealed', sessionId);
          } catch (error) {
            if (isCurrent()) {
              patch({ status: 'error', error: errorMessage(error) });
            }
          }
        })();
        revealAttempt = { key: revealKey, promise };
      }
      const attempt = revealAttempt;
      await attempt.promise;
      if (revealAttempt === attempt && !hasRevealed) {
        revealAttempt = null;
      }

      const perf = perfState;
      if (
        perf &&
        imagesReady &&
        !perf.hasLoggedImagesReady &&
        perf.sessionId === sessionId
      ) {
        perf.hasLoggedImagesReady = true;
        markPerf('images_ready', sessionId);
      }
    },

    async startSession(mode, requestedSessionId) {
      const actionGeneration = ++generation;
      detachPreviewScheduler();
      terminalOutputOperation = null;
      hasKeyboardAdjustedDraft = false;
      host?.resetInteraction();
      hasRevealed = false;
      revealAttempt = null;
      perfState = {
        mode,
        sessionId: null,
        startMs: performance.now(),
        hasLoggedImagesReady: false,
      };
      markPerf('start_session', null);
      state = {
        ...createInitialState(mode, state),
        status: 'loading',
      };
      listeners.forEach((listener) => listener());
      hydratedSessionId = null;
      snapshotHydration = null;

      try {
        const session = requestedSessionId
          ? await platform.commands.getCaptureSession(requestedSessionId)
          : await platform.commands.createCaptureSession();
        if (generation !== actionGeneration) {
          await platform.commands.cancelCaptureSession(session.id);
          return;
        }
        if (perfState) perfState.sessionId = session.id;
        markPerf('session_loaded', session.id);

        const cursorPoint =
          session.captured_cursor?.logical_position ??
          (await platform.commands
            .currentCaptureCursorPosition(session.id)
            .catch(() => null));
        if (generation !== actionGeneration) {
          await platform.commands.cancelCaptureSession(session.id);
          return;
        }

        patch({
          status: 'selecting',
          session,
          cursorPoint,
          hoverSelection: cursorPoint
            ? getBestCandidateAtPoint(
                buildCaptureCandidates(session.monitors, session.candidates),
                cursorPoint,
              )?.rect ?? null
            : null,
        });
      } catch (error) {
        if (generation === actionGeneration) {
          patch({ status: 'error', error: errorMessage(error) });
        }
      }
    },

    async refreshSession() {
      const previousSessionId = state.session?.id;
      if (!previousSessionId) return;
      const actionGeneration = ++generation;
      detachPreviewScheduler();
      terminalOutputOperation = null;
      hasKeyboardAdjustedDraft = false;
      host?.resetInteraction();
      hasRevealed = false;
      revealAttempt = null;
      state = {
        ...createInitialState(state.mode, state),
        status: 'loading',
      };
      listeners.forEach((listener) => listener());
      hydratedSessionId = null;
      snapshotHydration = null;
      let createdSession: CaptureSessionView | null = null;
      let adoptedCreatedSession = false;

      try {
        const session = await platform.commands.createCaptureSession();
        createdSession = session;
        await platform.commands.cancelCaptureSession(previousSessionId);
        if (generation !== actionGeneration) return;
        const cursorPoint =
          session.captured_cursor?.logical_position ??
          (await platform.commands
            .currentCaptureCursorPosition(session.id)
            .catch(() => null));
        if (generation !== actionGeneration) return;
        adoptedCreatedSession = true;
        patch({
          status: 'selecting',
          session,
          cursorPoint,
          hoverSelection: cursorPoint
            ? getBestCandidateAtPoint(
                buildCaptureCandidates(session.monitors, session.candidates),
                cursorPoint,
              )?.rect ?? null
            : null,
        });
      } catch (error) {
        if (generation === actionGeneration) {
          patch({ status: 'error', error: errorMessage(error) });
        }
      } finally {
        if (createdSession && !adoptedCreatedSession) {
          await platform.commands
            .cancelCaptureSession(createdSession.id)
            .catch(() => undefined);
        }
      }
    },

    cancelSession,
    renderSelectionPreview,
    completeCandidateSelection,
    completeManualSelection,
    async completePreviewSelection(
      action,
      rect,
      annotations,
      includeCursor = state.includeCapturedCursor,
    ) {
      const previewAnnotations =
        annotations ??
        (action === 'ocr'
          ? state.annotationHistory.annotations
          : commitTextDraftToHistory().annotations);
      await runCompletionEffects(
        rect,
        planCandidateSelectionCompletion(action),
        previewAnnotations,
        includeCursor,
      );
    },
    resetPreview() {
      generation += 1;
      detachPreviewScheduler();
      terminalOutputOperation = null;
      hasKeyboardAdjustedDraft = false;
      snapshotHydration = null;
      hydratedSessionId = null;
      host?.resetInteraction();
      patch({
        editGesture: null,
        activeAnnotationTool: null,
        annotationGesture: null,
        draftAnnotation: null,
        selectedAnnotationIndex: null,
        annotationMoveGesture: null,
        draftSelectionMoveGesture: null,
        textDraft: null,
        textDraftAnnotationIndex: null,
        annotationHistory: emptyAnnotationHistory(),
        isAnnotationToolbarVisible: true,
        cursorColor: null,
        colorSampleFormat: 'hex',
        isMagnifierRequested: false,
        status: 'selecting',
        selection: null,
        hoverSelection: null,
        previewImageBase64: null,
        isRenderingOutput: false,
        error: null,
      });
    },

    pointerDown(input) {
      if (!state.session) return false;
      const pointer = pointerInput(input);
      const {
        altKey = false,
        button = 0,
        ctrlKey = false,
        detail = 0,
        metaKey = false,
        point,
        shiftKey = false,
        source = 'root',
      } = pointer;
      if (state.status === 'preview') {
        if (source === 'preview' && button === 1 && state.selection) {
          const selection = state.selection;
          launch(() =>
            runCompletionEffects(
              selection,
              planCandidateSelectionCompletion('pin'),
              commitTextDraftToHistory().annotations,
              state.includeCapturedCursor,
            ),
          );
          return true;
        }
        if (
          source === 'preview' &&
          button === 0 &&
          detail >= 2 &&
          !metaKey &&
          !ctrlKey &&
          !altKey &&
          !shiftKey &&
          !state.textDraft &&
          state.selection
        ) {
          const selection = state.selection;
          launch(() =>
            runCompletionEffects(
              selection,
              planCandidateSelectionCompletion('copy'),
              state.annotationHistory.annotations,
              state.includeCapturedCursor,
            ),
          );
          return true;
        }
        if (source === 'root' && button === 2) {
          if (hasDismissibleLayer()) {
            const context = editorContext();
            if (!context.derived.selectionBounds) return false;
            const editorEvent = createEditorPointerEvent(
              pointer,
              context.derived.selectionBounds,
            );
            handleCaptureWorkspaceEditorPointerDown(editorEvent.event, context);
            return editorEvent.handled();
          }
          actions.resetPreview();
          return true;
        }
        if (source === 'root') {
          actions.resetPreview();
        } else {
          const context = editorContext();
          if (!context.derived.selectionBounds) return false;
          const editorEvent = createEditorPointerEvent(
            pointer,
            context.derived.selectionBounds,
          );
          handleCaptureWorkspaceEditorPreviewPointerDown(
            editorEvent.event,
            context,
          );
          return editorEvent.handled();
        }
      }
      if (state.status !== 'selecting') return false;
      if (button === 2) {
        if (state.selection) {
          hasKeyboardAdjustedDraft = false;
          patch({ startPoint: null, selection: null, hoverSelection: null });
        } else {
          launch(cancelSession);
        }
        return true;
      }

      const draftStart = planCaptureDraftSelectionStart({
        cursorPoint: point,
        anchorPoint: snapPointToRects(
          point,
          getEditorDerived().snapTargetRects,
          6,
        ),
      });

      hasKeyboardAdjustedDraft = false;
      patch({
        startPoint: draftStart.nextState.startPoint,
        cursorPoint: draftStart.nextState.cursorPoint,
        selection: draftStart.nextState.selection,
        hoverSelection: draftStart.nextState.hoverSelection,
        previewImageBase64: draftStart.nextState.previewImageBase64,
        isRenderingOutput: draftStart.nextState.renderingOutput,
      });
      return true;
    },

    pointerMove(input) {
      if (!state.session) return false;
      const pointer = pointerInput(input);
      if (state.status === 'preview') {
        const context = editorContext();
        if (!context.derived.selectionBounds) return false;
        handleCaptureWorkspaceEditorPointerMove(
          createEditorPointerEvent(pointer, context.derived.selectionBounds)
            .event,
          context,
        );
        return true;
      }
      if (state.status !== 'selecting') return false;
      const { point, shiftKey = false } = pointer;

      if (state.startPoint) {
        const draft = planCaptureDraftSelectionPointerMove({
          anchorPoint: state.startPoint,
          point,
          snapTargetRects: getEditorDerived().snapTargetRects,
          edgeSnapThreshold: 6,
          constrainSelection: shiftKey,
        });
        hasKeyboardAdjustedDraft = false;
        patch({
          cursorPoint: point,
          selection: draft.draftSelection,
        });
        return true;
      }

      patch({
        cursorPoint: point,
        hoverSelection:
          getBestCandidateAtPoint(
            buildCaptureCandidates(
              state.session.monitors,
              state.session.candidates,
            ),
            point,
          )?.rect ?? null,
      });
      return true;
    },

    async pointerUp(input) {
      if (!state.session) return false;
      const pointer = pointerInput(input);
      if (state.status === 'preview') {
        const context = editorContext();
        if (!context.derived.selectionBounds) return false;
        handleCaptureWorkspaceEditorPointerUp(
          createEditorPointerEvent(pointer, context.derived.selectionBounds)
            .event,
          context,
        );
        return true;
      }
      if (state.status !== 'selecting' || !state.startPoint) {
        return false;
      }
      const { point, shiftKey = false } = pointer;
      const releasePoint = hasKeyboardAdjustedDraft
        ? state.cursorPoint ?? point
        : point;
      hasKeyboardAdjustedDraft = false;

      const candidates = buildCaptureCandidates(
        state.session.monitors,
        state.session.candidates,
      );
      const draftCommit = planCaptureDraftSelectionCommit({
        anchorPoint: state.startPoint,
        releasePoint,
        snapTargetRects: getEditorDerived().snapTargetRects,
        edgeSnapThreshold: 6,
        constrainSelection: shiftKey,
        captureCandidates: candidates,
        activeHoverSelection: state.hoverSelection,
        minSelectionSize: MIN_SELECTION_SIZE,
      });
      const manualSelection = normalizeSelection(state.startPoint, releasePoint);
      const isManualSelection =
        manualSelection.width >= MIN_SELECTION_SIZE &&
        manualSelection.height >= MIN_SELECTION_SIZE;

      patch({ startPoint: null, cursorPoint: point });
      if (draftCommit.type === 'complete-selection' && isManualSelection) {
        await completeManualSelection(draftCommit.selection);
      } else if (draftCommit.type === 'complete-selection') {
        await completeCandidateSelection(draftCommit.selection);
      } else {
        patch({ selection: null, hoverSelection: null });
      }
      return true;
    },

    resizePointerDown(handle: SelectionHandle, input) {
      const context = editorContext();
      if (!context.derived.selectionBounds) return false;
      const editorEvent = createEditorPointerEvent(
        pointerInput(input),
        context.derived.selectionBounds,
      );
      handleCaptureWorkspaceEditorResizePointerDown(
        handle,
        editorEvent.event,
        context,
      );
      return editorEvent.handled();
    },

    wheel(input) {
      let handled = false;
      handleCaptureWorkspaceEditorWheel(
        {
          deltaY: input.deltaY,
          metaKey: input.metaKey ?? false,
          ctrlKey: input.ctrlKey ?? false,
          altKey: input.altKey ?? false,
          preventDefault: () => {
            handled = true;
          },
        },
        editorContext(),
      );
      return handled;
    },

    commitTextDraft: editorActions.commitTextDraft,
    updateTextDraftText(text) {
      if (state.textDraft) {
        patch({ textDraft: updateTextAnnotationDraft(state.textDraft, text) });
      }
    },
    discardTextDraft() {
      const shouldRender = state.textDraftAnnotationIndex !== null;
      patch({ textDraft: null, textDraftAnnotationIndex: null });
      if (shouldRender) {
        renderCurrentSelection(state.annotationHistory.annotations);
      }
    },
    selectMoveTool() {
      patch({ activeAnnotationTool: null });
    },
    toggleAnnotationTool,
    applySelectedAnnotationStyle,
    updateTextDraftFontSize(fontSize) {
      patch({
        textFontSize: fontSize,
        textDraft: state.textDraft ? { ...state.textDraft, fontSize } : null,
      });
    },
    updateCursorColor(cursorColor) {
      patch({ cursorColor });
    },

    updatePolledCursor(point) {
      if (
        state.status === 'selecting' &&
        !state.startPoint &&
        !arePointsEqual(state.cursorPoint, point)
      ) {
        patch({ cursorPoint: point });
      }
    },

    updatePolledHover(hoverSelection) {
      if (
        state.status === 'selecting' &&
        !state.startPoint &&
        !areRectsEqual(state.hoverSelection, hoverSelection)
      ) {
        patch({ hoverSelection });
      }
    },

    keyDown(input: CaptureWorkspaceKeyInput) {
      const event = {
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
        repeat: false,
        ...input,
      };
      if (
        (state.status === 'selecting' || state.status === 'preview') &&
        isRefreshCaptureShortcut(event)
      ) {
        launch(actions.refreshSession);
        return true;
      }
      if (
        (state.status === 'selecting' || state.status === 'preview') &&
        !state.textDraft &&
        canToggleCapturedCursor(state.session) &&
        isToggleCapturedCursorShortcut(event)
      ) {
        const includeCapturedCursor = !state.includeCapturedCursor;
        patch({ includeCapturedCursor });
        if (state.status === 'preview' && state.selection) {
          const selection = state.selection;
          launch(() =>
            renderSelectionPreview(
              selection,
              state.annotationHistory.annotations,
              includeCapturedCursor,
            ),
          );
        }
        return true;
      }
      const historyStep = getSelectionHistoryStepFromShortcut(event);
      if (
        historyStep &&
        !state.textDraft &&
        (state.status === 'selecting' || state.status === 'preview')
      ) {
        restoreSelectionHistory(historyStep);
        return true;
      }
      if (
        isRestoreLastSelectionShortcut(event) &&
        !state.textDraft &&
        (state.status === 'selecting' || state.status === 'preview')
      ) {
        restoreLastSelection();
        return true;
      }
      if (
        isSelectAllCaptureShortcut(event) &&
        !state.textDraft &&
        state.session &&
        (state.status === 'selecting' || state.status === 'preview')
      ) {
        const point =
          state.cursorPoint ??
          state.session.captured_cursor?.logical_position ??
          null;
        const rect = getCurrentMonitorBounds(state.session.monitors, point);
        launch(() => completeManualSelection(rect));
        return true;
      }
      const cursorNudgeDelta = getCursorNudgeDeltaFromShortcut(event);
      if (
        state.status === 'selecting' &&
        state.session &&
        state.startPoint &&
        state.cursorPoint &&
        state.selection &&
        cursorNudgeDelta
      ) {
        const draftNudge = planCaptureDraftSelectionKeyboardNudge({
          anchorPoint: state.startPoint,
          cursorPoint: state.cursorPoint,
          delta: cursorNudgeDelta,
          selectionBounds: getVirtualDesktopBounds(state.session.monitors),
        });
        hasKeyboardAdjustedDraft = true;
        patch({
          cursorPoint: draftNudge.cursorPoint,
          selection: draftNudge.selection,
          previewImageBase64: draftNudge.previewImageBase64,
          isRenderingOutput: draftNudge.renderingOutput,
        });
        return true;
      }
      if (
        state.status === 'selecting' &&
        state.session &&
        state.cursorPoint &&
        cursorNudgeDelta
      ) {
        const cursorNudge = planCaptureSelectionCursorKeyboardNudge({
          cursorPoint: state.cursorPoint,
          delta: cursorNudgeDelta,
          selectionBounds: getVirtualDesktopBounds(state.session.monitors),
        });
        const cursorPoint = cursorNudge.cursorPoint;
        patch({
          cursorPoint,
          hoverSelection:
            getBestCandidateAtPoint(
              buildCaptureCandidates(
                state.session.monitors,
                state.session.candidates,
              ),
              cursorPoint,
            )?.rect ?? null,
        });
        return true;
      }
      const cycleDirection = getCandidateCycleDirectionFromShortcut(event);
      if (
        state.status === 'selecting' &&
        state.session &&
        state.cursorPoint &&
        cycleDirection
      ) {
        patch({
          hoverSelection: planCaptureHoverSelectionCycle({
            captureCandidates: buildCaptureCandidates(
              state.session.monitors,
              state.session.candidates,
            ),
            cursorPoint: state.cursorPoint,
            hoverSelection: state.hoverSelection,
            direction: cycleDirection,
          }).hoverSelection,
        });
        return true;
      }
      const hoverAction = getHoverSelectionCompletionActionFromShortcut(event, {
        drafting: state.startPoint !== null,
        mode: state.mode,
      });
      if (
        state.status === 'selecting' &&
        state.hoverSelection &&
        hoverAction
      ) {
        const hoverSelection = state.hoverSelection;
        launch(() => completeCandidateSelection(hoverSelection, hoverAction));
        return true;
      }
      const previewAction = getPreviewCaptureCompletionActionFromShortcut(event);
      if (state.status === 'preview' && state.selection && previewAction) {
        const selection = state.selection;
        launch(() =>
          runCompletionEffects(
            selection,
            planCandidateSelectionCompletion(previewAction),
            commitTextDraftToHistory().annotations,
            state.includeCapturedCursor,
          ),
        );
        return true;
      }
      let editorHandled = false;
      handleCaptureWorkspaceEditorKeyDown(
        {
          ...event,
          preventDefault: () => {
            editorHandled = true;
          },
        } as KeyboardEvent,
        keyboardEditorContext(),
      );
      if (editorHandled) return true;
      if (event.key === 'Escape') {
        if (state.status !== 'selecting') return false;
        launch(cancelSession);
        return true;
      }

      if (event.key === 'Enter' && state.hoverSelection) {
        const hoverSelection = state.hoverSelection;
        launch(() => completeCandidateSelection(hoverSelection));
        return true;
      }
      return false;
    },

    async hydrateSnapshots() {
      const sessionId = state.session?.id;
      if (!sessionId) return;
      const actionGeneration = generation;

      if (
        snapshotHydration?.generation === actionGeneration &&
        snapshotHydration.sessionId === sessionId
      ) {
        await snapshotHydration.promise;
        return;
      }

      hydratedSessionId = null;
      const promise = platform.commands
        .hydrateCaptureSessionSnapshots(sessionId)
        .then((session) => {
          if (
            snapshotHydration?.generation !== actionGeneration ||
            snapshotHydration?.sessionId !== sessionId ||
            generation !== actionGeneration ||
            state.session?.id !== sessionId
          ) {
            return;
          }

          patch({ session });
          hydratedSessionId = sessionId;
          markPerf('snapshots_hydrated', sessionId);
        })
        .catch((error) => {
          if (
            snapshotHydration?.generation === actionGeneration &&
            snapshotHydration.sessionId === sessionId
          ) {
            snapshotHydration = null;
            hydratedSessionId = null;
          }
          throw error;
        });

      snapshotHydration = {
        generation: actionGeneration,
        sessionId,
        promise,
      };
      await promise;
    },
  };

  return {
    get renderState() {
      return {
        status: state.status,
        mode: state.mode,
        session: state.session,
        sessionId: state.session?.id ?? null,
        cursorPoint: state.cursorPoint,
        startPoint: state.startPoint,
        selection: state.selection,
        hoverSelection: state.hoverSelection,
        previewImageBase64: state.previewImageBase64,
        editGesture: state.editGesture,
        activeAnnotationTool: state.activeAnnotationTool,
        annotationGesture: state.annotationGesture,
        draftAnnotation: state.draftAnnotation,
        selectedAnnotationIndex: state.selectedAnnotationIndex,
        annotationMoveGesture: state.annotationMoveGesture,
        draftSelectionMoveGesture: state.draftSelectionMoveGesture,
        textDraft: state.textDraft,
        textDraftAnnotationIndex: state.textDraftAnnotationIndex,
        annotationStyle: state.annotationStyle,
        textFontSize: state.textFontSize,
        annotationHistory: state.annotationHistory,
        isAnnotationToolbarVisible: state.isAnnotationToolbarVisible,
        cursorColor: state.cursorColor,
        colorSampleFormat: state.colorSampleFormat,
        isMagnifierRequested: state.isMagnifierRequested,
        includeCapturedCursor: state.includeCapturedCursor,
        isRenderingOutput: state.isRenderingOutput,
        hasHydratedPixelSource:
          state.session !== null && hydratedSessionId === state.session.id,
        error: state.error,
      };
    },
    actions,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

function createInitialState(
  mode: CaptureMode = 'screenshot',
  previous?: Pick<RuntimeState, 'annotationStyle' | 'textFontSize'>,
): RuntimeState {
  const initial = createInitialCaptureWorkspaceState();
  return {
    ...initial,
    mode,
    annotationStyle: previous?.annotationStyle ?? initial.annotationStyle,
    textFontSize: previous?.textFontSize ?? initial.textFontSize,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function arePointsEqual(a: Point | null, b: Point | null) {
  return a === b || (a !== null && b !== null && a.x === b.x && a.y === b.y);
}

function areRectsEqual(a: LogicalRect | null, b: LogicalRect | null) {
  return (
    a === b ||
    (a !== null &&
      b !== null &&
      a.x === b.x &&
      a.y === b.y &&
      a.width === b.width &&
      a.height === b.height)
  );
}

function pointerInput(
  input: Point | CaptureWorkspacePointerInput,
): CaptureWorkspacePointerInput {
  return 'point' in input ? input : { point: input };
}

function createEditorPointerEvent(
  input: CaptureWorkspacePointerInput,
  selectionBounds: LogicalRect,
) {
  let wasHandled = false;
  const markHandled = () => {
    wasHandled = true;
  };
  const event: CaptureWorkspacePointerEvent = {
    clientX: input.point.x - selectionBounds.x,
    clientY: input.point.y - selectionBounds.y,
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
