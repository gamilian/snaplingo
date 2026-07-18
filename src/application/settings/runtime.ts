import type {
  AnnotationColorPreset,
  DurableSettingsPort,
  GeneralSettings,
  FavoriteKind,
  FavoritePage,
  FavoriteQuery,
  HotkeyCategory,
  HotkeySnapshot,
  HotkeyUpdateInput,
  HotkeyUpdateOutcome,
  HistoryKind,
  HistorySettings,
  HistoryPage,
  HistoryQuery,
  OcrHistoryEntry,
  OcrFavoriteInput,
  OcrSettings,
  ScreenshotSettings,
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
  SettingsSnapshot,
  SettingsWindowPort,
  SettingsWindowEventsPort,
  TranslationHistoryEntry,
  TranslationFavoriteInput,
  TranslationSettings,
} from './ports';
import { createSettingsLibrary, type SettingsLibrary } from './library';

export interface SettingsRuntimePorts {
  window: SettingsWindowPort;
  windowEvents: SettingsWindowEventsPort;
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
  window: {
    open(): Promise<void>;
    selectScreenshotDirectory(): Promise<string | null>;
    version(): Promise<string>;
    subscribeNavigationRequested: SettingsWindowEventsPort['subscribeNavigationRequested'];
  };
  durableSettings: {
    load(): Promise<SettingsSnapshot>;
    updateGeneral(input: GeneralSettings): Promise<SettingsSnapshot>;
    updateScreenshot(input: ScreenshotSettings): Promise<SettingsSnapshot>;
    updateAnnotationColors(
      colors: AnnotationColorPreset[],
    ): Promise<SettingsSnapshot>;
    updateTranslation(input: TranslationSettings): Promise<SettingsSnapshot>;
    updateOcr(input: OcrSettings): Promise<SettingsSnapshot>;
    updateHistory(input: HistorySettings): Promise<SettingsSnapshot>;
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
      updateOcr: (input) => ports.durableSettings.updateOcrSettings(input),
      updateHistory: (input) => ports.durableSettings.updateHistorySettings(input),
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
