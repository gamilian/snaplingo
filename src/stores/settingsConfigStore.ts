import { create } from 'zustand';
import type { SettingsRuntime } from '../application/settings/runtime';
import type {
  AnnotationColorPreset,
  GeneralSettings,
  HistorySettings,
  ScreenshotSettings,
  SettingsSnapshot,
  TranslationSettings,
} from '../application/settings/ports';

type DurableSettingsRuntime = SettingsRuntime['durableSettings'];

let durableSettingsRuntime: DurableSettingsRuntime | null = null;
let nextSnapshotRequest = 0;
let latestAppliedSnapshotRequest = 0;

export function initializeSettingsConfigStore(runtime: DurableSettingsRuntime) {
  durableSettingsRuntime = runtime;
  nextSnapshotRequest = 0;
  latestAppliedSnapshotRequest = 0;
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
  history: HistorySettings | null;
  hydrate: () => Promise<SettingsSnapshot>;
  refresh: () => Promise<SettingsSnapshot>;
  updateGeneralSettings: (input: GeneralSettings) => Promise<SettingsSnapshot>;
  updateScreenshotSettings: (input: ScreenshotSettings) => Promise<SettingsSnapshot>;
  updateAnnotationColors: (
    colors: AnnotationColorPreset[],
  ) => Promise<SettingsSnapshot>;
  updateTranslationSettings: (input: TranslationSettings) => Promise<SettingsSnapshot>;
  updateHistorySettings: (input: HistorySettings) => Promise<SettingsSnapshot>;
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
    history: snapshot.history,
  });
}

async function applyLatestSnapshot(
  set: (partial: Partial<SettingsConfigState>) => void,
  get: () => SettingsConfigState,
  request: () => Promise<SettingsSnapshot>,
) {
  const requestId = ++nextSnapshotRequest;
  const snapshot = await request();
  if (requestId > latestAppliedSnapshotRequest) {
    latestAppliedSnapshotRequest = requestId;
    applySnapshot(set, snapshot);
    return snapshot;
  }
  return currentSnapshot(get()) ?? snapshot;
}

function currentSnapshot(state: SettingsConfigState): SettingsSnapshot | null {
  if (!state.general || !state.screenshot || !state.translation || !state.history) {
    return null;
  }

  return {
    general: state.general,
    screenshot: state.screenshot,
    translation: state.translation,
    history: state.history,
  };
}

export const useSettingsConfigStore = create<SettingsConfigState>((set, get) => ({
  hydrated: false,
  general: null,
  screenshot: null,
  translation: null,
  history: null,
  hydrate: async () => {
    const existingSnapshot = currentSnapshot(get());

    if (get().hydrated && existingSnapshot) {
      return existingSnapshot;
    }

    return applyLatestSnapshot(set, get, () => settingsRuntime().load());
  },
  refresh: () => applyLatestSnapshot(set, get, () => settingsRuntime().load()),
  updateGeneralSettings: (input) =>
    applyLatestSnapshot(set, get, () => settingsRuntime().updateGeneral(input)),
  updateScreenshotSettings: (input) =>
    applyLatestSnapshot(set, get, () =>
      settingsRuntime().updateScreenshot(input),
    ),
  updateAnnotationColors: (colors) =>
    applyLatestSnapshot(set, get, () =>
      settingsRuntime().updateAnnotationColors(colors),
    ),
  updateTranslationSettings: (input) =>
    applyLatestSnapshot(set, get, () =>
      settingsRuntime().updateTranslation(input),
    ),
  updateHistorySettings: (input) =>
    applyLatestSnapshot(set, get, () => settingsRuntime().updateHistory(input)),
}));
