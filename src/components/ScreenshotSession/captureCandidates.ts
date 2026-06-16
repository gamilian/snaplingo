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
  return (
    candidates
      .filter((candidate) => containsPoint(candidate.rect, point))
      .sort((a, b) => {
        if (a.priority !== b.priority) return b.priority - a.priority;

        return area(a.rect) - area(b.rect);
      })[0] ?? null
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
