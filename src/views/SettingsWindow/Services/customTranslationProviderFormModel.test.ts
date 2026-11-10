import { describe, expect, it } from 'vitest';

import type { Provider } from '../../../stores/providerStore';
import {
  buildAddCustomProviderRequest,
  buildUpdateCustomProviderRequest,
  canSaveCustomProviderForm,
  DEFAULT_PROMPT_STRATEGY_ID,
  getInitialCustomProviderFormValues,
  getProtocolDefaults,
  getProtocolFamily,
  isLLMProtocol,
  SMART_PROMPT_STRATEGY_ID,
} from './customTranslationProviderFormModel';

describe('customTranslationProviderFormModel', () => {
  it('creates add-mode defaults matching the dialog behavior', () => {
    expect(getInitialCustomProviderFormValues(null)).toEqual({
      name: '',
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-4o',
      reasoningLevel: '',
      promptStrategyId: SMART_PROMPT_STRATEGY_ID,
      promptFallbackStrategyId: DEFAULT_PROMPT_STRATEGY_ID,
    });
  });

  it('creates edit-mode values from an existing custom provider', () => {
    const provider = {
      id: 'custom-claude',
      name: 'Claude',
      type: 'translation',
      status: 'active',
      isBuiltin: false,
      requiresApiKey: true,
      protocol: 'anthropic',
      endpoint: 'https://proxy.example.test/anthropic',
      model: 'claude-3-5-sonnet-latest',
      reasoningLevel: 'low',
      promptStrategyId: 'legal',
      promptFallbackStrategyId: 'general',
    } satisfies Provider;

    expect(getInitialCustomProviderFormValues(provider)).toEqual({
      name: 'Claude',
      protocol: 'anthropic',
      endpoint: 'https://proxy.example.test/anthropic',
      model: 'claude-3-5-sonnet-latest',
      reasoningLevel: 'low',
      promptStrategyId: 'legal',
      promptFallbackStrategyId: DEFAULT_PROMPT_STRATEGY_ID,
    });
  });

  it('falls back to OpenAI defaults for unknown provider protocols', () => {
    const provider = {
      id: 'custom-unknown',
      name: 'Unknown',
      type: 'translation',
      status: 'active',
      isBuiltin: false,
      requiresApiKey: true,
      protocol: 'legacy-protocol',
    } satisfies Provider;

    expect(getInitialCustomProviderFormValues(provider)).toMatchObject({
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-4o',
    });
  });

  it('classifies protocols and returns protocol defaults', () => {
    expect(getProtocolFamily('openai')).toBe('openai');
    expect(getProtocolFamily('openai-responses')).toBe('openai');
    expect(getProtocolFamily('anthropic')).toBe('anthropic');
    expect(getProtocolFamily('gemini')).toBe('gemini');

    expect(getProtocolDefaults('openai-responses')).toEqual({
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
    });
    expect(getProtocolDefaults('anthropic')).toEqual({
      endpoint: 'https://api.anthropic.com',
      model: 'claude-3-5-sonnet-latest',
    });
    expect(getProtocolDefaults('gemini')).toEqual({
      endpoint: 'https://generativelanguage.googleapis.com',
      model: 'gemini-1.5-flash',
    });

    expect(isLLMProtocol('openai-responses')).toBe(true);
    expect(isLLMProtocol('legacy')).toBe(false);
  });

  it('builds trimmed add requests with prompt strategy fallback', () => {
    expect(
      buildAddCustomProviderRequest({
        name: '  My GPT  ',
        protocol: 'openai',
        endpoint: '  https://api.openai.com  ',
        model: '  gpt-4o  ',
        apiKey: '  sk-test  ',
        reasoningLevel: '',
        promptStrategyId: 'smart',
      }),
    ).toEqual({
      name: 'My GPT',
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-4o',
      api_key: 'sk-test',
      reasoning_level: undefined,
      prompt_strategy_id: 'smart',
      prompt_fallback_strategy_id: 'general',
    });
  });

  it('builds edit requests without blank api keys', () => {
    expect(
      buildUpdateCustomProviderRequest({
        name: '  My Claude  ',
        protocol: 'anthropic',
        endpoint: '  https://api.anthropic.com  ',
        model: '  claude-3-5-sonnet-latest  ',
        apiKey: '   ',
        reasoningLevel: 'minimal',
        promptStrategyId: 'legal',
      }),
    ).toEqual({
      name: 'My Claude',
      protocol: 'anthropic',
      endpoint: 'https://api.anthropic.com',
      model: 'claude-3-5-sonnet-latest',
      api_key: undefined,
      reasoning_level: 'minimal',
      prompt_strategy_id: 'legal',
      prompt_fallback_strategy_id: 'general',
    });
  });

  it('rejects incomplete request payloads and mirrors save eligibility', () => {
    const valid = {
      name: 'Provider',
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-4o',
      apiKey: 'sk-test',
      reasoningLevel: '',
      promptStrategyId: 'smart',
    } as const;

    expect(canSaveCustomProviderForm({ ...valid, isSaving: false, isEditing: false })).toBe(true);
    expect(canSaveCustomProviderForm({ ...valid, isSaving: true, isEditing: false })).toBe(false);
    expect(canSaveCustomProviderForm({ ...valid, apiKey: '', isSaving: false, isEditing: false })).toBe(false);
    expect(canSaveCustomProviderForm({ ...valid, apiKey: '', isSaving: false, isEditing: true, canUpdate: true })).toBe(true);
    expect(canSaveCustomProviderForm({ ...valid, apiKey: '', isSaving: false, isEditing: true, canUpdate: false })).toBe(false);
    expect(buildAddCustomProviderRequest({ ...valid, name: '   ' })).toBeNull();
    expect(buildUpdateCustomProviderRequest({ ...valid, endpoint: '   ' })).toBeNull();
  });
});
