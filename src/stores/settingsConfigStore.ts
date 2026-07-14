import { create } from 'zustand';
import type { SettingsRuntime } from '../application/settings/runtime';
import type {
  AnnotationColorPreset,
  GeneralSettings,
  HistorySettings,
  OcrSettings,
  ScreenshotSettings,
  SettingsSnapshot,
  TranslationSettings,
} from '../application/settings/ports';

type DurableSettingsRuntime = SettingsRuntime['durableSettings'];

let durableSettingsRuntime: DurableSettingsRuntime | null = null;
let nextSnapshotRequest = 0;
let latestAppliedSnapshotRequest = 0;
let settingsUpdateQueue: Promise<void> = Promise.resolve();

export function initializeSettingsConfigStore(runtime: DurableSettingsRuntime) {
  durableSettingsRuntime = runtime;
  nextSnapshotRequest = 0;
  latestAppliedSnapshotRequest = 0;
  settingsUpdateQueue = Promise.resolve();
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
  ocr: OcrSettings | null;
  history: HistorySettings | null;
  hydrate: () => Promise<SettingsSnapshot>;
  refresh: () => Promise<SettingsSnapshot>;
  updateGeneralSettings: (input: GeneralSettings) => Promise<SettingsSnapshot>;
  updateScreenshotSettings: (input: ScreenshotSettings) => Promise<SettingsSnapshot>;
  updateAnnotationColors: (
    colors: AnnotationColorPreset[],
  ) => Promise<SettingsSnapshot>;
  updateTranslationSettings: (
    input: Partial<TranslationSettings>,
  ) => Promise<SettingsSnapshot>;
  updateOcrSettings: (input: Partial<OcrSettings>) => Promise<SettingsSnapshot>;
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
    ocr: snapshot.ocr,
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
  if (
    !state.general ||
    !state.screenshot ||
    !state.translation ||
    !state.ocr ||
    !state.history
  ) {
    return null;
  }

  return {
    general: state.general,
    screenshot: state.screenshot,
    translation: state.translation,
    ocr: state.ocr,
    history: state.history,
  };
}

function enqueueSettingsUpdate<T>(request: () => Promise<T>) {
  const result = settingsUpdateQueue.then(request, request);
  settingsUpdateQueue = result.then(
    () => undefined,
    () => undefined,
  );
  return result;
}

export const useSettingsConfigStore = create<SettingsConfigState>((set, get) => ({
  hydrated: false,
  general: null,
  screenshot: null,
  translation: null,
  ocr: null,
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
    enqueueSettingsUpdate(() => {
      const translation = get().translation;
      if (!translation) {
        throw new Error('Translation settings have not been loaded');
      }
      return applyLatestSnapshot(set, get, () =>
        settingsRuntime().updateTranslation({ ...translation, ...input }),
      );
    }),
  updateOcrSettings: (input) =>
    enqueueSettingsUpdate(() => {
      const ocr = get().ocr;
      if (!ocr) {
        throw new Error('OCR settings have not been loaded');
      }
      return applyLatestSnapshot(set, get, () =>
        settingsRuntime().updateOcr({ ...ocr, ...input }),
      );
    }),
  updateHistorySettings: (input) =>
    applyLatestSnapshot(set, get, () => settingsRuntime().updateHistory(input)),
}));
