import { describe, expect, it, vi } from 'vitest';
import {
  prepareCaptureSurfaceForReveal,
  waitForCaptureSurfacePaint,
} from './captureHostRuntime';

describe('capture surface DOM preparation', () => {
  it('waits for every image decode before painting and revealing the surface', async () => {
    let finishDecode!: () => void;
    const decode = new Promise<void>((resolve) => {
      finishDecode = resolve;
    });
    const paint = vi.fn();
    const waitForPaint = vi.fn(async () => undefined);
    const prepare = prepareCaptureSurfaceForReveal({
      images: [{ decode: async () => undefined }, { decode: () => decode }],
      frame: null,
      paintSelectionOverlayFrame: paint,
      waitForPaint,
    });
    await Promise.resolve();
    expect(paint).not.toHaveBeenCalled();
    finishDecode();
    await prepare;
    expect(paint).toHaveBeenCalledExactlyOnceWith(null);
    expect(waitForPaint).toHaveBeenCalledOnce();
  });

  it('rejects failed image decoding before preparing the reveal', async () => {
    const paint = vi.fn();
    await expect(
      prepareCaptureSurfaceForReveal({
        images: [{
          decode: async () => {
            throw new Error('decode failed');
          },
        }],
        frame: null,
        paintSelectionOverlayFrame: paint,
      }),
    ).rejects.toThrow('decode failed');
    expect(paint).not.toHaveBeenCalled();
  });

  it('decodes immediately but waits for native geometry before painting', async () => {
    let finishWindow!: () => void;
    const windowPrepared = new Promise<void>((resolve) => { finishWindow = resolve; });
    const decode = vi.fn(async () => undefined);
    const paint = vi.fn();
    const prepare = prepareCaptureSurfaceForReveal({
      images: [{ decode }],
      windowPrepared,
      frame: null,
      paintSelectionOverlayFrame: paint,
      waitForPaint: async () => undefined,
    });
    expect(decode).toHaveBeenCalledOnce();
    await Promise.resolve();
    expect(paint).not.toHaveBeenCalled();
    finishWindow();
    await prepare;
    expect(paint).toHaveBeenCalledOnce();
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
