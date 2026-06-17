import { describe, expect, it } from 'vitest';
import {
  getPinnedContextMenuPosition,
  getPinnedDisplaySize,
  getPinnedKeyboardZoomAction,
  getPinnedOpacityFromWheel,
  getPinnedOpacityPreset,
  getPinnedZoomFromWheel,
} from './pinControls';

describe('pinned image controls', () => {
  it('zooms pinned images with wheel direction and clamps the range', () => {
    expect(getPinnedZoomFromWheel(1, -1)).toBe(1.1);
    expect(getPinnedZoomFromWheel(1, 1)).toBe(0.9);
    expect(getPinnedZoomFromWheel(3.95, -1)).toBe(4);
    expect(getPinnedZoomFromWheel(0.3, 1)).toBe(0.25);
  });

  it('maps keyboard shortcuts to pinned zoom actions', () => {
    expect(getPinnedKeyboardZoomAction({ key: '+', metaKey: false, ctrlKey: false })).toBe(
      'zoom-in',
    );
    expect(getPinnedKeyboardZoomAction({ key: '=', metaKey: false, ctrlKey: false })).toBe(
      'zoom-in',
    );
    expect(getPinnedKeyboardZoomAction({ key: '-', metaKey: false, ctrlKey: false })).toBe(
      'zoom-out',
    );
    expect(getPinnedKeyboardZoomAction({ key: '0', metaKey: false, ctrlKey: false })).toBe(
      'reset',
    );
    expect(getPinnedKeyboardZoomAction({ key: '+', metaKey: true, ctrlKey: false })).toBeNull();
    expect(getPinnedKeyboardZoomAction({ key: 'x', metaKey: false, ctrlKey: false })).toBeNull();
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

  it('keeps context menus inside the pinned window', () => {
    expect(
      getPinnedContextMenuPosition(
        { x: 240, y: 160 },
        { width: 96, height: 120 },
        { width: 300, height: 220 },
      ),
    ).toEqual({ x: 204, y: 100 });

    expect(
      getPinnedContextMenuPosition(
        { x: -10, y: -20 },
        { width: 96, height: 120 },
        { width: 300, height: 220 },
      ),
    ).toEqual({ x: 0, y: 0 });
  });

  it('normalizes pinned opacity menu presets', () => {
    expect(getPinnedOpacityPreset(1)).toBe(1);
    expect(getPinnedOpacityPreset(0.75)).toBe(0.75);
    expect(getPinnedOpacityPreset(1.4)).toBe(1);
    expect(getPinnedOpacityPreset(0.05)).toBe(0.2);
  });
});
