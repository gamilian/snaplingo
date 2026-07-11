import { beforeEach, describe, expect, it, vi } from 'vitest';

const hotkeysRuntime = vi.hoisted(() => ({
  load: vi.fn(),
  update: vi.fn(),
}));

const backendSnapshot = {
  screenshot: {
    screenshot: '⇧⌘R',
  },
  translation: {
    'selection-translate': '⌥D',
    'screenshot-translate': '⌥S',
  },
  ocr: {
    'screenshot-ocr': '⇧⌥S',
  },
};

describe('hotkeyConfigStore', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    hotkeysRuntime.load.mockResolvedValue(structuredClone(backendSnapshot));
  });

  it('hydrates once from the backend hotkey snapshot', async () => {
    const { initializeHotkeyConfigStore, useHotkeyConfigStore } =
      await import('./hotkeyConfigStore');
    initializeHotkeyConfigStore(hotkeysRuntime);

    await useHotkeyConfigStore.getState().hydrate();
    await useHotkeyConfigStore.getState().hydrate();

    expect(hotkeysRuntime.load).toHaveBeenCalledTimes(1);
    expect(useHotkeyConfigStore.getState()).toMatchObject({
      hydrated: true,
      snapshot: backendSnapshot,
    });
  });

  it('updates through the backend and applies the returned snapshot', async () => {
    const { initializeHotkeyConfigStore, useHotkeyConfigStore } =
      await import('./hotkeyConfigStore');
    initializeHotkeyConfigStore(hotkeysRuntime);
    const updatedSnapshot = {
      ...backendSnapshot,
      translation: {
        ...backendSnapshot.translation,
        'selection-translate': '⇧⌥D',
      },
    };
    hotkeysRuntime.update.mockResolvedValueOnce({
      snapshot: updatedSnapshot,
      accelerator: 'Shift+Alt+KeyD',
    });

    await useHotkeyConfigStore.getState().hydrate();
    const snapshot = await useHotkeyConfigStore
      .getState()
      .updateHotkey('translation', 'selection-translate', '⇧⌥D');

    expect(hotkeysRuntime.update).toHaveBeenCalledWith({
      category: 'translation',
      action: 'selection-translate',
      hotkey: '⇧⌥D',
    });
    expect(snapshot).toEqual(updatedSnapshot);
    expect(useHotkeyConfigStore.getState().snapshot).toEqual(updatedSnapshot);
  });

  it('resets one hotkey from the backend-provided default snapshot', async () => {
    const { initializeHotkeyConfigStore, useHotkeyConfigStore } =
      await import('./hotkeyConfigStore');
    initializeHotkeyConfigStore(hotkeysRuntime);
    const changedSnapshot = {
      ...backendSnapshot,
      translation: {
        ...backendSnapshot.translation,
        'selection-translate': '⇧⌥D',
      },
    };
    hotkeysRuntime.update
      .mockResolvedValueOnce({
        snapshot: changedSnapshot,
        accelerator: 'Shift+Alt+KeyD',
      })
      .mockResolvedValueOnce({
        snapshot: backendSnapshot,
        accelerator: 'Alt+KeyD',
      });

    await useHotkeyConfigStore.getState().hydrate();
    await useHotkeyConfigStore
      .getState()
      .updateHotkey('translation', 'selection-translate', '⇧⌥D');
    await useHotkeyConfigStore
      .getState()
      .resetHotkey('translation', 'selection-translate');

    expect(hotkeysRuntime.update).toHaveBeenLastCalledWith({
      category: 'translation',
      action: 'selection-translate',
      hotkey: '⌥D',
    });
    expect(useHotkeyConfigStore.getState().snapshot).toEqual(backendSnapshot);
  });
});
