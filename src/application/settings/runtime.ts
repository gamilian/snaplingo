import type {
  DurableSettingsPort,
  FavoriteKind,
  FavoritePage,
  FavoriteQuery,
  HistoryKind,
  HistoryPage,
  HistoryQuery,
  OcrHistoryEntry,
  OcrFavoriteInput,
  ScreenshotFavoritePage,
  ScreenshotFavoriteQuery,
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
  TranslationHistoryEntry,
  TranslationFavoriteInput,
} from './ports';
import { createSettingsLibrary, type SettingsLibrary } from './library';
import {
  createSettingsConfiguration,
  type SettingsConfiguration,
  type SettingsConfigurationEventsPort,
} from './configuration';

export interface SettingsRuntimePorts {
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
  library: SettingsLibrary;
  configuration: SettingsConfiguration;
  window: {
    open(): Promise<void>;
    selectScreenshotDirectory(): Promise<string | null>;
    version(): Promise<string>;
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
  history: {
    loadTranslation(
      limit: number,
      offset: number,
    ): Promise<TranslationHistoryEntry[]>;
    loadOcr(limit: number, offset: number): Promise<OcrHistoryEntry[]>;
    queryTranslation(
      query: HistoryQuery,
    ): Promise<HistoryPage<TranslationHistoryEntry>>;
    queryOcr(query: HistoryQuery): Promise<HistoryPage<OcrHistoryEntry>>;
    deleteEntry(id: number): Promise<void>;
    updateNote(id: number, note: string | null): Promise<void>;
    replaceTags(id: number, tags: string[]): Promise<void>;
    clear(): Promise<void>;
    clearKind(kind: HistoryKind): Promise<void>;
    rerunOcr(id: number): Promise<string>;
  };
  favorites: {
    addTranslation(input: TranslationFavoriteInput): Promise<number>;
    addOcr(input: OcrFavoriteInput): Promise<number>;
    query(query: FavoriteQuery): Promise<FavoritePage>;
    updateMetadata(id: number, note: string | null, tags: string[]): Promise<void>;
    delete(id: number): Promise<void>;
    rerunOcr(id: number): Promise<string>;
    listTags(kind: FavoriteKind): Promise<string[]>;
  };
  screenshotFavorites: {
    query(query: ScreenshotFavoriteQuery): Promise<ScreenshotFavoritePage>;
    updateMetadata(
      id: number,
      note: string | null,
      tags: string[],
    ): Promise<void>;
    delete(id: number): Promise<void>;
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
    history: {
      loadTranslation: (limit, offset) =>
        ports.history.getTranslationHistory(limit, offset),
      loadOcr: (limit, offset) => ports.history.getOcrHistory(limit, offset),
      queryTranslation: (query) =>
        ports.history.queryTranslationHistory(query),
      queryOcr: (query) => ports.history.queryOcrHistory(query),
      deleteEntry: (id) => ports.history.deleteHistory(id),
      updateNote: (id, note) => ports.history.updateHistoryNote(id, note),
      replaceTags: (id, tags) => ports.history.replaceHistoryTags(id, tags),
      clear: () => ports.history.clearAllHistory(),
      clearKind: (kind) => ports.history.clearHistory(kind),
      rerunOcr: (id) => ports.history.rerunOcrHistory(id),
    },
    favorites: {
      addTranslation: (input) => ports.favorites.addTranslationFavorite(input),
      addOcr: (input) => ports.favorites.addOcrFavorite(input),
      query: (query) => ports.favorites.queryFavorites(query),
      updateMetadata: (id, note, tags) =>
        ports.favorites.updateFavoriteMetadata(id, note, tags),
      delete: (id) => ports.favorites.deleteFavorite(id),
      rerunOcr: (id) => ports.favorites.rerunOcrFavorite(id),
      listTags: (kind) => ports.favorites.listFavoriteTags(kind),
    },
    screenshotFavorites: {
      query: (query) => ports.screenshotFavorites.queryScreenshotFavorites(query),
      updateMetadata: (id, note, tags) =>
        ports.screenshotFavorites.updateScreenshotFavoriteMetadata(
          id,
          note,
          tags,
        ),
      delete: (id) => ports.screenshotFavorites.deleteScreenshotFavorite(id),
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
