// @vitest-environment happy-dom

import { StrictMode, act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { CaptureWorkspacePorts } from '../../application/capture-workspace/ports';
import type { CaptureSessionView } from './types';
import CaptureWorkspace from './index';
import * as selectionOverlay from './captureSelectionOverlay';

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
      const preview = container.querySelector<HTMLCanvasElement>('[data-capture-preview]');
      expect(preview).not.toBeNull();
      expect(preview?.style.width).toBe('160px');
      expect(preview?.style.height).toBe('110px');
      expect(platform.commands.renderCaptureOutput).not.toHaveBeenCalled();
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

  it('keeps the native session through StrictMode reconnect and cancels once on unmount', async () => {
    vi.spyOn(HTMLImageElement.prototype, 'decode').mockResolvedValue(undefined);
    const request = deferred<ReturnType<typeof createSession>>();
    const lateRegistration = deferred<() => void>();
    const disposeLate = vi.fn();
    const platform = createPlatform();
    platform.commands.getCaptureSession.mockReturnValue(request.promise);
    platform.events.subscribeHotkeyTriggered.mockReturnValueOnce(lateRegistration.promise);
    const addEventListener = vi.spyOn(window, 'addEventListener');
    const removeEventListener = vi.spyOn(window, 'removeEventListener');
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);
    await act(async () => {
      root.render(createElement(StrictMode, null, createElement(CaptureWorkspace, {
        ports: platform,
        initialMode: 'screenshot',
        initialSessionId: 'strict-session',
      })));
    });
    expect(platform.commands.getCaptureSession).toHaveBeenCalledOnce();
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();
    await act(async () => {
      lateRegistration.resolve(disposeLate);
      request.resolve(createSession('strict-session'));
    });
    await vi.waitFor(() => expect(platform.window.reveal).toHaveBeenCalledOnce());
    expect(disposeLate).toHaveBeenCalledOnce();
    expect(platform.commands.cancelCaptureSession).not.toHaveBeenCalled();
    await act(async () => root.unmount());
    expect(platform.commands.cancelCaptureSession).toHaveBeenCalledExactlyOnceWith('strict-session');
    for (const [handler] of platform.events.subscribeHotkeyTriggered.mock.calls) {
      await handler({ mode: 'screenshot', sessionId: 'after-unmount' });
    }
    expect(platform.commands.getCaptureSession).toHaveBeenCalledOnce();
    for (const event of ['keydown', 'keyup', 'blur'] as const) {
      expect(listenerCalls(addEventListener, event)).toBe(listenerCalls(removeEventListener, event));
    }
    container.remove();
  });

  it('paints the latest dragged rectangle in the first animation frame', async () => {
    const frameQueue = new Map<number, FrameRequestCallback>();
    let nextFrame = 0;
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation((callback) => {
      frameQueue.set(++nextFrame, callback);
      return nextFrame;
    });
    vi.spyOn(window, 'cancelAnimationFrame').mockImplementation((id) => { frameQueue.delete(id); });
    vi.spyOn(HTMLImageElement.prototype, 'decode').mockResolvedValue(undefined);
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({ setTransform: vi.fn() } as never);
    const paint = vi.spyOn(selectionOverlay, 'drawCaptureSelectionOverlayFrame').mockImplementation(() => undefined);
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);
    const flushFrame = async () => {
      const callbacks = [...frameQueue.values()];
      frameQueue.clear();
      await act(async () => callbacks.forEach((callback) => callback(performance.now())));
    };
    try {
      await act(async () => root.render(createElement(CaptureWorkspace, {
        ports: createPlatform(), initialMode: 'screenshot', initialSessionId: 'drag-frame',
      })));
      for (let frame = 0; frame < 4; frame++) await flushFrame();
      const surface = container.firstElementChild as HTMLDivElement;
      surface.setPointerCapture = vi.fn();
      await act(async () => { surface.dispatchEvent(new PointerEvent('pointerdown', {
        bubbles: true, pointerId: 1, button: 0, buttons: 1, clientX: 20, clientY: 30,
      })); });
      await flushFrame();
      paint.mockClear();
      await act(async () => { surface.dispatchEvent(new PointerEvent('pointermove', {
        bubbles: true, pointerId: 1, button: 0, buttons: 1, clientX: 140, clientY: 110,
      })); });
      await flushFrame();
      expect(paint).toHaveBeenCalledWith(
        expect.anything(), expect.anything(),
        expect.objectContaining({ rect: { x: 20, y: 30, width: 120, height: 80 } }),
        expect.anything(),
      );
    } finally {
      await act(async () => root.unmount());
      container.remove();
    }
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

  it.each(['input', 'textarea', 'contenteditable'] as const)('leaves editing shortcuts to the focused %s', async (kind) => {
    vi.spyOn(HTMLImageElement.prototype, 'decode').mockResolvedValue(undefined);
    const platform = createPlatform();
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);
    try {
      await act(async () => root.render(createElement(CaptureWorkspace, {
        ports: platform, initialMode: 'screenshot', initialSessionId: 'editing-session',
      })));
      await vi.waitFor(() => expect(platform.window.reveal).toHaveBeenCalledOnce());
      await act(async () => { window.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'a', metaKey: true, bubbles: true, cancelable: true,
      })); });
      const input = document.createElement(kind === 'contenteditable' ? 'div' : kind);
      if (kind === 'contenteditable') input.contentEditable = 'true';
      container.firstElementChild?.append(input);
      for (const key of ['c', 's', 'z', 'y', 'a']) {
        const event = new KeyboardEvent('keydown', { key, metaKey: true, bubbles: true, cancelable: true });
        await act(async () => { input.dispatchEvent(event); });
        expect(event.defaultPrevented).toBe(false);
      }
      const composing = new KeyboardEvent('keydown', { key: 'c', metaKey: true, isComposing: true, cancelable: true });
      await act(async () => { window.dispatchEvent(composing); });
      expect(composing.defaultPrevented).toBe(false);
      expect(platform.commands.outputCapture).not.toHaveBeenCalled();
      expect(platform.commands.defaultCaptureSavePath).not.toHaveBeenCalled();
      expect(platform.window.hide).not.toHaveBeenCalled();
    } finally {
      await act(async () => root.unmount());
      container.remove();
    }
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
      prepareCaptureOcr: vi.fn(async () => undefined),
      completeCaptureOcr: vi.fn(async () => undefined),
      runCaptureOcr: vi.fn(async () => ({ text: '', confidence: null })),
      openCaptureOcrResultWindow: vi.fn(async () => undefined),
      openCaptureTranslationResultWindow: vi.fn(async () => undefined),
      copyTextToClipboard: vi.fn(async () => undefined),
    },
    clipboard: { writeText: vi.fn(async () => undefined) },
    events: {
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
