import { describe, expect, it } from 'vitest';
import type { AnnotationCommand } from './types';
import {
  addAnnotationToHistory,
  clearAnnotationHistory,
  removeAnnotationFromHistory,
  replaceAnnotationInHistory,
  redoAnnotationHistory,
  undoAnnotationHistory,
} from './annotationHistory';

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

describe('annotation history', () => {
  it('adds annotations and clears redo history', () => {
    const history = addAnnotationToHistory(
      { annotations: [rectangle], undoneAnnotations: [arrow] },
      arrow,
    );

    expect(history).toEqual({
      annotations: [rectangle, arrow],
      undoneAnnotations: [],
      undoSnapshots: [[rectangle]],
      redoSnapshots: [],
    });
  });

  it('undos and redos the last annotation', () => {
    const history = addAnnotationToHistory(
      addAnnotationToHistory(
        { annotations: [], undoneAnnotations: [] },
        rectangle,
      ),
      arrow,
    );
    const undone = undoAnnotationHistory(history);

    expect(undone).toEqual({
      annotations: [rectangle],
      undoneAnnotations: [],
      undoSnapshots: [[]],
      redoSnapshots: [[rectangle, arrow]],
    });

    expect(redoAnnotationHistory(undone)).toEqual({
      annotations: [rectangle, arrow],
      undoneAnnotations: [],
      undoSnapshots: [[], [rectangle]],
      redoSnapshots: [],
    });
  });

  it('removes annotations with undo and redo support', () => {
    const removed = removeAnnotationFromHistory(
      {
        annotations: [rectangle, arrow],
        undoneAnnotations: [rectangle],
      },
      0,
    );

    expect(removed).toEqual({
      annotations: [arrow],
      undoneAnnotations: [],
      undoSnapshots: [[rectangle, arrow]],
      redoSnapshots: [],
    });

    expect(undoAnnotationHistory(removed)).toEqual({
      annotations: [rectangle, arrow],
      undoneAnnotations: [],
      undoSnapshots: [],
      redoSnapshots: [[arrow]],
    });
  });

  it('clears all annotations without leaving undo or redo history', () => {
    const history = {
      annotations: [rectangle, arrow],
      undoneAnnotations: [rectangle],
      undoSnapshots: [[rectangle]],
      redoSnapshots: [[rectangle, arrow]],
    };

    const cleared = clearAnnotationHistory(history);

    expect(cleared).toEqual({
      annotations: [],
      undoneAnnotations: [],
      undoSnapshots: [],
      redoSnapshots: [],
    });
    expect(undoAnnotationHistory(cleared)).toBe(cleared);
    expect(redoAnnotationHistory(cleared)).toBe(cleared);
  });

  it('replaces annotations with undo and redo support', () => {
    const movedArrow: AnnotationCommand = {
      ...arrow,
      start: { x: 10, y: 20 },
      end: { x: 30, y: 40 },
    };
    const replaced = replaceAnnotationInHistory(
      {
        annotations: [rectangle, arrow],
        undoneAnnotations: [rectangle],
      },
      1,
      movedArrow,
    );

    expect(replaced).toEqual({
      annotations: [rectangle, movedArrow],
      undoneAnnotations: [],
      undoSnapshots: [[rectangle, arrow]],
      redoSnapshots: [],
    });

    expect(undoAnnotationHistory(replaced)).toEqual({
      annotations: [rectangle, arrow],
      undoneAnnotations: [],
      undoSnapshots: [],
      redoSnapshots: [[rectangle, movedArrow]],
    });
  });

  it('keeps no-op annotation replacements stable', () => {
    const history = { annotations: [rectangle], undoneAnnotations: [] };

    expect(replaceAnnotationInHistory(history, 0, rectangle)).toBe(history);
    expect(replaceAnnotationInHistory(history, -1, arrow)).toBe(history);
    expect(replaceAnnotationInHistory(history, 1, arrow)).toBe(history);
  });

  it('keeps snapshot undo stable after the stack is exhausted', () => {
    const removed = removeAnnotationFromHistory(
      {
        annotations: [rectangle, arrow],
        undoneAnnotations: [],
      },
      1,
    );
    const undone = undoAnnotationHistory(removed);

    expect(undoAnnotationHistory(undone)).toBe(undone);
    expect(redoAnnotationHistory(redoAnnotationHistory(undone))).toEqual(
      redoAnnotationHistory(undone),
    );
  });

  it('keeps invalid annotation removals stable', () => {
    const history = { annotations: [rectangle], undoneAnnotations: [] };

    expect(removeAnnotationFromHistory(history, -1)).toBe(history);
    expect(removeAnnotationFromHistory(history, 1)).toBe(history);
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
