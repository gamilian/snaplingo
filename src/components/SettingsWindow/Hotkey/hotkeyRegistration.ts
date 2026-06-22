import type { HotkeyCategory } from '../../../tauri/hotkeys';

interface SaveHotkeyWithRegistrationInput {
  category: HotkeyCategory;
  action: string;
  hotkey: string;
  configureHotkey: (
    category: HotkeyCategory,
    action: string,
    hotkey: string,
  ) => Promise<unknown>;
  setHotkey: (category: HotkeyCategory, action: string, hotkey: string) => void;
  reportError: (message: string) => void;
}

export async function saveHotkeyWithRegistration({
  category,
  action,
  hotkey,
  configureHotkey,
  setHotkey,
  reportError,
}: SaveHotkeyWithRegistrationInput) {
  try {
    await configureHotkey(category, action, hotkey);
    setHotkey(category, action, hotkey);
    return true;
  } catch (err) {
    reportError(`快捷键 ${hotkey} 注册失败：${errorMessage(err)}`);
    return false;
  }
}

function errorMessage(err: unknown) {
  if (err instanceof Error) return err.message;
  return String(err);
}
