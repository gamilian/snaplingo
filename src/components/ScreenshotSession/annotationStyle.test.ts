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

  it('creates ellipse annotations with the selected color and stroke width', () => {
    expect(
      annotationFromGesture(
        'ellipse',
        { x: 12, y: 20 },
        { x: 4, y: 6 },
        style,
      ),
    ).toEqual({
      type: 'ellipse',
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

  it('creates line annotations with the selected color and stroke width', () => {
    expect(
      annotationFromGesture(
        'line',
        { x: 2, y: 3 },
        { x: 12, y: 9 },
        style,
      ),
    ).toEqual({
      type: 'line',
      start: { x: 2, y: 3 },
      end: { x: 12, y: 9 },
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });
  });

  it('creates freehand annotations from the captured stroke points', () => {
    expect(
      annotationFromGesture(
        'pen',
        { x: 1, y: 2 },
        { x: 9, y: 10 },
        style,
        [{ x: 1, y: 2 }, { x: 4, y: 6 }, { x: 9, y: 10 }],
      ),
    ).toEqual({
      type: 'freehand',
      points: [{ x: 1, y: 2 }, { x: 4, y: 6 }, { x: 9, y: 10 }],
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });
  });

  it('creates highlight annotations from the captured stroke points', () => {
    expect(
      annotationFromGesture(
        'highlight',
        { x: 1, y: 2 },
        { x: 9, y: 10 },
        style,
        [{ x: 1, y: 2 }, { x: 4, y: 6 }, { x: 9, y: 10 }],
      ),
    ).toEqual({
      type: 'highlight',
      points: [{ x: 1, y: 2 }, { x: 4, y: 6 }, { x: 9, y: 10 }],
      color: [40, 167, 69, 96],
      stroke_width: 5,
    });
  });

  it('creates mosaic annotations with the selected block size', () => {
    expect(
      annotationFromGesture(
        'mosaic',
        { x: 12, y: 20 },
        { x: 4, y: 6 },
        style,
      ),
    ).toEqual({
      type: 'mosaic',
      rect: { x: 4, y: 6, width: 8, height: 14 },
      block_size: 5,
    });
  });

  it('rejects tiny annotations', () => {
    expect(
      isCommittedAnnotation(
        annotationFromGesture(
          'mosaic',
          { x: 1, y: 1 },
          { x: 2, y: 2 },
          style,
        ),
      ),
    ).toBe(false);
    expect(
      isCommittedAnnotation(
        annotationFromGesture(
          'ellipse',
          { x: 1, y: 1 },
          { x: 2, y: 2 },
          style,
        ),
      ),
    ).toBe(false);
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
    expect(
      isCommittedAnnotation(
        annotationFromGesture(
          'line',
          { x: 1, y: 1 },
          { x: 2, y: 2 },
          style,
        ),
      ),
    ).toBe(false);
    expect(
      isCommittedAnnotation(
        annotationFromGesture(
          'highlight',
          { x: 1, y: 1 },
          { x: 2, y: 2 },
          style,
          [{ x: 1, y: 1 }, { x: 2, y: 2 }],
        ),
      ),
    ).toBe(false);
    expect(
      isCommittedAnnotation(
        annotationFromGesture(
          'pen',
          { x: 1, y: 1 },
          { x: 2, y: 2 },
          style,
          [{ x: 1, y: 1 }, { x: 2, y: 2 }],
        ),
      ),
    ).toBe(false);
  });
});
