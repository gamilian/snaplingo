import { describe, expect, it } from 'vitest';
import {
  getPolledHoverSelection,
  shouldPollCaptureHoverSelection,
} from './captureHoverPolling';
import type { CaptureCandidate } from './captureCandidates';

describe('capture hover polling', () => {
  it('polls only while selecting without an active drag or edit gesture', () => {
    expect(
      shouldPollCaptureHoverSelection({
        status: 'selecting',
        hasSession: true,
        hasSelectionBounds: true,
        hasActiveStartPoint: false,
        hasEditGesture: false,
      }),
    ).toBe(true);

    expect(
      shouldPollCaptureHoverSelection({
        status: 'preview',
        hasSession: true,
        hasSelectionBounds: true,
        hasActiveStartPoint: false,
        hasEditGesture: false,
      }),
    ).toBe(false);
    expect(
      shouldPollCaptureHoverSelection({
        status: 'selecting',
        hasSession: true,
        hasSelectionBounds: true,
        hasActiveStartPoint: true,
        hasEditGesture: false,
      }),
    ).toBe(false);
    expect(
      shouldPollCaptureHoverSelection({
        status: 'selecting',
        hasSession: true,
        hasSelectionBounds: true,
        hasActiveStartPoint: false,
        hasEditGesture: true,
      }),
    ).toBe(false);
  });

  it('uses the best candidate under the polled cursor point', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'window:outer',
        kind: 'window',
        rect: { x: 0, y: 0, width: 500, height: 500 },
        priority: 10,
      },
      {
        id: 'window:inner',
        kind: 'window',
        rect: { x: 100, y: 100, width: 200, height: 150 },
        priority: 10,
      },
    ];

    expect(getPolledHoverSelection(candidates, { x: 120, y: 120 })).toEqual({
      x: 100,
      y: 100,
      width: 200,
      height: 150,
    });
    expect(getPolledHoverSelection(candidates, { x: 800, y: 800 })).toBeNull();
  });
});
