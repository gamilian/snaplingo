import type { HotkeyCategory } from '../../../application/settings/ports';

interface SaveHotkeyWithRegistrationInput {
  category: HotkeyCategory;
  action: string;
  hotkey: string;
  updateHotkey: (
    category: HotkeyCategory,
    action: string,
    hotkey: string,
  ) => Promise<unknown>;
  reportError: (message: string) => void;
}

export async function saveHotkeyWithRegistration({
  category,
  action,
  hotkey,
  updateHotkey,
  reportError,
}: SaveHotkeyWithRegistrationInput) {
  try {
    await updateHotkey(category, action, hotkey);
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
