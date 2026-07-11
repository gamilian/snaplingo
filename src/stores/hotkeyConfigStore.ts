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

async function defaultSnapshotFor(state: HotkeyConfigState) {
  return state.defaultSnapshot ?? state.hydrate();
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

    const snapshot = await runtime().load();
    set({
      hydrated: true,
      snapshot: cloneSnapshot(snapshot),
      defaultSnapshot: cloneSnapshot(snapshot),
    });
    return cloneSnapshot(snapshot);
  },
  updateHotkey: async (category, action, hotkey) => {
    const outcome = await runtime().update({ category, action, hotkey });
    applySnapshot(set, outcome.snapshot);
    return cloneSnapshot(outcome.snapshot);
  },
  resetHotkey: async (category, action) => {
    const defaults = await defaultSnapshotFor(get());
    const hotkey = defaults[category][action];

    if (typeof hotkey !== 'string') {
      throw new Error(`Unknown hotkey action '${category}:${action}'`);
    }

    return get().updateHotkey(category, action, hotkey);
  },
  resetCategory: async (category) => {
    const defaults = await defaultSnapshotFor(get());
    let snapshot = get().snapshot ?? (await get().hydrate());

    for (const [action, hotkey] of Object.entries(defaults[category])) {
      snapshot = await get().updateHotkey(category, action, hotkey);
    }

    return cloneSnapshot(snapshot);
  },
}));
