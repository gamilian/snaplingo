import { describe, expect, it, vi } from 'vitest';

import type { CaptureWorkspacePlatformRuntime } from './platformRuntime';
import { createCaptureWorkspaceRuntime } from './runtime';

const selection = { x: 20, y: 30, width: 120, height: 80 };

function createKeyboardTarget() {
  const listeners = new Map<string, Set<(event: KeyboardEvent) => void>>();
  return {
    target: {
      addEventListener(type: string, listener: (event: KeyboardEvent) => void) {
        const current = listeners.get(type) ?? new Set();
        current.add(listener);
        listeners.set(type, current);
      },
      removeEventListener(type: string, listener: (event: KeyboardEvent) => void) {
        listeners.get(type)?.delete(listener);
      },
    },
    dispatch(key: string) {
      let defaultPrevented = false;
      const event = {
        key,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
        repeat: false,
        preventDefault: () => {
          defaultPrevented = true;
        },
        get defaultPrevented() {
          return defaultPrevented;
        },
      } as KeyboardEvent;
      listeners.get('keydown')?.forEach((listener) => listener(event));
      return event;
    },
    listenerCount(type: string) {
      return listeners.get(type)?.size ?? 0;
    },
  };
}

describe('capture workspace runtime', () => {
  it('disposes acquired and late host registrations exactly once', async () => {
    const platform = createPlatform();
    const keyboardTarget = createKeyboardTarget();
    const disposeHotkey = vi.fn();
    const disposeCancel = vi.fn();
    const cancelRegistration = deferred<() => void>();
    platform.onHotkeyTriggered.mockResolvedValue(disposeHotkey);
    platform.onCancelRequested.mockReturnValue(cancelRegistration.promise);
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      keyboard: { target: keyboardTarget.target },
    });

    const connecting = runtime.actions.connectHost();
    await vi.waitFor(() =>
      expect(platform.onCancelRequested).toHaveBeenCalledOnce(),
    );
    runtime.dispose();

    expect(keyboardTarget.listenerCount('keydown')).toBe(0);
    expect(keyboardTarget.listenerCount('keyup')).toBe(0);
    expect(keyboardTarget.listenerCount('blur')).toBe(0);
    expect(disposeHotkey).toHaveBeenCalledOnce();

    cancelRegistration.resolve(disposeCancel);
    const disconnect = await connecting;
    disconnect();
    disconnect();

    expect(disposeCancel).toHaveBeenCalledOnce();
    expect(platform.onCopyRequested).not.toHaveBeenCalled();
    expect(disposeHotkey).toHaveBeenCalledOnce();
  });

  it('disposes a late third host registration without retaining listeners', async () => {
    const platform = createPlatform();
    const keyboardTarget = createKeyboardTarget();
    const disposeHotkey = vi.fn();
    const disposeCancel = vi.fn();
    const disposeCopy = vi.fn();
    const copyRegistration = deferred<() => void>();
    platform.onHotkeyTriggered.mockResolvedValue(disposeHotkey);
    platform.onCancelRequested.mockResolvedValue(disposeCancel);
    platform.onCopyRequested.mockReturnValue(copyRegistration.promise);
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      keyboard: { target: keyboardTarget.target },
    });

    const connecting = runtime.actions.connectHost();
    await vi.waitFor(() =>
      expect(platform.onCopyRequested).toHaveBeenCalledOnce(),
    );
    runtime.dispose();
    copyRegistration.resolve(disposeCopy);
    const disconnect = await connecting;
    disconnect();

    expect(disposeHotkey).toHaveBeenCalledOnce();
    expect(disposeCancel).toHaveBeenCalledOnce();
    expect(disposeCopy).toHaveBeenCalledOnce();
    expect(keyboardTarget.listenerCount('keydown')).toBe(0);
    expect(keyboardTarget.listenerCount('keyup')).toBe(0);
    expect(keyboardTarget.listenerCount('blur')).toBe(0);
  });

  it('cancels a session that resolves after disposal without adopting it', async () => {
    const platform = createPlatform();
    const session = createSession({ id: 'late-disposed-session' });
    const sessionRequest = deferred<typeof session>();
    platform.commands.getCaptureSession.mockReturnValue(sessionRequest.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    const starting = runtime.actions.startSession(
      'screenshot',
      'late-disposed-session',
    );
    runtime.dispose();
    sessionRequest.resolve(session);
    await starting;

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'late-disposed-session',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'idle',
      sessionId: null,
    });

    await runtime.actions.startSession('screenshot', 'ignored-after-dispose');
    expect(platform.commands.getCaptureSession).toHaveBeenCalledTimes(1);
  });

  it('cancels a resolved provisional session while cursor loading is pending', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'provisional-session' }),
    });
    const cursor = deferred<{ x: number; y: number } | null>();
    platform.commands.currentCaptureCursorPosition.mockReturnValue(
      cursor.promise,
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });

    const starting = runtime.actions.startSession(
      'screenshot',
      'provisional-session',
    );
    await vi.waitFor(() =>
      expect(
        platform.commands.currentCaptureCursorPosition,
      ).toHaveBeenCalledOnce(),
    );
    runtime.dispose();

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledOnce();
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'provisional-session',
    );
    cursor.resolve(null);
    await starting;
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledOnce();
  });

  it('cancels an active session once and invalidates pending preview work', async () => {
    const platform = createPlatform();
    const preview = deferred<string>();
    platform.commands.renderCaptureOutput.mockReturnValue(preview.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-1');

    const rendering = runtime.actions.renderSelectionPreview(selection);
    runtime.dispose();
    runtime.dispose();
    preview.resolve('late-preview');
    await rendering;

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-1',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'idle',
      sessionId: null,
      previewImageBase64: null,
      isRenderingOutput: false,
    });
  });

  it('invalidates pending terminal output when disposing an active session', async () => {
    const platform = createPlatform();
    const output = deferred<void>();
    platform.commands.outputCapture.mockReturnValue(output.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-1');
    await runtime.actions.renderSelectionPreview(selection);

    const completing = runtime.actions.completePreviewSelection(
      'copy',
      selection,
    );
    await vi.waitFor(() =>
      expect(platform.commands.outputCapture).toHaveBeenCalledOnce(),
    );
    runtime.dispose();
    output.resolve();
    await completing;

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1);
    expect(runtime.renderState).toMatchObject({
      status: 'idle',
      sessionId: null,
      isRenderingOutput: false,
      error: null,
    });
  });

  it('cancels both the previous and late-created sessions when disposed during refresh', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'refresh-previous' }),
    });
    const createdSession = createSession({ id: 'refresh-created-late' });
    const created = deferred<typeof createdSession>();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'refresh-previous');
    platform.commands.createCaptureSession.mockReturnValue(created.promise);

    const refreshing = runtime.actions.refreshSession();
    runtime.dispose();

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'refresh-previous',
    );
    created.resolve(createdSession);
    await refreshing;
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([sessionId]) => sessionId === 'refresh-previous',
      ),
    ).toHaveLength(1);
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([sessionId]) => sessionId === 'refresh-created-late',
      ),
    ).toHaveLength(1);
  });

  it('cancels both the previous and late-loaded sessions when disposed during replacement', async () => {
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    const replacementSession = createSession({ id: 'replacement-late' });
    const replacement = deferred<typeof replacementSession>();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'replacement-previous');
    platform.commands.getCaptureSession.mockReturnValue(replacement.promise);

    const replacing = runtime.actions.startSession(
      'screenshot',
      'replacement-late',
    );
    runtime.dispose();

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'replacement-previous',
    );
    replacement.resolve(replacementSession);
    await replacing;
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([sessionId]) => sessionId === 'replacement-previous',
      ),
    ).toHaveLength(1);
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([sessionId]) => sessionId === 'replacement-late',
      ),
    ).toHaveLength(1);
  });

  it('restores the previous preview when replacement loading fails', async () => {
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) => {
      if (sessionId === 'replacement-failed') {
        throw new Error('replacement load failed');
      }
      return createSession({ id: sessionId });
    });
    platform.commands.hydrateCaptureSessionSnapshots.mockImplementation(
      async (sessionId) =>
        createSession({
          id: sessionId,
          monitors: [createMonitor({ image_base64: 'hydrated-pixels' })],
        }),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'replacement-previous');
    await runtime.actions.hydrateSnapshots();
    await runtime.actions.renderSelectionPreview(selection);

    await runtime.actions.startSession('screenshot', 'replacement-failed');

    expect(runtime.renderState).toMatchObject({
      status: 'preview',
      sessionId: 'replacement-previous',
      selection,
      previewImageBase64: 'preview-image',
      hasHydratedPixelSource: true,
      error: 'replacement load failed',
    });
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();

    await runtime.actions.startSession('screenshot', 'replacement-success');
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'replacement-previous',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'replacement-success',
      error: null,
    });
  });

  it('restores the previous preview when refresh creation fails', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'refresh-previous' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'refresh-previous');
    await runtime.actions.renderSelectionPreview(selection);
    platform.commands.createCaptureSession.mockRejectedValueOnce(
      new Error('refresh create failed'),
    );

    await runtime.actions.refreshSession();

    expect(runtime.renderState).toMatchObject({
      status: 'preview',
      sessionId: 'refresh-previous',
      selection,
      previewImageBase64: 'preview-image',
      error: 'refresh create failed',
    });
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();

    platform.commands.createCaptureSession.mockResolvedValue(
      createSession({ id: 'refresh-success' }),
    );
    await runtime.actions.refreshSession();
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'refresh-previous',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'refresh-success',
      error: null,
    });
  });

  it('cancels a provisional previous session after a stale replacement failure', async () => {
    const failedReplacement = deferred<ReturnType<typeof createSession>>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation((sessionId) => {
      if (sessionId === 'replacement-stale') {
        return failedReplacement.promise;
      }
      return Promise.resolve(createSession({ id: sessionId }));
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'replacement-previous');

    const stale = runtime.actions.startSession(
      'screenshot',
      'replacement-stale',
    );
    await runtime.actions.startSession('screenshot', 'replacement-current');
    failedReplacement.reject(new Error('stale replacement failed'));
    await stale;

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'replacement-previous',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'replacement-current',
      error: null,
    });
  });

  it('prevents runtime and delegated editor shortcuts synchronously', async () => {
    const platform = createPlatform();
    const keyboardTarget = createKeyboardTarget();
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      keyboard: {
        target: keyboardTarget.target,
      },
    });
    await runtime.actions.connectHost();
    await runtime.actions.startSession('screenshot', 'session-key-listener');

    expect(keyboardTarget.dispatch('F5').defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(runtime.renderState.status).toBe('selecting'));
    await runtime.actions.renderSelectionPreview(selection);
    expect(keyboardTarget.dispatch('t').defaultPrevented).toBe(true);
  });

  it('contains an unexpected rejection from work launched by a synchronous shortcut', async () => {
    const platform = createPlatform();
    platform.commands.renderCaptureOutput.mockRejectedValue(
      new Error('selection render failed'),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-key-rejection');

    expect(runtime.actions.keyDown({ key: 'a', metaKey: true })).toBe(true);

    await vi.waitFor(() =>
      expect(runtime.renderState).toMatchObject({
        status: 'error',
        error: 'selection render failed',
      }),
    );
  });

  it('owns host subscriptions and cleans every listener up together', async () => {
    const platform = createPlatform();
    const unlistenHotkey = vi.fn();
    const unlistenCancel = vi.fn();
    const unlistenCopy = vi.fn();
    const unlistenSave = vi.fn();
    platform.onHotkeyTriggered.mockResolvedValue(unlistenHotkey);
    platform.onCancelRequested.mockResolvedValue(unlistenCancel);
    platform.onCopyRequested.mockResolvedValue(unlistenCopy);
    platform.onSaveRequested.mockResolvedValue(unlistenSave);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    const disconnect = await runtime.actions.connectHost();
    const launch = platform.onHotkeyTriggered.mock.calls[0]?.[0];
    await launch?.({ mode: 'screenshot', sessionId: 'session-host' });

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-1',
    });
    expect(platform.onCancelRequested).toHaveBeenCalledTimes(1);
    expect(platform.onCopyRequested).toHaveBeenCalledTimes(1);
    expect(platform.onSaveRequested).toHaveBeenCalledTimes(1);

    disconnect();
    expect(unlistenHotkey).toHaveBeenCalledTimes(1);
    expect(unlistenCancel).toHaveBeenCalledTimes(1);
    expect(unlistenCopy).toHaveBeenCalledTimes(1);
    expect(unlistenSave).toHaveBeenCalledTimes(1);
  });

  it.each([
    ['second', 'cancel'],
    ['third', 'copy'],
    ['fourth', 'save'],
  ] as const)('cleans partial subscriptions when the %s registration fails', async (_label, failure) => {
    const platform = createPlatform();
    const keyboardTarget = createKeyboardTarget();
    const unlistenHotkey = vi.fn();
    const unlistenCancel =
      failure === 'copy' || failure === 'save'
        ? vi.fn(() => {
            throw new Error('cancel dispose failed');
          })
        : vi.fn();
    const unlistenCopy = vi.fn();
    platform.onHotkeyTriggered.mockResolvedValue(unlistenHotkey);
    if (failure === 'cancel') {
      platform.onCancelRequested.mockRejectedValue(new Error('cancel listen failed'));
    } else if (failure === 'copy') {
      platform.onCancelRequested.mockResolvedValue(unlistenCancel);
      platform.onCopyRequested.mockRejectedValue(new Error('copy listen failed'));
    } else {
      platform.onCancelRequested.mockResolvedValue(unlistenCancel);
      platform.onCopyRequested.mockResolvedValue(unlistenCopy);
      platform.onSaveRequested.mockRejectedValue(new Error('save listen failed'));
    }
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      keyboard: {
        target: keyboardTarget.target,
      },
    });

    await runtime.actions.connectHost();

    expect(unlistenHotkey).toHaveBeenCalledOnce();
    expect(unlistenCancel).toHaveBeenCalledTimes(failure === 'cancel' ? 0 : 1);
    expect(unlistenCopy).toHaveBeenCalledTimes(failure === 'save' ? 1 : 0);
    expect(keyboardTarget.listenerCount('keydown')).toBe(0);
    expect(keyboardTarget.listenerCount('keyup')).toBe(0);
    expect(keyboardTarget.listenerCount('blur')).toBe(0);
  });

  it('handles native preview copy through runtime-owned completion effects', async () => {
    const annotation = {
      type: 'text' as const,
      position: { x: 10, y: 10 },
      text: 'SnapLingo',
      color: [255, 0, 0, 255] as [number, number, number, number],
      font_size: 24,
    };
    const platform = createPlatform({
      session: createSession({ id: 'session-native-copy' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-native-copy');
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.applySelectedAnnotationStyle(
      { color: [255, 0, 0, 255], strokeWidth: 2, filled: false },
      24,
    );
    runtime.actions.toggleAnnotationTool('text');
    runtime.actions.pointerDown({
      point: { x: 30, y: 40 },
      source: 'preview',
    });
    runtime.actions.updateTextDraftText('SnapLingo');
    await runtime.actions.connectHost();

    const copy = platform.onCopyRequested.mock.calls[0]?.[0];
    await copy?.();

    expect(platform.commands.outputCapture).toHaveBeenCalledWith({
      sessionId: 'session-native-copy',
      rect: selection,
      annotations: [annotation],
      action: { type: 'copy' },
    });
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-native-copy',
    );
  });

  it('handles native preview save through runtime-owned completion effects', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-native-save' }),
    });
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      screenshotPreferences: () => ({
        savePath: '/custom/captures',
        format: 'webp',
        quality: 73,
        namingRule: 'custom',
        customFileName: 'Review',
        autoCopy: true,
        defaultStrokeWidth: 6,
        defaultFontSize: 20,
        rememberLastTool: false,
        showSelectionSize: false,
        showMagnifier: true,
      }),
    });
    await runtime.actions.startSession('screenshot', 'session-native-save');
    await runtime.actions.renderSelectionPreview(selection);
    await runtime.actions.connectHost();

    const save = platform.onSaveRequested.mock.calls[0]?.[0];
    await save?.();

    expect(platform.commands.defaultCaptureSavePath).toHaveBeenCalledWith({
      directory: '/custom/captures',
      format: 'webp',
      namingRule: 'custom',
      customFileName: 'Review',
    });
    expect(platform.commands.outputCapture).toHaveBeenCalledWith({
      sessionId: 'session-native-save',
      rect: selection,
      annotations: [],
      action: {
        type: 'save',
        path: '/captures/capture.png',
        format: 'webp',
        quality: 73,
        copyAfterSave: true,
      },
    });
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-native-save',
    );
  });

  it('handles native undo and redo requests while preview editing is active', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-native-undo' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-native-undo');
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'preview' });
    await runtime.actions.pointerUp({ point: { x: 70, y: 80 }, source: 'preview' });
    await runtime.actions.connectHost();

    await platform.onUndoRequested.mock.calls[0]?.[0]?.();
    expect(runtime.renderState.annotationHistory.annotations).toEqual([]);

    await platform.onRedoRequested.mock.calls[0]?.[0]?.();
    expect(runtime.renderState.annotationHistory.annotations).toHaveLength(1);
  });

  it('reveals once when runtime host readiness becomes complete', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-reveal' }),
    });
    const prepareSurface = vi.fn(async () => undefined);
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      host: {
        resetInteraction: vi.fn(),
        resetSession: vi.fn(),
        prepareSurface,
      },
    });

    await runtime.actions.startSession('screenshot', 'session-reveal');
    await runtime.actions.updateHostReadiness(false);
    await runtime.actions.updateHostReadiness(true);
    await runtime.actions.updateHostReadiness(true);

    expect(platform.prepareForReveal).toHaveBeenCalledTimes(1);
    expect(prepareSurface).toHaveBeenCalledTimes(1);
    expect(platform.reveal).toHaveBeenCalledTimes(1);
  });

  it('coalesces concurrent readiness and ignores a stale reveal preparation', async () => {
    const firstPrepare = deferred<void>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (id) =>
      createSession({ id }),
    );
    platform.prepareForReveal
      .mockImplementationOnce(() => firstPrepare.promise.then(() => undefined))
      .mockResolvedValue(undefined);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-a');

    const first = runtime.actions.updateHostReadiness(true);
    const duplicate = runtime.actions.updateHostReadiness(true);
    expect(platform.prepareForReveal).toHaveBeenCalledTimes(1);

    await runtime.actions.startSession('screenshot', 'session-b');
    await runtime.actions.updateHostReadiness(true);
    expect(platform.reveal).toHaveBeenCalledTimes(1);

    firstPrepare.resolve();
    await Promise.all([first, duplicate]);
    expect(platform.reveal).toHaveBeenCalledTimes(1);
    await runtime.actions.updateHostReadiness(true);
    expect(platform.reveal).toHaveBeenCalledTimes(1);
  });

  it('retries reveal preparation after the current attempt fails', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-reveal-retry' }),
    });
    platform.prepareForReveal
      .mockRejectedValueOnce(new Error('prepare failed'))
      .mockResolvedValue(undefined);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-reveal-retry');

    await runtime.actions.updateHostReadiness(true);
    expect(runtime.renderState).toMatchObject({
      status: 'error',
      error: 'prepare failed',
    });

    await runtime.actions.updateHostReadiness(true);

    expect(platform.prepareForReveal).toHaveBeenCalledTimes(2);
    expect(platform.reveal).toHaveBeenCalledOnce();
  });

  it('ignores refresh without an active session', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.refreshSession();

    expect(platform.commands.createCaptureSession).not.toHaveBeenCalled();
  });

  it('restores the previous session and cleans the unadopted refresh when cancellation fails', async () => {
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (id) =>
      createSession({ id }),
    );
    platform.commands.createCaptureSession.mockResolvedValue(
      createSession({ id: 'session-refresh-new' }),
    );
    platform.commands.cancelCaptureSession.mockImplementation(async (id) => {
      if (id === 'session-refresh-old') {
        throw new Error('cancel previous failed');
      }
      throw new Error('cleanup failed');
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-refresh-old');

    await runtime.actions.refreshSession();

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-refresh-old',
      error: 'cancel previous failed',
    });
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-refresh-old',
    );
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([id]) => id === 'session-refresh-new',
      ),
    ).toHaveLength(1);
  });

  it('retries a rejected previous-session cancellation on the next replacement', async () => {
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (id) =>
      createSession({ id }),
    );
    let previousCancellationAttempt = 0;
    platform.commands.cancelCaptureSession.mockImplementation(async (id) => {
      if (id === 'retry-cancel-previous') {
        previousCancellationAttempt += 1;
        if (previousCancellationAttempt === 1) {
          throw new Error('transient cancel failure');
        }
      }
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'retry-cancel-previous');

    await runtime.actions.startSession('screenshot', 'retry-cancel-first');
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'retry-cancel-previous',
      error: 'transient cancel failure',
    });
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([id]) => id === 'retry-cancel-previous',
      ),
    ).toHaveLength(1);
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([id]) => id === 'retry-cancel-first',
      ),
    ).toHaveLength(1);

    await runtime.actions.startSession('screenshot', 'retry-cancel-success');
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([id]) => id === 'retry-cancel-previous',
      ),
    ).toHaveLength(2);
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([id]) => id === 'retry-cancel-success',
      ),
    ).toHaveLength(0);
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([id]) => id === 'retry-cancel-first',
      ),
    ).toHaveLength(1);
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'retry-cancel-success',
      error: null,
    });
  });

  it('coalesces concurrent cancellation attempts while the native call is pending', async () => {
    const firstDismiss = deferred<void>();
    const nativeCancellation = deferred<void>();
    const platform = createPlatform({
      session: createSession({ id: 'concurrent-cancel' }),
    });
    platform.dismiss
      .mockImplementationOnce(() => firstDismiss.promise)
      .mockResolvedValue(undefined);
    platform.commands.cancelCaptureSession.mockReturnValue(
      nativeCancellation.promise,
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'concurrent-cancel');

    const first = runtime.actions.cancelSession();
    const second = runtime.actions.cancelSession();
    await vi.waitFor(() =>
      expect(platform.commands.cancelCaptureSession).toHaveBeenCalledOnce(),
    );
    firstDismiss.resolve();
    await Promise.resolve();
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledOnce();

    nativeCancellation.resolve();
    await Promise.all([first, second]);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledOnce();
    expect(runtime.renderState).toMatchObject({
      status: 'idle',
      sessionId: null,
    });
  });

  it('cleans a stale unadopted refresh session without changing its replacement', async () => {
    const cancelPrevious = deferred<void>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (id) =>
      createSession({ id }),
    );
    platform.commands.createCaptureSession.mockResolvedValue(
      createSession({ id: 'session-refresh-stale' }),
    );
    platform.commands.cancelCaptureSession.mockImplementation((id) => {
      if (id === 'session-refresh-old') return cancelPrevious.promise;
      if (id === 'session-refresh-stale') {
        return Promise.reject(new Error('late cleanup failed'));
      }
      return Promise.resolve();
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-refresh-old');

    const refresh = runtime.actions.refreshSession();
    await vi.waitFor(() =>
      expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
        'session-refresh-old',
      ),
    );
    await runtime.actions.startSession('screenshot', 'session-replacement');
    cancelPrevious.reject(new Error('late cancel previous failed'));
    await refresh;

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-replacement',
      error: null,
    });
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([id]) => id === 'session-refresh-stale',
      ),
    ).toHaveLength(1);
  });

  it('reports session start failures through runtime state', async () => {
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockRejectedValue(
      new Error('load failed'),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-failure');

    expect(runtime.renderState).toMatchObject({
      status: 'error',
      sessionId: null,
      error: 'load failed',
    });
  });

  it('clears terminal rendering state after a direct selecting output failure', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-direct-output-failure' }),
    });
    platform.commands.outputCapture.mockRejectedValue(
      new Error('direct output failed'),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot-copy',
      'session-direct-output-failure',
    );
    await runtime.actions.completeCandidateSelection(selection, 'copy');

    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(runtime.renderState).toMatchObject({
      status: 'error',
      sessionId: 'session-direct-output-failure',
      isRenderingOutput: false,
      error: 'direct output failed',
    });
  });

  it('keeps a canceled loading session idle when its load resolves later', async () => {
    const load = deferred<ReturnType<typeof createSession>>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(() => load.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    const start = runtime.actions.startSession('screenshot', 'session-late');
    await runtime.actions.cancelSession();
    load.resolve(createSession({ id: 'session-late' }));
    await start;

    expect(runtime.renderState.status).toBe('idle');
    expect(runtime.renderState.sessionId).toBeNull();
  });

  it('does not surface a stale preview failure after cancel', async () => {
    const preview = deferred<string>();
    const platform = createPlatform({ session: createSession({ id: 'session-preview-cancel' }) });
    platform.commands.renderCaptureOutput.mockImplementation(() => preview.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-preview-cancel');

    const rendering = runtime.actions.renderSelectionPreview(selection);
    await runtime.actions.cancelSession();
    preview.reject(new Error('late preview failure'));
    await rendering;

    expect(runtime.renderState).toMatchObject({ status: 'idle', error: null });
  });

  it('does not surface stale output or preview work after reset or cancel', async () => {
    const preview = deferred<string>();
    const output = deferred<void>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (id) => createSession({ id }));
    platform.commands.renderCaptureOutput.mockImplementationOnce(() => preview.promise);
    platform.commands.outputCapture.mockImplementationOnce(() => output.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-reset-preview');
    const rendering = runtime.actions.renderSelectionPreview(selection);
    runtime.actions.resetPreview();
    preview.reject(new Error('late reset preview failure'));
    await rendering;
    expect(runtime.renderState).toMatchObject({ status: 'selecting', error: null });
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();

    await runtime.actions.startSession('screenshot-copy', 'session-cancel-output');
    const completing = runtime.actions.completeCandidateSelection(selection, 'copy');
    await runtime.actions.cancelSession();
    output.reject(new Error('late cancel output failure'));
    await completing;
    expect(runtime.renderState).toMatchObject({ status: 'idle', error: null });
  });

  it('guards duplicate candidate completion while output is pending', async () => {
    const output = deferred<void>();
    const platform = createPlatform({
      session: createSession({ id: 'session-duplicate' }),
    });
    platform.commands.outputCapture.mockImplementation(() => output.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot-copy', 'session-duplicate');

    const first = runtime.actions.completeCandidateSelection(selection, 'copy');
    await runtime.actions.completeCandidateSelection(selection, 'copy');
    expect(platform.commands.outputCapture).toHaveBeenCalledTimes(1);

    output.resolve();
    await first;
  });

  it('owns refresh and full-area keyboard workflows', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-keyboard-host' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-keyboard-host');

    expect(runtime.actions.keyDown({ key: 'F5' })).toBe(true);
    await vi.waitFor(() => expect(runtime.renderState.status).toBe('selecting'));
    expect(runtime.actions.keyDown({ key: 'a', metaKey: true })).toBe(true);
    await vi.waitFor(() => expect(runtime.renderState.status).toBe('preview'));

    expect(platform.commands.createCaptureSession).toHaveBeenCalledTimes(1);
    expect(platform.commands.renderCaptureOutput).toHaveBeenCalledWith({
      sessionId: 'session-keyboard-host',
      rect: { x: 0, y: 0, width: 500, height: 300 },
      annotations: [],
    });
    expect(runtime.renderState.status).toBe('preview');
  });

  it('owns preview output and remembered-selection keyboard workflows', async () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
    };
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform, storage });

    await runtime.actions.startSession('screenshot-copy', 'session-record');
    await runtime.actions.completeCandidateSelection(selection, 'copy');
    await runtime.actions.startSession('screenshot', 'session-restore');
    expect(runtime.actions.keyDown({ key: 'r' })).toBe(true);
    await vi.waitFor(() => expect(runtime.renderState.isRenderingOutput).toBe(false));
    expect(runtime.actions.keyDown({ key: 'c', metaKey: true })).toBe(true);
    await vi.waitFor(() => expect(platform.commands.outputCapture).toHaveBeenCalledTimes(2));

    expect(platform.commands.renderCaptureOutput).toHaveBeenCalledWith({
      sessionId: 'session-restore',
      rect: selection,
      annotations: [],
    });
    expect(platform.commands.outputCapture).toHaveBeenLastCalledWith({
      sessionId: 'session-restore',
      rect: selection,
      annotations: [],
      action: { type: 'copy' },
    });
  });

  it('owns preview pin and root reset pointer decisions', async () => {
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-reset');
    await runtime.actions.renderSelectionPreview(selection);
    expect(
      runtime.actions.pointerDown({
        point: { x: 30, y: 40 },
        button: 2,
        source: 'root',
      }),
    ).toBe(true);
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      selection: null,
    });

    await runtime.actions.renderSelectionPreview(selection);
    expect(
      runtime.actions.pointerDown({
        point: { x: 30, y: 40 },
        button: 1,
        source: 'preview',
      }),
    ).toBe(true);
    await vi.waitFor(() => {
      expect(platform.commands.outputCapture).toHaveBeenCalledWith({
        sessionId: 'session-reset',
        rect: selection,
        annotations: [],
        action: { type: 'pin' },
      });
    });

    await runtime.actions.startSession('screenshot', 'session-copy');
    await runtime.actions.renderSelectionPreview(selection);
    expect(
      runtime.actions.pointerDown({
        point: { x: 30, y: 40 },
        button: 0,
        detail: 2,
        source: 'preview',
      }),
    ).toBe(true);
    await vi.waitFor(() => {
      expect(platform.commands.outputCapture).toHaveBeenCalledWith({
        sessionId: 'session-copy',
        rect: selection,
        annotations: [],
        action: { type: 'copy' },
      });
    });
  });

  it('owns annotation draw and preview commit transactions', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-draw' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-editor-draw');
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.toggleAnnotationTool('rectangle');

    expect(
      runtime.actions.pointerDown({
        point: { x: 30, y: 40 },
        source: 'preview',
      }),
    ).toBe(true);
    expect(
      runtime.actions.pointerMove({
        point: { x: 70, y: 80 },
        source: 'preview',
      }),
    ).toBe(true);
    await expect(
      runtime.actions.pointerUp({
        point: { x: 70, y: 80 },
        source: 'preview',
      }),
    ).resolves.toBe(true);

    expect(runtime.renderState.annotationHistory.annotations).toEqual([
      {
        type: 'rectangle',
        rect: { x: 10, y: 10, width: 40, height: 40 },
        color: [255, 77, 79, 255],
        stroke_width: 2,
        filled: false,
      },
    ]);
    expect(platform.commands.renderCaptureOutput).toHaveBeenLastCalledWith({
      sessionId: 'session-editor-draw',
      rect: selection,
      annotations: [],
    });
  });

  it('commits a pen stroke to canvas-backed history without retaining a draft', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-pen' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-editor-pen');
    await runtime.actions.renderSelectionPreview(selection);
    platform.commands.renderCaptureOutput.mockClear();
    runtime.actions.toggleAnnotationTool('pen');

    await runtime.actions.pointerDown({
      point: { x: 30, y: 40 },
      source: 'preview',
    });
    await runtime.actions.pointerMove({
      point: { x: 70, y: 80 },
      source: 'preview',
    });
    await runtime.actions.pointerUp({
      point: { x: 70, y: 80 },
      source: 'preview',
    });

    expect(runtime.renderState.previewImageBase64).toBe('preview-image');
    expect(runtime.renderState.annotationHistory.annotations[0]?.type).toBe(
      'freehand',
    );
    expect(runtime.renderState.draftAnnotation).toBeNull();
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
  });

  it('resizes a selected rectangle annotation from its edge handle', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-resize-annotation' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot',
      'session-editor-resize-annotation',
    );
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 70, y: 80 }, source: 'root' });
    runtime.actions.selectMoveTool();
    runtime.actions.pointerDown({ point: { x: 50, y: 60 }, source: 'preview' });
    await runtime.actions.pointerUp({ point: { x: 50, y: 60 }, source: 'root' });

    expect(
      runtime.actions.resizeAnnotationPointerDown('e', {
        point: { x: 70, y: 60 },
        source: 'preview',
      }),
    ).toBe(true);
    runtime.actions.pointerMove({ point: { x: 90, y: 60 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 90, y: 60 }, source: 'root' });

    expect(runtime.renderState.annotationHistory.annotations[0]).toMatchObject({
      type: 'rectangle',
      rect: { x: 10, y: 10, width: 60, height: 40 },
    });
  });

  it('preserves the captured cursor when delegated editor rerenders omit an override', async () => {
    const platform = createPlatform({
      session: createSession({
        id: 'session-editor-cursor',
        captured_cursor: {
          logical_position: { x: 30, y: 40 },
          hotspot: { x: 1, y: 2 },
          image_width: 16,
          image_height: 20,
          scale_factor: 2,
          image_base64: 'cursor-image',
        },
      }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-editor-cursor');
    await runtime.actions.renderSelectionPreview(selection);
    expect(runtime.actions.keyDown({ key: '`' })).toBe(true);
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );

    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 70, y: 80 }, source: 'root' });
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );

    runtime.actions.selectMoveTool();
    runtime.actions.pointerDown({ point: { x: 45, y: 55 }, source: 'preview' });
    platform.commands.renderCaptureOutput.mockClear();

    runtime.actions.pointerMove({ point: { x: 46, y: 55 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 46, y: 55 }, source: 'root' });
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
  });

  it('owns text draft, style, undo, and redo transactions', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-text' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-editor-text');
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.applySelectedAnnotationStyle(
      { color: [24, 144, 255, 255], strokeWidth: 4, filled: false },
      30,
    );
    runtime.actions.toggleAnnotationTool('text');
    runtime.actions.pointerDown({
      point: { x: 50, y: 60 },
      source: 'preview',
    });
    runtime.actions.updateTextDraftText('runtime text');
    runtime.actions.commitTextDraft();

    expect(runtime.renderState.textDraft).toBeNull();
    expect(runtime.renderState.activeAnnotationTool).toBeNull();
    expect(runtime.renderState.annotationHistory.annotations).toEqual([
      {
        type: 'text',
        position: { x: 30, y: 30 },
        text: 'runtime text',
        color: [24, 144, 255, 255],
        font_size: 30,
      },
    ]);

    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );
    expect(runtime.actions.keyDown({ key: 'z', metaKey: true })).toBe(true);
    expect(runtime.renderState.annotationHistory.annotations).toEqual([]);
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );
    expect(runtime.actions.keyDown({ key: 'y', metaKey: true })).toBe(true);
    expect(runtime.renderState.annotationHistory.annotations).toHaveLength(1);
  });

  it('updates canvas-backed undo and redo history without backend previews', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-latest' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-editor-latest');
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 70, y: 80 }, source: 'root' });
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );

    platform.commands.renderCaptureOutput.mockReset();
    expect(runtime.actions.keyDown({ key: 'z', metaKey: true })).toBe(true);
    expect(runtime.actions.keyDown({ key: 'y', metaKey: true })).toBe(true);
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(runtime.renderState.annotationHistory.annotations).toHaveLength(1);
    expect(runtime.renderState.previewImageBase64).toBe('preview-image');
  });

  it('ignores a superseded preview failure and publishes the queued revision', async () => {
    const supersededPreview = deferred<string>();
    const platform = createPlatform({
      session: createSession({ id: 'session-preview-revision' }),
    });
    platform.commands.renderCaptureOutput
      .mockImplementationOnce(() => supersededPreview.promise)
      .mockResolvedValueOnce('latest-preview');
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot',
      'session-preview-revision',
    );
    const first = runtime.actions.renderSelectionPreview(selection, []);
    await vi.waitFor(() =>
      expect(platform.commands.renderCaptureOutput).toHaveBeenCalledTimes(1),
    );
    const finalAnnotations = [
      {
        type: 'rectangle' as const,
        rect: { x: 4, y: 5, width: 30, height: 20 },
        color: [255, 77, 79, 255] as [number, number, number, number],
        stroke_width: 2,
        filled: false,
      },
    ];
    const latest = runtime.actions.renderSelectionPreview(
      selection,
      finalAnnotations,
    );

    supersededPreview.reject(new Error('superseded failure'));
    await Promise.all([first, latest]);

    expect(runtime.renderState).toMatchObject({
      status: 'preview',
      previewImageBase64: 'latest-preview',
      isRenderingOutput: false,
      error: null,
    });
    expect(platform.commands.renderCaptureOutput).toHaveBeenLastCalledWith({
      sessionId: 'session-preview-revision',
      rect: selection,
      annotations: finalAnnotations,
    });
  });

  it('detaches an unresolved preview scheduler when a replacement session starts', async () => {
    const oldPreview = deferred<string>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.commands.renderCaptureOutput
      .mockImplementationOnce(() => oldPreview.promise)
      .mockResolvedValueOnce('replacement-preview');
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-preview-old');
    const oldRendering = runtime.actions.renderSelectionPreview(selection);
    await vi.waitFor(() =>
      expect(platform.commands.renderCaptureOutput).toHaveBeenCalledTimes(1),
    );

    await runtime.actions.startSession('screenshot', 'session-preview-new');
    const replacementRendering =
      runtime.actions.renderSelectionPreview(selection);

    try {
      await vi.waitFor(() =>
        expect(platform.commands.renderCaptureOutput).toHaveBeenCalledTimes(2),
      );
      await replacementRendering;
      expect(runtime.renderState).toMatchObject({
        sessionId: 'session-preview-new',
        status: 'preview',
        previewImageBase64: 'replacement-preview',
        error: null,
      });
    } finally {
      oldPreview.reject(new Error('late old preview failure'));
      await Promise.all([oldRendering, replacementRendering]);
    }

    expect(runtime.renderState).toMatchObject({
      sessionId: 'session-preview-new',
      status: 'preview',
      previewImageBase64: 'replacement-preview',
      error: null,
    });
  });

  it('detaches an unresolved preview scheduler when preview state resets', async () => {
    const oldPreview = deferred<string>();
    const platform = createPlatform({
      session: createSession({ id: 'session-preview-reset-owner' }),
    });
    platform.commands.renderCaptureOutput
      .mockImplementationOnce(() => oldPreview.promise)
      .mockResolvedValueOnce('reset-owner-preview');
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot',
      'session-preview-reset-owner',
    );
    const oldRendering = runtime.actions.renderSelectionPreview(selection);
    await vi.waitFor(() =>
      expect(platform.commands.renderCaptureOutput).toHaveBeenCalledTimes(1),
    );

    runtime.actions.resetPreview();
    const resetRendering = runtime.actions.renderSelectionPreview(selection);
    await vi.waitFor(() =>
      expect(platform.commands.renderCaptureOutput).toHaveBeenCalledTimes(2),
    );
    await resetRendering;
    expect(runtime.renderState.previewImageBase64).toBe('reset-owner-preview');

    oldPreview.reject(new Error('late reset-owner failure'));
    await oldRendering;
    expect(runtime.renderState).toMatchObject({
      status: 'preview',
      previewImageBase64: 'reset-owner-preview',
      error: null,
    });
  });

  it('executes copy once with current annotations while a preview is pending', async () => {
    const pendingPreview = deferred<string>();
    const platform = createPlatform({
      session: createSession({ id: 'session-copy-pending-preview' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot',
      'session-copy-pending-preview',
    );
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 70, y: 80 }, source: 'root' });
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );

    runtime.actions.selectMoveTool();
    runtime.actions.pointerDown({ point: { x: 45, y: 55 }, source: 'preview' });
    platform.commands.renderCaptureOutput.mockReset();
    platform.commands.renderCaptureOutput.mockImplementationOnce(
      () => pendingPreview.promise,
    );
    runtime.actions.applySelectedAnnotationStyle(
      { color: [24, 144, 255, 255], strokeWidth: 4, filled: true },
      24,
    );
    const rendering = runtime.actions.renderSelectionPreview(selection);
    await vi.waitFor(() =>
      expect(platform.commands.renderCaptureOutput).toHaveBeenCalledTimes(1),
    );

    await runtime.actions.completePreviewSelection('copy', selection);

    expect(platform.commands.outputCapture).toHaveBeenCalledTimes(1);
    expect(platform.commands.outputCapture).toHaveBeenCalledWith({
      sessionId: 'session-copy-pending-preview',
      rect: selection,
      annotations: [
        {
          type: 'rectangle',
          rect: { x: 10, y: 10, width: 40, height: 40 },
          color: [24, 144, 255, 255],
          stroke_width: 4,
          filled: true,
        },
      ],
      action: { type: 'copy' },
    });

    pendingPreview.resolve('stale-preview');
    await rendering;
    await vi.waitFor(() => expect(runtime.renderState.status).toBe('idle'));
  });

  it('scopes terminal output exclusion to the current session owner', async () => {
    const oldOutput = deferred<void>();
    const currentOutput = deferred<void>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.commands.outputCapture
      .mockImplementationOnce(() => oldOutput.promise)
      .mockImplementationOnce(() => currentOutput.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-output-old');
    await runtime.actions.renderSelectionPreview(selection);
    const oldCompletion = runtime.actions.completePreviewSelection(
      'copy',
      selection,
    );
    await vi.waitFor(() =>
      expect(platform.commands.outputCapture).toHaveBeenCalledTimes(1),
    );

    await runtime.actions.startSession('screenshot', 'session-output-new');
    await runtime.actions.renderSelectionPreview(selection);
    const currentCompletion = runtime.actions.completePreviewSelection(
      'copy',
      selection,
    );

    try {
      await vi.waitFor(() =>
        expect(platform.commands.outputCapture).toHaveBeenCalledTimes(2),
      );
      oldOutput.resolve();
      await oldCompletion;

      await runtime.actions.completePreviewSelection('copy', selection);
      expect(platform.commands.outputCapture).toHaveBeenCalledTimes(2);
    } finally {
      oldOutput.resolve();
      currentOutput.resolve();
      await Promise.all([oldCompletion, currentCompletion]);
    }
  });

  it('scopes terminal output exclusion to the reset preview generation', async () => {
    const oldOutput = deferred<void>();
    const resetOutput = deferred<void>();
    const platform = createPlatform({
      session: createSession({ id: 'session-output-reset-owner' }),
    });
    platform.commands.outputCapture
      .mockImplementationOnce(() => oldOutput.promise)
      .mockImplementationOnce(() => resetOutput.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot',
      'session-output-reset-owner',
    );
    await runtime.actions.renderSelectionPreview(selection);
    const oldCompletion = runtime.actions.completePreviewSelection(
      'copy',
      selection,
    );
    await vi.waitFor(() =>
      expect(platform.commands.outputCapture).toHaveBeenCalledTimes(1),
    );

    runtime.actions.resetPreview();
    await runtime.actions.renderSelectionPreview(selection);
    const resetCompletion = runtime.actions.completePreviewSelection(
      'copy',
      selection,
    );
    await vi.waitFor(() =>
      expect(platform.commands.outputCapture).toHaveBeenCalledTimes(2),
    );

    oldOutput.resolve();
    await oldCompletion;
    await runtime.actions.completePreviewSelection('copy', selection);
    expect(platform.commands.outputCapture).toHaveBeenCalledTimes(2);

    resetOutput.resolve();
    await resetCompletion;
  });

  it('applies rapid selected styles directly to canvas-backed history', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-style-latest' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-style-latest');
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 70, y: 80 }, source: 'root' });
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );
    runtime.actions.selectMoveTool();
    runtime.actions.pointerDown({ point: { x: 45, y: 55 }, source: 'preview' });

    platform.commands.renderCaptureOutput.mockReset();
    runtime.actions.applySelectedAnnotationStyle(
      { color: [24, 144, 255, 255], strokeWidth: 3, filled: false },
      24,
    );
    runtime.actions.applySelectedAnnotationStyle(
      { color: [40, 167, 69, 255], strokeWidth: 6, filled: true },
      24,
    );

    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(runtime.renderState.annotationHistory.annotations[0]).toMatchObject({
      color: [40, 167, 69, 255],
      stroke_width: 6,
      filled: true,
    });
    expect(runtime.renderState.previewImageBase64).toBe('preview-image');
  });

  it('restores canvas text immediately when text editing is discarded', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-text-discard-latest' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot',
      'session-text-discard-latest',
    );
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.toggleAnnotationTool('text');
    runtime.actions.pointerDown({ point: { x: 50, y: 60 }, source: 'preview' });
    runtime.actions.updateTextDraftText('restored text');
    runtime.actions.commitTextDraft();
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );
    runtime.actions.selectMoveTool();

    platform.commands.renderCaptureOutput.mockClear();
    runtime.actions.pointerDown({
      point: { x: 50, y: 60 },
      detail: 2,
      shiftKey: true,
      source: 'preview',
    });
    runtime.actions.discardTextDraft();

    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(runtime.renderState.previewImageBase64).toBe('preview-image');
    expect(runtime.renderState.textDraft).toBeNull();
    expect(runtime.renderState.annotationHistory.annotations[0]).toMatchObject({
      type: 'text',
      text: 'restored text',
    });
  });

  it('owns selection move and resize edit transactions', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-selection' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot',
      'session-editor-selection',
    );
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.pointerDown({
      point: { x: 40, y: 50 },
      source: 'preview',
    });
    runtime.actions.pointerMove({ point: { x: 55, y: 65 }, source: 'root' });
    await runtime.actions.pointerUp({
      point: { x: 55, y: 65 },
      source: 'root',
    });

    expect(runtime.renderState.selection).toEqual({
      x: 35,
      y: 45,
      width: 120,
      height: 80,
    });
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );

    expect(
      runtime.actions.resizePointerDown('se', {
        point: { x: 155, y: 125 },
        source: 'preview',
      }),
    ).toBe(true);
    runtime.actions.pointerMove({ point: { x: 175, y: 140 }, source: 'root' });
    await runtime.actions.pointerUp({
      point: { x: 175, y: 140 },
      source: 'root',
    });

    expect(runtime.renderState.selection).toEqual({
      x: 35,
      y: 45,
      width: 140,
      height: 95,
    });
  });

  it('ignores a blank root click after the selection enters preview editing', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-blank-click' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot',
      'session-editor-blank-click',
    );
    await runtime.actions.renderSelectionPreview(selection);

    expect(
      runtime.actions.pointerDown({
        point: { x: 5, y: 5 },
        source: 'root',
      }),
    ).toBe(false);
    expect(runtime.renderState.status).toBe('preview');
    expect(runtime.renderState.selection).toEqual(selection);
    expect(runtime.renderState.startPoint).toBeNull();
  });

  it('owns delete, erase, clear, and preview failure state', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-delete' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-editor-delete');
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 70, y: 80 }, source: 'root' });
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );

    runtime.actions.selectMoveTool();
    runtime.actions.pointerDown({ point: { x: 45, y: 55 }, source: 'preview' });
    expect(runtime.actions.keyDown({ key: 'Delete' })).toBe(true);
    expect(runtime.renderState.annotationHistory.annotations).toEqual([]);

    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 70, y: 80 }, source: 'root' });
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );
    runtime.actions.toggleAnnotationTool('eraser');
    runtime.actions.pointerDown({ point: { x: 45, y: 55 }, source: 'preview' });
    await runtime.actions.pointerUp({ point: { x: 45, y: 55 }, source: 'root' });
    expect(runtime.renderState.annotationHistory.annotations).toMatchObject([
      { type: 'rectangle' },
      { type: 'eraser', points: [{ x: 25, y: 25 }] },
    ]);
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );

    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'root' });
    await runtime.actions.pointerUp({ point: { x: 70, y: 80 }, source: 'root' });
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );
    expect(
      runtime.actions.keyDown({ key: 'z', metaKey: true, shiftKey: true }),
    ).toBe(true);
    expect(runtime.renderState.annotationHistory.annotations).toEqual([]);
    await vi.waitFor(() =>
      expect(runtime.renderState.isRenderingOutput).toBe(false),
    );

    platform.commands.renderCaptureOutput.mockRejectedValueOnce(
      new Error('editor preview failed'),
    );
    await runtime.actions.renderSelectionPreview(selection);
    expect(runtime.renderState).toMatchObject({
      status: 'error',
      selection,
      isRenderingOutput: false,
      error: 'editor preview failed',
    });
  });

  it('nudges an active selecting draft through the runtime keyboard path', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-draft-nudge');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 100, y: 80 });

    expect(runtime.actions.keyDown({ key: 'd' })).toBe(true);

    expect(runtime.renderState).toMatchObject({
      startPoint: { x: 20, y: 30 },
      cursorPoint: { x: 101, y: 80 },
      selection: { x: 20, y: 30, width: 81, height: 50 },
    });
  });

  it('nudges a floating selecting cursor and refreshes its hover candidate', async () => {
    const platform = createPlatform({
      session: createSession({
        candidates: [
          { id: 'window-1', kind: 'window', rect: selection, priority: 10 },
        ],
      }),
    });
    platform.commands.currentCaptureCursorPosition.mockResolvedValue({
      x: 39,
      y: 50,
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-cursor-nudge');

    expect(runtime.actions.keyDown({ key: 'd' })).toBe(true);

    expect(runtime.renderState).toMatchObject({
      cursorPoint: { x: 40, y: 50 },
      hoverSelection: selection,
    });
  });

  it('cycles overlapping selecting candidates through the runtime keyboard path', async () => {
    const higher = { x: 20, y: 30, width: 120, height: 80 };
    const lower = { x: 10, y: 20, width: 160, height: 120 };
    const platform = createPlatform({
      session: createSession({
        candidates: [
          { id: 'higher', kind: 'window', rect: higher, priority: 20 },
          { id: 'lower', kind: 'window', rect: lower, priority: 10 },
        ],
      }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-cycle');
    runtime.actions.pointerMove({ x: 40, y: 50 });
    expect(runtime.renderState.hoverSelection).toEqual(higher);

    expect(runtime.actions.keyDown({ key: 'Tab' })).toBe(true);

    expect(runtime.renderState.hoverSelection).toEqual(lower);
  });

  it('commits the keyboard-adjusted draft endpoint on pointer release', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-nudge-release');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 100, y: 80 });
    expect(runtime.actions.keyDown({ key: 'd' })).toBe(true);

    await runtime.actions.pointerUp({ x: 100, y: 80 });

    expect(platform.commands.renderCaptureOutput).toHaveBeenCalledWith({
      sessionId: 'session-1',
      rect: { x: 20, y: 30, width: 81, height: 50 },
      annotations: [],
    });
  });

  it('uses the pointer-up coordinate when the draft was not keyboard-adjusted', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-pointer-release');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 100, y: 80 });

    await runtime.actions.pointerUp({ x: 120, y: 90 });

    expect(platform.commands.renderCaptureOutput).toHaveBeenCalledWith({
      sessionId: 'session-1',
      rect: { x: 20, y: 30, width: 100, height: 60 },
      annotations: [],
    });
  });

  it('uses runtime-owned polled hover state for keyboard and native copy', async () => {
    const candidateB = { x: 200, y: 40, width: 80, height: 70 };
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (id) => createSession({ id }));
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot-copy', 'session-poll-enter');
    runtime.actions.updatePolledCursor({ x: 220, y: 60 });
    runtime.actions.updatePolledHover(candidateB);
    expect(runtime.actions.keyDown({ key: 'Enter' })).toBe(true);
    await vi.waitFor(() => expect(platform.commands.outputCapture).toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: 'session-poll-enter', rect: candidateB }),
    ));

    await runtime.actions.startSession('screenshot-copy', 'session-poll-native');
    runtime.actions.updatePolledCursor({ x: 220, y: 60 });
    runtime.actions.updatePolledHover(candidateB);
    await runtime.actions.connectHost();
    const copyCalls = platform.onCopyRequested.mock.calls;
    await copyCalls[copyCalls.length - 1]?.[0]?.();
    expect(platform.commands.outputCapture).toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: 'session-poll-native', rect: candidateB }),
    );
  });

  it('does not notify subscribers for unchanged polled cursor or hover state', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-poll-dedupe');
    const listener = vi.fn();
    runtime.subscribe(listener);

    runtime.actions.updatePolledCursor({ x: 220, y: 60 });
    runtime.actions.updatePolledCursor({ x: 220, y: 60 });
    runtime.actions.updatePolledHover({ x: 200, y: 40, width: 80, height: 70 });
    runtime.actions.updatePolledHover({ x: 200, y: 40, width: 80, height: 70 });

    expect(listener).toHaveBeenCalledTimes(2);
  });

  it('cancels a selecting draft on Escape without delegating to the editor', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-draft-escape');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 100, y: 80 });

    expect(runtime.actions.keyDown({ key: 'Escape' })).toBe(true);
    await vi.waitFor(() => expect(runtime.renderState.status).toBe('idle'));
    expect(platform.dismiss).toHaveBeenCalledOnce();
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith('session-1');
  });

  it('keeps the newest session authoritative when an older start resolves later', async () => {
    const oldSession = deferred<ReturnType<typeof createSession>>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation((sessionId) =>
      sessionId === 'session-old'
        ? oldSession.promise
        : Promise.resolve(createSession({ id: 'session-new' })),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });

    const oldStart = runtime.actions.startSession('screenshot', 'session-old');
    await runtime.actions.startSession('screenshot', 'session-new');
    oldSession.resolve(createSession({ id: 'session-old' }));
    await oldStart;

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-new',
      error: null,
    });
    expect(platform.commands.currentCaptureCursorPosition).toHaveBeenCalledTimes(
      1,
    );
    expect(platform.commands.currentCaptureCursorPosition).toHaveBeenCalledWith(
      'session-new',
    );
  });

  it('ignores an older start rejection after a newer session loads', async () => {
    const oldSession = deferred<ReturnType<typeof createSession>>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation((sessionId) =>
      sessionId === 'session-old'
        ? oldSession.promise
        : Promise.resolve(createSession({ id: 'session-new' })),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });

    const oldStart = runtime.actions.startSession('screenshot', 'session-old');
    await runtime.actions.startSession('screenshot', 'session-new');
    oldSession.reject(new Error('old load failed'));
    await oldStart;

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-new',
      error: null,
    });
  });

  it('ignores an older cursor lookup after a newer session loads', async () => {
    const oldCursor = deferred<{ x: number; y: number } | null>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.commands.currentCaptureCursorPosition.mockImplementation(
      (sessionId) =>
        sessionId === 'session-old' ? oldCursor.promise : Promise.resolve(null),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });

    const oldStart = runtime.actions.startSession('screenshot', 'session-old');
    await vi.waitFor(() => {
      expect(
        platform.commands.currentCaptureCursorPosition,
      ).toHaveBeenCalledWith('session-old');
    });
    await runtime.actions.startSession('screenshot', 'session-new');
    oldCursor.resolve({ x: 40, y: 50 });
    await oldStart;

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-new',
      cursorPoint: null,
      error: null,
    });
  });

  it('completes a pointer selection through effect interpretation and execution', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-pointer' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot-copy', 'session-pointer');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    await runtime.actions.pointerUp({ x: 140, y: 110 });

    expect(platform.commands.getCaptureSession).toHaveBeenCalledWith(
      'session-pointer',
    );
    expect(platform.commands.outputCapture).toHaveBeenCalledWith({
      sessionId: 'session-pointer',
      rect: selection,
      annotations: [],
      action: { type: 'copy' },
    });
    await vi.waitFor(() => expect(platform.dismiss).toHaveBeenCalledTimes(1));
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-pointer',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'idle',
      selection: null,
      isRenderingOutput: false,
      error: null,
    });
  });

  it('confirms a keyboard candidate through the same completion effects', async () => {
    const platform = createPlatform({
      session: createSession({
        id: 'session-confirm',
        candidates: [
          {
            id: 'window-1',
            kind: 'window',
            rect: selection,
            priority: 10,
          },
        ],
      }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot-copy', 'session-confirm');
    runtime.actions.pointerMove({ x: 40, y: 50 });
    expect(runtime.actions.keyDown({ key: 'Enter' })).toBe(true);

    await vi.waitFor(() =>
      expect(platform.commands.outputCapture).toHaveBeenCalledWith(
        expect.objectContaining({
          sessionId: 'session-confirm',
          rect: selection,
          action: { type: 'copy' },
        }),
      ),
    );
    await vi.waitFor(() => expect(platform.dismiss).toHaveBeenCalledTimes(1));
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-confirm',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'idle',
      selection: null,
      isRenderingOutput: false,
      error: null,
    });
  });

  it('cancels from Escape through the keyboard runtime action', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-cancel' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-cancel');
    await runtime.actions.keyDown({ key: 'Escape' });

    expect(platform.commands.outputCapture).not.toHaveBeenCalled();
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-cancel',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'idle',
      selection: null,
      error: null,
    });
  });

  it('clears the draft overlay before presenting a confirmed manual selection', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-clear-draft-overlay' }),
    });
    const resetInteraction = vi.fn();
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      host: {
        resetInteraction,
        resetSession: vi.fn(),
        prepareSurface: vi.fn(),
      },
    });

    await runtime.actions.startSession(
      'screenshot',
      'session-clear-draft-overlay',
    );
    resetInteraction.mockClear();
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    await runtime.actions.pointerUp({ x: 140, y: 110 });

    expect(resetInteraction).toHaveBeenCalledOnce();
    expect(runtime.renderState.status).toBe('preview');
  });

  it('rolls back failed host hydration so the active session can retry cleanly', async () => {
    const initialSession = createSession({ id: 'session-hydration' });
    const hydratedSession = createSession({
      id: 'session-hydration',
      monitors: [createMonitor({ image_base64: 'hydrated-pixels' })],
    });
    const platform = createPlatform({ session: initialSession });
    platform.commands.hydrateCaptureSessionSnapshots
      .mockRejectedValueOnce(new Error('hydrate failed'))
      .mockResolvedValueOnce(hydratedSession);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-hydration');

    await expect(runtime.actions.hydrateSnapshots()).rejects.toThrow(
      'hydrate failed',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-hydration',
      hasHydratedPixelSource: false,
      error: null,
    });
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();
    expect(platform.dismiss).not.toHaveBeenCalled();

    await expect(runtime.actions.hydrateSnapshots()).resolves.toBeUndefined();

    expect(platform.commands.hydrateCaptureSessionSnapshots).toHaveBeenCalledTimes(
      2,
    );
    expect(
      platform.commands.hydrateCaptureSessionSnapshots,
    ).toHaveBeenNthCalledWith(1, 'session-hydration');
    expect(
      platform.commands.hydrateCaptureSessionSnapshots,
    ).toHaveBeenNthCalledWith(2, 'session-hydration');
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-hydration',
      hasHydratedPixelSource: true,
      error: null,
    });
  });

  it('keeps the preview open when the save dialog is cancelled', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-save-cancelled' }),
    });
    platform.commands.defaultCaptureSavePath.mockResolvedValueOnce(null);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-save-cancelled');
    await runtime.actions.renderSelectionPreview(selection);
    platform.commands.outputCapture.mockClear();
    await runtime.actions.completePreviewSelection('save', selection);

    expect(platform.commands.outputCapture).not.toHaveBeenCalled();
    expect(runtime.renderState).toMatchObject({
      status: 'preview',
      sessionId: 'session-save-cancelled',
      isRenderingOutput: false,
      error: null,
    });
  });

  it('does not let an old pending output dismiss or reset a replacement session', async () => {
    const output = deferred<void>();
    const platform = createPlatform({
      session: createSession({ id: 'session-old' }),
    });
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.commands.outputCapture.mockImplementation(() => output.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot-copy', 'session-old');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    const oldCompletion = runtime.actions.pointerUp({ x: 140, y: 110 });
    await runtime.actions.startSession('screenshot', 'session-new');
    output.resolve();
    await oldCompletion;

    expect(platform.dismiss).not.toHaveBeenCalled();
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-old',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-new',
      isRenderingOutput: false,
      error: null,
    });
  });

  it('cancels an old native session when its pending output rejects after replacement', async () => {
    const output = deferred<void>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.commands.outputCapture.mockImplementation(() => output.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot-copy', 'session-old');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    const oldCompletion = runtime.actions.pointerUp({ x: 140, y: 110 });
    await runtime.actions.startSession('screenshot', 'session-new');
    output.reject(new Error('old output failed'));
    await oldCompletion;

    expect(platform.dismiss).not.toHaveBeenCalled();
    await vi.waitFor(() =>
      expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1),
    );
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-old',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-new',
      isRenderingOutput: false,
      error: null,
    });
  });

  it('does not apply a stale manual preview after a replacement session starts', async () => {
    const preview = deferred<string>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.commands.renderCaptureOutput.mockImplementation(() => preview.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-old');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    const oldPreview = runtime.actions.pointerUp({ x: 140, y: 110 });
    await runtime.actions.startSession('screenshot', 'session-new');
    preview.resolve('stale-preview');
    await oldPreview;

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-old',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-new',
      previewImageBase64: null,
      isRenderingOutput: false,
      error: null,
    });
  });

  it('does not open an OCR result for a stale completion', async () => {
    const ocr = deferred<{ text: string; confidence: null }>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.commands.runCaptureOcr.mockImplementation(() => ocr.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot-ocr', 'session-old');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    const oldCompletion = runtime.actions.pointerUp({ x: 140, y: 110 });
    await runtime.actions.startSession('screenshot', 'session-new');
    ocr.resolve({ text: 'stale text', confidence: null });
    await oldCompletion;

    expect(platform.commands.openCaptureOcrResultWindow).not.toHaveBeenCalled();
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-old',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-new',
      isRenderingOutput: false,
      error: null,
    });
  });

  it('does not let an old pending dismiss reset a replacement session', async () => {
    const dismiss = deferred<void>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.dismiss.mockImplementation(() => dismiss.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-old');
    expect(runtime.actions.keyDown({ key: 'Escape' })).toBe(true);
    await runtime.actions.startSession('screenshot', 'session-new');
    dismiss.resolve();
    await vi.waitFor(() =>
      expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
        'session-old',
      ),
    );

    expect(platform.dismiss).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-old',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-new',
      isRenderingOutput: false,
      error: null,
    });
  });

  it('cancels an old native session when its pending dismiss rejects after replacement', async () => {
    const dismiss = deferred<void>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.dismiss.mockImplementation(() => dismiss.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-old');
    expect(runtime.actions.keyDown({ key: 'Escape' })).toBe(true);
    await runtime.actions.startSession('screenshot', 'session-new');
    dismiss.reject(new Error('old dismiss failed'));
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
    await vi.waitFor(() =>
      expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1),
    );
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-old',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-new',
      isRenderingOutput: false,
      error: null,
    });
  });

  it('renders a manual screenshot selection into preview without finishing the session', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-preview' }),
    });
    platform.commands.renderCaptureOutput.mockResolvedValue('preview-image');
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-preview');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    await runtime.actions.pointerUp({ x: 140, y: 110 });

    expect(platform.commands.renderCaptureOutput).toHaveBeenCalledWith({
      sessionId: 'session-preview',
      rect: selection,
      annotations: [],
    });
    expect(platform.dismiss).not.toHaveBeenCalled();
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();
    expect(runtime.renderState).toMatchObject({
      status: 'preview',
      sessionId: 'session-preview',
      selection,
      previewImageBase64: 'preview-image',
      isRenderingOutput: false,
      error: null,
    });
  });

  it.each([
    {
      mode: 'screenshot-ocr' as const,
      expectedText: 'recognized text',
      assertResult: (platform: ReturnType<typeof createPlatform>) => {
        expect(platform.commands.renderCaptureOutput).toHaveBeenCalledWith({
          sessionId: 'session-ocr',
          rect: selection,
          annotations: [],
        });
        expect(platform.commands.openCaptureOcrResultWindow).toHaveBeenCalledWith(
          'recognized text',
          'preview-image',
        );
      },
    },
    {
      mode: 'silent-screenshot-ocr' as const,
      expectedText: 'recognized text',
      assertResult: (platform: ReturnType<typeof createPlatform>) => {
        expect(platform.commands.copyTextToClipboard).toHaveBeenCalledWith(
          'recognized text',
        );
      },
    },
    {
      mode: 'screenshot-translate' as const,
      expectedText: 'recognized text',
      assertResult: (platform: ReturnType<typeof createPlatform>) => {
        expect(
          platform.commands.openCaptureTranslationResultWindow,
        ).toHaveBeenCalledWith('recognized text');
      },
    },
  ])('executes $mode completion effects before finishing', async ({
    assertResult,
    mode,
  }) => {
    const platform = createPlatform({
      session: createSession({ id: 'session-ocr' }),
    });
    platform.commands.runCaptureOcr.mockResolvedValue({
      text: ' recognized text ',
      confidence: null,
    });
    platform.commands.renderCaptureOutput.mockResolvedValue('preview-image');
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(mode, 'session-ocr');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    await runtime.actions.pointerUp({ x: 140, y: 110 });

    expect(platform.commands.runCaptureOcr).toHaveBeenCalledWith(
      'session-ocr',
      selection,
    );
    assertResult(platform);
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-ocr',
    );
    expect(runtime.renderState).toMatchObject({
      status: 'idle',
      sessionId: null,
      isRenderingOutput: false,
      error: null,
    });
  });

  it('opens the OCR result even when automatic clipboard copy fails', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-ocr' }),
    });
    platform.commands.runCaptureOcr.mockResolvedValue({
      text: 'recognized text',
      confidence: null,
    });
    platform.commands.copyTextToClipboard.mockRejectedValue(
      new Error('clipboard unavailable'),
    );
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      ocrPreferences: () => ({
        recognitionLanguage: 'auto',
        autoCopy: true,
        preserveFormatting: true,
        removeChineseSpaces: true,
        showConfidence: false,
      }),
    });

    await runtime.actions.startSession('screenshot-ocr', 'session-ocr');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    await runtime.actions.pointerUp({ x: 140, y: 110 });

    expect(platform.commands.openCaptureOcrResultWindow).toHaveBeenCalledWith(
      'recognized text',
      'preview-image',
    );
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
    consoleError.mockRestore();
  });
});

