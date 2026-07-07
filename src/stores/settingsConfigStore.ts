import { create } from 'zustand';
import {
  getSettingsSnapshot as loadSettingsSnapshot,
  updateGeneralSettings as persistGeneralSettings,
  updateScreenshotSettings as persistScreenshotSettings,
  updateTranslationSettings as persistTranslationSettings,
} from '../tauri/settings';

type SettingsSnapshot = Awaited<ReturnType<typeof loadSettingsSnapshot>>;
type GeneralSettings = SettingsSnapshot['general'];
type ScreenshotSettings = SettingsSnapshot['screenshot'];
type TranslationSettings = SettingsSnapshot['translation'];

const LEGACY_STORAGE_KEY = 'snaplingo-settings';
const DURABLE_LEGACY_KEYS = [
  'language',
  'theme',
  'startOnBoot',
  'screenshotSavePath',
  'screenshotFormat',
  'screenshotQuality',
  'defaultSourceLang',
  'defaultTargetLang',
] as const;

interface LegacySettingsDocument {
  state?: Record<string, unknown>;
  version?: number;
}

interface SettingsConfigState {
  hydrated: boolean;
  general: GeneralSettings | null;
  screenshot: ScreenshotSettings | null;
  translation: TranslationSettings | null;
  hydrate: () => Promise<SettingsSnapshot>;
  updateGeneralSettings: (input: GeneralSettings) => Promise<SettingsSnapshot>;
  updateScreenshotSettings: (input: ScreenshotSettings) => Promise<SettingsSnapshot>;
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

function readLegacySettingsDocument(): LegacySettingsDocument | null {
  if (typeof localStorage === 'undefined') {
    return null;
  }

  const value = localStorage.getItem(LEGACY_STORAGE_KEY);
  if (!value) {
    return null;
  }

  try {
    const parsed = JSON.parse(value) as LegacySettingsDocument;
    if (!parsed || typeof parsed !== 'object') {
      return null;
    }

    return parsed;
  } catch {
    return null;
  }
}

function hasString(value: unknown): value is string {
  return typeof value === 'string';
}

function hasBoolean(value: unknown): value is boolean {
  return typeof value === 'boolean';
}

function hasNumber(value: unknown): value is number {
  return typeof value === 'number';
}

function clearMigratedLegacyDurableKeys(document: LegacySettingsDocument) {
  if (typeof localStorage === 'undefined' || !document.state || typeof document.state !== 'object') {
    return;
  }

  const nextState = { ...document.state };

  for (const key of DURABLE_LEGACY_KEYS) {
    delete nextState[key];
  }

  localStorage.setItem(
    LEGACY_STORAGE_KEY,
    JSON.stringify({
      ...document,
      state: nextState,
    }),
  );
}

async function migrateLegacyDurableSettings(snapshot: SettingsSnapshot) {
  const document = readLegacySettingsDocument();
  const state = document?.state;

  if (!state || typeof state !== 'object') {
    return snapshot;
  }

  let nextSnapshot = snapshot;
  let migrated = false;

  if (
    hasString(state.language) ||
    hasString(state.theme) ||
    hasBoolean(state.startOnBoot)
  ) {
    nextSnapshot = await persistGeneralSettings({
      language: hasString(state.language) ? state.language : nextSnapshot.general.language,
      theme: hasString(state.theme) ? state.theme : nextSnapshot.general.theme,
      startOnBoot: hasBoolean(state.startOnBoot)
        ? state.startOnBoot
        : nextSnapshot.general.startOnBoot,
    });
    migrated = true;
  }

  if (
    hasString(state.screenshotSavePath) ||
    hasString(state.screenshotFormat) ||
    hasNumber(state.screenshotQuality)
  ) {
    nextSnapshot = await persistScreenshotSettings({
      savePath: hasString(state.screenshotSavePath)
        ? state.screenshotSavePath
        : nextSnapshot.screenshot.savePath,
      format: hasString(state.screenshotFormat)
        ? state.screenshotFormat
        : nextSnapshot.screenshot.format,
      quality: hasNumber(state.screenshotQuality)
        ? state.screenshotQuality
        : nextSnapshot.screenshot.quality,
    });
    migrated = true;
  }

  if (
    hasString(state.defaultSourceLang) ||
    hasString(state.defaultTargetLang)
  ) {
    nextSnapshot = await persistTranslationSettings({
      defaultSourceLang: hasString(state.defaultSourceLang)
        ? state.defaultSourceLang
        : nextSnapshot.translation.defaultSourceLang,
      defaultTargetLang: hasString(state.defaultTargetLang)
        ? state.defaultTargetLang
        : nextSnapshot.translation.defaultTargetLang,
    });
    migrated = true;
  }

  if (migrated && document) {
    clearMigratedLegacyDurableKeys(document);
  }

  return nextSnapshot;
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

    let snapshot = await loadSettingsSnapshot();
    snapshot = await migrateLegacyDurableSettings(snapshot);
    applySnapshot(set, snapshot);
    return snapshot;
  },
  updateGeneralSettings: async (input) => {
    const snapshot = await persistGeneralSettings(input);
    applySnapshot(set, snapshot);
    return snapshot;
  },
  updateScreenshotSettings: async (input) => {
    const snapshot = await persistScreenshotSettings(input);
    applySnapshot(set, snapshot);
    return snapshot;
  },
  updateTranslationSettings: async (input) => {
    const snapshot = await persistTranslationSettings(input);
    applySnapshot(set, snapshot);
    return snapshot;
  },
}));
