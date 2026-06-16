import { describe, expect, it } from 'vitest';
import {
  annotationFromText,
  annotationFromGesture,
  applyAnnotationStyle,
  annotationSizeDirectionFromShortcut,
  annotationToolFromShortcut,
  isCommittedAnnotation,
  nextAnnotationStrokeWidth,
  nextTextFontSize,
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

  it('creates blur annotations with the selected radius', () => {
    expect(
      annotationFromGesture(
        'blur',
        { x: 12, y: 20 },
        { x: 4, y: 6 },
        style,
      ),
    ).toEqual({
      type: 'blur',
      rect: { x: 4, y: 6, width: 8, height: 14 },
      radius: 5,
    });
  });

  it('creates text annotations with the selected color and font size', () => {
    expect(
      annotationFromText({ x: 6, y: 8 }, 'Snap text', style, 24),
    ).toEqual({
      type: 'text',
      position: { x: 6, y: 8 },
      text: 'Snap text',
      color: [40, 167, 69, 255],
      font_size: 24,
    });
  });

  it('maps plain tool shortcut keys to annotation tools', () => {
    const plainKey = { metaKey: false, ctrlKey: false, altKey: false };

    expect(annotationToolFromShortcut({ ...plainKey, key: 'r' })).toBe('rectangle');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'E' })).toBe('ellipse');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'a' })).toBe('arrow');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'l' })).toBe('line');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'p' })).toBe('pen');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'h' })).toBe('highlight');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'm' })).toBe('mosaic');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'b' })).toBe('blur');
    expect(annotationToolFromShortcut({ ...plainKey, key: 't' })).toBe('text');
  });

  it('does not map modified or unknown tool shortcut keys', () => {
    expect(
      annotationToolFromShortcut({
        key: 'r',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBeNull();
    expect(
      annotationToolFromShortcut({
        key: 'r',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
      }),
    ).toBeNull();
    expect(
      annotationToolFromShortcut({
        key: 'r',
        metaKey: false,
        ctrlKey: false,
        altKey: true,
      }),
    ).toBeNull();
    expect(
      annotationToolFromShortcut({
        key: 'x',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBeNull();
  });

  it('maps bracket shortcuts to annotation size directions', () => {
    const plainKey = { metaKey: false, ctrlKey: false, altKey: false };

    expect(annotationSizeDirectionFromShortcut({ ...plainKey, key: '[' })).toBe(
      'decrease',
    );
    expect(annotationSizeDirectionFromShortcut({ ...plainKey, key: ']' })).toBe(
      'increase',
    );
    expect(
      annotationSizeDirectionFromShortcut({
        key: ']',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBeNull();
    expect(annotationSizeDirectionFromShortcut({ ...plainKey, key: 'r' })).toBeNull();
  });

  it('steps annotation stroke width and text font size within toolbar bounds', () => {
    expect(nextAnnotationStrokeWidth(4, 'increase')).toBe(5);
    expect(nextAnnotationStrokeWidth(1, 'decrease')).toBe(1);
    expect(nextAnnotationStrokeWidth(8, 'increase')).toBe(8);

    expect(nextTextFontSize(24, 'increase')).toBe(25);
    expect(nextTextFontSize(12, 'decrease')).toBe(12);
    expect(nextTextFontSize(48, 'increase')).toBe(48);
  });

  it('updates committed annotations with the selected style', () => {
    expect(
      applyAnnotationStyle(
        {
          type: 'rectangle',
          rect: { x: 1, y: 2, width: 10, height: 8 },
          color: [255, 77, 79, 255],
          stroke_width: 2,
        },
        style,
      ),
    ).toEqual({
      type: 'rectangle',
      rect: { x: 1, y: 2, width: 10, height: 8 },
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });

    expect(
      applyAnnotationStyle(
        {
          type: 'highlight',
          points: [{ x: 1, y: 2 }, { x: 4, y: 6 }],
          color: [255, 77, 79, 96],
          stroke_width: 2,
        },
        style,
      ),
    ).toEqual({
      type: 'highlight',
      points: [{ x: 1, y: 2 }, { x: 4, y: 6 }],
      color: [40, 167, 69, 96],
      stroke_width: 5,
    });

    expect(
      applyAnnotationStyle(
        {
          type: 'mosaic',
          rect: { x: 1, y: 2, width: 10, height: 8 },
          block_size: 2,
        },
        style,
      ),
    ).toEqual({
      type: 'mosaic',
      rect: { x: 1, y: 2, width: 10, height: 8 },
      block_size: 5,
    });

    expect(
      applyAnnotationStyle(
        {
          type: 'blur',
          rect: { x: 1, y: 2, width: 10, height: 8 },
          radius: 2,
        },
        style,
      ),
    ).toEqual({
      type: 'blur',
      rect: { x: 1, y: 2, width: 10, height: 8 },
      radius: 5,
    });
  });

  it('updates text annotations with the selected color and font size', () => {
    expect(
      applyAnnotationStyle(
        {
          type: 'text',
          position: { x: 1, y: 2 },
          text: 'Snap',
          color: [255, 77, 79, 255],
          font_size: 18,
        },
        style,
        32,
      ),
    ).toEqual({
      type: 'text',
      position: { x: 1, y: 2 },
      text: 'Snap',
      color: [40, 167, 69, 255],
      font_size: 32,
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
          'blur',
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
