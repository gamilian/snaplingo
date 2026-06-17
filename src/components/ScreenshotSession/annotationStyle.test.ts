import { describe, expect, it } from 'vitest';
import {
  ANNOTATION_COLORS,
  appendAnnotationGesturePoint,
  annotationColorFromShortcut,
  annotationFromGestureDraft,
  annotationFromText,
  annotationFromGesture,
  applyAnnotationStyle,
  completeAnnotationGesture,
  constrainAnnotationGesturePoint,
  isAnnotationFillToggleShortcut,
  nextAnnotationToolFromCycleShortcut,
  annotationSizeDirectionFromShortcut,
  annotationSizeDirectionFromWheel,
  annotationToolFromShortcut,
  isCommittedAnnotation,
  nextAnnotationStrokeWidth,
  nextTextFontSize,
  undoAnnotationGesturePoint,
  type AnnotationStyle,
} from './annotationStyle';

const style: AnnotationStyle = {
  color: [40, 167, 69, 255],
  strokeWidth: 5,
  filled: false,
};

const filledStyle: AnnotationStyle = {
  ...style,
  filled: true,
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
      filled: false,
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
      filled: false,
    });
  });

  it('creates filled geometric annotations from the selected fill style', () => {
    expect(
      annotationFromGesture(
        'rectangle',
        { x: 2, y: 3 },
        { x: 12, y: 11 },
        filledStyle,
      ),
    ).toMatchObject({
      type: 'rectangle',
      filled: true,
    });
    expect(
      annotationFromGesture(
        'ellipse',
        { x: 2, y: 3 },
        { x: 12, y: 11 },
        filledStyle,
      ),
    ).toMatchObject({
      type: 'ellipse',
      filled: true,
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

  it('creates polyline annotations from clicked vertices', () => {
    expect(
      annotationFromGesture(
        'polyline',
        { x: 1, y: 2 },
        { x: 14, y: 9 },
        style,
        [{ x: 1, y: 2 }, { x: 8, y: 2 }, { x: 14, y: 9 }],
      ),
    ).toEqual({
      type: 'polyline',
      points: [{ x: 1, y: 2 }, { x: 8, y: 2 }, { x: 14, y: 9 }],
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

  it('completes annotation gestures from the current pointer position', () => {
    expect(
      completeAnnotationGesture(
        {
          tool: 'arrow',
          startPoint: { x: 10, y: 10 },
        },
        { x: 34, y: 18 },
        style,
        true,
      ),
    ).toEqual({
      type: 'arrow',
      start: { x: 10, y: 10 },
      end: { x: 34, y: 10 },
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });

    expect(
      annotationFromGestureDraft(
        {
          tool: 'polyline',
          startPoint: { x: 1, y: 2 },
          points: [{ x: 1, y: 2 }, { x: 8, y: 2 }],
        },
        { x: 14, y: 9 },
        style,
      ),
    ).toEqual({
      type: 'polyline',
      points: [{ x: 1, y: 2 }, { x: 8, y: 2 }, { x: 14, y: 9 }],
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });

    expect(
      annotationFromGestureDraft(
        {
          tool: 'polyline',
          startPoint: { x: 1, y: 2 },
          points: [{ x: 1, y: 2 }, { x: 8, y: 2 }],
        },
        { x: 14, y: 9 },
        style,
        true,
      ),
    ).toMatchObject({
      type: 'polyline',
      points: [{ x: 1, y: 2 }, { x: 8, y: 2 }, { x: 14, y: 8 }],
    });

    expect(
      completeAnnotationGesture(
        {
          tool: 'pen',
          startPoint: { x: 1, y: 2 },
          points: [{ x: 1, y: 2 }, { x: 6, y: 4 }],
        },
        { x: 9, y: 10 },
        style,
        false,
      ),
    ).toEqual({
      type: 'freehand',
      points: [{ x: 1, y: 2 }, { x: 6, y: 4 }, { x: 9, y: 10 }],
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });
  });

  it('converts pen and highlight gestures to straight strokes while holding shift', () => {
    expect(
      completeAnnotationGesture(
        {
          tool: 'pen',
          startPoint: { x: 10, y: 10 },
          points: [{ x: 10, y: 10 }, { x: 14, y: 18 }, { x: 22, y: 16 }],
        },
        { x: 36, y: 19 },
        style,
        true,
      ),
    ).toEqual({
      type: 'freehand',
      points: [{ x: 10, y: 10 }, { x: 36, y: 10 }],
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });

    expect(
      completeAnnotationGesture(
        {
          tool: 'highlight',
          startPoint: { x: 10, y: 10 },
          points: [{ x: 10, y: 10 }, { x: 15, y: 19 }, { x: 21, y: 30 }],
        },
        { x: 20, y: 40 },
        style,
        true,
      ),
    ).toEqual({
      type: 'highlight',
      points: [{ x: 10, y: 10 }, { x: 10, y: 40 }],
      color: [40, 167, 69, 96],
      stroke_width: 5,
    });
  });

  it('previews shifted point strokes as straight gesture drafts', () => {
    expect(
      annotationFromGestureDraft(
        {
          tool: 'pen',
          startPoint: { x: 4, y: 4 },
          points: [{ x: 4, y: 4 }, { x: 9, y: 15 }],
        },
        { x: 20, y: 12 },
        style,
        true,
      ),
    ).toEqual({
      type: 'freehand',
      points: [{ x: 4, y: 4 }, { x: 20, y: 4 }],
      color: [40, 167, 69, 255],
      stroke_width: 5,
    });
  });

  it('appends constrained polyline vertices while holding shift', () => {
    expect(
      appendAnnotationGesturePoint(
        {
          tool: 'polyline',
          startPoint: { x: 1, y: 2 },
          points: [{ x: 1, y: 2 }, { x: 8, y: 2 }],
        },
        { x: 14, y: 9 },
        true,
      ),
    ).toEqual([{ x: 1, y: 2 }, { x: 8, y: 2 }, { x: 14, y: 8 }]);
  });

  it('undoes fixed polyline vertices from the active gesture', () => {
    expect(
      undoAnnotationGesturePoint({
        tool: 'polyline',
        startPoint: { x: 1, y: 2 },
        points: [{ x: 1, y: 2 }, { x: 8, y: 2 }, { x: 14, y: 8 }],
      }),
    ).toEqual({
      tool: 'polyline',
      startPoint: { x: 1, y: 2 },
      points: [{ x: 1, y: 2 }, { x: 8, y: 2 }],
    });

    expect(
      undoAnnotationGesturePoint({
        tool: 'polyline',
        startPoint: { x: 1, y: 2 },
        points: [{ x: 1, y: 2 }],
      }),
    ).toBeNull();
    expect(
      undoAnnotationGesturePoint({
        tool: 'line',
        startPoint: { x: 1, y: 2 },
      }),
    ).toBeNull();
  });

  it('constrains annotation gestures while holding shift', () => {
    expect(
      constrainAnnotationGesturePoint(
        'rectangle',
        { x: 10, y: 10 },
        { x: 40, y: 20 },
      ),
    ).toEqual({ x: 20, y: 20 });
    expect(
      constrainAnnotationGesturePoint(
        'ellipse',
        { x: 40, y: 40 },
        { x: 15, y: 20 },
      ),
    ).toEqual({ x: 20, y: 20 });
    expect(
      constrainAnnotationGesturePoint(
        'arrow',
        { x: 10, y: 10 },
        { x: 30, y: 18 },
      ),
    ).toEqual({ x: 30, y: 10 });
    expect(
      constrainAnnotationGesturePoint(
        'line',
        { x: 10, y: 10 },
        { x: 20, y: 40 },
      ),
    ).toEqual({ x: 10, y: 40 });
    expect(
      constrainAnnotationGesturePoint(
        'polyline',
        { x: 10, y: 10 },
        { x: 20, y: 40 },
      ),
    ).toEqual({ x: 10, y: 40 });
    expect(
      constrainAnnotationGesturePoint(
        'pen',
        { x: 10, y: 10 },
        { x: 30, y: 18 },
      ),
    ).toEqual({ x: 30, y: 10 });
  });

  it('maps plain tool shortcut keys to annotation tools', () => {
    const plainKey = { metaKey: false, ctrlKey: false, altKey: false };

    expect(annotationToolFromShortcut({ ...plainKey, key: 'r' })).toBe('rectangle');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'O' })).toBe('ellipse');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'a' })).toBe('arrow');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'l' })).toBe('line');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'p' })).toBe('pen');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'h' })).toBe('highlight');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'm' })).toBe('mosaic');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'b' })).toBe('blur');
    expect(annotationToolFromShortcut({ ...plainKey, key: 't' })).toBe('text');
    expect(annotationToolFromShortcut({ ...plainKey, key: 'E' })).toBe('eraser');
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
    expect(
      annotationToolFromShortcut({
        key: 'e',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBeNull();
  });

  it('cycles between line, polyline, and arrow tools with plain Tab', () => {
    const plainTab = {
      key: 'Tab',
      metaKey: false,
      ctrlKey: false,
      altKey: false,
    };

    expect(nextAnnotationToolFromCycleShortcut(plainTab, 'line')).toBe('polyline');
    expect(nextAnnotationToolFromCycleShortcut(plainTab, 'polyline')).toBe('arrow');
    expect(nextAnnotationToolFromCycleShortcut(plainTab, 'arrow')).toBe('line');
    expect(nextAnnotationToolFromCycleShortcut(plainTab, 'rectangle')).toBeNull();
    expect(
      nextAnnotationToolFromCycleShortcut(
        { ...plainTab, key: 'Tab', ctrlKey: true },
        'line',
      ),
    ).toBeNull();
  });

  it('maps number shortcuts to annotation colors', () => {
    const plainKey = { metaKey: false, ctrlKey: false, altKey: false };

    expect(annotationColorFromShortcut({ ...plainKey, key: '1' })).toEqual(
      ANNOTATION_COLORS[0],
    );
    expect(annotationColorFromShortcut({ ...plainKey, key: '6' })).toEqual(
      ANNOTATION_COLORS[5],
    );
  });

  it('does not map modified or unavailable color shortcut keys', () => {
    expect(
      annotationColorFromShortcut({
        key: '1',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBeNull();
    expect(
      annotationColorFromShortcut({
        key: '0',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBeNull();
    expect(
      annotationColorFromShortcut({
        key: '7',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBeNull();
  });

  it('uses plain F for toggling geometric annotation fill', () => {
    const plainKey = { metaKey: false, ctrlKey: false, altKey: false };

    expect(isAnnotationFillToggleShortcut({ ...plainKey, key: 'f' })).toBe(true);
    expect(isAnnotationFillToggleShortcut({ ...plainKey, key: 'F' })).toBe(true);
    expect(
      isAnnotationFillToggleShortcut({
        key: 'f',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe(false);
    expect(isAnnotationFillToggleShortcut({ ...plainKey, key: 'r' })).toBe(false);
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

  it('maps 1 and 2 to annotation size directions only in editing mode', () => {
    const plainKey = { metaKey: false, ctrlKey: false, altKey: false };

    expect(
      annotationSizeDirectionFromShortcut(
        { ...plainKey, key: '1' },
        { editing: true },
      ),
    ).toBe('decrease');
    expect(
      annotationSizeDirectionFromShortcut(
        { ...plainKey, key: '2' },
        { editing: true },
      ),
    ).toBe('increase');
    expect(
      annotationSizeDirectionFromShortcut(
        { ...plainKey, key: '1' },
        { editing: false },
      ),
    ).toBeNull();
    expect(
      annotationSizeDirectionFromShortcut(
        {
          key: '1',
          metaKey: true,
          ctrlKey: false,
          altKey: false,
        },
        { editing: true },
      ),
    ).toBeNull();
  });

  it('maps unmodified mouse wheel movement to annotation size directions', () => {
    expect(
      annotationSizeDirectionFromWheel({
        deltaY: -1,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe('increase');
    expect(
      annotationSizeDirectionFromWheel({
        deltaY: 1,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe('decrease');
    expect(
      annotationSizeDirectionFromWheel({
        deltaY: -1,
        metaKey: false,
        ctrlKey: true,
        altKey: false,
      }),
    ).toBeNull();
    expect(
      annotationSizeDirectionFromWheel({
        deltaY: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBeNull();
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
          filled: false,
        },
        filledStyle,
      ),
    ).toEqual({
      type: 'rectangle',
      rect: { x: 1, y: 2, width: 10, height: 8 },
      color: [40, 167, 69, 255],
      stroke_width: 5,
      filled: true,
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
