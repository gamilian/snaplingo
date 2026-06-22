import { invoke } from '@tauri-apps/api/core';

export type HotkeyCategory = 'screenshot' | 'translation' | 'ocr';

export type TranslationHotkeyAction =
  | 'selection-translate'
  | 'screenshot-translate'
  | 'input-translate'
  | 'show-window';

export async function configureHotkey(
  category: HotkeyCategory,
  action: string,
  hotkey: string,
) {
  return invoke<string | null>('configure_hotkey', {
    category,
    action,
    hotkey,
  });
}

export async function configureTranslationHotkey(
  action: TranslationHotkeyAction,
  hotkey: string,
) {
  return configureHotkey('translation', action, hotkey);
}
