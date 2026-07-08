import { create } from 'zustand';
import {
  getHotkeySnapshot as loadHotkeySnapshot,
  updateHotkey as persistHotkey,
  type HotkeyCategory,
  type HotkeySnapshot,
} from '../tauri/hotkeys';

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
    if (get().hydrated && get().snapshot) {
      return cloneSnapshot(get().snapshot);
    }

    const snapshot = await loadHotkeySnapshot();
    set({
      hydrated: true,
      snapshot: cloneSnapshot(snapshot),
      defaultSnapshot: cloneSnapshot(snapshot),
    });
    return cloneSnapshot(snapshot);
  },
  updateHotkey: async (category, action, hotkey) => {
    const outcome = await persistHotkey({ category, action, hotkey });
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
