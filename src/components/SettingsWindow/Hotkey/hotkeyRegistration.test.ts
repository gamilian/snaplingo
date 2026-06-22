import { describe, expect, it, vi } from 'vitest';
import { saveHotkeyWithRegistration } from './hotkeyRegistration';

describe('hotkey registration before persistence', () => {
  it('persists a hotkey only after backend registration succeeds', async () => {
    const configureHotkey = vi.fn(async () => 'Shift+CmdOrCtrl+KeyD');
    const setHotkey = vi.fn();
    const reportError = vi.fn();

    const saved = await saveHotkeyWithRegistration({
      category: 'translation',
      action: 'selection-translate',
      hotkey: '⇧⌘D',
      configureHotkey,
      setHotkey,
      reportError,
    });

    expect(saved).toBe(true);
    expect(configureHotkey).toHaveBeenCalledWith(
      'translation',
      'selection-translate',
      '⇧⌘D',
    );
    expect(setHotkey).toHaveBeenCalledWith(
      'translation',
      'selection-translate',
      '⇧⌘D',
    );
    expect(reportError).not.toHaveBeenCalled();
  });

  it('does not persist a hotkey when backend registration fails', async () => {
    const configureHotkey = vi.fn(async () => {
      throw new Error('Failed to register shortcut: already registered');
    });
    const setHotkey = vi.fn();
    const reportError = vi.fn();

    const saved = await saveHotkeyWithRegistration({
      category: 'translation',
      action: 'selection-translate',
      hotkey: '⇧⌘D',
      configureHotkey,
      setHotkey,
      reportError,
    });

    expect(saved).toBe(false);
    expect(setHotkey).not.toHaveBeenCalled();
    expect(reportError).toHaveBeenCalledWith(
      '快捷键 ⇧⌘D 注册失败：Failed to register shortcut: already registered',
    );
  });
});
