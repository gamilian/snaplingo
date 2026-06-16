import type { AnnotationCommand } from './types';

export interface AnnotationHistory {
  annotations: AnnotationCommand[];
  undoneAnnotations: AnnotationCommand[];
  undoSnapshots?: AnnotationCommand[][];
  redoSnapshots?: AnnotationCommand[][];
}

export function emptyAnnotationHistory(): AnnotationHistory {
  return {
    annotations: [],
    undoneAnnotations: [],
    undoSnapshots: [],
    redoSnapshots: [],
  };
}

function snapshotHistory(
  history: AnnotationHistory,
  annotations: AnnotationCommand[],
): AnnotationHistory {
  return {
    annotations,
    undoneAnnotations: [],
    undoSnapshots: [...(history.undoSnapshots ?? []), history.annotations],
    redoSnapshots: [],
  };
}

function sameAnnotation(a: AnnotationCommand, b: AnnotationCommand) {
  return JSON.stringify(a) === JSON.stringify(b);
}

export function addAnnotationToHistory(
  history: AnnotationHistory,
  annotation: AnnotationCommand,
): AnnotationHistory {
  return snapshotHistory(history, [...history.annotations, annotation]);
}

export function removeAnnotationFromHistory(
  history: AnnotationHistory,
  annotationIndex: number,
): AnnotationHistory {
  if (
    annotationIndex < 0 ||
    annotationIndex >= history.annotations.length
  ) {
    return history;
  }

  return snapshotHistory(
    history,
    history.annotations.filter((_, index) => index !== annotationIndex),
  );
}

export function replaceAnnotationInHistory(
  history: AnnotationHistory,
  annotationIndex: number,
  annotation: AnnotationCommand,
): AnnotationHistory {
  if (
    annotationIndex < 0 ||
    annotationIndex >= history.annotations.length ||
    sameAnnotation(history.annotations[annotationIndex], annotation)
  ) {
    return history;
  }

  return snapshotHistory(
    history,
    history.annotations.map((currentAnnotation, index) =>
      index === annotationIndex ? annotation : currentAnnotation,
    ),
  );
}

export function undoAnnotationHistory(
  history: AnnotationHistory,
): AnnotationHistory {
  if (history.undoSnapshots && history.undoSnapshots.length === 0) {
    return history;
  }

  if (history.undoSnapshots?.length) {
    const undoSnapshots = history.undoSnapshots.slice(0, -1);
    const previousAnnotations =
      history.undoSnapshots[history.undoSnapshots.length - 1];

    return {
      annotations: previousAnnotations,
      undoneAnnotations: [],
      undoSnapshots,
      redoSnapshots: [history.annotations, ...(history.redoSnapshots ?? [])],
    };
  }

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
  if (history.redoSnapshots && history.redoSnapshots.length === 0) {
    return history;
  }

  if (history.redoSnapshots?.length) {
    const [nextAnnotations, ...redoSnapshots] = history.redoSnapshots;

    return {
      annotations: nextAnnotations,
      undoneAnnotations: [],
      undoSnapshots: [...(history.undoSnapshots ?? []), history.annotations],
      redoSnapshots,
    };
  }

  if (history.undoneAnnotations.length === 0) {
    return history;
  }

  const [redoneAnnotation, ...undoneAnnotations] = history.undoneAnnotations;

  return {
    annotations: [...history.annotations, redoneAnnotation],
    undoneAnnotations,
  };
}
