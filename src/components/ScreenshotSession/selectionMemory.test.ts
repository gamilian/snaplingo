import { describe, expect, it } from 'vitest';
import {
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
