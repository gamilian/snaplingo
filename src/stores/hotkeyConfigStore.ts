import { create } from 'zustand';
import type { SettingsRuntime } from '../application/settings/runtime';
import type {
  HotkeyCategory,
  HotkeySnapshot,
} from '../application/settings/ports';

type HotkeysRuntime = SettingsRuntime['hotkeys'];

let hotkeysRuntime: HotkeysRuntime | null = null;

export function initializeHotkeyConfigStore(runtime: HotkeysRuntime) {
  hotkeysRuntime = runtime;
}

function runtime() {
  if (!hotkeysRuntime) {
    throw new Error('Hotkey config store runtime has not been initialized');
  }

  return hotkeysRuntime;
}

interface HotkeyConfigState {
  hydrated: boolean;
  snapshot: HotkeySnapshot | null;
  defaultSnapshot: HotkeySnapshot | null;
  hydrate: () => Promise<HotkeySnapshot>;
  refresh: () => Promise<HotkeySnapshot>;
  updateHotkey: (
    category: HotkeyCategory,
    action: string,
    hotkey: string,
  ) => Promise<HotkeySnapshot>;
  resetHotkey: (category: HotkeyCategory, action: string) => Promise<HotkeySnapshot>;
  resetCategory: (category: HotkeyCategory) => Promise<HotkeySnapshot>;
}

function cloneSnapshot(snapshot: HotkeySnapshot): HotkeySnapshot {
  return {
    screenshot: { ...snapshot.screenshot },
    translation: { ...snapshot.translation },
    ocr: { ...snapshot.ocr },
  };
}

function applySnapshot(
  set: (partial: Partial<HotkeyConfigState>) => void,
  snapshot: HotkeySnapshot,
) {
  set({
    hydrated: true,
    snapshot: cloneSnapshot(snapshot),
  });
}

export const useHotkeyConfigStore = create<HotkeyConfigState>((set, get) => ({
  hydrated: false,
  snapshot: null,
  defaultSnapshot: null,
  hydrate: async () => {
    const state = get();
    const existingSnapshot = state.snapshot;

    if (state.hydrated && existingSnapshot) {
      return cloneSnapshot(existingSnapshot);
    }

    const [snapshot, defaults] = await Promise.all([
      runtime().load(),
      runtime().loadDefaults(),
    ]);
    set({
      hydrated: true,
      snapshot: cloneSnapshot(snapshot),
      defaultSnapshot: cloneSnapshot(defaults),
    });
    return cloneSnapshot(snapshot);
  },
  refresh: async () => {
    const snapshot = await runtime().load();
    applySnapshot(set, snapshot);
    return cloneSnapshot(snapshot);
  },
  updateHotkey: async (category, action, hotkey) => {
    const outcome = await runtime().update({ category, action, hotkey });
    applySnapshot(set, outcome.snapshot);
    return cloneSnapshot(outcome.snapshot);
  },
  resetHotkey: async (category, action) => {
    const outcome = await runtime().reset(category, action);
    applySnapshot(set, outcome.snapshot);
    return cloneSnapshot(outcome.snapshot);
  },
  resetCategory: async (category) => {
    const snapshot = await runtime().resetCategory(category);
    applySnapshot(set, snapshot);
    return cloneSnapshot(snapshot);
  },
}));
