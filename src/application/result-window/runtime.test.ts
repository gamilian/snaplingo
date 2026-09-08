import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  createResultWindowRuntime,
  resultWindowStandaloneWindowHeight,
} from './runtime';
import type { ResultWindowState } from './projection';
import type { TranslationResult } from '../../types';
import type { TranslationSettings } from '../settings/ports';
import type {
  CaptureResultWindowPayload,
  ResultPayloadReadyHandler,
  ResultWindowUnsubscribe,
} from './ports';

describe('result window application runtime', () => {
  afterEach(() => { vi.useRealTimers(); });
  it('hydrates the current translation payload by request ID', async () => {
    const payload: CaptureResultWindowPayload = {
      mode: 'translation', origin: 'selection',
      text: 'Visit https://example.\ncom', autoTranslate: true,
    };
    const { runtime, platform } = createRuntime({
      currentPayloadRequestId: '42', payloads: { '42': payload },
      activeProviderIds: ['google'],
    });
    await expect(runtime.loadCurrentPayload()).resolves.toBe(true);
    expect(platform.commands.currentPayloadRequestId).toHaveBeenCalledOnce();
    expect(platform.commands.takePayload).toHaveBeenCalledWith('42');
    expect(runtime.getState()).toMatchObject({
      sourceText: 'Visit https://example.com', resultWindowOrigin: 'selection',
      resultWindowVisible: true, resultWindowMode: 'translation',
    });
    await vi.waitFor(() => expect(platform.commands.translateTextWithProvider)
      .toHaveBeenCalledExactlyOnceWith('google', {
        text: 'Visit https://example.com', sourceLang: 'auto', targetLang: 'zh-CN',
      }));
  });

  it('uses OCR detected language as the initial source language for screenshot translation', async () => {
    const { runtime } = createRuntime();

    await runtime.applyPayload({
      mode: 'translation',
      origin: 'screenshot',
      text: 'Bonjour',
      autoTranslate: true,
      detectedLanguage: 'fr',
    });

    expect(runtime.getState()).toMatchObject({ sourceLang: 'fr', sourceText: 'Bonjour' });
  });

  it('does not take a payload when no current request ID exists', async () => {
    const { runtime, platform } = createRuntime({
      currentPayloadRequestId: null,
    });

    await expect(runtime.loadCurrentPayload()).resolves.toBe(false);

    expect(platform.commands.takePayload).not.toHaveBeenCalled();
    expect(runtime.getState().resultWindowVisible).toBe(false);
  });

  it('subscribes to payload-ready events and takes only the matching payload', async () => {
    const payload: CaptureResultWindowPayload = {
      mode: 'ocr',
      origin: 'ocr',
      text: 'recognized',
      autoTranslate: false,
      ocrIntent: 'display-text',
      imageBase64: 'image-base64',
    };
    const onLoaded = vi.fn();
    const { runtime, platform, emitPayloadReady, unsubscribe } =
      createRuntime({
        payloads: { '7': payload },
      });

    await expect(runtime.subscribeToPayloads(onLoaded)).resolves.toBe(
      unsubscribe,
    );
    await emitPayloadReady('7');
    await Promise.resolve();

    expect(platform.commands.takePayload).toHaveBeenCalledWith('7');
    expect(runtime.getState()).toMatchObject({
      ocrText: 'recognized', ocrImageBase64: 'image-base64',
      resultWindowOrigin: 'ocr', resultWindowMode: 'ocr', resultWindowVisible: true,
    });
    expect(onLoaded).toHaveBeenCalledTimes(1);
  });

  it('contains payload-ready hydration failures inside the runtime handler', async () => {
    const onLoaded = vi.fn();
    const { runtime, emitPayloadReady } = createRuntime({
      takePayloadError: new Error('payload unavailable'),
    });

    await runtime.subscribeToPayloads(onLoaded);
    await expect(emitPayloadReady('7')).resolves.toBeUndefined();
    await Promise.resolve();

    expect(onLoaded).not.toHaveBeenCalled();
  });

  it('starts OCR file workflow from a file-intent payload', async () => {
    const { runtime, platform, projectedStates } = createRuntime({
      selectedImagePath: '/tmp/example.png', recognizedFileText: 'file text',
    });
    await runtime.applyPayload({
      mode: 'ocr', text: '', autoTranslate: false, ocrIntent: 'file',
    });
    expect(platform.commands.selectImageFile).toHaveBeenCalledOnce();
    expect(platform.commands.recognizeImageFile).toHaveBeenCalledWith('/tmp/example.png');
    expect(projectedStates).toContainEqual(expect.objectContaining({
      ocrText: '', ocrImageBase64: null, isOcrRunning: true,
    }));
    expect(runtime.getState()).toMatchObject({
      resultWindowVisible: true, resultWindowMode: 'ocr',
      ocrText: 'file text', ocrImageBase64: 'data:image/png;base64,aW1hZ2U=',
      isOcrRunning: false,
    });
    expect(platform.clipboard.copyText).not.toHaveBeenCalled();
  });

  it('favorites OCR with the retained image and configured language', async () => {
    const { runtime, platform } = createRuntime({
      ocrSettings: {
        recognitionLanguage: 'ja',
        preserveFormatting: true,
        removeChineseSpaces: true,
        showConfidence: true,
      },
    });

    await runtime.favoriteOcrResult(
      'data:image/png;base64,AQID',
      'recognized',
      0.9,
    );

    expect(platform.commands.favoriteOcrResult).toHaveBeenCalledWith({
      imageData: new Uint8Array([1, 2, 3]),
      result: { text: 'recognized', confidence: 0.9 },
      language: 'ja',
      providerUsed: 'manual',
    });
  });

  it('owns OCR provider fallback when favoriting a result', async () => {
    const { runtime, platform } = createRuntime({
      activeOcrProviderId: 'system',
    });

    await runtime.favoriteOcrResult(null, 'recognized', null);

    expect(platform.commands.favoriteOcrResult).toHaveBeenCalledWith({
      imageData: [],
      result: { text: 'recognized', confidence: null },
      language: undefined,
      providerUsed: 'system',
    });
  });

  it('owns single and aggregate translation favorite workflows', async () => {
    const { runtime, platform } = createRuntime();
    const google = {
      provider_id: 'google',
      translated_text: '你好',
      detected_language: 'en',
      confidence: null,
    };
    const deeplx = {
      provider_id: 'deeplx',
      translated_text: '您好',
      detected_language: 'en',
      confidence: null,
    };

    await runtime.favoriteTranslationResult({
      text: 'hello',
      sourceLang: 'en',
      targetLang: 'auto',
      result: google,
    });
    await runtime.favoriteTranslationResults({
      text: '你好',
      sourceLang: 'zh-CN',
      targetLang: 'auto',
      results: [google, deeplx],
    });

    expect(platform.commands.favoriteTranslationResult).toHaveBeenNthCalledWith(
      1,
      {
        text: 'hello',
        sourceLang: 'en',
        targetLang: 'zh-CN',
        result: google,
      },
    );
    expect(platform.commands.favoriteTranslationResult).toHaveBeenNthCalledWith(
      2,
      {
        text: '你好',
        sourceLang: 'zh-CN',
        targetLang: 'en',
        result: google,
      },
    );
    expect(platform.commands.favoriteTranslationResult).toHaveBeenNthCalledWith(
      3,
      {
        text: '你好',
        sourceLang: 'zh-CN',
        targetLang: 'en',
        result: deeplx,
      },
    );
  });

  it('exposes result-window intents without leaking platform adapters', async () => {
    const { runtime, platform } = createRuntime();

    await runtime.copyText('sample');

    expect(platform.clipboard.copyText).toHaveBeenCalledWith('sample');
    expect(runtime).not.toHaveProperty('commands');
    expect(runtime).not.toHaveProperty('clipboard');
  });

  it('owns editable projection intents and language pair policy', async () => {
    const { runtime, providers } = createRuntime({ activeProviderIds: ['google'] });
    runtime.updateSourceText('updated');
    runtime.changeSourceLanguage('ja');
    expect(runtime.getState()).toMatchObject({ sourceLang: 'ja', targetLang: 'zh-CN' });
    runtime.changeTargetLanguage('en');
    runtime.swapTranslationLanguages();
    runtime.updateOcrText('recognized');
    runtime.clearOcrImage();
    await runtime.loadTranslationProviders();
    expect(runtime.getState()).toMatchObject({
      sourceText: 'updated', sourceLang: 'en', targetLang: 'ja',
      ocrText: 'recognized', ocrImageBase64: null,
    });
    expect(providers.loadTranslation).toHaveBeenCalledOnce();
  });

  it('owns provider fan-out and records one aggregate translation history entry', async () => {
    const { runtime, platform } = createRuntime({
      activeProviderIds: ['google', 'deeplx'],
      translationResults: {
        google: {
          provider_id: 'google',
          translated_text: '你好',
          detected_language: 'en',
          confidence: null,
        },
        deeplx: {
          provider_id: 'deeplx',
          translated_text: '您好',
          detected_language: 'en',
          confidence: null,
        },
      },
    });

    await runtime.translate({
      text: 'hello',
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });

    expect(runtime.getState()).toMatchObject({
      sourceText: 'hello', isTranslating: false,
      providerTranslations: [
        expect.objectContaining({ provider_id: 'google', status: 'success' }),
        expect.objectContaining({ provider_id: 'deeplx', status: 'success' }),
      ],
    });
    expect(platform.commands.translateTextWithProvider).toHaveBeenCalledTimes(2);
    expect(platform.commands.recordTranslationHistory).toHaveBeenCalledWith(
      expect.objectContaining({
        text: 'hello',
        sourceLang: 'en',
        targetLang: 'zh-CN',
        results: [
          expect.objectContaining({ provider_id: 'google' }),
          expect.objectContaining({ provider_id: 'deeplx' }),
        ],
      }),
    );
  });

  it('applies translation text and clipboard preferences', async () => {
    const { runtime, platform } = createRuntime({
      activeProviderIds: ['google'],
      translationSettings: {
        defaultSourceLang: 'auto',
        defaultTargetLang: 'zh-CN',
        autoTranslate: true,
        autoCopy: true,
        preserveLineBreaks: false,
        incrementalTranslation: false,
        windowAlwaysOnTop: true,
        hideOnBlur: false,
      },
      translationResults: {
        google: {
          provider_id: 'google',
          translated_text: '译文',
          detected_language: 'en',
          confidence: null,
        },
      },
    });

    await runtime.translate({
      text: 'first\n  second',
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });

    expect(runtime.getState().sourceText).toBe('first\n  second');
    expect(platform.commands.translateTextWithProvider).toHaveBeenCalledWith(
      'google',
      expect.objectContaining({ text: 'first second' }),
    );
    expect(platform.commands.recordTranslationHistory).toHaveBeenCalledWith(
      expect.objectContaining({ text: 'first\n  second' }),
    );
    expect(platform.clipboard.copyText).toHaveBeenCalledWith('译文');
  });

  it('keeps a completed translation when automatic copy fails', async () => {
    const { runtime, platform } = createRuntime({
      activeProviderIds: ['google'],
      translationSettings: {
        defaultSourceLang: 'auto',
        defaultTargetLang: 'zh-CN',
        autoTranslate: true,
        autoCopy: true,
        preserveLineBreaks: true,
        incrementalTranslation: false,
        windowAlwaysOnTop: true,
        hideOnBlur: false,
      },
      translationResults: {
        google: {
          provider_id: 'google',
          translated_text: '译文',
          detected_language: 'en',
          confidence: null,
        },
      },
    });
    platform.clipboard.copyText.mockRejectedValue(
      new Error('clipboard unavailable'),
    );
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    await expect(
      runtime.translate({
        text: 'hello',
        sourceLang: 'en',
        targetLang: 'zh-CN',
      }),
    ).resolves.toBeUndefined();

    expect(platform.commands.recordTranslationHistory).toHaveBeenCalledTimes(1);
    consoleError.mockRestore();
  });

  it('renders structured provider failures without recording them as translations', async () => {
    const { runtime, platform } = createRuntime({
      activeProviderIds: ['google'],
      translationResults: {
        google: {
          provider_id: 'google',
          translated_text: '',
          detected_language: null,
          confidence: null,
          error: {
            code: 'invalid_request',
            message: 'Invalid target language',
            retryable: false,
          },
        },
      },
    });

    await runtime.translate({
      text: 'hello',
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });

    expect(runtime.getState()).toMatchObject({
      isTranslating: false,
      providerTranslations: [expect.objectContaining({
        provider_id: 'google', status: 'error', translated_text: '',
        error: expect.objectContaining({ code: 'invalid_request' }),
      })],
    });
    expect(platform.commands.recordTranslationHistory).not.toHaveBeenCalled();
  });

  it('ignores a late result from an older translation generation', async () => {
    const first = deferred<import('../../types').TranslationResult>();
    const second = deferred<import('../../types').TranslationResult>();
    const { runtime, platform } = createRuntime({
      activeProviderIds: ['google'],
      translationResults: {
        google: {
          provider_id: 'google',
          translated_text: 'unused',
          detected_language: 'en',
          confidence: null,
        },
      },
    });
    platform.commands.translateTextWithProvider
      .mockImplementationOnce(async () => first.promise)
      .mockImplementationOnce(async () => second.promise);

    const firstRun = runtime.translate({
      text: 'first',
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });
    await vi.waitFor(() =>
      expect(platform.commands.translateTextWithProvider).toHaveBeenCalledOnce(),
    );
    const secondRun = runtime.translate({
      text: 'second',
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });
    await vi.waitFor(() =>
      expect(platform.commands.translateTextWithProvider).toHaveBeenCalledTimes(2),
    );

    first.resolve({
      provider_id: 'google',
      translated_text: 'old result',
      detected_language: 'en',
      confidence: null,
    });
    second.resolve({
      provider_id: 'google',
      translated_text: 'new result',
      detected_language: 'en',
      confidence: null,
    });
    await Promise.all([firstRun, secondRun]);

    expect(runtime.getState().providerTranslations).toEqual([
      expect.objectContaining({ translated_text: 'new result', status: 'success' }),
    ]);
    expect(platform.commands.recordTranslationHistory).toHaveBeenCalledTimes(1);
    expect(platform.commands.recordTranslationHistory).toHaveBeenCalledWith(
      expect.objectContaining({ text: 'second' }),
    );
  });

  it('updates a successful provider retry without duplicating history', async () => {
    const { runtime, platform } = createRuntime({ activeProviderIds: ['google'] });
    await runtime.translate({ text: 'hello', sourceLang: 'en', targetLang: 'zh-CN' });
    platform.commands.recordTranslationHistory.mockClear();
    platform.commands.translateTextWithProvider.mockResolvedValue(translationResult('google', 'retry'));
    await runtime.retryTranslationProvider('google');
    expect(runtime.getState().providerTranslations).toEqual([
      expect.objectContaining({ provider_id: 'google', status: 'success', translated_text: 'retry' }),
    ]);
    expect(platform.commands.translateTextWithProvider).toHaveBeenCalledTimes(2);
    expect(platform.commands.recordTranslationHistory).not.toHaveBeenCalled();
  });

  it('closes overlay state locally and standalone state plus native window', async () => {
    const { runtime, platform } = createRuntime();

    await runtime.close('overlay');
    await runtime.close('standalone');

    expect(runtime.getState().resultWindowVisible).toBe(false);
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
  });

  it('keeps standalone close state hidden when native hide fails', async () => {
    const { runtime } = createRuntime({
      dismissError: new Error('window unavailable'),
    });

    await expect(runtime.close('standalone')).resolves.toBeUndefined();

    expect(runtime.getState().resultWindowVisible).toBe(false);
  });

  it('resizes only visible standalone result windows', async () => {
    const { runtime, platform } = createRuntime();

    await runtime.resizeStandaloneWindow({
      presentation: 'overlay',
      visible: true,
      mode: 'translation',
      panelHeightPx: 300,
    });
    await runtime.resizeStandaloneWindow({
      presentation: 'standalone',
      visible: false,
      mode: 'translation',
      panelHeightPx: 300,
    });
    await runtime.resizeStandaloneWindow({
      presentation: 'standalone',
      visible: true,
      mode: 'translation',
      panelHeightPx: 300,
    });

    expect(platform.resizeTo).toHaveBeenCalledOnce();
    expect(platform.resizeTo).toHaveBeenCalledWith(
      660,
      resultWindowStandaloneWindowHeight(300),
    );
  });

  it('applies configured width only to translation windows', async () => {
    const { runtime, platform } = createRuntime({
      translationSettings: {
        defaultSourceLang: 'auto',
        defaultTargetLang: 'auto',
        autoTranslate: true,
        autoCopy: false,
        preserveLineBreaks: true,
        incrementalTranslation: false,
        windowAlwaysOnTop: true,
        hideOnBlur: false,
        windowWidth: 720,
      },
    });

    await runtime.resizeStandaloneWindow({
      presentation: 'standalone',
      visible: true,
      mode: 'translation',
      panelHeightPx: 300,
    });
    await runtime.resizeStandaloneWindow({
      presentation: 'standalone',
      visible: true,
      mode: 'ocr',
      panelHeightPx: 300,
    });

    expect(platform.resizeTo).toHaveBeenNthCalledWith(
      1,
      720,
      resultWindowStandaloneWindowHeight(300),
    );
    expect(platform.resizeTo).toHaveBeenNthCalledWith(
      2,
      660,
      resultWindowStandaloneWindowHeight(300),
    );
  });

  it('places translation and OCR windows from their configured trigger settings', async () => {
    const translation = createRuntime({
      translationSettings: {
        defaultSourceLang: 'auto',
        defaultTargetLang: 'auto',
        autoTranslate: true,
        autoCopy: false,
        preserveLineBreaks: true,
        incrementalTranslation: false,
        windowAlwaysOnTop: true,
        hideOnBlur: false,
        selectionWindowPosition: 'cursor',
        inputWindowPosition: 'center',
      },
    });

    await translation.runtime.resizeStandaloneWindow({
      presentation: 'standalone',
      visible: true,
      mode: 'translation',
      origin: 'selection',
      panelHeightPx: 300,
    });

    expect(translation.platform.placeAt).toHaveBeenCalledWith('cursor');

    const ocr = createRuntime({
      ocrSettings: {
        recognitionLanguage: 'auto',
        preserveFormatting: true,
        removeChineseSpaces: true,
        showConfidence: false,
        windowPosition: 'below-cursor',
      },
    });

    await ocr.runtime.resizeStandaloneWindow({
      presentation: 'standalone',
      visible: true,
      mode: 'ocr',
      origin: 'ocr',
      panelHeightPx: 300,
    });

    expect(ocr.platform.placeAt).toHaveBeenCalledWith('below-cursor');
  });

  it('passes the durable last position to the window adapter', async () => {
    const { runtime, platform } = createRuntime({
      lastWindowPosition: { x: 420, y: 240 },
      translationSettings: {
        defaultSourceLang: 'auto',
        defaultTargetLang: 'auto',
        autoTranslate: true,
        autoCopy: false,
        preserveLineBreaks: true,
        incrementalTranslation: false,
        windowAlwaysOnTop: true,
        hideOnBlur: false,
        inputWindowPosition: 'last-position',
      },
    });

    await runtime.resizeStandaloneWindow({
      presentation: 'standalone',
      visible: true,
      mode: 'translation',
      origin: 'input',
      panelHeightPx: 300,
    });

    expect(platform.placeAt).toHaveBeenCalledWith('last-position', {
      x: 420,
      y: 240,
    });
  });

  it('persists the final position returned by a user drag', async () => {
    const { runtime, saveLastWindowPosition } = createRuntime({
      draggedWindowPosition: { x: 640, y: 360 },
    });

    await runtime.beginDrag();

    expect(saveLastWindowPosition).toHaveBeenCalledWith({ x: 640, y: 360 });
  });

  it('owns speech normalization instead of leaving it in the View', async () => {
    const { runtime, speech } = createRuntime();

    await runtime.speakText('\n hello \n', 'en');
    await runtime.speakText('   ');

    expect(speech.speak).toHaveBeenCalledOnce();
    expect(speech.speak).toHaveBeenCalledWith('hello', 'en-US');
  });

  it.each([[false, 500], [true, 150]] as const)(
    'debounces automatic translation (incremental=%s) for %d ms',
    async (incremental, delay) => {
      vi.useFakeTimers();
      const { runtime, platform } = createRuntime({
        activeProviderIds: ['google'],
        translationSettings: automaticTranslationSettings(incremental),
      });
      await runtime.applyPayload({ mode: 'translation', text: '', autoTranslate: false });
      runtime.updateSourceText('h');
      await vi.advanceTimersByTimeAsync(delay - 1);
      runtime.updateSourceText('hello');
      await vi.advanceTimersByTimeAsync(delay - 1);
      expect(platform.commands.translateTextWithProvider).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(1);
      expect(platform.commands.translateTextWithProvider).toHaveBeenCalledExactlyOnceWith(
        'google', { text: 'hello', sourceLang: 'auto', targetLang: 'zh-CN' },
      );
      runtime.updateSourceText('hello');
      await vi.runAllTimersAsync();
      expect(platform.commands.translateTextWithProvider).toHaveBeenCalledOnce();
    },
  );

  it.each(['payload', 'manual'] as const)('does not repeat a %s translation after debounce', async (trigger) => {
    vi.useFakeTimers();
    const { runtime, platform } = createRuntime({
      activeProviderIds: ['google'], translationSettings: automaticTranslationSettings(),
    });
    await runtime.applyPayload({
      mode: 'translation', text: 'hello', autoTranslate: trigger === 'payload',
    });
    if (trigger === 'manual') {
      await runtime.translate({ text: 'hello', sourceLang: 'auto', targetLang: 'zh-CN' });
    }
    await vi.runAllTimersAsync();
    expect(platform.commands.translateTextWithProvider).toHaveBeenCalledOnce();
  });

  it('reschedules when translation preferences change', async () => {
    vi.useFakeTimers();
    const options = { activeProviderIds: ['google'], translationSettings: automaticTranslationSettings() };
    const { runtime, platform } = createRuntime(options);
    await runtime.applyPayload({ mode: 'translation', text: 'hello', autoTranslate: false });
    options.translationSettings = { ...options.translationSettings, autoTranslate: false };
    runtime.applyTranslationDefaults(options.translationSettings);
    await vi.runAllTimersAsync();
    expect(platform.commands.translateTextWithProvider).not.toHaveBeenCalled();
    options.translationSettings = automaticTranslationSettings(true);
    runtime.applyTranslationDefaults(options.translationSettings);
    await vi.advanceTimersByTimeAsync(150);
    expect(platform.commands.translateTextWithProvider).toHaveBeenCalledOnce();
  });

  it('reopens retained translation results without automatically translating them again', async () => {
    vi.useFakeTimers();
    const { runtime, platform } = createRuntime({
      activeProviderIds: ['google'],
      translationSettings: automaticTranslationSettings(),
    });
    await runtime.applyPayload({ mode: 'translation', text: 'hello', autoTranslate: false });
    await runtime.translate({ text: 'hello', sourceLang: 'auto', targetLang: 'zh-CN' });
    await runtime.close('overlay');

    await runtime.applyPayload({ mode: 'translation', text: '', autoTranslate: false });
    await vi.runAllTimersAsync();

    expect(runtime.getState()).toMatchObject({
      sourceText: 'hello',
      resultWindowVisible: true,
      providerTranslations: [expect.objectContaining({ status: 'success' })],
    });
    expect(platform.commands.translateTextWithProvider).toHaveBeenCalledOnce();
  });

  it('reopens a retained provider failure with its retry action still available', async () => {
    const { runtime, platform } = createRuntime({ activeProviderIds: ['google'] });
    platform.commands.translateTextWithProvider.mockRejectedValueOnce(new Error('offline'));
    await runtime.applyPayload({ mode: 'translation', text: 'hello', autoTranslate: false });
    await runtime.translate({ text: 'hello', sourceLang: 'auto', targetLang: 'zh-CN' });
    await runtime.close('overlay');

    await runtime.applyPayload({ mode: 'translation', text: '', autoTranslate: false });
    await runtime.retryTranslationProvider('google');

    expect(platform.commands.translateTextWithProvider).toHaveBeenCalledTimes(2);
    expect(runtime.getState().providerTranslations[0]).toMatchObject({ status: 'success' });
  });

  it.each(['language', 'text', 'close', 'ocr'] as const)('ignores a pending result after a %s intent', async (intent) => {
    const pending = deferred<TranslationResult>();
    const { runtime, platform } = createRuntime({ activeProviderIds: ['google'] });
    platform.commands.translateTextWithProvider.mockReturnValueOnce(pending.promise);
    const run = runtime.translate({ text: 'hello', sourceLang: 'en', targetLang: 'zh-CN' });
    await vi.waitFor(() => expect(platform.commands.translateTextWithProvider).toHaveBeenCalledOnce());
    if (intent === 'language') runtime.changeTargetLanguage('ja');
    if (intent === 'text') runtime.updateSourceText('new text');
    if (intent === 'close') await runtime.close('overlay');
    if (intent === 'ocr') await runtime.startFileOcr();
    pending.resolve(translationResult('google', 'obsolete'));
    await run;
    expect(runtime.getState().providerTranslations).not.toContainEqual(
      expect.objectContaining({ translated_text: 'obsolete' }),
    );
    expect(runtime.getState().isTranslating).toBe(false);
    expect(platform.commands.recordTranslationHistory).not.toHaveBeenCalled();
    await runtime.retryTranslationProvider('google');
    expect(platform.commands.translateTextWithProvider).toHaveBeenCalledOnce();
  });

  it.each(['close', 'blank', 'ocr'] as const)('cancels scheduled work after %s', async (intent) => {
    vi.useFakeTimers();
    const { runtime, platform } = createRuntime({
      activeProviderIds: ['google'], translationSettings: automaticTranslationSettings(),
    });
    await runtime.applyPayload({ mode: 'translation', text: 'hello', autoTranslate: false });
    if (intent === 'close') await runtime.close('standalone');
    if (intent === 'blank') runtime.updateSourceText('  ');
    if (intent === 'ocr') await runtime.startFileOcr();
    await vi.runAllTimersAsync();
    expect(platform.commands.translateTextWithProvider).not.toHaveBeenCalled();
  });

  it('publishes pending cards and trims successful results in provider order', async () => {
    const first = deferred<TranslationResult>();
    const second = deferred<TranslationResult>();
    const { runtime, platform } = createRuntime({ activeProviderIds: ['google', 'deeplx'] });
    platform.commands.translateTextWithProvider
      .mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const run = runtime.translate({ text: 'hello', sourceLang: 'en', targetLang: 'zh-CN' });
    await vi.waitFor(() => expect(runtime.getState().providerTranslations).toHaveLength(2));
    expect(runtime.getState().providerTranslations.map(({ provider_id, status }) => [provider_id, status]))
      .toEqual([['google', 'pending'], ['deeplx', 'pending']]);
    second.resolve(translationResult('deeplx', '\n second \n'));
    first.resolve(translationResult('google', '\n first \n'));
    await run;
    expect(runtime.getState().providerTranslations.map(({ translated_text }) => translated_text))
      .toEqual(['first', 'second']);
    expect(runtime.getState().isTranslating).toBe(false);
  });

  it('keeps other provider requests valid while retrying one failed provider', async () => {
    const other = deferred<TranslationResult>();
    const retry = deferred<TranslationResult>();
    const { runtime, platform } = createRuntime({ activeProviderIds: ['google', 'deeplx'] });
    platform.commands.translateTextWithProvider
      .mockRejectedValueOnce(new Error('offline'))
      .mockReturnValueOnce(other.promise)
      .mockReturnValueOnce(retry.promise);
    const run = runtime.translate({ text: 'hello', sourceLang: 'en', targetLang: 'zh-CN' });
    await vi.waitFor(() => expect(runtime.getState().providerTranslations[0]?.status).toBe('error'));
    const retryRun = runtime.retryTranslationProvider('google');
    expect(runtime.getState().providerTranslations[0]).toMatchObject({ status: 'pending' });
    expect(runtime.getState().providerTranslations[0].error).toBeUndefined();
    retry.resolve(translationResult('google', 'retry'));
    await retryRun;
    expect(runtime.getState().isTranslating).toBe(true);
    other.resolve(translationResult('deeplx', 'other'));
    await run;
    expect(runtime.getState().providerTranslations.map(({ translated_text }) => translated_text))
      .toEqual(['retry', 'other']);
    expect(runtime.getState().isTranslating).toBe(false);
    expect(platform.commands.recordTranslationHistory).toHaveBeenCalledOnce();
  });

  it('ignores an older retry of the same provider', async () => {
    const older = deferred<TranslationResult>();
    const { runtime, platform } = createRuntime({ activeProviderIds: ['google'] });
    await runtime.translate({ text: 'hello', sourceLang: 'en', targetLang: 'zh-CN' });
    platform.commands.translateTextWithProvider
      .mockReturnValueOnce(older.promise)
      .mockResolvedValueOnce(translationResult('google', 'latest'));
    const olderRun = runtime.retryTranslationProvider('google');
    await runtime.retryTranslationProvider('google');
    older.resolve(translationResult('google', 'older'));
    await olderRun;
    expect(runtime.getState().providerTranslations[0].translated_text).toBe('latest');
  });

  it('does not start providers when an intent supersedes their configuration load', async () => {
    const loaded = deferred<void>();
    const { runtime, platform, providers } = createRuntime({ activeProviderIds: ['google'] });
    providers.loadTranslation.mockReturnValueOnce(loaded.promise);
    const run = runtime.translate({ text: 'old', sourceLang: 'en', targetLang: 'zh-CN' });
    runtime.updateSourceText('new');
    loaded.resolve();
    await run;
    expect(platform.commands.translateTextWithProvider).not.toHaveBeenCalled();
    expect(runtime.getState()).toMatchObject({ sourceText: 'new', isTranslating: false });
  });

  it('recovers from provider configuration loading failures', async () => {
    const { runtime, providers } = createRuntime({ activeProviderIds: ['google'] });
    providers.loadTranslation.mockRejectedValueOnce(new Error('unavailable'));
    const input = { text: 'hello', sourceLang: 'en', targetLang: 'zh-CN' };
    await expect(runtime.translate(input)).rejects.toThrow('unavailable');
    expect(runtime.getState().isTranslating).toBe(false);
    await runtime.translate(input);
    expect(runtime.getState().providerTranslations[0].status).toBe('success');
  });

  it('applies defaults and clears results when a new payload replaces a translation', async () => {
    const options = { activeProviderIds: ['google'] };
    const { runtime } = createRuntime(options);
    runtime.applyTranslationDefaults({ defaultSourceLang: 'ja', defaultTargetLang: 'en' });
    expect(runtime.getState()).toMatchObject({ sourceLang: 'ja', targetLang: 'en' });
    await runtime.translate({ text: 'old', sourceLang: 'ja', targetLang: 'en' });
    options.activeProviderIds = [];
    await runtime.applyPayload({ mode: 'translation', text: 'new', autoTranslate: true });
    await vi.waitFor(() => expect(runtime.getState()).toMatchObject({
      sourceText: 'new', providerTranslations: [], isTranslating: false,
    }));
    await runtime.applyPayload({ mode: 'ocr', text: 'recognized', autoTranslate: false, ocrIntent: 'display-text', imageBase64: 'pixels' });
    expect(runtime.getState().ocrImageBase64).toBe('pixels');
    runtime.clearOcrImage();
    expect(runtime.getState().ocrImageBase64).toBeNull();
  });
});

