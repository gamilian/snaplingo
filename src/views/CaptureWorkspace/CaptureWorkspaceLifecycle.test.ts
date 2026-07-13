// @vitest-environment happy-dom

import { StrictMode, act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it, vi } from 'vitest';

import type { CaptureWorkspacePlatformRuntime } from '../../application/capture-workspace/platformRuntime';
import CaptureWorkspace from './index';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;
Object.defineProperty(window, 'localStorage', {
  configurable: true,
  value: createMemoryStorage(),
});

describe('CaptureWorkspace React lifecycle', () => {
  it('reveals after session metadata loads before capture images are hydrated', async () => {
    const sessionRequest = deferred<ReturnType<typeof createSession>>();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(
      () => sessionRequest.promise,
    );
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        createElement(CaptureWorkspace, {
          runtime: platform,
          initialMode: 'screenshot',
          initialSessionId: 'metadata-session',
        }),
      );
    });

    await act(async () => {
      sessionRequest.resolve(createSession('metadata-session'));
      await sessionRequest.promise;
    });

    await vi.waitFor(() => expect(platform.reveal).toHaveBeenCalledOnce());
    expect(platform.prepareForReveal).toHaveBeenCalledOnce();

    await act(async () => root.unmount());
    container.remove();
  });

  it('replaces the disposed StrictMode runtime and cleans late work on unmount', async () => {
    const oldSessionRequest = deferred<ReturnType<typeof createSession>>();
    const currentSessionRequest = deferred<ReturnType<typeof createSession>>();
    const lateHotkeyRegistration = deferred<() => void>();
    const disposeLateHotkey = vi.fn();
    const hotkeyHandlers: Array<
      Parameters<CaptureWorkspacePlatformRuntime['onHotkeyTriggered']>[0]
    > = [];
    let getSessionCall = 0;
    let hotkeyRegistrationCall = 0;
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(() => {
      getSessionCall += 1;
      return getSessionCall === 1
        ? oldSessionRequest.promise
        : currentSessionRequest.promise;
    });
    platform.onHotkeyTriggered.mockImplementation((handler) => {
      hotkeyHandlers.push(handler);
      hotkeyRegistrationCall += 1;
      return hotkeyRegistrationCall === 1
        ? lateHotkeyRegistration.promise
        : Promise.resolve(vi.fn<() => void>());
    });
    const addEventListener = vi.spyOn(window, 'addEventListener');
    const removeEventListener = vi.spyOn(window, 'removeEventListener');
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        createElement(
          StrictMode,
          null,
          createElement(CaptureWorkspace, {
            runtime: platform,
            initialMode: 'screenshot',
            initialSessionId: 'strict-session',
          }),
        ),
      );
    });

    await vi.waitFor(() =>
      expect(platform.commands.getCaptureSession).toHaveBeenCalledTimes(2),
    );
    await act(async () => {
      lateHotkeyRegistration.resolve(disposeLateHotkey);
      oldSessionRequest.resolve(createSession('strict-old'));
      currentSessionRequest.resolve(createSession('strict-current'));
      await Promise.resolve();
    });
    await vi.waitFor(() =>
      expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
        'strict-old',
      ),
    );
    expect(disposeLateHotkey).toHaveBeenCalledOnce();

    await act(async () => root.unmount());
    await vi.waitFor(() =>
      expect(platform.commands.cancelCaptureSession).toHaveBeenCalledWith(
        'strict-current',
      ),
    );

    for (const handler of hotkeyHandlers) {
      await handler({ mode: 'screenshot', sessionId: 'after-unmount' });
    }
    expect(platform.commands.getCaptureSession).toHaveBeenCalledTimes(2);
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([sessionId]) => sessionId === 'strict-old',
      ),
    ).toHaveLength(1);
    expect(
      platform.commands.cancelCaptureSession.mock.calls.filter(
        ([sessionId]) => sessionId === 'strict-current',
      ),
    ).toHaveLength(1);
    expect(listenerCalls(addEventListener, 'keydown')).toBe(
      listenerCalls(removeEventListener, 'keydown'),
    );
    expect(listenerCalls(addEventListener, 'keyup')).toBe(
      listenerCalls(removeEventListener, 'keyup'),
    );
    expect(listenerCalls(addEventListener, 'blur')).toBe(
      listenerCalls(removeEventListener, 'blur'),
    );

    container.remove();
    addEventListener.mockRestore();
    removeEventListener.mockRestore();
  });
});

function listenerCalls(
  spy: { mock: { calls: unknown[][] } },
  type: string,
) {
  return spy.mock.calls.filter((call) => call[0] === type).length;
}

function createPlatform() {
  return {
    commands: {
      createCaptureSession: vi.fn(async () => createSession('created')),
      getCaptureSession: vi.fn<
        CaptureWorkspacePlatformRuntime['commands']['getCaptureSession']
      >(async () => createSession('loaded')),
      hydrateCaptureSessionSnapshots: vi.fn(async () =>
        createSession('hydrated'),
      ),
      logCaptureFrontendPerf: vi.fn(async () => undefined),
      currentCaptureCursorPosition: vi.fn(async () => null),
      cancelCaptureSession: vi.fn<
        CaptureWorkspacePlatformRuntime['commands']['cancelCaptureSession']
      >(async () => undefined),
      restoreCaptureSnapshotWindowsForSession: vi.fn(async () => undefined),
      renderCaptureOutput: vi.fn(async () => 'preview-image'),
      defaultCaptureSavePath: vi.fn(async () => '/capture.png'),
      quickCaptureSavePath: vi.fn(async () => '/quick.png'),
      outputCapture: vi.fn(async () => undefined),
      runCaptureOcr: vi.fn(async () => ({ text: '', confidence: null })),
      openCaptureOcrResultWindow: vi.fn(async () => undefined),
      openCaptureTranslationResultWindow: vi.fn(async () => undefined),
      copyTextToClipboard: vi.fn(async () => undefined),
    },
    clipboard: { copyText: vi.fn(async () => undefined) },
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
    dismiss: vi.fn(async () => undefined),
  } satisfies CaptureWorkspacePlatformRuntime;
}

function createSession(id: string) {
  return {
    id,
    monitors: [
      {
        id: 'monitor-1',
        logical_bounds: { x: 0, y: 0, width: 500, height: 300 },
        physical_bounds: { x: 0, y: 0, width: 1000, height: 600 },
        scale_factor: 2,
        image_base64: '',
      },
    ],
    candidates: [],
    captured_cursor: null,
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

function createMemoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => {
      values.delete(key);
    },
    setItem: (key, value) => {
      values.set(key, value);
    },
  };
}
