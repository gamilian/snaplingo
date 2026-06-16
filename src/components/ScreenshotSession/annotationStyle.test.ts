import { describe, expect, it } from 'vitest';
import {
  annotationFromGesture,
  isCommittedAnnotation,
  type AnnotationStyle,
} from './annotationStyle';

const style: AnnotationStyle = {
  color: [40, 167, 69, 255],
  strokeWidth: 5,
};

describe('annotation style', () => {
  it('creates rectangle annotations with the selected color and stroke width', () => {
    expect(
      annotationFromGesture(
        'rectangle',
        { x: 12, y: 20 },
        { x: 4, y: 6 },
        style,
      ),
    ).toEqual({
      type: 'rectangle',
      rect: { x: 4, y: 6, width: 8, height: 14 },
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });
  });

  it('creates arrow annotations with the selected color and stroke width', () => {
    expect(
      annotationFromGesture(
        'arrow',
        { x: 1, y: 2 },
        { x: 30, y: 40 },
        style,
      ),
    ).toEqual({
      type: 'arrow',
      start: { x: 1, y: 2 },
      end: { x: 30, y: 40 },
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });
  });

  it('rejects tiny annotations', () => {
    expect(
      isCommittedAnnotation(
        annotationFromGesture(
          'arrow',
          { x: 1, y: 1 },
          { x: 2, y: 2 },
          style,
        ),
      ),
    ).toBe(false);
  });
});
