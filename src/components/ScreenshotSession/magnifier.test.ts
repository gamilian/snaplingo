import { describe, expect, it } from 'vitest';
import {
  getMagnifierImageStyle,
  getMagnifierPosition,
  shouldAutoShowCaptureMagnifier,
  shouldShowMagnifier,
  shouldTrackCaptureCursorForMagnifier,
} from './magnifier';
import type { LogicalRect, Point } from './types';

const bounds: LogicalRect = { x: 0, y: 0, width: 300, height: 200 };

describe('capture magnifier', () => {
  it('positions near the cursor and flips away from capture edges', () => {
    const size = { width: 120, height: 96 };

    expect(getMagnifierPosition({ x: 40, y: 30 }, bounds, size, 12)).toEqual({
      x: 52,
      y: 42,
    });
    expect(getMagnifierPosition({ x: 260, y: 170 }, bounds, size, 12)).toEqual({
      x: 128,
      y: 62,
    });
  });

  it('centers the frozen image background on the cursor point', () => {
    const cursor: Point = { x: 30, y: 20 };

    expect(
      getMagnifierImageStyle(
        'frozen-image',
        cursor,
        { width: 300, height: 200 },
        { width: 120, height: 96 },
        4,
      ),
    ).toEqual({
      backgroundImage: 'url(data:image/png;base64,frozen-image)',
      backgroundSize: '1200px 800px',
      backgroundPosition: '-60px -32px',
      imageRendering: 'pixelated',
    });
  });

  it('only shows after the user requests it and all cursor context is available', () => {
    expect(
      shouldShowMagnifier({
        requested: true,
        automatic: false,
        hasCursorMonitor: true,
        hasViewportCursor: true,
        hasImageCursor: true,
        hasViewportBounds: true,
      }),
    ).toBe(true);
    expect(
      shouldShowMagnifier({
        requested: false,
        automatic: false,
        hasCursorMonitor: true,
        hasViewportCursor: true,
        hasImageCursor: true,
        hasViewportBounds: true,
      }),
    ).toBe(false);
    expect(
      shouldShowMagnifier({
        requested: true,
        automatic: false,
        hasCursorMonitor: false,
        hasViewportCursor: true,
        hasImageCursor: true,
        hasViewportBounds: true,
      }),
    ).toBe(false);
  });

  it('shows automatically when the capture interaction asks for it and context is available', () => {
    expect(
      shouldShowMagnifier({
        requested: false,
        automatic: true,
        hasCursorMonitor: true,
        hasViewportCursor: true,
        hasImageCursor: true,
        hasViewportBounds: true,
      }),
    ).toBe(true);
  });

  it('auto requests magnification while selecting after hydrated pixels are available', () => {
    expect(
      shouldAutoShowCaptureMagnifier({
        status: 'selecting',
        hasHydratedPixels: true,
      }),
    ).toBe(true);
    expect(
      shouldAutoShowCaptureMagnifier({
        status: 'selecting',
        hasHydratedPixels: false,
      }),
    ).toBe(false);
    expect(
      shouldAutoShowCaptureMagnifier({
        status: 'preview',
        hasHydratedPixels: true,
      }),
    ).toBe(false);
  });

  it('tracks the cursor while automatic magnification can be shown', () => {
    expect(
      shouldTrackCaptureCursorForMagnifier({
        status: 'selecting',
        requested: false,
        hasHydratedPixels: true,
      }),
    ).toBe(true);
    expect(
      shouldTrackCaptureCursorForMagnifier({
        status: 'selecting',
        requested: false,
        hasHydratedPixels: false,
      }),
    ).toBe(false);
    expect(
      shouldTrackCaptureCursorForMagnifier({
        status: 'selecting',
        requested: true,
        hasHydratedPixels: false,
      }),
    ).toBe(true);
    expect(
      shouldTrackCaptureCursorForMagnifier({
        status: 'preview',
        requested: false,
        hasHydratedPixels: false,
      }),
    ).toBe(true);
  });
});