function createRuntime(options: {
  currentPayloadRequestId?: string | null;
  payloads?: Record<string, CaptureResultWindowPayload | null>;
  selectedImagePath?: string | null;
  recognizedFileText?: string;
  takePayloadError?: unknown;
  dismissError?: unknown;
  activeProviderIds?: string[];
  activeOcrProviderId?: string | null;
  translationResults?: Record<string, import('../../types').TranslationResult>;
  translationSettings?: import('../settings/ports').TranslationSettings;
  ocrSettings?: import('../settings/ports').OcrSettings;
  lastWindowPosition?: import('./ports').ResultWindowPhysicalPosition;
  draggedWindowPosition?: import('./ports').ResultWindowPhysicalPosition;
} = {}) {
  let payloadReadyHandler: ResultPayloadReadyHandler | null = null;
  const unsubscribe: ResultWindowUnsubscribe = vi.fn();
  const platform = {
    commands: {
      currentPayloadRequestId: vi.fn(
        async () => options.currentPayloadRequestId ?? null,
      ),
      takePayload: vi.fn(async (requestId: string) => {
        if (options.takePayloadError) throw options.takePayloadError;
        return options.payloads?.[requestId] ?? null;
      }),
      selectImageFile: vi.fn(async () => options.selectedImagePath ?? null),
      recognizeImageFile: vi.fn(async () => ({
        text: options.recognizedFileText ?? '',
        confidence: null,
        imageDataUrl: 'data:image/png;base64,aW1hZ2U=',
      })),
      translateTextWithProvider: vi.fn(async (providerId: string) => {
        return options.translationResults?.[providerId] ?? translationResult(providerId);
      }),
      recordTranslationHistory: vi.fn(async () => undefined),
      favoriteTranslationResult: vi.fn(async () => 1),
      favoriteOcrResult: vi.fn(async () => 1),
    },
    clipboard: { copyText: vi.fn() },
    onPayloadReady: vi.fn(async (handler: ResultPayloadReadyHandler) => {
      payloadReadyHandler = handler;
      return unsubscribe;
    }),
    resizeTo: vi.fn(async () => undefined),
    placeAt: vi.fn(async () => undefined),
    dismiss: vi.fn(async () => {
      if (options.dismissError) throw options.dismissError;
    }),
    beginDrag: vi.fn(async () =>
      options.draggedWindowPosition ?? { x: 0, y: 0 }),
    setAlwaysOnTop: vi.fn(async () => undefined),
  };
  const speech = { speak: vi.fn(async () => undefined) };
  const saveLastWindowPosition = vi.fn(async () => undefined);
  const providers = {
    getState: () => ({
      activeTranslationProviders: options.activeProviderIds ?? [],
      activeOcrProvider: options.activeOcrProviderId ?? null,
    }),
    loadTranslation: vi.fn(async (): Promise<void> => undefined),
    loadOcr: vi.fn(async (): Promise<void> => undefined),
  };
  const runtime = createResultWindowRuntime({
    platform,
    speech,
    providers,
    getTranslationSettings: () => options.translationSettings,
    getOcrSettings: () => options.ocrSettings,
    positionStore: {
      load: () => options.lastWindowPosition,
      save: saveLastWindowPosition,
    },
  });

  const projectedStates: ResultWindowState[] = [];
  runtime.subscribe((state) => projectedStates.push(state));
  return {
    runtime,
    platform,
    projectedStates,
    speech,
    saveLastWindowPosition,
    providers,
    unsubscribe,
    emitPayloadReady: async (requestId: string) => {
      await payloadReadyHandler?.(requestId);
    },
  };
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function translationResult(providerId = 'google', text = 'translated'): TranslationResult {
  return {
    provider_id: providerId, translated_text: text,
    detected_language: 'en', confidence: null,
  };
}

function automaticTranslationSettings(incrementalTranslation = false): TranslationSettings {
  return {
    defaultSourceLang: 'auto', defaultTargetLang: 'zh-CN',
    autoTranslate: true, autoCopy: false, preserveLineBreaks: true,
    incrementalTranslation, windowAlwaysOnTop: true, hideOnBlur: false,
  };
}
