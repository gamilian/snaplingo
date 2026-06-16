import type { AnnotationCommand } from './types';

export interface AnnotationHistory {
  annotations: AnnotationCommand[];
  undoneAnnotations: AnnotationCommand[];
}

export function emptyAnnotationHistory(): AnnotationHistory {
  return {
    annotations: [],
    undoneAnnotations: [],
  };
}

export function addAnnotationToHistory(
  history: AnnotationHistory,
  annotation: AnnotationCommand,
): AnnotationHistory {
  return {
    annotations: [...history.annotations, annotation],
    undoneAnnotations: [],
  };
}

export function undoAnnotationHistory(
  history: AnnotationHistory,
): AnnotationHistory {
  if (history.annotations.length === 0) {
    return history;
  }

  const annotations = history.annotations.slice(0, -1);
  const undoneAnnotation = history.annotations[history.annotations.length - 1];

  return {
    annotations,
    undoneAnnotations: [undoneAnnotation, ...history.undoneAnnotations],
  };
}

export function redoAnnotationHistory(
  history: AnnotationHistory,
): AnnotationHistory {
  if (history.undoneAnnotations.length === 0) {
    return history;
  }

  const [redoneAnnotation, ...undoneAnnotations] = history.undoneAnnotations;

  return {
    annotations: [...history.annotations, redoneAnnotation],
    undoneAnnotations,
  };
}
