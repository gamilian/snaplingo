import { describe, expect, it, vi } from 'vitest';
import {
  createInitialResultWindowState,
  type ResultWindowState,
} from '../application/result-window/projection';
import { initializeResultWindowStore, useResultWindowStore } from './resultWindowStore';

describe('result window projection adapter', () => {
  it('publishes application snapshots without owning workflow mutations', () => {
    const initial = createInitialResultWindowState();
    let publish!: (state: ResultWindowState) => void;
    initializeResultWindowStore({
      getState: () => initial,
      subscribe: (listener) => { publish = listener; return vi.fn(); },
    });
    expect(useResultWindowStore.getState()).toEqual(initial);
    const next = { ...initial, sourceText: 'hello', resultWindowVisible: true };
    publish(next);
    expect(useResultWindowStore.getState()).toBe(next);
    expect(useResultWindowStore.getState()).not.toHaveProperty('startTranslationSession');
  });

  it('unsubscribes the old runtime before initializing another projection', () => {
    const unsubscribe = vi.fn();
    initializeResultWindowStore({
      getState: createInitialResultWindowState,
      subscribe: () => unsubscribe,
    });
    initializeResultWindowStore({
      getState: () => ({ ...createInitialResultWindowState(), sourceText: 'replacement' }),
      subscribe: () => vi.fn(),
    });
    expect(unsubscribe).toHaveBeenCalledOnce();
    expect(useResultWindowStore.getState().sourceText).toBe('replacement');
  });
});
