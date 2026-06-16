import { describe, expect, it } from 'vitest';
import {
  getPinnedDisplaySize,
  getPinnedOpacityFromWheel,
  getPinnedZoomFromWheel,
} from './pinControls';

describe('pinned image controls', () => {
  it('zooms pinned images with wheel direction and clamps the range', () => {
    expect(getPinnedZoomFromWheel(1, -1)).toBe(1.1);
    expect(getPinnedZoomFromWheel(1, 1)).toBe(0.9);
    expect(getPinnedZoomFromWheel(3.95, -1)).toBe(4);
    expect(getPinnedZoomFromWheel(0.3, 1)).toBe(0.25);
  });

  it('adjusts opacity with wheel direction and clamps the range', () => {
    expect(getPinnedOpacityFromWheel(0.8, -1)).toBe(0.85);
    expect(getPinnedOpacityFromWheel(0.8, 1)).toBe(0.75);
    expect(getPinnedOpacityFromWheel(0.98, -1)).toBe(1);
    expect(getPinnedOpacityFromWheel(0.22, 1)).toBe(0.2);
  });

  it('keeps pinned display size proportional to its initial fit', () => {
    expect(getPinnedDisplaySize({ width: 300, height: 200 }, 1)).toEqual({
      width: 300,
      height: 200,
    });
    expect(getPinnedDisplaySize({ width: 1800, height: 900 }, 1)).toEqual({
      width: 900,
      height: 450,
    });
    expect(getPinnedDisplaySize({ width: 1800, height: 900 }, 2)).toEqual({
      width: 1800,
      height: 900,
    });
  });
});
