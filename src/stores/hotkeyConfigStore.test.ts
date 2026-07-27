import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { HotkeyConfigurationState } from '../application/settings/configuration';

const snapshot = {
  screenshot: { screenshot: 'Shift+Command+R' },
  translation: { 'selection-translate': 'Alt+D' },
  ocr: { 'screenshot-ocr': 'Shift+Alt+S' },
};

describe('hotkeyConfigStore projection', () => {
  beforeEach(() => vi.resetModules());

  it('projects Application state and forwards hotkey intents', async () => {
    let listener!: (state: {
      hydrated: boolean;
      snapshot: typeof snapshot | null;
      defaultSnapshot: typeof snapshot | null;
    }) => void;
    const configuration = {
      getState: (): HotkeyConfigurationState => ({
        hydrated: false,
        snapshot: null,
        defaultSnapshot: null,
      }),
      subscribe: vi.fn((next) => {
        listener = next;
        return () => undefined;
      }),
      hydrate: vi.fn(async () => snapshot),
      refresh: vi.fn(async () => snapshot),
      update: vi.fn(async () => snapshot),
      reset: vi.fn(async () => snapshot),
      resetCategory: vi.fn(async () => snapshot),
    };
    const { initializeHotkeyConfigStore, useHotkeyConfigStore } =
      await import('./hotkeyConfigStore');
    initializeHotkeyConfigStore(configuration);

    listener({ hydrated: true, snapshot, defaultSnapshot: snapshot });
    await useHotkeyConfigStore
      .getState()
      .updateHotkey('translation', 'selection-translate', 'Shift+Alt+D');

    expect(useHotkeyConfigStore.getState()).toMatchObject({
      hydrated: true,
      snapshot,
      defaultSnapshot: snapshot,
    });
    expect(configuration.update).toHaveBeenCalledWith(
      'translation',
      'selection-translate',
      'Shift+Alt+D',
    );
  });
});
