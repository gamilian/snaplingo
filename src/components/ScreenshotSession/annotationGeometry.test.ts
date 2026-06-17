import { describe, expect, it } from 'vitest';
import {
  constrainAnnotationMoveDelta,
  getAnnotationBounds,
  getAnnotationKeyboardNudgeDelta,
  hitTestAnnotations,
  moveAnnotationByDelta,
} from './annotationGeometry';
import type { AnnotationCommand, RectangleAnnotationCommand } from './types';

const rectangle: RectangleAnnotationCommand = {
  type: 'rectangle',
  rect: { x: 10, y: 20, width: 30, height: 40 },
  color: [255, 77, 79, 255],
  stroke_width: 2,
  filled: false,
};

const arrow: AnnotationCommand = {
  type: 'arrow',
  start: { x: 80, y: 20 },
  end: { x: 120, y: 60 },
  color: [255, 77, 79, 255],
  stroke_width: 4,
};

const text: AnnotationCommand = {
  type: 'text',
  position: { x: 12, y: 90 },
  text: 'Snap',
  color: [255, 255, 255, 255],
  font_size: 20,
};

const blur: AnnotationCommand = {
  type: 'blur',
  rect: { x: 30, y: 40, width: 20, height: 10 },
  radius: 6,
};

describe('annotation geometry', () => {
  it('bounds rectangular and endpoint annotations', () => {
    expect(getAnnotationBounds(rectangle)).toEqual({
      x: 10,
      y: 20,
      width: 30,
      height: 40,
    });
    expect(getAnnotationBounds(arrow)).toEqual({
      x: 78,
      y: 18,
      width: 44,
      height: 44,
    });
    expect(getAnnotationBounds(blur)).toEqual({
      x: 30,
      y: 40,
      width: 20,
      height: 10,
    });
  });

  it('bounds text annotations from the baseline position', () => {
    expect(getAnnotationBounds(text)).toEqual({
      x: 12,
      y: 66,
      width: 48,
      height: 24,
    });
  });

  it('hit tests annotations from topmost to bottommost', () => {
    expect(
      hitTestAnnotations(
        [
          rectangle,
          {
            ...rectangle,
            rect: { x: 20, y: 30, width: 30, height: 40 },
          },
        ],
        { x: 25, y: 35 },
      ),
    ).toBe(1);
  });

  it('uses tolerance when hit testing thin annotations', () => {
    expect(hitTestAnnotations([arrow], { x: 79, y: 19 }, 2)).toBe(0);
    expect(hitTestAnnotations([arrow], { x: 75, y: 15 }, 2)).toBeNull();
  });

  it('moves annotations by delta without changing their style', () => {
    expect(moveAnnotationByDelta(rectangle, { x: 4, y: -6 })).toEqual({
      ...rectangle,
      rect: { x: 14, y: 14, width: 30, height: 40 },
    });
    expect(moveAnnotationByDelta(arrow, { x: 4, y: -6 })).toEqual({
      ...arrow,
      start: { x: 84, y: 14 },
      end: { x: 124, y: 54 },
    });
    expect(moveAnnotationByDelta(text, { x: 4, y: -6 })).toEqual({
      ...text,
      position: { x: 16, y: 84 },
    });
    expect(moveAnnotationByDelta(blur, { x: 4, y: -6 })).toEqual({
      ...blur,
      rect: { x: 34, y: 34, width: 20, height: 10 },
    });
  });

  it('maps arrow keys to annotation nudge deltas', () => {
    expect(getAnnotationKeyboardNudgeDelta('ArrowLeft', 1)).toEqual({ x: -1, y: 0 });
    expect(getAnnotationKeyboardNudgeDelta('ArrowRight', 10)).toEqual({
      x: 10,
      y: 0,
    });
    expect(getAnnotationKeyboardNudgeDelta('ArrowUp', 1)).toEqual({ x: 0, y: -1 });
    expect(getAnnotationKeyboardNudgeDelta('ArrowDown', 10)).toEqual({
      x: 0,
      y: 10,
    });
    expect(getAnnotationKeyboardNudgeDelta('Enter', 1)).toBeNull();
  });

  it('constrains annotation move deltas to the dominant axis', () => {
    expect(constrainAnnotationMoveDelta({ x: 24, y: 8 })).toEqual({
      x: 24,
      y: 0,
    });
    expect(constrainAnnotationMoveDelta({ x: -6, y: 18 })).toEqual({
      x: 0,
      y: 18,
    });
    expect(constrainAnnotationMoveDelta({ x: 10, y: 10 })).toEqual({
      x: 10,
      y: 10,
    });
  });
});
