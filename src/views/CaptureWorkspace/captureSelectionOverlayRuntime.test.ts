import { describe, expect, it } from 'vitest';
import {
  getSelectionOverlayCanvasSize,
  getSelectionOverlayPixelRatio,
} from './captureSelectionOverlayRuntime';

describe('captureSelectionOverlayRuntime', () => {
  it('derives a non-negative integer canvas size from viewport bounds', () => {
    expect(
      getSelectionOverlayCanvasSize({
        x: 0,
        y: 0,
        width: 240.4,
        height: 120.6,
      }),
    ).toEqual({
      width: 240,
      height: 121,
    });

    expect(
      getSelectionOverlayCanvasSize({
        x: 0,
        y: 0,
        width: -2,
        height: -5,
      }),
    ).toEqual({
      width: 0,
      height: 0,
    });
  });

  it('keeps selection overlay pixel ratio safe across host environments', () => {
    expect(getSelectionOverlayPixelRatio({ devicePixelRatio: 2 })).toBe(2);
    expect(getSelectionOverlayPixelRatio({ devicePixelRatio: 0 })).toBe(1);
    expect(getSelectionOverlayPixelRatio(undefined)).toBe(1);
  });
});
