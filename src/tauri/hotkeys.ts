import { invoke } from '@tauri-apps/api/core';

export type HotkeyCategory = 'screenshot' | 'translation' | 'ocr';

export type HotkeySnapshot = Record<HotkeyCategory, Record<string, string>>;

export interface HotkeyUpdateInput {
  category: HotkeyCategory;
  action: string;
  hotkey: string;
}

export interface HotkeyUpdateOutcome {
  snapshot: HotkeySnapshot;
  accelerator: string | null;
}

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

export async function getHotkeySnapshot(): Promise<HotkeySnapshot> {
  return invoke<HotkeySnapshot>('get_hotkey_snapshot');
}

export async function updateHotkey(input: HotkeyUpdateInput): Promise<HotkeyUpdateOutcome> {
  return invoke<HotkeyUpdateOutcome>('update_hotkey', {
    category: input.category,
    action: input.action,
    hotkey: input.hotkey,
  });
}
