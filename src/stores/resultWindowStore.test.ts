import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  createResultWindowStatePort,
  useResultWindowStore,
} from './resultWindowStore';

describe('translation session state', () => {
  beforeEach(() => {
    useResultWindowStore.getState().reset();
  });

  it('starts a new translation session with pending provider cards', () => {
    const sessionId = useResultWindowStore
      .getState()
      .startTranslationSession('first text', ['deeplx', 'google-translate']);

    const state = useResultWindowStore.getState();
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
    const staleSessionId = useResultWindowStore
      .getState()
      .startTranslationSession('old text', ['deeplx']);
    const currentSessionId = useResultWindowStore
      .getState()
      .startTranslationSession('new text', ['deeplx']);

    useResultWindowStore
      .getState()
      .completeProviderTranslation(staleSessionId, {
        provider_id: 'deeplx',
        translated_text: 'old result',
        detected_language: 'en',
        confidence: 1,
      });
    useResultWindowStore
      .getState()
      .completeProviderTranslation(currentSessionId, {
        provider_id: 'deeplx',
        translated_text: 'new result',
        detected_language: 'en',
        confidence: 1,
      });

    expect(
      useResultWindowStore.getState().providerTranslations[0].translated_text,
    ).toBe('new result');
  });

  it('keeps failed provider cards with error text in the result body', () => {
    const sessionId = useResultWindowStore
      .getState()
      .startTranslationSession('source text', ['deeplx']);

    useResultWindowStore
      .getState()
      .failProviderTranslation(sessionId, 'deeplx', 'Invalid target_lang.');

    expect(useResultWindowStore.getState().providerTranslations).toEqual([
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
    const sessionId = useResultWindowStore
      .getState()
      .startTranslationSession('source text', ['deeplx']);

    useResultWindowStore.getState().completeProviderTranslation(sessionId, {
      provider_id: 'deeplx',
      translated_text: '\n\n  translated result  \n',
      detected_language: 'en',
      confidence: 1,
    });

    const state = useResultWindowStore.getState();
    expect(state.providerTranslations[0].translated_text).toBe(
      'translated result',
    );
    expect(state.translations[0].translated_text).toBe('translated result');
  });

  it('clears previous translation results before applying new source text', () => {
    const sessionId = useResultWindowStore
      .getState()
      .startTranslationSession('old text', ['deeplx']);
    useResultWindowStore.getState().completeProviderTranslation(sessionId, {
      provider_id: 'deeplx',
      translated_text: 'old result',
      detected_language: 'en',
      confidence: 1,
    });

    useResultWindowStore.getState().clearTranslationResults();

    const state = useResultWindowStore.getState();
    expect(state.translations).toEqual([]);
    expect(state.providerTranslations).toEqual([]);
    expect(state.isTranslating).toBe(false);
  });

  it('stores and clears the source image for screenshot OCR results', () => {
    useResultWindowStore.getState().setOcrImageBase64('rendered-image-base64');

    expect(useResultWindowStore.getState().ocrImageBase64).toBe(
      'rendered-image-base64',
    );

    useResultWindowStore.getState().setOcrImageBase64(null);

    expect(useResultWindowStore.getState().ocrImageBase64).toBeNull();
  });

  it('applies translation defaults from durable settings', () => {
    useResultWindowStore.getState().applyTranslationDefaults({
      defaultSourceLang: 'ja',
      defaultTargetLang: 'en',
    });

    expect(useResultWindowStore.getState()).toMatchObject({
      sourceLang: 'ja',
      targetLang: 'en',
    });
  });

  it('adapts Result Window state and Provider configuration behind one port', async () => {
    const providerState = {
      activeTranslationProviders: [] as string[],
      activeOcrProvider: null as string | null,
    };
    const providers = {
      getState: () => providerState,
      loadTranslation: vi.fn(async () => {
        providerState.activeTranslationProviders = ['google'];
        return [];
      }),
      loadOcr: vi.fn(async () => {
        providerState.activeOcrProvider = 'system';
        return [];
      }),
    } as unknown as Parameters<typeof createResultWindowStatePort>[0];
    const port = createResultWindowStatePort(providers);

    port.setSourceText('hello');

    expect(port.getTranslationSession()).toMatchObject({ sourceText: 'hello' });
    await expect(port.loadActiveTranslationProviderIds()).resolves.toEqual([
      'google',
    ]);
    await expect(port.loadActiveOcrProviderId()).resolves.toBe('system');
    expect(providers.loadTranslation).toHaveBeenCalledTimes(1);
    expect(providers.loadOcr).toHaveBeenCalledTimes(1);
  });
});
