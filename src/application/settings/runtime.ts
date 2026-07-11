import type {
  AddCustomTranslationProviderRequest,
  CredentialField,
  DurableSettingsPort,
  GeneralSettings,
  HotkeySnapshot,
  HotkeyUpdateInput,
  HotkeyUpdateOutcome,
  OcrHistoryEntry,
  OcrProviderInfo,
  ProviderInfo,
  ProviderModelInfo,
  ProviderModelsRequest,
  ScreenshotSettings,
  SettingsCapturePort,
  SettingsClipboardPort,
  SettingsHistoryPort,
  SettingsHotkeysPort,
  SettingsProvidersPort,
  SettingsSnapshot,
  SettingsWindowPort,
  TestProviderRequest,
  TranslationHistoryEntry,
  TranslationPromptStrategyConfig,
  TranslationSettings,
  UpdateCustomTranslationProviderRequest,
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
  providers: {
    listTranslation(): Promise<ProviderInfo[]>;
    activateTranslation(providerId: string): Promise<void>;
    deactivateTranslation(providerId: string): Promise<void>;
    reorderActiveTranslation(providerIds: string[]): Promise<void>;
    getTranslationCredentialSchema(
      providerId: string,
    ): Promise<CredentialField[]>;
    getOcrCredentialSchema(providerId: string): Promise<CredentialField[]>;
    configureTranslationCredentials(
      providerId: string,
      credentials: Record<string, string>,
    ): Promise<void>;
    addCustomTranslation(
      request: AddCustomTranslationProviderRequest,
    ): Promise<ProviderInfo>;
    updateCustomTranslation(
      providerId: string,
      request: UpdateCustomTranslationProviderRequest,
    ): Promise<ProviderInfo>;
    removeCustomTranslation(providerId: string): Promise<void>;
    testCustomTranslation(providerId: string): Promise<void>;
    listTranslationPromptStrategies(): Promise<TranslationPromptStrategyConfig>;
    saveTranslationPromptStrategies(
      config: TranslationPromptStrategyConfig,
    ): Promise<TranslationPromptStrategyConfig>;
    listOpenAICompatibleModels(
      request: ProviderModelsRequest,
    ): Promise<ProviderModelInfo[]>;
    testOpenAICompatible(request: TestProviderRequest): Promise<void>;
    testOpenAIResponses(request: TestProviderRequest): Promise<void>;
    listAnthropicModels(
      request: ProviderModelsRequest,
    ): Promise<ProviderModelInfo[]>;
    testAnthropic(request: TestProviderRequest): Promise<void>;
    listGeminiModels(request: ProviderModelsRequest): Promise<ProviderModelInfo[]>;
    testGemini(request: TestProviderRequest): Promise<void>;
    listOcr(): Promise<OcrProviderInfo[]>;
    activateOcr(providerId: string): Promise<void>;
    configureOcrCredentials(
      providerId: string,
      credentials: Record<string, string>,
    ): Promise<void>;
  };
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
    providers: {
      listTranslation: () => ports.providers.listTranslationProviders(),
      activateTranslation: (providerId) =>
        ports.providers.activateTranslationProvider(providerId),
      deactivateTranslation: (providerId) =>
        ports.providers.deactivateTranslationProvider(providerId),
      reorderActiveTranslation: (providerIds) =>
        ports.providers.reorderActiveTranslationProviders(providerIds),
      getTranslationCredentialSchema: (providerId) =>
        ports.providers.getProviderCredentialSchema(providerId),
      getOcrCredentialSchema: (providerId) =>
        ports.providers.getOcrProviderCredentialSchema(providerId),
      configureTranslationCredentials: (providerId, credentials) =>
        ports.providers.configureTranslationProviderCredentials(
          providerId,
          credentials,
        ),
      addCustomTranslation: (request) =>
        ports.providers.addCustomTranslationProvider(request),
      updateCustomTranslation: (providerId, request) =>
        ports.providers.updateCustomTranslationProvider(providerId, request),
      removeCustomTranslation: (providerId) =>
        ports.providers.removeCustomTranslationProvider(providerId),
      testCustomTranslation: (providerId) =>
        ports.providers.testCustomTranslationProvider(providerId),
      listTranslationPromptStrategies: () =>
        ports.providers.listTranslationPromptStrategies(),
      saveTranslationPromptStrategies: (config) =>
        ports.providers.saveTranslationPromptStrategies(config),
      listOpenAICompatibleModels: (request) =>
        ports.providers.listOpenAICompatibleModels(request),
      testOpenAICompatible: (request) =>
        ports.providers.testOpenAICompatibleProvider(request),
      testOpenAIResponses: (request) =>
        ports.providers.testOpenAIResponsesProvider(request),
      listAnthropicModels: (request) =>
        ports.providers.listAnthropicModels(request),
      testAnthropic: (request) =>
        ports.providers.testAnthropicProvider(request),
      listGeminiModels: (request) => ports.providers.listGeminiModels(request),
      testGemini: (request) => ports.providers.testGeminiProvider(request),
      listOcr: () => ports.providers.listOcrProviders(),
      activateOcr: (providerId) =>
        ports.providers.activateOcrProvider(providerId),
      configureOcrCredentials: (providerId, credentials) =>
        ports.providers.configureOcrProviderCredentials(
          providerId,
          credentials,
        ),
    },
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
