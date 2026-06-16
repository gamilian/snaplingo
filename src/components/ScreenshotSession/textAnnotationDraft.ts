import {
  annotationFromText,
  isCommittedAnnotation,
  type AnnotationStyle,
} from './annotationStyle';
import type { TextAnnotationCommand, Point } from './types';

export interface TextAnnotationDraft {
  position: Point;
  text: string;
  fontSize: number;
}

export function startTextAnnotationDraft(
  position: Point,
  fontSize: number,
): TextAnnotationDraft {
  return {
    position,
    text: '',
    fontSize,
  };
}

export function updateTextAnnotationDraft(
  draft: TextAnnotationDraft,
  text: string,
): TextAnnotationDraft {
  return {
    ...draft,
    text,
  };
}

export function annotationFromTextDraft(
  draft: TextAnnotationDraft,
  style: AnnotationStyle,
): TextAnnotationCommand | null {
  const annotation = annotationFromText(
    draft.position,
    draft.text,
    style,
    draft.fontSize,
  );

  return isCommittedAnnotation(annotation) ? annotation : null;
}
