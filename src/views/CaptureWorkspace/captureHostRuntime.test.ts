import { describe, expect, it, vi } from 'vitest';
import {
  prepareCaptureSurfaceForReveal,
  waitForCaptureSurfacePaint,
} from './captureHostRuntime';

describe('capture surface DOM preparation', () => {
  it('paints selection as soon as native geometry is prepared', async () => {
    let finishWindow!: () => void;
    const windowPrepared = new Promise<void>((resolve) => { finishWindow = resolve; });
    const paint = vi.fn();
    const waitForPaint = vi.fn(async () => undefined);
    const prepare = prepareCaptureSurfaceForReveal({
      windowPrepared,
      frame: null,
      paintSelectionOverlayFrame: paint,
      waitForPaint,
    });
    await Promise.resolve();
    expect(paint).not.toHaveBeenCalled();
    finishWindow();
    await prepare;
    expect(paint).toHaveBeenCalledExactlyOnceWith(null);
    expect(waitForPaint).toHaveBeenCalledOnce();
  });

  it('rejects failed geometry preparation before revealing the surface', async () => {
    const paint = vi.fn();
    await expect(prepareCaptureSurfaceForReveal({
      windowPrepared: Promise.reject(new Error('window unavailable')),
      frame: null,
      paintSelectionOverlayFrame: paint,
    })).rejects.toThrow('window unavailable');
    expect(paint).not.toHaveBeenCalled();
  });

  it('waits for one animation frame before fading in the capture overlay', async () => {
    const calls: string[] = [];
    const pendingFrameCallbacks: FrameRequestCallback[] = [];
    const requestAnimationFrame = (callback: FrameRequestCallback) => {
      calls.push('requestAnimationFrame');
      pendingFrameCallbacks.push(callback);
      return pendingFrameCallbacks.length;
    };

    const wait = waitForCaptureSurfacePaint(requestAnimationFrame);
    expect(calls).toEqual(['requestAnimationFrame']);

    pendingFrameCallbacks.shift()?.(1);
    await wait;
    expect(calls).toEqual(['requestAnimationFrame']);
  });
});
