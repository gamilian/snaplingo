export interface SettingsWindowPort {
  openSettings(): Promise<void>;
}

export interface GeneralSettings {
  language: string;
  theme: string;
  startOnBoot: boolean;
}

export type AnnotationColorPreset = [number, number, number, number];

export interface ScreenshotSettings {
  savePath: string;
  format: string;
  quality: number;
  annotationColors: AnnotationColorPreset[];
}

export interface TranslationSettings {
  defaultSourceLang: string;
  defaultTargetLang: string;
}

export interface SettingsSnapshot {
  general: GeneralSettings;
  screenshot: ScreenshotSettings;
  translation: TranslationSettings;
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
    favorite: boolean;
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
    favorite: boolean;
    note: string | null;
    tags: string[];
  imageHash: string;
  language: string | null;
  providerUsed: string;
  recognizedText: string;
  confidence: number | null;
  durationMs: number;
}

export type HistoryEntry =
  | (TranslationHistoryEntry & { type: 'translation' })
  | (OcrHistoryEntry & { type: 'ocr' });

export interface SettingsHistoryPort {
  getTranslationHistory(
    limit: number,
    offset: number,
  ): Promise<TranslationHistoryEntry[]>;
  getOcrHistory(limit: number, offset: number): Promise<OcrHistoryEntry[]>;
  deleteHistory(id: number): Promise<void>;
  setHistoryFavorite(id: number, favorite: boolean): Promise<void>;
  updateHistoryNote(id: number, note: string | null): Promise<void>;
  replaceHistoryTags(id: number, tags: string[]): Promise<void>;
  clearAllHistory(): Promise<void>;
}

export interface SettingsClipboardPort {
  writeText(text: string): Promise<void>;
}

export interface SettingsCapturePort {
  triggerScreenshot(): Promise<void>;
}
