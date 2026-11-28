import { describe, expect, it, vi } from 'vitest';
import { prepareCaptureSurfaceForReveal } from './captureHostRuntime';

describe('capture surface DOM preparation', () => {
  it('does not wait for a hidden WebView animation frame before revealing', async () => {
    vi.useFakeTimers();
    const originalFrame = globalThis.requestAnimationFrame;
    globalThis.requestAnimationFrame = vi.fn(() => 1);
    try {
      let ready = false;
      const prepare = prepareCaptureSurfaceForReveal({
        windowPrepared: Promise.resolve(),
        frame: null,
        paintSelectionOverlayFrame: vi.fn(),
        flushStyles: vi.fn(),
      }).then(() => { ready = true; });
      await vi.advanceTimersByTimeAsync(0);
      const readyWithoutTimer = ready;
      await vi.runAllTimersAsync();
      await prepare;
      expect(readyWithoutTimer).toBe(true);
      expect(globalThis.requestAnimationFrame).not.toHaveBeenCalled();
    } finally {
      globalThis.requestAnimationFrame = originalFrame;
      vi.useRealTimers();
    }
  });

  it('paints selection as soon as native geometry is prepared', async () => {
    let finishWindow!: () => void;
    const windowPrepared = new Promise<void>((resolve) => { finishWindow = resolve; });
    const paint = vi.fn();
    const flushStyles = vi.fn();
    const prepare = prepareCaptureSurfaceForReveal({
      windowPrepared,
      frame: null,
      paintSelectionOverlayFrame: paint,
      flushStyles,
    });
    await Promise.resolve();
    expect(paint).not.toHaveBeenCalled();
    finishWindow();
    await prepare;
    expect(paint).toHaveBeenCalledExactlyOnceWith(null);
    expect(flushStyles).toHaveBeenCalledOnce();
    expect(paint.mock.invocationCallOrder[0]).toBeLessThan(flushStyles.mock.invocationCallOrder[0]);
  });

  it('rejects failed geometry preparation before revealing the surface', async () => {
    const paint = vi.fn();
    const flushStyles = vi.fn();
    await expect(prepareCaptureSurfaceForReveal({
      windowPrepared: Promise.reject(new Error('window unavailable')),
      frame: null,
      paintSelectionOverlayFrame: paint,
      flushStyles,
    })).rejects.toThrow('window unavailable');
    expect(paint).not.toHaveBeenCalled();
    expect(flushStyles).not.toHaveBeenCalled();
  });
});
