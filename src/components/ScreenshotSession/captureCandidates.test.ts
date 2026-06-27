import { describe, expect, it } from 'vitest';
import {
  buildCaptureCandidates,
  buildMonitorCandidates,
  getBestCandidateAtPoint,
  getCandidateForPointerCompletion,
  getCandidateForPointerReleaseCompletion,
  getNextCandidateAtPoint,
  type CaptureCandidate,
} from './captureCandidates';
import type { CaptureCandidateView, MonitorSnapshotView } from './types';

const monitors: MonitorSnapshotView[] = [
  {
    id: 'left',
    logical_bounds: { x: -1280, y: 0, width: 1280, height: 720 },
    physical_bounds: { x: -2560, y: 0, width: 2560, height: 1440 },
    scale_factor: 2,
    image_base64: 'left-image',
  },
  {
    id: 'primary',
    logical_bounds: { x: 0, y: 0, width: 1440, height: 900 },
    physical_bounds: { x: 0, y: 0, width: 2880, height: 1800 },
    scale_factor: 2,
    image_base64: 'primary-image',
  },
];

describe('capture candidates', () => {
  it('builds monitor candidates from frozen monitor bounds', () => {
    expect(buildMonitorCandidates(monitors)).toEqual([
      {
        id: 'monitor:left',
        kind: 'monitor',
        rect: { x: -1280, y: 0, width: 1280, height: 720 },
        priority: 0,
      },
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
    ]);
  });

  it('combines backend candidates with monitor candidates', () => {
    const backendCandidates: CaptureCandidateView[] = [
      {
        id: 'window:settings',
        kind: 'window',
        rect: { x: 100, y: 100, width: 500, height: 400 },
        priority: 10,
      },
    ];

    expect(buildCaptureCandidates(monitors, backendCandidates)).toEqual([
      {
        id: 'monitor:left',
        kind: 'monitor',
        rect: { x: -1280, y: 0, width: 1280, height: 720 },
        priority: 0,
      },
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
      {
        id: 'window:settings',
        kind: 'window',
        rect: { x: 100, y: 100, width: 500, height: 400 },
        priority: 10,
      },
    ]);
  });

  it('returns the highest priority candidate under a point', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
      {
        id: 'window:settings',
        kind: 'window',
        rect: { x: 100, y: 100, width: 500, height: 400 },
        priority: 10,
      },
    ];

    expect(getBestCandidateAtPoint(candidates, { x: 120, y: 120 })?.id).toBe(
      'window:settings',
    );
  });

  it('uses the smallest candidate when priorities match', () => {
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

    expect(getBestCandidateAtPoint(candidates, { x: 120, y: 120 })?.id).toBe(
      'window:inner',
    );
  });

  it('ignores tiny automatic hover candidates and uses the enclosing window', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'window:editor',
        kind: 'window',
        rect: { x: 100, y: 100, width: 700, height: 500 },
        priority: 10,
      },
      {
        id: 'window:tiny-overlay',
        kind: 'window',
        rect: { x: 180, y: 140, width: 18, height: 18 },
        priority: 10,
      },
    ];

    expect(getBestCandidateAtPoint(candidates, { x: 185, y: 145 })?.id).toBe(
      'window:editor',
    );
  });

  it('returns null when no candidate contains the point', () => {
    expect(
      getBestCandidateAtPoint(buildMonitorCandidates(monitors), {
        x: 2000,
        y: 20,
      }),
    ).toBeNull();
  });

  it('does not use monitor candidates as automatic hover selections', () => {
    expect(
      getBestCandidateAtPoint(buildMonitorCandidates(monitors), {
        x: 120,
        y: 120,
      }),
    ).toBeNull();
  });

  it('does not use full-monitor window candidates as automatic hover selections', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
      {
        id: 'window:desktop',
        kind: 'window',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 10,
      },
    ];

    expect(getBestCandidateAtPoint(candidates, { x: 120, y: 120 })).toBeNull();
  });

  it('ignores full-monitor window candidates and recommends the real window under the pointer', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
      {
        id: 'window:desktop',
        kind: 'window',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 10,
      },
      {
        id: 'window:editor',
        kind: 'window',
        rect: { x: 100, y: 100, width: 500, height: 400 },
        priority: 10,
      },
    ];

    expect(getBestCandidateAtPoint(candidates, { x: 120, y: 120 })?.id).toBe(
      'window:editor',
    );
  });

  it('cycles candidates under a point by priority and area', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
      {
        id: 'window:outer',
        kind: 'window',
        rect: { x: 100, y: 100, width: 700, height: 500 },
        priority: 10,
      },
      {
        id: 'window:inner',
        kind: 'window',
        rect: { x: 200, y: 150, width: 200, height: 150 },
        priority: 10,
      },
    ];

    const point = { x: 220, y: 170 };

    expect(getNextCandidateAtPoint(candidates, point, null, 1)?.id).toBe(
      'window:inner',
    );
    expect(
      getNextCandidateAtPoint(candidates, point, candidates[2].rect, 1)?.id,
    ).toBe('window:outer');
    expect(
      getNextCandidateAtPoint(candidates, point, candidates[1].rect, 1)?.id,
    ).toBe('monitor:primary');
    expect(
      getNextCandidateAtPoint(candidates, point, candidates[0].rect, 1)?.id,
    ).toBe('window:inner');
    expect(
      getNextCandidateAtPoint(candidates, point, candidates[2].rect, -1)?.id,
    ).toBe('monitor:primary');
  });

  it('falls back to the candidate under the pointer when no hover selection is active', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
      {
        id: 'window:settings',
        kind: 'window',
        rect: { x: 100, y: 100, width: 500, height: 400 },
        priority: 10,
      },
    ];

    expect(
      getCandidateForPointerCompletion(candidates, { x: 120, y: 120 }, null),
    )?.toMatchObject({ id: 'window:settings' });
  });

  it('falls back to the candidate under the pointer when the active hover selection moved away', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
      {
        id: 'window:settings',
        kind: 'window',
        rect: { x: 100, y: 100, width: 500, height: 400 },
        priority: 10,
      },
      {
        id: 'window:editor',
        kind: 'window',
        rect: { x: 700, y: 100, width: 400, height: 300 },
        priority: 10,
      },
    ];

    expect(
      getCandidateForPointerCompletion(
        candidates,
        { x: 740, y: 140 },
        candidates[1].rect,
      ),
    )?.toMatchObject({ id: 'window:editor' });
  });

  it('does not complete a candidate when pointer release produced a manual selection', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
      {
        id: 'window:settings',
        kind: 'window',
        rect: { x: 100, y: 100, width: 500, height: 400 },
        priority: 10,
      },
    ];

    expect(
      getCandidateForPointerReleaseCompletion(
        candidates,
        { x: 260, y: 220 },
        null,
        { x: 120, y: 120, width: 140, height: 100 },
        6,
      ),
    ).toBeNull();
  });

  it('completes the candidate under a click release when no hover state is active', () => {
    const candidates: CaptureCandidate[] = [
      {
        id: 'monitor:primary',
        kind: 'monitor',
        rect: { x: 0, y: 0, width: 1440, height: 900 },
        priority: 0,
      },
      {
        id: 'window:settings',
        kind: 'window',
        rect: { x: 100, y: 100, width: 500, height: 400 },
        priority: 10,
      },
    ];

    expect(
      getCandidateForPointerReleaseCompletion(
        candidates,
        { x: 120, y: 120 },
        null,
        { x: 120, y: 120, width: 0, height: 0 },
        6,
      ),
    )?.toMatchObject({ id: 'window:settings' });
  });

  it('does not complete a full monitor candidate from a plain click release', () => {
    expect(
      getCandidateForPointerReleaseCompletion(
        buildMonitorCandidates(monitors),
        { x: 120, y: 120 },
        null,
        { x: 120, y: 120, width: 0, height: 0 },
        6,
      ),
    ).toBeNull();
  });
});