function createPlatform({
  session = createSession(),
}: {
  session?: ReturnType<typeof createSession>;
} = {}) {
  return {
    commands: {
      createCaptureSession: vi.fn(async () => session),
      getCaptureSession: vi.fn<
        CaptureWorkspacePlatformRuntime['commands']['getCaptureSession']
      >(async () => session),
      hydrateCaptureSessionSnapshots: vi.fn<
        CaptureWorkspacePlatformRuntime['commands']['hydrateCaptureSessionSnapshots']
      >(async () => session),
      logCaptureFrontendPerf: vi.fn(async () => undefined),
      currentCaptureCursorPosition: vi.fn<
        CaptureWorkspacePlatformRuntime['commands']['currentCaptureCursorPosition']
      >(async () => null),
      cancelCaptureSession: vi.fn<
        CaptureWorkspacePlatformRuntime['commands']['cancelCaptureSession']
      >(async () => undefined),
      restoreCaptureSnapshotWindowsForSession: vi.fn(async () => undefined),
      renderCaptureOutput: vi.fn(async () => 'preview-image'),
      defaultCaptureSavePath: vi.fn<
        CaptureWorkspacePlatformRuntime['commands']['defaultCaptureSavePath']
      >(async () => '/captures/capture.png'),
      quickCaptureSavePath: vi.fn(async () => '/captures/quick.png'),
      outputCapture: vi.fn<
        CaptureWorkspacePlatformRuntime['commands']['outputCapture']
      >(async () => undefined),
      runCaptureOcr: vi.fn(async () => ({ text: '', confidence: null })),
      openCaptureOcrResultWindow: vi.fn(async () => undefined),
      openCaptureTranslationResultWindow: vi.fn(async () => undefined),
      copyTextToClipboard: vi.fn(async () => undefined),
    },
    clipboard: {
      copyText: vi.fn(async () => undefined),
    },
    onCancelRequested: vi.fn<
      CaptureWorkspacePlatformRuntime['onCancelRequested']
    >(async () => () => undefined),
    onCopyRequested: vi.fn<
      CaptureWorkspacePlatformRuntime['onCopyRequested']
    >(async () => () => undefined),
    onSaveRequested: vi.fn<
      CaptureWorkspacePlatformRuntime['onSaveRequested']
    >(async () => () => undefined),
    onUndoRequested: vi.fn<
      CaptureWorkspacePlatformRuntime['onUndoRequested']
    >(async () => () => undefined),
    onRedoRequested: vi.fn<
      CaptureWorkspacePlatformRuntime['onRedoRequested']
    >(async () => () => undefined),
    onHotkeyTriggered: vi.fn<
      CaptureWorkspacePlatformRuntime['onHotkeyTriggered']
    >(async () => () => undefined),
    prepareForReveal: vi.fn(async () => undefined),
    reveal: vi.fn(async () => undefined),
    dismiss: vi.fn<CaptureWorkspacePlatformRuntime['dismiss']>(async () =>
      undefined,
    ),
  } satisfies CaptureWorkspacePlatformRuntime;
}

function createSession(
  overrides: Partial<{
    id: string;
    monitors: ReturnType<typeof createMonitor>[];
    candidates: Array<{
      id: string;
      kind: 'window';
      rect: typeof selection;
      priority: number;
    }>;
    captured_cursor: {
      logical_position: { x: number; y: number };
      hotspot: { x: number; y: number };
      image_width: number;
      image_height: number;
      scale_factor: number;
      image_base64: string;
    } | null;
  }> = {},
) {
  return {
    id: 'session-1',
    monitors: [createMonitor()],
    candidates: [],
    captured_cursor: null,
    ...overrides,
  };
}

function createMonitor(
  overrides: Partial<{
    image_base64: string;
  }> = {},
) {
  return {
    id: 'monitor-1',
    logical_bounds: { x: 0, y: 0, width: 500, height: 300 },
    physical_bounds: { x: 0, y: 0, width: 1000, height: 600 },
    scale_factor: 2,
    image_base64: '',
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });

  return { promise, resolve, reject };
}
