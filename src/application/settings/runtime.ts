import type {
  AnnotationColorPreset,
  DurableSettingsPort,
  GeneralSettings,
  HotkeyCategory,
  HotkeySnapshot,
  HotkeyUpdateInput,
  HotkeyUpdateOutcome,
  OcrHistoryEntry,
  ScreenshotSettings,
  SettingsCapturePort,
  SettingsClipboardPort,
  SettingsHistoryPort,
  SettingsHotkeysPort,
  SettingsProvidersPort,
  SettingsSnapshot,
  SettingsWindowPort,
  TranslationHistoryEntry,
  TranslationSettings,
} from './ports';

export interface SettingsRuntimePorts {
  window: SettingsWindowPort;
  durableSettings: DurableSettingsPort;
  providers: SettingsProvidersPort;
  hotkeys: SettingsHotkeysPort;
  history: SettingsHistoryPort;
  clipboard: SettingsClipboardPort;
  capture: SettingsCapturePort;
}

export interface SettingsRuntime {
  window: {
    open(): Promise<void>;
  };
  durableSettings: {
    load(): Promise<SettingsSnapshot>;
    updateGeneral(input: GeneralSettings): Promise<SettingsSnapshot>;
    updateScreenshot(input: ScreenshotSettings): Promise<SettingsSnapshot>;
    updateAnnotationColors(
      colors: AnnotationColorPreset[],
    ): Promise<SettingsSnapshot>;
    updateTranslation(input: TranslationSettings): Promise<SettingsSnapshot>;
  };
  providers: SettingsProvidersPort;
  hotkeys: {
    load(): Promise<HotkeySnapshot>;
    loadDefaults(): Promise<HotkeySnapshot>;
    update(input: HotkeyUpdateInput): Promise<HotkeyUpdateOutcome>;
    reset(category: HotkeyCategory, action: string): Promise<HotkeyUpdateOutcome>;
    resetCategory(category: HotkeyCategory): Promise<HotkeySnapshot>;
  };
  history: {
    loadTranslation(
      limit: number,
      offset: number,
    ): Promise<TranslationHistoryEntry[]>;
    loadOcr(limit: number, offset: number): Promise<OcrHistoryEntry[]>;
    deleteEntry(id: number): Promise<void>;
    setFavorite(id: number, favorite: boolean): Promise<void>;
    updateNote(id: number, note: string | null): Promise<void>;
    replaceTags(id: number, tags: string[]): Promise<void>;
    clear(): Promise<void>;
  };
  clipboard: {
    copyText(text: string): Promise<void>;
  };
  advanced: {
    triggerCapture(): Promise<void>;
  };
}

export function createSettingsRuntime(
  ports: SettingsRuntimePorts,
): SettingsRuntime {
  return {
    window: {
      open: () => ports.window.openSettings(),
    },
    durableSettings: {
      load: () => ports.durableSettings.getSettingsSnapshot(),
      updateGeneral: (input) =>
        ports.durableSettings.updateGeneralSettings(input),
      updateScreenshot: (input) =>
        ports.durableSettings.updateScreenshotSettings(input),
      updateAnnotationColors: (colors) =>
        ports.durableSettings.updateAnnotationColors(colors),
      updateTranslation: (input) =>
        ports.durableSettings.updateTranslationSettings(input),
    },
    providers: ports.providers,
    hotkeys: {
      load: () => ports.hotkeys.getHotkeySnapshot(),
      loadDefaults: () => ports.hotkeys.getDefaultHotkeySnapshot(),
      update: (input) => ports.hotkeys.updateHotkey(input),
      reset: (category, action) => ports.hotkeys.resetHotkey(category, action),
      resetCategory: (category) => ports.hotkeys.resetHotkeyCategory(category),
    },
    history: {
      loadTranslation: (limit, offset) =>
        ports.history.getTranslationHistory(limit, offset),
      loadOcr: (limit, offset) => ports.history.getOcrHistory(limit, offset),
      deleteEntry: (id) => ports.history.deleteHistory(id),
      setFavorite: (id, favorite) => ports.history.setHistoryFavorite(id, favorite),
      updateNote: (id, note) => ports.history.updateHistoryNote(id, note),
      replaceTags: (id, tags) => ports.history.replaceHistoryTags(id, tags),
      clear: () => ports.history.clearAllHistory(),
    },
    clipboard: {
      copyText: (text) => ports.clipboard.writeText(text),
    },
    advanced: {
      triggerCapture: () => ports.capture.triggerScreenshot(),
    },
  };
}
