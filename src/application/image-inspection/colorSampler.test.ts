import { describe, expect, it, vi } from 'vitest';
import {
  colorSamplesEqual,
  colorSampleToClipboardText,
  getImageSamplePoint,
  isColorSampleCopyShortcut,
  isColorSampleFormatToggleShortcut,
  rgbaToHex,
  sampleImageColor,
} from './colorSampler';

describe('capture color sampler', () => {
  it('compares sampled colors by channels instead of object identity', () => {
    expect(colorSamplesEqual(null, null)).toBe(true);
    expect(
      colorSamplesEqual(
        { hex: '#0A141E', red: 10, green: 20, blue: 30 },
        { hex: '#0A141E', red: 10, green: 20, blue: 30 },
      ),
    ).toBe(true);
    expect(
      colorSamplesEqual(
        { hex: '#0A141E', red: 10, green: 20, blue: 30 },
        { hex: '#0A141F', red: 10, green: 20, blue: 31 },
      ),
    ).toBe(false);
  });

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

  it('copies sampled colors as rgb text when requested', () => {
    expect(
      colorSampleToClipboardText(
        {
          hex: '#0A141E',
          red: 10,
          green: 20,
          blue: 30,
        },
        'rgb',
      ),
    ).toBe('rgb(10, 20, 30)');
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
    expect(
      isColorSampleCopyShortcut({
        key: 'c',
        metaKey: false,
        ctrlKey: false,
        altKey: true,
      }),
    ).toBe(true);
  });

  it('uses plain Shift for toggling sampled color format', () => {
    expect(
      isColorSampleFormatToggleShortcut({
        key: 'Shift',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe(true);
    expect(
      isColorSampleFormatToggleShortcut({
        key: 'Shift',
        metaKey: false,
        ctrlKey: false,
        altKey: true,
      }),
    ).toBe(true);
  });
});
