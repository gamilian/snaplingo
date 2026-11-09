import { buildCaptureCandidates, type CaptureCandidate } from './captureCandidates';
import {
  getCaptureMagnifierRuntimeState,
  type CaptureMagnifierRuntimeState,
} from './captureMagnifierState';
import { getCaptureSelectedAnnotationBounds } from './captureEditorRuntime';
import { fitToolbarSize, getToolbarPosition } from './selection';
import {
  getVirtualDesktopBounds,
  virtualPointToViewportPoint,
  virtualRectToViewportRect,
} from './virtualDesktop';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import type {
  AnnotationCommand,
  CaptureSessionView,
  LogicalRect,
  Point,
} from '../../domain/capture';

interface Size {
  width: number;
  height: number;
}

interface CachedCaptureCandidates {
  candidates: CaptureCandidate[];
  rects: LogicalRect[];
  selectionBounds: LogicalRect;
  viewportBounds: LogicalRect;
}

const captureCandidateCache = new WeakMap<
  CaptureSessionView,
  CachedCaptureCandidates
>();

export function getCaptureCandidatesForSession(
  session: CaptureSessionView,
): CaptureCandidate[] {
  return getCachedCaptureCandidates(session).candidates;
}

export function getCaptureCandidateRectsForSession(
  session: CaptureSessionView,
): LogicalRect[] {
  return getCachedCaptureCandidates(session).rects;
}

function getCachedCaptureCandidates(
  session: CaptureSessionView,
): CachedCaptureCandidates {
  const cached = captureCandidateCache.get(session);
  if (cached) return cached;

  const candidates = buildCaptureCandidates(session.monitors, session.candidates);
  const selectionBounds = getVirtualDesktopBounds(session.monitors);
  const next = {
    candidates,
    rects: candidates.map((candidate) => candidate.rect),
    selectionBounds,
    viewportBounds: {
      x: 0,
      y: 0,
      width: selectionBounds.width,
      height: selectionBounds.height,
    },
  };
  captureCandidateCache.set(session, next);
  return next;
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
  toolbarSize: Size;
}

interface CaptureWorkspaceDerivedOptions {
  state: CaptureWorkspaceState;
  hydratedCaptureSessionId: string | null;
  toolbarSize: Size;
  toolbarGap: number;
  toolbarClientBounds?: LogicalRect;
}

export function getCaptureWorkspaceDerivedState({
  state,
  hydratedCaptureSessionId,
  toolbarGap,
  toolbarSize,
  toolbarClientBounds,
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
  const cachedCandidates = state.session
    ? getCachedCaptureCandidates(state.session)
    : null;
  const captureCandidates = cachedCandidates?.candidates ?? [];
  const areCaptureImagesReady = state.session
    ? hydratedCaptureSessionId === state.session.id
    : false;
  const snapTargetRects = cachedCandidates?.rects ?? [];
  const selectionBounds = cachedCandidates?.selectionBounds ?? null;
  const viewportBounds = cachedCandidates?.viewportBounds ?? null;
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
  const clientBounds = toolbarClientBounds ?? viewportBounds;
  const fittedToolbarSize = fitToolbarSize(
    toolbarSize,
    clientBounds ?? { x: 0, y: 0, width: 0, height: 0 },
  );
  const toolbarPosition =
    selectionViewportRect && clientBounds && state.status === 'preview'
      ? getToolbarPosition(
          selectionViewportRect,
          clientBounds,
          fittedToolbarSize,
          toolbarGap,
        )
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
    toolbarSize: fittedToolbarSize,
    ...magnifier,
  };
}
