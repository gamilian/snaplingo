import { describe, expect, it } from 'vitest';
import type {
  ArrowAnnotationCommand,
  BlurAnnotationCommand,
  TextAnnotationCommand,
} from '../../domain/capture';
import {
  annotationFromGesture,
  completeAnnotationGesture,
  DEFAULT_ANNOTATION_STYLE,
} from './annotationStyle';
import {
  arrowCurvePoints,
  getAnnotationBounds,
  hitTestAnnotations,
  moveAnnotationByDelta,
  resizeAnnotationByHandle,
} from './annotationGeometry';

const bounds = { x: 0, y: 0, width: 500, height: 500 };
const arrow: ArrowAnnotationCommand = {
  type: 'arrow',
  start: { x: 20, y: 100 },
  end: { x: 120, y: 100 },
  color: [255, 0, 0, 255],
  stroke_width: 2,
};

describe('extended annotation editing', () => {
  it('creates a rounded blur region and applies Shift square constraints', () => {
    const blur = completeAnnotationGesture(
      { tool: 'blur', startPoint: { x: 10, y: 20 } },
      { x: 70, y: 60 },
      { ...DEFAULT_ANNOTATION_STYLE, blurRadius: 12, cornerRadius: 8 },
      true,
    );
    expect(blur).toEqual({
      type: 'blur',
      rect: { x: 10, y: 20, width: 40, height: 40 },
      radius: 12,
      corner_radius: 8,
    });
    expect(
      annotationFromGesture(
        'spotlight',
        { x: 10, y: 20 },
        { x: 40, y: 60 },
        DEFAULT_ANNOTATION_STYLE,
      ).type,
    ).toBe('spotlight');
  });

  it('selects a curved arrow along its stroke and leaves empty parts of its bounding box unselected', () => {
    const curved = {
      ...arrow,
      control1: { x: 40, y: 20 },
      control2: { x: 100, y: 20 },
    };
    const points = arrowCurvePoints(curved);
    const middle = points[Math.floor(points.length / 2)];
    expect(middle.y).toBeLessThan(50);
    expect(hitTestAnnotations([curved], middle)).toBe(0);
    expect(hitTestAnnotations([curved], { x: 70, y: 90 })).toBeNull();
  });

  it('moves both control points with the arrow and preserves its fixed endpoint during endpoint editing', () => {
    const curved = {
      ...arrow,
      control1: { x: 40, y: 60 },
      control2: { x: 100, y: 60 },
    };
    const moved = moveAnnotationByDelta(curved, {
      x: 10,
      y: 20,
    }) as ArrowAnnotationCommand;
    expect(moved.control1).toEqual({ x: 50, y: 80 });
    const resized = resizeAnnotationByHandle(
      curved,
      'arrow-end',
      { x: 100, y: 0 },
      bounds,
    ) as ArrowAnnotationCommand;
    expect(resized.start).toEqual(arrow.start);
    expect(resized.end).toEqual({ x: 220, y: 100 });
    expect(resized.control1).toEqual({ x: 60, y: 20 });
  });

  it('limits a radius handle to half the short edge without changing blur strength', () => {
    const blur: BlurAnnotationCommand = {
      type: 'blur',
      rect: { x: 20, y: 20, width: 80, height: 40 },
      radius: 12,
    };
    expect(
      resizeAnnotationByHandle(blur, 'radius', { x: -100, y: 100 }, bounds),
    ).toMatchObject({ corner_radius: 20, radius: 12 });
  });

  it('resizes text with its frame while retaining the selected typeface and style', () => {
    const text: TextAnnotationCommand = {
      type: 'text',
      position: { x: 40, y: 100 },
      text: 'Example',
      color: [0, 0, 0, 255],
      font_size: 20,
      font_family: 'Arial',
      bold: true,
    };
    const frame = getAnnotationBounds(text);
    const resized = resizeAnnotationByHandle(
      text,
      'se',
      { x: frame.width, y: frame.height },
      bounds,
    ) as TextAnnotationCommand;
    expect(resized.font_size).toBe(40);
    expect(resized.font_family).toBe('Arial');
    expect(resized.bold).toBe(true);
    expect(resizeAnnotationByHandle(text, 'se', { x: 0, y: -10000 }, bounds)).toMatchObject({ font_size: 8 });
    expect(resizeAnnotationByHandle(text, 'se', { x: 0, y: 10000 }, bounds)).toMatchObject({ font_size: 160 });
    expect(resizeAnnotationByHandle(text, 'e', { x: 100, y: 0 }, bounds)).toMatchObject({ font_size: 20 });
  });
});
