import { describe, expect, it } from 'vitest';
import {
  getCustomProviderEndpointPreview,
  getOpenAICompatibleChatPreview,
} from './customTranslationProviderForm';

describe('custom translation provider form helpers', () => {
  it('previews the OpenAI-compatible chat completions URL', () => {
    expect(getOpenAICompatibleChatPreview('https://llm.example.test')).toBe(
      'https://llm.example.test/v1/chat/completions',
    );
    expect(getOpenAICompatibleChatPreview('https://llm.example.test/v1')).toBe(
      'https://llm.example.test/v1/chat/completions',
    );
    expect(
      getOpenAICompatibleChatPreview('https://llm.example.test/v1/chat/completions'),
    ).toBe('https://llm.example.test/v1/chat/completions');
    expect(getOpenAICompatibleChatPreview('https://llm.example.test/v1/chat/')).toBe(
      'https://llm.example.test/v1/chat/completions',
    );
    expect(getOpenAICompatibleChatPreview('https://llm.example.test/chat/completions')).toBe(
      'https://llm.example.test/chat/completions',
    );
    expect(getOpenAICompatibleChatPreview('https://api.openai.com/v1/chat/')).toBe(
      'https://api.openai.com/v1/chat/completions',
    );
    expect(getOpenAICompatibleChatPreview('https://api.openai.com/v1/ch')).toBe(
      'https://api.openai.com/v1/chat/completions',
    );
    expect(getOpenAICompatibleChatPreview('https://llm.example.test/v1/responses')).toBe(
      'https://llm.example.test/v1/responses',
    );
    expect(getOpenAICompatibleChatPreview('https://llm.example.test/v1/models')).toBe(
      'https://llm.example.test/v1/models',
    );
  });

  it('previews the OpenAI Responses request URL', () => {
    expect(
      getCustomProviderEndpointPreview('openai-responses', 'https://api.openai.com', 'gpt-5-mini'),
    ).toBe('https://api.openai.com/v1/responses');
    expect(
      getCustomProviderEndpointPreview(
        'openai-responses',
        'https://proxy.example.test/openai/v1',
        'gpt-5-mini',
      ),
    ).toBe('https://proxy.example.test/openai/v1/responses');
    expect(
      getCustomProviderEndpointPreview(
        'openai-responses',
        'https://api.openai.com/responses',
        'gpt-5-mini',
      ),
    ).toBe('https://api.openai.com/responses');
  });

  it('previews Anthropic and Gemini provider request URLs', () => {
    expect(
      getCustomProviderEndpointPreview(
        'anthropic',
        'https://api.anthropic.com',
        'claude-sonnet-4-5',
      ),
    ).toBe('https://api.anthropic.com/v1/messages');
    expect(
      getCustomProviderEndpointPreview(
        'anthropic',
        'https://api.anthropic.com/v1',
        'claude-sonnet-4-5',
      ),
    ).toBe('https://api.anthropic.com/v1/messages');
    expect(
      getCustomProviderEndpointPreview(
        'anthropic',
        'https://api.anthropic.com/messages',
        'claude-sonnet-4-5',
      ),
    ).toBe('https://api.anthropic.com/messages');
    expect(
      getCustomProviderEndpointPreview(
        'gemini',
        'https://generativelanguage.googleapis.com/v1beta/models',
        'gemini-2.5-pro',
      ),
    ).toBe(
      'https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent',
    );
    expect(
      getCustomProviderEndpointPreview(
        'gemini',
        'https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro',
        'gemini-2.5-pro',
      ),
    ).toBe(
      'https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent',
    );
    expect(
      getCustomProviderEndpointPreview(
        'gemini',
        'https://generativelanguage.googleapis.com/models/gemini-2.5-pro:generateContent',
        'gemini-2.5-pro',
      ),
    ).toBe('https://generativelanguage.googleapis.com/models/gemini-2.5-pro:generateContent');
  });
});
