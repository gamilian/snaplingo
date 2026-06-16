import { describe, expect, it } from 'vitest';
import type { AnnotationCommand } from './types';
import {
  addAnnotationToHistory,
  redoAnnotationHistory,
  undoAnnotationHistory,
} from './annotationHistory';

const rectangle: AnnotationCommand = {
  type: 'rectangle',
  rect: { x: 1, y: 2, width: 10, height: 8 },
  color: [255, 77, 79, 255],
  stroke_width: 2,
};

const arrow: AnnotationCommand = {
  type: 'arrow',
  start: { x: 3, y: 4 },
  end: { x: 20, y: 24 },
  color: [255, 77, 79, 255],
  stroke_width: 2,
};

describe('annotation history', () => {
  it('adds annotations and clears redo history', () => {
    const history = addAnnotationToHistory(
      { annotations: [rectangle], undoneAnnotations: [arrow] },
      arrow,
    );

    expect(history).toEqual({
      annotations: [rectangle, arrow],
      undoneAnnotations: [],
    });
  });

  it('undos and redos the last annotation', () => {
    const undone = undoAnnotationHistory({
      annotations: [rectangle, arrow],
      undoneAnnotations: [],
    });

    expect(undone).toEqual({
      annotations: [rectangle],
      undoneAnnotations: [arrow],
    });

    expect(redoAnnotationHistory(undone)).toEqual({
      annotations: [rectangle, arrow],
      undoneAnnotations: [],
    });
  });

  it('keeps empty undo and redo operations stable', () => {
    expect(
      undoAnnotationHistory({ annotations: [], undoneAnnotations: [arrow] }),
    ).toEqual({ annotations: [], undoneAnnotations: [arrow] });

    expect(
      redoAnnotationHistory({ annotations: [rectangle], undoneAnnotations: [] }),
    ).toEqual({ annotations: [rectangle], undoneAnnotations: [] });
  });
});
