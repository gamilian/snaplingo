import {
  getBestCandidateAtPoint,
  type CaptureCandidate,
} from './captureCandidates';
import type { CapturedCursorView, LogicalRect, Point } from './types';

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

export function getInitialHoverSelection(
  candidates: CaptureCandidate[],
  capturedCursor: CapturedCursorView | null | undefined,
): LogicalRect | null {
  if (!capturedCursor) return null;

  return getPolledHoverSelection(candidates, capturedCursor.logical_position);
}

export interface CaptureHoverSelectionPollOptions {
  sessionId: string;
  candidates: CaptureCandidate[];
  shouldTrackMagnifierCursor: boolean;
  canPoll: () => boolean;
  isDisposed?: () => boolean;
  getCursorPosition: (sessionId: string) => Promise<Point | null>;
  getHoverSelection?: (point: Point) => Promise<LogicalRect | null>;
  setCursorPointRef: (point: Point) => void;
  setCursorPoint: (point: Point) => void;
  scheduleSelectionOverlayPaint: () => void;
  syncHoverSelection: (selection: LogicalRect | null) => void;
  scheduleNextPoll: () => void;
}

export async function runCaptureHoverSelectionPoll({
  candidates,
  canPoll,
  getCursorPosition,
  getHoverSelection,
  isDisposed = () => false,
  scheduleNextPoll,
  scheduleSelectionOverlayPaint,
  sessionId,
  setCursorPoint,
  setCursorPointRef,
  shouldTrackMagnifierCursor,
  syncHoverSelection,
}: CaptureHoverSelectionPollOptions) {
  if (!canPoll()) return;

  try {
    const point = await getCursorPosition(sessionId);
    if (isDisposed() || !canPoll()) return;

    if (!point) {
      syncHoverSelection(null);
      scheduleNextPoll();
      return;
    }

    setCursorPointRef(point);
    if (shouldTrackMagnifierCursor) {
      setCursorPoint(point);
    }
    const hoverSelection = getHoverSelection
      ? await getHoverSelection(point)
      : getPolledHoverSelection(candidates, point);
    if (isDisposed() || !canPoll()) return;
    syncHoverSelection(hoverSelection);
    scheduleSelectionOverlayPaint();
    scheduleNextPoll();
  } catch {
    syncHoverSelection(null);
  }
}

export interface StartCaptureHoverSelectionPollingOptions<TTimer>
  extends Omit<
    CaptureHoverSelectionPollOptions,
    'isDisposed' | 'scheduleNextPoll'
  > {
  intervalMs: number;
  setTimeout: (handler: () => void, delayMs: number) => TTimer;
  clearTimeout: (timer: TTimer) => void;
}

export function startCaptureHoverSelectionPolling<TTimer = number>({
  intervalMs,
  setTimeout,
  clearTimeout,
  ...pollOptions
}: StartCaptureHoverSelectionPollingOptions<TTimer>) {
  let disposed = false;
  let timeoutId: TTimer | null = null;

  const scheduleNextPoll = (delayMs = intervalMs) => {
    if (disposed) return;

    timeoutId = setTimeout(() => {
      timeoutId = null;
      void runCaptureHoverSelectionPoll({
        ...pollOptions,
        isDisposed: () => disposed,
        scheduleNextPoll,
      });
    }, delayMs);
  };

  if (pollOptions.canPoll()) {
    scheduleNextPoll(0);
  }

  return () => {
    disposed = true;
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
  };
}
