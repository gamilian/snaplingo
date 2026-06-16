import { describe, expect, it } from 'vitest';
import {
  colorSampleToClipboardText,
  getImageSamplePoint,
  isColorSampleCopyShortcut,
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

  it('copies sampled colors as hex text', () => {
    expect(
      colorSampleToClipboardText({
        hex: '#0A141E',
        red: 10,
        green: 20,
        blue: 30,
      }),
    ).toBe('#0A141E');
  });

  it('uses plain C for copying the sampled color without replacing selection copy', () => {
    expect(
      isColorSampleCopyShortcut({
        key: 'c',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe(true);
    expect(
      isColorSampleCopyShortcut({
        key: 'c',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe(false);
  });
});
