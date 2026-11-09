import type {
  CaptureCandidateKind,
  CaptureCandidateView,
  LogicalRect,
  MonitorSnapshotView,
  Point,
} from '../../domain/capture';

export interface CaptureCandidate {
  id: string;
  kind: CaptureCandidateKind;
  rect: LogicalRect;
  priority: number;
}

const MONITOR_SIZED_WINDOW_TOLERANCE = 1;
const MIN_AUTOMATIC_HOVER_CANDIDATE_SIZE = 48;

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
  return sortCandidatesAtPoint(candidates, point)[0] ?? null;
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

function sortCandidatesAtPoint(
  candidates: CaptureCandidate[],
  point: Point,
): CaptureCandidate[] {
  return candidates
    .filter((candidate) => isAutomaticHoverCandidate(candidate, candidates))
    .filter((candidate) => containsPoint(candidate.rect, point))
    .sort((a, b) => {
      if (a.priority !== b.priority) return b.priority - a.priority;

      return area(a.rect) - area(b.rect);
    });
}

function isAutomaticHoverCandidate(
  candidate: CaptureCandidate,
  candidates: CaptureCandidate[],
) {
  if (candidate.kind === 'monitor') return false;
  if (isTinyAutomaticHoverCandidate(candidate)) return false;

  return !isMonitorSizedWindowCandidate(candidate, candidates);
}

function isTinyAutomaticHoverCandidate(candidate: CaptureCandidate) {
  return (
    candidate.rect.width < MIN_AUTOMATIC_HOVER_CANDIDATE_SIZE ||
    candidate.rect.height < MIN_AUTOMATIC_HOVER_CANDIDATE_SIZE
  );
}

function isMonitorSizedWindowCandidate(
  candidate: CaptureCandidate,
  candidates: CaptureCandidate[],
) {
  if (candidate.kind !== 'window') return false;

  return candidates.some(
    (otherCandidate) =>
      otherCandidate.kind === 'monitor' &&
      areRectsNearlyEqual(candidate.rect, otherCandidate.rect),
  );
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

function areRectsNearlyEqual(a: LogicalRect, b: LogicalRect) {
  return (
    Math.abs(a.x - b.x) <= MONITOR_SIZED_WINDOW_TOLERANCE &&
    Math.abs(a.y - b.y) <= MONITOR_SIZED_WINDOW_TOLERANCE &&
    Math.abs(a.width - b.width) <= MONITOR_SIZED_WINDOW_TOLERANCE &&
    Math.abs(a.height - b.height) <= MONITOR_SIZED_WINDOW_TOLERANCE
  );
}
