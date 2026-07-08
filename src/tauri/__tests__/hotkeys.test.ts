import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('hotkeys tauri adapter', () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it('loads the backend hotkey snapshot through the hotkey command seam', async () => {
    const { getHotkeySnapshot } = await import('../hotkeys');
    invoke.mockResolvedValueOnce({
      screenshot: {
        screenshot: '⇧⌘R',
      },
      translation: {
        'selection-translate': '⌥D',
      },
      ocr: {
        'screenshot-ocr': '⇧⌥S',
      },
    });

    await expect(getHotkeySnapshot()).resolves.toEqual({
      screenshot: {
        screenshot: '⇧⌘R',
      },
      translation: {
        'selection-translate': '⌥D',
      },
      ocr: {
        'screenshot-ocr': '⇧⌥S',
      },
    });
    expect(invoke).toHaveBeenCalledWith('get_hotkey_snapshot');
  });

  it('updates a hotkey through the unified update command', async () => {
    const { updateHotkey } = await import('../hotkeys');
    invoke.mockResolvedValueOnce({
      snapshot: {
        screenshot: {},
        translation: {
          'selection-translate': '⇧⌥D',
        },
        ocr: {},
      },
      accelerator: 'Shift+Alt+KeyD',
    });

    await expect(
      updateHotkey({
        category: 'translation',
        action: 'selection-translate',
        hotkey: '⇧⌥D',
      }),
    ).resolves.toEqual({
      snapshot: {
        screenshot: {},
        translation: {
          'selection-translate': '⇧⌥D',
        },
        ocr: {},
      },
      accelerator: 'Shift+Alt+KeyD',
    });
    expect(invoke).toHaveBeenCalledWith('update_hotkey', {
      category: 'translation',
      action: 'selection-translate',
      hotkey: '⇧⌥D',
    });
  });
});
