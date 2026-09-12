import { describe, expect, it, vi } from 'vitest';

import type { CaptureWorkspacePorts } from './ports';
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
  it.each(['cancel', 'dispose'] as const)('cleans an unreadable native session on %s', async (exit) => {
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockRejectedValue(new Error('Session load failed'));
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'unreadable-session');

    if (exit === 'cancel') await runtime.actions.cancelSession();
    else runtime.dispose();

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledExactlyOnceWith('unreadable-session');
    expect(runtime.renderState.status).toBe('idle');
  });

  it('reveals a session load error even when no session id was adopted', async () => {
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockRejectedValue(new Error('Session load failed'));
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'missing-session');

    await runtime.actions.updateHostReadiness(false);

    expect(runtime.renderState.error).toBe('Session load failed');
    expect(platform.window.reveal).toHaveBeenCalledOnce();
    expect(platform.window.prepareForReveal).toHaveBeenCalledWith(null);
    expect(platform.window.reveal).toHaveBeenCalledWith(null);
  });

  it('cancels the restored session and its unreadable replacement on exit', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-1');
    platform.commands.getCaptureSession.mockRejectedValue(new Error('Session load failed'));
    await runtime.actions.startSession('screenshot', 'unreadable-replacement');

    await runtime.actions.cancelSession();

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith('session-1');
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith('unreadable-replacement');
    expect(runtime.renderState.status).toBe('idle');
  });

  it.each([false, true])('releases native resources when hiding fails (loaded: %s)', async (loaded) => {
    const platform = createPlatform();
    if (!loaded) {
      platform.commands.getCaptureSession.mockRejectedValue(new Error('Session load failed'));
    }
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-1');
    platform.window.hide.mockRejectedValueOnce(new Error('Hide failed'));

    await runtime.actions.cancelSession();

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledExactlyOnceWith('session-1');
    expect(runtime.renderState.error).toBe('Hide failed');
  });

  it('waits for every frozen monitor image before revealing the selection surface', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-1');

    await runtime.actions.updateHostReadiness(false);
    expect(platform.window.reveal).not.toHaveBeenCalled();

    await runtime.actions.updateHostReadiness(true);
    expect(platform.window.reveal).toHaveBeenCalledOnce();
  });

  it('previews the original frozen monitors without recapturing or encoding', async () => {
    const session = createSession({ monitors: [createMonitor({ image_base64: 'frozen-foreground' })] });
    const platform = createPlatform({ session });
    const recapture = vi.fn();
    Object.assign(platform.commands, { refreshCaptureSessionSnapshots: recapture });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', session.id);
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    await runtime.actions.pointerUp({ x: 140, y: 110 });
    expect(runtime.renderState.session?.monitors).toEqual(session.monitors);
    expect(runtime.renderState).toMatchObject({ status: 'preview', selection, isRenderingOutput: false });
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(recapture).not.toHaveBeenCalled();
  });

  it('does not notify subscribers for duplicate magnifier color samples', () => {
    const runtime = createCaptureWorkspaceRuntime({ platform: createPlatform() });
    const listener = vi.fn();
    runtime.subscribe(listener);

    runtime.actions.updateCursorColor({
      hex: '#0A141E',
      red: 10,
      green: 20,
      blue: 30,
    });
    runtime.actions.updateCursorColor({
      hex: '#0A141E',
      red: 10,
      green: 20,
      blue: 30,
    });

    expect(listener).toHaveBeenCalledOnce();
  });

  it('disposes local keyboard listeners and a late launch subscription exactly once', async () => {
    const platform = createPlatform();
    const keyboardTarget = createKeyboardTarget();
    const disposeHotkey = vi.fn();
    const registration = deferred<() => void>();
    platform.events.subscribeHotkeyTriggered.mockReturnValue(registration.promise);
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      keyboard: { target: keyboardTarget.target },
    });
    const connecting = runtime.actions.connectHost();
    runtime.dispose();
    expect(keyboardTarget.listenerCount('keydown')).toBe(0);
    expect(keyboardTarget.listenerCount('keyup')).toBe(0);
    expect(keyboardTarget.listenerCount('blur')).toBe(0);
    registration.resolve(disposeHotkey);
    const disconnect = await connecting;
    disconnect();
    disconnect();
    expect(disposeHotkey).toHaveBeenCalledOnce();
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
      hasHydratedPixelSource: true,
      error: 'replacement load failed',
    });
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();

    await runtime.actions.startSession('screenshot', 'replacement-success');
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(2);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith('replacement-failed');
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

    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(2);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith('replacement-stale');
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
    platform.commands.outputCapture.mockRejectedValue(
      new Error('copy failed'),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-key-rejection');

    await runtime.actions.renderSelectionPreview(selection);
    expect(runtime.actions.keyDown({ key: 'c', metaKey: true })).toBe(true);

    await vi.waitFor(() =>
      expect(runtime.renderState).toMatchObject({
        status: 'error',
        error: 'copy failed',
      }),
    );
  });

  it('owns host subscriptions and cleans every listener up together', async () => {
    const platform = createPlatform();
    const unlistenHotkey = vi.fn();
    platform.events.subscribeHotkeyTriggered.mockResolvedValue(unlistenHotkey);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    const disconnect = await runtime.actions.connectHost();
    const launch = platform.events.subscribeHotkeyTriggered.mock.calls[0]?.[0];
    await launch?.({ mode: 'screenshot', sessionId: 'session-host' });

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      sessionId: 'session-1',
    });

    disconnect();
    expect(unlistenHotkey).toHaveBeenCalledTimes(1);
  });

  it('cleans local keyboard listeners when launch subscription fails', async () => {
    const platform = createPlatform();
    const keyboardTarget = createKeyboardTarget();
    platform.events.subscribeHotkeyTriggered.mockRejectedValue(new Error('listen failed'));
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      keyboard: { target: keyboardTarget.target },
    });
    await runtime.actions.connectHost();
    expect(runtime.renderState.error).toBe('listen failed');
    expect(keyboardTarget.listenerCount('keydown')).toBe(0);
    expect(keyboardTarget.listenerCount('keyup')).toBe(0);
    expect(keyboardTarget.listenerCount('blur')).toBe(0);
  });

  it('copies committed annotations through the local keyboard', async () => {
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

    expect(runtime.actions.keyDown({ key: 'c', metaKey: true })).toBe(false);
    expect(platform.commands.outputCapture).not.toHaveBeenCalled();
    runtime.actions.commitTextDraft();
    expect(runtime.actions.keyDown({ key: 'c', metaKey: true })).toBe(true);
    await vi.waitFor(() => expect(platform.window.hide).toHaveBeenCalledOnce());

    expect(platform.commands.outputCapture).toHaveBeenCalledWith({
      sessionId: 'session-native-copy',
      rect: selection,
      annotations: [annotation],
      action: { type: 'copy' },
    });
    expect(platform.window.hide).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-native-copy',
    );
  });

  it('saves through the local keyboard with the configured output preferences', async () => {
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

    expect(runtime.actions.keyDown({ key: 's', metaKey: true })).toBe(true);
    await vi.waitFor(() => expect(platform.window.hide).toHaveBeenCalledOnce());

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
    expect(platform.window.hide).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-native-save',
    );
  });

  it('handles local undo and redo while preview editing is active', async () => {
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

    runtime.actions.keyDown({ key: 'z', metaKey: true });
    expect(runtime.renderState.annotationHistory.annotations).toEqual([]);

    runtime.actions.keyDown({ key: 'y', metaKey: true });
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

    expect(platform.window.prepareForReveal).toHaveBeenCalledTimes(1);
    expect(platform.window.prepareForReveal).toHaveBeenCalledWith('session-reveal');
    expect(prepareSurface).toHaveBeenCalledTimes(1);
    expect(platform.window.reveal).toHaveBeenCalledTimes(1);
    expect(platform.window.reveal).toHaveBeenCalledWith('session-reveal');
  });

  it('coalesces concurrent readiness and ignores a stale reveal preparation', async () => {
    const firstPrepare = deferred<void>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (id) =>
      createSession({ id }),
    );
    platform.window.prepareForReveal
      .mockImplementationOnce(() => firstPrepare.promise.then(() => undefined))
      .mockResolvedValue(undefined);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-a');

    const first = runtime.actions.updateHostReadiness(true);
    const duplicate = runtime.actions.updateHostReadiness(true);
    expect(platform.window.prepareForReveal).toHaveBeenCalledTimes(1);

    await runtime.actions.startSession('screenshot', 'session-b');
    await runtime.actions.updateHostReadiness(true);
    expect(platform.window.reveal).toHaveBeenCalledTimes(1);

    firstPrepare.resolve();
    await Promise.all([first, duplicate]);
    expect(platform.window.reveal).toHaveBeenCalledTimes(1);
    await runtime.actions.updateHostReadiness(true);
    expect(platform.window.reveal).toHaveBeenCalledTimes(1);
  });

  it('retries reveal preparation after the current attempt fails', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-reveal-retry' }),
    });
    platform.window.prepareForReveal
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

    expect(platform.window.prepareForReveal).toHaveBeenCalledTimes(2);
    expect(platform.window.reveal).toHaveBeenCalledOnce();
  });

  it('ignores refresh without an active session', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.refreshSession();

    expect(platform.commands.createCaptureSession).not.toHaveBeenCalled();
  });

  it('uses the refreshed session for native window preparation and reveal', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-before-refresh' }),
    });
    platform.commands.createCaptureSession.mockResolvedValue(
      createSession({ id: 'session-after-refresh' }),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-before-refresh');
    await runtime.actions.updateHostReadiness(true);

    await runtime.actions.refreshSession();
    await runtime.actions.updateHostReadiness(true);

    expect(platform.window.prepareForReveal).toHaveBeenNthCalledWith(
      1,
      'session-before-refresh',
    );
    expect(platform.window.prepareForReveal).toHaveBeenNthCalledWith(
      2,
      'session-after-refresh',
    );
    expect(platform.window.reveal).toHaveBeenNthCalledWith(1, 'session-before-refresh');
    expect(platform.window.reveal).toHaveBeenNthCalledWith(2, 'session-after-refresh');
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
    platform.window.hide
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
    ).toHaveLength(2);
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

  it('confirms a recommended interface element after flushing the latest pointer move', async () => {
    const recommended = { x: 40, y: 30, width: 180, height: 120 };
    const platform = createPlatform({
      session: createSession({
        id: 'session-candidate-click',
        captured_cursor: {
          logical_position: { x: 80, y: 70 },
          hotspot: { x: 0, y: 0 },
          image_width: 16,
          image_height: 16,
          scale_factor: 2,
          image_base64: '',
        },
      }),
    });
    platform.commands.currentCaptureControlCandidate.mockResolvedValue({
      id: 'control-recommended',
      kind: 'control',
      rect: recommended,
      priority: 10_001,
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot-copy', 'session-candidate-click');
    runtime.actions.keyDown({ key: 'Tab' });
    await vi.waitFor(() =>
      expect(runtime.renderState.hoverSelection).toEqual(recommended),
    );

    runtime.actions.pointerMove({
      point: { x: 80, y: 70 },
      button: 0,
      source: 'root',
    });
    expect(runtime.renderState.hoverSelection).toEqual(recommended);
    runtime.actions.pointerDown({
      point: { x: 80, y: 70 },
      button: 0,
      detail: 1,
      source: 'root',
    });
    await runtime.actions.pointerUp({
      point: { x: 80, y: 70 },
      button: 0,
      detail: 1,
      source: 'root',
    });

    expect(platform.commands.outputCapture).toHaveBeenCalledWith(
      expect.objectContaining({
        sessionId: 'session-candidate-click',
        rect: recommended,
      }),
    );
    expect(platform.window.hide).toHaveBeenCalledOnce();
  });

  it('does not confirm a recommended interface element with a modified primary click', async () => {
    const recommended = { x: 40, y: 30, width: 180, height: 120 };
    const platform = createPlatform({
      session: createSession({
        id: 'session-candidate-modified-click',
        captured_cursor: {
          logical_position: { x: 80, y: 70 },
          hotspot: { x: 0, y: 0 },
          image_width: 16,
          image_height: 16,
          scale_factor: 2,
          image_base64: '',
        },
      }),
    });
    platform.commands.currentCaptureControlCandidate.mockResolvedValue({
      id: 'control-recommended',
      kind: 'control',
      rect: recommended,
      priority: 10_001,
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession(
      'screenshot-copy',
      'session-candidate-modified-click',
    );
    runtime.actions.keyDown({ key: 'Tab' });
    await vi.waitFor(() =>
      expect(runtime.renderState.hoverSelection).toEqual(recommended),
    );

    expect(
      runtime.actions.pointerDown({
        point: { x: 80, y: 70 },
        button: 0,
        detail: 1,
        metaKey: true,
        source: 'root',
      }),
    ).toBe(true);
    await expect(
      runtime.actions.pointerUp({
        point: { x: 80, y: 70 },
        button: 0,
        detail: 1,
        metaKey: true,
        source: 'root',
      }),
    ).resolves.toBe(true);

    expect(platform.commands.outputCapture).not.toHaveBeenCalled();
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      startPoint: null,
      selection: null,
    });
  });

  it('does not start or commit a selection with the middle button', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-middle-click');

    expect(
      runtime.actions.pointerDown({
        point: { x: 80, y: 70 },
        button: 1,
        source: 'root',
      }),
    ).toBe(false);
    await expect(
      runtime.actions.pointerUp({
        point: { x: 80, y: 70 },
        button: 1,
        source: 'root',
      }),
    ).resolves.toBe(false);

    expect(
      runtime.actions.pointerDown({
        point: { x: 20, y: 30 },
        button: 0,
        source: 'root',
      }),
    ).toBe(true);
    runtime.actions.pointerMove({
      point: { x: 80, y: 70 },
      button: 0,
      source: 'root',
    });
    await expect(
      runtime.actions.pointerUp({
        point: { x: 80, y: 70 },
        button: 1,
        source: 'root',
      }),
    ).resolves.toBe(false);

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      startPoint: { x: 20, y: 30 },
      selection: { x: 20, y: 30, width: 60, height: 40 },
    });
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(runtime.actions.pointerCancel()).toBe(true);
  });

  it('cancels a pressed recommendation without confirming it', async () => {
    const recommended = { x: 40, y: 30, width: 180, height: 120 };
    const platform = createPlatform({
      session: createSession({
        id: 'session-candidate-cancel',
        candidates: [
          {
            id: 'window-recommended',
            kind: 'window',
            rect: recommended,
            priority: 10,
          },
        ],
        captured_cursor: {
          logical_position: { x: 80, y: 70 },
          hotspot: { x: 0, y: 0 },
          image_width: 16,
          image_height: 16,
          scale_factor: 2,
          image_base64: '',
        },
      }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot-copy', 'session-candidate-cancel');

    runtime.actions.pointerDown({
      point: { x: 80, y: 70 },
      button: 0,
      detail: 1,
      source: 'root',
    });
    expect(runtime.actions.pointerCancel()).toBe(true);
    expect(
      await runtime.actions.pointerUp({
        point: { x: 80, y: 70 },
        button: 0,
        detail: 1,
        source: 'root',
      }),
    ).toBe(false);

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      startPoint: null,
      selection: null,
      hoverSelection: null,
    });
    expect(platform.commands.outputCapture).not.toHaveBeenCalled();
  });

  it('enters the preview editor when a candidate is clicked in a preview-flow mode', async () => {
    const recommended = { x: 40, y: 30, width: 180, height: 120 };
    const platform = createPlatform({
      session: createSession({
        id: 'session-candidate-preview',
        candidates: [
          {
            id: 'window-recommended',
            kind: 'window',
            rect: recommended,
            priority: 10,
          },
        ],
      }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-candidate-preview');

    runtime.actions.pointerMove({
      point: { x: 80, y: 70 },
      button: 0,
      source: 'root',
    });
    expect(runtime.renderState.hoverSelection).toEqual(recommended);
    runtime.actions.pointerDown({
      point: { x: 80, y: 70 },
      button: 0,
      detail: 1,
      source: 'root',
    });
    await runtime.actions.pointerUp({
      point: { x: 80, y: 70 },
      button: 0,
      detail: 1,
      source: 'root',
    });

    // A clicked candidate in screenshot (preview) mode should open the editor
    // just like a manual drag, not copy-and-finish.
    expect(runtime.renderState).toMatchObject({
      status: 'preview',
      selection: recommended,
    });
    expect(runtime.renderState.selection).toEqual(recommended);
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(platform.commands.outputCapture).not.toHaveBeenCalled();
    expect(platform.window.hide).not.toHaveBeenCalled();
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

  it('does not surface stale output after resetting preview or cancelling', async () => {
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
    preview.resolve('unused-preview');
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
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(runtime.renderState.status).toBe('preview');
  });

  it('uses Shift+Cmd/Ctrl+A for the entire virtual desktop', async () => {
    const platform = createPlatform({
      session: createSession({
        id: 'session-virtual-desktop',
        monitors: [
          createMonitor(),
          {
            ...createMonitor(),
            id: 'monitor-2',
            logical_bounds: { x: 500, y: -100, width: 300, height: 400 },
            physical_bounds: { x: 1000, y: -200, width: 600, height: 800 },
          },
        ],
      }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-virtual-desktop');

    expect(
      runtime.actions.keyDown({ key: 'a', metaKey: true, shiftKey: true }),
    ).toBe(true);
    await vi.waitFor(() => expect(runtime.renderState.status).toBe('preview'));

    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
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

    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
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
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
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

    expect(runtime.renderState.annotationHistory.annotations[0]?.type).toBe(
      'freehand',
    );
    expect(runtime.renderState.draftAnnotation).toBeNull();
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
  });

  it('cancels an in-progress pen gesture without committing the draft', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-pen-cancel' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-editor-pen-cancel');
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.toggleAnnotationTool('pen');
    runtime.actions.pointerDown({ point: { x: 30, y: 40 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 70, y: 80 }, source: 'root' });

    expect(runtime.renderState.annotationGesture).not.toBeNull();
    expect(runtime.renderState.draftAnnotation).not.toBeNull();
    expect(runtime.actions.pointerCancel()).toBe(true);
    expect(runtime.renderState).toMatchObject({
      status: 'preview',
      selection,
      annotationGesture: null,
      draftAnnotation: null,
      annotationMoveGesture: null,
      editGesture: null,
    });
    expect(runtime.renderState.annotationHistory.annotations).toEqual([]);

    runtime.actions.pointerMove({ point: { x: 90, y: 90 }, source: 'root' });
    expect(runtime.renderState.annotationHistory.annotations).toEqual([]);
    expect(runtime.renderState.draftAnnotation).toBeNull();
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
  });

  it('updates preview geometry immediately even when the output renderer is unavailable', async () => {
    const platform = createPlatform();
    platform.commands.renderCaptureOutput.mockImplementation(() => new Promise(() => undefined));
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-1');
    const latest = { x: 40, y: 50, width: 220, height: 130 };
    await runtime.actions.renderSelectionPreview(selection);
    await runtime.actions.renderSelectionPreview(latest);
    expect(runtime.renderState).toMatchObject({
      status: 'preview', selection: latest, isRenderingOutput: false, error: null,
    });
    runtime.actions.toggleAnnotationTool('rectangle');
    runtime.actions.pointerDown({ point: { x: 50, y: 60 }, source: 'preview' });
    runtime.actions.pointerMove({ point: { x: 90, y: 100 }, source: 'preview' });
    await runtime.actions.pointerUp({ point: { x: 90, y: 100 }, source: 'preview' });
    expect(runtime.renderState.annotationHistory.annotations).toHaveLength(1);
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
  });

  it('copies current annotations immediately without waiting for preview encoding', async () => {
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
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(runtime.renderState.isRenderingOutput).toBe(false);

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

  it('cancels preview selection move and resize gestures at their current selection', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-editor-selection-cancel' }),
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(
      'screenshot',
      'session-editor-selection-cancel',
    );
    await runtime.actions.renderSelectionPreview(selection);
    runtime.actions.pointerDown({
      point: { x: 40, y: 50 },
      source: 'preview',
    });
    runtime.actions.pointerMove({ point: { x: 55, y: 65 }, source: 'root' });

    const movedSelection = { x: 35, y: 45, width: 120, height: 80 };
    expect(runtime.renderState.editGesture).not.toBeNull();
    expect(runtime.renderState.selection).toEqual(movedSelection);
    expect(runtime.actions.pointerCancel()).toBe(true);
    expect(runtime.renderState.editGesture).toBeNull();
    expect(runtime.renderState.selection).toEqual(movedSelection);
    runtime.actions.pointerMove({ point: { x: 80, y: 90 }, source: 'root' });
    expect(runtime.renderState.selection).toEqual(movedSelection);

    expect(
      runtime.actions.resizePointerDown('se', {
        point: { x: 155, y: 125 },
        source: 'preview',
      }),
    ).toBe(true);
    runtime.actions.pointerMove({ point: { x: 175, y: 140 }, source: 'root' });

    const resizedSelection = { x: 35, y: 45, width: 140, height: 95 };
    expect(runtime.renderState.editGesture).not.toBeNull();
    expect(runtime.renderState.selection).toEqual(resizedSelection);
    expect(runtime.actions.pointerCancel()).toBe(true);
    expect(runtime.renderState.editGesture).toBeNull();
    expect(runtime.renderState.selection).toEqual(resizedSelection);
    runtime.actions.pointerMove({ point: { x: 200, y: 160 }, source: 'root' });
    expect(runtime.renderState.selection).toEqual(resizedSelection);
    expect(runtime.renderState.annotationHistory.annotations).toEqual([]);
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

  it('owns delete, erase, clear, and output failure state', async () => {
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

    platform.commands.outputCapture.mockRejectedValueOnce(
      new Error('editor output failed'),
    );
    await runtime.actions.completePreviewSelection('copy', selection);
    expect(runtime.renderState).toMatchObject({
      status: 'error',
      selection,
      isRenderingOutput: false,
      error: 'editor output failed',
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

  it('keeps an active draft alive when the system cursor cannot be moved', async () => {
    const platform = createPlatform();
    platform.commands.moveCaptureCursor.mockRejectedValueOnce(
      new Error('cursor unavailable'),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-draft-move-error');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 100, y: 80 });

    expect(runtime.actions.keyDown({ key: 'd' })).toBe(true);

    await vi.waitFor(() =>
      expect(runtime.renderState.error).toBe(
        '鼠标移动失败：cursor unavailable',
      ),
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      startPoint: { x: 20, y: 30 },
      selection: { x: 20, y: 30, width: 81, height: 50 },
    });

    expect(runtime.actions.keyDown({ key: 'd' })).toBe(true);
    await vi.waitFor(() => expect(runtime.renderState.error).toBeNull());
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
    expect(platform.commands.moveCaptureCursor).toHaveBeenCalledWith({
      x: 1,
      y: 0,
    });
  });

  it('toggles between window and interface-element detection with Tab', async () => {
    const higher = { x: 20, y: 30, width: 120, height: 80 };
    const control = { x: 30, y: 40, width: 60, height: 24 };
    const platform = createPlatform({
      session: createSession({
        candidates: [
          { id: 'higher', kind: 'window', rect: higher, priority: 20 },
        ],
      }),
    });
    platform.commands.currentCaptureControlCandidate.mockResolvedValue({
      id: 'control-1',
      kind: 'control',
      rect: control,
      priority: 10_001,
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-cycle');
    runtime.actions.pointerMove({ x: 40, y: 50 });
    expect(runtime.renderState.hoverSelection).toEqual(higher);

    expect(runtime.actions.keyDown({ key: 'Tab' })).toBe(true);

    expect(runtime.renderState.candidateDetectionMode).toBe('control');
    await vi.waitFor(() =>
      expect(runtime.renderState.hoverSelection).toEqual(control),
    );
    expect(
      platform.commands.currentCaptureControlCandidate,
    ).toHaveBeenCalledWith('session-1', { x: 40, y: 50 });

    expect(runtime.actions.keyDown({ key: 'Tab' })).toBe(true);

    expect(runtime.renderState.candidateDetectionMode).toBe('window');
    expect(runtime.renderState.hoverSelection).toEqual(higher);
  });

  it('refreshes interface-element detection from the current pointer move', async () => {
    const control = { x: 30, y: 40, width: 60, height: 24 };
    const platform = createPlatform();
    platform.commands.currentCaptureControlCandidate.mockResolvedValue({
      id: 'control-pointer-move',
      kind: 'control',
      rect: control,
      priority: 10_001,
    });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-control-pointer');
    expect(runtime.actions.keyDown({ key: 'Tab' })).toBe(true);

    runtime.actions.pointerMove({ x: 40, y: 50 });

    await vi.waitFor(() =>
      expect(platform.commands.currentCaptureControlCandidate).toHaveBeenCalledWith(
        'session-1',
        { x: 40, y: 50 },
      ),
    );
    expect(runtime.renderState.hoverSelection).toEqual(control);
  });

  it('drops a visible interface-element candidate when the pointer leaves it', async () => {
    const control = { x: 30, y: 40, width: 60, height: 24 };
    const pendingOutsideCandidate = deferred<null>();
    const platform = createPlatform();
    platform.commands.currentCaptureControlCandidate
      .mockResolvedValueOnce({
        id: 'control-visible',
        kind: 'control',
        rect: control,
        priority: 10_001,
      })
      .mockImplementationOnce(() => pendingOutsideCandidate.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-control-leave');
    runtime.actions.pointerMove({ x: 40, y: 50 });
    expect(runtime.actions.keyDown({ key: 'Tab' })).toBe(true);
    await vi.waitFor(() =>
      expect(runtime.renderState.hoverSelection).toEqual(control),
    );

    runtime.actions.pointerMove({ x: 120, y: 90 });

    expect(runtime.renderState.hoverSelection).toBeNull();
    pendingOutsideCandidate.resolve(null);
  });

  it('keeps one interface-element query in flight and queues only the latest point across Tab and pointer moves', async () => {
    const latestControl = { x: 70, y: 80, width: 50, height: 20 };
    const pendingInitial = deferred<null>();
    const platform = createPlatform();
    platform.commands.currentCaptureControlCandidate
      .mockImplementationOnce(() => pendingInitial.promise)
      .mockResolvedValue({
        id: 'control-latest',
        kind: 'control',
        rect: latestControl,
        priority: 10_001,
      });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-control-coalesce');
    runtime.actions.pointerMove({ x: 20, y: 20 });
    expect(runtime.actions.keyDown({ key: 'Tab' })).toBe(true);
    expect(platform.commands.currentCaptureControlCandidate).toHaveBeenCalledTimes(1);

    runtime.actions.pointerMove({ x: 40, y: 50 });
    runtime.actions.pointerMove({ x: 60, y: 70 });
    runtime.actions.pointerMove({ x: 80, y: 90 });

    expect(platform.commands.currentCaptureControlCandidate).toHaveBeenCalledTimes(1);
    pendingInitial.resolve(null);
    await vi.waitFor(() =>
      expect(platform.commands.currentCaptureControlCandidate).toHaveBeenCalledTimes(2),
    );
    expect(platform.commands.currentCaptureControlCandidate).toHaveBeenLastCalledWith(
      'session-1',
      { x: 80, y: 90 },
    );
    expect(runtime.renderState.hoverSelection).toEqual(latestControl);
  });

  it('consumes Tab before the initial cursor position is available', async () => {
    const platform = createPlatform();
    platform.commands.currentCaptureCursorPosition.mockResolvedValue(null);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-no-cursor');

    expect(runtime.renderState.cursorPoint).toBeNull();
    expect(runtime.actions.keyDown({ key: 'Tab' })).toBe(true);
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      candidateDetectionMode: 'control',
      hoverSelection: null,
    });
    expect(
      platform.commands.currentCaptureControlCandidate,
    ).not.toHaveBeenCalled();
  });

  it('ignores a stale control-detection failure after the cursor moves', async () => {
    const staleCandidate = deferred<null>();
    const control = { x: 31, y: 40, width: 60, height: 24 };
    const platform = createPlatform();
    platform.commands.currentCaptureControlCandidate
      .mockImplementationOnce(() => staleCandidate.promise)
      .mockResolvedValueOnce({
        id: 'control-current',
        kind: 'control',
        rect: control,
        priority: 10_001,
      });
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-stale-control');
    runtime.actions.pointerMove({ x: 40, y: 50 });

    expect(runtime.actions.keyDown({ key: 'Tab' })).toBe(true);
    expect(runtime.actions.keyDown({ key: 'd' })).toBe(true);

    expect(platform.commands.currentCaptureControlCandidate).toHaveBeenCalledTimes(1);
    staleCandidate.reject(new Error('stale failure'));
    await staleCandidate.promise.catch(() => undefined);
    await vi.waitFor(() =>
      expect(runtime.renderState.hoverSelection).toEqual(control),
    );

    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      candidateDetectionMode: 'control',
      hoverSelection: control,
      error: null,
    });
  });

  it('keeps the capture active and restores window detection when control detection fails', async () => {
    const platform = createPlatform({
      session: createSession({
        candidates: [
          { id: 'window-1', kind: 'window', rect: selection, priority: 10 },
        ],
      }),
    });
    platform.commands.currentCaptureControlCandidate.mockRejectedValue(
      new Error('需要辅助功能权限'),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-permission');
    runtime.actions.pointerMove({ x: 40, y: 50 });

    expect(runtime.actions.keyDown({ key: 'Tab' })).toBe(true);

    await vi.waitFor(() =>
      expect(runtime.renderState.candidateDetectionMode).toBe('window'),
    );
    expect(runtime.renderState).toMatchObject({
      status: 'selecting',
      hoverSelection: selection,
      error: '界面元素检测不可用：需要辅助功能权限',
    });
  });

  it('commits the keyboard-adjusted draft endpoint on pointer release', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-nudge-release');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 100, y: 80 });
    expect(runtime.actions.keyDown({ key: 'd' })).toBe(true);

    await runtime.actions.pointerUp({ x: 100, y: 80 });

    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
  });

  it('uses the pointer-up coordinate when the draft was not keyboard-adjusted', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-pointer-release');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 100, y: 80 });

    await runtime.actions.pointerUp({ x: 120, y: 90 });

    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
  });

  it('uses pointer-derived hover state for Enter and local copy', async () => {
    const candidateB = { x: 200, y: 40, width: 80, height: 70 };
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (id) =>
      createSession({
        id,
        candidates: [
          { id: 'window-b', kind: 'window', rect: candidateB, priority: 10 },
        ],
      }),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot-copy', 'session-poll-enter');
    runtime.actions.pointerMove({ x: 220, y: 60 });
    expect(runtime.actions.keyDown({ key: 'Enter' })).toBe(true);
    await vi.waitFor(() => expect(platform.commands.outputCapture).toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: 'session-poll-enter', rect: candidateB }),
    ));

    await runtime.actions.startSession('screenshot-copy', 'session-poll-native');
    runtime.actions.pointerMove({ x: 220, y: 60 });
    await runtime.actions.connectHost();
    runtime.actions.keyDown({ key: 'c', metaKey: true });
    await vi.waitFor(() => expect(platform.commands.outputCapture).toHaveBeenCalledTimes(2));
    expect(platform.commands.outputCapture).toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: 'session-poll-native', rect: candidateB }),
    );
  });

  it('cancels a selecting draft on Escape without delegating to the editor', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-draft-escape');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 100, y: 80 });

    expect(runtime.actions.keyDown({ key: 'Escape' })).toBe(true);
    await vi.waitFor(() => expect(runtime.renderState.status).toBe('idle'));
    expect(platform.window.hide).toHaveBeenCalledOnce();
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
    await vi.waitFor(() => expect(platform.window.hide).toHaveBeenCalledTimes(1));
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
    await vi.waitFor(() => expect(platform.window.hide).toHaveBeenCalledTimes(1));
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
    expect(platform.window.hide).toHaveBeenCalledTimes(1);
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

  it('surfaces failed hydration and hydrates a restarted session cleanly', async () => {
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
      status: 'error',
      sessionId: 'session-hydration',
      hasHydratedPixelSource: false,
      error: 'hydrate failed',
    });
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();
    expect(platform.window.hide).not.toHaveBeenCalled();

    await runtime.actions.startSession('screenshot', 'session-hydration');
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

  it('hydrates only the requested monitor for the magnifier and reuses it', async () => {
    const platform = createPlatform({
      session: createSession({ id: 'session-magnifier' }),
    });
    platform.commands.hydrateCaptureMonitorSnapshot.mockResolvedValue(
      createMonitor({ image_base64: 'magnifier-pixels' }),
    );
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-magnifier');
    await runtime.actions.hydrateMagnifierMonitor('monitor-1');
    await runtime.actions.hydrateMagnifierMonitor('monitor-1');

    expect(
      platform.commands.hydrateCaptureMonitorSnapshot,
    ).toHaveBeenCalledOnce();
    expect(
      platform.commands.hydrateCaptureMonitorSnapshot,
    ).toHaveBeenCalledWith('session-magnifier', 'monitor-1');
    expect(runtime.renderState.session?.monitors[0].image_base64).toBe(
      'magnifier-pixels',
    );
    expect(runtime.renderState.hasHydratedPixelSource).toBe(false);
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

    expect(platform.window.hide).not.toHaveBeenCalled();
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

    expect(platform.window.hide).not.toHaveBeenCalled();
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
      isRenderingOutput: false,
      error: null,
    });
  });

  it('hands OCR to the backend without waiting for recognition', async () => {
    const platform = createPlatform({ session: createSession({ id: 'session-ocr' }) });
    const recognition = deferred<{ text: string; confidence: null }>();
    platform.commands.runCaptureOcr.mockImplementation(() => recognition.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot-ocr', 'session-ocr');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    const completion = runtime.actions.pointerUp({ x: 140, y: 110 });

    await vi.waitFor(() => expect(platform.commands.completeCaptureOcr).toHaveBeenCalledWith('session-ocr'));
    await completion;
    expect(platform.commands.prepareCaptureOcr).toHaveBeenCalledWith({
      sessionId: 'session-ocr', rect: selection, annotations: [], target: 'ocr-window',
    });
    expect(runtime.renderState.status).toBe('idle');
    expect(platform.commands.runCaptureOcr).not.toHaveBeenCalled();
  });

  it('does not commit a prepared OCR selection for a stale session', async () => {
    const ocr = deferred<void>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(async (sessionId) =>
      createSession({ id: sessionId }),
    );
    platform.commands.prepareCaptureOcr.mockImplementation(() => ocr.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot-ocr', 'session-old');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    const oldCompletion = runtime.actions.pointerUp({ x: 140, y: 110 });
    await runtime.actions.startSession('screenshot', 'session-new');
    ocr.resolve();
    await oldCompletion;

    expect(platform.commands.completeCaptureOcr).not.toHaveBeenCalled();
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
    platform.window.hide.mockImplementation(() => dismiss.promise);
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

    expect(platform.window.hide).toHaveBeenCalledTimes(1);
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
    platform.window.hide.mockImplementation(() => dismiss.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('screenshot', 'session-old');
    expect(runtime.actions.keyDown({ key: 'Escape' })).toBe(true);
    await runtime.actions.startSession('screenshot', 'session-new');
    dismiss.reject(new Error('old dismiss failed'));
    expect(platform.window.hide).toHaveBeenCalledTimes(1);
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

    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
    expect(platform.window.hide).not.toHaveBeenCalled();
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();
    expect(runtime.renderState).toMatchObject({
      status: 'preview',
      sessionId: 'session-preview',
      selection,
      isRenderingOutput: false,
      error: null,
    });
  });

  it.each([
    ['screenshot-ocr', 'ocr-window'],
    ['silent-screenshot-ocr', 'clipboard'],
    ['screenshot-translate', 'translation-window'],
  ] as const)('hands %s to the correct backend target', async (mode, target) => {
    const platform = createPlatform({ session: createSession({ id: 'session-ocr' }) });
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession(mode, 'session-ocr');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    await runtime.actions.pointerUp({ x: 140, y: 110 });

    expect(platform.commands.prepareCaptureOcr).toHaveBeenCalledWith({
      sessionId: 'session-ocr', rect: selection, annotations: [], target,
    });
    expect(platform.commands.completeCaptureOcr).toHaveBeenCalledWith('session-ocr');
    expect(platform.commands.copyTextToClipboard).not.toHaveBeenCalled();
    expect(platform.commands.runCaptureOcr).not.toHaveBeenCalled();
    expect(runtime.renderState).toMatchObject({ status: 'idle', sessionId: null, error: null });
  });

  it('releases silent OCR as soon as the selected pixels are handed off', async () => {
    const prepared = deferred<void>();
    const platform = createPlatform({ session: createSession({ id: 'session-silent' }) });
    platform.commands.prepareCaptureOcr.mockImplementation(() => prepared.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.startSession('silent-screenshot-ocr', 'session-silent');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    const completion = runtime.actions.pointerUp({ x: 140, y: 110 });
    expect(runtime.renderState.silentOcrHint).toEqual({
      status: 'loading', point: { x: 140, y: 110 },
    });
    expect(platform.commands.completeCaptureOcr).not.toHaveBeenCalled();

    prepared.resolve();
    await completion;
    expect(runtime.renderState.status).toBe('idle');
    expect(runtime.renderState.silentOcrHint).toBeNull();
    expect(platform.commands.completeCaptureOcr).toHaveBeenCalledWith('session-silent');
  });

  it('suppresses silent OCR preparation status when configured', async () => {
    const prepared = deferred<void>();
    const platform = createPlatform();
    platform.commands.prepareCaptureOcr.mockImplementation(() => prepared.promise);
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      ocrPreferences: () => ({
        recognitionLanguage: 'auto', preserveFormatting: true,
        removeChineseSpaces: true, showConfidence: false, hideSilentStatus: true,
      }),
    });
    await runtime.actions.startSession('silent-screenshot-ocr', 'session-hidden');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    const completion = runtime.actions.pointerUp({ x: 140, y: 110 });
    expect(runtime.renderState.silentOcrHint).toBeNull();
    prepared.resolve();
    await completion;
  });

  it('does not restore hover feedback while OCR pixels are being prepared', async () => {
    const prepared = deferred<void>();
    const platform = createPlatform({ session: createSession({ id: 'session-pending' }) });
    platform.commands.prepareCaptureOcr.mockImplementation(() => prepared.promise);
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot-ocr', 'session-pending');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    const completion = runtime.actions.pointerUp({ x: 140, y: 110 });
    runtime.actions.pointerMove({ x: 60, y: 70 });
    expect(runtime.renderState).toMatchObject({
      status: 'loading', cursorPoint: null, hoverSelection: null,
      selection, isRenderingOutput: true,
    });
    prepared.resolve();
    await completion;
  });

  it('preserves the selected OCR language during handoff', async () => {
    const platform = createPlatform({ session: createSession({ id: 'session-ocr' }) });
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      ocrPreferences: () => ({
        recognitionLanguage: 'ja', preserveFormatting: true,
        removeChineseSpaces: true, showConfidence: false,
      }),
    });
    await runtime.actions.startSession('screenshot-ocr', 'session-ocr');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    await runtime.actions.pointerUp({ x: 140, y: 110 });
    expect(platform.commands.prepareCaptureOcr).toHaveBeenCalledWith({
      sessionId: 'session-ocr', rect: selection, annotations: [],
      target: 'ocr-window', language: 'ja',
    });
  });

  it('keeps preparation errors visible without committing an OCR job', async () => {
    const platform = createPlatform();
    platform.commands.prepareCaptureOcr.mockRejectedValue(new Error('pixels unavailable'));
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot-ocr', 'session-error');
    runtime.actions.pointerDown({ x: 20, y: 30 });
    runtime.actions.pointerMove({ x: 140, y: 110 });
    await runtime.actions.pointerUp({ x: 140, y: 110 });
    expect(runtime.renderState).toMatchObject({ status: 'error', error: 'pixels unavailable' });
    expect(platform.commands.completeCaptureOcr).not.toHaveBeenCalled();
  });

  it('persists font and stroke defaults only through the injected settings seam', () => {
    const persistScreenshotDefaults = vi.fn();
    const runtime = createCaptureWorkspaceRuntime({
      platform: createPlatform(),
      persistScreenshotDefaults,
    });

    runtime.actions.commitAnnotationSizeDefault('stroke', 5);
    runtime.actions.commitAnnotationSizeDefault('font', 28);

    expect(persistScreenshotDefaults.mock.calls).toEqual([
      [{ defaultStrokeWidth: 5 }],
      [{ defaultFontSize: 28 }],
    ]);
  });

  it('delivers one rendered image to the print adapter for overlapping print intents', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-1');
    await runtime.actions.renderSelectionPreview(selection);
    const rendered = deferred<string>();
    platform.commands.renderCaptureOutput.mockReturnValueOnce(rendered.promise);
    platform.commands.renderCaptureOutput.mockClear();
    const printing = runtime.actions.completePreviewSelection('print', selection);
    await runtime.actions.completePreviewSelection('print', selection);
    expect(platform.commands.renderCaptureOutput).toHaveBeenCalledOnce();
    expect(platform.print.printImage).not.toHaveBeenCalled();
    rendered.resolve('frozen-print-pixels');
    await printing;
    expect(platform.print.printImage).toHaveBeenCalledExactlyOnceWith('frozen-print-pixels');
    expect(runtime.renderState.status).toBe('idle');
  });

  it('does not print an output whose session was cancelled while rendering', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });
    await runtime.actions.startSession('screenshot', 'session-1');
    await runtime.actions.renderSelectionPreview(selection);
    const rendered = deferred<string>();
    platform.commands.renderCaptureOutput.mockReturnValueOnce(rendered.promise);
    const printing = runtime.actions.completePreviewSelection('print', selection);
    await runtime.actions.cancelSession();
    rendered.resolve('obsolete-print-pixels');
    await printing;
    expect(platform.print.printImage).not.toHaveBeenCalled();
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledOnce();
    expect(runtime.renderState.status).toBe('idle');
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
        CaptureWorkspacePorts['commands']['getCaptureSession']
      >(async () => session),
      hydrateCaptureSessionSnapshots: vi.fn<
        CaptureWorkspacePorts['commands']['hydrateCaptureSessionSnapshots']
      >(async () => session),
      hydrateCaptureMonitorSnapshot: vi.fn<
        CaptureWorkspacePorts['commands']['hydrateCaptureMonitorSnapshot']
      >(async (_sessionId, monitorId) => {
        const monitor = session.monitors.find((candidate) => candidate.id === monitorId);
        if (!monitor) throw new Error(`Monitor not found: ${monitorId}`);
        return monitor;
      }),
      logCaptureFrontendPerf: vi.fn(async () => undefined),
      currentCaptureCursorPosition: vi.fn<
        CaptureWorkspacePorts['commands']['currentCaptureCursorPosition']
      >(async () => null),
      currentCaptureControlCandidate: vi.fn<
        CaptureWorkspacePorts['commands']['currentCaptureControlCandidate']
      >(async () => null),
      moveCaptureCursor: vi.fn<
        CaptureWorkspacePorts['commands']['moveCaptureCursor']
      >(async () => undefined),
      cancelCaptureSession: vi.fn<
        CaptureWorkspacePorts['commands']['cancelCaptureSession']
      >(async () => undefined),
      restoreCaptureSnapshotWindowsForSession: vi.fn(async () => undefined),
      renderCaptureOutput: vi.fn(async () => 'preview-image'),
      defaultCaptureSavePath: vi.fn<
        CaptureWorkspacePorts['commands']['defaultCaptureSavePath']
      >(async () => '/captures/capture.png'),
      quickCaptureSavePath: vi.fn(async () => '/captures/quick.png'),
      outputCapture: vi.fn<
        CaptureWorkspacePorts['commands']['outputCapture']
      >(async () => undefined),
      runCaptureOcr: vi.fn(async () => ({ text: '', confidence: null })),
      prepareCaptureOcr: vi.fn(async (_input: unknown): Promise<void> => undefined),
      completeCaptureOcr: vi.fn(async (_sessionId: string): Promise<void> => undefined),
      openCaptureOcrResultWindow: vi.fn(async () => undefined),
      openCaptureTranslationResultWindow: vi.fn(async () => undefined),
      copyTextToClipboard: vi.fn(async () => undefined),
    },
    clipboard: {
      writeText: vi.fn(async () => undefined),
    },
    events: {
    subscribeHotkeyTriggered: vi.fn<
      CaptureWorkspacePorts['events']['subscribeHotkeyTriggered']
    >(async () => () => undefined),
    },
    window: {
    prepareForReveal: vi.fn(async () => undefined),
    reveal: vi.fn(async () => undefined),
    hide: vi.fn<CaptureWorkspacePorts['window']['hide']>(async () =>
      undefined,
    ),
    },
    print: { printImage: vi.fn<CaptureWorkspacePorts['print']['printImage']>(async () => undefined) },
  } satisfies CaptureWorkspacePorts;
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
