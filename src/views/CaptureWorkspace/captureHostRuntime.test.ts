import { describe, expect, it } from 'vitest';
import { loadLastCaptureSelection, saveLastCaptureSelection } from './selectionMemory';
import {
  executeCaptureRuntimeEffect,
  runCaptureCompletionAction,
  runCaptureRuntimeEffects,
  ensureCaptureHostSnapshotsHydrated,
  loadCaptureHostSession,
  recordSuccessfulCaptureSelection,
  runCaptureHostCompletionFlow,
  runCaptureHostPreviewRender,
  runCaptureHostSessionRefresh,
  runCaptureHostSessionStart,
  runCaptureHostTransitionEffects,
  restoreCaptureSelectionFromHistory,
  revealCaptureHostWindow,
  subscribeCaptureCancelHostRequests,
  subscribeCaptureCopyHostRequests,
  subscribeCaptureHotkeyLaunches,
  type CaptureHostSnapshotHydration,
} from './captureHostRuntime';
import type { CaptureSessionView, LogicalRect } from './types';

function createMemoryStorage() {
  const values = new Map<string, string>();

  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
}

function createCaptureSessionView(
  overrides: Partial<CaptureSessionView> = {},
): CaptureSessionView {
  return {
    id: 'session-1',
    monitors: [
      {
        id: 'monitor-1',
        logical_bounds: { x: 0, y: 0, width: 500, height: 300 },
        physical_bounds: { x: 0, y: 0, width: 1000, height: 600 },
        scale_factor: 2,
        image_base64: '',
      },
    ],
    candidates: [
      {
        id: 'window-1',
        kind: 'window',
        rect: { x: 100, y: 100, width: 200, height: 120 },
        priority: 10,
      },
    ],
    captured_cursor: null,
    ...overrides,
  };
}

