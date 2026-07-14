export interface SettingsWindowPort {
  openSettings(): Promise<void>;
  selectScreenshotDirectory(): Promise<string | null>;
}

export interface GeneralSettings {
  language: string;
  theme: string;
  startOnBoot: boolean;
}

export type AnnotationColorPreset = [number, number, number, number];
export type ScreenshotFormat = 'png' | 'jpg' | 'webp';
export type ScreenshotNamingRule = 'timestamp' | 'date' | 'counter' | 'custom';

export interface ScreenshotSettings {
  savePath: string;
  format: ScreenshotFormat;
  quality: number;
  namingRule: ScreenshotNamingRule;
  customFileName: string;
  autoCopy: boolean;
  defaultStrokeWidth: number;
  defaultFontSize: number;
  rememberLastTool: boolean;
  showSelectionSize: boolean;
  showMagnifier: boolean;
  pinOpacity: number;
  pinShadow: boolean;
  annotationColors: AnnotationColorPreset[];
}

export interface TranslationSettings {
  defaultSourceLang: string;
  defaultTargetLang: string;
}

export interface HistorySettings {
  autoCleanupEnabled: boolean;
  retentionDays: number;
  maximumRecords: number;
}

export interface SettingsSnapshot {
  general: GeneralSettings;
  screenshot: ScreenshotSettings;
  translation: TranslationSettings;
  history: HistorySettings;
}

export interface DurableSettingsPort {
  getSettingsSnapshot(): Promise<SettingsSnapshot>;
  updateGeneralSettings(input: GeneralSettings): Promise<SettingsSnapshot>;
  updateScreenshotSettings(
    input: ScreenshotSettings,
  ): Promise<SettingsSnapshot>;
  updateAnnotationColors(
    colors: AnnotationColorPreset[],
  ): Promise<SettingsSnapshot>;
  updateTranslationSettings(
    input: TranslationSettings,
  ): Promise<SettingsSnapshot>;
  updateHistorySettings(input: HistorySettings): Promise<SettingsSnapshot>;
}

export interface CredentialField {
  name: string;
  label: string;
  secret: boolean;
}

export interface ProviderInfo {
  id: string;
  name: string;
  isConfigured: boolean;
  requiresApiKey: boolean;
  isActive: boolean;
  isBuiltin: boolean;
  protocol: string | null;
  endpoint: string | null;
  model: string | null;
  reasoningLevel: string | null;
  promptStrategyId: string | null;
  promptFallbackStrategyId: string | null;
}

export interface OcrProviderInfo {
  id: string;
  name: string;
  isConfigured: boolean;
  requiresApiKey: boolean;
  isActive: boolean;
}

export interface AddCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  apiKey: string;
  reasoningLevel?: string;
  promptStrategyId?: string;
  promptFallbackStrategyId?: string;
}

export interface UpdateCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  apiKey?: string;
  reasoningLevel?: string;
  promptStrategyId?: string;
  promptFallbackStrategyId?: string;
}

export interface ProviderModelsRequest {
  endpoint: string;
  apiKey: string;
}

export interface TestProviderRequest extends ProviderModelsRequest {
  model: string;
}

export interface ProviderModelInfo {
  id: string;
}

export interface TranslationPromptStrategy {
  id: string;
  name: string;
  description: string;
  systemPrompt: string;
  isBuiltin: boolean;
  isDeletable: boolean;
}

export interface TranslationPromptStrategyConfig {
  strategies: TranslationPromptStrategy[];
}

export interface SettingsProvidersPort {
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
}

export type HotkeyCategory = 'screenshot' | 'translation' | 'ocr';

export type HotkeySnapshot = Record<HotkeyCategory, Record<string, string>>;

export interface HotkeyUpdateInput {
  category: HotkeyCategory;
  action: string;
  hotkey: string;
}

export interface HotkeyUpdateOutcome {
  snapshot: HotkeySnapshot;
  accelerator: string | null;
}

export interface SettingsHotkeysPort {
  getHotkeySnapshot(): Promise<HotkeySnapshot>;
  getDefaultHotkeySnapshot(): Promise<HotkeySnapshot>;
  updateHotkey(input: HotkeyUpdateInput): Promise<HotkeyUpdateOutcome>;
  resetHotkey(
    category: HotkeyCategory,
    action: string,
  ): Promise<HotkeyUpdateOutcome>;
  resetHotkeyCategory(category: HotkeyCategory): Promise<HotkeySnapshot>;
}

export interface TranslationHistoryEntry {
    id: number;
    timestamp: string;
    note: string | null;
    tags: string[];
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  providersUsed: string[];
  results: Array<{
    providerId: string;
    translatedText: string;
    detectedLanguage: string | null;
    confidence: number | null;
  }>;
  durationMs: number;
}

