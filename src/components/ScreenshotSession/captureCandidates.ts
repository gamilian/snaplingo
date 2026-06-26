import type {
  CaptureCandidateKind,
  CaptureCandidateView,
  LogicalRect,
  MonitorSnapshotView,
  Point,
} from './types';

export interface CaptureCandidate {
  id: string;
  kind: CaptureCandidateKind;
  rect: LogicalRect;
  priority: number;
}

export function buildMonitorCandidates(
  monitors: MonitorSnapshotView[],
): CaptureCandidate[] {
  return monitors.map((monitor) => ({
    id: `monitor:${monitor.id}`,
    kind: 'monitor',
    rect: monitor.logical_bounds,
    priority: 0,
  }));
}

export function buildCaptureCandidates(
  monitors: MonitorSnapshotView[],
  candidates: CaptureCandidateView[] = [],
): CaptureCandidate[] {
  return [...buildMonitorCandidates(monitors), ...candidates];
}

export function getBestCandidateAtPoint(
  candidates: CaptureCandidate[],
  point: Point,
): CaptureCandidate | null {
  return sortCandidatesAtPoint(candidates, point, false)[0] ?? null;
}

export function getCandidateForPointerCompletion(
  candidates: CaptureCandidate[],
  point: Point,
  activeRect: LogicalRect | null,
): CaptureCandidate | null {
  if (activeRect && containsPoint(activeRect, point)) {
    return candidates.find((candidate) => areRectsEqual(candidate.rect, activeRect)) ?? null;
  }

  return getBestCandidateAtPoint(candidates, point);
}

export function getCandidateForPointerReleaseCompletion(
  candidates: CaptureCandidate[],
  point: Point,
  activeRect: LogicalRect | null,
  selection: LogicalRect,
  minimumSelectionSize: number,
): CaptureCandidate | null {
  if (
    selection.width >= minimumSelectionSize &&
    selection.height >= minimumSelectionSize
  ) {
    return null;
  }

  return getCandidateForPointerCompletion(candidates, point, activeRect);
}

export function getNextCandidateAtPoint(
  candidates: CaptureCandidate[],
  point: Point,
  currentRect: LogicalRect | null,
  direction: 1 | -1,
): CaptureCandidate | null {
  const matches = sortCandidatesAtPoint(candidates, point, true);
  if (matches.length === 0) return null;

  const currentIndex = currentRect
    ? matches.findIndex((candidate) => areRectsEqual(candidate.rect, currentRect))
    : -1;
  const nextIndex =
    currentIndex === -1
      ? direction === 1
        ? 0
        : matches.length - 1
      : (currentIndex + direction + matches.length) % matches.length;

  return matches[nextIndex];
}

function sortCandidatesAtPoint(
  candidates: CaptureCandidate[],
  point: Point,
  includeMonitorCandidates: boolean,
): CaptureCandidate[] {
  return candidates
    .filter(
      (candidate) => includeMonitorCandidates || candidate.kind !== 'monitor',
    )
    .filter((candidate) => containsPoint(candidate.rect, point))
    .sort((a, b) => {
      if (a.priority !== b.priority) return b.priority - a.priority;

      return area(a.rect) - area(b.rect);
    });
}

function containsPoint(rect: LogicalRect, point: Point) {
  return (
    point.x >= rect.x &&
    point.x < rect.x + rect.width &&
    point.y >= rect.y &&
    point.y < rect.y + rect.height
  );
}

function area(rect: LogicalRect) {
  return rect.width * rect.height;
}

function areRectsEqual(a: LogicalRect, b: LogicalRect) {
  return (
    a.x === b.x &&
    a.y === b.y &&
    a.width === b.width &&
    a.height === b.height
  );
}
