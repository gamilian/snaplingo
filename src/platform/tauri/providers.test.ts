import { describe, expect, expectTypeOf, it, vi } from 'vitest';
import type {
  ProviderInfo,
  TranslationPromptStrategy,
} from '../../application/settings/ports';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('Tauri providers command adapter', () => {
  it('maps serialized provider metadata into the Settings model', async () => {
    const { settingsProviders } = await import('./providers');
    invoke.mockResolvedValueOnce([
      {
        id: 'google-translate',
        name: 'Google Translate',
        is_configured: true,
        requires_api_key: false,
        is_active: true,
        is_builtin: true,
        protocol: null,
        endpoint: null,
        model: null,
        reasoning_level: null,
        prompt_strategy_id: null,
        prompt_fallback_strategy_id: null,
      },
    ]);

    await expect(settingsProviders.listTranslation()).resolves.toEqual([
      {
        id: 'google-translate',
        name: 'Google Translate',
        isConfigured: true,
        requiresApiKey: false,
        isActive: true,
        isBuiltin: true,
        protocol: null,
        endpoint: null,
        model: null,
        reasoningLevel: null,
        promptStrategyId: null,
        promptFallbackStrategyId: null,
      },
    ]);
    expect(invoke).toHaveBeenCalledWith('list_translation_providers');
    expectTypeOf<ProviderInfo['protocol']>().toEqualTypeOf<string | null>();
    expectTypeOf<ProviderInfo['reasoningLevel']>()
      .toEqualTypeOf<string | null>();
  });

  it('maps custom provider requests and returned values at the boundary', async () => {
    const { settingsProviders } = await import('./providers');
    invoke.mockResolvedValueOnce({
      id: 'custom-gpt',
      name: 'Custom GPT',
      is_configured: true,
      requires_api_key: true,
      is_active: false,
      is_builtin: false,
      protocol: 'openai-responses',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      reasoning_level: 'minimal',
      prompt_strategy_id: 'general',
      prompt_fallback_strategy_id: 'general',
    });

    await expect(
      settingsProviders.addCustomTranslation({
        name: 'Custom GPT',
        protocol: 'openai-responses',
        endpoint: 'https://api.openai.com',
        model: 'gpt-5-mini',
        apiKey: 'sk-test',
        reasoningLevel: 'minimal',
        promptStrategyId: 'general',
        promptFallbackStrategyId: 'general',
      }),
    ).resolves.toMatchObject({
      id: 'custom-gpt',
      isConfigured: true,
      requiresApiKey: true,
      reasoningLevel: 'minimal',
      promptStrategyId: 'general',
    });
    expect(invoke).toHaveBeenCalledWith('add_custom_translation_provider', {
      request: {
        name: 'Custom GPT',
        protocol: 'openai-responses',
        endpoint: 'https://api.openai.com',
        model: 'gpt-5-mini',
        api_key: 'sk-test',
        reasoning_level: 'minimal',
        prompt_strategy_id: 'general',
        prompt_fallback_strategy_id: 'general',
      },
    });
  });

  it('maps optional custom provider updates to serialized request fields', async () => {
    const { settingsProviders } = await import('./providers');
    invoke.mockResolvedValueOnce({
      id: 'custom-gpt',
      name: 'Custom GPT',
      is_configured: true,
      requires_api_key: true,
      is_active: false,
      is_builtin: false,
      protocol: 'openai-responses',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      reasoning_level: null,
      prompt_strategy_id: null,
      prompt_fallback_strategy_id: null,
    });

    await settingsProviders.updateCustomTranslation('custom-gpt', {
      name: 'Custom GPT',
      protocol: 'openai-responses',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      apiKey: 'sk-test',
    });

    expect(invoke).toHaveBeenCalledWith('update_custom_translation_provider', {
      providerId: 'custom-gpt',
      request: {
        name: 'Custom GPT',
        protocol: 'openai-responses',
        endpoint: 'https://api.openai.com',
        model: 'gpt-5-mini',
        api_key: 'sk-test',
        reasoning_level: undefined,
        prompt_strategy_id: undefined,
        prompt_fallback_strategy_id: undefined,
      },
    });
  });

  it('maps prompt strategies in both directions', async () => {
    const { settingsProviders } = await import('./providers');
    const backendConfig = {
      strategies: [
        {
          id: 'general',
          name: '通用翻译',
          description: 'Default',
          system_prompt: 'Translate to {target_lang}',
          is_builtin: true,
          is_deletable: false,
        },
      ],
    };
    const applicationConfig = {
      strategies: [
        {
          id: 'general',
          name: '通用翻译',
          description: 'Default',
          systemPrompt: 'Translate to {target_lang}',
          isBuiltin: true,
          isDeletable: false,
        },
      ],
    };
    invoke
      .mockResolvedValueOnce(backendConfig)
      .mockResolvedValueOnce(backendConfig);

    await expect(
      settingsProviders.listTranslationPromptStrategies(),
    ).resolves.toEqual(applicationConfig);
    await expect(
      settingsProviders.saveTranslationPromptStrategies(applicationConfig),
    ).resolves.toEqual(applicationConfig);

    expect(invoke).toHaveBeenCalledWith('list_translation_prompt_strategies');
    expect(invoke).toHaveBeenCalledWith('save_translation_prompt_strategies', {
      config: backendConfig,
    });
    expectTypeOf<TranslationPromptStrategy['systemPrompt']>()
      .toEqualTypeOf<string>();
  });

  it('keeps backend request keys private for model introspection', async () => {
    const { settingsProviders } = await import('./providers');
    invoke.mockResolvedValueOnce([{ id: 'DeepSeek-V4-Pro' }]);

    await settingsProviders.listOpenAICompatibleModels({
      endpoint: 'https://llm.example.test',
      apiKey: 'sk-test',
    });

    expect(invoke).toHaveBeenCalledWith('list_openai_compatible_models', {
      request: {
        endpoint: 'https://llm.example.test',
        api_key: 'sk-test',
      },
    });
  });

  it.each([
    ['testOpenAICompatible', 'test_openai_compatible_provider'],
    ['testOpenAIResponses', 'test_openai_responses_provider'],
    ['testAnthropic', 'test_anthropic_provider'],
    ['testGemini', 'test_gemini_provider'],
  ] as const)('maps %s requests to %s', async (method, command) => {
    const { settingsProviders } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await settingsProviders[method]({
      endpoint: 'https://llm.example.test',
      apiKey: 'sk-test',
      model: 'model',
    });

    expect(invoke).toHaveBeenCalledWith(command, {
      request: {
        endpoint: 'https://llm.example.test',
        api_key: 'sk-test',
        model: 'model',
      },
    });
  });

  it.each([
    ['listAnthropicModels', 'list_anthropic_models'],
    ['listGeminiModels', 'list_gemini_models'],
  ] as const)('maps %s requests to %s', async (method, command) => {
    const { settingsProviders } = await import('./providers');
    invoke.mockResolvedValueOnce([{ id: 'model' }]);

    await settingsProviders[method]({
      endpoint: 'https://llm.example.test',
      apiKey: 'sk-test',
    });

    expect(invoke).toHaveBeenCalledWith(command, {
      request: {
        endpoint: 'https://llm.example.test',
        api_key: 'sk-test',
      },
    });
  });

  it('maps OCR providers and delegates credential operations', async () => {
    const { settingsProviders } = await import('./providers');
    invoke
      .mockResolvedValueOnce([
        {
          id: 'system-ocr',
          name: 'System OCR',
          is_configured: true,
          requires_api_key: false,
          is_active: true,
        },
      ])
      .mockResolvedValueOnce(undefined);

    await expect(settingsProviders.listOcr()).resolves.toEqual([
      {
        id: 'system-ocr',
        name: 'System OCR',
        isConfigured: true,
        requiresApiKey: false,
        isActive: true,
      },
    ]);
    await settingsProviders.configureOcrCredentials('baidu-ocr', {
      api_key: 'key',
      secret_key: 'secret',
    });

    expect(invoke).toHaveBeenCalledWith('configure_ocr_provider_credentials', {
      providerId: 'baidu-ocr',
      credentials: { api_key: 'key', secret_key: 'secret' },
    });
  });

  it('preserves the standalone OCR credential adapter', async () => {
    const { configureOcrProvider } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await configureOcrProvider('baidu-ocr', 'api', 'secret');

    expect(invoke).toHaveBeenCalledWith('configure_ocr_provider', {
      providerId: 'baidu-ocr',
      apiKey: 'api',
      secretKey: 'secret',
    });
  });

  it('delegates provider identity and credential actions', async () => {
    const { settingsProviders } = await import('./providers');
    invoke.mockResolvedValue(undefined);

    await settingsProviders.activateTranslation('deeplx');
    await settingsProviders.deactivateTranslation('deeplx');
    await settingsProviders.reorderActiveTranslation(['deeplx']);
    await settingsProviders.getTranslationCredentialSchema('deeplx');
    await settingsProviders.getOcrCredentialSchema('system-ocr');
    await settingsProviders.configureTranslationCredentials('deeplx', {
      api_key: 'secret',
    });
    await settingsProviders.removeCustomTranslation('custom-gpt');
    await settingsProviders.testCustomTranslation('custom-gpt');
    await settingsProviders.activateOcr('system-ocr');

    expect(invoke).toHaveBeenCalledWith('activate_translation_provider', {
      providerId: 'deeplx',
    });
    expect(invoke).toHaveBeenCalledWith('deactivate_translation_provider', {
      providerId: 'deeplx',
    });
    expect(invoke).toHaveBeenCalledWith('reorder_active_translation_providers', {
      providerIds: ['deeplx'],
    });
    expect(invoke).toHaveBeenCalledWith('get_provider_credential_schema', {
      providerId: 'deeplx',
    });
    expect(invoke).toHaveBeenCalledWith('get_ocr_provider_credential_schema', {
      providerId: 'system-ocr',
    });
    expect(invoke).toHaveBeenCalledWith(
      'configure_translation_provider_credentials',
      { providerId: 'deeplx', credentials: { api_key: 'secret' } },
    );
    expect(invoke).toHaveBeenCalledWith('remove_custom_translation_provider', {
      providerId: 'custom-gpt',
    });
    expect(invoke).toHaveBeenCalledWith('test_custom_translation_provider', {
      providerId: 'custom-gpt',
    });
    expect(invoke).toHaveBeenCalledWith('activate_ocr_provider', {
      providerId: 'system-ocr',
    });
  });
});