export interface OcrHistoryEntry {
    id: number;
    timestamp: string;
    note: string | null;
    tags: string[];
  imageHash: string;
  language: string | null;
  providerUsed: string;
  recognizedText: string;
  confidence: number | null;
  durationMs: number;
  thumbnailDataUrl: string | null;
}

export type HistoryEntry =
  | (TranslationHistoryEntry & { type: 'translation' })
  | (OcrHistoryEntry & { type: 'ocr' });

export type HistoryKind = 'translation' | 'ocr';

export interface HistoryQuery {
  search: string;
  tag?: string;
  limit: number;
  offset: number;
}

export interface HistoryPage<T> {
  items: T[];
  total: number;
}

export interface ScreenshotFavoriteQuery {
  search: string;
  limit: number;
  offset: number;
}

export interface ScreenshotFavoriteItem {
  id: number;
  contentKind: 'screenshot';
  createdAt: string;
  thumbnailDataUrl: string;
  width: number;
  height: number;
  note: string | null;
  tags: string[];
}

export interface ScreenshotFavoritePage {
  items: ScreenshotFavoriteItem[];
  total: number;
}

export type FavoriteKind = 'translation' | 'ocr';

export interface TranslationFavoriteInput {
  sourceHistoryId?: number | null;
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  providerId: string;
  translatedText: string;
  detectedLanguage?: string | null;
  confidence?: number | null;
}

export interface OcrFavoriteInput {
  sourceHistoryId?: number | null;
  imageData?: Uint8Array | number[];
  recognizedText: string;
  language?: string | null;
  providerUsed: string;
  confidence?: number | null;
}

export interface TranslationFavoriteItem {
  id: number;
  createdAt: string;
  sourceHistoryId: number | null;
  content: {
    contentKind: 'translation';
    snapshot: {
      sourceText: string;
      sourceLang: string;
      targetLang: string;
      result: {
        provider_id: string;
        translated_text: string;
        detected_language: string | null;
        confidence: number | null;
      };
    };
  };
  note: string | null;
  tags: string[];
  thumbnailDataUrl: null;
}

export interface OcrFavoriteItem {
  id: number;
  createdAt: string;
  sourceHistoryId: number | null;
  content: {
    contentKind: 'ocr';
    snapshot: {
      imageHash: string;
      recognizedText: string;
      language: string | null;
      providerUsed: string;
      confidence: number | null;
      sourceAssetPath: string | null;
      thumbnailAssetPath: string | null;
    };
  };
  note: string | null;
  tags: string[];
  thumbnailDataUrl: string | null;
}

export type FavoriteItem = TranslationFavoriteItem | OcrFavoriteItem;

export interface FavoriteQuery {
  kind?: FavoriteKind;
  search?: string;
  tag?: string;
  limit: number;
  offset: number;
}

export interface FavoritePage {
  items: FavoriteItem[];
  total: number;
}

export interface SettingsFavoritesPort {
  addTranslationFavorite(input: TranslationFavoriteInput): Promise<number>;
  addOcrFavorite(input: OcrFavoriteInput): Promise<number>;
  queryFavorites(query: FavoriteQuery): Promise<FavoritePage>;
  updateFavoriteMetadata(
    id: number,
    note: string | null,
    tags: string[],
  ): Promise<void>;
  deleteFavorite(id: number): Promise<void>;
  rerunOcrFavorite(id: number): Promise<string>;
  listFavoriteTags(kind: FavoriteKind): Promise<string[]>;
}

export interface SettingsScreenshotFavoritesPort {
  queryScreenshotFavorites(
    query: ScreenshotFavoriteQuery,
  ): Promise<ScreenshotFavoritePage>;
  updateScreenshotFavoriteMetadata(
    id: number,
    note: string | null,
    tags: string[],
  ): Promise<void>;
  deleteScreenshotFavorite(id: number): Promise<void>;
  copyScreenshotFavorite(id: number): Promise<void>;
  revealScreenshotFavorite(id: number): Promise<void>;
}

export interface SettingsHistoryPort {
  getTranslationHistory(
    limit: number,
    offset: number,
  ): Promise<TranslationHistoryEntry[]>;
  getOcrHistory(limit: number, offset: number): Promise<OcrHistoryEntry[]>;
  queryTranslationHistory(
    query: HistoryQuery,
  ): Promise<HistoryPage<TranslationHistoryEntry>>;
  queryOcrHistory(query: HistoryQuery): Promise<HistoryPage<OcrHistoryEntry>>;
  deleteHistory(id: number): Promise<void>;
  updateHistoryNote(id: number, note: string | null): Promise<void>;
  replaceHistoryTags(id: number, tags: string[]): Promise<void>;
  clearAllHistory(): Promise<void>;
  clearHistory(kind: HistoryKind): Promise<void>;
  rerunOcrHistory(id: number): Promise<string>;
}

export interface SettingsClipboardPort {
  writeText(text: string): Promise<void>;
}
