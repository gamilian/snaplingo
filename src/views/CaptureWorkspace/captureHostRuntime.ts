import { normalizeOcrText as normalizeCapturedOcrText } from '../../utils/ocrTextProcessing';
import type { CaptureWorkspaceCommandsPort } from '../../application/capture-workspace/ports';
import { buildCaptureCandidates } from './captureCandidates';
import { getInitialHoverSelection } from './captureHoverPolling';
import type { CaptureCompletionAction } from './captureActions';
import {
  planCandidateSelectionCompletion,
  type CaptureRuntimeEffect,
} from './captureInteractionRuntime';
import { shouldRecordSuccessfulCaptureCompletion } from './captureInteractionModel';
import { printBase64PngImage } from './capturePrint';
import {
  shouldRevealCaptureWindow,
  waitForCaptureSurfacePaint,
} from './captureWindowVisibility';
import type { CaptureSelectionOverlayFrame } from './captureSelectionOverlay';
import {
  getSelectionHistoryEntry,
  loadCaptureSelectionHistory,
  loadLastCaptureSelection,
  saveLastCaptureSelection,
  type SelectionHistoryStep,
} from './selectionMemory';
import { restoreSelectionWithinBounds } from './selection';
import { parseCaptureLaunchPayload } from './windowMode';
import type {
  AnnotationCommand,
  CaptureLaunch,
  CaptureMode,
  CaptureSessionView,
  LogicalRect,
  Point,
} from './types';

type CaptureSelectionStorage = Parameters<typeof saveLastCaptureSelection>[0];
type CaptureHotkeyEvent = { payload: unknown };

export interface ExecuteCaptureRuntimeEffectContext {
  sessionId: string;
  rect: LogicalRect;
  includeCursor: boolean;
  screenshotSavePath?: string;
  annotations?: AnnotationCommand[];
  recordSuccessfulSelection: (
    action: CaptureCompletionAction,
    rect: LogicalRect,
  ) => void;
  finishCaptureSession: (sessionId: string) => Promise<void>;
}

export interface CaptureHostRuntimeEffectClient {
  copyCaptureSelection(
    sessionId: string,
    rect: LogicalRect,
    annotations: AnnotationCommand[],
    includeCursor: boolean,
  ): Promise<void>;
  saveCaptureSelection(
    sessionId: string,
    rect: LogicalRect,
    annotations: AnnotationCommand[],
    includeCursor: boolean,
  ): Promise<void>;
  quickSaveCaptureSelection(
    sessionId: string,
    rect: LogicalRect,
    annotations: AnnotationCommand[],
    directory: string | undefined,
    includeCursor: boolean,
  ): Promise<void>;
  printCaptureSelection(
    sessionId: string,
    rect: LogicalRect,
    annotations: AnnotationCommand[],
    printImage: (imageBase64: string) => Promise<void> | void,
    includeCursor: boolean,
  ): Promise<void>;
  outputCapture: CaptureWorkspaceCommandsPort['outputCapture'];
  runCaptureOcr: CaptureWorkspaceCommandsPort['runCaptureOcr'];
  normalizeOcrText: typeof normalizeCapturedOcrText;
  renderCaptureOutput: CaptureWorkspaceCommandsPort['renderCaptureOutput'];
  openCaptureOcrResultWindow: CaptureWorkspaceCommandsPort['openCaptureOcrResultWindow'];
  openCaptureTranslationResultWindow: CaptureWorkspaceCommandsPort['openCaptureTranslationResultWindow'];
  copyTextToClipboard: CaptureWorkspaceCommandsPort['copyTextToClipboard'];
  printImage: typeof printBase64PngImage;
}

const unavailableCaptureCommand = async (..._args: any[]): Promise<never> => {
  throw new Error('Capture workspace runtime command is unavailable');
};

const defaultCaptureHostRuntimeEffectClient: CaptureHostRuntimeEffectClient = {
  copyCaptureSelection: unavailableCaptureCommand,
  saveCaptureSelection: unavailableCaptureCommand,
  quickSaveCaptureSelection: unavailableCaptureCommand,
  printCaptureSelection: unavailableCaptureCommand,
  outputCapture: unavailableCaptureCommand,
  runCaptureOcr: unavailableCaptureCommand,
  normalizeOcrText: normalizeCapturedOcrText,
  renderCaptureOutput: unavailableCaptureCommand,
  openCaptureOcrResultWindow: unavailableCaptureCommand,
  openCaptureTranslationResultWindow: unavailableCaptureCommand,
  copyTextToClipboard: unavailableCaptureCommand,
  printImage: printBase64PngImage,
};

