import { describe, expect, it } from 'vitest';
import {
  getCurrentMonitorBounds,
  getMonitorAtVirtualPoint,
  getMonitorViewportRect,
  getVirtualDesktopBounds,
  nudgeVirtualPoint,
  virtualPointToViewportPoint,
  virtualRectToViewportRect,
  viewportPointToVirtualPoint,
} from './virtualDesktop';
import type { MonitorSnapshotView } from './types';

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
  {
    id: 'top',
    logical_bounds: { x: 0, y: -600, width: 960, height: 600 },
    physical_bounds: { x: 0, y: -1200, width: 1920, height: 1200 },
    scale_factor: 2,
    image_base64: 'top-image',
  },
];

describe('virtual desktop geometry', () => {
  it('unions monitor logical bounds including negative coordinates', () => {
    expect(getVirtualDesktopBounds(monitors)).toEqual({
      x: -1280,
      y: -600,
      width: 2720,
      height: 1500,
    });
  });

  it('normalizes monitor and selection rectangles into viewport coordinates', () => {
    const bounds = getVirtualDesktopBounds(monitors);

    expect(getMonitorViewportRect(monitors[0], bounds)).toEqual({
      x: 0,
      y: 600,
      width: 1280,
      height: 720,
    });
    expect(
      virtualRectToViewportRect(
        { x: -100, y: 10, width: 80, height: 40 },
        bounds,
      ),
    ).toEqual({ x: 1180, y: 610, width: 80, height: 40 });
  });

  it('maps viewport pointer coordinates to virtual desktop coordinates', () => {
    const bounds = getVirtualDesktopBounds(monitors);

    expect(viewportPointToVirtualPoint({ x: 1180, y: 610 }, bounds)).toEqual({
      x: -100,
      y: 10,
    });
    expect(virtualPointToViewportPoint({ x: -100, y: 10 }, bounds)).toEqual({
      x: 1180,
      y: 610,
    });
  });

  it('finds the monitor under a virtual desktop point', () => {
    expect(getMonitorAtVirtualPoint(monitors, { x: -20, y: 20 })?.id).toBe('left');
    expect(getMonitorAtVirtualPoint(monitors, { x: 20, y: -20 })?.id).toBe('top');
    expect(getMonitorAtVirtualPoint(monitors, { x: 20, y: 20 })?.id).toBe('primary');
    expect(getMonitorAtVirtualPoint(monitors, { x: 2000, y: 20 })).toBeNull();
  });

  it('uses the monitor under the cursor for current full-screen selection bounds', () => {
    expect(getCurrentMonitorBounds(monitors, { x: 20, y: -20 })).toEqual({
      x: 0,
      y: -600,
      width: 960,
      height: 600,
    });
  });

  it('falls back to virtual desktop bounds when no current monitor is known', () => {
    expect(getCurrentMonitorBounds(monitors, null)).toEqual({
      x: -1280,
      y: -600,
      width: 2720,
      height: 1500,
    });
  });

  it('nudges virtual cursor points while keeping them inside desktop bounds', () => {
    const bounds = getVirtualDesktopBounds(monitors);

    expect(nudgeVirtualPoint({ x: 10, y: 10 }, { x: -1, y: 0 }, bounds)).toEqual({
      x: 9,
      y: 10,
    });
    expect(nudgeVirtualPoint({ x: -1280, y: -600 }, { x: -1, y: -1 }, bounds)).toEqual({
      x: -1280,
      y: -600,
    });
    expect(nudgeVirtualPoint({ x: 1439, y: 899 }, { x: 1, y: 1 }, bounds)).toEqual({
      x: 1439,
      y: 899,
    });
  });
});
