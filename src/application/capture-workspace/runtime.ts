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
  normalizeSelection,
  snapPointToRects,
} from '../../views/CaptureWorkspace/selection';
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
  CaptureWorkspaceRenderState,
  CaptureWorkspaceRuntime,
  CaptureWorkspacePointerInput,
  CaptureWorkspaceKeyInput,
  CaptureWorkspaceRuntimePlatform,
} from './types';

const MIN_SELECTION_SIZE = 10;

interface RuntimeState {
  status: CaptureWorkspaceRenderState['status'];
  mode: CaptureMode;
  session: CaptureSessionView | null;
  cursorPoint: Point | null;
  startPoint: Point | null;
  selection: LogicalRect | null;
  hoverSelection: LogicalRect | null;
  previewImageBase64: string | null;
  includeCapturedCursor: boolean;
  isRenderingOutput: boolean;
  error: string | null;
}

interface SnapshotHydration {
  generation: number;
  sessionId: string;
  promise: Promise<void>;
}

interface CaptureWorkspaceRuntimeHost {
  resetInteraction(): void;
  resetSession(): void;
  applyManualSelection(rect: LogicalRect, mode: CaptureMode): void;
  getAnnotations(): AnnotationCommand[];
  commitTextDraft(): AnnotationCommand[];
  shouldIncludeCursor(): boolean;
  hasTextDraft(): boolean;
  prepareSurface(): void | Promise<void>;
  getSnapTargetRects(): LogicalRect[];
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
  onUnhandledKeyDown(event: KeyboardEvent): void;
  releaseMagnifierRequest(): void;
  hasDraftSelectionMoveGesture(): boolean;
  finishDraftSelectionMove(): void;
  hasDismissibleLayer(): boolean;
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

