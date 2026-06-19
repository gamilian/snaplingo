import { describe, expect, it } from 'vitest';
import { undoAnnotationHistory } from './annotationHistory';
import { eraseAnnotationAtPoint } from './annotationEraser';
import type { AnnotationCommand } from './types';

const rectangle: AnnotationCommand = {
  type: 'rectangle',
  rect: { x: 1, y: 2, width: 10, height: 8 },
  color: [255, 77, 79, 255],
  stroke_width: 2,
  filled: false,
};

const arrow: AnnotationCommand = {
  type: 'arrow',
  start: { x: 3, y: 4 },
  end: { x: 20, y: 24 },
  color: [255, 77, 79, 255],
  stroke_width: 2,
};

describe('annotation eraser', () => {
  it('erases the topmost annotation hit by a point', () => {
    const topRectangle: AnnotationCommand = {
      ...rectangle,
      rect: { x: 4, y: 5, width: 6, height: 6 },
    };
    const history = {
      annotations: [arrow, rectangle, topRectangle],
      undoneAnnotations: [arrow],
    };

    const erased = eraseAnnotationAtPoint(history, { x: 5, y: 6 });

    expect(erased).toEqual({
      annotations: [arrow, rectangle],
      undoneAnnotations: [],
      undoSnapshots: [[arrow, rectangle, topRectangle]],
      redoSnapshots: [],
    });
    expect(undoAnnotationHistory(erased)).toEqual({
      annotations: [arrow, rectangle, topRectangle],
      undoneAnnotations: [],
      undoSnapshots: [],
      redoSnapshots: [[arrow, rectangle]],
    });
  });

  it('keeps eraser misses stable', () => {
    const history = { annotations: [rectangle], undoneAnnotations: [] };

    expect(eraseAnnotationAtPoint(history, { x: 80, y: 90 })).toBe(history);
  });
});
