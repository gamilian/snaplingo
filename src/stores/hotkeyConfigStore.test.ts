import { beforeEach, describe, expect, it, vi } from 'vitest';

const hotkeysRuntime = vi.hoisted(() => ({
  load: vi.fn(),
  loadDefaults: vi.fn(),
  update: vi.fn(),
  reset: vi.fn(),
  resetCategory: vi.fn(),
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
    hotkeysRuntime.loadDefaults.mockResolvedValue(structuredClone(backendSnapshot));
  });

  it('hydrates once from the backend hotkey snapshot', async () => {
    const { initializeHotkeyConfigStore, useHotkeyConfigStore } =
      await import('./hotkeyConfigStore');
    initializeHotkeyConfigStore(hotkeysRuntime);

    await useHotkeyConfigStore.getState().hydrate();
    await useHotkeyConfigStore.getState().hydrate();

    expect(hotkeysRuntime.load).toHaveBeenCalledTimes(1);
    expect(hotkeysRuntime.loadDefaults).toHaveBeenCalledTimes(1);
    expect(useHotkeyConfigStore.getState()).toMatchObject({
      hydrated: true,
      snapshot: backendSnapshot,
    });
  });

  it('refreshes an already hydrated snapshot from the backend', async () => {
    const refreshedSnapshot = {
      ...backendSnapshot,
      screenshot: { screenshot: 'F12' },
    };
    const { initializeHotkeyConfigStore, useHotkeyConfigStore } =
      await import('./hotkeyConfigStore');
    initializeHotkeyConfigStore(hotkeysRuntime);

    await useHotkeyConfigStore.getState().hydrate();
    hotkeysRuntime.load.mockResolvedValueOnce(refreshedSnapshot);
    const snapshot = await useHotkeyConfigStore.getState().refresh();

    expect(hotkeysRuntime.load).toHaveBeenCalledTimes(2);
    expect(snapshot).toEqual(refreshedSnapshot);
    expect(useHotkeyConfigStore.getState().snapshot).toEqual(refreshedSnapshot);
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
    hotkeysRuntime.update.mockResolvedValueOnce({
      snapshot: changedSnapshot,
      accelerator: 'Shift+Alt+KeyD',
    });
    hotkeysRuntime.reset.mockResolvedValueOnce({
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

    expect(hotkeysRuntime.reset).toHaveBeenCalledWith(
      'translation',
      'selection-translate',
    );
    expect(useHotkeyConfigStore.getState().snapshot).toEqual(backendSnapshot);
  });

  it('resets a category through one backend operation', async () => {
    const { initializeHotkeyConfigStore, useHotkeyConfigStore } =
      await import('./hotkeyConfigStore');
    initializeHotkeyConfigStore(hotkeysRuntime);
    hotkeysRuntime.resetCategory.mockResolvedValueOnce(backendSnapshot);

    await useHotkeyConfigStore.getState().hydrate();
    await useHotkeyConfigStore.getState().resetCategory('screenshot');

    expect(hotkeysRuntime.resetCategory).toHaveBeenCalledWith('screenshot');
    expect(useHotkeyConfigStore.getState().snapshot).toEqual(backendSnapshot);
  });
});
