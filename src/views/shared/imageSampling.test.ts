import { describe, expect, it, vi } from 'vitest';
import { sampleImageColor } from './imageSampling';

describe('browser image sampling adapter', () => {
  it('samples one physical image pixel through a reusable 1x1 canvas', () => {
    const context = {
      clearRect: vi.fn(),
      drawImage: vi.fn(),
      getImageData: vi.fn(() => ({
        data: new Uint8ClampedArray([10, 20, 30, 255]),
      })),
      imageSmoothingEnabled: true,
    };
    const canvas = {
      width: 1,
      height: 1,
      getContext: vi.fn(() => context),
    } as unknown as HTMLCanvasElement;
    const image = {
      naturalWidth: 1200,
      naturalHeight: 800,
    } as HTMLImageElement;

    expect(
      sampleImageColor(
        image,
        canvas,
        { x: 75, y: 25 },
        { width: 300, height: 200 },
      ),
    ).toEqual({ hex: '#0A141E', red: 10, green: 20, blue: 30 });
    expect(context.drawImage).toHaveBeenCalledWith(
      image,
      300,
      100,
      1,
      1,
      0,
      0,
      1,
      1,
    );
    expect(context.imageSmoothingEnabled).toBe(false);
  });
});
