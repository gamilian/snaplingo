import { describe, expect, it, vi } from 'vitest';

import {
  dispatchCaptureWorkspacePreviewPointerDown,
  dispatchCaptureWorkspaceResizePointerDown,
} from './useCaptureWorkspaceController';

const selectionBounds = { x: -20, y: 10, width: 500, height: 300 };

function createPointerEvent(order: string[]) {
  return {
    clientX: 40,
    clientY: 50,
    pointerId: 7,
    button: 0,
    detail: 1,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    currentTarget: {
      setPointerCapture: vi.fn((pointerId: number) => {
        order.push(`capture:${pointerId}`);
      }),
    },
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  };
}

describe('capture workspace DOM pointer delegation', () => {
  it('captures the real preview pointer before runtime delegation', () => {
    const order: string[] = [];
    const event = createPointerEvent(order);
    const pointerDown = vi.fn(() => {
      order.push('runtime');
      return true;
    });

    expect(
      dispatchCaptureWorkspacePreviewPointerDown({
        event,
        selectionBounds,
        pointerDown,
      }),
    ).toBe(true);

    expect(order).toEqual(['capture:7', 'runtime']);
    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(event.stopPropagation).toHaveBeenCalledOnce();
  });

  it('captures the real resize pointer before runtime delegation', () => {
    const order: string[] = [];
    const event = createPointerEvent(order);
    const resizePointerDown = vi.fn(() => {
      order.push('runtime');
      return true;
    });

    expect(
      dispatchCaptureWorkspaceResizePointerDown({
        handle: 'se',
        event,
        selectionBounds,
        resizePointerDown,
      }),
    ).toBe(true);

    expect(order).toEqual(['capture:7', 'runtime']);
    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(event.stopPropagation).toHaveBeenCalledOnce();
  });

  it('does not capture or delegate ignored interactions without bounds', () => {
    const order: string[] = [];
    const previewEvent = createPointerEvent(order);
    const resizeEvent = createPointerEvent(order);
    const pointerDown = vi.fn(() => true);
    const resizePointerDown = vi.fn(() => true);

    expect(
      dispatchCaptureWorkspacePreviewPointerDown({
        event: previewEvent,
        selectionBounds: null,
        pointerDown,
      }),
    ).toBe(false);
    expect(
      dispatchCaptureWorkspaceResizePointerDown({
        handle: 'se',
        event: resizeEvent,
        selectionBounds: null,
        resizePointerDown,
      }),
    ).toBe(false);

    expect(order).toEqual([]);
    expect(pointerDown).not.toHaveBeenCalled();
    expect(resizePointerDown).not.toHaveBeenCalled();
  });
});
