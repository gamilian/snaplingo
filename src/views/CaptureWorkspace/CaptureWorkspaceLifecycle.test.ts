// @vitest-environment happy-dom

import { StrictMode, act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { CaptureWorkspacePorts } from '../../application/capture-workspace/ports';
import type { CaptureSessionView } from './types';
import CaptureWorkspace from './index';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;
Object.defineProperty(window, 'localStorage', {
  configurable: true,
  value: createMemoryStorage(),
});

afterEach(() => vi.restoreAllMocks());

describe('CaptureWorkspace React lifecycle', () => {
  it.each(['supported', 'rejected'] as const)('drags a selection into a preview when pointer capture is %s', async (capture) => {
    vi.spyOn(HTMLImageElement.prototype, 'decode').mockResolvedValue(undefined);
    const platform = createPlatform();
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);

    try {
      await act(async () => {
        root.render(createElement(CaptureWorkspace, {
          ports: platform,
          initialMode: 'screenshot',
          initialSessionId: 'drag-session',
        }));
      });
      await vi.waitFor(() => expect(platform.window.reveal).toHaveBeenCalledOnce());
      const surface = container.firstElementChild as HTMLDivElement;
      surface.setPointerCapture = vi.fn(() => {
        if (capture === 'rejected') {
          throw new DOMException('No active pointer', 'NotFoundError');
        }
      });
      for (const [type, x, y] of [
        ['pointerdown', 40, 50],
        ['pointermove', 200, 160],
        ['pointerup', 200, 160],
      ] as const) {
        await act(async () => {
          surface.dispatchEvent(new PointerEvent(type, {
            bubbles: true,
            cancelable: true,
            pointerId: 1,
            pointerType: 'mouse',
            button: 0,
            buttons: type === 'pointerup' ? 0 : 1,
            clientX: x,
            clientY: y,
          }));
        });
      }
      await vi.waitFor(() => expect(platform.commands.renderCaptureOutput).toHaveBeenCalledWith(
        expect.objectContaining({ rect: { x: 40, y: 50, width: 160, height: 110 } }),
      ));
      expect(container.querySelector('img[src="data:image/png;base64,preview-image"]')).not.toBeNull();
    } finally {
      await act(async () => root.unmount());
      container.remove();
    }
  });

  it('shows frozen images on every monitor and waits for both images to decode before reveal', async () => {
    const sessionRequest = deferred<ReturnType<typeof createSession>>();
    const pixelsRequest = deferred<ReturnType<typeof createSession>>();
    const primaryDecoded = deferred<void>();
    const secondaryDecoded = deferred<void>();
    const decode = vi.spyOn(HTMLImageElement.prototype, 'decode')
      .mockImplementationOnce(() => primaryDecoded.promise)
      .mockImplementationOnce(() => secondaryDecoded.promise);
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockImplementation(
      () => sessionRequest.promise,
    );
    platform.commands.hydrateCaptureSessionSnapshots.mockReturnValue(pixelsRequest.promise);
    const session = createSession('frozen-session');
    session.monitors.push({
      id: 'monitor-left',
      logical_bounds: { x: -400, y: -100, width: 400, height: 300 },
      physical_bounds: { x: -400, y: -100, width: 400, height: 300 },
      scale_factor: 1,
      image_base64: '',
    });
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        createElement(CaptureWorkspace, {
          ports: platform,
          initialMode: 'screenshot',
          initialSessionId: session.id,
        }),
      );
    });

    await act(async () => {
      sessionRequest.resolve(session);
      await sessionRequest.promise;
    });

    expect(platform.window.reveal).not.toHaveBeenCalled();
    expect(platform.commands.hydrateCaptureSessionSnapshots).toHaveBeenCalledWith(session.id);

    await act(async () => {
      pixelsRequest.resolve({
        ...session,
        monitors: session.monitors.map((monitor) => ({
          ...monitor,
          image_base64: `frozen-${monitor.id}`,
        })),
      });
      await pixelsRequest.promise;
    });

    const images = container.querySelectorAll<HTMLImageElement>('[data-capture-monitor]');
    expect(images).toHaveLength(2);
    expect(images[0].src).toBe('data:image/png;base64,frozen-monitor-1');
    expect(images[0].style.left).toBe('400px');
    expect(images[0].style.top).toBe('100px');
    expect(images[0].style.width).toBe('500px');
    expect(images[1].src).toBe('data:image/png;base64,frozen-monitor-left');
    expect(images[1].style.left).toBe('0px');
    expect(images[1].style.top).toBe('0px');
    expect(images[1].style.width).toBe('400px');
    expect(decode).toHaveBeenCalledTimes(2);
    expect(platform.window.reveal).not.toHaveBeenCalled();

    await act(async () => primaryDecoded.resolve());
    expect(platform.window.reveal).not.toHaveBeenCalled();
    await act(async () => secondaryDecoded.resolve());
    await vi.waitFor(() => expect(platform.window.reveal).toHaveBeenCalledOnce());
    expect(platform.window.prepareForReveal).toHaveBeenCalledOnce();

    await act(async () => root.unmount());
    container.remove();
  });

  it('replaces the disposed StrictMode runtime and cleans late work on unmount', async () => {
    vi.spyOn(HTMLImageElement.prototype, 'decode').mockResolvedValue(undefined);
    const oldSessionRequest = deferred<ReturnType<typeof createSession>>();
    const currentSessionRequest = deferred<ReturnType<typeof createSession>>();
    const lateHotkeyRegistration = deferred<() => void>();
    const disposeLateHotkey = vi.fn();
    const hotkeyHandlers: Array<
      Parameters<CaptureWorkspacePorts['events']['subscribeHotkeyTriggered']>[0]
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
    platform.events.subscribeHotkeyTriggered.mockImplementation((handler) => {
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
            ports: platform,
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
    expect(addEventListener.mock.calls).toContainEqual([
      'keydown',
      expect.any(Function),
      true,
    ]);
    expect(removeEventListener.mock.calls).toContainEqual([
      'keydown',
      expect.any(Function),
      true,
    ]);
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

  it('shows a frozen-image decode failure and lets Escape close the capture window', async () => {
    vi.spyOn(HTMLImageElement.prototype, 'decode')
      .mockRejectedValue(new Error('Frozen image could not be decoded'));
    const platform = createPlatform();
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(createElement(CaptureWorkspace, {
        ports: platform,
        initialMode: 'screenshot',
        initialSessionId: 'failed-image-session',
      }));
    });
    await vi.waitFor(() => expect(platform.window.reveal).toHaveBeenCalledOnce());
    expect(container.textContent).toContain('Frozen image could not be decoded');
    expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();

    await act(async () => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });
    await vi.waitFor(() => expect(platform.window.hide).toHaveBeenCalledOnce());

    await act(async () => root.unmount());
    container.remove();
  });

  it('handles a real keyboard copy shortcut with its modifier keys', async () => {
    vi.spyOn(HTMLImageElement.prototype, 'decode').mockResolvedValue(undefined);
    const platform = createPlatform();
    const session = createSession('keyboard-session');
    session.monitors[0].image_base64 = 'frozen-monitor';
    session.candidates = [{
      id: 'visible-window',
      kind: 'window',
      rect: { x: 20, y: 30, width: 300, height: 200 },
      priority: 1,
    }];
    platform.commands.getCaptureSession.mockResolvedValue(session);
    platform.commands.hydrateCaptureSessionSnapshots.mockResolvedValue(session);
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(createElement(CaptureWorkspace, {
        ports: platform,
        initialMode: 'screenshot',
        initialSessionId: 'keyboard-session',
      }));
    });
    await vi.waitFor(() => expect(platform.window.reveal).toHaveBeenCalledOnce());
    await act(async () => {
      container.firstElementChild?.dispatchEvent(new PointerEvent('pointermove', {
        bubbles: true,
        clientX: 100,
        clientY: 100,
      }));
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    });
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'c',
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }));
    });

    await vi.waitFor(() => expect(platform.commands.outputCapture).toHaveBeenCalledOnce());
    expect(platform.commands.outputCapture).toHaveBeenCalledWith(expect.objectContaining({
      action: { type: 'copy' },
      rect: { x: 20, y: 30, width: 300, height: 200 },
    }));
    await act(async () => root.unmount());
    container.remove();
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
        CaptureWorkspacePorts['commands']['getCaptureSession']
      >(async () => createSession('loaded')),
      hydrateCaptureSessionSnapshots: vi.fn(async (sessionId: string) => ({
        ...createSession(sessionId),
        monitors: createSession(sessionId).monitors.map((monitor) => ({
          ...monitor,
          image_base64: 'frozen-monitor',
        })),
      })),
      hydrateCaptureMonitorSnapshot: vi.fn(async (_sessionId, monitorId) => {
        const monitor = createSession('hydrated').monitors.find(
          (candidate) => candidate.id === monitorId,
        );
        if (!monitor) throw new Error(`Monitor not found: ${monitorId}`);
        return monitor;
      }),
      logCaptureFrontendPerf: vi.fn(async () => undefined),
      currentCaptureCursorPosition: vi.fn(async () => null),
      currentCaptureControlCandidate: vi.fn(async () => null),
      moveCaptureCursor: vi.fn(async () => undefined),
      cancelCaptureSession: vi.fn<
        CaptureWorkspacePorts['commands']['cancelCaptureSession']
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
    clipboard: { writeText: vi.fn(async () => undefined) },
    events: {
    subscribeCaptureCancel: vi.fn<
      CaptureWorkspacePorts['events']['subscribeCaptureCancel']
    >(async () => () => undefined),
    subscribeCaptureCopy: vi.fn<
      CaptureWorkspacePorts['events']['subscribeCaptureCopy']
    >(async () => () => undefined),
    subscribeCaptureSave: vi.fn<
      CaptureWorkspacePorts['events']['subscribeCaptureSave']
    >(async () => () => undefined),
    subscribeCaptureUndo: vi.fn<
      CaptureWorkspacePorts['events']['subscribeCaptureUndo']
    >(async () => () => undefined),
    subscribeCaptureRedo: vi.fn<
      CaptureWorkspacePorts['events']['subscribeCaptureRedo']
    >(async () => () => undefined),
    subscribeHotkeyTriggered: vi.fn<
      CaptureWorkspacePorts['events']['subscribeHotkeyTriggered']
    >(async () => () => undefined),
    },
    window: {
    prepareForReveal: vi.fn(async () => undefined),
    reveal: vi.fn(async () => undefined),
    hide: vi.fn(async () => undefined),
    },
    print: { printImage: vi.fn<CaptureWorkspacePorts['print']['printImage']>(async () => undefined) },
  } satisfies CaptureWorkspacePorts;
}

function createSession(id: string): CaptureSessionView {
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
