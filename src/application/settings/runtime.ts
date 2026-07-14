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
  ScreenshotSettings,
  ScreenshotFavoritePage,
  ScreenshotFavoriteQuery,
  SettingsCapturePort,
  SettingsClipboardPort,
  SettingsHistoryPort,
  SettingsFavoritesPort,
  SettingsScreenshotFavoritesPort,
  SettingsHotkeysPort,
  SettingsProvidersPort,
  SettingsSnapshot,
  SettingsWindowPort,
  TranslationHistoryEntry,
  TranslationFavoriteInput,
  TranslationSettings,
} from './ports';

export interface SettingsRuntimePorts {
  window: SettingsWindowPort;
  durableSettings: DurableSettingsPort;
  providers: SettingsProvidersPort;
  hotkeys: SettingsHotkeysPort;
  history: SettingsHistoryPort;
  favorites: SettingsFavoritesPort;
  screenshotFavorites: SettingsScreenshotFavoritesPort;
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
    setFavorite(id: number, favorite: boolean): Promise<void>;
    updateNote(id: number, note: string | null): Promise<void>;
    replaceTags(id: number, tags: string[]): Promise<void>;
    clear(): Promise<void>;
    clearKind(kind: HistoryKind): Promise<void>;
    rerunOcr(id: number): Promise<string>;
    exportTranslationFavorites(): Promise<number | null>;
    listTags(kind: HistoryKind, favoriteOnly: boolean): Promise<string[]>;
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
      setFavorite: (id, favorite) => ports.history.setHistoryFavorite(id, favorite),
      updateNote: (id, note) => ports.history.updateHistoryNote(id, note),
      replaceTags: (id, tags) => ports.history.replaceHistoryTags(id, tags),
      clear: () => ports.history.clearAllHistory(),
      clearKind: (kind) => ports.history.clearHistory(kind),
      rerunOcr: (id) => ports.history.rerunOcrHistory(id),
      exportTranslationFavorites: () =>
        ports.history.exportTranslationFavorites(),
      listTags: (kind, favoriteOnly) =>
        ports.history.listHistoryTags(kind, favoriteOnly),
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
    advanced: {
      triggerCapture: () => ports.capture.triggerScreenshot(),
    },
  };
}
