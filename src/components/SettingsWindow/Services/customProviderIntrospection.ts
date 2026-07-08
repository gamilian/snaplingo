import {
  listAnthropicModels,
  listGeminiModels,
  listOpenAICompatibleModels,
  testAnthropicProvider,
  testGeminiProvider,
  testOpenAICompatibleProvider,
  testOpenAIResponsesProvider,
  type OpenAICompatibleModelInfo,
} from '../../../tauri/providers';
import {
  formatCustomProviderError,
  type LLMProtocol,
} from './customTranslationProviderFormModel';

export interface CustomProviderIntrospectionClients {
  listOpenAICompatibleModels: typeof listOpenAICompatibleModels;
  listAnthropicModels: typeof listAnthropicModels;
  listGeminiModels: typeof listGeminiModels;
  testOpenAICompatibleProvider: typeof testOpenAICompatibleProvider;
  testOpenAIResponsesProvider: typeof testOpenAIResponsesProvider;
  testAnthropicProvider: typeof testAnthropicProvider;
  testGeminiProvider: typeof testGeminiProvider;
}

export interface LoadCustomProviderModelsInput {
  protocol: LLMProtocol;
  endpoint: string;
  apiKey: string;
  clients?: CustomProviderIntrospectionClients;
}

export interface LoadCustomProviderModelsResult {
  models: OpenAICompatibleModelInfo[];
  error: string | null;
}

export interface TestCustomProviderConnectionInput {
  protocol: LLMProtocol;
  endpoint: string;
  apiKey: string;
  model: string;
  clients?: CustomProviderIntrospectionClients;
}

export interface TestCustomProviderConnectionResult {
  type: 'success' | 'error';
  message: string;
}

const defaultClients: CustomProviderIntrospectionClients = {
  listOpenAICompatibleModels,
  listAnthropicModels,
  listGeminiModels,
  testOpenAICompatibleProvider,
  testOpenAIResponsesProvider,
  testAnthropicProvider,
  testGeminiProvider,
};

export async function loadCustomProviderModels({
  apiKey,
  clients = defaultClients,
  endpoint,
  protocol,
}: LoadCustomProviderModelsInput): Promise<LoadCustomProviderModelsResult> {
  try {
    const request = {
      endpoint,
      api_key: apiKey,
    };
    const models =
      protocol === 'openai' || protocol === 'openai-responses'
        ? await clients.listOpenAICompatibleModels(request)
        : protocol === 'anthropic'
          ? await clients.listAnthropicModels(request)
          : await clients.listGeminiModels(request);

    return {
      models,
      error: models.length === 0 ? '未返回可用模型' : null,
    };
  } catch (error) {
    return {
      models: [],
      error: `获取模型失败: ${formatCustomProviderError(error)}`,
    };
  }
}

export async function testCustomProviderConnection({
  apiKey,
  clients = defaultClients,
  endpoint,
  model,
  protocol,
}: TestCustomProviderConnectionInput): Promise<TestCustomProviderConnectionResult> {
  try {
    const request = {
      endpoint,
      api_key: apiKey,
      model,
    };

    if (protocol === 'openai') {
      await clients.testOpenAICompatibleProvider(request);
    } else if (protocol === 'openai-responses') {
      await clients.testOpenAIResponsesProvider(request);
    } else if (protocol === 'anthropic') {
      await clients.testAnthropicProvider(request);
    } else {
      await clients.testGeminiProvider(request);
    }

    return { type: 'success', message: '检测成功' };
  } catch (error) {
    return {
      type: 'error',
      message: `检测失败: ${formatCustomProviderError(error)}`,
    };
  }
}
