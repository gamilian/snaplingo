import type { ProviderModelInfo } from './providerViewTypes';
import {
  formatCustomProviderError,
  type LLMProtocol,
} from './customTranslationProviderFormModel';

export interface CustomProviderIntrospectionClients {
  listOpenAICompatibleModels(request: {
    endpoint: string;
    api_key: string;
  }): Promise<ProviderModelInfo[]>;
  listAnthropicModels(request: {
    endpoint: string;
    api_key: string;
  }): Promise<ProviderModelInfo[]>;
  listGeminiModels(request: {
    endpoint: string;
    api_key: string;
  }): Promise<ProviderModelInfo[]>;
  testOpenAICompatibleProvider(request: TestProviderRequest): Promise<void>;
  testOpenAIResponsesProvider(request: TestProviderRequest): Promise<void>;
  testAnthropicProvider(request: TestProviderRequest): Promise<void>;
  testGeminiProvider(request: TestProviderRequest): Promise<void>;
}

interface TestProviderRequest {
  endpoint: string;
  api_key: string;
  model: string;
}

export interface LoadCustomProviderModelsInput {
  protocol: LLMProtocol;
  endpoint: string;
  apiKey: string;
  clients: CustomProviderIntrospectionClients;
}

export interface LoadCustomProviderModelsResult {
  models: ProviderModelInfo[];
  error: string | null;
}

export interface TestCustomProviderConnectionInput {
  protocol: LLMProtocol;
  endpoint: string;
  apiKey: string;
  model: string;
  clients: CustomProviderIntrospectionClients;
}

export interface TestCustomProviderConnectionResult {
  type: 'success' | 'error';
  message: string;
}

export async function loadCustomProviderModels({
  apiKey,
  clients,
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
  clients,
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
