import { describe, expect, it } from 'vitest';
import {
  getSelectionHistoryEntry,
  loadCaptureSelectionHistory,
  loadLastCaptureSelection,
  saveLastCaptureSelection,
} from './selectionMemory';
import type { LogicalRect } from './types';

function createMemoryStorage() {
  const values = new Map<string, string>();

  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
}

describe('capture selection memory', () => {
  it('stores and loads the last successful capture selection', () => {
    const storage = createMemoryStorage();
    const rect: LogicalRect = { x: 10, y: 20, width: 120, height: 80 };

    saveLastCaptureSelection(storage, rect);

    expect(loadLastCaptureSelection(storage)).toEqual(rect);
  });

  it('stores successful capture selections in newest-first history order', () => {
    const storage = createMemoryStorage();
    const first: LogicalRect = { x: 10, y: 20, width: 120, height: 80 };
    const second: LogicalRect = { x: 30, y: 40, width: 100, height: 60 };
    const third: LogicalRect = { x: 50, y: 60, width: 90, height: 50 };

    saveLastCaptureSelection(storage, first);
    saveLastCaptureSelection(storage, second);
    saveLastCaptureSelection(storage, third);

    expect(loadCaptureSelectionHistory(storage)).toEqual([third, second, first]);
  });

  it('moves duplicate capture selections to the front of history', () => {
    const storage = createMemoryStorage();
    const first: LogicalRect = { x: 10, y: 20, width: 120, height: 80 };
    const second: LogicalRect = { x: 30, y: 40, width: 100, height: 60 };

    saveLastCaptureSelection(storage, first);
    saveLastCaptureSelection(storage, second);
    saveLastCaptureSelection(storage, first);

    expect(loadCaptureSelectionHistory(storage)).toEqual([first, second]);
  });

  it('caps the stored capture selection history', () => {
    const storage = createMemoryStorage();

    saveLastCaptureSelection(storage, { x: 0, y: 0, width: 10, height: 10 }, 2);
    saveLastCaptureSelection(storage, { x: 1, y: 1, width: 10, height: 10 }, 2);
    saveLastCaptureSelection(storage, { x: 2, y: 2, width: 10, height: 10 }, 2);

    expect(loadCaptureSelectionHistory(storage)).toEqual([
      { x: 2, y: 2, width: 10, height: 10 },
      { x: 1, y: 1, width: 10, height: 10 },
    ]);
  });

  it('cycles capture selection history from older to newer entries', () => {
    const newest: LogicalRect = { x: 50, y: 60, width: 90, height: 50 };
    const middle: LogicalRect = { x: 30, y: 40, width: 100, height: 60 };
    const oldest: LogicalRect = { x: 10, y: 20, width: 120, height: 80 };
    const history = [newest, middle, oldest];

    expect(getSelectionHistoryEntry(history, null, 'previous')).toEqual(newest);
    expect(getSelectionHistoryEntry(history, middle, 'previous')).toEqual(oldest);
    expect(getSelectionHistoryEntry(history, middle, 'next')).toEqual(newest);
    expect(getSelectionHistoryEntry(history, oldest, 'previous')).toEqual(newest);
    expect(getSelectionHistoryEntry(history, newest, 'next')).toEqual(oldest);
    expect(
      getSelectionHistoryEntry(
        history,
        { x: 0, y: 0, width: 10, height: 10 },
        'next',
      ),
    ).toEqual(newest);
  });

  it('ignores missing or malformed stored selections', () => {
    const storage = createMemoryStorage();

    expect(loadLastCaptureSelection(storage)).toBeNull();

    storage.setItem('snaplingo:last-capture-selection', '{nope');

    expect(loadLastCaptureSelection(storage)).toBeNull();

    storage.setItem(
      'snaplingo:last-capture-selection',
      JSON.stringify({ x: 10, y: 20, width: '120', height: 80 }),
    );

    expect(loadLastCaptureSelection(storage)).toBeNull();
  });

  it('ignores storage read failures', () => {
    const storage = {
      getItem: () => {
        throw new Error('storage unavailable');
      },
      setItem: () => undefined,
    };

    expect(loadLastCaptureSelection(storage)).toBeNull();
  });
});
