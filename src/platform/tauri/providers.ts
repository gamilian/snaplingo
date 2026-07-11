import { invoke } from '@tauri-apps/api/core';
import type {
  AddCustomTranslationProviderRequest,
  OcrProviderInfo,
  ProviderInfo,
  ProviderModelsRequest,
  SettingsProvidersPort,
  TestProviderRequest,
  TranslationPromptStrategy,
  TranslationPromptStrategyConfig,
  UpdateCustomTranslationProviderRequest,
} from '../../application/settings/ports';

interface BackendProviderInfo {
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

interface BackendOcrProviderInfo {
  id: string;
  name: string;
  is_configured: boolean;
  requires_api_key: boolean;
  is_active: boolean;
}

interface BackendCustomProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key?: string;
  reasoning_level?: string;
  prompt_strategy_id?: string;
  prompt_fallback_strategy_id?: string;
}

interface BackendProviderModelsRequest {
  endpoint: string;
  api_key: string;
}

interface BackendTestProviderRequest extends BackendProviderModelsRequest {
  model: string;
}

interface BackendProviderModelInfo {
  id: string;
}

interface BackendTranslationPromptStrategy {
  id: string;
  name: string;
  description: string;
  system_prompt: string;
  is_builtin: boolean;
  is_deletable: boolean;
}

interface BackendTranslationPromptStrategyConfig {
  strategies: BackendTranslationPromptStrategy[];
}

function toProviderInfo(provider: BackendProviderInfo): ProviderInfo {
  return {
    id: provider.id,
    name: provider.name,
    isConfigured: provider.is_configured,
    requiresApiKey: provider.requires_api_key,
    isActive: provider.is_active,
    isBuiltin: provider.is_builtin,
    protocol: provider.protocol,
    endpoint: provider.endpoint,
    model: provider.model,
    reasoningLevel: provider.reasoning_level,
    promptStrategyId: provider.prompt_strategy_id,
    promptFallbackStrategyId: provider.prompt_fallback_strategy_id,
  };
}

function toOcrProviderInfo(provider: BackendOcrProviderInfo): OcrProviderInfo {
  return {
    id: provider.id,
    name: provider.name,
    isConfigured: provider.is_configured,
    requiresApiKey: provider.requires_api_key,
    isActive: provider.is_active,
  };
}

function toBackendCustomProviderRequest(
  request:
    | AddCustomTranslationProviderRequest
    | UpdateCustomTranslationProviderRequest,
): BackendCustomProviderRequest {
  return {
    name: request.name,
    protocol: request.protocol,
    endpoint: request.endpoint,
    model: request.model,
    api_key: request.apiKey,
    reasoning_level: request.reasoningLevel,
    prompt_strategy_id: request.promptStrategyId,
    prompt_fallback_strategy_id: request.promptFallbackStrategyId,
  };
}

function toBackendProviderModelsRequest(
  request: ProviderModelsRequest,
): BackendProviderModelsRequest {
  return {
    endpoint: request.endpoint,
    api_key: request.apiKey,
  };
}

function toBackendTestProviderRequest(
  request: TestProviderRequest,
): BackendTestProviderRequest {
  return {
    ...toBackendProviderModelsRequest(request),
    model: request.model,
  };
}

function toTranslationPromptStrategy(
  strategy: BackendTranslationPromptStrategy,
): TranslationPromptStrategy {
  return {
    id: strategy.id,
    name: strategy.name,
    description: strategy.description,
    systemPrompt: strategy.system_prompt,
    isBuiltin: strategy.is_builtin,
    isDeletable: strategy.is_deletable,
  };
}

function toBackendTranslationPromptStrategy(
  strategy: TranslationPromptStrategy,
): BackendTranslationPromptStrategy {
  return {
    id: strategy.id,
    name: strategy.name,
    description: strategy.description,
    system_prompt: strategy.systemPrompt,
    is_builtin: strategy.isBuiltin,
    is_deletable: strategy.isDeletable,
  };
}

function toTranslationPromptStrategyConfig(
  config: BackendTranslationPromptStrategyConfig,
): TranslationPromptStrategyConfig {
  return {
    strategies: config.strategies.map(toTranslationPromptStrategy),
  };
}

