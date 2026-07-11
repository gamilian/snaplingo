import { describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('Tauri hotkeys command adapter', () => {
  it('loads the hotkey snapshot', async () => {
    const { getHotkeySnapshot } = await import('./hotkeys');
    invoke.mockResolvedValueOnce({ screenshot: {}, translation: {}, ocr: {} });
    await getHotkeySnapshot();
    expect(invoke).toHaveBeenCalledWith('get_hotkey_snapshot');
  });

  it('updates a hotkey through the unified command', async () => {
    const { updateHotkey } = await import('./hotkeys');
    invoke.mockResolvedValueOnce({ snapshot: {}, accelerator: 'Alt+KeyD' });
    await updateHotkey({ category: 'translation', action: 'selection-translate', hotkey: '⌥D' });
    expect(invoke).toHaveBeenCalledWith('update_hotkey', {
      category: 'translation', action: 'selection-translate', hotkey: '⌥D',
    });
  });
});
