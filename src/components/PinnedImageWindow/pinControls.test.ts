import { describe, expect, it } from 'vitest';
import {
  getPinnedContextMenuPosition,
  getPinnedDisplaySize,
  getPinnedDisplaySizeForTransform,
  getPinnedKeyboardOpacityAction,
  getPinnedKeyboardTransformAction,
  getPinnedKeyboardZoomAction,
  getPinnedOpacityFromWheel,
  getPinnedOpacityPreset,
  getPinnedTransformStyle,
  getPinnedWheelAction,
  getPinnedZoomFromWheel,
  nextPinnedTransform,
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

  it('maps Snipaste keyboard shortcuts to pinned opacity actions', () => {
    expect(
      getPinnedKeyboardOpacityAction({ key: '+', metaKey: true, ctrlKey: false }),
    ).toBe('increase');
    expect(
      getPinnedKeyboardOpacityAction({ key: '=', metaKey: false, ctrlKey: true }),
    ).toBe('increase');
    expect(
      getPinnedKeyboardOpacityAction({ key: '-', metaKey: false, ctrlKey: true }),
    ).toBe('decrease');
    expect(
      getPinnedKeyboardOpacityAction({ key: '+', metaKey: false, ctrlKey: false }),
    ).toBeNull();
    expect(
      getPinnedKeyboardOpacityAction({
        key: '+',
        metaKey: true,
        ctrlKey: false,
        altKey: true,
      }),
    ).toBeNull();
  });

  it('maps number keys to Snipaste pinned image transform actions', () => {
    expect(
      getPinnedKeyboardTransformAction({ key: '1', metaKey: false, ctrlKey: false }),
    ).toBe('rotate-clockwise');
    expect(
      getPinnedKeyboardTransformAction({ key: '2', metaKey: false, ctrlKey: false }),
    ).toBe('rotate-counterclockwise');
    expect(
      getPinnedKeyboardTransformAction({ key: '3', metaKey: false, ctrlKey: false }),
    ).toBe('flip-horizontal');
    expect(
      getPinnedKeyboardTransformAction({ key: '4', metaKey: false, ctrlKey: false }),
    ).toBe('flip-vertical');
    expect(
      getPinnedKeyboardTransformAction({ key: '1', metaKey: true, ctrlKey: false }),
    ).toBeNull();
    expect(
      getPinnedKeyboardTransformAction({
        key: '1',
        metaKey: false,
        ctrlKey: false,
        altKey: true,
      }),
    ).toBeNull();
    expect(
      getPinnedKeyboardTransformAction({
        key: '1',
        metaKey: false,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBeNull();
    expect(
      getPinnedKeyboardTransformAction({ key: '5', metaKey: false, ctrlKey: false }),
    ).toBeNull();
  });

  it('updates pinned image transform state from Snipaste actions', () => {
    expect(
      nextPinnedTransform(
        { rotation: 0, flipX: false, flipY: false },
        'rotate-clockwise',
      ),
    ).toEqual({ rotation: 90, flipX: false, flipY: false });
    expect(
      nextPinnedTransform(
        { rotation: 0, flipX: false, flipY: false },
        'rotate-counterclockwise',
      ),
    ).toEqual({ rotation: 270, flipX: false, flipY: false });
    expect(
      nextPinnedTransform(
        { rotation: 90, flipX: false, flipY: true },
        'flip-horizontal',
      ),
    ).toEqual({ rotation: 90, flipX: true, flipY: true });
    expect(
      nextPinnedTransform(
        { rotation: 90, flipX: true, flipY: true },
        'flip-vertical',
      ),
    ).toEqual({ rotation: 90, flipX: true, flipY: false });
  });

  it('formats pinned image transforms for CSS rendering', () => {
    expect(
      getPinnedTransformStyle({ rotation: 90, flipX: true, flipY: false }),
    ).toBe('rotate(90deg) scale(-1, 1)');
  });

  it('adjusts opacity with wheel direction and clamps the range', () => {
    expect(getPinnedOpacityFromWheel(0.8, -1)).toBe(0.85);
    expect(getPinnedOpacityFromWheel(0.8, 1)).toBe(0.75);
    expect(getPinnedOpacityFromWheel(0.98, -1)).toBe(1);
    expect(getPinnedOpacityFromWheel(0.22, 1)).toBe(0.2);
  });

  it('uses Snipaste wheel modifiers for pinned zoom and opacity', () => {
    expect(
      getPinnedWheelAction({
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe('zoom');
    expect(
      getPinnedWheelAction({
        metaKey: true,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe('opacity');
    expect(
      getPinnedWheelAction({
        metaKey: false,
        ctrlKey: true,
        altKey: false,
      }),
    ).toBe('opacity');
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

  it('swaps pinned display size for quarter-turn rotations', () => {
    expect(
      getPinnedDisplaySizeForTransform(
        { width: 300, height: 200 },
        1,
        { rotation: 90, flipX: false, flipY: false },
      ),
    ).toEqual({ width: 200, height: 300 });

    expect(
      getPinnedDisplaySizeForTransform(
        { width: 300, height: 200 },
        1,
        { rotation: 270, flipX: true, flipY: false },
      ),
    ).toEqual({ width: 200, height: 300 });

    expect(
      getPinnedDisplaySizeForTransform(
        { width: 300, height: 200 },
        1,
        { rotation: 180, flipX: true, flipY: true },
      ),
    ).toEqual({ width: 300, height: 200 });
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