export interface LoadCaptureHostSessionOptions {
  loadSession: () => Promise<CaptureSessionView>;
  getCurrentCursorPosition: (sessionId: string) => Promise<Point | null>;
}

export interface LoadedCaptureHostSession {
  session: CaptureSessionView;
  cursorPoint: Point | null;
  hoverSelection: LogicalRect | null;
}

export async function loadCaptureHostSession({
  getCurrentCursorPosition,
  loadSession,
}: LoadCaptureHostSessionOptions): Promise<LoadedCaptureHostSession> {
  const session = await loadSession();
  const cursorPoint =
    session.captured_cursor?.logical_position ??
    await getCurrentCursorPosition(session.id).catch(() => null);
  const capturedCursor = cursorPoint
    ? {
        logical_position: cursorPoint,
        hotspot: { x: 0, y: 0 },
        image_width: 0,
        image_height: 0,
        scale_factor: 1,
        image_base64: '',
      }
    : null;

  return {
    session,
    cursorPoint,
    hoverSelection: getInitialHoverSelection(
      buildCaptureCandidates(session.monitors, session.candidates),
      capturedCursor,
    ),
  };
}

export interface CaptureHostSessionStartPerfState {
  mode: CaptureMode;
  sessionId: string | null;
  startMs: number;
  hasLoggedImagesReady: boolean;
}

export interface RunCaptureHostSessionStartOptions {
  mode: CaptureMode;
  sessionId?: string;
  now: () => number;
  setCancelling: (value: boolean) => void;
  setRevealed: (value: boolean) => void;
  showLoading: (mode: CaptureMode) => void;
  resetInteractionState: () => void;
  setPerfState: (state: CaptureHostSessionStartPerfState) => void;
  setLoadedPerfSessionId: (sessionId: string) => void;
  markPerf: (event: string, sessionId?: string | null) => void;
  loadSession: () => Promise<LoadedCaptureHostSession>;
  applyLoadedSession: (loadedSession: LoadedCaptureHostSession) => void;
  onError: (err: unknown) => void;
}

export async function runCaptureHostSessionStart({
  applyLoadedSession,
  loadSession,
  markPerf,
  mode,
  now,
  onError,
  resetInteractionState,
  sessionId,
  setCancelling,
  setLoadedPerfSessionId,
  setPerfState,
  setRevealed,
  showLoading,
}: RunCaptureHostSessionStartOptions) {
  setCancelling(false);
  setRevealed(false);
  showLoading(mode);
  resetInteractionState();
  setPerfState({
    mode,
    sessionId: sessionId ?? null,
    startMs: now(),
    hasLoggedImagesReady: false,
  });
  markPerf('start_session', sessionId);

  try {
    const loadedSession = await loadSession();
    setLoadedPerfSessionId(loadedSession.session.id);
    markPerf('session_loaded', loadedSession.session.id);
    applyLoadedSession(loadedSession);
    return loadedSession;
  } catch (err) {
    onError(err);
    return null;
  }
}

export interface RunCaptureHostSessionRefreshOptions {
  sessionId?: string | null;
  setRevealed: (value: boolean) => void;
  showLoading: () => void;
  resetInteractionState: () => void;
  loadSession: (sessionId: string) => Promise<LoadedCaptureHostSession>;
  applyLoadedSession: (loadedSession: LoadedCaptureHostSession) => void;
  onError: (err: unknown) => void;
}

export async function runCaptureHostSessionRefresh({
  applyLoadedSession,
  loadSession,
  onError,
  resetInteractionState,
  sessionId,
  setRevealed,
  showLoading,
}: RunCaptureHostSessionRefreshOptions) {
  if (!sessionId) return null;

  setRevealed(false);
  showLoading();
  resetInteractionState();

  try {
    const loadedSession = await loadSession(sessionId);
    applyLoadedSession(loadedSession);
    return loadedSession;
  } catch (err) {
    onError(err);
    return null;
  }
}

