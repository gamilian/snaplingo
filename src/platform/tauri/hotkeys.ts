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

export async function getHotkeySnapshot(): Promise<HotkeySnapshot> {
  return invoke<HotkeySnapshot>('get_hotkey_snapshot');
}

export async function getDefaultHotkeySnapshot(): Promise<HotkeySnapshot> {
  return invoke<HotkeySnapshot>('get_default_hotkey_snapshot');
}

export async function updateHotkey(input: HotkeyUpdateInput): Promise<HotkeyUpdateOutcome> {
  return invoke<HotkeyUpdateOutcome>('update_hotkey', {
    category: input.category,
    action: input.action,
    hotkey: input.hotkey,
  });
}

export async function resetHotkey(
  category: HotkeyCategory,
  action: string,
): Promise<HotkeyUpdateOutcome> {
  return invoke<HotkeyUpdateOutcome>('reset_hotkey', { category, action });
}

export async function resetHotkeyCategory(
  category: HotkeyCategory,
): Promise<HotkeySnapshot> {
  return invoke<HotkeySnapshot>('reset_hotkey_category', { category });
}
