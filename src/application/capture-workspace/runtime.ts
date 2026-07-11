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
  type CaptureSelectionStorage,
} from '../../views/CaptureWorkspace/captureHostRuntime';
import { printBase64PngImage } from '../../views/CaptureWorkspace/capturePrint';
import {
  normalizeSelection,
  snapPointToRects,
} from '../../views/CaptureWorkspace/selection';
import {
  planCaptureDraftSelectionCommit,
  planCaptureDraftSelectionPointerMove,
  planCaptureDraftSelectionStart,
} from '../../views/CaptureWorkspace/captureSelectionRuntime';
import { shouldRevealCaptureWindow } from '../../views/CaptureWorkspace/captureWindowVisibility';
import { normalizeOcrText } from '../../utils/ocrTextProcessing';
import type {
  CaptureWorkspaceRenderState,
  CaptureWorkspaceRuntime,
  CaptureWorkspacePointerInput,
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
    addEventListener(type: string, listener: EventListener): void;
    removeEventListener(type: string, listener: EventListener): void;
  };
  onUnhandledKeyDown(event: KeyboardEvent): void;
  releaseMagnifierRequest(): void;
  hasDraftSelectionMoveGesture(): boolean;
  finishDraftSelectionMove(): void;
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
  let perfState: CaptureFrontendPerfState | null = null;

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

  const resetSession = () => {
    state = createInitialState(state.mode);
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
    cancelNativeSession: () => Promise<void>,
  ) => {
    if (generation !== actionGeneration) {
      await cancelNativeSession();
      return;
    }

    await (onInactive ? onInactive() : platform.dismiss());
    if (generation !== actionGeneration) {
      await cancelNativeSession();
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

      await cancelNativeSession();
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
      await finishSession(actionGeneration, cancelNativeSession);
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
          await cancelNativeSession();
          return;
        }
      }
    } catch (error) {
      if (generation === actionGeneration) {
        patch({ status: 'error', error: errorMessage(error) });
      } else {
        await cancelNativeSession().catch(() => undefined);
      }
    } finally {
      if (generation === actionGeneration) {
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
        await cancelNativeSession();
        return;
      }
      patch({ previewImageBase64 });
    } catch (error) {
      if (generation === actionGeneration) {
        patch({ status: 'error', error: errorMessage(error) });
      } else {
        await cancelNativeSession().catch(() => undefined);
      }
    } finally {
      if (generation === actionGeneration) {
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

  const cancelSession = async () => {
    const sessionId = state.session?.id;
    if (!sessionId) {
      try {
        await (onInactive ? onInactive() : platform.dismiss());
        resetSession();
      } catch (error) {
        patch({ status: 'error', error: errorMessage(error) });
      }
      return;
    }
    const actionGeneration = generation;
    const cancelNativeSession = createNativeSessionCancellation(sessionId);

    try {
      await finishSession(actionGeneration, cancelNativeSession);
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
      const handleKeyDown = (event: Event) => {
        const keyboardEvent = event as KeyboardEvent;
        void actions.keyDown(keyboardEvent).then((handled) => {
          if (handled) {
            keyboardEvent.preventDefault();
            return;
          }
          keyboard?.onUnhandledKeyDown(keyboardEvent);
        });
      };
      const handleKeyUp = (event: Event) => {
        const keyboardEvent = event as KeyboardEvent;
        const action = getCaptureKeyboardKeyUpAction(keyboardEvent, {
          hasDraftSelectionMoveGesture:
            keyboard?.hasDraftSelectionMoveGesture() ?? false,
        });
        if (action === 'release-magnifier-request') {
          keyboard?.releaseMagnifierRequest();
        } else if (action === 'finish-draft-selection-move') {
          keyboardEvent.preventDefault();
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
          void cancelSession();
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
          await platform.onCancelRequested(() => cancelSession()),
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
        unlisten.forEach((dispose) => dispose());
        patch({ status: 'error', error: errorMessage(error) });
        return () => undefined;
      }

      return () => unlisten.forEach((dispose) => dispose());
    },

    async updateHostReadiness(imagesReady) {
      const sessionId = state.session?.id ?? null;
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

      try {
        await platform.prepareForReveal();
        await host?.prepareSurface();
        await platform.reveal();
        hasRevealed = true;
        if (sessionId) markPerf('revealed', sessionId);
      } catch (error) {
        patch({ status: 'error', error: errorMessage(error) });
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
      host?.resetInteraction();
      hasRevealed = false;
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
        if (generation !== actionGeneration) return;
        if (perfState) perfState.sessionId = session.id;
        markPerf('session_loaded', session.id);

        const cursorPoint =
          session.captured_cursor?.logical_position ??
          (await platform.commands
            .currentCaptureCursorPosition(session.id)
            .catch(() => null));
        if (generation !== actionGeneration) return;

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
      host?.resetInteraction();
      hasRevealed = false;
      state = {
        ...createInitialState(state.mode),
        status: 'loading',
      };
      listeners.forEach((listener) => listener());
      hydratedSessionId = null;
      snapshotHydration = null;

      try {
        const session = await platform.commands.createCaptureSession();
        await platform.commands.cancelCaptureSession(previousSessionId);
        if (generation !== actionGeneration) {
          await platform.commands.cancelCaptureSession(session.id);
          return;
        }
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
      if (state.status !== 'selecting' || !state.session) return;
      const { button = 0, point } = pointerInput(input);
      if (button === 2) {
        if (state.selection) {
          patch({ startPoint: null, selection: null, hoverSelection: null });
        } else {
          void cancelSession();
        }
        return;
      }

      const draftStart = planCaptureDraftSelectionStart({
        cursorPoint: point,
        anchorPoint: snapPointToRects(
          point,
          host?.getSnapTargetRects() ?? [],
          6,
        ),
      });

      patch({
        startPoint: draftStart.nextState.startPoint,
        cursorPoint: draftStart.nextState.cursorPoint,
        selection: draftStart.nextState.selection,
        hoverSelection: draftStart.nextState.hoverSelection,
        previewImageBase64: draftStart.nextState.previewImageBase64,
        isRenderingOutput: draftStart.nextState.renderingOutput,
      });
    },

    pointerMove(input) {
      if (state.status !== 'selecting' || !state.session) return;
      const { point, shiftKey = false } = pointerInput(input);

      if (state.startPoint) {
        const draft = planCaptureDraftSelectionPointerMove({
          anchorPoint: state.startPoint,
          point,
          snapTargetRects: host?.getSnapTargetRects() ?? [],
          edgeSnapThreshold: 6,
          constrainSelection: shiftKey,
        });
        patch({
          cursorPoint: point,
          selection: draft.draftSelection,
        });
        return;
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
    },

    async pointerUp(input) {
      if (state.status !== 'selecting' || !state.session || !state.startPoint) {
        return;
      }
      const { point, shiftKey = false } = pointerInput(input);

      const candidates = buildCaptureCandidates(
        state.session.monitors,
        state.session.candidates,
      );
      const draftCommit = planCaptureDraftSelectionCommit({
        anchorPoint: state.startPoint,
        releasePoint: point,
        snapTargetRects: host?.getSnapTargetRects() ?? [],
        edgeSnapThreshold: 6,
        constrainSelection: shiftKey,
        captureCandidates: candidates,
        activeHoverSelection: state.hoverSelection,
        minSelectionSize: MIN_SELECTION_SIZE,
      });
      const manualSelection = normalizeSelection(state.startPoint, point);
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
    },

    async keyDown({ key }) {
      if (key === 'Escape') {
        if (
          state.status !== 'selecting' ||
          state.startPoint ||
          state.selection
        ) {
          return false;
        }
        await cancelSession();
        return true;
      }

      if (key === 'Enter' && state.hoverSelection) {
        await completeCandidateSelection(state.hoverSelection);
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
    isRenderingOutput: false,
    error: null,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function pointerInput(
  input: Point | CaptureWorkspacePointerInput,
): CaptureWorkspacePointerInput {
  return 'point' in input ? input : { point: input };
}
