import { hitTestAnnotations } from './annotationGeometry';
import {
  type AnnotationHistory,
  removeAnnotationFromHistory,
} from './annotationHistory';
import type { Point } from './types';

export function eraseAnnotationAtPoint(
  history: AnnotationHistory,
  point: Point,
): AnnotationHistory {
  const annotationIndex = hitTestAnnotations(history.annotations, point);
  if (annotationIndex === null) return history;

  return removeAnnotationFromHistory(history, annotationIndex);
}
