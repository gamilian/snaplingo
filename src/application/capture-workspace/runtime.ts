import type { CaptureWorkspacePorts } from './ports';
import type {
  AnnotationCommand,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from '../../domain/capture';
import {
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
  getBestCandidateAtPoint,
} from './captureCandidates';
import {
  planCandidateSelectionCompletion,
  planManualSelectionCompletion,
  type CaptureRuntimeEffect,
} from './captureInteractionRuntime';
import {
  recordSuccessfulCaptureSelection,
  restoreCaptureSelectionFromHistory,
  restoreLastSuccessfulCaptureSelection,
  type CaptureSelectionStorage,
} from './captureSelectionHistory';
import { loadCaptureSelectionHistory, storeCaptureSelectionHistory } from './selectionMemory';
import { emptyAnnotationHistory } from './annotationHistory';
import type { AnnotationTool } from './annotationStyle';
import { isScrollingCaptureShortcut, annotationShortcutBindings, normalizeEditorShortcutKey } from './annotationShortcuts';
import { planCaptureManualSelectionTransition } from './captureEditorRuntime';
import { createCaptureEditing } from './editing';
import {
  getCaptureCandidateRectsForSession,
  getCaptureCandidatesForSession,
} from './captureWorkspaceDerived';
import {
  createInitialCaptureWorkspaceState,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import {
  colorSamplesEqual,
} from '../image-inspection/colorSampler';
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
import { captureMonitorImageSource } from './captureSnapshot';
import {
  getCurrentMonitorBounds,
  getVirtualDesktopBounds,
} from './virtualDesktop';
import type { OcrSettings } from '../settings/ports';
import type {
  CaptureWorkspaceRuntime,
  CaptureWorkspacePointerInput,
  CaptureWorkspaceKeyInput,
  CaptureWorkspaceKeyboardEvent,
} from './captureWorkspaceRuntimeTypes';

const MIN_SELECTION_SIZE = 2;

type RuntimeState = CaptureWorkspaceState;

interface SnapshotHydration {
  generation: number;
  sessionId: string;
  promise: Promise<void>;
}

interface MagnifierMonitorHydration extends SnapshotHydration {
  monitorId: string;
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
  prepareSurface(windowPrepared: Promise<void>): void | Promise<void>;
  scheduleSelectionOverlayPaint?(): void;
}

interface CaptureWorkspaceRuntimeKeyboard {
  isEditingText?(event: CaptureWorkspaceKeyboardEvent): boolean;
  target: {
    addEventListener(
      type: 'keydown',
      listener: (event: CaptureWorkspaceKeyboardEvent) => void,
      capture?: boolean,
    ): void;
    addEventListener(
      type: 'keyup',
      listener: (event: CaptureWorkspaceKeyboardEvent) => void,
    ): void;
    addEventListener(type: 'blur', listener: () => void): void;
    removeEventListener(
      type: 'keydown',
      listener: (event: CaptureWorkspaceKeyboardEvent) => void,
      capture?: boolean,
    ): void;
    removeEventListener(
      type: 'keyup',
      listener: (event: CaptureWorkspaceKeyboardEvent) => void,
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
  namingRule: 'timestamp' | 'unix-ms' | 'date' | 'counter' | 'custom';
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
  captureUiScale?: import('../settings/ports').CaptureUiScale;
  annotationToolShortcuts?: Record<string, string>;
}

export function createCaptureWorkspaceRuntime({
  platform,
  host,
  keyboard,
  onInactive,
  annotationColorPresets,
  screenshotPreferences,
  persistScreenshotDefaults,
  persistAnnotationColors,
  ocrPreferences,
  storage,
}: {
  platform: CaptureWorkspacePorts;
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
  persistAnnotationColors?: (colors: AnnotationColor[]) => void | Promise<unknown>;
  ocrPreferences?: () => OcrSettings | undefined;
  storage?: CaptureSelectionStorage;
}): CaptureWorkspaceRuntime {
  let state = createInitialState('screenshot', undefined, screenshotPreferences?.());
  let lastAnnotationTool: AnnotationTool = 'rectangle';
  let toolWrite = Promise.resolve();
  const initialAnnotationTool = () =>
    screenshotPreferences?.()?.rememberLastTool ? lastAnnotationTool : 'rectangle';
  let generation = 0;
  let disposed = false;
  let hydratedSessionId: string | null = null;
  let magnifierMonitorHydration: MagnifierMonitorHydration | null = null;
  let snapshotHydration: SnapshotHydration | null = null;
  const listeners = new Set<() => void>();
  let hasRevealed = false;
  let revealAttempt: { key: string; promise: Promise<void> } | null = null;
  let spacePressed = false;
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
    keyboardEditCursorPoint: editing.captureInteraction(),
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
    editing.restoreInteraction(snapshot.keyboardEditCursorPoint);
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
    if (next.activeAnnotationTool && next.activeAnnotationTool !== state.activeAnnotationTool) {
      lastAnnotationTool = next.activeAnnotationTool;
      if (screenshotPreferences?.()?.rememberLastTool && platform.commands.rememberAnnotationTool) {
        const tool = next.activeAnnotationTool;
        toolWrite = toolWrite.then(() => platform.commands.rememberAnnotationTool!(tool)).catch(() => {});
      }
    }
    state = { ...state, ...next };
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

  const hasCurrentTerminalOutput = () =>
    terminalOutputOperation?.generation === generation &&
    terminalOutputOperation.sessionId === state.session?.id;

  const dispose = () => {
    if (disposed) return;
    disposed = true;
    generation += 1;
    const activeSessionId = state.session?.id ?? null;
    terminalOutputOperation = null;
    revealAttempt = null;
    hasRevealed = false;
    perfState = null;
    snapshotHydration = null;
    hydratedSessionId = null;
    hasKeyboardAdjustedDraft = false;
    clearPressedHoverSelection();
    editing.restoreInteraction(null);
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
    terminalOutputOperation = null;
    spacePressed = false;
    state = createInitialState(state.mode, state, screenshotPreferences?.());
    editing.restoreInteraction(null);
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
    completeOcr = false,
  ) => {
    if (generation !== actionGeneration) {
      if (state.session?.id !== sessionId) await cancelNativeSession();
      return;
    }

    if (completeOcr) {
      // The backend owns both window cleanup and the OCR job from this point on.
      await platform.commands.completeCaptureOcr(sessionId);
      provisionalSessionIds.delete(sessionId);
      if (generation === actionGeneration) resetSession();
      return;
    }
    await (onInactive ? onInactive() : platform.window.hide());
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
    completeOcr: boolean,
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
        await platform.print.printImage(imageBase64);
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
                    rememberChoice: true,
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
      await platform.commands.prepareCaptureOcr({
        sessionId,
        rect,
        annotations,
        target: effect.target,
        ...(recognitionLanguage ? { language: recognitionLanguage } : {}),
      });
      return;
    }

    if (effect.type === 'record-selection') {
      if (storage) {
        recordSuccessfulCaptureSelection(storage, effect.action, rect);
      }
      await platform.commands.rememberCaptureSelection?.(rect).catch(() => undefined);
      return;
    }

    if (effect.type === 'finish-session') {
      await finishSession(
        actionGeneration,
        sessionId,
        cancelNativeSession,
        completeOcr,
      );
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
    const completeOcr = effects.some((effect) => effect.type === 'run-ocr');
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
          completeOcr,
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
        patch({ isRenderingOutput: false });
      }
    }
  };

  const renderSelectionPreview = async (
    rect: LogicalRect,
    _annotations: AnnotationCommand[] = [],
    includeCursor = false,
  ) => {
    if (!state.session || hasCurrentTerminalOutput()) return;
    const activeAnnotationTool =
      state.status === 'preview'
        ? state.activeAnnotationTool
        : initialAnnotationTool();
    // The view crops the already decoded frozen monitors. Encoding is only
    // needed for terminal output, so entering or resizing preview never blocks.
    patch({
      status: 'preview',
      selection: rect,
      hoverSelection: null,
      activeAnnotationTool,
      includeCapturedCursor: includeCursor,
      isRenderingOutput: false,
      error: null,
    });
  };

  const editing = createCaptureEditing({
    annotationToolShortcuts: () => screenshotPreferences?.()?.annotationToolShortcuts,
    read: () => state,
    update: patch,
    hydratedSessionId: () => hydratedSessionId,
    renderSelectionPreview,
    cancelSession: () => launch(cancelSession),
    copyText: platform.clipboard.writeText,
    annotationColorPresets,
  });

  const completeCandidateSelection = async (
    rect: LogicalRect,
    action?: HoverSelectionCompletionAction,
  ) => {
    // A plain click/Enter on a smart candidate should follow the same flow as a
    // manual drag: enter the preview/edit page for screenshot modes, or run the
    // mode's effects otherwise. Only an explicit action (Cmd+C/S, hover
    // shortcuts) bypasses preview and completes immediately.
    if (action === undefined) {
      await completeManualSelection(rect);
      return;
    }
    await runCompletionEffects(
      rect,
      planCandidateSelectionCompletion(action),
    );
  };

  const completeManualSelection = async (rect: LogicalRect) => {
    const completion = planManualSelectionCompletion(state.mode);
    const transition = planCaptureManualSelectionTransition({
      rect,
      completion,
      activeAnnotationTool:
        completion.type === 'preview' && state.status !== 'preview'
          ? initialAnnotationTool()
          : state.activeAnnotationTool,
    });
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
          getCaptureCandidatesForSession(state.session),
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
    const sessionsToCancel = new Set(provisionalSessionIds);
    if (state.session) sessionsToCancel.add(state.session.id);
    for (const sessionId of sessionsToCancel) provisionalSessionIds.add(sessionId);
    let failure: string | null = null;

    try {
      await (onInactive ? onInactive() : platform.window.hide());
      if (generation === actionGeneration) resetSession();
    } catch (error) {
      failure = errorMessage(error);
    }
    // Hiding is independent of native cleanup, including failed replacements.
    for (const sessionId of sessionsToCancel) {
      if (generation !== actionGeneration && state.session?.id === sessionId) continue;
      try {
        await cancelNativeSessionOnce(sessionId);
        provisionalSessionIds.delete(sessionId);
      } catch (error) {
        failure ??= errorMessage(error);
      }
    }
    if (generation !== actionGeneration) return;
    if (failure !== null) patch({ status: 'error', error: failure });
  };

  const actions: CaptureWorkspaceRuntime['actions'] = {
    async connectHost(onHotkeySubscriptionReady?: () => void | Promise<void>) {
      if (disposed) return () => undefined;
      const connection = createHostConnection();
      const handleKeyDown = (event: CaptureWorkspaceKeyboardEvent) => {
        if (keyboard?.isEditingText?.(event)) return;
        if (actions.keyDown(event)) {
          event.preventDefault();
        }
      };
      const handleKeyUp = (event: CaptureWorkspaceKeyboardEvent) => {
        if (event.key === ' ') spacePressed = false;
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
        spacePressed = false;
        const plan = planCaptureKeyboardBlur({
          status: state.status,
          isRenderingOutput: state.isRenderingOutput,
        });
        if (plan.releaseMagnifierRequest) {
          patch({ isMagnifierRequested: false });
        }
        if (plan.cancelSession) {
          const id = state.session?.id;
          const ownsFocus = platform.commands.captureEditorOwnsFocus;
          if (state.status === 'preview' && id && ownsFocus) {
            const blurGeneration = generation;
            launch(async () => {
              const owned = await ownsFocus(id).catch(() => false);
              if (owned || disposed || connection.isClosed || generation !== blurGeneration ||
                  state.session?.id !== id || state.status !== 'preview' || state.isRenderingOutput) return;
              await cancelSession();
            });
          } else {
            launch(cancelSession);
          }
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
          await platform.events.subscribeHotkeyTriggered((launch) => {
            if (disposed || connection.isClosed) return;
            return actions.startSession(launch.mode, launch.sessionId);
          }),
        );
        if (!disposed && !connection.isClosed && platform.events.subscribeScrollingCaptureEnded) {
          connection.retain(await platform.events.subscribeScrollingCaptureEnded(({ sessionId, completed }) => {
            if (disposed || connection.isClosed || state.status !== 'scrolling' || state.session?.id !== sessionId) return;
            if (completed && state.selection) {
              if (storage) recordSuccessfulCaptureSelection(storage, 'copy', state.selection);
              void platform.commands.rememberCaptureSelection?.(state.selection).catch(() => undefined);
            }
            generation += 1;
            resetSession();
          }));
        }
        if (!disposed && !connection.isClosed && platform.events.subscribePinRequested) {
          connection.retain(await platform.events.subscribePinRequested(({ sessionId }) => {
            if (disposed || connection.isClosed || state.session?.id !== sessionId || state.isRenderingOutput) return;
            if (state.status === 'preview' && state.selection) launch(() => actions.completePreviewSelection('pin', state.selection!));
            else if (state.status === 'selecting' && state.hoverSelection && !state.startPoint) launch(() => completeCandidateSelection(state.hoverSelection!, 'pin'));
          }));
        }
        if (!disposed && !connection.isClosed && platform.events.subscribeEditorToolbarAction) {
          connection.retain(await platform.events.subscribeEditorToolbarAction((event) => {
            const { sessionId, action } = event;
            if (disposed || connection.isClosed || state.session?.id !== sessionId ||
                state.status !== 'preview' || !state.selection || state.isRenderingOutput) return;
            if (event.action === 'presets') {
              const presetGeneration = generation;
              launch(async () => {
                if (presetGeneration === generation) patch({ error: null });
                try {
                  if (!persistAnnotationColors) throw new Error('颜色预设保存未配置');
                  await persistAnnotationColors(event.colors);
                } catch (error) {
                  if (!disposed && presetGeneration === generation && state.session?.id === sessionId) {
                    patch({ error: errorMessage(error) });
                  }
                }
              });
              return;
            }
            if (event.action === 'style') {
              const style = { ...state.annotationStyle };
              let fontSize = state.textFontSize;
              switch (event.property) {
                case 'color': style.color = event.value; break;
                case 'stroke-width': style.strokeWidth = event.value; break;
                case 'font-size': fontSize = event.value; break;
                case 'filled': style.filled = event.value; break;
                case 'corner-radius': style.cornerRadius = event.value; break;
                case 'blur-radius': style.blurRadius = event.value; break;
                case 'font-family': style.fontFamily = event.value; break;
                case 'bold': style.bold = event.value; break;
                case 'italic': style.italic = event.value; break;
              }
              actions.applySelectedAnnotationStyle(style, fontSize);
              if (state.textDraft && event.property === 'font-size') actions.updateTextDraftFontSize(fontSize);
              if (event.commit && event.property === 'stroke-width') actions.commitAnnotationSizeDefault('stroke', event.value);
              if (event.commit && event.property === 'font-size') actions.commitAnnotationSizeDefault('font', event.value);
              return;
            }
            switch (action) {
              case 'select': actions.selectMoveTool(); break;
              case 'rectangle': case 'ellipse': case 'arrow': case 'line':
                if (state.activeAnnotationTool !== action) actions.toggleAnnotationTool(action);
                break;
              case 'pen': case 'text': case 'mosaic': case 'eraser': case 'highlight':
              case 'region-highlight': case 'blur': case 'spotlight':
                actions.toggleAnnotationTool(action); break;
              case 'undo': actions.undoAnnotation(); break;
              case 'redo': actions.redoAnnotation(); break;
              case 'clear': actions.clearAnnotations(); break;
              case 'cancel': launch(cancelSession); break;
              case 'scrolling': launch(actions.beginScrollingCapture); break;
              case 'ocr': case 'copy': case 'save': case 'quick-save': case 'pin': case 'favorite':
                launch(() => actions.completePreviewSelection(action, state.selection!)); break;
            }
          }));
        }
        if (!disposed && !connection.isClosed) {
          if (storage && platform.commands.captureSelectionHistory) {
            try {
              const history = await platform.commands.captureSelectionHistory(loadCaptureSelectionHistory(storage));
              if (!disposed && !connection.isClosed) storeCaptureSelectionHistory(storage, history);
            } catch { /* Capture remains usable if selection history cannot be read. */ }
          }
          await onHotkeySubscriptionReady?.();
        }
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
          generation === readinessGeneration && (state.session?.id ?? null) === sessionId;
        const promise = (async () => {
          try {
            const windowPrepared = platform.window.prepareForReveal(sessionId).then(() => {
              if (isCurrent()) markPerf('window_prepared', sessionId);
            });
            await Promise.all([windowPrepared, host?.prepareSurface(windowPrepared)]);
            if (!isCurrent()) return;
            markPerf('surface_decoded', sessionId);
            await platform.window.reveal(sessionId);
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
    async beginScrollingCapture() {
      if (disposed || state.status !== 'preview' || state.isRenderingOutput || !state.session || !state.selection) return;
      const sessionId = state.session.id;
      const rect = state.selection;
      const startGeneration = generation;
      patch({ status: 'scrolling', error: null });
      try {
        if (!platform.commands.beginScrollingCapture) throw new Error('长截图功能不可用');
        await platform.commands.beginScrollingCapture(sessionId, rect);
      } catch (error) {
        if (!disposed && startGeneration === generation && state.session?.id === sessionId) patch({ status: 'preview', error: errorMessage(error) });
      }
    },

    async startSession(mode, requestedSessionId) {
      if (disposed) return;
      // The native presentation already exists even if reading its metadata fails.
      if (requestedSessionId) provisionalSessionIds.add(requestedSessionId);
      const previousSessionId = state.session?.id ?? null;
      for (const sessionId of provisionalSessionIds) {
        if (sessionId === previousSessionId || sessionId === requestedSessionId) continue;
        void cancelNativeSessionOnce(sessionId).then(
          () => provisionalSessionIds.delete(sessionId),
          () => undefined,
        );
      }
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
        const [session, rememberedTool, editorSelection] = await Promise.all([
          requestedSessionId
            ? platform.commands.getCaptureSession(requestedSessionId)
            : platform.commands.createCaptureSession(),
          screenshotPreferences?.()?.rememberLastTool
            ? toolWrite.then(() => platform.commands.lastAnnotationTool?.()).catch(() => null)
            : Promise.resolve(null),
          requestedSessionId ? platform.commands.captureEditorSelection?.(requestedSessionId) : null,
        ]);
        provisionalSessionIds.add(session.id);
        if (generation !== actionGeneration) {
          provisionalSessionIds.delete(session.id);
          await cancelNativeSessionOnce(session.id).catch(() => undefined);
          await cancelStalePrevious();
          return;
        }
        if (editorSelection && (mode !== 'screenshot' || editorSelection.sessionId !== session.id)) {
          throw new Error('截图编辑选区所属会话已失效');
        }
        if (rememberedTool) lastAnnotationTool = rememberedTool;
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
        hydratedSessionId = hasCaptureSessionPixelSource(session)
          ? session.id
          : null;

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
                getCaptureCandidatesForSession(session),
                cursorPoint,
              )?.rect ?? null
            : null,
        });
        if (editorSelection) await renderSelectionPreview(editorSelection.rect, [], editorSelection.includeCursor);
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
        hydratedSessionId = hasCaptureSessionPixelSource(session)
          ? session.id
          : null;
        patch({
          status: 'selecting',
          session,
          cursorPoint,
          hoverSelection: cursorPoint
            ? getBestCandidateAtPoint(
                getCaptureCandidatesForSession(session),
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
          : editing.commitTextDraftToHistory().annotations);
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
        isRenderingOutput: false,
        error: null,
      });
    },

    pointerDown(input) {
      if (!state.session) return false;
      if (state.status === 'selecting') markPerf('selection_pointer_down');
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
        if (source === 'root' && button === 0 && state.selection && !rectContainsPoint(state.selection, point) && !state.isRenderingOutput) {
          const previousStyle = state.annotationStyle;
          const previousTool = state.activeAnnotationTool;
          actions.resetPreview();
          patch({ annotationStyle: previousStyle, activeAnnotationTool: previousTool });
          return actions.pointerDown(pointer);
        }
        if (button === 0 && (spacePressed || pointer.spaceKey) && state.selection && rectContainsPoint(state.selection, point) && !state.textDraft && !state.annotationGesture && !state.annotationMoveGesture) {
          patch({ cursorPoint: point, editGesture: { type: 'move', startPoint: point, startSelection: state.selection } });
          return true;
        }
        if (source === 'preview' && button === 1 && state.selection) {
          const selection = state.selection;
          launch(() =>
            runCompletionEffects(
              selection,
              planCandidateSelectionCompletion('pin'),
              editing.commitTextDraftToHistory().annotations,
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
          !state.activeAnnotationTool &&
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
        if (source === 'root' && button === 2 && !editing.hasDismissibleLayer()) {
          actions.resetPreview();
          return true;
        }
        return editing.pointerDown(pointer);
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

      const hoverSelectionAtPress =
        state.hoverSelection && rectContainsPoint(state.hoverSelection, point)
          ? state.hoverSelection
          : null;
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
          ? hoverSelectionAtPress
          : null;
      pressedHoverDetectionMode = pressedHoverSelection
        ? state.candidateDetectionMode
        : null;
      const draftStart = planCaptureDraftSelectionStart({
        cursorPoint: point,
        anchorPoint: snapPointToRects(
          point,
          getCaptureCandidateRectsForSession(state.session),
          6,
        ),
      });

      hasKeyboardAdjustedDraft = false;
      patch({
        startPoint: draftStart.nextState.startPoint,
        cursorPoint: draftStart.nextState.cursorPoint,
        selection: draftStart.nextState.selection,
        hoverSelection: draftStart.nextState.hoverSelection,
        isRenderingOutput: draftStart.nextState.renderingOutput,
      });
      return true;
    },

    pointerMove(input) {
      if (!state.session) return false;
      const pointer = pointerInput(input);
      if (state.status === 'preview') {
        return editing.pointerMove(pointer);
      }
      if (state.status !== 'selecting') return false;
      const { point, shiftKey = false } = pointer;

      if (state.startPoint) {
        const draft = planCaptureDraftSelectionPointerMove({
          anchorPoint: state.startPoint,
          point,
          snapTargetRects: getCaptureCandidateRectsForSession(state.session),
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
      const retainedControlCandidate =
        state.hoverSelection && rectContainsPoint(state.hoverSelection, point)
          ? state.hoverSelection
          : null;
      patch({
        cursorPoint: point,
        hoverSelection:
          candidateDetectionMode === 'window'
            ? getBestCandidateAtPoint(
                getCaptureCandidatesForSession(state.session),
                point,
              )?.rect ?? null
            : retainedControlCandidate,
      });
      if (candidateDetectionMode === 'control') {
        scheduleControlCandidateRefresh(sessionId, point);
      }
      return true;
    },

    async pointerUp(input) {
      if (!state.session) return false;
      if (state.status === 'selecting') markPerf('selection_pointer_up');
      const pointer = pointerInput(input);
      if (state.status === 'preview') {
        return editing.pointerUp(pointer);
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

      const candidates = getCaptureCandidatesForSession(state.session);
      const draftCommit = planCaptureDraftSelectionCommit({
        anchorPoint: state.startPoint,
        releasePoint,
        snapTargetRects: getCaptureCandidateRectsForSession(state.session),
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
      if (state.status === 'preview') return editing.cancelPointer();
      if (state.status !== 'selecting' || !state.startPoint) return false;

      hasKeyboardAdjustedDraft = false;
      patch({
        startPoint: null,
        selection: null,
        hoverSelection: null,
      });
      return true;
    },

    resizePointerDown: (handle: SelectionHandle, input) => editing.resizePointerDown(handle, pointerInput(input)),
    resizeAnnotationPointerDown: (handle: import('./annotationGeometry').AnnotationHandle, input) => editing.resizeAnnotationPointerDown(handle, pointerInput(input)),
    wheel: editing.wheel,
    commitTextDraft: editing.commitTextDraft,
    updateTextDraftText: editing.updateTextDraftText,
    discardTextDraft: editing.discardTextDraft,
    selectMoveTool: editing.selectMoveTool,
    toggleAnnotationTool: editing.toggleAnnotationTool,
    applySelectedAnnotationStyle: editing.applySelectedAnnotationStyle,
    updateTextDraftFontSize: editing.updateTextDraftFontSize,
    commitAnnotationSizeDefault(kind, value) {
      persistScreenshotDefaults?.(
        kind === 'font'
          ? { defaultFontSize: value }
          : { defaultStrokeWidth: value },
      );
    },
    undoAnnotation: editing.undoAnnotation,
    redoAnnotation: editing.redoAnnotation,
    clearAnnotations: editing.clearAnnotations,
    updateCursorColor(cursorColor) {
      if (colorSamplesEqual(state.cursorColor, cursorColor)) return;
      patch({ cursorColor });
    },

    keyDown(input: CaptureWorkspaceKeyInput) {
      if (disposed || state.textDraft) return false;
      if (input.isComposing && (!/^(Key[A-Z]|Digit[0-9]|F(?:[1-9]|1[0-9]|2[0-4]))$/.test(input.code ?? '') || input.ctrlKey || input.metaKey || input.altKey)) return false;
      const event = {
        key: input.key,
        code: input.code,
        metaKey: input.metaKey ?? false,
        ctrlKey: input.ctrlKey ?? false,
        altKey: input.altKey ?? false,
        shiftKey: input.shiftKey ?? false,
        repeat: input.repeat ?? false,
      };
      if (event.key === ' ' && !event.ctrlKey && !event.metaKey && !event.altKey) spacePressed = true;
      if (state.status === 'preview' && !state.isRenderingOutput && isScrollingCaptureShortcut(event, screenshotPreferences?.()?.annotationToolShortcuts) && !state.annotationGesture && !state.annotationMoveGesture && !state.editGesture && !state.draftSelectionMoveGesture) {
        launch(() => actions.beginScrollingCapture());
        return true;
      }
      if (
        (state.status === 'selecting' || state.status === 'preview') &&
        isRefreshCaptureShortcut(event) && !annotationShortcutBindings(screenshotPreferences?.()?.annotationToolShortcuts)[normalizeEditorShortcutKey(event)]
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
                  getCaptureCandidatesForSession(state.session),
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
            editing.commitTextDraftToHistory().annotations,
            state.includeCapturedCursor,
          ),
        );
        return true;
      }
      if (editing.keyDown(input)) return true;
      if (event.key === 'Escape') {
        if (state.status !== 'selecting' && state.status !== 'error') return false;
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
          captureMonitorImageSource(state.session?.monitors.find((monitor) => monitor.id === monitorId)),
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

      if (hasCaptureSessionPixelSource(state.session)) {
        hydratedSessionId = sessionId;
        snapshotHydration = null;
        return;
      }

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

          hydratedSessionId = sessionId;
          patch({ session });
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
            if (generation === actionGeneration && state.session?.id === sessionId) {
              patch({ status: 'error', error: errorMessage(error) });
            }
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

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function hasCaptureSessionPixelSource(session: CaptureSessionView | null) {
  return Boolean(
    session &&
      session.monitors.length > 0 &&
      session.monitors.every((monitor) =>
        Boolean(captureMonitorImageSource(monitor)),
      ),
  );
}

function arePointsEqual(a: Point | null, b: Point | null) {
  return a === b || (a !== null && b !== null && a.x === b.x && a.y === b.y);
}

function rectContainsPoint(rect: LogicalRect, point: Point) {
  return (
    point.x >= rect.x &&
    point.x < rect.x + rect.width &&
    point.y >= rect.y &&
    point.y < rect.y + rect.height
  );
}

function pointerInput(
  input: Point | CaptureWorkspacePointerInput,
): CaptureWorkspacePointerInput {
  return 'point' in input ? input : { point: input };
}
