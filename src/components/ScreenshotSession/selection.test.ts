import { describe, expect, it } from 'vitest';
import {
  getToolbarPosition,
  moveSelectionByDelta,
  nudgeSelection,
  resizeSelectionByHandle,
} from './selection';
import type { LogicalRect } from './types';

const bounds: LogicalRect = { x: 0, y: 0, width: 300, height: 200 };

describe('selection editing', () => {
  it('moves a selection while keeping it inside the capture bounds', () => {
    const rect: LogicalRect = { x: 40, y: 30, width: 100, height: 80 };

    expect(moveSelectionByDelta(rect, { x: 25, y: 10 }, bounds)).toEqual({
      x: 65,
      y: 40,
      width: 100,
      height: 80,
    });
    expect(moveSelectionByDelta(rect, { x: 400, y: 400 }, bounds)).toEqual({
      x: 200,
      y: 120,
      width: 100,
      height: 80,
    });
    expect(moveSelectionByDelta(rect, { x: -100, y: -100 }, bounds)).toEqual({
      x: 0,
      y: 0,
      width: 100,
      height: 80,
    });
  });

  it('resizes from handles and preserves a minimum size', () => {
    const rect: LogicalRect = { x: 40, y: 30, width: 100, height: 80 };

    expect(resizeSelectionByHandle(rect, 'se', { x: 30, y: 20 }, bounds, 10)).toEqual({
      x: 40,
      y: 30,
      width: 130,
      height: 100,
    });
    expect(resizeSelectionByHandle(rect, 'nw', { x: 120, y: 70 }, bounds, 10)).toEqual({
      x: 130,
      y: 100,
      width: 10,
      height: 10,
    });
    expect(resizeSelectionByHandle(rect, 'w', { x: -80, y: 0 }, bounds, 10)).toEqual({
      x: 0,
      y: 30,
      width: 140,
      height: 80,
    });
  });

  it('nudges a selection by keyboard direction and keeps it in bounds', () => {
    const rect: LogicalRect = { x: 40, y: 30, width: 100, height: 80 };

    expect(nudgeSelection(rect, 'ArrowRight', bounds, 1)).toEqual({
      x: 41,
      y: 30,
      width: 100,
      height: 80,
    });
    expect(nudgeSelection(rect, 'ArrowUp', bounds, 10)).toEqual({
      x: 40,
      y: 20,
      width: 100,
      height: 80,
    });
    expect(nudgeSelection(rect, 'ArrowLeft', bounds, 100)).toEqual({
      x: 0,
      y: 30,
      width: 100,
      height: 80,
    });
  });

  it('positions the toolbar near the selection within capture bounds', () => {
    const toolbarSize = { width: 120, height: 32 };

    expect(
      getToolbarPosition(
        { x: 40, y: 30, width: 100, height: 80 },
        bounds,
        toolbarSize,
        8,
      ),
    ).toEqual({ x: 40, y: 118 });
    expect(
      getToolbarPosition(
        { x: 230, y: 160, width: 60, height: 30 },
        bounds,
        toolbarSize,
        8,
      ),
    ).toEqual({ x: 180, y: 120 });
  });
});
