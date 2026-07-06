import { describe, expect, it, vi } from 'vitest';
import {
  getInitialHoverSelection,
  getPolledHoverSelection,
  runCaptureHoverSelectionPoll,
  shouldPollCaptureHoverSelection,
  startCaptureHoverSelectionPolling,
} from './captureHoverPolling';
import type { CaptureCandidate } from './captureCandidates';
import type { Point } from './types';

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

  it('uses the captured cursor to choose the initial hover selection', () => {
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

    expect(
      getInitialHoverSelection(candidates, {
        logical_position: { x: 120, y: 120 },
        hotspot: { x: 0, y: 0 },
        image_width: 16,
        image_height: 16,
        scale_factor: 2,
        image_base64: '',
      }),
    ).toEqual({
      x: 100,
      y: 100,
      width: 200,
      height: 150,
    });
    expect(getInitialHoverSelection(candidates, null)).toBeNull();
  });

  it('runs a hover poll tick through injected cursor and hover adapters', async () => {
    const events: string[] = [];
    const candidates: CaptureCandidate[] = [
      {
        id: 'window:inner',
        kind: 'window',
        rect: { x: 100, y: 100, width: 200, height: 150 },
        priority: 10,
      },
    ];

    await runCaptureHoverSelectionPoll({
      sessionId: 'capture-1',
      candidates,
      shouldTrackMagnifierCursor: true,
      canPoll: () => true,
      getCursorPosition: async () => ({ x: 120, y: 120 }),
      setCursorPointRef: (point) => {
        events.push(`ref:${point.x},${point.y}`);
      },
      setCursorPoint: (point) => {
        events.push(`state:${point.x},${point.y}`);
      },
      scheduleSelectionOverlayPaint: () => {
        events.push('paint');
      },
      syncHoverSelection: (rect) => {
        events.push(`hover:${rect?.x ?? 'none'},${rect?.y ?? 'none'}`);
      },
      scheduleNextPoll: () => {
        events.push('next');
      },
    });

    expect(events).toEqual([
      'ref:120,120',
      'state:120,120',
      'paint',
      'hover:100,100',
      'next',
    ]);
  });

  it('clears hover selection without painting when a poll has no cursor point', async () => {
    const events: string[] = [];

    await runCaptureHoverSelectionPoll({
      sessionId: 'capture-1',
      candidates: [],
      shouldTrackMagnifierCursor: true,
      canPoll: () => true,
      getCursorPosition: async () => null,
      setCursorPointRef: () => {
        events.push('ref');
      },
      setCursorPoint: () => {
        events.push('state');
      },
      scheduleSelectionOverlayPaint: () => {
        events.push('paint');
      },
      syncHoverSelection: (rect) => {
        events.push(rect ? 'hover' : 'hover:none');
      },
      scheduleNextPoll: () => {
        events.push('next');
      },
    });

    expect(events).toEqual(['hover:none', 'next']);
  });

  it('ignores a stale hover poll result after disposal', async () => {
    const events: string[] = [];
    let disposed = false;
    let resolvePoint: (point: Point) => void = () => undefined;
    const cursorPromise = new Promise<Point>((resolve) => {
      resolvePoint = resolve;
    });

    const pollPromise = runCaptureHoverSelectionPoll({
      sessionId: 'capture-1',
      candidates: [],
      shouldTrackMagnifierCursor: true,
      canPoll: () => true,
      isDisposed: () => disposed,
      getCursorPosition: async () => cursorPromise,
      setCursorPointRef: () => {
        events.push('ref');
      },
      setCursorPoint: () => {
        events.push('state');
      },
      scheduleSelectionOverlayPaint: () => {
        events.push('paint');
      },
      syncHoverSelection: () => {
        events.push('hover');
      },
      scheduleNextPoll: () => {
        events.push('next');
      },
    });

    disposed = true;
    resolvePoint({ x: 120, y: 120 });
    await pollPromise;

    expect(events).toEqual([]);
  });

  it('starts hover polling immediately and clears the active timer on cleanup', () => {
    let nextTimerId = 1;
    const timers: Array<{ id: number; delayMs: number; handler: () => void }> = [];
    const clearTimeout = vi.fn();

    const stop = startCaptureHoverSelectionPolling({
      sessionId: 'capture-1',
      candidates: [],
      shouldTrackMagnifierCursor: false,
      intervalMs: 16,
      canPoll: () => true,
      getCursorPosition: async () => null,
      setCursorPointRef: () => undefined,
      setCursorPoint: () => undefined,
      scheduleSelectionOverlayPaint: () => undefined,
      syncHoverSelection: () => undefined,
      setTimeout: (handler, delayMs) => {
        const id = nextTimerId;
        nextTimerId += 1;
        timers.push({ id, delayMs, handler });
        return id;
      },
      clearTimeout,
    });

    expect(timers).toHaveLength(1);
    expect(timers[0].delayMs).toBe(0);

    stop();

    expect(clearTimeout).toHaveBeenCalledWith(1);
  });
});
