import { beforeEach, describe, expect, it } from 'vitest';
import { useAppStore } from './appStore';

describe('translation session state', () => {
  beforeEach(() => {
    useAppStore.getState().reset();
  });

  it('starts a new translation session with pending provider cards', () => {
    const sessionId = useAppStore
      .getState()
      .startTranslationSession('first text', ['deeplx', 'google-translate']);

    const state = useAppStore.getState();
    expect(state.translationSessionId).toBe(sessionId);
    expect(state.sourceText).toBe('first text');
    expect(state.providerTranslations).toEqual([
      {
        provider_id: 'deeplx',
        status: 'pending',
        translated_text: '',
        detected_language: null,
        confidence: null,
      },
      {
        provider_id: 'google-translate',
        status: 'pending',
        translated_text: '',
        detected_language: null,
        confidence: null,
      },
    ]);
  });

  it('ignores provider updates from stale translation sessions', () => {
    const staleSessionId = useAppStore
      .getState()
      .startTranslationSession('old text', ['deeplx']);
    const currentSessionId = useAppStore
      .getState()
      .startTranslationSession('new text', ['deeplx']);

    useAppStore.getState().completeProviderTranslation(staleSessionId, {
      provider_id: 'deeplx',
      translated_text: 'old result',
      detected_language: 'en',
      confidence: 1,
    });
    useAppStore.getState().completeProviderTranslation(currentSessionId, {
      provider_id: 'deeplx',
      translated_text: 'new result',
      detected_language: 'en',
      confidence: 1,
    });

    expect(useAppStore.getState().providerTranslations[0].translated_text).toBe(
      'new result',
    );
  });

  it('keeps failed provider cards with error text in the result body', () => {
    const sessionId = useAppStore
      .getState()
      .startTranslationSession('source text', ['deeplx']);

    useAppStore
      .getState()
      .failProviderTranslation(sessionId, 'deeplx', 'Invalid target_lang.');

    expect(useAppStore.getState().providerTranslations).toEqual([
      {
        provider_id: 'deeplx',
        status: 'error',
        translated_text: 'Translation failed: Invalid target_lang.',
        detected_language: null,
        confidence: null,
      },
    ]);
  });

  it('trims leading and trailing whitespace from successful provider results', () => {
    const sessionId = useAppStore
      .getState()
      .startTranslationSession('source text', ['deeplx']);

    useAppStore.getState().completeProviderTranslation(sessionId, {
      provider_id: 'deeplx',
      translated_text: '\n\n  translated result  \n',
      detected_language: 'en',
      confidence: 1,
    });

    const state = useAppStore.getState();
    expect(state.providerTranslations[0].translated_text).toBe('translated result');
    expect(state.translations[0].translated_text).toBe('translated result');
  });

  it('clears previous translation results before applying new source text', () => {
    const sessionId = useAppStore
      .getState()
      .startTranslationSession('old text', ['deeplx']);
    useAppStore.getState().completeProviderTranslation(sessionId, {
      provider_id: 'deeplx',
      translated_text: 'old result',
      detected_language: 'en',
      confidence: 1,
    });

    useAppStore.getState().clearTranslationResults();

    const state = useAppStore.getState();
    expect(state.translations).toEqual([]);
    expect(state.providerTranslations).toEqual([]);
    expect(state.isTranslating).toBe(false);
  });

  it('stores and clears the source image for screenshot OCR results', () => {
    useAppStore.getState().setOcrImageBase64('rendered-image-base64');

    expect(useAppStore.getState().ocrImageBase64).toBe('rendered-image-base64');

    useAppStore.getState().setOcrImageBase64(null);

    expect(useAppStore.getState().ocrImageBase64).toBeNull();
  });

  it('applies translation defaults from durable settings', () => {
    useAppStore.getState().applyTranslationDefaults({
      defaultSourceLang: 'ja',
      defaultTargetLang: 'en',
    });

    expect(useAppStore.getState()).toMatchObject({
      sourceLang: 'ja',
      targetLang: 'en',
    });
  });
});