function toBackendTranslationPromptStrategyConfig(
  config: TranslationPromptStrategyConfig,
): BackendTranslationPromptStrategyConfig {
  return {
    strategies: config.strategies.map(toBackendTranslationPromptStrategy),
  };
}

export const settingsProviders: SettingsProvidersPort = {
  async listTranslation() {
    const providers = await invoke<BackendProviderInfo[]>(
      'list_translation_providers',
    );
    return providers.map(toProviderInfo);
  },
  activateTranslation(providerId) {
    return invoke<void>('activate_translation_provider', { providerId });
  },
  deactivateTranslation(providerId) {
    return invoke<void>('deactivate_translation_provider', { providerId });
  },
  reorderActiveTranslation(providerIds) {
    return invoke<void>('reorder_active_translation_providers', {
      providerIds,
    });
  },
  getTranslationCredentialSchema(providerId) {
    return invoke('get_provider_credential_schema', { providerId });
  },
  getOcrCredentialSchema(providerId) {
    return invoke('get_ocr_provider_credential_schema', { providerId });
  },
  configureTranslationCredentials(providerId, credentials) {
    return invoke<void>('configure_translation_provider_credentials', {
      providerId,
      credentials,
    });
  },
  async addCustomTranslation(request) {
    const provider = await invoke<BackendProviderInfo>(
      'add_custom_translation_provider',
      { request: toBackendCustomProviderRequest(request) },
    );
    return toProviderInfo(provider);
  },
  async updateCustomTranslation(providerId, request) {
    const provider = await invoke<BackendProviderInfo>(
      'update_custom_translation_provider',
      {
        providerId,
        request: toBackendCustomProviderRequest(request),
      },
    );
    return toProviderInfo(provider);
  },
  removeCustomTranslation(providerId) {
    return invoke<void>('remove_custom_translation_provider', { providerId });
  },
  testCustomTranslation(providerId) {
    return invoke<void>('test_custom_translation_provider', { providerId });
  },
  async listTranslationPromptStrategies() {
    const config = await invoke<BackendTranslationPromptStrategyConfig>(
      'list_translation_prompt_strategies',
    );
    return toTranslationPromptStrategyConfig(config);
  },
  async saveTranslationPromptStrategies(config) {
    const saved = await invoke<BackendTranslationPromptStrategyConfig>(
      'save_translation_prompt_strategies',
      { config: toBackendTranslationPromptStrategyConfig(config) },
    );
    return toTranslationPromptStrategyConfig(saved);
  },
  listOpenAICompatibleModels(request) {
    return invoke<BackendProviderModelInfo[]>(
      'list_openai_compatible_models',
      { request: toBackendProviderModelsRequest(request) },
    );
  },
  testOpenAICompatible(request) {
    return invoke<void>('test_openai_compatible_provider', {
      request: toBackendTestProviderRequest(request),
    });
  },
  testOpenAIResponses(request) {
    return invoke<void>('test_openai_responses_provider', {
      request: toBackendTestProviderRequest(request),
    });
  },
  listAnthropicModels(request) {
    return invoke<BackendProviderModelInfo[]>('list_anthropic_models', {
      request: toBackendProviderModelsRequest(request),
    });
  },
  testAnthropic(request) {
    return invoke<void>('test_anthropic_provider', {
      request: toBackendTestProviderRequest(request),
    });
  },
  listGeminiModels(request) {
    return invoke<BackendProviderModelInfo[]>('list_gemini_models', {
      request: toBackendProviderModelsRequest(request),
    });
  },
  testGemini(request) {
    return invoke<void>('test_gemini_provider', {
      request: toBackendTestProviderRequest(request),
    });
  },
  async listOcr() {
    const providers = await invoke<BackendOcrProviderInfo[]>(
      'list_ocr_providers',
    );
    return providers.map(toOcrProviderInfo);
  },
  activateOcr(providerId) {
    return invoke<void>('activate_ocr_provider', { providerId });
  },
  configureOcrCredentials(providerId, credentials) {
    return invoke<void>('configure_ocr_provider_credentials', {
      providerId,
      credentials,
    });
  },
};

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
