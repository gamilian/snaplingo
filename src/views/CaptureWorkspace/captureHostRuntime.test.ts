import { describe, expect, it, vi } from 'vitest';

import {
  prepareCaptureSurfaceForReveal,
  recordSuccessfulCaptureSelection,
  restoreCaptureSelectionFromHistory,
  restoreLastSuccessfulCaptureSelection,
} from './captureHostRuntime';

const selection = { x: 20, y: 30, width: 120, height: 80 };

function createStorage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
  };
}

describe('capture host view helpers', () => {
  it('records successful selections and restores the latest within bounds', () => {
    const storage = createStorage();
    const completeSelection = vi.fn();
    recordSuccessfulCaptureSelection(storage, 'copy', selection);

    restoreLastSuccessfulCaptureSelection({
      storage,
      selectionBounds: { x: 0, y: 0, width: 500, height: 300 },
      minSelectionSize: 10,
      completeSelection,
    });

    expect(completeSelection).toHaveBeenCalledWith(selection);
  });

  it('does not record unsuccessful completion actions', () => {
    const storage = createStorage();
    recordSuccessfulCaptureSelection(storage, 'ocr', selection);
    const completeSelection = vi.fn();

    restoreLastSuccessfulCaptureSelection({
      storage,
      selectionBounds: { x: 0, y: 0, width: 500, height: 300 },
      minSelectionSize: 10,
      completeSelection,
    });

    expect(completeSelection).not.toHaveBeenCalled();
  });

  it('restores neighboring history entries and prepares the reveal surface', async () => {
    const storage = createStorage();
    const previous = { x: 1, y: 2, width: 30, height: 40 };
    recordSuccessfulCaptureSelection(storage, 'copy', previous);
    recordSuccessfulCaptureSelection(storage, 'copy', selection);
    const completeSelection = vi.fn();

    restoreCaptureSelectionFromHistory({
      storage,
      currentSelection: selection,
      step: 'previous',
      selectionBounds: { x: 0, y: 0, width: 500, height: 300 },
      minSelectionSize: 10,
      completeSelection,
    });

    const frame = { variant: 'preview' as const, rect: selection, label: '120 × 80' };
    const paint = vi.fn();
    const waitForPaint = vi.fn(async () => undefined);
    await prepareCaptureSurfaceForReveal({
      frame,
      paintSelectionOverlayFrame: paint,
      waitForPaint,
    });

    expect(completeSelection).toHaveBeenCalledWith(previous);
    expect(paint).toHaveBeenCalledWith(frame);
    expect(waitForPaint).toHaveBeenCalledTimes(1);
  });
});
