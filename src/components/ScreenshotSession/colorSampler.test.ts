import { describe, expect, it } from 'vitest';
import {
  getImageSamplePoint,
  rgbaToHex,
} from './colorSampler';

describe('capture color sampler', () => {
  it('maps logical cursor coordinates into physical image pixels', () => {
    expect(
      getImageSamplePoint(
        { x: 75, y: 25 },
        { width: 300, height: 200 },
        { width: 1200, height: 800 },
      ),
    ).toEqual({ x: 300, y: 100 });
    expect(
      getImageSamplePoint(
        { x: 299.9, y: 199.9 },
        { width: 300, height: 200 },
        { width: 1200, height: 800 },
      ),
    ).toEqual({ x: 1199, y: 799 });
  });

  it('formats sampled rgba channels as uppercase hex', () => {
    expect(rgbaToHex(0, 15, 255)).toBe('#000FFF');
    expect(rgbaToHex(260, -10, 128)).toBe('#FF0080');
  });
});
