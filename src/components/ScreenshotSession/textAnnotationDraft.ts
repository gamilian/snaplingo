import {
  annotationFromText,
  isCommittedAnnotation,
  type AnnotationStyle,
} from './annotationStyle';
import {
  addAnnotationToHistory,
  removeAnnotationFromHistory,
  replaceAnnotationInHistory,
  type AnnotationHistory,
} from './annotationHistory';
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

export function startTextAnnotationDraftFromAnnotation(
  annotation: TextAnnotationCommand,
): TextAnnotationDraft {
  return {
    position: annotation.position,
    text: annotation.text,
    fontSize: annotation.font_size,
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

export function commitTextAnnotationDraft(
  history: AnnotationHistory,
  draft: TextAnnotationDraft,
  style: AnnotationStyle,
  replaceIndex?: number,
): AnnotationHistory {
  const annotation = annotationFromTextDraft(draft, style);
  if (!annotation && replaceIndex !== undefined) {
    return removeAnnotationFromHistory(history, replaceIndex);
  }
  if (!annotation) return history;

  if (replaceIndex !== undefined) {
    return replaceAnnotationInHistory(history, replaceIndex, annotation);
  }

  return addAnnotationToHistory(history, annotation);
}
