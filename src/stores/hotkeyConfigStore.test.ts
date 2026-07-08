import { beforeEach, describe, expect, it, vi } from 'vitest';

const hotkeysApi = vi.hoisted(() => ({
  getHotkeySnapshot: vi.fn(),
  updateHotkey: vi.fn(),
}));

vi.mock('../tauri/hotkeys', () => hotkeysApi);

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
    hotkeysApi.getHotkeySnapshot.mockResolvedValue(structuredClone(backendSnapshot));
  });

  it('hydrates once from the backend hotkey snapshot', async () => {
    const { useHotkeyConfigStore } = await import('./hotkeyConfigStore');

    await useHotkeyConfigStore.getState().hydrate();
    await useHotkeyConfigStore.getState().hydrate();

    expect(hotkeysApi.getHotkeySnapshot).toHaveBeenCalledTimes(1);
    expect(useHotkeyConfigStore.getState()).toMatchObject({
      hydrated: true,
      snapshot: backendSnapshot,
    });
  });

  it('updates through the backend and applies the returned snapshot', async () => {
    const { useHotkeyConfigStore } = await import('./hotkeyConfigStore');
    const updatedSnapshot = {
      ...backendSnapshot,
      translation: {
        ...backendSnapshot.translation,
        'selection-translate': '⇧⌥D',
      },
    };
    hotkeysApi.updateHotkey.mockResolvedValueOnce({
      snapshot: updatedSnapshot,
      accelerator: 'Shift+Alt+KeyD',
    });

    await useHotkeyConfigStore.getState().hydrate();
    const snapshot = await useHotkeyConfigStore
      .getState()
      .updateHotkey('translation', 'selection-translate', '⇧⌥D');

    expect(hotkeysApi.updateHotkey).toHaveBeenCalledWith({
      category: 'translation',
      action: 'selection-translate',
      hotkey: '⇧⌥D',
    });
    expect(snapshot).toEqual(updatedSnapshot);
    expect(useHotkeyConfigStore.getState().snapshot).toEqual(updatedSnapshot);
  });

  it('resets one hotkey from the backend-provided default snapshot', async () => {
    const { useHotkeyConfigStore } = await import('./hotkeyConfigStore');
    const changedSnapshot = {
      ...backendSnapshot,
      translation: {
        ...backendSnapshot.translation,
        'selection-translate': '⇧⌥D',
      },
    };
    hotkeysApi.updateHotkey
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

    expect(hotkeysApi.updateHotkey).toHaveBeenLastCalledWith({
      category: 'translation',
      action: 'selection-translate',
      hotkey: '⌥D',
    });
    expect(useHotkeyConfigStore.getState().snapshot).toEqual(backendSnapshot);
  });
});
