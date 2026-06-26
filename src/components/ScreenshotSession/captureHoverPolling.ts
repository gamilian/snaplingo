import {
  getBestCandidateAtPoint,
  type CaptureCandidate,
} from './captureCandidates';
import type { LogicalRect, Point } from './types';

type CaptureHoverPollingStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';

export interface CaptureHoverPollingInput {
  status: CaptureHoverPollingStatus;
  hasSession: boolean;
  hasSelectionBounds: boolean;
  hasActiveStartPoint: boolean;
  hasEditGesture: boolean;
}

export function shouldPollCaptureHoverSelection({
  status,
  hasSession,
  hasSelectionBounds,
  hasActiveStartPoint,
  hasEditGesture,
}: CaptureHoverPollingInput) {
  return (
    status === 'selecting' &&
    hasSession &&
    hasSelectionBounds &&
    !hasActiveStartPoint &&
    !hasEditGesture
  );
}

export function getPolledHoverSelection(
  candidates: CaptureCandidate[],
  point: Point,
): LogicalRect | null {
  return getBestCandidateAtPoint(candidates, point)?.rect ?? null;
}
