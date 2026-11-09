import { describe, expect, it, vi } from 'vitest';
import { saveHotkeyWithRegistration } from './hotkeyRegistration';

describe('hotkey registration before persistence', () => {
  it('updates a hotkey through the backend-backed store', async () => {
    const updateHotkey = vi.fn(async () => ({
      translation: { 'selection-translate': '⇧⌘D' },
    }));
    const reportError = vi.fn();

    const saved = await saveHotkeyWithRegistration({
      category: 'translation',
      action: 'selection-translate',
      hotkey: '⇧⌘D',
      updateHotkey,
      reportError,
    });

    expect(saved).toBe(true);
    expect(updateHotkey).toHaveBeenCalledWith(
      'translation',
      'selection-translate',
      '⇧⌘D',
    );
    expect(reportError).not.toHaveBeenCalled();
  });

  it('reports backend update failures', async () => {
    const updateHotkey = vi.fn(async () => {
      throw new Error('Failed to register shortcut: already registered');
    });
    const reportError = vi.fn();

    const saved = await saveHotkeyWithRegistration({
      category: 'translation',
      action: 'selection-translate',
      hotkey: '⇧⌘D',
      updateHotkey,
      reportError,
    });

    expect(saved).toBe(false);
    expect(reportError).toHaveBeenCalledWith(
      '快捷键 ⇧⌘D 注册失败：Failed to register shortcut: already registered',
    );
  });
});
