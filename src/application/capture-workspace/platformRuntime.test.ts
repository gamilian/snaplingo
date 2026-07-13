import { describe, expect, it, vi } from 'vitest';
import { createCaptureWorkspacePlatformRuntime } from './platformRuntime';

describe('capture workspace platform runtime', () => {
  it('translates capture workspace actions into portable event and window calls', async () => {
    const cancelUnsubscribe = vi.fn();
    const copyUnsubscribe = vi.fn();
    const undoUnsubscribe = vi.fn();
    const redoUnsubscribe = vi.fn();
    const hotkeyUnsubscribe = vi.fn();
    const ports = {
      commands: {} as never,
      clipboard: { writeText: vi.fn(async () => undefined) },
      events: {
        subscribeCaptureCancel: vi.fn(async () => cancelUnsubscribe),
        subscribeCaptureCopy: vi.fn(async () => copyUnsubscribe),
        subscribeCaptureUndo: vi.fn(async () => undoUnsubscribe),
        subscribeCaptureRedo: vi.fn(async () => redoUnsubscribe),
        subscribeHotkeyTriggered: vi.fn(async () => hotkeyUnsubscribe),
      },
      window: {
        prepareForReveal: vi.fn(async () => undefined),
        reveal: vi.fn(async () => undefined),
        hide: vi.fn(async () => undefined),
      },
    };
    const runtime = createCaptureWorkspacePlatformRuntime(ports);
    const onCancel = vi.fn();
    const onCopy = vi.fn();
    const onUndo = vi.fn();
    const onRedo = vi.fn();
    const onHotkey = vi.fn();

    await expect(runtime.onCancelRequested(onCancel)).resolves.toBe(
      cancelUnsubscribe,
    );
    await expect(runtime.onCopyRequested(onCopy)).resolves.toBe(
      copyUnsubscribe,
    );
    await expect(runtime.onUndoRequested(onUndo)).resolves.toBe(
      undoUnsubscribe,
    );
    await expect(runtime.onRedoRequested(onRedo)).resolves.toBe(
      redoUnsubscribe,
    );
    await expect(runtime.onHotkeyTriggered(onHotkey)).resolves.toBe(
      hotkeyUnsubscribe,
    );
    await runtime.prepareForReveal();
    await runtime.reveal();
    await runtime.dismiss();
    await runtime.clipboard.copyText('sample');

    expect(ports.events.subscribeCaptureCancel).toHaveBeenCalledWith(onCancel);
    expect(ports.events.subscribeCaptureCopy).toHaveBeenCalledWith(onCopy);
    expect(ports.events.subscribeCaptureUndo).toHaveBeenCalledWith(onUndo);
    expect(ports.events.subscribeCaptureRedo).toHaveBeenCalledWith(onRedo);
    expect(ports.events.subscribeHotkeyTriggered).toHaveBeenCalledWith(onHotkey);
    expect(ports.window.prepareForReveal).toHaveBeenCalledTimes(1);
    expect(ports.window.reveal).toHaveBeenCalledTimes(1);
    expect(ports.window.hide).toHaveBeenCalledTimes(1);
    expect(ports.clipboard.writeText).toHaveBeenCalledWith('sample');
  });
});
