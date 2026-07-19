import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
  currentMonitor,
  cursorPosition,
  getCurrentWindow,
  hide,
  innerSize,
  monitorFromPoint,
  onMoved,
  outerPosition,
  setPosition,
  setSize,
  startDragging,
} = vi.hoisted(() => {
  const hide = vi.fn();
  const setSize = vi.fn();
  const startDragging = vi.fn();
  const innerSize = vi.fn();
  const setPosition = vi.fn();
  const outerPosition = vi.fn();
  const onMoved = vi.fn();

  return {
    currentMonitor: vi.fn(),
    cursorPosition: vi.fn(),
    getCurrentWindow: vi.fn(() => ({
      hide,
      innerSize,
      setPosition,
      outerPosition,
      onMoved,
      setSize,
      startDragging,
    })),
    hide,
    innerSize,
    monitorFromPoint: vi.fn(),
    setPosition,
    setSize,
    startDragging,
    outerPosition,
    onMoved,
  };
});

vi.mock('@tauri-apps/api/window', () => ({
  currentMonitor,
  cursorPosition,
  getCurrentWindow,
  LogicalSize: class LogicalSize {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
  monitorFromPoint,
  PhysicalPosition: class PhysicalPosition {
    constructor(
      public x: number,
      public y: number,
    ) {}
  },
}));

import { resultWindow } from './resultWindow';

describe('Tauri result window adapter', () => {
  beforeEach(() => {
    getCurrentWindow.mockClear();
    hide.mockReset().mockResolvedValue(undefined);
    setSize.mockReset().mockResolvedValue(undefined);
    setPosition.mockReset().mockResolvedValue(undefined);
    cursorPosition.mockReset().mockResolvedValue({ x: 1500, y: 900 });
    monitorFromPoint.mockReset().mockResolvedValue({
      scaleFactor: 2,
      workArea: {
        position: { x: 1000, y: 100 },
        size: { width: 1200, height: 900 },
      },
    });
    currentMonitor.mockReset().mockResolvedValue(null);
    innerSize.mockReset().mockResolvedValue({ width: 600, height: 400 });
    startDragging.mockReset().mockResolvedValue(undefined);
    outerPosition.mockReset().mockResolvedValue({ x: 1420, y: 360 });
    onMoved.mockReset().mockResolvedValue(vi.fn());
  });

  it('resizes the current result window from portable dimensions', async () => {
    await resultWindow.resize(660, 480);

    expect(setSize).toHaveBeenCalledWith({ width: 660, height: 480 });
  });

  it('hides the current result window instead of closing it', async () => {
    await resultWindow.hide();

    expect(hide).toHaveBeenCalledOnce();
  });

  it('centers the result window inside the cursor monitor work area', async () => {
    await resultWindow.place('center');

    expect(setPosition).toHaveBeenCalledWith({ x: 1300, y: 350 });
  });

  it('places below the cursor and clamps the window into the work area', async () => {
    cursorPosition.mockResolvedValue({ x: 2100, y: 800 });

    await resultWindow.place('below-cursor');

    expect(setPosition).toHaveBeenCalledWith({ x: 1600, y: 600 });
  });

  it('starts dragging the current result window', async () => {
    onMoved.mockImplementation(async (handler: (event: {
      payload: { x: number; y: number };
    }) => void) => {
      handler({ payload: { x: 1600, y: 720 } });
      return vi.fn();
    });

    await expect(resultWindow.startDragging()).resolves.toEqual({
      x: 1600,
      y: 720,
    });

    expect(startDragging).toHaveBeenCalledOnce();
    expect(onMoved).toHaveBeenCalledOnce();
  });

  it('restores and clamps the durable last user-dragged position', async () => {
    await resultWindow.place('last-position', { x: 2100, y: 800 });

    expect(monitorFromPoint).toHaveBeenCalledWith(2100, 800);
    expect(setPosition).toHaveBeenCalledWith({ x: 1600, y: 600 });
  });
});
