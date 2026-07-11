import { invoke } from '@tauri-apps/api/core';

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
  protocol?: string;
  endpoint?: string;
  model?: string;
  reasoning_level?: string;
  prompt_strategy_id?: string;
  prompt_fallback_strategy_id?: string;
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

export interface OpenAICompatibleModelsRequest {
  endpoint: string;
  api_key: string;
}

export interface TestOpenAICompatibleProviderRequest {
  endpoint: string;
  api_key: string;
  model: string;
}

export interface OpenAICompatibleModelInfo {
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

export function listTranslationProviders() {
  return invoke<ProviderInfo[]>('list_translation_providers');
}

export function activateTranslationProvider(providerId: string) {
  return invoke<void>('activate_translation_provider', { providerId });
}

export function deactivateTranslationProvider(providerId: string) {
  return invoke<void>('deactivate_translation_provider', { providerId });
}

export function reorderActiveTranslationProviders(providerIds: string[]) {
  return invoke<void>('reorder_active_translation_providers', { providerIds });
}

export function getProviderCredentialSchema(providerId: string) {
  return invoke<CredentialField[]>('get_provider_credential_schema', { providerId });
}

export function getOcrProviderCredentialSchema(providerId: string) {
  return invoke<CredentialField[]>('get_ocr_provider_credential_schema', { providerId });
}

export function configureTranslationProviderCredentials(
  providerId: string,
  credentials: Record<string, string>,
) {
  return invoke<void>('configure_translation_provider_credentials', {
    providerId,
    credentials,
  });
}

export function addCustomTranslationProvider(
  request: AddCustomTranslationProviderRequest,
) {
  return invoke<ProviderInfo>('add_custom_translation_provider', { request });
}

export function updateCustomTranslationProvider(
  providerId: string,
  request: UpdateCustomTranslationProviderRequest,
) {
  return invoke<ProviderInfo>('update_custom_translation_provider', {
    providerId,
    request,
  });
}

export function removeCustomTranslationProvider(providerId: string) {
  return invoke<void>('remove_custom_translation_provider', { providerId });
}

export function testCustomTranslationProvider(providerId: string) {
  return invoke<void>('test_custom_translation_provider', { providerId });
}

export function listTranslationPromptStrategies() {
  return invoke<TranslationPromptStrategyConfig>('list_translation_prompt_strategies');
}

export function saveTranslationPromptStrategies(
  config: TranslationPromptStrategyConfig,
) {
  return invoke<TranslationPromptStrategyConfig>('save_translation_prompt_strategies', {
    config,
  });
}

export function listOpenAICompatibleModels(
  request: OpenAICompatibleModelsRequest,
) {
  return invoke<OpenAICompatibleModelInfo[]>('list_openai_compatible_models', {
    request,
  });
}

export function testOpenAICompatibleProvider(
  request: TestOpenAICompatibleProviderRequest,
) {
  return invoke<void>('test_openai_compatible_provider', { request });
}

export function testOpenAIResponsesProvider(
  request: TestOpenAICompatibleProviderRequest,
) {
  return invoke<void>('test_openai_responses_provider', { request });
}

export function listAnthropicModels(
  request: OpenAICompatibleModelsRequest,
) {
  return invoke<OpenAICompatibleModelInfo[]>('list_anthropic_models', {
    request,
  });
}

export function testAnthropicProvider(
  request: TestOpenAICompatibleProviderRequest,
) {
  return invoke<void>('test_anthropic_provider', { request });
}

export function listGeminiModels(
  request: OpenAICompatibleModelsRequest,
) {
  return invoke<OpenAICompatibleModelInfo[]>('list_gemini_models', {
    request,
  });
}

export function testGeminiProvider(
  request: TestOpenAICompatibleProviderRequest,
) {
  return invoke<void>('test_gemini_provider', { request });
}

export function listOcrProviders() {
  return invoke<OcrProviderInfo[]>('list_ocr_providers');
}

export function activateOcrProvider(providerId: string) {
  return invoke<void>('activate_ocr_provider', { providerId });
}

export function configureOcrProvider(
  providerId: string,
  apiKey: string,
  secretKey?: string,
) {
  return invoke<void>('configure_ocr_provider', {
    providerId,
    apiKey,
    secretKey: secretKey || null,
  });
}

export function configureOcrProviderCredentials(
  providerId: string,
  credentials: Record<string, string>,
) {
  return invoke<void>('configure_ocr_provider_credentials', {
    providerId,
    credentials,
  });
}
