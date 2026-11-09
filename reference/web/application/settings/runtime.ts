import type {
  DurableSettingsPort,
  OcrFavoriteInput,
  SettingsClipboardPort,
  SettingsHistoryPort,
  SettingsLibraryIndexPort,
  SettingsFavoritesPort,
  SettingsScreenshotFavoritesPort,
  SettingsHotkeysPort,
  SettingsMaintenancePort,
  SettingsTtsPort,
  SettingsProvidersPort,
  SettingsWindowPort,
  SettingsWindowEventsPort,
  TranslationFavoriteInput,
} from './ports';
import { createSettingsLibrary, type SettingsLibrary } from './library';
import type { RequiredPermissionsRuntime } from '../permissions/runtime';
import type { AppUpdatesRuntime } from '../updates/runtime';
import {
  createSettingsConfiguration,
  type SettingsConfiguration,
  type SettingsConfigurationEventsPort,
} from './configuration';

export interface SettingsRuntimePorts {
  permissions: RequiredPermissionsRuntime;
  updates: AppUpdatesRuntime;
  window: SettingsWindowPort;
  windowEvents: SettingsWindowEventsPort;
  configurationEvents: SettingsConfigurationEventsPort;
  durableSettings: DurableSettingsPort;
  providers: SettingsProvidersPort;
  hotkeys: SettingsHotkeysPort;
  history: SettingsHistoryPort;
  libraryIndex: SettingsLibraryIndexPort;
  favorites: SettingsFavoritesPort;
  screenshotFavorites: SettingsScreenshotFavoritesPort;
  clipboard: SettingsClipboardPort;
  maintenance: SettingsMaintenancePort;
  tts: SettingsTtsPort;
}

export interface SettingsRuntime {
  permissions: RequiredPermissionsRuntime;
  updates: AppUpdatesRuntime;
  library: SettingsLibrary;
  configuration: SettingsConfiguration;
  window: {
    open(): Promise<void>;
    selectScreenshotDirectory(): Promise<string | null>;
    version(): Promise<string>;
    displayBounds(): Promise<import('../../domain/capture').LogicalRect | null>;
    applyInterfaceScale(scale: number): Promise<void>;
    openExternal(url: string): Promise<void>;
    presentNativeSettingsPage?: SettingsWindowPort['presentNativeSettingsPage'];
    subscribeNavigationRequested: SettingsWindowEventsPort['subscribeNavigationRequested'];
  };
  providers: Pick<
    SettingsProvidersPort,
    | 'getTranslationCredentialSchema'
    | 'getOcrCredentialSchema'
    | 'listTranslationPromptStrategies'
    | 'saveTranslationPromptStrategies'
    | 'listOpenAICompatibleModels'
    | 'testOpenAICompatible'
    | 'testOpenAIResponses'
    | 'listAnthropicModels'
    | 'testAnthropic'
    | 'listGeminiModels'
    | 'testGemini'
  >;
  favorites: {
    addTranslation(input: TranslationFavoriteInput): Promise<number>;
    addOcr(input: OcrFavoriteInput): Promise<number>;
  };
  screenshotFavorites: {
    copy(id: number): Promise<void>;
    reveal(id: number): Promise<void>;
  };
  clipboard: {
    copyText(text: string): Promise<void>;
  };
  maintenance: SettingsMaintenancePort;
  tts: SettingsTtsPort;
}

export function createSettingsRuntime(
  ports: SettingsRuntimePorts,
): SettingsRuntime {
  return {
    permissions: ports.permissions,
    updates: ports.updates,
    configuration: createSettingsConfiguration({
      durableSettings: ports.durableSettings,
      providers: ports.providers,
      hotkeys: ports.hotkeys,
      events: ports.configurationEvents,
    }),
    library: createSettingsLibrary({
      history: ports.history,
      favorites: ports.favorites,
      screenshotFavorites: ports.screenshotFavorites,
      index: ports.libraryIndex,
      clipboard: ports.clipboard,
    }),
    window: {
      open: () => ports.window.openSettings(),
      selectScreenshotDirectory: () => ports.window.selectScreenshotDirectory(),
      version: () => ports.window.getAppVersion(),
      displayBounds: () => ports.window.displayBounds?.() ?? Promise.resolve(null),
      applyInterfaceScale: (scale) => ports.window.applyInterfaceScale?.(scale) ?? Promise.resolve(),
      openExternal: (url) => ports.window.openExternal?.(url) ?? Promise.resolve(),
      presentNativeSettingsPage: ports.window.presentNativeSettingsPage,
      subscribeNavigationRequested: (handler) =>
        ports.windowEvents.subscribeNavigationRequested(handler),
    },
    providers: {
      getTranslationCredentialSchema: (providerId) =>
        ports.providers.getTranslationCredentialSchema(providerId),
      getOcrCredentialSchema: (providerId) =>
        ports.providers.getOcrCredentialSchema(providerId),
      listTranslationPromptStrategies: () =>
        ports.providers.listTranslationPromptStrategies(),
      saveTranslationPromptStrategies: (config) =>
        ports.providers.saveTranslationPromptStrategies(config),
      listOpenAICompatibleModels: (request) =>
        ports.providers.listOpenAICompatibleModels(request),
      testOpenAICompatible: (request) =>
        ports.providers.testOpenAICompatible(request),
      testOpenAIResponses: (request) =>
        ports.providers.testOpenAIResponses(request),
      listAnthropicModels: (request) =>
        ports.providers.listAnthropicModels(request),
      testAnthropic: (request) => ports.providers.testAnthropic(request),
      listGeminiModels: (request) => ports.providers.listGeminiModels(request),
      testGemini: (request) => ports.providers.testGemini(request),
    },
    favorites: {
      addTranslation: (input) => ports.favorites.addTranslationFavorite(input),
      addOcr: (input) => ports.favorites.addOcrFavorite(input),
    },
    screenshotFavorites: {
      copy: (id) => ports.screenshotFavorites.copyScreenshotFavorite(id),
      reveal: (id) => ports.screenshotFavorites.revealScreenshotFavorite(id),
    },
    clipboard: {
      copyText: (text) => ports.clipboard.writeText(text),
    },
    maintenance: ports.maintenance,
    tts: ports.tts,
  };
}
