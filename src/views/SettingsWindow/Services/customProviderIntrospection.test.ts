import { describe, expect, it, vi } from 'vitest';

import {
  loadCustomProviderModels,
  testCustomProviderConnection,
} from './customProviderIntrospection';

describe('customProviderIntrospection', () => {
  it('routes OpenAI model listing through the OpenAI-compatible client', async () => {
    const clients = createClients();
    clients.listOpenAICompatibleModels.mockResolvedValue([{ id: 'gpt-4o' }]);

    const result = await loadCustomProviderModels({
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      apiKey: 'sk-test',
      clients,
    });

    expect(clients.listOpenAICompatibleModels).toHaveBeenCalledWith({
      endpoint: 'https://api.openai.com',
      api_key: 'sk-test',
    });
    expect(result).toEqual({
      models: [{ id: 'gpt-4o' }],
      error: null,
    });
  });

  it('routes OpenAI Responses model listing through the OpenAI-compatible client', async () => {
    const clients = createClients();
    clients.listOpenAICompatibleModels.mockResolvedValue([{ id: 'gpt-5-mini' }]);

    await loadCustomProviderModels({
      protocol: 'openai-responses',
      endpoint: 'https://api.openai.com',
      apiKey: 'sk-test',
      clients,
    });

    expect(clients.listOpenAICompatibleModels).toHaveBeenCalledTimes(1);
  });

  it('routes Anthropic and Gemini model listing through protocol clients', async () => {
    const anthropicClients = createClients();
    anthropicClients.listAnthropicModels.mockResolvedValue([
      { id: 'claude-3-5-sonnet-latest' },
    ]);
    const geminiClients = createClients();
    geminiClients.listGeminiModels.mockResolvedValue([{ id: 'gemini-1.5-flash' }]);

    await loadCustomProviderModels({
      protocol: 'anthropic',
      endpoint: 'https://api.anthropic.com',
      apiKey: 'sk-test',
      clients: anthropicClients,
    });
    await loadCustomProviderModels({
      protocol: 'gemini',
      endpoint: 'https://generativelanguage.googleapis.com',
      apiKey: 'sk-test',
      clients: geminiClients,
    });

    expect(anthropicClients.listAnthropicModels).toHaveBeenCalledTimes(1);
    expect(geminiClients.listGeminiModels).toHaveBeenCalledTimes(1);
  });

  it('returns empty-list and failure messages for model listing', async () => {
    const emptyClients = createClients();
    emptyClients.listOpenAICompatibleModels.mockResolvedValue([]);
    const emptyResult = await loadCustomProviderModels({
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      apiKey: 'sk-test',
      clients: emptyClients,
    });

    expect(emptyResult).toEqual({
      models: [],
      error: '未返回可用模型',
    });

    const failingClients = createClients();
    failingClients.listOpenAICompatibleModels.mockRejectedValue(new Error('offline'));
    const failureResult = await loadCustomProviderModels({
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      apiKey: 'sk-test',
      clients: failingClients,
    });

    expect(failureResult).toEqual({
      models: [],
      error: '获取模型失败: offline',
    });
  });

  it('routes provider connection tests and returns status messages', async () => {
    const clients = createClients();

    await expect(
      testCustomProviderConnection({
        protocol: 'openai',
        endpoint: 'https://api.openai.com',
        apiKey: 'sk-test',
        model: 'gpt-4o',
        clients,
      }),
    ).resolves.toEqual({ type: 'success', message: '检测成功' });
    expect(clients.testOpenAICompatibleProvider).toHaveBeenCalledWith({
      endpoint: 'https://api.openai.com',
      api_key: 'sk-test',
      model: 'gpt-4o',
    });

    await testCustomProviderConnection({
      protocol: 'openai-responses',
      endpoint: 'https://api.openai.com',
      apiKey: 'sk-test',
      model: 'gpt-5-mini',
      clients,
    });
    await testCustomProviderConnection({
      protocol: 'anthropic',
      endpoint: 'https://api.anthropic.com',
      apiKey: 'sk-test',
      model: 'claude-3-5-sonnet-latest',
      clients,
    });
    await testCustomProviderConnection({
      protocol: 'gemini',
      endpoint: 'https://generativelanguage.googleapis.com',
      apiKey: 'sk-test',
      model: 'gemini-1.5-flash',
      clients,
    });

    expect(clients.testOpenAIResponsesProvider).toHaveBeenCalledTimes(1);
    expect(clients.testAnthropicProvider).toHaveBeenCalledTimes(1);
    expect(clients.testGeminiProvider).toHaveBeenCalledTimes(1);
  });

  it('formats provider connection test failures', async () => {
    const clients = createClients();
    clients.testOpenAICompatibleProvider.mockRejectedValue(new Error('bad key'));

    await expect(
      testCustomProviderConnection({
        protocol: 'openai',
        endpoint: 'https://api.openai.com',
        apiKey: 'sk-test',
        model: 'gpt-4o',
        clients,
      }),
    ).resolves.toEqual({
      type: 'error',
      message: '检测失败: bad key',
    });
  });
});

function createClients() {
  return {
    listOpenAICompatibleModels: vi.fn(),
    listAnthropicModels: vi.fn(),
    listGeminiModels: vi.fn(),
    testOpenAICompatibleProvider: vi.fn().mockResolvedValue(undefined),
    testOpenAIResponsesProvider: vi.fn().mockResolvedValue(undefined),
    testAnthropicProvider: vi.fn().mockResolvedValue(undefined),
    testGeminiProvider: vi.fn().mockResolvedValue(undefined),
  };
}