export interface CaptureHostSnapshotHydration {
  sessionId: string;
  promise: Promise<CaptureSessionView>;
}

export interface EnsureCaptureHostSnapshotsHydratedOptions {
  sessionId: string;
  getCurrentHydration: () => CaptureHostSnapshotHydration | null;
  setCurrentHydration: (
    hydration: CaptureHostSnapshotHydration | null,
  ) => void;
  hydrateSnapshots: (sessionId: string) => Promise<CaptureSessionView>;
  clearHydratedSession: () => void;
  applyHydratedSession: (
    sessionId: string,
    session: CaptureSessionView,
  ) => void;
  markHydratedSession: (sessionId: string) => void;
  markSnapshotsHydrated?: (sessionId: string) => void;
}

export function ensureCaptureHostSnapshotsHydrated({
  applyHydratedSession,
  clearHydratedSession,
  getCurrentHydration,
  hydrateSnapshots,
  markHydratedSession,
  markSnapshotsHydrated,
  sessionId,
  setCurrentHydration,
}: EnsureCaptureHostSnapshotsHydratedOptions) {
  const currentHydration = getCurrentHydration();
  if (currentHydration?.sessionId === sessionId) {
    return currentHydration.promise;
  }

  clearHydratedSession();
  const hydrationPromise = hydrateSnapshots(sessionId)
    .then((hydratedSession) => {
      if (getCurrentHydration()?.sessionId !== sessionId) {
        return hydratedSession;
      }

      applyHydratedSession(sessionId, hydratedSession);
      markHydratedSession(sessionId);
      markSnapshotsHydrated?.(sessionId);
      return hydratedSession;
    })
    .catch((err) => {
      if (getCurrentHydration()?.sessionId === sessionId) {
        setCurrentHydration(null);
        clearHydratedSession();
      }
      throw err;
    });

  setCurrentHydration({
    sessionId,
    promise: hydrationPromise,
  });

  return hydrationPromise;
}

export async function executeCaptureRuntimeEffect(
  effect: CaptureRuntimeEffect,
  context: ExecuteCaptureRuntimeEffectContext,
  clientOverrides: Partial<CaptureHostRuntimeEffectClient> = {},
) {
  const client = {
    ...defaultCaptureHostRuntimeEffectClient,
    ...clientOverrides,
  };
  const {
    annotations = [],
    finishCaptureSession,
    includeCursor,
    recordSuccessfulSelection,
    rect,
    screenshotSavePath,
    sessionId,
  } = context;

  if (effect.type === 'output-capture') {
    if (effect.action === 'copy') {
      await client.copyCaptureSelection(
        sessionId,
        rect,
        annotations,
        includeCursor,
      );
    } else if (effect.action === 'save') {
      await client.saveCaptureSelection(
        sessionId,
        rect,
        annotations,
        includeCursor,
      );
    } else if (effect.action === 'quick-save') {
      await client.quickSaveCaptureSelection(
        sessionId,
        rect,
        annotations,
        screenshotSavePath,
        includeCursor,
      );
    } else if (effect.action === 'print') {
      await client.printCaptureSelection(
        sessionId,
        rect,
        annotations,
        client.printImage,
        includeCursor,
      );
    } else if (effect.action === 'pin') {
      await client.outputCapture({
        sessionId,
        rect,
        annotations,
        ...(includeCursor ? { includeCursor: true } : {}),
        action: { type: 'pin' },
      });
    }
    return;
  }

  if (effect.type === 'run-ocr') {
    const ocrResult = await client.runCaptureOcr(sessionId, rect);
    const normalizedOcrText = client.normalizeOcrText(ocrResult.text);

    if (effect.target === 'translation-window') {
      await client.openCaptureTranslationResultWindow(normalizedOcrText);
      return;
    }

    if (effect.target === 'ocr-window') {
      const imageBase64 = await client.renderCaptureOutput({
        sessionId,
        rect,
        annotations,
      });
      await client.openCaptureOcrResultWindow(normalizedOcrText, imageBase64);
      return;
    }

    await client.copyTextToClipboard(normalizedOcrText);
    return;
  }

  if (effect.type === 'record-selection') {
    recordSuccessfulSelection(effect.action, rect);
    return;
  }

  await finishCaptureSession(sessionId);
}

