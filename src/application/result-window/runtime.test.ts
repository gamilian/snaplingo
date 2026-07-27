import { describe, expect, it, vi } from 'vitest';
import {
  createResultWindowRuntime,
  resultWindowStandaloneWindowHeight,
} from './runtime';
import type {
  CaptureResultWindowPayload,
  ResultPayloadReadyHandler,
  ResultWindowUnsubscribe,
} from './ports';

describe('result window application runtime', () => {
  it('hydrates the current translation payload by request ID', async () => {
    const payload: CaptureResultWindowPayload = {
      mode: 'translation',
      origin: 'selection',
      text: 'Visit https://example.\ncom',
      autoTranslate: true,
    };
    const { runtime, platform, state } = createRuntime({
      currentPayloadRequestId: '42',
      payloads: { '42': payload },
    });

    await expect(runtime.loadCurrentPayload()).resolves.toBe(true);

    expect(platform.commands.currentPayloadRequestId).toHaveBeenCalledTimes(1);
    expect(platform.commands.takePayload).toHaveBeenCalledWith('42');
    expect(state.clearTranslationResults).toHaveBeenCalledTimes(1);
    expect(state.setResultWindowOrigin).toHaveBeenCalledWith('selection');
    expect(state.setSourceText).toHaveBeenCalledWith(
      'Visit https://example.com',
    );
    expect(state.requestAutoTranslate).toHaveBeenCalledTimes(1);
    expect(state.showResultWindow).toHaveBeenCalledTimes(1);
  });

  it('does not take a payload when no current request ID exists', async () => {
    const { runtime, platform, state } = createRuntime({
      currentPayloadRequestId: null,
    });

    await expect(runtime.loadCurrentPayload()).resolves.toBe(false);

    expect(platform.commands.takePayload).not.toHaveBeenCalled();
    expect(state.showResultWindow).not.toHaveBeenCalled();
    expect(state.showOcrWindow).not.toHaveBeenCalled();
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
    const { runtime, platform, state, emitPayloadReady, unsubscribe } =
      createRuntime({
        payloads: { '7': payload },
      });

    await expect(runtime.subscribeToPayloads(onLoaded)).resolves.toBe(
      unsubscribe,
    );
    await emitPayloadReady('7');
    await Promise.resolve();

    expect(platform.commands.takePayload).toHaveBeenCalledWith('7');
    expect(state.setOcrText).toHaveBeenCalledWith('recognized');
    expect(state.setResultWindowOrigin).toHaveBeenCalledWith('ocr');
    expect(state.setOcrImageBase64).toHaveBeenCalledWith('image-base64');
    expect(state.showOcrWindow).toHaveBeenCalledTimes(1);
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
    const { runtime, platform, state } = createRuntime({
      selectedImagePath: '/tmp/example.png',
      recognizedFileText: 'file text',
    });

    await runtime.applyPayload({
      mode: 'ocr',
      text: '',
      autoTranslate: false,
      ocrIntent: 'file',
    });

    expect(state.showOcrWindow).toHaveBeenCalledTimes(1);
    expect(state.setOcrText).toHaveBeenCalledWith('');
    expect(state.setOcrImageBase64).toHaveBeenCalledWith(null);
    expect(platform.commands.selectImageFile).toHaveBeenCalledTimes(1);
    expect(platform.commands.recognizeImageFile).toHaveBeenCalledWith(
      '/tmp/example.png',
    );
    expect(state.setOcrText).toHaveBeenLastCalledWith('file text');
    expect(state.setOcrImageBase64).toHaveBeenLastCalledWith(
      'data:image/png;base64,aW1hZ2U=',
    );
    expect(state.setOcrRunning).toHaveBeenNthCalledWith(1, true);
    expect(state.setOcrRunning).toHaveBeenLastCalledWith(false);
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
    const { runtime, platform, state } = createRuntime({
      activeOcrProviderId: 'system',
    });

    await runtime.favoriteOcrResult(null, 'recognized', null);

    expect(state.loadActiveOcrProviderId).toHaveBeenCalledTimes(1);
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

  it('owns provider fan-out and records one aggregate translation history entry', async () => {
    const { runtime, platform, state } = createRuntime({
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

    expect(state.startTranslationSession).toHaveBeenCalledWith('hello', [
      'google',
      'deeplx',
    ]);
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
    const { runtime, platform, state } = createRuntime({
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

    expect(state.startTranslationSession).toHaveBeenCalledWith(
      'first\n  second',
      ['google'],
    );
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
    const { runtime, platform, state } = createRuntime({
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

    expect(state.completeProviderTranslation).toHaveBeenCalledTimes(1);
    expect(platform.commands.recordTranslationHistory).toHaveBeenCalledTimes(1);
    consoleError.mockRestore();
  });

  it('records a successful provider retry instead of bypassing history', async () => {
    const { runtime, platform, state } = createRuntime({
      translationSession: {
        sessionId: 'translation-1',
        sourceText: 'hello',
        sourceLang: 'en',
        targetLang: 'zh-CN',
      },
      translationResults: {
        google: {
          provider_id: 'google',
          translated_text: '你好',
          detected_language: 'en',
          confidence: null,
        },
      },
    });

    await runtime.retryTranslationProvider('google');

    expect(state.beginProviderTranslation).toHaveBeenCalledWith(
      'translation-1',
      'google',
    );
    expect(platform.commands.recordTranslationHistory).toHaveBeenCalledWith(
      expect.objectContaining({
        text: 'hello',
        results: [expect.objectContaining({ provider_id: 'google' })],
      }),
    );
  });

  it('closes overlay state locally and standalone state plus native window', async () => {
    const { runtime, platform, state } = createRuntime();

    await runtime.close('overlay');
    await runtime.close('standalone');

    expect(state.hideResultWindow).toHaveBeenCalledTimes(2);
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
  });

  it('keeps standalone close state hidden when native hide fails', async () => {
    const { runtime, state } = createRuntime({
      dismissError: new Error('window unavailable'),
    });

    await expect(runtime.close('standalone')).resolves.toBeUndefined();

    expect(state.hideResultWindow).toHaveBeenCalledTimes(1);
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
  translationSession?: {
    sessionId: string | null;
    sourceText: string;
    sourceLang: string;
    targetLang: string;
  };
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
        const result = options.translationResults?.[providerId];
        if (!result) throw new Error(`missing result for ${providerId}`);
        return result;
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
  const state = {
    setSourceText: vi.fn(),
    setResultWindowOrigin: vi.fn(),
    clearTranslationResults: vi.fn(),
    setOcrText: vi.fn(),
    setOcrConfidence: vi.fn(),
    setOcrImageBase64: vi.fn(),
    setOcrRunning: vi.fn(),
    setOcrError: vi.fn(),
    requestAutoTranslate: vi.fn(),
    showResultWindow: vi.fn(),
    showOcrWindow: vi.fn(),
    hideResultWindow: vi.fn(),
    loadActiveTranslationProviderIds: vi.fn(
      async () => options.activeProviderIds ?? [],
    ),
    loadActiveOcrProviderId: vi.fn(
      async () => options.activeOcrProviderId ?? null,
    ),
    getTranslationSession: vi.fn(() =>
      options.translationSession ?? {
        sessionId: null,
        sourceText: '',
        sourceLang: 'auto',
        targetLang: 'zh-CN',
      },
    ),
    startTranslationSession: vi.fn(() => 'translation-1'),
    beginProviderTranslation: vi.fn(),
    completeProviderTranslation: vi.fn(),
    failProviderTranslation: vi.fn(),
    setTranslating: vi.fn(),
  };
  const runtime = createResultWindowRuntime({
    platform,
    speech,
    state,
    getTranslationSettings: () => options.translationSettings,
    getOcrSettings: () => options.ocrSettings,
    positionStore: {
      load: () => options.lastWindowPosition,
      save: saveLastWindowPosition,
    },
  });

  return {
    runtime,
    platform,
    speech,
    saveLastWindowPosition,
    state,
    unsubscribe,
    emitPayloadReady: async (requestId: string) => {
      await payloadReadyHandler?.(requestId);
    },
  };
}
