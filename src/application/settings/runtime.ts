import type {
  DurableSettingsPort,
  GeneralSettings,
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
    updateTranslation(input: TranslationSettings): Promise<SettingsSnapshot>;
  };
  providers: SettingsProvidersPort;
  hotkeys: {
    load(): Promise<HotkeySnapshot>;
    update(input: HotkeyUpdateInput): Promise<HotkeyUpdateOutcome>;
  };
  history: {
    loadTranslation(
      limit: number,
      offset: number,
    ): Promise<TranslationHistoryEntry[]>;
    loadOcr(limit: number, offset: number): Promise<OcrHistoryEntry[]>;
    deleteEntry(id: number): Promise<void>;
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
      updateTranslation: (input) =>
        ports.durableSettings.updateTranslationSettings(input),
    },
    providers: ports.providers,
    hotkeys: {
      load: () => ports.hotkeys.getHotkeySnapshot(),
      update: (input) => ports.hotkeys.updateHotkey(input),
    },
    history: {
      loadTranslation: (limit, offset) =>
        ports.history.getTranslationHistory(limit, offset),
      loadOcr: (limit, offset) => ports.history.getOcrHistory(limit, offset),
      deleteEntry: (id) => ports.history.deleteHistory(id),
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
