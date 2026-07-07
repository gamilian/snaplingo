import { describe, expect, it } from 'vitest';
import {
  getCaptureMagnifierRuntimeState,
  shouldHydrateCaptureMagnifierPixels,
} from './captureMagnifierRuntime';
import type { CaptureSessionView } from './types';

function createCaptureSessionView(): CaptureSessionView {
  return {
    id: 'session-1',
    monitors: [
      {
        id: 'left',
        logical_bounds: { x: -300, y: 0, width: 300, height: 200 },
        physical_bounds: { x: -600, y: 0, width: 600, height: 400 },
        scale_factor: 2,
        image_base64: '',
      },
      {
        id: 'main',
        logical_bounds: { x: 0, y: 0, width: 500, height: 300 },
        physical_bounds: { x: 0, y: 0, width: 1000, height: 600 },
        scale_factor: 2,
        image_base64: 'pixels',
      },
    ],
    candidates: [],
    captured_cursor: null,
  };
}

describe('captureMagnifierRuntime', () => {
  it('hydrates snapshot pixels only after the magnifier is explicitly requested', () => {
    expect(
      shouldHydrateCaptureMagnifierPixels({
        hasSession: true,
        hasHydratedPixelSource: false,
        isMagnifierRequested: true,
      }),
    ).toBe(true);

    expect(
      shouldHydrateCaptureMagnifierPixels({
        hasSession: true,
        hasHydratedPixelSource: false,
        isMagnifierRequested: false,
      }),
    ).toBe(false);

    expect(
      shouldHydrateCaptureMagnifierPixels({
        hasSession: true,
        hasHydratedPixelSource: true,
        isMagnifierRequested: true,
      }),
    ).toBe(false);
  });

  it('derives the active magnifier monitor, image point, and visibility state', () => {
    const state = getCaptureMagnifierRuntimeState({
      session: createCaptureSessionView(),
      status: 'selecting',
      cursorPoint: { x: 120, y: 80 },
      cursorViewportPoint: { x: 420, y: 80 },
      viewportBounds: { x: 0, y: 0, width: 800, height: 300 },
      isMagnifierRequested: false,
    });

    expect(state.hasHydratedPixelSource).toBe(true);
    expect(state.cursorMonitor?.id).toBe('main');
    expect(state.cursorInMonitorPoint).toEqual({ x: 120, y: 80 });
    expect(state.shouldTrackMagnifierCursor).toBe(true);
    expect(state.isMagnifierShown).toBe(true);
  });

  it('keeps the magnifier hidden when the cursor monitor has no hydrated pixels', () => {
    const state = getCaptureMagnifierRuntimeState({
      session: createCaptureSessionView(),
      status: 'selecting',
      cursorPoint: { x: -120, y: 80 },
      cursorViewportPoint: { x: 180, y: 80 },
      viewportBounds: { x: 0, y: 0, width: 800, height: 300 },
      isMagnifierRequested: true,
    });

    expect(state.hasHydratedPixelSource).toBe(true);
    expect(state.cursorMonitor?.id).toBe('left');
    expect(state.cursorInMonitorPoint).toEqual({ x: 180, y: 80 });
    expect(state.shouldTrackMagnifierCursor).toBe(true);
    expect(state.isMagnifierShown).toBe(false);
  });
});
