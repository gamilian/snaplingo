import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

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

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

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
    vi.useFakeTimers();
    invoke.mockReset().mockResolvedValue(undefined);
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

  afterEach(() => vi.useRealTimers());

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

    const drag = resultWindow.startDragging();
    await vi.runAllTimersAsync();
    await expect(drag).resolves.toEqual({
      x: 1600,
      y: 720,
    });

    expect(invoke).toHaveBeenCalledExactlyOnceWith('start_result_window_drag');
    expect(startDragging).not.toHaveBeenCalled();
    expect(onMoved).toHaveBeenCalledOnce();
  });

  it('starts the native drag immediately while the move listener is still registering', async () => {
    let register!: (unlisten: () => void) => void;
    const unlisten = vi.fn();
    onMoved.mockReturnValue(new Promise<() => void>((resolve) => {
      register = resolve;
    }));
    const drag = resultWindow.startDragging();

    try {
      expect(invoke).toHaveBeenCalledExactlyOnceWith('start_result_window_drag');
    } finally {
      register(unlisten);
      await vi.runAllTimersAsync();
      await drag;
    }
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('returns the latest moved position without repositioning the window', async () => {
    let moved!: (event: { payload: { x: number; y: number } }) => void;
    const unlisten = vi.fn();
    onMoved.mockImplementation(async (handler) => {
      moved = handler;
      return unlisten;
    });
    const drag = resultWindow.startDragging();
    moved({ payload: { x: 100, y: 200 } });
    await vi.advanceTimersByTimeAsync(200);
    moved({ payload: { x: 220, y: 260 } });
    await vi.runAllTimersAsync();

    await expect(drag).resolves.toEqual({ x: 220, y: 260 });
    expect(setPosition).not.toHaveBeenCalled();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('reads the actual position when the user releases without moving', async () => {
    const drag = resultWindow.startDragging();
    await vi.runAllTimersAsync();

    await expect(drag).resolves.toEqual({ x: 1420, y: 360 });
  });

  it('removes a late-registered listener after native dragging fails', async () => {
    let register!: (unlisten: () => void) => void;
    const unlisten = vi.fn();
    onMoved.mockReturnValue(new Promise<() => void>((resolve) => {
      register = resolve;
    }));
    invoke.mockRejectedValue(new Error('drag failed'));

    await expect(resultWindow.startDragging()).rejects.toThrow('drag failed');
    register(unlisten);
    await vi.runAllTimersAsync();

    expect(unlisten).toHaveBeenCalledOnce();
    expect(outerPosition).not.toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('restores and clamps the durable last user-dragged position', async () => {
    await resultWindow.place('last-position', { x: 2100, y: 800 });

    expect(monitorFromPoint).toHaveBeenCalledWith(2100, 800);
    expect(setPosition).toHaveBeenCalledWith({ x: 1600, y: 600 });
  });
});
