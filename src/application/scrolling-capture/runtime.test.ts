import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ScrollingCaptureView } from '../../domain/scrollingCapture';
import type { ScrollingCapturePorts } from './ports';
import { createScrollingCaptureRuntime } from './runtime';

function view(
  revision = 0,
  overrides: Partial<ScrollingCaptureView> = {},
): ScrollingCaptureView {
  return {
    id: 'scrolling-1',
    revision,
    width: 300,
    height: 200,
    can_undo: false,
    preview_base64: 'preview',
    status: 'capturing',
    hint: null,
    shortcut_actions: ['copy', 'undo', 'cancel'],
    preview_expanded: true,
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((settle) => {
    resolve = settle;
  });
  return { promise, resolve };
}

function fixture() {
  let update!: (next: ScrollingCaptureView) => void;
  let error!: (message: string) => void;
  const cleanupUpdates = vi.fn();
  const cleanupErrors = vi.fn();
  const ports = {
    commands: {
      get: vi.fn(async () => view()),
      capture: vi.fn(async () => view(1)),
      undo: vi.fn(async () => view(2)),
      finish: vi.fn(async () => undefined),
      cancel: vi.fn(async () => undefined),
      saveAction: vi.fn<ScrollingCapturePorts['commands']['saveAction']>(async () => ({
        type: 'save' as const,
        path: '/capture.png',
        format: 'png' as const,
        quality: 90,
        copyAfterSave: false,
      })),
    },
    events: {
      subscribeUpdates: vi.fn<
        ScrollingCapturePorts['events']['subscribeUpdates']
      >(async (handler) => {
        update = handler;
        return cleanupUpdates;
      }),
      subscribeErrors: vi.fn<
        ScrollingCapturePorts['events']['subscribeErrors']
      >(async (handler) => {
        error = handler;
        return cleanupErrors;
      }),
    },
    window: {
      resizePreview: vi.fn(async () => undefined),
      startDragging: vi.fn(async () => undefined),
    },
  } satisfies ScrollingCapturePorts;
  const runtime = createScrollingCaptureRuntime('scrolling-1', ports);
  return {
    runtime,
    ports,
    cleanupUpdates,
    cleanupErrors,
    update: (next: ScrollingCaptureView) => update(next),
    error: (message: string) => error(message),
  };
}

afterEach(() => {
  vi.useRealTimers();
});

describe('scrolling capture runtime', () => {
  it('watches preview events without starting another frame sampler', async () => {
    vi.useFakeTimers();
    const f = fixture();
    const preview = createScrollingCaptureRuntime('scrolling-1', f.ports, { sampling: false });
    await preview.mount();
    await vi.advanceTimersByTimeAsync(3000);
    expect(f.ports.commands.capture).not.toHaveBeenCalled();
    f.update(view(4, { height: 800 }));
    expect(preview.getSnapshot().view?.height).toBe(800);
    preview.dispose();
  });
  it('stops sampling after native automatic completion', async () => {
    vi.useFakeTimers();
    const f = fixture();
    f.ports.commands.capture.mockResolvedValueOnce(view(1, { status: 'completed' }));
    await f.runtime.mount();
    await vi.advanceTimersByTimeAsync(3000);
    expect(f.ports.commands.capture).toHaveBeenCalledOnce();
    expect(f.runtime.getSnapshot().finished).toBe(true);
    f.runtime.dispose();
  });
  it('keeps the editor open when a save dialog is cancelled and still finishes a completed retry', async () => {
    vi.useFakeTimers();
    const f = fixture();
    await f.runtime.mount();
    f.ports.commands.saveAction.mockResolvedValueOnce(null);
    await f.runtime.save();
    expect(f.ports.commands.finish).not.toHaveBeenCalled();
    expect(f.runtime.getSnapshot().finished).toBe(false);
    f.ports.commands.capture.mockResolvedValueOnce(view(3, { status: 'completed' }));
    await f.runtime.resume();
    expect(f.runtime.getSnapshot().finished).toBe(true);
    f.runtime.dispose();
  });
  it('samples serially and only queues a new frame after the previous request settles', async () => {
    vi.useFakeTimers();
    const { runtime, ports } = fixture();
    const pending = deferred<ScrollingCaptureView>();
    ports.commands.capture.mockReturnValueOnce(pending.promise);
    await runtime.mount();
    await vi.advanceTimersByTimeAsync(1250);
    expect(ports.commands.capture).toHaveBeenCalledTimes(1);
    pending.resolve(view(1, { height: 260 }));
    await vi.advanceTimersByTimeAsync(0);
    expect(runtime.getSnapshot().view?.height).toBe(260);
    await vi.advanceTimersByTimeAsync(249);
    expect(ports.commands.capture).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(ports.commands.capture).toHaveBeenCalledTimes(2);
    runtime.dispose();
  });

  it('rejects updates for another capture and responses older than an event revision', async () => {
    vi.useFakeTimers();
    const f = fixture();
    const pending = deferred<ScrollingCaptureView>();
    f.ports.commands.capture.mockReturnValueOnce(pending.promise);
    await f.runtime.mount();
    await vi.advanceTimersByTimeAsync(250);
    f.update(view(9, { height: 450 }));
    f.update(view(20, { id: 'another-capture', height: 1000 }));
    pending.resolve(view(1, { height: 250 }));
    await vi.advanceTimersByTimeAsync(0);
    expect(f.runtime.getSnapshot().view).toMatchObject({
      revision: 9,
      height: 450,
    });
    f.runtime.dispose();
  });

  it('waits for an active sample before finishing and stops polling afterward', async () => {
    vi.useFakeTimers();
    const { runtime, ports } = fixture();
    const pending = deferred<ScrollingCaptureView>();
    ports.commands.capture.mockReturnValueOnce(pending.promise);
    await runtime.mount();
    await vi.advanceTimersByTimeAsync(250);
    const finishing = runtime.finish({ type: 'pin' });
    await runtime.finish({ type: 'copy' });
    expect(ports.commands.finish).not.toHaveBeenCalled();
    pending.resolve(view(1));
    await finishing;
    expect(ports.commands.finish).toHaveBeenCalledExactlyOnceWith(
      'scrolling-1',
      { type: 'pin' },
    );
    expect(runtime.getSnapshot()).toMatchObject({
      working: false,
      finished: true,
    });
    await vi.advanceTimersByTimeAsync(1000);
    expect(ports.commands.capture).toHaveBeenCalledTimes(1);
    runtime.dispose();
  });

  it('preserves the preview after output failure and allows saving again', async () => {
    vi.useFakeTimers();
    const { runtime, ports } = fixture();
    ports.commands.finish.mockRejectedValueOnce(new Error('Disk full'));
    await runtime.mount();
    await runtime.save();
    expect(runtime.getSnapshot()).toMatchObject({
      finished: false,
      working: false,
      error: 'Error: Disk full',
      view: { preview_base64: 'preview' },
    });
    await vi.advanceTimersByTimeAsync(1000);
    expect(ports.commands.capture).not.toHaveBeenCalled();
    await runtime.save();
    expect(runtime.getSnapshot().finished).toBe(true);
    expect(ports.commands.finish).toHaveBeenLastCalledWith('scrolling-1', {
      type: 'save',
      path: '/capture.png',
      format: 'png',
      quality: 90,
      copyAfterSave: false,
    });
    runtime.dispose();
  });

  it('can retry an error received during an active sample without overlapping requests', async () => {
    vi.useFakeTimers();
    const f = fixture();
    const pending = deferred<ScrollingCaptureView>();
    f.ports.commands.capture
      .mockReturnValueOnce(pending.promise)
      .mockResolvedValue(view(3));
    await f.runtime.mount();
    await vi.advanceTimersByTimeAsync(250);
    f.error('Display unavailable');
    const retry = f.runtime.resume();
    expect(f.ports.commands.capture).toHaveBeenCalledTimes(1);
    pending.resolve(view(1));
    await retry;
    await vi.advanceTimersByTimeAsync(0);
    expect(f.ports.commands.capture).toHaveBeenCalledTimes(2);
    expect(f.runtime.getSnapshot()).toMatchObject({
      error: null,
      view: { revision: 3 },
    });
    f.runtime.dispose();
  });

  it('releases a subscription that resolves after disposal and avoids further native requests', async () => {
    const f = fixture();
    const pending = deferred<() => void>();
    f.ports.events.subscribeUpdates.mockReturnValueOnce(pending.promise);
    const mounting = f.runtime.mount();
    f.runtime.dispose();
    pending.resolve(f.cleanupUpdates);
    await mounting;
    f.runtime.dispose();
    expect(f.cleanupUpdates).toHaveBeenCalledOnce();
    expect(f.ports.events.subscribeErrors).not.toHaveBeenCalled();
    expect(f.ports.commands.get).not.toHaveBeenCalled();
  });

  it('keeps subscriptions during undo and ignores a late response after disposal', async () => {
    vi.useFakeTimers();
    const f = fixture();
    const pending = deferred<ScrollingCaptureView>();
    f.ports.commands.undo.mockReturnValueOnce(pending.promise);
    await f.runtime.mount();
    const undoing = f.runtime.undo();
    await vi.advanceTimersByTimeAsync(0);
    const listener = vi.fn();
    f.runtime.subscribe(listener);
    f.runtime.dispose();
    pending.resolve(view(10));
    await undoing;
    expect(listener).not.toHaveBeenCalled();
    expect(f.cleanupUpdates).toHaveBeenCalledOnce();
    expect(f.cleanupErrors).toHaveBeenCalledOnce();
  });

  it('reports a failed preview resize while retaining the existing preview', async () => {
    vi.useFakeTimers();
    const { runtime, ports } = fixture();
    await runtime.mount();
    ports.window.resizePreview.mockRejectedValueOnce(
      new Error('Resize failed'),
    );
    await runtime.togglePreview();
    expect(runtime.getSnapshot()).toMatchObject({
      previewExpanded: true,
      error: 'Error: Resize failed',
    });
    runtime.dispose();
  });
});
