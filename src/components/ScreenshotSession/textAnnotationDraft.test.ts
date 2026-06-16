import { describe, expect, it } from 'vitest';
import type { AnnotationStyle } from './annotationStyle';
import type { AnnotationHistory } from './annotationHistory';
import {
  annotationFromTextDraft,
  commitTextAnnotationDraft,
  startTextAnnotationDraft,
  updateTextAnnotationDraft,
} from './textAnnotationDraft';

const style: AnnotationStyle = {
  color: [24, 144, 255, 255],
  strokeWidth: 4,
};

const history: AnnotationHistory = {
  annotations: [
    {
      type: 'rectangle',
      rect: { x: 1, y: 2, width: 3, height: 4 },
      color: [255, 77, 79, 255],
      stroke_width: 2,
    },
  ],
  undoneAnnotations: [
    {
      type: 'line',
      start: { x: 0, y: 0 },
      end: { x: 8, y: 8 },
      color: [255, 255, 255, 255],
      stroke_width: 1,
    },
  ],
};

describe('text annotation draft', () => {
  it('starts an empty text draft at the clicked position', () => {
    expect(startTextAnnotationDraft({ x: 12, y: 18 }, 28)).toEqual({
      position: { x: 12, y: 18 },
      text: '',
      fontSize: 28,
    });
  });

  it('updates the draft text without moving the anchor', () => {
    const draft = startTextAnnotationDraft({ x: 4, y: 9 }, 24);

    expect(updateTextAnnotationDraft(draft, 'First line\nSecond line')).toEqual({
      position: { x: 4, y: 9 },
      text: 'First line\nSecond line',
      fontSize: 24,
    });
  });

  it('does not create annotations from blank text', () => {
    const draft = updateTextAnnotationDraft(
      startTextAnnotationDraft({ x: 4, y: 9 }, 24),
      '   \n  ',
    );

    expect(annotationFromTextDraft(draft, style)).toBeNull();
  });

  it('creates text annotations with draft text, color, and font size', () => {
    const draft = updateTextAnnotationDraft(
      startTextAnnotationDraft({ x: 4, y: 9 }, 24),
      'Snap\nLingo',
    );

    expect(annotationFromTextDraft(draft, style)).toEqual({
      type: 'text',
      position: { x: 4, y: 9 },
      text: 'Snap\nLingo',
      color: [24, 144, 255, 255],
      font_size: 24,
    });
  });

  it('commits text drafts into annotation history and clears redo history', () => {
    const draft = updateTextAnnotationDraft(
      startTextAnnotationDraft({ x: 4, y: 9 }, 24),
      'Snap',
    );

    expect(commitTextAnnotationDraft(history, draft, style)).toEqual({
      annotations: [
        history.annotations[0],
        {
          type: 'text',
          position: { x: 4, y: 9 },
          text: 'Snap',
          color: [24, 144, 255, 255],
          font_size: 24,
        },
      ],
      undoneAnnotations: [],
      undoSnapshots: [history.annotations],
      redoSnapshots: [],
    });
  });

  it('leaves history unchanged when committing blank drafts', () => {
    const draft = updateTextAnnotationDraft(
      startTextAnnotationDraft({ x: 4, y: 9 }, 24),
      ' ',
    );

    expect(commitTextAnnotationDraft(history, draft, style)).toBe(history);
  });
});
