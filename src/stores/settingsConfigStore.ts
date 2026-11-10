import { create } from 'zustand';
import type {
  DurableSettingsConfigurationState,
  SettingsConfiguration,
} from '../application/settings/configuration';
import type {
  AnnotationColorPreset,
  GeneralSettings,
  HistorySettings,
  OcrSettings,
  ScreenshotSettings,
  SettingsSnapshot,
  TranslationSettings,
} from '../application/settings/ports';

type DurableSettingsConfiguration = SettingsConfiguration['settings'];

let configuration: DurableSettingsConfiguration | null = null;
let unsubscribe: (() => void) | null = null;

export function initializeSettingsConfigStore(runtime: DurableSettingsConfiguration) {
  unsubscribe?.();
  configuration = runtime;
  projectSettingsState(runtime.getState());
  unsubscribe = runtime.subscribe(projectSettingsState);
}

function runtime() {
  if (!configuration) {
    throw new Error('Settings config store runtime has not been initialized');
  }
  return configuration;
}

interface SettingsConfigState {
  hydrated: boolean;
  general: GeneralSettings | null;
  screenshot: ScreenshotSettings | null;
  translation: TranslationSettings | null;
  ocr: OcrSettings | null;
  history: HistorySettings | null;
  hydrate: () => Promise<SettingsSnapshot>;
  refresh: () => Promise<SettingsSnapshot>;
  updateGeneralSettings: (
    input: Partial<GeneralSettings>,
  ) => Promise<SettingsSnapshot>;
  updateScreenshotSettings: (
    input: Partial<ScreenshotSettings>,
  ) => Promise<SettingsSnapshot>;
  updateAnnotationColors: (
    colors: AnnotationColorPreset[],
  ) => Promise<SettingsSnapshot>;
  updateTranslationSettings: (
    input: Partial<TranslationSettings>,
  ) => Promise<SettingsSnapshot>;
  updateOcrSettings: (input: Partial<OcrSettings>) => Promise<SettingsSnapshot>;
  updateHistorySettings: (input: HistorySettings) => Promise<SettingsSnapshot>;
}

export const useSettingsConfigStore = create<SettingsConfigState>(() => ({
  hydrated: false,
  general: null,
  screenshot: null,
  translation: null,
  ocr: null,
  history: null,
  hydrate: () => runtime().hydrate(),
  refresh: () => runtime().refresh(),
  updateGeneralSettings: (input) => runtime().updateGeneral(input),
  updateScreenshotSettings: (input) => runtime().updateScreenshot(input),
  updateAnnotationColors: (colors) => runtime().updateAnnotationColors(colors),
  updateTranslationSettings: (input) => runtime().updateTranslation(input),
  updateOcrSettings: (input) => runtime().updateOcr(input),
  updateHistorySettings: (input) => runtime().updateHistory(input),
}));

function projectSettingsState(state: DurableSettingsConfigurationState) {
  const snapshot = state.snapshot;
  useSettingsConfigStore.setState({
    hydrated: state.hydrated,
    general: snapshot?.general ?? null,
    screenshot: snapshot?.screenshot ?? null,
    translation: snapshot?.translation ?? null,
    ocr: snapshot?.ocr ?? null,
    history: snapshot?.history ?? null,
  });
}
