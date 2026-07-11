import { describe, expect, it, vi } from 'vitest';
import { createResultWindowPlatformRuntime } from './platformRuntime';

describe('result window platform runtime', () => {
  it('translates result window actions into portable window calls', async () => {
    const unsubscribe = vi.fn();
    const ports = {
      commands: {} as never,
      clipboard: { writeText: vi.fn(async () => undefined) },
      events: {
        subscribeResultPayloadReady: vi.fn(async () => unsubscribe),
      },
      window: {
        resize: vi.fn(async () => undefined),
        hide: vi.fn(async () => undefined),
        startDragging: vi.fn(async () => undefined),
      },
    };
    const runtime = createResultWindowPlatformRuntime(ports);
    const onPayloadReady = vi.fn();

    await expect(runtime.onPayloadReady(onPayloadReady)).resolves.toBe(
      unsubscribe,
    );
    await runtime.resizeTo(640, 480);
    await runtime.dismiss();
    await runtime.beginDrag();
    await runtime.clipboard.copyText('sample');

    expect(ports.events.subscribeResultPayloadReady).toHaveBeenCalledWith(
      onPayloadReady,
    );
    expect(ports.window.resize).toHaveBeenCalledWith(640, 480);
    expect(ports.window.hide).toHaveBeenCalledTimes(1);
    expect(ports.window.startDragging).toHaveBeenCalledTimes(1);
    expect(ports.clipboard.writeText).toHaveBeenCalledWith('sample');
  });
});
