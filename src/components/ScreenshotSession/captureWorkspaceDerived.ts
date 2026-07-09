import { buildCaptureCandidates, type CaptureCandidate } from './captureCandidates';
import {
  getCaptureMagnifierRuntimeState,
  type CaptureMagnifierRuntimeState,
} from './captureMagnifierRuntime';
import { getCaptureSelectedAnnotationBounds } from './captureEditorRuntime';
import { getToolbarPosition } from './selection';
import {
  getVirtualDesktopBounds,
  virtualPointToViewportPoint,
  virtualRectToViewportRect,
} from './virtualDesktop';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import type { AnnotationCommand, LogicalRect, Point } from './types';

interface Size {
  width: number;
  height: number;
}

export interface CaptureWorkspaceDerivedState
  extends CaptureMagnifierRuntimeState {
  annotations: AnnotationCommand[];
  selectedAnnotation: AnnotationCommand | null;
  hasAnnotationEditingContext: boolean;
  canUndoAnnotation: boolean;
  canRedoAnnotation: boolean;
  isTextSizingActive: boolean;
  isFillModeActive: boolean;
  captureCandidates: CaptureCandidate[];
  areCaptureImagesReady: boolean;
  snapTargetRects: LogicalRect[];
  selectionBounds: LogicalRect | null;
  viewportBounds: LogicalRect | null;
  selectionViewportRect: LogicalRect | null;
  cursorViewportPoint: Point | null;
  selectedAnnotationBounds: LogicalRect | null;
  toolbarPosition: Point | null;
}

interface CaptureWorkspaceDerivedOptions {
  state: CaptureWorkspaceState;
  hydratedCaptureSessionId: string | null;
  toolbarSize: Size;
  toolbarGap: number;
}

export function getCaptureWorkspaceDerivedState({
  state,
  hydratedCaptureSessionId,
  toolbarGap,
  toolbarSize,
}: CaptureWorkspaceDerivedOptions): CaptureWorkspaceDerivedState {
  const annotations = state.annotationHistory.annotations;
  const selectedAnnotation =
    state.selectedAnnotationIndex === null
      ? null
      : annotations[state.selectedAnnotationIndex] ?? null;
  const hasAnnotationEditingContext =
    state.activeAnnotationTool !== null ||
    state.selectedAnnotationIndex !== null;
  const canUndoAnnotation =
    state.annotationHistory.undoSnapshots !== undefined
      ? state.annotationHistory.undoSnapshots.length > 0
      : state.annotationHistory.annotations.length > 0;
  const canRedoAnnotation =
    state.annotationHistory.redoSnapshots !== undefined
      ? state.annotationHistory.redoSnapshots.length > 0
      : state.annotationHistory.undoneAnnotations.length > 0;
  const isTextSizingActive =
    state.activeAnnotationTool === 'text' ||
    Boolean(state.textDraft) ||
    selectedAnnotation?.type === 'text';
  const isFillModeActive =
    state.activeAnnotationTool === 'rectangle' ||
    state.activeAnnotationTool === 'ellipse' ||
    selectedAnnotation?.type === 'rectangle' ||
    selectedAnnotation?.type === 'ellipse';
  const captureCandidates = state.session
    ? buildCaptureCandidates(state.session.monitors, state.session.candidates)
    : [];
  const areCaptureImagesReady = state.session
    ? hydratedCaptureSessionId === state.session.id
    : false;
  const snapTargetRects = captureCandidates.map((candidate) => candidate.rect);
  const selectionBounds = state.session
    ? getVirtualDesktopBounds(state.session.monitors)
    : null;
  const viewportBounds = selectionBounds
    ? {
        x: 0,
        y: 0,
        width: selectionBounds.width,
        height: selectionBounds.height,
      }
    : null;
  const selectionViewportRect =
    state.selection && selectionBounds
      ? virtualRectToViewportRect(state.selection, selectionBounds)
      : null;
  const cursorViewportPoint =
    state.cursorPoint && selectionBounds
      ? virtualPointToViewportPoint(state.cursorPoint, selectionBounds)
      : null;
  const magnifier = getCaptureMagnifierRuntimeState({
    session: state.session,
    status: state.status,
    cursorPoint: state.cursorPoint,
    cursorViewportPoint,
    viewportBounds,
    isMagnifierRequested: state.isMagnifierRequested,
  });
  const selectedAnnotationBounds = getCaptureSelectedAnnotationBounds({
    annotations,
    selectedAnnotationIndex: state.selectedAnnotationIndex,
    annotationMoveGesture: state.annotationMoveGesture,
  });
  const toolbarPosition =
    selectionViewportRect && viewportBounds && state.status === 'preview'
      ? getToolbarPosition(selectionViewportRect, viewportBounds, toolbarSize, toolbarGap)
      : null;

  return {
    annotations,
    selectedAnnotation,
    hasAnnotationEditingContext,
    canUndoAnnotation,
    canRedoAnnotation,
    isTextSizingActive,
    isFillModeActive,
    captureCandidates,
    areCaptureImagesReady,
    snapTargetRects,
    selectionBounds,
    viewportBounds,
    selectionViewportRect,
    cursorViewportPoint,
    selectedAnnotationBounds,
    toolbarPosition,
    ...magnifier,
  };
}
