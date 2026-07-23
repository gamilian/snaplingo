import type {
  AnnotationCommand,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from '../../domain/capture';
import {
  ANNOTATION_COLORS,
  type AnnotationColor,
} from '../../domain/annotationColor';
import type {
  HoverSelectionCompletionAction,
} from './captureActions';
import {
  canToggleCapturedCursor,
  getCaptureScreenSelectionScopeFromShortcut,
  getCursorNudgeDeltaFromShortcut,
  getHoverSelectionCompletionActionFromPointer,
  getHoverSelectionCompletionActionFromShortcut,
  getPreviewCaptureCompletionActionFromShortcut,
  getSelectionHistoryStepFromShortcut,
  isRefreshCaptureShortcut,
  isRestoreLastSelectionShortcut,
  isCandidateDetectionModeToggleShortcut,
  isToggleCapturedCursorShortcut,
} from './captureActions';
import {
  getCaptureKeyboardKeyUpAction,
  planCaptureKeyboardBlur,
} from './captureKeyboardHostRuntime';
import {
  buildCaptureCandidates,
  getBestCandidateAtPoint,
} from './captureCandidates';
import {
  getPrimaryCaptureCompletionActionForMode,
  planCandidateSelectionCompletion,
  planManualSelectionCompletion,
  type CaptureRuntimeEffect,
} from './captureInteractionRuntime';
import {
  recordSuccessfulCaptureSelection,
  restoreCaptureSelectionFromHistory,
  restoreLastSuccessfulCaptureSelection,
  type CaptureSelectionStorage,
} from './captureHostRuntime';
import { printBase64PngImage } from './capturePrint';
import {
  clearAnnotationHistory,
  emptyAnnotationHistory,
  redoAnnotationHistory,
  removeAnnotationFromHistory,
  undoAnnotationHistory,
} from './annotationHistory';
import {
  type AnnotationSizeDirection,
  type AnnotationStyle,
  type AnnotationTool,
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
  planCaptureManualSelectionTransition,
} from './captureEditorRuntime';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import {
  handleCaptureWorkspaceEditorKeyDown,
  type CaptureWorkspaceKeyboardEditorActions,
  type CaptureWorkspaceKeyboardEditorContext,
} from './captureWorkspaceKeyboard';
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
} from './captureWorkspacePointer';
import {
  createInitialCaptureWorkspaceState,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import {
  colorSamplesEqual,
  colorSampleToClipboardText,
} from './colorSampler';
import { updateTextAnnotationDraft } from './textAnnotationDraft';
import {
  normalizeSelection,
  snapPointToRects,
} from './selection';
import type { SelectionHandle } from './selection';
import {
  planCaptureDraftSelectionCommit,
  planCaptureDraftSelectionKeyboardNudge,
  planCaptureDraftSelectionPointerMove,
  planCaptureDraftSelectionStart,
  planCaptureSelectionCursorKeyboardNudge,
} from './captureSelectionRuntime';
import { shouldRevealCaptureWindow } from './captureWindowVisibility';
import {
  getCurrentMonitorBounds,
  getVirtualDesktopBounds,
} from './virtualDesktop';
import {
  applyOcrTextPreferences,
  normalizeOcrText,
} from '../../utils/ocrTextProcessing';
import type { OcrSettings } from '../../application/settings/ports';
import type {
  CaptureWorkspaceRuntime,
  CaptureWorkspacePointerInput,
  CaptureWorkspaceKeyInput,
  CaptureWorkspaceRuntimePlatform,
} from './captureWorkspaceRuntimeTypes';

const MIN_SELECTION_SIZE = 10;

type RuntimeState = CaptureWorkspaceState;

interface SnapshotHydration {
  generation: number;
  sessionId: string;
  promise: Promise<void>;
}

interface MagnifierMonitorHydration extends SnapshotHydration {
  monitorId: string;
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

interface HostConnection {
  readonly isClosed: boolean;
  retain(dispose: () => void): void;
  disconnect(): void;
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
      capture?: boolean,
    ): void;
    addEventListener(
      type: 'keyup',
      listener: (event: KeyboardEvent) => void,
    ): void;
    addEventListener(type: 'blur', listener: () => void): void;
    removeEventListener(
      type: 'keydown',
      listener: (event: KeyboardEvent) => void,
      capture?: boolean,
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

interface RuntimeReplacementSnapshot {
  state: RuntimeState;
  hydratedSessionId: string | null;
  hasRevealed: boolean;
  perfState: CaptureFrontendPerfState | null;
  hasKeyboardAdjustedDraft: boolean;
  keyboardEditCursorPoint: Point | null;
}

export interface CaptureScreenshotPreferences {
  savePath?: string;
  format: 'png' | 'jpg' | 'webp';
  quality: number;
  namingRule: 'timestamp' | 'date' | 'counter' | 'custom';
  customFileName: string;
  autoCopy: boolean;
  defaultStrokeWidth: number;
  defaultFontSize: number;
  rememberLastTool: boolean;
  showSelectionSize: boolean;
  showMagnifier: boolean;
  magnifierZoom?: number;
  selectionBorderWidth?: number;
  selectionBorderColor?: [number, number, number, number];
  selectionMaskColor?: [number, number, number, number];
}

export function createCaptureWorkspaceRuntime({
  platform,
  host,
  keyboard,
  onInactive,
  annotationColorPresets,
  screenshotPreferences,
  persistScreenshotDefaults,
  ocrPreferences,
  delay,
  storage,
}: {
  platform: CaptureWorkspaceRuntimePlatform;
  host?: CaptureWorkspaceRuntimeHost;
  keyboard?: CaptureWorkspaceRuntimeKeyboard;
  onInactive?: () => void | Promise<void>;
  annotationColorPresets?: () => readonly AnnotationColor[];
  screenshotPreferences?: () => CaptureScreenshotPreferences | undefined;
  persistScreenshotDefaults?: (
    input: Partial<
      Pick<CaptureScreenshotPreferences, 'defaultStrokeWidth' | 'defaultFontSize'>
    >,
  ) => void;
  ocrPreferences?: () => OcrSettings | undefined;
  delay?: (milliseconds: number) => Promise<void>;
  storage?: CaptureSelectionStorage;
}): CaptureWorkspaceRuntime {
  const wait =
    delay ??
    ((milliseconds: number) =>
      new Promise<void>((resolve) => globalThis.setTimeout(resolve, milliseconds)));
  let state = createInitialState('screenshot', undefined, screenshotPreferences?.());
  let generation = 0;
  let disposed = false;
  let hydratedSessionId: string | null = null;
  let magnifierMonitorHydration: MagnifierMonitorHydration | null = null;
  let snapshotHydration: SnapshotHydration | null = null;
  const listeners = new Set<() => void>();
  let hasRevealed = false;
  let revealAttempt: { key: string; promise: Promise<void> } | null = null;
  let perfState: CaptureFrontendPerfState | null = null;
  let hasKeyboardAdjustedDraft = false;
  let pressedHoverSelection: LogicalRect | null = null;
  let pressedHoverDetectionMode: RuntimeState['candidateDetectionMode'] | null = null;
  let controlCandidateRequestRevision = 0;
  let controlCandidateRefresh: Promise<void> | null = null;
  let queuedControlCandidateRefresh: {
    sessionId: string;
    cursorPoint: Point;
  } | null = null;
  const invalidateControlCandidateRefresh = () => {
    controlCandidateRequestRevision += 1;
    queuedControlCandidateRefresh = null;
  };
  const clearPressedHoverSelection = () => {
    pressedHoverSelection = null;
    pressedHoverDetectionMode = null;
    invalidateControlCandidateRefresh();
  };
  const cursorPointRef = { current: null as Point | null };
  const keyboardEditCursorPointRef = { current: null as Point | null };
  let previewScheduler: PreviewRenderScheduler | null = null;
  let terminalOutputSequence = 0;
  let terminalOutputOperation: TerminalOutputOperation | null = null;
  let cursorMoveRequestRevision = 0;
  const hostConnections = new Set<HostConnection>();
  const nativeSessionCancellations = new Map<string, Promise<void>>();
  const provisionalSessionIds = new Set<string>();

  const releaseDisposer = (dispose: () => void) => {
    try {
      const result = dispose() as unknown;
      if (result && typeof (result as PromiseLike<void>).then === 'function') {
        void Promise.resolve(result).catch(() => undefined);
      }
    } catch {
      // Disposal is best-effort and must not interrupt remaining cleanup.
    }
  };

  const createHostConnection = (): HostConnection => {
    let closed = false;
    const disposers: Array<() => void> = [];
    const connection: HostConnection = {
      get isClosed() {
        return closed;
      },
      retain(dispose) {
        if (disposed || closed) {
          releaseDisposer(dispose);
          return;
        }
        disposers.push(dispose);
      },
      disconnect() {
        if (closed) return;
        closed = true;
        hostConnections.delete(connection);
        for (const dispose of disposers.splice(0).reverse()) {
          releaseDisposer(dispose);
        }
      },
    };
    hostConnections.add(connection);
    return connection;
  };

  const cancelNativeSessionOnce = (sessionId: string) => {
    const existing = nativeSessionCancellations.get(sessionId);
    if (existing) return existing;

    let cancellation: Promise<void>;
    try {
      cancellation = platform.commands.cancelCaptureSession(sessionId);
    } catch (error) {
      cancellation = Promise.reject(error);
    }
    nativeSessionCancellations.set(sessionId, cancellation);
    void cancellation.catch(() => {
      if (nativeSessionCancellations.get(sessionId) === cancellation) {
        nativeSessionCancellations.delete(sessionId);
      }
    });
    return cancellation;
  };

  const captureReplacementSnapshot = (): RuntimeReplacementSnapshot => ({
    state,
    hydratedSessionId,
    hasRevealed,
    perfState: perfState ? { ...perfState } : null,
    hasKeyboardAdjustedDraft,
    keyboardEditCursorPoint: keyboardEditCursorPointRef.current,
  });

  const restoreReplacementSnapshot = (
    snapshot: RuntimeReplacementSnapshot,
    error: unknown,
  ) => {
    state = {
      ...snapshot.state,
      isRenderingOutput: false,
      error: errorMessage(error),
    };
    cursorPointRef.current = state.cursorPoint;
    keyboardEditCursorPointRef.current = snapshot.keyboardEditCursorPoint;
    hydratedSessionId = snapshot.hydratedSessionId;
    snapshotHydration = null;
    hasRevealed = snapshot.hasRevealed;
    revealAttempt = null;
    perfState = snapshot.perfState;
    hasKeyboardAdjustedDraft = snapshot.hasKeyboardAdjustedDraft;
    listeners.forEach((listener) => listener());
    try {
      host?.scheduleSelectionOverlayPaint?.();
    } catch {
      // The restored runtime state remains authoritative if the old DOM is gone.
    }
  };

  const markPerf = (event: string, sessionId?: string | null) => {
    if (disposed) return;
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
    if (disposed) return;
    state = { ...state, ...next };
    if ('cursorPoint' in next) cursorPointRef.current = next.cursorPoint ?? null;
    listeners.forEach((listener) => listener());
  };

  const launch = (operation: () => Promise<unknown>) => {
    if (disposed) return;
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

  const dispose = () => {
    if (disposed) return;
    disposed = true;
    generation += 1;
    const activeSessionId = state.session?.id ?? null;
    detachPreviewScheduler();
    terminalOutputOperation = null;
    revealAttempt = null;
    hasRevealed = false;
    perfState = null;
    snapshotHydration = null;
    hydratedSessionId = null;
    hasKeyboardAdjustedDraft = false;
    clearPressedHoverSelection();
    cursorPointRef.current = null;
    keyboardEditCursorPointRef.current = null;
    for (const connection of [...hostConnections]) {
      connection.disconnect();
    }
    listeners.clear();
    state = createInitialState(state.mode, state, screenshotPreferences?.());
    try {
      host?.resetSession();
    } catch {
      // Continue native cleanup even if the presentation host is already gone.
    }
    const sessionsToCancel = new Set(provisionalSessionIds);
    provisionalSessionIds.clear();
    if (activeSessionId) sessionsToCancel.add(activeSessionId);
    for (const sessionId of sessionsToCancel) {
      void cancelNativeSessionOnce(sessionId).catch(() => undefined);
    }
  };

  const resetSession = () => {
    if (disposed) return;
    detachPreviewScheduler();
    terminalOutputOperation = null;
    state = createInitialState(state.mode, state, screenshotPreferences?.());
    cursorPointRef.current = null;
    keyboardEditCursorPointRef.current = null;
    hasKeyboardAdjustedDraft = false;
    clearPressedHoverSelection();
    hydratedSessionId = null;
    snapshotHydration = null;
    host?.resetSession();
    listeners.forEach((listener) => listener());
  };

  const createNativeSessionCancellation = (sessionId: string) => {
    return () => cancelNativeSessionOnce(sessionId);
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
          ? await (async () => {
              const preferences = screenshotPreferences?.();
              const path = await platform.commands.defaultCaptureSavePath(
                captureSavePathOptions(preferences),
              );
              return path
                ? ({
                    type: 'save' as const,
                    path,
                    format: preferences?.format ?? 'png',
                    quality: preferences?.quality ?? 90,
                    copyAfterSave: preferences?.autoCopy ?? false,
                  } as const)
                : null;
            })()
          : effect.action === 'quick-save'
            ? await (async () => {
                const preferences = screenshotPreferences?.();
                return {
                  type: 'save' as const,
                  path: await platform.commands.quickCaptureSavePath(
                    captureSavePathOptions(preferences),
                  ),
                  format: preferences?.format ?? 'png',
                  quality: preferences?.quality ?? 90,
                  copyAfterSave: preferences?.autoCopy ?? false,
                };
              })()
            : effect.action === 'pin'
              ? { type: 'pin' as const }
              : effect.action === 'favorite'
                ? { type: 'favorite' as const }
              : { type: 'copy' as const };
      if (!action) return true;
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
      const ocrSettings = ocrPreferences?.();
      const recognitionLanguage =
        ocrSettings?.recognitionLanguage === 'auto'
          ? undefined
          : ocrSettings?.recognitionLanguage;
      const result = recognitionLanguage
        ? await platform.commands.runCaptureOcr(
            sessionId,
            rect,
            recognitionLanguage,
          )
        : await platform.commands.runCaptureOcr(sessionId, rect);
      if (await cancelIfStale()) return;
      const text = ocrSettings
        ? applyOcrTextPreferences(result.text, ocrSettings)
        : normalizeOcrText(result.text);

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
        if (result.confidence === null) {
          await platform.commands.openCaptureOcrResultWindow(text, imageBase64);
        } else {
          await platform.commands.openCaptureOcrResultWindow(
            text,
            imageBase64,
            result.confidence,
          );
        }
        return;
      }

      await platform.commands.copyTextToClipboard(text);
      if (!ocrSettings?.hideSilentStatus) {
        const point = state.silentOcrHint?.point ?? {
          x: rect.x + rect.width / 2,
          y: rect.y + rect.height / 2,
        };
        patch({ silentOcrHint: { status: 'success', point } });
        await wait(550);
      }
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

    const preservesPreview = state.status === 'preview';
    const isSilentOcr = effects.some(
      (effect) => effect.type === 'run-ocr' && effect.target === 'clipboard',
    );
    const showSilentOcrHint =
      isSilentOcr && !ocrPreferences?.()?.hideSilentStatus;
    const hintPoint = state.cursorPoint ?? {
      x: rect.x + rect.width / 2,
      y: rect.y + rect.height / 2,
    };
    patch({
      ...(preservesPreview ? {} : { status: 'loading', cursorPoint: null }),
      selection: rect,
      hoverSelection: null,
      isRenderingOutput: true,
      silentOcrHint: showSilentOcrHint
        ? { status: 'loading', point: hintPoint }
        : null,
      error: null,
    });

    try {
      for (const effect of effects) {
        const cancelled = await executeEffect(
          effect,
          session.id,
          rect,
          annotations,
          includeCursor,
          actionGeneration,
          cancelNativeSession,
        );
        if (cancelled) return;
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
        patch({
          status: 'error',
          error: errorMessage(error),
          silentOcrHint: null,
        });
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
    preservePreviewImage = false,
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

    const preferences = screenshotPreferences?.();
    patch({
      status: 'preview',
      selection: rect,
      hoverSelection: null,
      activeAnnotationTool: rememberedAnnotationTool(preferences, storage),
      ...(preservePreviewImage ? {} : { previewImageBase64: null }),
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
    _annotations: AnnotationCommand[] = state.annotationHistory.annotations,
    includeCursor =
      state.includeCapturedCursor && canToggleCapturedCursor(state.session),
  ) => renderSelectionPreview(rect, [], includeCursor, true);

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
  };

  const redoAnnotation = () => {
    const nextHistory = redoAnnotationHistory(state.annotationHistory);
    if (nextHistory === state.annotationHistory) return;
    patch({
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      annotationHistory: nextHistory,
    });
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
      ...(result.selectedAnnotationIndex === null
        ? {}
        : { activeAnnotationTool: null }),
      annotationGesture: result.annotationGesture,
      draftAnnotation: result.draftAnnotation,
    });
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
    commitTextDraftToHistory();
    const activation = planCaptureAnnotationToolActivation({
      currentTool: state.activeAnnotationTool,
      nextTool,
      selectedAnnotationIndex: state.selectedAnnotationIndex,
      clearSelectedAnnotation: false,
      toggle: true,
    });
    patch(activation);
    if (activation.activeAnnotationTool && screenshotPreferences?.()?.rememberLastTool) {
      storage?.setItem('snaplingo.capture.lastAnnotationTool', activation.activeAnnotationTool);
    }
  };

  const editorActions: CaptureWorkspacePointerEditorActions &
    CaptureWorkspaceKeyboardEditorActions = {
    commitTextDraft() {
      commitTextDraftToHistory();
      patch({ activeAnnotationTool: null, selectedAnnotationIndex: null });
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
    setAnnotationGesture: (
      annotationGesture,
      draftAnnotation?: AnnotationCommand | null,
    ) =>
      patch(
        draftAnnotation === undefined
          ? { annotationGesture }
          : { annotationGesture, draftAnnotation },
      ),
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
      annotationColorPresets: annotationColorPresets?.() ?? ANNOTATION_COLORS,
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
    if (transition.clearOverlay) host?.resetInteraction();
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

  const windowCandidateAt = (point: Point) =>
    state.session
      ? getBestCandidateAtPoint(
          buildCaptureCandidates(
            state.session.monitors,
            state.session.candidates,
          ),
          point,
        )?.rect ?? null
      : null;

  const refreshControlCandidate = async (
    sessionId: string,
    cursorPoint: Point,
  ) => {
    const requestRevision = ++controlCandidateRequestRevision;
    try {
      const candidate = await platform.commands.currentCaptureControlCandidate(
        sessionId,
        cursorPoint,
      );
      if (
        requestRevision === controlCandidateRequestRevision &&
        state.status === 'selecting' &&
        state.session?.id === sessionId &&
        state.candidateDetectionMode === 'control' &&
        !state.startPoint &&
        arePointsEqual(state.cursorPoint, cursorPoint)
      ) {
        patch({ hoverSelection: candidate?.rect ?? null, error: null });
      }
    } catch (error) {
      if (
        requestRevision === controlCandidateRequestRevision &&
        state.status === 'selecting' &&
        state.session?.id === sessionId &&
        state.candidateDetectionMode === 'control' &&
        !state.startPoint &&
        arePointsEqual(state.cursorPoint, cursorPoint)
      ) {
        patch({
          candidateDetectionMode: 'window',
          hoverSelection: windowCandidateAt(state.cursorPoint ?? cursorPoint),
          error: `界面元素检测不可用：${errorMessage(error)}`,
        });
      }
    }
  };

  const scheduleControlCandidateRefresh = (
    sessionId: string,
    cursorPoint: Point,
  ) => {
    queuedControlCandidateRefresh = { sessionId, cursorPoint };
    if (controlCandidateRefresh) return;

    const drain = async () => {
      while (queuedControlCandidateRefresh) {
        const request = queuedControlCandidateRefresh;
        queuedControlCandidateRefresh = null;
        if (
          state.status !== 'selecting' ||
          state.session?.id !== request.sessionId ||
          state.candidateDetectionMode !== 'control' ||
          state.startPoint
        ) {
          continue;
        }
        await refreshControlCandidate(request.sessionId, request.cursorPoint);
      }
    };

    const request = drain().finally(() => {
      if (controlCandidateRefresh !== request) return;
      controlCandidateRefresh = null;
      const queued = queuedControlCandidateRefresh;
      if (queued) {
        scheduleControlCandidateRefresh(queued.sessionId, queued.cursorPoint);
      }
    });
    controlCandidateRefresh = request;
    void request.catch(() => undefined);
  };

  const moveCaptureCursor = async (sessionId: string, delta: Point) => {
    const requestRevision = ++cursorMoveRequestRevision;
    try {
      await platform.commands.moveCaptureCursor(delta);
      if (
        requestRevision === cursorMoveRequestRevision &&
        state.status === 'selecting' &&
        state.session?.id === sessionId &&
        state.error?.startsWith('鼠标移动失败：')
      ) {
        patch({ error: null });
      }
      return true;
    } catch (error) {
      if (
        requestRevision === cursorMoveRequestRevision &&
        state.status === 'selecting' &&
        state.session?.id === sessionId
      ) {
        patch({ error: `鼠标移动失败：${errorMessage(error)}` });
      }
      return false;
    }
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
    if (disposed) return;
    clearPressedHoverSelection();
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
      if (disposed) return () => undefined;
      const connection = createHostConnection();
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

      if (keyboard && !disposed && !connection.isClosed) {
        keyboard.target.addEventListener('keydown', handleKeyDown, true);
        keyboard.target.addEventListener('keyup', handleKeyUp);
        keyboard.target.addEventListener('blur', handleBlur);
        connection.retain(() => {
          keyboard.target.removeEventListener('keydown', handleKeyDown, true);
          keyboard.target.removeEventListener('keyup', handleKeyUp);
          keyboard.target.removeEventListener('blur', handleBlur);
        });
      }

      try {
        connection.retain(
          await platform.onHotkeyTriggered((launch) => {
            if (disposed) return;
            return actions.startSession(launch.mode, launch.sessionId);
          }),
        );
        if (disposed || connection.isClosed) return connection.disconnect;

        connection.retain(
          await platform.onCancelRequested(() => {
            if (disposed) return;
            return cancelSession();
          }),
        );
        if (disposed || connection.isClosed) return connection.disconnect;

        connection.retain(
          await platform.onCopyRequested(async () => {
            if (disposed) return;
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
        if (disposed || connection.isClosed) return connection.disconnect;

        connection.retain(
          await platform.onSaveRequested(async () => {
            if (disposed) return;
            if (state.status === 'preview' && state.selection) {
              await runCompletionEffects(
                state.selection,
                planCandidateSelectionCompletion('save'),
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
              await completeCandidateSelection(state.hoverSelection, 'save');
            }
          }),
        );
        if (disposed || connection.isClosed) return connection.disconnect;

        connection.retain(
          await platform.onUndoRequested(() => {
            if (state.status === 'preview' && !state.textDraft) {
              undoAnnotation();
            }
          }),
        );
        if (disposed || connection.isClosed) return connection.disconnect;

        connection.retain(
          await platform.onRedoRequested(() => {
            if (state.status === 'preview' && !state.textDraft) {
              redoAnnotation();
            }
          }),
        );
      } catch (error) {
        connection.disconnect();
        if (!disposed) patch({ status: 'error', error: errorMessage(error) });
        return () => undefined;
      }

      return connection.disconnect;
    },

    async updateHostReadiness(imagesReady) {
      if (disposed) return;
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
      if (disposed) return;
      const previousSessionId = state.session?.id ?? null;
      const previousSnapshot = previousSessionId
        ? captureReplacementSnapshot()
        : null;
      if (previousSessionId) provisionalSessionIds.add(previousSessionId);
      const cancelStalePrevious = async () => {
        if (
          !previousSessionId ||
          !provisionalSessionIds.delete(previousSessionId)
        ) {
          return;
        }
        await cancelNativeSessionOnce(previousSessionId).catch(() => undefined);
      };
      const actionGeneration = ++generation;
      detachPreviewScheduler();
      terminalOutputOperation = null;
      hasKeyboardAdjustedDraft = false;
      clearPressedHoverSelection();
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
        ...createInitialState(mode, state, screenshotPreferences?.()),
        status: 'loading',
      };
      listeners.forEach((listener) => listener());
      hydratedSessionId = null;
      snapshotHydration = null;

      try {
        const session = requestedSessionId
          ? await platform.commands.getCaptureSession(requestedSessionId)
          : await platform.commands.createCaptureSession();
        provisionalSessionIds.add(session.id);
        if (generation !== actionGeneration) {
          provisionalSessionIds.delete(session.id);
          await cancelNativeSessionOnce(session.id).catch(() => undefined);
          await cancelStalePrevious();
          return;
        }
        if (previousSessionId && previousSessionId !== session.id) {
          try {
            await cancelNativeSessionOnce(previousSessionId);
          } catch (error) {
            provisionalSessionIds.delete(session.id);
            await cancelNativeSessionOnce(session.id).catch(() => undefined);
            throw error;
          }
          provisionalSessionIds.delete(previousSessionId);
          if (generation !== actionGeneration) {
            provisionalSessionIds.delete(session.id);
            await cancelNativeSessionOnce(session.id).catch(() => undefined);
            await cancelStalePrevious();
            return;
          }
        }
        if (perfState) perfState.sessionId = session.id;
        markPerf('session_loaded', session.id);

        const cursorPoint =
          session.captured_cursor?.logical_position ??
          (await platform.commands
            .currentCaptureCursorPosition(session.id)
            .catch(() => null));
        if (generation !== actionGeneration) {
          provisionalSessionIds.delete(session.id);
          await cancelNativeSessionOnce(session.id).catch(() => undefined);
          await cancelStalePrevious();
          return;
        }

        provisionalSessionIds.delete(session.id);
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
          if (
            previousSessionId &&
            previousSnapshot &&
            provisionalSessionIds.delete(previousSessionId)
          ) {
            restoreReplacementSnapshot(previousSnapshot, error);
          } else {
            patch({ status: 'error', error: errorMessage(error) });
          }
        } else {
          await cancelStalePrevious();
        }
      }
    },

    async refreshSession() {
      if (disposed) return;
      const previousSessionId = state.session?.id;
      if (!previousSessionId) return;
      const previousSnapshot = captureReplacementSnapshot();
      provisionalSessionIds.add(previousSessionId);
      const cancelStalePrevious = async () => {
        if (!provisionalSessionIds.delete(previousSessionId)) return;
        await cancelNativeSessionOnce(previousSessionId).catch(() => undefined);
      };
      const actionGeneration = ++generation;
      detachPreviewScheduler();
      terminalOutputOperation = null;
      hasKeyboardAdjustedDraft = false;
      clearPressedHoverSelection();
      host?.resetInteraction();
      hasRevealed = false;
      revealAttempt = null;
      state = {
        ...createInitialState(state.mode, state, screenshotPreferences?.()),
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
        provisionalSessionIds.add(session.id);
        await cancelNativeSessionOnce(previousSessionId);
        provisionalSessionIds.delete(previousSessionId);
        if (generation !== actionGeneration) {
          await cancelStalePrevious();
          return;
        }
        const cursorPoint =
          session.captured_cursor?.logical_position ??
          (await platform.commands
            .currentCaptureCursorPosition(session.id)
            .catch(() => null));
        if (generation !== actionGeneration) {
          await cancelStalePrevious();
          return;
        }
        adoptedCreatedSession = true;
        provisionalSessionIds.delete(session.id);
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
          if (provisionalSessionIds.delete(previousSessionId)) {
            restoreReplacementSnapshot(previousSnapshot, error);
          } else {
            patch({ status: 'error', error: errorMessage(error) });
          }
        } else {
          await cancelStalePrevious();
        }
      } finally {
        if (createdSession && !adoptedCreatedSession) {
          provisionalSessionIds.delete(createdSession.id);
          await cancelNativeSessionOnce(createdSession.id).catch(
            () => undefined,
          );
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
      if (disposed) return;
      generation += 1;
      detachPreviewScheduler();
      terminalOutputOperation = null;
      hasKeyboardAdjustedDraft = false;
      clearPressedHoverSelection();
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
          const context = editorContext();
          if (!context.derived.selectionBounds) return false;
          const editorEvent = createEditorPointerEvent(
            pointer,
            context.derived.selectionBounds,
          );
          handleCaptureWorkspaceEditorPointerDown(editorEvent.event, context);
          return editorEvent.handled();
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
        clearPressedHoverSelection();
        if (state.selection) {
          hasKeyboardAdjustedDraft = false;
          patch({ startPoint: null, selection: null, hoverSelection: null });
        } else {
          launch(cancelSession);
        }
        return true;
      }
      if (button !== 0) return false;
      clearPressedHoverSelection();

      pressedHoverSelection =
        button === 0 &&
        getHoverSelectionCompletionActionFromPointer(
          {
            button,
            detail,
            metaKey,
            ctrlKey,
            altKey,
            shiftKey,
          },
          { mode: state.mode },
        )
          ? state.hoverSelection
          : null;
      pressedHoverDetectionMode = pressedHoverSelection
        ? state.candidateDetectionMode
        : null;
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

      const sessionId = state.session.id;
      const candidateDetectionMode = state.candidateDetectionMode;
      patch({
        cursorPoint: point,
        hoverSelection:
          candidateDetectionMode === 'window'
            ? getBestCandidateAtPoint(
                buildCaptureCandidates(
                  state.session.monitors,
                  state.session.candidates,
                ),
                point,
              )?.rect ?? null
            : null,
      });
      if (candidateDetectionMode === 'control') {
        scheduleControlCandidateRefresh(sessionId, point);
      }
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
      if (state.status !== 'selecting') return false;
      if ((pointer.button ?? 0) !== 0) return false;
      if (!state.startPoint) {
        clearPressedHoverSelection();
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
        captureCandidates:
          pressedHoverDetectionMode === 'control' && pressedHoverSelection
            ? [
                ...candidates,
                {
                  id: 'active-control-candidate',
                  kind: 'control',
                  rect: pressedHoverSelection,
                  priority: Number.MAX_SAFE_INTEGER,
                },
              ]
            : candidates,
        activeHoverSelection: pressedHoverSelection,
        minSelectionSize: MIN_SELECTION_SIZE,
      });
      const manualSelection = normalizeSelection(state.startPoint, releasePoint);
      const isManualSelection =
        manualSelection.width >= MIN_SELECTION_SIZE &&
        manualSelection.height >= MIN_SELECTION_SIZE;

      clearPressedHoverSelection();
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

    pointerCancel() {
      clearPressedHoverSelection();
      if (state.status === 'preview') {
        if (
          !state.annotationGesture &&
          !state.draftAnnotation &&
          !state.annotationMoveGesture &&
          !state.editGesture
        ) {
          return false;
        }
        keyboardEditCursorPointRef.current = null;
        patch({
          annotationGesture: null,
          draftAnnotation: null,
          annotationMoveGesture: null,
          editGesture: null,
        });
        return true;
      }
      if (state.status !== 'selecting' || !state.startPoint) return false;

      hasKeyboardAdjustedDraft = false;
      patch({
        startPoint: null,
        selection: null,
        hoverSelection: null,
      });
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

    resizeAnnotationPointerDown(handle: SelectionHandle, input) {
      if (
        state.status !== 'preview' ||
        !state.selection ||
        state.selectedAnnotationIndex === null
      ) {
        return false;
      }
      const annotation =
        state.annotationHistory.annotations[state.selectedAnnotationIndex];
      if (
        !annotation ||
        (annotation.type !== 'rectangle' &&
          annotation.type !== 'ellipse')
      ) {
        return false;
      }
      const pointer = pointerInput(input);
      const localPoint = {
        x: pointer.point.x - state.selection.x,
        y: pointer.point.y - state.selection.y,
      };
      patch({
        annotationMoveGesture: {
          annotationIndex: state.selectedAnnotationIndex,
          startPoint: localPoint,
          startAnnotation: annotation,
          resizeHandle: handle,
        },
        draftAnnotation: annotation,
      });
      return true;
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
      patch({ textDraft: null, textDraftAnnotationIndex: null });
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
    commitAnnotationSizeDefault(kind, value) {
      persistScreenshotDefaults?.(
        kind === 'font'
          ? { defaultFontSize: value }
          : { defaultStrokeWidth: value },
      );
    },
    undoAnnotation,
    redoAnnotation,
    updateCursorColor(cursorColor) {
      if (colorSamplesEqual(state.cursorColor, cursorColor)) return;
      patch({ cursorColor });
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
      const screenSelectionScope =
        getCaptureScreenSelectionScopeFromShortcut(event);
      if (
        screenSelectionScope &&
        !state.textDraft &&
        state.session &&
        (state.status === 'selecting' || state.status === 'preview')
      ) {
        const point =
          state.cursorPoint ??
          state.session.captured_cursor?.logical_position ??
          null;
        const rect =
          screenSelectionScope === 'virtual-desktop'
            ? getVirtualDesktopBounds(state.session.monitors)
            : getCurrentMonitorBounds(state.session.monitors, point);
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
        const sessionId = state.session.id;
        launch(() => moveCaptureCursor(sessionId, cursorNudgeDelta));
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
        const candidateDetectionMode = state.candidateDetectionMode;
        patch({
          cursorPoint,
          hoverSelection:
            candidateDetectionMode === 'window'
              ? getBestCandidateAtPoint(
                  buildCaptureCandidates(
                    state.session.monitors,
                    state.session.candidates,
                  ),
                  cursorPoint,
                )?.rect ?? null
              : null,
        });
        const sessionId = state.session.id;
        launch(async () => {
          if (!(await moveCaptureCursor(sessionId, cursorNudgeDelta))) return;
          if (candidateDetectionMode === 'control') {
            scheduleControlCandidateRefresh(sessionId, cursorPoint);
          }
        });
        return true;
      }
      if (
        state.status === 'selecting' &&
        state.session &&
        isCandidateDetectionModeToggleShortcut(event)
      ) {
        const candidateDetectionMode =
          state.candidateDetectionMode === 'window' ? 'control' : 'window';
        invalidateControlCandidateRefresh();
        const sessionId = state.session.id;
        const cursorPoint =
          state.cursorPoint ??
          state.session.captured_cursor?.logical_position ??
          null;
        patch({
          candidateDetectionMode,
          cursorPoint,
          error: null,
          hoverSelection:
            candidateDetectionMode === 'window' && cursorPoint
              ? windowCandidateAt(cursorPoint)
              : null,
        });
        if (candidateDetectionMode === 'control' && cursorPoint) {
          scheduleControlCandidateRefresh(sessionId, cursorPoint);
        }
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

    async hydrateMagnifierMonitor(monitorId: string) {
      if (disposed) return;
      const sessionId = state.session?.id;
      if (!sessionId) return;
      const hasPixels = () =>
        Boolean(
          state.session?.monitors.find((monitor) => monitor.id === monitorId)
            ?.image_base64,
        );
      if (hasPixels()) return;

      const actionGeneration = generation;
      if (
        magnifierMonitorHydration?.generation === actionGeneration &&
        magnifierMonitorHydration.sessionId === sessionId
      ) {
        await magnifierMonitorHydration.promise;
        if (
          disposed ||
          generation !== actionGeneration ||
          state.session?.id !== sessionId ||
          hasPixels()
        ) {
          return;
        }
      }

      let hydration: MagnifierMonitorHydration;
      const promise = platform.commands
        .hydrateCaptureMonitorSnapshot(sessionId, monitorId)
        .then((snapshot) => {
          if (
            disposed ||
            generation !== actionGeneration ||
            state.session?.id !== sessionId ||
            snapshot.id !== monitorId
          ) {
            return;
          }
          patch({
            session: {
              ...state.session,
              monitors: state.session.monitors.map((monitor) =>
                monitor.id === monitorId ? snapshot : monitor,
              ),
            },
          });
        })
        .finally(() => {
          if (magnifierMonitorHydration === hydration) {
            magnifierMonitorHydration = null;
          }
        });
      hydration = {
        generation: actionGeneration,
        sessionId,
        monitorId,
        promise,
      };
      magnifierMonitorHydration = hydration;
      await promise;
    },

    async hydrateSnapshots() {
      if (disposed) return;
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
          if (disposed) return;
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
        candidateDetectionMode: state.candidateDetectionMode,
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
        silentOcrHint: state.silentOcrHint,
        hasHydratedPixelSource:
          state.session !== null && hydratedSessionId === state.session.id,
        error: state.error,
      };
    },
    actions,
    subscribe(listener) {
      if (disposed) return () => undefined;
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispose,
  };
}

function createInitialState(
  mode: CaptureMode = 'screenshot',
  previous?: Pick<RuntimeState, 'annotationStyle' | 'textFontSize'>,
  preferences?: CaptureScreenshotPreferences,
): RuntimeState {
  const initial = createInitialCaptureWorkspaceState();
  return {
    ...initial,
    mode,
    annotationStyle: {
      ...(previous?.annotationStyle ?? initial.annotationStyle),
      strokeWidth:
        preferences?.defaultStrokeWidth ??
        previous?.annotationStyle.strokeWidth ??
        initial.annotationStyle.strokeWidth,
    },
    textFontSize:
      preferences?.defaultFontSize ?? previous?.textFontSize ?? initial.textFontSize,
  };
}

function captureSavePathOptions(preferences?: CaptureScreenshotPreferences) {
  if (!preferences) return undefined;
  return {
    directory: preferences.savePath,
    format: preferences.format,
    namingRule: preferences.namingRule,
    customFileName: preferences.customFileName,
  };
}

function rememberedAnnotationTool(
  preferences: CaptureScreenshotPreferences | undefined,
  storage: CaptureSelectionStorage | undefined,
): AnnotationTool | null {
  if (!preferences?.rememberLastTool) return null;
  const tool = storage?.getItem('snaplingo.capture.lastAnnotationTool');
  return tool && isAnnotationTool(tool) ? tool : null;
}

function isAnnotationTool(value: string): value is AnnotationTool {
  return [
    'rectangle',
    'ellipse',
    'arrow',
    'line',
    'pen',
    'highlight',
    'mosaic',
    'text',
    'eraser',
  ].includes(value);
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function arePointsEqual(a: Point | null, b: Point | null) {
  return a === b || (a !== null && b !== null && a.x === b.x && a.y === b.y);
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