export interface RunCaptureHostCompletionFlowOptions {
  guardCompletion: boolean;
  isCompleting: () => boolean;
  setCompleting: (value: boolean) => void;
  setRendering: (value: boolean) => void;
  clearError: () => void;
  runCompletion: () => Promise<void>;
  onError: (err: unknown) => void;
}

export async function runCaptureHostCompletionFlow({
  clearError,
  guardCompletion,
  isCompleting,
  onError,
  runCompletion,
  setCompleting,
  setRendering,
}: RunCaptureHostCompletionFlowOptions) {
  if (guardCompletion && isCompleting()) return false;

  if (guardCompletion) {
    setCompleting(true);
  }
  setRendering(true);
  clearError();

  try {
    await runCompletion();
    return true;
  } catch (err) {
    onError(err);
    return false;
  } finally {
    if (guardCompletion) {
      setCompleting(false);
    }
    setRendering(false);
  }
}

export interface RunCaptureHostPreviewRenderOptions {
  sessionId: string;
  rect: LogicalRect;
  annotations: AnnotationCommand[];
  includeCursor: boolean;
  setRendering: (value: boolean) => void;
  clearPreview: () => void;
  clearError: () => void;
  renderPreview?: CaptureWorkspaceCommandsPort['renderCaptureOutput'];
  setPreviewImage: (base64: string) => void;
  onError: (err: unknown) => void;
}

export async function runCaptureHostPreviewRender({
  annotations,
  clearError,
  clearPreview,
  includeCursor,
  onError,
  rect,
  renderPreview = unavailableCaptureCommand,
  sessionId,
  setPreviewImage,
  setRendering,
}: RunCaptureHostPreviewRenderOptions) {
  setRendering(true);
  clearPreview();
  clearError();

  try {
    const base64 = await renderPreview({
      sessionId,
      rect,
      annotations,
      ...(includeCursor ? { includeCursor: true } : {}),
    });
    setPreviewImage(base64);
    return true;
  } catch (err) {
    onError(err);
    return false;
  } finally {
    setRendering(false);
  }
}

export interface RunCaptureHostTransitionEffectsOptions {
  rendering: boolean;
  error: string | null;
  setRendering: (value: boolean) => void;
  setError: (value: string | null) => void;
  runEffects: () => Promise<void>;
  onError: (err: unknown) => void;
}

export async function runCaptureHostTransitionEffects({
  error,
  onError,
  rendering,
  runEffects,
  setError,
  setRendering,
}: RunCaptureHostTransitionEffectsOptions) {
  setRendering(rendering);
  setError(error);

  try {
    await runEffects();
    return true;
  } catch (err) {
    onError(err);
    return false;
  } finally {
    setRendering(false);
  }
}

export async function runCaptureRuntimeEffects(
  effects: CaptureRuntimeEffect[],
  context: ExecuteCaptureRuntimeEffectContext,
  clientOverrides: Partial<CaptureHostRuntimeEffectClient> = {},
) {
  for (const effect of effects) {
    await executeCaptureRuntimeEffect(effect, context, clientOverrides);
  }
}

export async function runCaptureCompletionAction(
  action: CaptureCompletionAction,
  context: ExecuteCaptureRuntimeEffectContext,
  clientOverrides: Partial<CaptureHostRuntimeEffectClient> = {},
) {
  await runCaptureRuntimeEffects(
    planCandidateSelectionCompletion(action),
    context,
    clientOverrides,
  );
}

export type CaptureHotkeyLaunchListener = (
  launch: CaptureLaunch,
) => void | Promise<void>;

export type CaptureHotkeyEventListener = (
  handler: (event: CaptureHotkeyEvent) => void,
) => Promise<() => void>;

export async function subscribeCaptureHotkeyLaunches(
  onLaunch: CaptureHotkeyLaunchListener,
  listenForHotkey: CaptureHotkeyEventListener,
  parseLaunch = parseCaptureLaunchPayload,
) {
  return listenForHotkey((event) => {
    const launch = parseLaunch(event.payload);
    if (!launch) return;
    void onLaunch(launch);
  });
}

export type CaptureHostRequestSubscriber = (
  handler: () => void | Promise<void>,
) => Promise<() => void>;

