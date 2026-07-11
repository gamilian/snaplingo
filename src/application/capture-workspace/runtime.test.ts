import { describe, expect, it, vi } from 'vitest';

import type { CaptureWorkspacePlatformRuntime } from './platformRuntime';
import { createCaptureWorkspaceRuntime } from './runtime';

const selection = { x: 20, y: 30, width: 120, height: 80 };

describe('capture workspace runtime', () => {
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
});

function createPlatform({
  session = createSession(),
}: {
  session?: ReturnType<typeof createSession>;
} = {}) {
  return {
    commands: {
      createCaptureSession: vi.fn(async () => session),
      getCaptureSession: vi.fn(async () => session),
      hydrateCaptureSessionSnapshots: vi.fn(async () => session),
      logCaptureFrontendPerf: vi.fn(async () => undefined),
      currentCaptureCursorPosition: vi.fn(async () => null),
      cancelCaptureSession: vi.fn(async () => undefined),
      restoreCaptureSnapshotWindowsForSession: vi.fn(async () => undefined),
      renderCaptureOutput: vi.fn(async () => 'preview-image'),
      defaultCaptureSavePath: vi.fn(async () => '/captures/capture.png'),
      quickCaptureSavePath: vi.fn(async () => '/captures/quick.png'),
      outputCapture: vi.fn(async () => undefined),
      runCaptureOcr: vi.fn(async () => ({ text: '', confidence: null })),
      openCaptureOcrResultWindow: vi.fn(async () => undefined),
      openCaptureTranslationResultWindow: vi.fn(async () => undefined),
      copyTextToClipboard: vi.fn(async () => undefined),
    },
    clipboard: {
      copyText: vi.fn(async () => undefined),
    },
    onCancelRequested: vi.fn(async () => vi.fn()),
    onCopyRequested: vi.fn(async () => vi.fn()),
    onHotkeyTriggered: vi.fn(async () => vi.fn()),
    prepareForReveal: vi.fn(async () => undefined),
    reveal: vi.fn(async () => undefined),
    dismiss: vi.fn(async () => undefined),
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
