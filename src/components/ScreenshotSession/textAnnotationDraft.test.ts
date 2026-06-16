import { describe, expect, it } from 'vitest';
import type { AnnotationStyle } from './annotationStyle';
import {
  annotationFromTextDraft,
  startTextAnnotationDraft,
  updateTextAnnotationDraft,
} from './textAnnotationDraft';

const style: AnnotationStyle = {
  color: [24, 144, 255, 255],
  strokeWidth: 4,
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
});