export function subscribeCaptureCancelHostRequests(
  onCancel: () => void | Promise<void>,
  subscribe: CaptureHostRequestSubscriber,
) {
  return subscribe(onCancel);
}

export function subscribeCaptureCopyHostRequests(
  onCopy: () => void | Promise<void>,
  subscribe: CaptureHostRequestSubscriber,
) {
  return subscribe(onCopy);
}

export interface RevealCaptureHostWindowOptions {
  status: 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
  sessionId?: string | null;
  hasCaptureImagesReady: boolean;
  hasRevealed: boolean;
  prepareSurface?: () => void | Promise<void>;
}

export interface CaptureHostWindowRevealRuntime {
  shouldRevealCaptureWindow: typeof shouldRevealCaptureWindow;
  revealCaptureWindow(): Promise<void>;
  revealCaptureWindowForSession(options: {
    sessionId: string;
    prepareSurface?: () => void | Promise<void>;
  }): Promise<void>;
}

export async function revealCaptureHostWindow(
  options: RevealCaptureHostWindowOptions,
  runtime: CaptureHostWindowRevealRuntime,
) {
  if (
    !runtime.shouldRevealCaptureWindow({
      status: options.status,
      hasSession: Boolean(options.sessionId),
      hasCaptureImagesReady: options.hasCaptureImagesReady,
      hasRevealed: options.hasRevealed,
    })
  ) {
    return false;
  }

  if (!options.sessionId) {
    await runtime.revealCaptureWindow();
    return true;
  }

  await runtime.revealCaptureWindowForSession({
    sessionId: options.sessionId,
    prepareSurface: options.prepareSurface,
  });
  return true;
}

export function recordSuccessfulCaptureSelection(
  storage: CaptureSelectionStorage,
  action: CaptureCompletionAction,
  rect: LogicalRect,
) {
  if (!shouldRecordSuccessfulCaptureCompletion(action)) return;
  saveLastCaptureSelection(storage, rect);
}

export interface RestoreLastCaptureSelectionOptions {
  storage: CaptureSelectionStorage;
  selectionBounds: LogicalRect;
  minSelectionSize: number;
  completeSelection: (rect: LogicalRect) => void;
}

export function restoreLastSuccessfulCaptureSelection({
  completeSelection,
  minSelectionSize,
  selectionBounds,
  storage,
}: RestoreLastCaptureSelectionOptions) {
  const savedSelection = loadLastCaptureSelection(storage);
  if (!savedSelection) return;

  const restoredSelection = restoreSelectionWithinBounds(
    savedSelection,
    selectionBounds,
    minSelectionSize,
  );
  if (!restoredSelection) return;

  completeSelection(restoredSelection);
}

export interface PrepareCaptureSurfaceForRevealOptions {
  frame: CaptureSelectionOverlayFrame | null;
  paintSelectionOverlayFrame: (
    frame: CaptureSelectionOverlayFrame | null,
  ) => void;
  waitForPaint?: typeof waitForCaptureSurfacePaint;
}

export async function prepareCaptureSurfaceForReveal({
  frame,
  paintSelectionOverlayFrame,
  waitForPaint = waitForCaptureSurfacePaint,
}: PrepareCaptureSurfaceForRevealOptions) {
  paintSelectionOverlayFrame(frame);
  await waitForPaint();
}

export interface RestoreCaptureSelectionFromHistoryOptions {
  storage: CaptureSelectionStorage;
  currentSelection: LogicalRect | null;
  step: SelectionHistoryStep | null;
  selectionBounds: LogicalRect;
  minSelectionSize: number;
  completeSelection: (rect: LogicalRect) => void;
}

export function restoreCaptureSelectionFromHistory({
  completeSelection,
  currentSelection,
  minSelectionSize,
  selectionBounds,
  step,
  storage,
}: RestoreCaptureSelectionFromHistoryOptions) {
  if (!step) return;

  const historySelection = getSelectionHistoryEntry(
    loadCaptureSelectionHistory(storage),
    currentSelection,
    step,
  );
  if (!historySelection) return;

  const restoredSelection = restoreSelectionWithinBounds(
    historySelection,
    selectionBounds,
    minSelectionSize,
  );
  if (!restoredSelection) return;

  completeSelection(restoredSelection);
}
