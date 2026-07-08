import type { AnnotationHistory } from './annotationHistory';
import type {
  CaptureCompletionAction,
  HoverSelectionCompletionAction,
  PreviewCaptureCompletionAction,
} from './captureActions';
import {
  ensureCaptureHostSnapshotsHydrated,
  loadCaptureHostSession,
  recordSuccessfulCaptureSelection,
  runCaptureCompletionAction as runHostCaptureCompletionAction,
  runCaptureHostCompletionFlow,
  runCaptureHostPreviewRender,
  runCaptureHostSessionRefresh,
  runCaptureHostSessionStart,
  runCaptureRuntimeEffects as runHostCaptureRuntimeEffects,
  type CaptureHostRuntimeEffectClient,
  type CaptureHostSessionStartPerfState,
  type CaptureHostSnapshotHydration,
  type LoadedCaptureHostSession,
  type RunCaptureHostPreviewRenderOptions,
} from './captureHostRuntime';
import type { CaptureRuntimeEffect } from './captureInteractionRuntime';
import {
  cancelCaptureSessionFlow,
  closeInactiveCaptureSession,
  finishCaptureSession,
} from './captureSessionLifecycle';
import {
  loadedCaptureHostSessionPatch,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import type {
  AnnotationCommand,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from './types';

export interface CaptureWorkspaceHostAdapter {
  getState(): CaptureWorkspaceState;
  patch(next: Partial<CaptureWorkspaceState>): void;
  resetInteraction(): void;
  resetSession(): void;
}

export interface CaptureWorkspaceSelectionStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export interface CaptureWorkspaceHostClients {
  createCaptureSession(): Promise<CaptureSessionView>;
  getCaptureSession(sessionId: string): Promise<CaptureSessionView>;
  refreshCaptureSession(sessionId: string): Promise<CaptureSessionView>;
  currentCaptureCursorPosition(sessionId: string): Promise<Point | null>;
  hydrateCaptureSessionSnapshots(sessionId: string): Promise<CaptureSessionView>;
  cancelCaptureSession(sessionId: string): Promise<void>;
  renderPreview?: RunCaptureHostPreviewRenderOptions['renderPreview'];
  runtimeEffectClient?: Partial<CaptureHostRuntimeEffectClient>;
}

export interface CaptureWorkspaceHostDeps {
  workspace: CaptureWorkspaceHostAdapter;
  getScreenshotSavePath(): string | undefined;
  getOnInactive(): (() => void | Promise<void>) | undefined;
  getSelection(): LogicalRect | null;
  getAnnotations(): AnnotationCommand[];
  getShouldIncludeCapturedCursor(): boolean;
  commitTextDraftToHistory(): AnnotationHistory;
  isCompletingCapture(): boolean;
  setCompletingCapture(value: boolean): void;
  isCancellingSession(): boolean;
  setCancellingSession(value: boolean): void;
  setRevealed(value: boolean): void;
  now(): number;
  getPerfState(): CaptureHostSessionStartPerfState | null;
  setPerfState(state: CaptureHostSessionStartPerfState | null): void;
  markPerf(event: string, sessionId?: string | null): void;
  storage: CaptureWorkspaceSelectionStorage;
  logWarning?(message: string, err: unknown): void;
  getSnapshotHydration(): CaptureHostSnapshotHydration | null;
  setSnapshotHydration(hydration: CaptureHostSnapshotHydration | null): void;
  clearHydratedSession(): void;
  markHydratedSession(sessionId: string): void;
  resetSelectionOverlay(): void;
  resetCaptureImageReadiness(): void;
  clients: CaptureWorkspaceHostClients;
}

export interface CompletePreviewSelectionOptions {
  commitTextDraft?: boolean;
  guardCompletion?: boolean;
}

export interface CaptureWorkspaceHostActions {
  ensureCaptureSnapshotsHydrated(
    sessionId: string,
  ): Promise<CaptureSessionView>;
  finishCurrentCaptureSession(sessionId: string): Promise<void>;
  cancelSession(): Promise<boolean>;
  startSession(
    nextMode: CaptureMode,
    sessionId?: string,
  ): Promise<LoadedCaptureHostSession | null>;
  recordSuccessfulSelection(
    action: CaptureCompletionAction,
    rect: LogicalRect,
  ): void;
  runCaptureRuntimeEffects(
    effects: CaptureRuntimeEffect[],
    rect: LogicalRect,
    nextAnnotations?: AnnotationCommand[],
  ): Promise<void>;
  runCaptureCompletionAction(
    action: CaptureCompletionAction,
    rect: LogicalRect,
    nextAnnotations?: AnnotationCommand[],
  ): Promise<void>;
  renderSelectionPreview(
    rect: LogicalRect,
    nextAnnotations?: AnnotationCommand[],
    includeCursor?: boolean,
  ): Promise<boolean | undefined>;
  completePreviewSelection(
    action: PreviewCaptureCompletionAction,
    options?: CompletePreviewSelectionOptions,
  ): Promise<boolean | undefined>;
  completeCandidateSelection(
    rect: LogicalRect,
    action: HoverSelectionCompletionAction,
  ): Promise<boolean | undefined>;
  refreshSession(): Promise<LoadedCaptureHostSession | null>;
}

export function createCaptureWorkspaceHostActions(
  deps: CaptureWorkspaceHostDeps,
): CaptureWorkspaceHostActions {
  const { workspace } = deps;

  const setHostError = (err: unknown) => {
    workspace.patch({
      status: 'error',
      error: errorMessage(err),
    });
  };

  const resetInteractionState = () => {
    workspace.resetInteraction();
    deps.resetSelectionOverlay();
    deps.setCompletingCapture(false);
    deps.resetCaptureImageReadiness();
  };

  const resetSessionState = () => {
    workspace.resetSession();
    deps.resetSelectionOverlay();
    deps.setCompletingCapture(false);
    deps.resetCaptureImageReadiness();
  };

  const applyLoadedSession = (loaded: LoadedCaptureHostSession) => {
    workspace.patch(loadedCaptureHostSessionPatch(loaded));
  };

  const setLoadedPerfSessionId = (sessionId: string) => {
    const perfState = deps.getPerfState();
    if (!perfState) return;

    deps.setPerfState({
      ...perfState,
      sessionId,
    });
  };

  const finishCurrentCaptureSession = async (sessionId: string) => {
    await finishCaptureSession({
      client: {
        cancelCaptureSession: deps.clients.cancelCaptureSession,
      },
      sessionId,
      onInactive: deps.getOnInactive(),
      resetSessionState,
    });
  };

  const recordSuccessfulSelection = (
    action: CaptureCompletionAction,
    rect: LogicalRect,
  ) => {
    try {
      recordSuccessfulCaptureSelection(deps.storage, action, rect);
    } catch (err) {
      deps.logWarning?.('Failed to remember capture selection:', err);
    }
  };

  const createRuntimeContext = (
    rect: LogicalRect,
    annotations: AnnotationCommand[],
  ) => {
    const session = workspace.getState().session;
    if (!session) return null;

    return {
      sessionId: session.id,
      rect,
      annotations,
      includeCursor: deps.getShouldIncludeCapturedCursor(),
      screenshotSavePath: deps.getScreenshotSavePath(),
      recordSuccessfulSelection,
      finishCaptureSession: finishCurrentCaptureSession,
    };
  };

  const runCaptureRuntimeEffects = async (
    effects: CaptureRuntimeEffect[],
    rect: LogicalRect,
    nextAnnotations: AnnotationCommand[] = [],
  ) => {
    const context = createRuntimeContext(rect, nextAnnotations);
    if (!context) return;

    await runHostCaptureRuntimeEffects(
      effects,
      context,
      deps.clients.runtimeEffectClient,
    );
  };

  const runCaptureCompletionAction = async (
    action: CaptureCompletionAction,
    rect: LogicalRect,
    nextAnnotations: AnnotationCommand[] = [],
  ) => {
    const context = createRuntimeContext(rect, nextAnnotations);
    if (!context) return;

    await runHostCaptureCompletionAction(
      action,
      context,
      deps.clients.runtimeEffectClient,
    );
  };

  const renderSelectionPreview = async (
    rect: LogicalRect,
    nextAnnotations = deps.getAnnotations(),
    includeCursor = deps.getShouldIncludeCapturedCursor(),
  ) => {
    const session = workspace.getState().session;
    if (!session) return undefined;

    return runCaptureHostPreviewRender({
      sessionId: session.id,
      rect,
      annotations: nextAnnotations,
      includeCursor,
      setRendering: (isRenderingOutput) => {
        workspace.patch({ isRenderingOutput });
      },
      clearPreview: () => {
        workspace.patch({ previewImageBase64: null });
      },
      clearError: () => {
        workspace.patch({ error: null });
      },
      renderPreview: deps.clients.renderPreview,
      setPreviewImage: (previewImageBase64) => {
        workspace.patch({ previewImageBase64 });
      },
      onError: setHostError,
    });
  };

  const completePreviewSelection = async (
    action: PreviewCaptureCompletionAction,
    {
      commitTextDraft = true,
      guardCompletion = false,
    }: CompletePreviewSelectionOptions = {},
  ) => {
    if (!workspace.getState().session) return undefined;

    const selection = deps.getSelection();
    if (!selection) return undefined;

    return runCaptureHostCompletionFlow({
      guardCompletion,
      isCompleting: deps.isCompletingCapture,
      setCompleting: deps.setCompletingCapture,
      setRendering: (isRenderingOutput) => {
        workspace.patch({ isRenderingOutput });
      },
      clearError: () => {
        workspace.patch({ error: null });
      },
      runCompletion: async () => {
        if (commitTextDraft) {
          const outputHistory = deps.commitTextDraftToHistory();
          await runCaptureCompletionAction(
            action,
            selection,
            outputHistory.annotations,
          );
          return;
        }

        await runCaptureCompletionAction(action, selection);
      },
      onError: setHostError,
    });
  };

  const completeCandidateSelection = async (
    rect: LogicalRect,
    action: HoverSelectionCompletionAction,
  ) => {
    if (!workspace.getState().session) return undefined;

    return runCaptureHostCompletionFlow({
      guardCompletion: true,
      isCompleting: deps.isCompletingCapture,
      setCompleting: deps.setCompletingCapture,
      setRendering: (isRenderingOutput) => {
        workspace.patch({ isRenderingOutput });
      },
      clearError: () => {
        workspace.patch({ error: null });
      },
      runCompletion: () => runCaptureCompletionAction(action, rect),
      onError: setHostError,
    });
  };

  return {
    ensureCaptureSnapshotsHydrated(sessionId) {
      return ensureCaptureHostSnapshotsHydrated({
        sessionId,
        getCurrentHydration: deps.getSnapshotHydration,
        setCurrentHydration: deps.setSnapshotHydration,
        hydrateSnapshots: deps.clients.hydrateCaptureSessionSnapshots,
        clearHydratedSession: deps.clearHydratedSession,
        applyHydratedSession: (hydratedSessionId, hydratedSession) => {
          if (workspace.getState().session?.id !== hydratedSessionId) return;

          workspace.patch({ session: hydratedSession });
        },
        markHydratedSession: deps.markHydratedSession,
        markSnapshotsHydrated: (hydratedSessionId) => {
          deps.markPerf('snapshots_hydrated', hydratedSessionId);
        },
      });
    },
    finishCurrentCaptureSession,
    cancelSession() {
      return cancelCaptureSessionFlow({
        sessionId: workspace.getState().session?.id,
        isCancelling: deps.isCancellingSession,
        setCancelling: deps.setCancellingSession,
        finishSession: finishCurrentCaptureSession,
        closeInactiveSession: () =>
          closeInactiveCaptureSession({
            onInactive: deps.getOnInactive(),
            resetSessionState,
          }),
        onError: setHostError,
      });
    },
    startSession(nextMode, sessionId) {
      return runCaptureHostSessionStart({
        mode: nextMode,
        sessionId,
        now: deps.now,
        setCancelling: deps.setCancellingSession,
        setRevealed: deps.setRevealed,
        showLoading: (mode) => {
          workspace.patch({
            status: 'loading',
            mode,
          });
        },
        resetInteractionState,
        setPerfState: deps.setPerfState,
        setLoadedPerfSessionId,
        markPerf: deps.markPerf,
        loadSession: () =>
          loadCaptureHostSession({
            loadSession: () =>
              sessionId
                ? deps.clients.getCaptureSession(sessionId)
                : deps.clients.createCaptureSession(),
            getCurrentCursorPosition: deps.clients.currentCaptureCursorPosition,
          }),
        applyLoadedSession,
        onError: setHostError,
      });
    },
    recordSuccessfulSelection,
    runCaptureRuntimeEffects,
    runCaptureCompletionAction,
    renderSelectionPreview,
    completePreviewSelection,
    completeCandidateSelection,
    refreshSession() {
      return runCaptureHostSessionRefresh({
        sessionId: workspace.getState().session?.id,
        setRevealed: deps.setRevealed,
        showLoading: () => {
          workspace.patch({ status: 'loading' });
        },
        resetInteractionState,
        loadSession: (sessionId) =>
          loadCaptureHostSession({
            loadSession: () => deps.clients.refreshCaptureSession(sessionId),
            getCurrentCursorPosition: deps.clients.currentCaptureCursorPosition,
          }),
        applyLoadedSession,
        onError: setHostError,
      });
    },
  };
}

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}
