import { describe, expect, it, vi } from 'vitest';
import {
  areRequiredPermissionsGranted,
  createRequiredPermissionsRuntime,
  type PermissionsPollingScheduler,
} from './runtime';

describe('required permissions runtime', () => {
  it('polls missing permissions without requesting them on startup', async () => {
    const scheduler = createScheduler();
    const status = vi
      .fn()
      .mockResolvedValueOnce({ screenRecording: false, accessibility: false })
      .mockResolvedValueOnce({ screenRecording: true, accessibility: true });
    const request = vi.fn();
    const runtime = createRequiredPermissionsRuntime(
      { status, request },
      scheduler.port,
    );
    const listener = vi.fn();

    const unsubscribe = runtime.subscribe(listener);
    await flushPromises();

    expect(status).toHaveBeenCalledOnce();
    expect(request).not.toHaveBeenCalled();
    expect(listener).toHaveBeenLastCalledWith({
      status: { screenRecording: false, accessibility: false },
      error: null,
    });
    expect(scheduler.delays).toEqual([750]);

    scheduler.runNext();
    await flushPromises();

    expect(status).toHaveBeenCalledTimes(2);
    expect(listener).toHaveBeenLastCalledWith({
      status: { screenRecording: true, accessibility: true },
      error: null,
    });

    unsubscribe();
  });

  it('requests the next permission only after an explicit action', async () => {
    const request = vi.fn(async () => ({
      screenRecording: true,
      accessibility: false,
    }));
    const runtime = createRequiredPermissionsRuntime({
      status: vi.fn(async () => ({
        screenRecording: false,
        accessibility: false,
      })),
      request,
    });

    const status = await runtime.requestNext();

    expect(request).toHaveBeenCalledOnce();
    expect(areRequiredPermissionsGranted(status)).toBe(false);
  });

  it('publishes polling errors and retries at the slower interval', async () => {
    const scheduler = createScheduler();
    const runtime = createRequiredPermissionsRuntime(
      {
        status: vi.fn().mockRejectedValue(new Error('status unavailable')),
        request: vi.fn(),
      },
      scheduler.port,
    );
    const listener = vi.fn();

    runtime.subscribe(listener);
    await flushPromises();

    expect(listener).toHaveBeenLastCalledWith({
      status: null,
      error: 'status unavailable',
    });
    expect(scheduler.delays).toEqual([1500]);
  });

  it('ignores an in-flight poll result after an explicit permission request', async () => {
    const scheduler = createScheduler();
    let resolvePoll: ((status: {
      screenRecording: boolean;
      accessibility: boolean;
    }) => void) | undefined;
    const status = vi.fn(
      () =>
        new Promise<{
          screenRecording: boolean;
          accessibility: boolean;
        }>((resolve) => {
          resolvePoll = resolve;
        }),
    );
    const request = vi.fn(async () => ({
      screenRecording: true,
      accessibility: true,
    }));
    const runtime = createRequiredPermissionsRuntime(
      { status, request },
      scheduler.port,
    );
    const listener = vi.fn();
    const unsubscribe = runtime.subscribe(listener);

    await runtime.requestNext();
    resolvePoll?.({ screenRecording: false, accessibility: false });
    await flushPromises();

    expect(listener).toHaveBeenLastCalledWith({
      status: { screenRecording: true, accessibility: true },
      error: null,
    });
    expect(scheduler.delays).toEqual([]);
    unsubscribe();
  });

  it('refreshes a previously granted status after permissions are revoked', async () => {
    const status = vi
      .fn()
      .mockResolvedValueOnce({ screenRecording: true, accessibility: true })
      .mockResolvedValueOnce({ screenRecording: false, accessibility: true });
    const runtime = createRequiredPermissionsRuntime({
      status,
      request: vi.fn(),
    });
    const listener = vi.fn();
    const unsubscribe = runtime.subscribe(listener);
    await flushPromises();

    await runtime.refresh();

    expect(status).toHaveBeenCalledTimes(2);
    expect(listener).toHaveBeenLastCalledWith({
      status: { screenRecording: false, accessibility: true },
      error: null,
    });
    unsubscribe();
  });
});

function createScheduler() {
  const scheduled: Array<() => void> = [];
  const delays: number[] = [];
  const port: PermissionsPollingScheduler = {
    schedule(callback, delayMs) {
      scheduled.push(callback);
      delays.push(delayMs);
      return callback;
    },
    cancel: vi.fn(),
  };

  return {
    port,
    delays,
    runNext() {
      scheduled.shift()?.();
    },
  };
}

async function flushPromises() {
  await Promise.resolve();
  await Promise.resolve();
}