describe('captureHostRuntime', () => {
  it('executes translation-window OCR effects through provided adapters', async () => {
    const calls: string[] = [];

    await executeCaptureRuntimeEffect(
      { type: 'run-ocr', target: 'translation-window' },
      {
        sessionId: 'session-1',
        rect: { x: 10, y: 20, width: 120, height: 80 },
        includeCursor: false,
        screenshotSavePath: '/captures',
        recordSuccessfulSelection: () => {
          calls.push('record_selection');
        },
        finishCaptureSession: async () => {
          calls.push('finish_session');
        },
      },
      {
        copyCaptureSelection: async () => {
          calls.push('copy_capture');
        },
        saveCaptureSelection: async () => {
          calls.push('save_capture');
        },
        quickSaveCaptureSelection: async () => {
          calls.push('quick_save_capture');
        },
        printCaptureSelection: async () => {
          calls.push('print_capture');
        },
        outputCapture: async () => {
          calls.push('pin_capture');
        },
        runCaptureOcr: async (sessionId) => {
          calls.push(`run_ocr:${sessionId}`);
          return { text: '  Hello world  ', confidence: 1 };
        },
        normalizeOcrText: (text) => {
          calls.push(`normalize:${text}`);
          return text.trim().toUpperCase();
        },
        renderCaptureOutput: async () => {
          calls.push('render_capture_output');
          return 'image-base64';
        },
        openCaptureOcrResultWindow: async () => {
          calls.push('open_capture_ocr_window');
        },
        openCaptureTranslationResultWindow: async (text) => {
          calls.push(`open_capture_translation_window:${text}`);
        },
        copyTextToClipboard: async (text) => {
          calls.push(`copy_text:${text}`);
        },
      },
    );

    expect(calls).toEqual([
      'run_ocr:session-1',
      'normalize:  Hello world  ',
      'open_capture_translation_window:HELLO WORLD',
    ]);
  });

  it('subscribes capture hotkey launches and ignores invalid payloads', async () => {
    const calls: string[] = [];
    let handler:
      | ((event: { payload: unknown }) => void)
      | undefined;

    const unlisten = await subscribeCaptureHotkeyLaunches(
      (launch) => {
        calls.push(`${launch.mode}:${launch.sessionId ?? ''}`);
      },
      async (nextHandler) => {
        calls.push('listen');
        handler = nextHandler;
        return () => {
          calls.push('unlisten');
        };
      },
      (payload) => {
        if (payload === 'valid') {
          return { mode: 'screenshot-copy', sessionId: 'session-2' };
        }
        return null;
      },
    );

    handler?.({ payload: 'ignored' });
    handler?.({ payload: 'valid' });
    unlisten();

    expect(calls).toEqual([
      'listen',
      'screenshot-copy:session-2',
      'unlisten',
    ]);
  });

  it('subscribes native cancel and copy requests through the provided host adapters', async () => {
    const calls: string[] = [];

    const cancelUnlisten = await subscribeCaptureCancelHostRequests(
      () => {
        calls.push('cancel');
      },
      async (onCancel) => {
        calls.push('subscribe_cancel');
        await onCancel();
        return () => {
          calls.push('cancel_unlisten');
        };
      },
    );
    const copyUnlisten = await subscribeCaptureCopyHostRequests(
      () => {
        calls.push('copy');
      },
      async (onCopy) => {
        calls.push('subscribe_copy');
        await onCopy();
        return () => {
          calls.push('copy_unlisten');
        };
      },
    );

    cancelUnlisten();
    copyUnlisten();

    expect(calls).toEqual([
      'subscribe_cancel',
      'cancel',
      'subscribe_copy',
      'copy',
      'cancel_unlisten',
      'copy_unlisten',
    ]);
  });

  it('reveals a capture session through the session-aware reveal path once the host is ready', async () => {
    const calls: string[] = [];

    const didReveal = await revealCaptureHostWindow(
      {
        status: 'selecting',
        sessionId: 'session-3',
        hasCaptureImagesReady: true,
        hasRevealed: false,
        window: {
          show: async () => {
            calls.push('show');
          },
          setFocus: async () => {
            calls.push('set_focus');
          },
        },
        prepareSurface: async () => {
          calls.push('prepare_surface');
        },
      },
      {
        shouldRevealCaptureWindow: () => true,
        revealCaptureWindow: async () => {
          calls.push('reveal_plain_window');
        },
        revealCaptureWindowForSession: async ({ sessionId, prepareSurface }) => {
          calls.push(`reveal_session:${sessionId}`);
          await prepareSurface?.();
        },
      },
    );

    expect(didReveal).toBe(true);
    expect(calls).toEqual([
      'reveal_session:session-3',
      'prepare_surface',
    ]);
  });

  it('records and restores successful capture selections through storage-backed history', () => {
    const storage = createMemoryStorage();
    const first: LogicalRect = { x: 10, y: 20, width: 120, height: 80 };
    const second: LogicalRect = { x: 30, y: 40, width: 90, height: 70 };
    const restored: LogicalRect[] = [];

    recordSuccessfulCaptureSelection(storage, 'copy', first);
    recordSuccessfulCaptureSelection(storage, 'copy', second);

    expect(loadLastCaptureSelection(storage)).toEqual(second);

    restoreCaptureSelectionFromHistory({
      storage,
      currentSelection: second,
      step: 'previous',
      selectionBounds: { x: 0, y: 0, width: 500, height: 300 },
      minSelectionSize: 10,
      completeSelection: (rect) => {
        restored.push(rect);
      },
    });

    expect(restored).toEqual([first]);
  });

  it('does not record untracked selection actions', () => {
    const storage = createMemoryStorage();
    const rect: LogicalRect = { x: 10, y: 20, width: 120, height: 80 };

    recordSuccessfulCaptureSelection(storage, 'cancel', rect);
    saveLastCaptureSelection(storage, rect);

    expect(loadLastCaptureSelection(storage)).toEqual(rect);
  });

  it('runs copy completion through output, record, and finish helpers in order', async () => {
    const calls: string[] = [];

    await runCaptureCompletionAction(
      'copy',
      {
        sessionId: 'session-4',
        rect: { x: 10, y: 20, width: 120, height: 80 },
        annotations: [{ type: 'rectangle', rect: { x: 1, y: 1, width: 10, height: 10 }, color: [255, 0, 0, 255], stroke_width: 2, filled: false }],
        includeCursor: true,
        screenshotSavePath: '/captures',
        recordSuccessfulSelection: (action) => {
          calls.push(`record:${action}`);
        },
        finishCaptureSession: async (sessionId) => {
          calls.push(`finish:${sessionId}`);
        },
      },
      {
        copyCaptureSelection: async (sessionId, rect, annotations, includeCursor) => {
          calls.push(
            `copy:${sessionId}:${rect.width}:${annotations?.length ?? 0}:${includeCursor}`,
          );
        },
      },
    );

    expect(calls).toEqual([
      'copy:session-4:120:1:true',
      'record:copy',
      'finish:session-4',
    ]);
  });

  it('skips a guarded capture host completion while another completion is running', async () => {
    const calls: string[] = [];

    const didRun = await runCaptureHostCompletionFlow({
      guardCompletion: true,
      isCompleting: () => true,
      setCompleting: (value) => {
        calls.push(`set_completing:${value}`);
      },
      setRendering: (value) => {
        calls.push(`set_rendering:${value}`);
      },
      clearError: () => {
        calls.push('clear_error');
      },
      runCompletion: async () => {
        calls.push('run_completion');
      },
      onError: (err) => {
        calls.push(`error:${String(err)}`);
      },
    });

    expect(didRun).toBe(false);
    expect(calls).toEqual([]);
  });

  it('runs capture host completion with guarded rendering and cleanup', async () => {
    const calls: string[] = [];

    const didRun = await runCaptureHostCompletionFlow({
      guardCompletion: true,
      isCompleting: () => false,
      setCompleting: (value) => {
        calls.push(`set_completing:${value}`);
      },
      setRendering: (value) => {
        calls.push(`set_rendering:${value}`);
      },
      clearError: () => {
        calls.push('clear_error');
      },
      runCompletion: async () => {
        calls.push('run_completion');
      },
      onError: (err) => {
        calls.push(`error:${String(err)}`);
      },
    });

    expect(didRun).toBe(true);
    expect(calls).toEqual([
      'set_completing:true',
      'set_rendering:true',
      'clear_error',
      'run_completion',
      'set_completing:false',
      'set_rendering:false',
    ]);
  });

  it('runs unguarded capture host completion even when another guarded completion is active', async () => {
    const calls: string[] = [];

    const didRun = await runCaptureHostCompletionFlow({
      guardCompletion: false,
      isCompleting: () => true,
      setCompleting: (value) => {
        calls.push(`set_completing:${value}`);
      },
      setRendering: (value) => {
        calls.push(`set_rendering:${value}`);
      },
      clearError: () => {
        calls.push('clear_error');
      },
      runCompletion: async () => {
        calls.push('run_completion');
      },
      onError: (err) => {
        calls.push(`error:${String(err)}`);
      },
    });

    expect(didRun).toBe(true);
    expect(calls).toEqual([
      'set_rendering:true',
      'clear_error',
      'run_completion',
      'set_rendering:false',
    ]);
  });

  it('reports capture host completion errors and still clears rendering state', async () => {
    const calls: string[] = [];

    const didRun = await runCaptureHostCompletionFlow({
      guardCompletion: true,
      isCompleting: () => false,
      setCompleting: (value) => {
        calls.push(`set_completing:${value}`);
      },
      setRendering: (value) => {
        calls.push(`set_rendering:${value}`);
      },
      clearError: () => {
        calls.push('clear_error');
      },
      runCompletion: async () => {
        calls.push('run_completion');
        throw new Error('completion failed');
      },
      onError: (err) => {
        calls.push(err instanceof Error ? `error:${err.message}` : 'error');
      },
    });

    expect(didRun).toBe(false);
    expect(calls).toEqual([
      'set_completing:true',
      'set_rendering:true',
      'clear_error',
      'run_completion',
      'error:completion failed',
      'set_completing:false',
      'set_rendering:false',
    ]);
  });

  it('renders capture host preview through the shared rendering lifecycle', async () => {
    const calls: string[] = [];

    const didRender = await runCaptureHostPreviewRender({
      sessionId: 'session-preview',
      rect: { x: 10, y: 20, width: 120, height: 80 },
      annotations: [
        {
          type: 'rectangle',
          rect: { x: 1, y: 1, width: 10, height: 10 },
          color: [255, 0, 0, 255],
          stroke_width: 2,
          filled: false,
        },
      ],
      includeCursor: true,
      setRendering: (value) => {
        calls.push(`set_rendering:${value}`);
      },
      clearPreview: () => {
        calls.push('clear_preview');
      },
      clearError: () => {
        calls.push('clear_error');
      },
      renderPreview: async (input) => {
        calls.push(
          `render:${input.sessionId}:${input.rect.width}:${input.annotations.length}:${input.includeCursor}`,
        );
        return 'preview-base64';
      },
      setPreviewImage: (base64) => {
        calls.push(`set_preview:${base64}`);
      },
      onError: (err) => {
        calls.push(`error:${String(err)}`);
      },
    });

    expect(didRender).toBe(true);
    expect(calls).toEqual([
      'set_rendering:true',
      'clear_preview',
      'clear_error',
      'render:session-preview:120:1:true',
      'set_preview:preview-base64',
      'set_rendering:false',
    ]);
  });

  it('reports capture host preview render errors and still clears rendering state', async () => {
    const calls: string[] = [];

    const didRender = await runCaptureHostPreviewRender({
      sessionId: 'session-preview',
      rect: { x: 10, y: 20, width: 120, height: 80 },
      annotations: [],
      includeCursor: false,
      setRendering: (value) => {
        calls.push(`set_rendering:${value}`);
      },
      clearPreview: () => {
        calls.push('clear_preview');
      },
      clearError: () => {
        calls.push('clear_error');
      },
      renderPreview: async (input) => {
        calls.push(`render:${'includeCursor' in input}`);
        throw new Error('preview failed');
      },
      setPreviewImage: (base64) => {
        calls.push(`set_preview:${base64}`);
      },
      onError: (err) => {
        calls.push(err instanceof Error ? `error:${err.message}` : 'error');
      },
    });

    expect(didRender).toBe(false);
    expect(calls).toEqual([
      'set_rendering:true',
      'clear_preview',
      'clear_error',
      'render:false',
      'error:preview failed',
      'set_rendering:false',
    ]);
  });

  it('runs capture host transition effects after applying transition output state', async () => {
    const calls: string[] = [];

    const didRun = await runCaptureHostTransitionEffects({
      rendering: true,
      error: null,
      setRendering: (value) => {
        calls.push(`set_rendering:${value}`);
      },
      setError: (value) => {
        calls.push(`set_error:${value ?? 'none'}`);
      },
      runEffects: async () => {
        calls.push('run_effects');
      },
      onError: (err) => {
        calls.push(`error:${String(err)}`);
      },
    });

    expect(didRun).toBe(true);
    expect(calls).toEqual([
      'set_rendering:true',
      'set_error:none',
      'run_effects',
      'set_rendering:false',
    ]);
  });

  it('reports capture host transition effect errors and still clears rendering state', async () => {
    const calls: string[] = [];

    const didRun = await runCaptureHostTransitionEffects({
      rendering: true,
      error: 'previous warning',
      setRendering: (value) => {
        calls.push(`set_rendering:${value}`);
      },
      setError: (value) => {
        calls.push(`set_error:${value ?? 'none'}`);
      },
      runEffects: async () => {
        calls.push('run_effects');
        throw new Error('effects failed');
      },
      onError: (err) => {
        calls.push(err instanceof Error ? `error:${err.message}` : 'error');
      },
    });

    expect(didRun).toBe(false);
    expect(calls).toEqual([
      'set_rendering:true',
      'set_error:previous warning',
      'run_effects',
      'error:effects failed',
      'set_rendering:false',
    ]);
  });

  it('runs explicit effect lists through the shared host runtime executor', async () => {
    const calls: string[] = [];

    await runCaptureRuntimeEffects(
      [
        { type: 'run-ocr', target: 'ocr-window' },
        { type: 'record-selection', action: 'ocr' },
        { type: 'finish-session' },
      ],
      {
        sessionId: 'session-5',
        rect: { x: 10, y: 20, width: 120, height: 80 },
        includeCursor: false,
        recordSuccessfulSelection: (action) => {
          calls.push(`record:${action}`);
        },
        finishCaptureSession: async (sessionId) => {
          calls.push(`finish:${sessionId}`);
        },
      },
      {
        runCaptureOcr: async () => {
          calls.push('run_ocr');
          return { text: 'hello', confidence: 1 };
        },
        normalizeOcrText: (text) => {
          calls.push(`normalize:${text}`);
          return text.toUpperCase();
        },
        renderCaptureOutput: async () => {
          calls.push('render');
          return 'image-base64';
        },
        openCaptureOcrResultWindow: async (text, imageBase64) => {
          calls.push(`open_ocr:${text}:${imageBase64}`);
        },
      },
    );

    expect(calls).toEqual([
      'run_ocr',
      'normalize:hello',
      'render',
      'open_ocr:HELLO:image-base64',
      'record:ocr',
      'finish:session-5',
    ]);
  });

  it('loads a capture host session and primes hover selection from the captured cursor', async () => {
    const calls: string[] = [];
    const session = createCaptureSessionView({
      captured_cursor: {
        logical_position: { x: 120, y: 120 },
        hotspot: { x: 0, y: 0 },
        image_width: 16,
        image_height: 16,
        scale_factor: 2,
        image_base64: '',
      },
    });

    const result = await loadCaptureHostSession({
      loadSession: async () => {
        calls.push('load_session');
        return session;
      },
      getCurrentCursorPosition: async () => {
        calls.push('get_cursor');
        return { x: 400, y: 200 };
      },
    });

    expect(calls).toEqual(['load_session']);
    expect(result).toEqual({
      session,
      cursorPoint: { x: 120, y: 120 },
      hoverSelection: { x: 100, y: 100, width: 200, height: 120 },
    });
  });

  it('keeps loading a capture host session when initial cursor lookup fails', async () => {
    const calls: string[] = [];
    const session = createCaptureSessionView({ id: 'session-without-cursor' });

    const result = await loadCaptureHostSession({
      loadSession: async () => {
        calls.push('load_session');
        return session;
      },
      getCurrentCursorPosition: async (sessionId) => {
        calls.push(`get_cursor:${sessionId}`);
        throw new Error('cursor unavailable');
      },
    });

    expect(calls).toEqual([
      'load_session',
      'get_cursor:session-without-cursor',
    ]);
    expect(result).toEqual({
      session,
      cursorPoint: null,
      hoverSelection: null,
    });
  });

  it('reuses in-flight capture snapshot hydration for the same session', () => {
    const calls: string[] = [];
    const hydrationPromise = Promise.resolve(createCaptureSessionView());
    const currentHydration: CaptureHostSnapshotHydration = {
      sessionId: 'session-1',
      promise: hydrationPromise,
    };

    const result = ensureCaptureHostSnapshotsHydrated({
      sessionId: 'session-1',
      getCurrentHydration: () => currentHydration,
      setCurrentHydration: () => {
        calls.push('set_hydration');
      },
      hydrateSnapshots: async () => {
        calls.push('hydrate');
        return createCaptureSessionView();
      },
      clearHydratedSession: () => {
        calls.push('clear_ready');
      },
      applyHydratedSession: () => {
        calls.push('apply');
      },
      markHydratedSession: () => {
        calls.push('mark_ready');
      },
    });

    expect(result).toBe(hydrationPromise);
    expect(calls).toEqual([]);
  });

  it('applies hydrated snapshots only while the hydration request is current', async () => {
    const calls: string[] = [];
    let currentHydration: CaptureHostSnapshotHydration | null = null;
    const hydratedSession = createCaptureSessionView({ id: 'session-2' });

    const result = ensureCaptureHostSnapshotsHydrated({
      sessionId: 'session-2',
      getCurrentHydration: () => currentHydration,
      setCurrentHydration: (hydration) => {
        currentHydration = hydration;
        calls.push(`set_hydration:${hydration?.sessionId ?? 'none'}`);
      },
      hydrateSnapshots: async (sessionId) => {
        calls.push(`hydrate:${sessionId}`);
        return hydratedSession;
      },
      clearHydratedSession: () => {
        calls.push('clear_ready');
      },
      applyHydratedSession: (sessionId, session) => {
        calls.push(`apply:${sessionId}:${session.id}`);
      },
      markHydratedSession: (sessionId) => {
        calls.push(`mark_ready:${sessionId}`);
      },
      markSnapshotsHydrated: (sessionId) => {
        calls.push(`perf:${sessionId}`);
      },
    });

    await expect(result).resolves.toBe(hydratedSession);
    expect(calls).toEqual([
      'clear_ready',
      'hydrate:session-2',
      'set_hydration:session-2',
      'apply:session-2:session-2',
      'mark_ready:session-2',
      'perf:session-2',
    ]);
  });

  it('skips stale hydrated snapshot application when another request replaced it', async () => {
    const calls: string[] = [];
    let currentHydration: CaptureHostSnapshotHydration | null = null;
    let resolveHydration: (session: CaptureSessionView) => void = () => undefined;
    const hydrationPromise = new Promise<CaptureSessionView>((resolve) => {
      resolveHydration = resolve;
    });
    const hydratedSession = createCaptureSessionView({ id: 'session-3' });

    const result = ensureCaptureHostSnapshotsHydrated({
      sessionId: 'session-3',
      getCurrentHydration: () => currentHydration,
      setCurrentHydration: (hydration) => {
        currentHydration = hydration;
        calls.push(`set_hydration:${hydration?.sessionId ?? 'none'}`);
      },
      hydrateSnapshots: async () => hydrationPromise,
      clearHydratedSession: () => {
        calls.push('clear_ready');
      },
      applyHydratedSession: () => {
        calls.push('apply');
      },
      markHydratedSession: () => {
        calls.push('mark_ready');
      },
    });
    currentHydration = {
      sessionId: 'newer-session',
      promise: Promise.resolve(createCaptureSessionView({ id: 'newer-session' })),
    };

    resolveHydration(hydratedSession);
    await expect(result).resolves.toBe(hydratedSession);

    expect(calls).toEqual(['clear_ready', 'set_hydration:session-3']);
  });

  it('clears only the active snapshot hydration after a hydration failure', async () => {
    const calls: string[] = [];
    let currentHydration: CaptureHostSnapshotHydration | null = null;

    const result = ensureCaptureHostSnapshotsHydrated({
      sessionId: 'session-4',
      getCurrentHydration: () => currentHydration,
      setCurrentHydration: (hydration) => {
        currentHydration = hydration;
        calls.push(`set_hydration:${hydration?.sessionId ?? 'none'}`);
      },
      hydrateSnapshots: async () => {
        throw new Error('hydrate failed');
      },
      clearHydratedSession: () => {
        calls.push('clear_ready');
      },
      applyHydratedSession: () => {
        calls.push('apply');
      },
      markHydratedSession: () => {
        calls.push('mark_ready');
      },
    });

    await expect(result).rejects.toThrow('hydrate failed');
    expect(calls).toEqual([
      'clear_ready',
      'set_hydration:session-4',
      'set_hydration:none',
      'clear_ready',
    ]);
  });

  it('starts a capture host session with loading, perf, and loaded-session application in order', async () => {
    const calls: string[] = [];
    const loadedSession = {
      session: createCaptureSessionView({ id: 'session-7' }),
      cursorPoint: null,
      hoverSelection: null,
    };

    const result = await runCaptureHostSessionStart({
      mode: 'screenshot-copy',
      now: () => 123,
      setCancelling: (value) => {
        calls.push(`set_cancelling:${value}`);
      },
      setRevealed: (value) => {
        calls.push(`set_revealed:${value}`);
      },
      showLoading: (mode) => {
        calls.push(`loading:${mode}`);
      },
      resetInteractionState: () => {
        calls.push('reset_interaction');
      },
      setPerfState: (state) => {
        calls.push(
          `perf_state:${state.mode}:${state.sessionId ?? 'none'}:${state.startMs}:${state.hasLoggedImagesReady}`,
        );
      },
      setLoadedPerfSessionId: (sessionId) => {
        calls.push(`perf_session:${sessionId}`);
      },
      markPerf: (event, sessionId) => {
        calls.push(`perf:${event}:${sessionId ?? 'none'}`);
      },
      loadSession: async () => {
        calls.push('load_session');
        return loadedSession;
      },
      applyLoadedSession: (loaded) => {
        calls.push(`apply:${loaded.session.id}`);
      },
      onError: (err) => {
        calls.push(`error:${String(err)}`);
      },
    });

    expect(result).toBe(loadedSession);
    expect(calls).toEqual([
      'set_cancelling:false',
      'set_revealed:false',
      'loading:screenshot-copy',
      'reset_interaction',
      'perf_state:screenshot-copy:none:123:false',
      'perf:start_session:none',
      'load_session',
      'perf_session:session-7',
      'perf:session_loaded:session-7',
      'apply:session-7',
    ]);
  });

  it('starts an existing capture host session with the launch session id in initial perf state', async () => {
    const calls: string[] = [];
    const loadedSession = {
      session: createCaptureSessionView({ id: 'session-loaded' }),
      cursorPoint: null,
      hoverSelection: null,
    };

    await runCaptureHostSessionStart({
      mode: 'screenshot',
      sessionId: 'session-launch',
      now: () => 456,
      setCancelling: () => undefined,
      setRevealed: () => undefined,
      showLoading: () => undefined,
      resetInteractionState: () => undefined,
      setPerfState: (state) => {
        calls.push(`perf_state:${state.sessionId ?? 'none'}`);
      },
      setLoadedPerfSessionId: (sessionId) => {
        calls.push(`perf_session:${sessionId}`);
      },
      markPerf: (event, sessionId) => {
        calls.push(`perf:${event}:${sessionId ?? 'none'}`);
      },
      loadSession: async () => loadedSession,
      applyLoadedSession: () => undefined,
      onError: () => undefined,
    });

    expect(calls).toEqual([
      'perf_state:session-launch',
      'perf:start_session:session-launch',
      'perf_session:session-loaded',
      'perf:session_loaded:session-loaded',
    ]);
  });

  it('reports start-session errors without applying a loaded session', async () => {
    const calls: string[] = [];

    const result = await runCaptureHostSessionStart({
      mode: 'screenshot-ocr',
      now: () => 789,
      setCancelling: (value) => {
        calls.push(`set_cancelling:${value}`);
      },
      setRevealed: (value) => {
        calls.push(`set_revealed:${value}`);
      },
      showLoading: (mode) => {
        calls.push(`loading:${mode}`);
      },
      resetInteractionState: () => {
        calls.push('reset_interaction');
      },
      setPerfState: (state) => {
        calls.push(`perf_state:${state.mode}`);
      },
      setLoadedPerfSessionId: (sessionId) => {
        calls.push(`perf_session:${sessionId}`);
      },
      markPerf: (event, sessionId) => {
        calls.push(`perf:${event}:${sessionId ?? 'none'}`);
      },
      loadSession: async () => {
        calls.push('load_session');
        throw new Error('load failed');
      },
      applyLoadedSession: () => {
        calls.push('apply');
      },
      onError: (err) => {
        calls.push(err instanceof Error ? `error:${err.message}` : 'error');
      },
    });

    expect(result).toBeNull();
    expect(calls).toEqual([
      'set_cancelling:false',
      'set_revealed:false',
      'loading:screenshot-ocr',
      'reset_interaction',
      'perf_state:screenshot-ocr',
      'perf:start_session:none',
      'load_session',
      'error:load failed',
    ]);
  });

  it('ignores capture host refresh when no current session is active', async () => {
    const calls: string[] = [];

    const result = await runCaptureHostSessionRefresh({
      sessionId: null,
      setRevealed: (value) => {
        calls.push(`set_revealed:${value}`);
      },
      showLoading: () => {
        calls.push('loading');
      },
      resetInteractionState: () => {
        calls.push('reset_interaction');
      },
      loadSession: async () => {
        calls.push('load_session');
        return {
          session: createCaptureSessionView(),
          cursorPoint: null,
          hoverSelection: null,
        };
      },
      applyLoadedSession: () => {
        calls.push('apply');
      },
      onError: (err) => {
        calls.push(`error:${String(err)}`);
      },
    });

    expect(result).toBeNull();
    expect(calls).toEqual([]);
  });

  it('refreshes an active capture host session through loading and loaded-session application', async () => {
    const calls: string[] = [];
    const loadedSession = {
      session: createCaptureSessionView({ id: 'session-refreshed' }),
      cursorPoint: null,
      hoverSelection: null,
    };

    const result = await runCaptureHostSessionRefresh({
      sessionId: 'session-current',
      setRevealed: (value) => {
        calls.push(`set_revealed:${value}`);
      },
      showLoading: () => {
        calls.push('loading');
      },
      resetInteractionState: () => {
        calls.push('reset_interaction');
      },
      loadSession: async (sessionId) => {
        calls.push(`load_session:${sessionId}`);
        return loadedSession;
      },
      applyLoadedSession: (loaded) => {
        calls.push(`apply:${loaded.session.id}`);
      },
      onError: (err) => {
        calls.push(`error:${String(err)}`);
      },
    });

    expect(result).toBe(loadedSession);
    expect(calls).toEqual([
      'set_revealed:false',
      'loading',
      'reset_interaction',
      'load_session:session-current',
      'apply:session-refreshed',
    ]);
  });

  it('reports capture host refresh errors without applying a loaded session', async () => {
    const calls: string[] = [];

    const result = await runCaptureHostSessionRefresh({
      sessionId: 'session-current',
      setRevealed: (value) => {
        calls.push(`set_revealed:${value}`);
      },
      showLoading: () => {
        calls.push('loading');
      },
      resetInteractionState: () => {
        calls.push('reset_interaction');
      },
      loadSession: async (sessionId) => {
        calls.push(`load_session:${sessionId}`);
        throw new Error('refresh failed');
      },
      applyLoadedSession: () => {
        calls.push('apply');
      },
      onError: (err) => {
        calls.push(err instanceof Error ? `error:${err.message}` : 'error');
      },
    });

    expect(result).toBeNull();
    expect(calls).toEqual([
      'set_revealed:false',
      'loading',
      'reset_interaction',
      'load_session:session-current',
      'error:refresh failed',
    ]);
  });
});
