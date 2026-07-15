import { describe, expect, it } from 'vitest';
import {
  maskColorHex,
  maskColorOpacity,
  maskColorWithHex,
  maskColorWithOpacity,
} from './maskColor';

describe('screenshot mask color helpers', () => {
  it('changes RGB without losing opacity', () => {
    expect(maskColorWithHex([0, 0, 0, 46], '#2060ff')).toEqual([
      32, 96, 255, 46,
    ]);
    expect(maskColorHex([32, 96, 255, 46])).toBe('#2060ff');
  });

  it('changes opacity without losing RGB', () => {
    expect(maskColorWithOpacity([32, 96, 255, 46], 40)).toEqual([
      32, 96, 255, 102,
    ]);
    expect(maskColorOpacity([32, 96, 255, 102])).toBe(40);
  });
});