  const resetSession = () => {
    state = createInitialState(state.mode);
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
    if (!session || state.isRenderingOutput) return;
    const actionGeneration = generation;
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
      if (generation === actionGeneration && state.session?.id === session.id) {
        patch({ isRenderingOutput: false });
      }
    }
  };

  const renderSelectionPreview = async (
    rect: LogicalRect,
    annotations: AnnotationCommand[] = [],
    includeCursor = false,
  ) => {
    const session = state.session;
    if (!session || state.isRenderingOutput) return;
    const actionGeneration = generation;
    const cancelNativeSession = createNativeSessionCancellation(session.id);

    patch({
      status: 'preview',
      selection: rect,
      hoverSelection: null,
      previewImageBase64: null,
      isRenderingOutput: true,
      error: null,
    });

    try {
      const previewImageBase64 = await platform.commands.renderCaptureOutput({
        sessionId: session.id,
        rect,
        annotations,
        ...(includeCursor ? { includeCursor: true } : {}),
      });
      if (generation !== actionGeneration || state.session?.id !== session.id) {
        if (state.session?.id !== session.id) await cancelNativeSession();
        return;
      }
      patch({ previewImageBase64 });
    } catch (error) {
      if (generation === actionGeneration && state.session?.id === session.id) {
        patch({ status: 'error', error: errorMessage(error) });
      } else if (state.session?.id !== session.id) {
        await cancelNativeSession().catch(() => undefined);
      }
    } finally {
      if (generation === actionGeneration && state.session?.id === session.id) {
        patch({ isRenderingOutput: false });
      }
    }
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
    host?.applyManualSelection(rect, state.mode);
    const completion = planManualSelectionCompletion(state.mode);
    if (completion.type === 'preview') {
      await renderSelectionPreview(rect);
      return;
    }

    await runCompletionEffects(rect, completion.effects);
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
          return;
        }
        keyboard?.onUnhandledKeyDown(event);
      };
      const handleKeyUp = (event: KeyboardEvent) => {
        const action = getCaptureKeyboardKeyUpAction(event, {
          hasDraftSelectionMoveGesture:
            keyboard?.hasDraftSelectionMoveGesture() ?? false,
        });
        if (action === 'release-magnifier-request') {
          keyboard?.releaseMagnifierRequest();
        } else if (action === 'finish-draft-selection-move') {
          event.preventDefault();
          keyboard?.finishDraftSelectionMove();
        }
      };
      const handleBlur = () => {
        const plan = planCaptureKeyboardBlur({
          status: state.status,
          isRenderingOutput: state.isRenderingOutput,
        });
        if (plan.releaseMagnifierRequest) {
          keyboard?.releaseMagnifierRequest();
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
                host?.commitTextDraft() ?? host?.getAnnotations() ?? [],
                host?.shouldIncludeCursor() ?? false,
              );
              return;
            }

            if (
              state.status === 'selecting' &&
              !state.startPoint &&
              state.hoverSelection &&
              !host?.hasTextDraft()
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
        ...createInitialState(mode),
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
      hasKeyboardAdjustedDraft = false;
      host?.resetInteraction();
      hasRevealed = false;
      revealAttempt = null;
      state = {
        ...createInitialState(state.mode),
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
    async completePreviewSelection(action, rect, annotations = [], includeCursor = false) {
      await runCompletionEffects(
        rect,
        planCandidateSelectionCompletion(action),
        annotations,
        includeCursor,
      );
    },
    resetPreview() {
      generation += 1;
      hasKeyboardAdjustedDraft = false;
      snapshotHydration = null;
      hydratedSessionId = null;
      host?.resetInteraction();
      patch({
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
      const {
        altKey = false,
        button = 0,
        ctrlKey = false,
        detail = 0,
        metaKey = false,
        point,
        shiftKey = false,
        source = 'root',
      } = pointerInput(input);
      if (state.status === 'preview') {
        if (source === 'preview' && button === 1 && state.selection) {
          const selection = state.selection;
          launch(() =>
            runCompletionEffects(
              selection,
              planCandidateSelectionCompletion('pin'),
              host?.commitTextDraft() ?? host?.getAnnotations() ?? [],
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
          !host?.hasTextDraft() &&
          state.selection
        ) {
          const selection = state.selection;
          launch(() =>
            runCompletionEffects(
              selection,
              planCandidateSelectionCompletion('copy'),
              host?.getAnnotations() ?? [],
              state.includeCapturedCursor,
            ),
          );
          return true;
        }
        if (source === 'root' && button === 2) {
          if (keyboard?.hasDismissibleLayer()) return false;
          actions.resetPreview();
          return true;
        }
        if (source === 'root') {
          actions.resetPreview();
        } else {
          return false;
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
          host?.getSnapTargetRects() ?? [],
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
      if (state.status !== 'selecting' || !state.session) return false;
      const { point, shiftKey = false } = pointerInput(input);

      if (state.startPoint) {
        const draft = planCaptureDraftSelectionPointerMove({
          anchorPoint: state.startPoint,
          point,
          snapTargetRects: host?.getSnapTargetRects() ?? [],
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
      if (state.status !== 'selecting' || !state.session || !state.startPoint) {
        return false;
      }
      const { point, shiftKey = false } = pointerInput(input);
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
        snapTargetRects: host?.getSnapTargetRects() ?? [],
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
        !host?.hasTextDraft() &&
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
              host?.getAnnotations() ?? [],
              includeCapturedCursor,
            ),
          );
        }
        return true;
      }
      const historyStep = getSelectionHistoryStepFromShortcut(event);
      if (
        historyStep &&
        !host?.hasTextDraft() &&
        (state.status === 'selecting' || state.status === 'preview')
      ) {
        restoreSelectionHistory(historyStep);
        return true;
      }
      if (
        isRestoreLastSelectionShortcut(event) &&
        !host?.hasTextDraft() &&
        (state.status === 'selecting' || state.status === 'preview')
      ) {
        restoreLastSelection();
        return true;
      }
      if (
        isSelectAllCaptureShortcut(event) &&
        !host?.hasTextDraft() &&
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
            host?.commitTextDraft() ?? host?.getAnnotations() ?? [],
            state.includeCapturedCursor,
          ),
        );
        return true;
      }
      if (event.key === 'Escape') {
        if (state.status === 'preview') {
          if (keyboard?.hasDismissibleLayer()) return false;
          launch(cancelSession);
          return true;
        }
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

function createInitialState(mode: CaptureMode = 'screenshot'): RuntimeState {
  return {
    status: 'idle',
    mode,
    session: null,
    cursorPoint: null,
    startPoint: null,
    selection: null,
    hoverSelection: null,
    previewImageBase64: null,
    includeCapturedCursor: false,
    isRenderingOutput: false,
    error: null,
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
