import { describe, expect, it, vi } from 'vitest';

import type { CaptureWorkspacePlatformRuntime } from './platformRuntime';
import { createCaptureWorkspaceRuntime } from './runtime';

const selection = { x: 20, y: 30, width: 120, height: 80 };

describe('capture workspace runtime', () => {
  it('owns host subscriptions and cleans every listener up together', async () => {
    const platform = createPlatform();
    const unlistenHotkey = vi.fn();
    const unlistenCancel = vi.fn();
    const unlistenCopy = vi.fn();
    platform.onHotkeyTriggered.mockResolvedValue(unlistenHotkey);
    platform.onCancelRequested.mockResolvedValue(unlistenCancel);
    platform.onCopyRequested.mockResolvedValue(unlistenCopy);
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

    disconnect();
    expect(unlistenHotkey).toHaveBeenCalledTimes(1);
    expect(unlistenCancel).toHaveBeenCalledTimes(1);
    expect(unlistenCopy).toHaveBeenCalledTimes(1);
  });

  it('handles native preview copy through runtime-owned completion effects', async () => {
    const annotation = {
      type: 'rectangle' as const,
      rect: { x: 1, y: 2, width: 10, height: 20 },
      color: [255, 0, 0, 255] as [number, number, number, number],
      stroke_width: 2,
      filled: false,
    };
    const platform = createPlatform({
      session: createSession({ id: 'session-native-copy' }),
    });
    const runtime = createCaptureWorkspaceRuntime({
      platform,
      host: {
        resetInteraction: vi.fn(),
        resetSession: vi.fn(),
        applyManualSelection: vi.fn(),
        getAnnotations: () => [],
        commitTextDraft: () => [annotation],
        shouldIncludeCursor: () => true,
        hasTextDraft: () => false,
        prepareSurface: vi.fn(),
        getSnapTargetRects: () => [],
      },
    });
    await runtime.actions.startSession('screenshot', 'session-native-copy');
    await runtime.actions.renderSelectionPreview(selection);
    await runtime.actions.connectHost();

    const copy = platform.onCopyRequested.mock.calls[0]?.[0];
    await copy?.();

    expect(platform.commands.outputCapture).toHaveBeenCalledWith({
      sessionId: 'session-native-copy',
      rect: selection,
      annotations: [annotation],
      includeCursor: true,
      action: { type: 'copy' },
    });
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
      'session-native-copy',
    );
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
        applyManualSelection: vi.fn(),
        getAnnotations: () => [],
        commitTextDraft: () => [],
        shouldIncludeCursor: () => false,
        hasTextDraft: () => false,
        prepareSurface,
        getSnapTargetRects: () => [],
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

  it('ignores refresh without an active session', async () => {
    const platform = createPlatform();
    const runtime = createCaptureWorkspaceRuntime({ platform });

    await runtime.actions.refreshSession();

    expect(platform.commands.createCaptureSession).not.toHaveBeenCalled();
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

    await expect(runtime.actions.keyDown({ key: 'F5' })).resolves.toBe(true);
    await expect(
      runtime.actions.keyDown({ key: 'a', metaKey: true }),
    ).resolves.toBe(true);

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
    await expect(runtime.actions.keyDown({ key: 'r' })).resolves.toBe(true);
    await expect(
      runtime.actions.keyDown({ key: 'c', metaKey: true }),
    ).resolves.toBe(true);

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
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
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
    await runtime.actions.keyDown({ key: 'Enter' });

    expect(platform.commands.outputCapture).toHaveBeenCalledWith(
      expect.objectContaining({
        sessionId: 'session-confirm',
        rect: selection,
        action: { type: 'copy' },
      }),
    );
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
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
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1);
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
    const oldCancel = runtime.actions.keyDown({ key: 'Escape' });
    await runtime.actions.startSession('screenshot', 'session-new');
    dismiss.resolve();
    await oldCancel;

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
    const oldCancel = runtime.actions.keyDown({ key: 'Escape' });
    await runtime.actions.startSession('screenshot', 'session-new');
    dismiss.reject(new Error('old dismiss failed'));
    await oldCancel;

    expect(platform.dismiss).toHaveBeenCalledTimes(1);
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledTimes(1);
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
      hydrateCaptureSessionSnapshots: vi.fn(async () => session),
      logCaptureFrontendPerf: vi.fn(async () => undefined),
      currentCaptureCursorPosition: vi.fn<
        CaptureWorkspacePlatformRuntime['commands']['currentCaptureCursorPosition']
      >(async () => null),
      cancelCaptureSession: vi.fn(async () => undefined),
      restoreCaptureSnapshotWindowsForSession: vi.fn(async () => undefined),
      renderCaptureOutput: vi.fn(async () => 'preview-image'),
      defaultCaptureSavePath: vi.fn(async () => '/captures/capture.png'),
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
