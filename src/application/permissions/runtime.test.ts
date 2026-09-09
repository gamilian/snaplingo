import { describe, expect, it, vi } from 'vitest';
import {
  areRequiredPermissionsGranted,
  createRequiredPermissionsRuntime,
  type PermissionsPollingScheduler,
  type RequiredPermissionsPort,
  type RequiredPermissionsStatus,
} from './runtime';

describe('required permissions runtime', () => {
  it('polls missing permissions without requesting them on startup', async () => {
    const scheduler = createScheduler();
    const status = vi
      .fn()
      .mockResolvedValueOnce({ screenRecording: false, accessibility: false })
      .mockResolvedValueOnce({ screenRecording: true, accessibility: false });
    const request = vi.fn();
    const runtime = createRequiredPermissionsRuntime(
      createPort({ status, request }),
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
      status: { screenRecording: true, accessibility: false },
      error: null,
    });

    unsubscribe();
  });

  it('requests only the selected permission after an explicit action', async () => {
    const request = vi.fn(async () => ({
      screenRecording: true,
      accessibility: false,
    }));
    const runtime = createRequiredPermissionsRuntime(createPort({
      status: vi.fn(async () => ({
        screenRecording: false,
        accessibility: false,
      })),
      request,
    }));

    const status = await runtime.request('screenRecording');

    expect(request).toHaveBeenCalledExactlyOnceWith('screenRecording');
    expect(areRequiredPermissionsGranted(status)).toBe(true);
  });

  it('publishes polling errors and retries at the slower interval', async () => {
    const scheduler = createScheduler();
    const runtime = createRequiredPermissionsRuntime(
      createPort({
        status: vi.fn().mockRejectedValue(new Error('status unavailable')),
        request: vi.fn(),
      }),
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
      createPort({ status, request }),
      scheduler.port,
    );
    const listener = vi.fn();
    const unsubscribe = runtime.subscribe(listener);

    await runtime.request('screenRecording');
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
    const runtime = createRequiredPermissionsRuntime(createPort({
      status,
      request: vi.fn(),
    }));
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

  it('deduplicates repair clicks and focus refreshes while a reset is in flight', async () => {
    let finish: (status: { screenRecording: boolean; accessibility: boolean }) => void = () => undefined;
    const scheduler = createScheduler();
    const port = createPort({
      status: vi.fn(async () => ({ screenRecording: false, accessibility: true })),
      reset: vi.fn(() => new Promise<RequiredPermissionsStatus>((resolve) => { finish = resolve; })),
    });
    const runtime = createRequiredPermissionsRuntime(port, scheduler.port);
    const listener = vi.fn();
    const unsubscribe = runtime.subscribe(listener);
    await flushPromises();
    const repair = runtime.reset('screenRecording');
    const duplicate = runtime.reset('screenRecording');
    const refresh = runtime.refresh();
    expect(port.reset).toHaveBeenCalledExactlyOnceWith('screenRecording');
    expect(port.status).toHaveBeenCalledOnce();
    finish({ screenRecording: true, accessibility: true });
    await Promise.all([repair, duplicate, refresh]);
    expect(listener).toHaveBeenLastCalledWith({ status: { screenRecording: true, accessibility: true }, error: null });
    expect(port.request).not.toHaveBeenCalled();
    unsubscribe();
  });
});

function createPort(overrides: Partial<RequiredPermissionsPort>): RequiredPermissionsPort {
  return {
    status: vi.fn(),
    request: vi.fn(),
    context: vi.fn(),
    reset: vi.fn(),
    restart: vi.fn(),
    ...overrides,
  };
}

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
