import { describe, expect, it } from 'vitest';
import { blurRgbaPixels } from './blur';

describe('software blur preview', () => {
  it('smooths contrast, preserves opacity, and leaves the supplied pixels intact', () => {
    const pixels = new Uint8ClampedArray(20 * 10 * 4);
    for (let y = 0; y < 10; y += 1) {
      for (let x = 0; x < 20; x += 1)
        pixels.set(
          [x < 10 ? 0 : 255, x < 10 ? 0 : 255, x < 10 ? 0 : 255, 255],
          (y * 20 + x) * 4,
        );
    }
    const output = blurRgbaPixels(pixels, 20, 10, 2);
    expect(output[(5 * 20 + 9) * 4]).toBeGreaterThan(60);
    expect(output[(5 * 20 + 10) * 4]).toBeLessThan(200);
    expect(output[0]).toBe(0);
    expect(output[(20 * 10 - 1) * 4]).toBe(255);
    expect(
      output
        .filter((_, index) => index % 4 === 3)
        .every((alpha) => alpha === 255),
    ).toBe(true);
    expect(pixels[(5 * 20 + 9) * 4]).toBe(0);
  });

  it('clamps samples to the image edges, including images narrower than the blur kernel', () => {
    const color = [70, 120, 200, 255];
    const pixels = new Uint8ClampedArray([...color, ...color]);
    expect(blurRgbaPixels(pixels, 1, 2, 40)).toEqual(pixels);
  });
});
