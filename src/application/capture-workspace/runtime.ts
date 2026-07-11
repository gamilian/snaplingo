import type {
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from '../../domain/capture';
import {
  buildCaptureCandidates,
  getBestCandidateAtPoint,
  getCandidateForPointerReleaseCompletion,
} from '../../views/CaptureWorkspace/captureCandidates';
import {
  getPrimaryCaptureCompletionActionForMode,
  planCandidateSelectionCompletion,
  type CaptureRuntimeEffect,
} from '../../views/CaptureWorkspace/captureInteractionRuntime';
import { normalizeSelection } from '../../views/CaptureWorkspace/selection';
import type {
  CaptureWorkspaceRenderState,
  CaptureWorkspaceRuntime,
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
  isRenderingOutput: boolean;
  error: string | null;
}

interface SnapshotHydration {
  sessionId: string;
  promise: Promise<void>;
}

export function createCaptureWorkspaceRuntime({
  platform,
}: {
  platform: CaptureWorkspaceRuntimePlatform;
}): CaptureWorkspaceRuntime {
  let state = createInitialState();
  let hydratedSessionId: string | null = null;
  let snapshotHydration: SnapshotHydration | null = null;

  const patch = (next: Partial<RuntimeState>) => {
    state = { ...state, ...next };
  };

  const resetSession = () => {
    state = createInitialState(state.mode);
    hydratedSessionId = null;
    snapshotHydration = null;
  };

  const finishSession = async (sessionId: string) => {
    await platform.dismiss();
    resetSession();
    await platform.commands.cancelCaptureSession(sessionId);
  };

  const executeEffect = async (
    effect: CaptureRuntimeEffect,
    sessionId: string,
    rect: LogicalRect,
  ) => {
    if (effect.type === 'output-capture') {
      if (effect.action !== 'copy') return;

      await platform.commands.outputCapture({
        sessionId,
        rect,
        annotations: [],
        action: { type: 'copy' },
      });
      return;
    }

    if (effect.type === 'finish-session') {
      await finishSession(sessionId);
    }
  };

  const completeSelection = async (rect: LogicalRect) => {
    const session = state.session;
    if (!session || state.isRenderingOutput) return;

    patch({
      selection: rect,
      hoverSelection: null,
      isRenderingOutput: true,
      error: null,
    });

    try {
      const effects = planCandidateSelectionCompletion(
        getPrimaryCaptureCompletionActionForMode(state.mode),
      );
      for (const effect of effects) {
        await executeEffect(effect, session.id, rect);
      }
    } catch (error) {
      patch({ status: 'error', error: errorMessage(error) });
    } finally {
      patch({ isRenderingOutput: false });
    }
  };

  const cancelSession = async () => {
    const sessionId = state.session?.id;
    if (!sessionId) return;

    try {
      await finishSession(sessionId);
    } catch (error) {
      patch({ status: 'error', error: errorMessage(error) });
    }
  };

  const actions: CaptureWorkspaceRuntime['actions'] = {
    async startSession(mode, requestedSessionId) {
      state = {
        ...createInitialState(mode),
        status: 'loading',
      };
      hydratedSessionId = null;
      snapshotHydration = null;

      try {
        const session = requestedSessionId
          ? await platform.commands.getCaptureSession(requestedSessionId)
          : await platform.commands.createCaptureSession();
        const cursorPoint =
          session.captured_cursor?.logical_position ??
          (await platform.commands
            .currentCaptureCursorPosition(session.id)
            .catch(() => null));

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
        patch({ status: 'error', error: errorMessage(error) });
      }
    },

    pointerDown(point) {
      if (state.status !== 'selecting' || !state.session) return;

      patch({
        startPoint: point,
        cursorPoint: point,
        selection: null,
        hoverSelection: null,
      });
    },

    pointerMove(point) {
      if (state.status !== 'selecting' || !state.session) return;

      if (state.startPoint) {
        patch({
          cursorPoint: point,
          selection: normalizeSelection(state.startPoint, point),
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

    async pointerUp(point) {
      if (state.status !== 'selecting' || !state.session || !state.startPoint) {
        return;
      }

      const selection = normalizeSelection(state.startPoint, point);
      const completionRect =
        getCandidateForPointerReleaseCompletion(
          buildCaptureCandidates(
            state.session.monitors,
            state.session.candidates,
          ),
          point,
          state.hoverSelection,
          selection,
          MIN_SELECTION_SIZE,
        )?.rect ??
        (selection.width >= MIN_SELECTION_SIZE &&
        selection.height >= MIN_SELECTION_SIZE
          ? selection
          : null);

      patch({ startPoint: null, cursorPoint: point });
      if (completionRect) {
        await completeSelection(completionRect);
      } else {
        patch({ selection: null, hoverSelection: null });
      }
    },

    async keyDown({ key }) {
      if (key === 'Escape') {
        await cancelSession();
        return;
      }

      if (key === 'Enter' && state.hoverSelection) {
        await completeSelection(state.hoverSelection);
      }
    },

    async hydrateSnapshots() {
      const sessionId = state.session?.id;
      if (!sessionId) return;

      if (snapshotHydration?.sessionId === sessionId) {
        await snapshotHydration.promise;
        return;
      }

      hydratedSessionId = null;
      const promise = platform.commands
        .hydrateCaptureSessionSnapshots(sessionId)
        .then((session) => {
          if (
            snapshotHydration?.sessionId !== sessionId ||
            state.session?.id !== sessionId
          ) {
            return;
          }

          patch({ session });
          hydratedSessionId = sessionId;
        })
        .catch((error) => {
          if (snapshotHydration?.sessionId === sessionId) {
            snapshotHydration = null;
            hydratedSessionId = null;
          }
          throw error;
        });

      snapshotHydration = { sessionId, promise };
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
        selection: state.selection,
        hoverSelection: state.hoverSelection,
        isRenderingOutput: state.isRenderingOutput,
        hasHydratedPixelSource:
          state.session !== null && hydratedSessionId === state.session.id,
        error: state.error,
      };
    },
    actions,
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
    isRenderingOutput: false,
    error: null,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
