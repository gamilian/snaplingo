import { describe, expect, it } from 'vitest';
import {
  getAnnotationBounds,
  hitTestAnnotations,
} from './annotationGeometry';
import type { AnnotationCommand } from './types';

const rectangle: AnnotationCommand = {
  type: 'rectangle',
  rect: { x: 10, y: 20, width: 30, height: 40 },
  color: [255, 77, 79, 255],
  stroke_width: 2,
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
});
