import { create } from 'zustand';
import type {
  HotkeyConfigurationState,
  SettingsConfiguration,
} from '../application/settings/configuration';
import type {
  HotkeyCategory,
  HotkeySnapshot,
} from '../application/settings/ports';

type HotkeyConfiguration = SettingsConfiguration['hotkeys'];

let configuration: HotkeyConfiguration | null = null;
let unsubscribe: (() => void) | null = null;

export function initializeHotkeyConfigStore(runtime: HotkeyConfiguration) {
  unsubscribe?.();
  configuration = runtime;
  projectHotkeyState(runtime.getState());
  unsubscribe = runtime.subscribe(projectHotkeyState);
}

function runtime() {
  if (!configuration) {
    throw new Error('Hotkey config store runtime has not been initialized');
  }
  return configuration;
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

export const useHotkeyConfigStore = create<HotkeyConfigState>(() => ({
  hydrated: false,
  snapshot: null,
  defaultSnapshot: null,
  hydrate: () => runtime().hydrate(),
  refresh: () => runtime().refresh(),
  updateHotkey: (category, action, hotkey) =>
    runtime().update(category, action, hotkey),
  resetHotkey: (category, action) => runtime().reset(category, action),
  resetCategory: (category) => runtime().resetCategory(category),
}));

function projectHotkeyState(state: HotkeyConfigurationState) {
  useHotkeyConfigStore.setState({
    hydrated: state.hydrated,
    snapshot: state.snapshot,
    defaultSnapshot: state.defaultSnapshot,
  });
}
