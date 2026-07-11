import { describe, expect, it } from 'vitest';
import {
  constrainSelectionPoint,
  getToolbarPosition,
  moveSelectionByDelta,
  moveDraftSelectionByDelta,
  nudgeResizedSelection,
  nudgeDraftSelection,
  nudgeMovedSelection,
  nudgeSelection,
  resizeSelectionBoundaryByArrow,
  resizeSelectionByHandle,
  restoreSelectionWithinBounds,
  snapMovedSelectionToRects,
  snapPointToRects,
  snapResizedSelectionToRects,
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

  it('moves an in-progress drawn selection and its resize anchor together', () => {
    const rect: LogicalRect = { x: 40, y: 30, width: 100, height: 80 };

    expect(
      moveDraftSelectionByDelta(rect, { x: 40, y: 30 }, { x: 25, y: 10 }, bounds),
    ).toEqual({
      selection: { x: 65, y: 40, width: 100, height: 80 },
      anchorPoint: { x: 65, y: 40 },
    });
  });

  it('nudges an in-progress drawn selection from its active cursor point', () => {
    expect(
      nudgeDraftSelection(
        { x: 40, y: 30 },
        { x: 90, y: 70 },
        { x: 1, y: 0 },
        bounds,
      ),
    ).toEqual({
      cursorPoint: { x: 91, y: 70 },
      selection: { x: 40, y: 30, width: 51, height: 40 },
    });
    expect(
      nudgeDraftSelection(
        { x: 90, y: 70 },
        { x: 40, y: 30 },
        { x: -1, y: 0 },
        bounds,
      ),
    ).toEqual({
      cursorPoint: { x: 39, y: 30 },
      selection: { x: 39, y: 30, width: 51, height: 40 },
    });
  });

  it('keeps an in-progress drawn selection cursor inside capture bounds', () => {
    expect(
      nudgeDraftSelection(
        { x: 40, y: 30 },
        { x: 0, y: 0 },
        { x: -1, y: -1 },
        bounds,
      ),
    ).toEqual({
      cursorPoint: { x: 0, y: 0 },
      selection: { x: 0, y: 0, width: 40, height: 30 },
    });
  });

  it('nudges a dragged selection and its active cursor point together', () => {
    expect(
      nudgeMovedSelection(
        { x: 40, y: 30, width: 100, height: 80 },
        { x: 90, y: 70 },
        { x: 1, y: 0 },
        bounds,
      ),
    ).toEqual({
      cursorPoint: { x: 91, y: 70 },
      selection: { x: 41, y: 30, width: 100, height: 80 },
    });
  });

  it('nudges a resized selection from its active handle point', () => {
    expect(
      nudgeResizedSelection(
        { x: 40, y: 30, width: 100, height: 80 },
        { x: 140, y: 110 },
        'se',
        { x: 1, y: 0 },
        bounds,
        10,
      ),
    ).toEqual({
      cursorPoint: { x: 141, y: 110 },
      selection: { x: 40, y: 30, width: 101, height: 80 },
    });
    expect(
      nudgeResizedSelection(
        { x: 40, y: 30, width: 100, height: 80 },
        { x: 40, y: 30 },
        'nw',
        { x: -1, y: -1 },
        bounds,
        10,
      ),
    ).toEqual({
      cursorPoint: { x: 39, y: 29 },
      selection: { x: 39, y: 29, width: 101, height: 81 },
    });
  });

  it('only moves a resized selection cursor by the clamped handle movement', () => {
    expect(
      nudgeResizedSelection(
        { x: 200, y: 120, width: 100, height: 80 },
        { x: 300, y: 200 },
        'se',
        { x: 1, y: 1 },
        bounds,
        10,
      ),
    ).toEqual({
      cursorPoint: { x: 300, y: 200 },
      selection: { x: 200, y: 120, width: 100, height: 80 },
    });
  });

  it('only moves a dragged selection cursor by the clamped movement', () => {
    expect(
      nudgeMovedSelection(
        { x: 0, y: 0, width: 100, height: 80 },
        { x: 20, y: 20 },
        { x: -1, y: -1 },
        bounds,
      ),
    ).toEqual({
      cursorPoint: { x: 20, y: 20 },
      selection: { x: 0, y: 0, width: 100, height: 80 },
    });
  });

  it('constrains a drawn selection point to a square from its anchor', () => {
    expect(constrainSelectionPoint({ x: 10, y: 10 }, { x: 40, y: 25 })).toEqual({
      x: 25,
      y: 25,
    });
    expect(constrainSelectionPoint({ x: 40, y: 40 }, { x: 15, y: 20 })).toEqual({
      x: 20,
      y: 20,
    });
  });

  it('updates the in-progress resize anchor by the clamped movement', () => {
    const rect: LogicalRect = { x: 40, y: 30, width: 100, height: 80 };

    expect(
      moveDraftSelectionByDelta(rect, { x: 40, y: 30 }, { x: -100, y: -100 }, bounds),
    ).toEqual({
      selection: { x: 0, y: 0, width: 100, height: 80 },
      anchorPoint: { x: 0, y: 0 },
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

  it('preserves selection aspect ratio when resizing from a corner handle', () => {
    const rect: LogicalRect = { x: 40, y: 30, width: 100, height: 50 };

    expect(
      resizeSelectionByHandle(rect, 'se', { x: 60, y: 5 }, bounds, 10, true),
    ).toEqual({
      x: 40,
      y: 30,
      width: 160,
      height: 80,
    });
    expect(
      resizeSelectionByHandle(rect, 'nw', { x: -40, y: -5 }, bounds, 10, true),
    ).toEqual({
      x: 0,
      y: 10,
      width: 140,
      height: 70,
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

  it('expands the corresponding selection boundary by keyboard direction', () => {
    const rect: LogicalRect = { x: 40, y: 30, width: 100, height: 80 };

    expect(resizeSelectionBoundaryByArrow(rect, 'ArrowLeft', 'expand', bounds, 10)).toEqual({
      x: 39,
      y: 30,
      width: 101,
      height: 80,
    });
    expect(resizeSelectionBoundaryByArrow(rect, 'ArrowRight', 'expand', bounds, 10)).toEqual({
      x: 40,
      y: 30,
      width: 101,
      height: 80,
    });
    expect(resizeSelectionBoundaryByArrow(rect, 'ArrowUp', 'expand', bounds, 10)).toEqual({
      x: 40,
      y: 29,
      width: 100,
      height: 81,
    });
    expect(resizeSelectionBoundaryByArrow(rect, 'ArrowDown', 'expand', bounds, 10)).toEqual({
      x: 40,
      y: 30,
      width: 100,
      height: 81,
    });
  });

  it('shrinks the corresponding selection boundary by keyboard direction', () => {
    const rect: LogicalRect = { x: 40, y: 30, width: 100, height: 80 };

    expect(resizeSelectionBoundaryByArrow(rect, 'ArrowLeft', 'shrink', bounds, 10)).toEqual({
      x: 41,
      y: 30,
      width: 99,
      height: 80,
    });
    expect(resizeSelectionBoundaryByArrow(rect, 'ArrowRight', 'shrink', bounds, 10)).toEqual({
      x: 40,
      y: 30,
      width: 99,
      height: 80,
    });
    expect(resizeSelectionBoundaryByArrow(rect, 'ArrowUp', 'shrink', bounds, 10)).toEqual({
      x: 40,
      y: 31,
      width: 100,
      height: 79,
    });
    expect(resizeSelectionBoundaryByArrow(rect, 'ArrowDown', 'shrink', bounds, 10)).toEqual({
      x: 40,
      y: 30,
      width: 100,
      height: 79,
    });
  });

  it('keeps keyboard boundary resizing within bounds and minimum size', () => {
    expect(
      resizeSelectionBoundaryByArrow(
        { x: 0, y: 0, width: 20, height: 20 },
        'ArrowLeft',
        'expand',
        bounds,
        10,
      ),
    ).toEqual({ x: 0, y: 0, width: 20, height: 20 });
    expect(
      resizeSelectionBoundaryByArrow(
        { x: 40, y: 30, width: 10, height: 20 },
        'ArrowLeft',
        'shrink',
        bounds,
        10,
      ),
    ).toEqual({ x: 40, y: 30, width: 10, height: 20 });
  });

  it('restores a previous selection inside the capture bounds', () => {
    expect(
      restoreSelectionWithinBounds(
        { x: 40, y: 30, width: 100, height: 80 },
        bounds,
        10,
      ),
    ).toEqual({ x: 40, y: 30, width: 100, height: 80 });
  });

  it('moves a previous selection back inside the capture bounds', () => {
    expect(
      restoreSelectionWithinBounds(
        { x: 260, y: 170, width: 100, height: 80 },
        bounds,
        10,
      ),
    ).toEqual({ x: 200, y: 120, width: 100, height: 80 });
    expect(
      restoreSelectionWithinBounds(
        { x: -20, y: -10, width: 100, height: 80 },
        bounds,
        10,
      ),
    ).toEqual({ x: 0, y: 0, width: 100, height: 80 });
  });

  it('drops previous selections that are too small or larger than the capture bounds', () => {
    expect(
      restoreSelectionWithinBounds(
        { x: 40, y: 30, width: 9, height: 80 },
        bounds,
        10,
      ),
    ).toBeNull();
    expect(
      restoreSelectionWithinBounds(
        { x: 40, y: 30, width: 100, height: 9 },
        bounds,
        10,
      ),
    ).toBeNull();
    expect(
      restoreSelectionWithinBounds(
        { x: 0, y: 0, width: 400, height: 80 },
        bounds,
        10,
      ),
    ).toBeNull();
    expect(
      restoreSelectionWithinBounds(
        { x: 0, y: 0, width: 100, height: 240 },
        bounds,
        10,
      ),
    ).toBeNull();
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

  it('keeps an oversized toolbar anchored inside the left capture bound', () => {
    expect(
      getToolbarPosition(
        { x: 40, y: 30, width: 100, height: 80 },
        bounds,
        { width: 680, height: 32 },
        8,
      ),
    ).toEqual({ x: 0, y: 118 });
  });

  it('snaps a drawn point to nearby target edges', () => {
    const targets: LogicalRect[] = [
      { x: 100, y: 50, width: 80, height: 60 },
    ];

    expect(snapPointToRects({ x: 97, y: 112 }, targets, 5)).toEqual({
      x: 100,
      y: 110,
    });
    expect(snapPointToRects({ x: 92, y: 112 }, targets, 5)).toEqual({
      x: 92,
      y: 110,
    });
  });

  it('snaps moved selection edges to nearby target edges', () => {
    const targets: LogicalRect[] = [
      { x: 150, y: 90, width: 80, height: 60 },
    ];
    const rect: LogicalRect = { x: 46, y: 33, width: 100, height: 60 };

    expect(snapMovedSelectionToRects(rect, targets, bounds, 5)).toEqual({
      x: 50,
      y: 30,
      width: 100,
      height: 60,
    });
  });

  it('snaps resized selection dragged edges without moving fixed edges', () => {
    const targets: LogicalRect[] = [
      { x: 150, y: 90, width: 80, height: 60 },
    ];
    const rect: LogicalRect = { x: 40, y: 30, width: 106, height: 58 };

    expect(snapResizedSelectionToRects(rect, 'se', targets, bounds, 10, 5)).toEqual({
      x: 40,
      y: 30,
      width: 110,
      height: 60,
    });
  });
});
