import type { Provider } from '../../../stores/providerStore';
import type {
  AddCustomTranslationProviderRequest,
  TranslationPromptStrategy,
  UpdateCustomTranslationProviderRequest,
} from './providerViewTypes';

export type LLMProtocol = 'openai' | 'openai-responses' | 'anthropic' | 'gemini';
export type LLMProtocolFamily = 'openai' | 'anthropic' | 'gemini';

export const SMART_PROMPT_STRATEGY_ID = 'smart';
export const DEFAULT_PROMPT_STRATEGY_ID = 'general';

export const DEFAULT_PROMPT_STRATEGIES: TranslationPromptStrategy[] = [
  {
    id: DEFAULT_PROMPT_STRATEGY_ID,
    name: '通用翻译',
    description: '适合大多数普通文本。',
    system_prompt:
      'You are a professional translation engine. Translate the user text from {source_lang} to {target_lang}. Return only the translation.',
    is_builtin: true,
    is_deletable: false,
  },
];

export const PROTOCOL_OPTIONS: Array<{
  value: LLMProtocolFamily;
  label: string;
}> = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'gemini', label: 'Gemini' },
];

export const OPENAI_MODE_OPTIONS: Array<{ value: LLMProtocol; label: string }> = [
  { value: 'openai', label: 'Chat Completions' },
  { value: 'openai-responses', label: 'Responses' },
];

export const REASONING_OPTIONS = [
  { value: '', label: '默认' },
  { value: 'minimal', label: 'Minimal' },
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
  { value: 'xhigh', label: 'XHigh' },
];

export interface CustomProviderFormValues {
  name: string;
  protocol: LLMProtocol;
  endpoint: string;
  model: string;
  reasoningLevel: string;
  promptStrategyId: string;
  promptFallbackStrategyId: string;
}

export interface CustomProviderFormInput {
  name: string;
  protocol: LLMProtocol;
  endpoint: string;
  model: string;
  apiKey: string;
  reasoningLevel: string;
  promptStrategyId: string;
}

export interface SaveEligibilityInput extends CustomProviderFormInput {
  isSaving: boolean;
  isEditing: boolean;
  canUpdate?: boolean;
}

export function getInitialCustomProviderFormValues(
  provider: Provider | null,
): CustomProviderFormValues {
  const protocol = isLLMProtocol(provider?.protocol) ? provider.protocol : 'openai';
  const defaults = getProtocolDefaults(protocol);

  return {
    name: provider?.name || '',
    protocol,
    endpoint: provider?.endpoint || defaults.endpoint,
    model: provider?.model || defaults.model,
    reasoningLevel: provider?.reasoningLevel || '',
    promptStrategyId: provider?.promptStrategyId || SMART_PROMPT_STRATEGY_ID,
    promptFallbackStrategyId: DEFAULT_PROMPT_STRATEGY_ID,
  };
}

export function getProtocolDefaults(protocol: LLMProtocol) {
  switch (protocol) {
    case 'anthropic':
      return {
        endpoint: 'https://api.anthropic.com',
        model: 'claude-3-5-sonnet-latest',
      };
    case 'openai-responses':
      return {
        endpoint: 'https://api.openai.com',
        model: 'gpt-5-mini',
      };
    case 'gemini':
      return {
        endpoint: 'https://generativelanguage.googleapis.com',
        model: 'gemini-1.5-flash',
      };
    case 'openai':
      return {
        endpoint: 'https://api.openai.com',
        model: 'gpt-4o',
      };
  }
}

export function getProtocolFamily(protocol: LLMProtocol): LLMProtocolFamily {
  return protocol === 'openai-responses' ? 'openai' : protocol;
}

export function isLLMProtocol(value: string | undefined): value is LLMProtocol {
  return (
    value === 'openai' ||
    value === 'openai-responses' ||
    value === 'anthropic' ||
    value === 'gemini'
  );
}

export function canSaveCustomProviderForm(input: SaveEligibilityInput) {
  if (input.isSaving) return false;
  if (!input.name.trim() || !input.endpoint.trim() || !input.model.trim()) {
    return false;
  }
  if (input.isEditing) return Boolean(input.canUpdate);

  return Boolean(input.apiKey.trim());
}

export function buildAddCustomProviderRequest(
  input: CustomProviderFormInput,
): AddCustomTranslationProviderRequest | null {
  const common = buildCommonRequestFields(input);
  if (!common) return null;

  const apiKey = input.apiKey.trim();
  if (!apiKey) return null;

  return {
    ...common,
    api_key: apiKey,
  };
}

export function buildUpdateCustomProviderRequest(
  input: CustomProviderFormInput,
): UpdateCustomTranslationProviderRequest | null {
  const common = buildCommonRequestFields(input);
  if (!common) return null;

  return {
    ...common,
    api_key: input.apiKey.trim() || undefined,
  };
}

export function formatCustomProviderError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function buildCommonRequestFields(input: CustomProviderFormInput) {
  const name = input.name.trim();
  const endpoint = input.endpoint.trim();
  const model = input.model.trim();
  if (!name || !endpoint || !model) return null;

  return {
    name,
    protocol: input.protocol,
    endpoint,
    model,
    reasoning_level: input.reasoningLevel || undefined,
    prompt_strategy_id: input.promptStrategyId,
    prompt_fallback_strategy_id: DEFAULT_PROMPT_STRATEGY_ID,
  };
}
