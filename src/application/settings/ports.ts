export interface SettingsWindowPort {
  openSettings(): Promise<void>;
}

export interface GeneralSettings {
  language: string;
  theme: string;
  startOnBoot: boolean;
}

export interface ScreenshotSettings {
  savePath: string;
  format: string;
  quality: number;
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
  is_configured: boolean;
  requires_api_key: boolean;
  is_active: boolean;
  is_builtin: boolean;
  protocol: string | null;
  endpoint: string | null;
  model: string | null;
  reasoning_level: string | null;
  prompt_strategy_id: string | null;
  prompt_fallback_strategy_id: string | null;
}

export interface OcrProviderInfo {
  id: string;
  name: string;
  is_configured: boolean;
  requires_api_key: boolean;
  is_active: boolean;
}

export interface AddCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key: string;
  reasoning_level?: string;
  prompt_strategy_id?: string;
  prompt_fallback_strategy_id?: string;
}

export interface UpdateCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key?: string;
  reasoning_level?: string;
  prompt_strategy_id?: string;
  prompt_fallback_strategy_id?: string;
}

export interface ProviderModelsRequest {
  endpoint: string;
  api_key: string;
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
  system_prompt: string;
  is_builtin: boolean;
  is_deletable: boolean;
}

export interface TranslationPromptStrategyConfig {
  strategies: TranslationPromptStrategy[];
}

export interface SettingsProvidersPort {
  listTranslationProviders(): Promise<ProviderInfo[]>;
  activateTranslationProvider(providerId: string): Promise<void>;
  deactivateTranslationProvider(providerId: string): Promise<void>;
  reorderActiveTranslationProviders(providerIds: string[]): Promise<void>;
  getProviderCredentialSchema(providerId: string): Promise<CredentialField[]>;
  getOcrProviderCredentialSchema(
    providerId: string,
  ): Promise<CredentialField[]>;
  configureTranslationProviderCredentials(
    providerId: string,
    credentials: Record<string, string>,
  ): Promise<void>;
  addCustomTranslationProvider(
    request: AddCustomTranslationProviderRequest,
  ): Promise<ProviderInfo>;
  updateCustomTranslationProvider(
    providerId: string,
    request: UpdateCustomTranslationProviderRequest,
  ): Promise<ProviderInfo>;
  removeCustomTranslationProvider(providerId: string): Promise<void>;
  testCustomTranslationProvider(providerId: string): Promise<void>;
  listTranslationPromptStrategies(): Promise<TranslationPromptStrategyConfig>;
  saveTranslationPromptStrategies(
    config: TranslationPromptStrategyConfig,
  ): Promise<TranslationPromptStrategyConfig>;
  listOpenAICompatibleModels(
    request: ProviderModelsRequest,
  ): Promise<ProviderModelInfo[]>;
  testOpenAICompatibleProvider(request: TestProviderRequest): Promise<void>;
  testOpenAIResponsesProvider(request: TestProviderRequest): Promise<void>;
  listAnthropicModels(
    request: ProviderModelsRequest,
  ): Promise<ProviderModelInfo[]>;
  testAnthropicProvider(request: TestProviderRequest): Promise<void>;
  listGeminiModels(request: ProviderModelsRequest): Promise<ProviderModelInfo[]>;
  testGeminiProvider(request: TestProviderRequest): Promise<void>;
  listOcrProviders(): Promise<OcrProviderInfo[]>;
  activateOcrProvider(providerId: string): Promise<void>;
  configureOcrProviderCredentials(
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
  updateHotkey(input: HotkeyUpdateInput): Promise<HotkeyUpdateOutcome>;
}

export interface TranslationHistoryEntry {
  id: number;
  timestamp: string;
  source_text: string;
  source_lang: string;
  target_lang: string;
  providers_used: string[];
  results: Array<{
    provider_id: string;
    translated_text: string;
    detected_language: string | null;
    confidence: number | null;
  }>;
  duration_ms: number;
}

export interface OcrHistoryEntry {
  id: number;
  timestamp: string;
  image_hash: string;
  language: string | null;
  provider_used: string;
  recognized_text: string;
  confidence: number | null;
  duration_ms: number;
}

export interface SettingsHistoryPort {
  getTranslationHistory(
    limit: number,
    offset: number,
  ): Promise<TranslationHistoryEntry[]>;
  getOcrHistory(limit: number, offset: number): Promise<OcrHistoryEntry[]>;
  deleteHistory(id: number): Promise<void>;
  clearAllHistory(): Promise<void>;
}

export interface SettingsClipboardPort {
  writeText(text: string): Promise<void>;
}

export interface SettingsCapturePort {
  triggerScreenshot(): Promise<void>;
}
