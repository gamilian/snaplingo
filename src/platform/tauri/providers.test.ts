import { describe, expect, expectTypeOf, it, vi } from 'vitest';
import type { ProviderInfo } from './providers';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('Tauri providers command adapter', () => {
  it('models built-in provider metadata as explicit nulls', async () => {
    const { listTranslationProviders } = await import('./providers');
    const expected: ProviderInfo[] = [
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
    ];
    invoke.mockResolvedValueOnce(expected);

    await expect(listTranslationProviders()).resolves.toEqual(expected);
    expect(invoke).toHaveBeenCalledWith('list_translation_providers');
    expectTypeOf<ProviderInfo['protocol']>().toEqualTypeOf<string | null>();
    expectTypeOf<ProviderInfo['reasoning_level']>()
      .toEqualTypeOf<string | null>();
  });

  it('activates translation provider with backend parameter name', async () => {
    const { activateTranslationProvider } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await activateTranslationProvider('deeplx');

    expect(invoke).toHaveBeenCalledWith('activate_translation_provider', {
      providerId: 'deeplx',
    });
  });

  it('saves translation credentials as a credentials map', async () => {
    const { configureTranslationProviderCredentials } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await configureTranslationProviderCredentials('baidu-translate', {
      app_id: 'app',
      secret_key: 'secret',
    });

    expect(invoke).toHaveBeenCalledWith('configure_translation_provider_credentials', {
      providerId: 'baidu-translate',
      credentials: { app_id: 'app', secret_key: 'secret' },
    });
  });

  it('loads OCR credential schema from the OCR command', async () => {
    const { getOcrProviderCredentialSchema } = await import('./providers');
    invoke.mockResolvedValueOnce([{ name: 'api_key', label: 'API Key', secret: true }]);

    await getOcrProviderCredentialSchema('baidu-ocr');

    expect(invoke).toHaveBeenCalledWith('get_ocr_provider_credential_schema', {
      providerId: 'baidu-ocr',
    });
  });

  it('saves OCR credentials as a credentials map', async () => {
    const { configureOcrProviderCredentials } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await configureOcrProviderCredentials('baidu-ocr', {
      api_key: 'key',
      secret_key: 'secret',
    });

    expect(invoke).toHaveBeenCalledWith('configure_ocr_provider_credentials', {
      providerId: 'baidu-ocr',
      credentials: { api_key: 'key', secret_key: 'secret' },
    });
  });

  it('loads OpenAI-compatible models with endpoint and api key', async () => {
    const { listOpenAICompatibleModels } = await import('./providers');
    invoke.mockResolvedValueOnce([{ id: 'DeepSeek-V4-Pro' }]);

    await listOpenAICompatibleModels({
      endpoint: 'https://llm.example.test',
      api_key: 'sk-test',
    });

    expect(invoke).toHaveBeenCalledWith('list_openai_compatible_models', {
      request: {
        endpoint: 'https://llm.example.test',
        api_key: 'sk-test',
      },
    });
  });

  it('tests OpenAI-compatible provider with selected model', async () => {
    const { testOpenAICompatibleProvider } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await testOpenAICompatibleProvider({
      endpoint: 'https://llm.example.test',
      api_key: 'sk-test',
      model: 'DeepSeek-V4-Pro',
    });

    expect(invoke).toHaveBeenCalledWith('test_openai_compatible_provider', {
      request: {
        endpoint: 'https://llm.example.test',
        api_key: 'sk-test',
        model: 'DeepSeek-V4-Pro',
      },
    });
  });

  it('tests OpenAI Responses provider with selected model', async () => {
    const { testOpenAIResponsesProvider } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await testOpenAIResponsesProvider({
      endpoint: 'https://api.openai.com',
      api_key: 'sk-test',
      model: 'gpt-5-mini',
    });

    expect(invoke).toHaveBeenCalledWith('test_openai_responses_provider', {
      request: {
        endpoint: 'https://api.openai.com',
        api_key: 'sk-test',
        model: 'gpt-5-mini',
      },
    });
  });

  it('loads Anthropic models with endpoint and api key', async () => {
    const { listAnthropicModels } = await import('./providers');
    invoke.mockResolvedValueOnce([{ id: 'claude-sonnet-4-5' }]);

    await listAnthropicModels({
      endpoint: 'https://api.anthropic.com',
      api_key: 'sk-ant-test',
    });

    expect(invoke).toHaveBeenCalledWith('list_anthropic_models', {
      request: {
        endpoint: 'https://api.anthropic.com',
        api_key: 'sk-ant-test',
      },
    });
  });

  it('tests Anthropic provider with selected model', async () => {
    const { testAnthropicProvider } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await testAnthropicProvider({
      endpoint: 'https://api.anthropic.com',
      api_key: 'sk-ant-test',
      model: 'claude-sonnet-4-5',
    });

    expect(invoke).toHaveBeenCalledWith('test_anthropic_provider', {
      request: {
        endpoint: 'https://api.anthropic.com',
        api_key: 'sk-ant-test',
        model: 'claude-sonnet-4-5',
      },
    });
  });

  it('loads Gemini models with endpoint and api key', async () => {
    const { listGeminiModels } = await import('./providers');
    invoke.mockResolvedValueOnce([{ id: 'gemini-2.5-pro' }]);

    await listGeminiModels({
      endpoint: 'https://generativelanguage.googleapis.com',
      api_key: 'gemini-key',
    });

    expect(invoke).toHaveBeenCalledWith('list_gemini_models', {
      request: {
        endpoint: 'https://generativelanguage.googleapis.com',
        api_key: 'gemini-key',
      },
    });
  });

  it('tests Gemini provider with selected model', async () => {
    const { testGeminiProvider } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await testGeminiProvider({
      endpoint: 'https://generativelanguage.googleapis.com',
      api_key: 'gemini-key',
      model: 'gemini-2.5-pro',
    });

    expect(invoke).toHaveBeenCalledWith('test_gemini_provider', {
      request: {
        endpoint: 'https://generativelanguage.googleapis.com',
        api_key: 'gemini-key',
        model: 'gemini-2.5-pro',
      },
    });
  });

  it('tests a saved custom translation provider by id', async () => {
    const { testCustomTranslationProvider } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await testCustomTranslationProvider('custom-gpt');

    expect(invoke).toHaveBeenCalledWith('test_custom_translation_provider', {
      providerId: 'custom-gpt',
    });
  });

  it('updates a custom translation provider with full provider settings', async () => {
    const { updateCustomTranslationProvider } = await import('./providers');
    invoke.mockResolvedValueOnce(undefined);

    await updateCustomTranslationProvider('custom-gpt', {
      name: 'gpt-5-mini',
      protocol: 'openai-responses',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      api_key: 'sk-test',
      reasoning_level: 'minimal',
    });

    expect(invoke).toHaveBeenCalledWith('update_custom_translation_provider', {
      providerId: 'custom-gpt',
      request: {
        name: 'gpt-5-mini',
        protocol: 'openai-responses',
        endpoint: 'https://api.openai.com',
        model: 'gpt-5-mini',
        api_key: 'sk-test',
        reasoning_level: 'minimal',
      },
    });
  });

  it('loads and saves translation prompt strategies', async () => {
    const {
      listTranslationPromptStrategies,
      saveTranslationPromptStrategies,
    } = await import('./providers');
    const config = {
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
    invoke.mockResolvedValueOnce(config).mockResolvedValueOnce(config);

    await listTranslationPromptStrategies();
    await saveTranslationPromptStrategies(config);

    expect(invoke).toHaveBeenCalledWith('list_translation_prompt_strategies');
    expect(invoke).toHaveBeenCalledWith('save_translation_prompt_strategies', {
      config,
    });
  });
});
