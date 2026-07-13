import { create } from 'zustand';
import type { SettingsRuntime } from '../application/settings/runtime';
import type {
  AnnotationColorPreset,
  GeneralSettings,
  ScreenshotSettings,
  SettingsSnapshot,
  TranslationSettings,
} from '../application/settings/ports';

type DurableSettingsRuntime = SettingsRuntime['durableSettings'];

let durableSettingsRuntime: DurableSettingsRuntime | null = null;

export function initializeSettingsConfigStore(runtime: DurableSettingsRuntime) {
  durableSettingsRuntime = runtime;
}

function settingsRuntime() {
  if (!durableSettingsRuntime) {
    throw new Error('Settings config store runtime has not been initialized');
  }

  return durableSettingsRuntime;
}

interface SettingsConfigState {
  hydrated: boolean;
  general: GeneralSettings | null;
  screenshot: ScreenshotSettings | null;
  translation: TranslationSettings | null;
  hydrate: () => Promise<SettingsSnapshot>;
  refresh: () => Promise<SettingsSnapshot>;
  updateGeneralSettings: (input: GeneralSettings) => Promise<SettingsSnapshot>;
  updateScreenshotSettings: (input: ScreenshotSettings) => Promise<SettingsSnapshot>;
  updateAnnotationColors: (
    colors: AnnotationColorPreset[],
  ) => Promise<SettingsSnapshot>;
  updateTranslationSettings: (input: TranslationSettings) => Promise<SettingsSnapshot>;
}

function applySnapshot(
  set: (partial: Partial<SettingsConfigState>) => void,
  snapshot: SettingsSnapshot,
) {
  set({
    hydrated: true,
    general: snapshot.general,
    screenshot: snapshot.screenshot,
    translation: snapshot.translation,
  });
}

function currentSnapshot(state: SettingsConfigState): SettingsSnapshot | null {
  if (!state.general || !state.screenshot || !state.translation) {
    return null;
  }

  return {
    general: state.general,
    screenshot: state.screenshot,
    translation: state.translation,
  };
}

export const useSettingsConfigStore = create<SettingsConfigState>((set, get) => ({
  hydrated: false,
  general: null,
  screenshot: null,
  translation: null,
  hydrate: async () => {
    const existingSnapshot = currentSnapshot(get());

    if (get().hydrated && existingSnapshot) {
      return existingSnapshot;
    }

    const snapshot = await settingsRuntime().load();
    applySnapshot(set, snapshot);
    return snapshot;
  },
  refresh: async () => {
    const snapshot = await settingsRuntime().load();
    applySnapshot(set, snapshot);
    return snapshot;
  },
  updateGeneralSettings: async (input) => {
    const snapshot = await settingsRuntime().updateGeneral(input);
    applySnapshot(set, snapshot);
    return snapshot;
  },
  updateScreenshotSettings: async (input) => {
    const snapshot = await settingsRuntime().updateScreenshot(input);
    applySnapshot(set, snapshot);
    return snapshot;
  },
  updateAnnotationColors: async (colors) => {
    const snapshot = await settingsRuntime().updateAnnotationColors(colors);
    applySnapshot(set, snapshot);
    return snapshot;
  },
  updateTranslationSettings: async (input) => {
    const snapshot = await settingsRuntime().updateTranslation(input);
    applySnapshot(set, snapshot);
    return snapshot;
  },
}));
