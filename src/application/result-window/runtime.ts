import type { TranslationError, TranslationResult } from '../../types';
import type {
  OcrSettings,
  ResultWindowPosition,
  TranslationSettings,
} from '../settings/ports';
import {
  defaultTargetLanguageForSource,
  resolveTranslationRequestLanguages,
  swapTranslationLanguagePair,
} from '../translation/languages';
import type { ResultWindowPlatformRuntime } from './platformRuntime';
import type {
  CaptureResultWindowPayload,
  ResultWindowOrigin,
  ResultWindowPositionStore,
  ResultWindowSpeechPort,
  ResultWindowUnsubscribe,
} from './ports';
import { runOcrFileWorkflow } from './fileOcrWorkflow';
import { applyOcrTextPreferences } from '../../utils/ocrTextProcessing';
import {
  ocrPayloadDisplayText,
  shouldApplyOcrPayloadText,
  shouldApplyTranslationPayloadText,
  shouldClearOcrResultsForPayload,
  shouldClearTranslationResultsForPayload,
  shouldStartFileOcrForPayload,
  translationPayloadSourceText,
} from './payload';
import { speakResultWindowText } from './speech';

export type ResultWindowPresentation = 'overlay' | 'standalone';

export interface ResultWindowStatePort {
  setSourceText(text: string): void;
  setSourceLang(language: string): void;
  setTargetLang(language: string): void;
  setResultWindowOrigin(origin: ResultWindowOrigin): void;
  clearTranslationResults(): void;
  setOcrText(text: string): void;
  setOcrConfidence(confidence: number | null): void;
  setOcrImageBase64(imageBase64: string | null): void;
  setOcrRunning(value: boolean): void;
  setOcrError(message: string | null): void;
  requestAutoTranslate(): void;
  showResultWindow(): void;
  showOcrWindow(): void;
  hideResultWindow(): void;
  loadActiveTranslationProviderIds(): Promise<string[]>;
  loadActiveOcrProviderId(): Promise<string | null>;
  getTranslationSession(): {
    sessionId: string | null;
    sourceText: string;
    sourceLang: string;
    targetLang: string;
  };
  startTranslationSession(text: string, providerIds: string[]): string;
  beginProviderTranslation(sessionId: string, providerId: string): void;
  completeProviderTranslation(sessionId: string, result: TranslationResult): void;
  failProviderTranslation(
    sessionId: string,
    result: TranslationResult,
  ): void;
  setTranslating(value: boolean): void;
}

export interface ResultWindowRuntimePorts {
  platform: ResultWindowPlatformRuntime;
  speech: ResultWindowSpeechPort;
  state: ResultWindowStatePort;
  getTranslationSettings?: () => TranslationSettings | undefined;
  getOcrSettings?: () => OcrSettings | undefined;
  positionStore: ResultWindowPositionStore;
}

export interface ResultWindowResizeInput {
  presentation: ResultWindowPresentation;
  visible: boolean;
  mode: 'translation' | 'ocr';
  origin?: ResultWindowOrigin;
  panelHeightPx: number;
}

export interface ResultWindowTranslationFavoriteInput {
  text: string;
  sourceLang: string;
  targetLang: string;
  result: TranslationResult;
}

export interface ResultWindowTranslationFavoritesInput {
  text: string;
  sourceLang: string;
  targetLang: string;
  results: TranslationResult[];
}

const resultWindowStandaloneContainerPaddingPx = 16;
const resultWindowStandaloneWidthPx = 660;

export function resultWindowStandaloneWindowHeight(panelHeightPx: number) {
  return panelHeightPx + resultWindowStandaloneContainerPaddingPx;
}

export function createResultWindowRuntime({
  platform,
  speech,
  state,
  getTranslationSettings,
  getOcrSettings,
  positionStore,
}: ResultWindowRuntimePorts) {
  let needsPlacement = true;
  let lastPlacedPosition: ResultWindowPosition | null = null;
  let translationGeneration = 0;

  function invalidateTranslationGeneration() {
    translationGeneration += 1;
    state.setTranslating(false);
  }

  function translationRequestText(text: string) {
    return getTranslationSettings?.()?.preserveLineBreaks === false
      ? text.replace(/\s+/g, ' ').trim()
      : text;
  }

  function changeSourceLanguage(language: string) {
    invalidateTranslationGeneration();
    const { targetLang } = state.getTranslationSession();
    state.setSourceLang(language);
    state.setTargetLang(
      targetLang === 'auto'
        ? 'auto'
        : defaultTargetLanguageForSource(language),
    );
  }

  function swapTranslationLanguages() {
    invalidateTranslationGeneration();
    const session = state.getTranslationSession();
    const next = swapTranslationLanguagePair(
      session.sourceLang,
      session.targetLang,
    );
    state.setSourceLang(next.sourceLang);
    state.setTargetLang(next.targetLang);
  }

  async function persistTranslationHistory(input: {
    text: string;
    sourceLang: string;
    targetLang: string;
    results: TranslationResult[];
    startedAt: number;
  }) {
    try {
      await platform.commands.recordTranslationHistory({
        text: input.text,
        sourceLang: input.sourceLang,
        targetLang: input.targetLang,
        results: input.results,
        durationMs: Math.max(0, Math.round(performance.now() - input.startedAt)),
      });
    } catch (error) {
      console.error('Failed to record translation history:', error);
    }
  }

  async function translate(input: {
    text: string;
    sourceLang: string;
    targetLang: string;
  }) {
    if (!input.text.trim()) return;

    const generation = ++translationGeneration;

    const settings = getTranslationSettings?.();
    const requestText = translationRequestText(input.text);

    const request = resolveTranslationRequestLanguages(
      requestText,
      input.sourceLang,
      input.targetLang,
    );
    const providerIds = await state.loadActiveTranslationProviderIds();
    if (generation !== translationGeneration) return;
    const sessionId = state.startTranslationSession(input.text, providerIds);

    if (providerIds.length === 0) {
      state.setTranslating(false);
      return;
    }

    const startedAt = performance.now();
    const results = await Promise.all(
      providerIds.map(async (providerId) => {
        state.beginProviderTranslation(sessionId, providerId);
        try {
          const result = await platform.commands.translateTextWithProvider(
            providerId,
            {
              text: requestText,
              sourceLang: request.sourceLang,
              targetLang: request.targetLang,
            },
          );
          if (generation !== translationGeneration) return null;
          if (result.error) {
            state.failProviderTranslation(sessionId, result);
            return null;
          }
          state.completeProviderTranslation(sessionId, result);
          return result;
        } catch (error) {
          if (generation === translationGeneration) {
            state.failProviderTranslation(
              sessionId,
              translationFailure(providerId, error),
            );
          }
          return null;
        }
      }),
    );
    const completedResults = results.filter(
      (result): result is TranslationResult => result !== null,
    );

    if (generation !== translationGeneration || completedResults.length === 0) {
      return;
    }

    await persistTranslationHistory({
      text: input.text,
      sourceLang: request.sourceLang,
      targetLang: request.targetLang,
      results: completedResults,
      startedAt,
    });
    if (generation !== translationGeneration) return;
    if (settings?.autoCopy) {
      try {
        await platform.clipboard.copyText(completedResults[0].translated_text);
      } catch (error) {
        console.error('Failed to auto-copy translation result:', error);
      }
    }
  }

  async function retryTranslationProvider(providerId: string) {
    const session = state.getTranslationSession();
    if (!session.sessionId || !session.sourceText.trim()) return;

    const generation = ++translationGeneration;
    const sessionId = session.sessionId;

    const request = resolveTranslationRequestLanguages(
      translationRequestText(session.sourceText),
      session.sourceLang,
      session.targetLang,
    );
    state.beginProviderTranslation(sessionId, providerId);

    try {
      const result = await platform.commands.translateTextWithProvider(
        providerId,
        {
          text: translationRequestText(session.sourceText),
          sourceLang: request.sourceLang,
          targetLang: request.targetLang,
        },
      );
      if (generation !== translationGeneration) return;
      if (result.error) {
        state.failProviderTranslation(sessionId, result);
        return;
      }
      state.completeProviderTranslation(sessionId, result);
    } catch (error) {
      if (generation === translationGeneration) {
            state.failProviderTranslation(
              sessionId,
              translationFailure(providerId, error),
            );
      }
    }
  }

  async function applyPayload(payload: CaptureResultWindowPayload) {
    invalidateTranslationGeneration();
    state.setResultWindowOrigin(
      payload.origin ?? (payload.mode === 'ocr' ? 'ocr' : 'input'),
    );
    needsPlacement = true;
    if (payload.mode === 'translation') {
      if (shouldClearTranslationResultsForPayload(payload)) {
        state.clearTranslationResults();
      }
      if (shouldApplyTranslationPayloadText(payload)) {
        state.setSourceText(
          translationPayloadSourceText(payload, getOcrSettings?.()),
        );
      }
      if (payload.autoTranslate) {
        state.requestAutoTranslate();
      }
      state.showResultWindow();
      return;
    }

    if (shouldClearOcrResultsForPayload(payload)) {
      state.setOcrText('');
      state.setOcrConfidence(null);
      state.setOcrImageBase64(null);
      state.setOcrError(null);
    }
    if (shouldApplyOcrPayloadText(payload)) {
      state.setOcrText(ocrPayloadDisplayText(payload, getOcrSettings?.()));
      state.setOcrConfidence(payload.confidence ?? null);
      state.setOcrImageBase64(payload.imageBase64 ?? null);
    }
    if (shouldStartFileOcrForPayload(payload)) {
      await startFileOcr();
      return;
    }
    state.showOcrWindow();
  }

  async function loadPayload(requestId: string) {
    const payload = await platform.commands.takePayload(requestId);
    if (!payload) return false;

    await applyPayload(payload);
    return true;
  }

  async function loadCurrentPayload() {
    const requestId = await platform.commands.currentPayloadRequestId();
    if (!requestId) return false;

    return loadPayload(requestId);
  }

  async function subscribeToPayloads(
    onLoaded?: () => void,
  ): Promise<ResultWindowUnsubscribe> {
    return platform.onPayloadReady((requestId) => {
      void loadPayload(requestId)
        .then((loaded) => {
          if (loaded) {
            onLoaded?.();
          }
        })
        .catch((err) => {
          console.error('Failed to reload result window payload:', err);
        });
    });
  }

  async function startFileOcr() {
    const settings = getOcrSettings?.();
    state.showOcrWindow();
    state.setOcrText('');
    state.setOcrConfidence(null);
    state.setOcrImageBase64(null);
    state.setOcrError(null);
    await runOcrFileWorkflow({
      selectImageFile: platform.commands.selectImageFile,
      recognizeImageFile: platform.commands.recognizeImageFile,
      language:
        settings?.recognitionLanguage === 'auto'
          ? undefined
          : settings?.recognitionLanguage,
      transformText: (text) =>
        settings ? applyOcrTextPreferences(text, settings) : text,
      setText: state.setOcrText,
      setConfidence: state.setOcrConfidence,
      setImageDataUrl: state.setOcrImageBase64,
      setRunning: state.setOcrRunning,
      setError: state.setOcrError,
    });
  }

  async function favoriteOcrResult(
    imageBase64: string | null,
    text: string,
    confidence: number | null,
  ) {
    const recognitionLanguage = getOcrSettings?.()?.recognitionLanguage;
    const providerUsed = await state.loadActiveOcrProviderId();
    return platform.commands.favoriteOcrResult({
      imageData: imageBase64 ? base64ToBytes(imageBase64) : [],
      result: { text, confidence },
      language:
        recognitionLanguage === 'auto' ? undefined : recognitionLanguage,
      providerUsed: providerUsed ?? 'manual',
    });
  }

  function favoriteTranslationResult(
    input: ResultWindowTranslationFavoriteInput,
  ) {
    const { targetLang } = resolveTranslationRequestLanguages(
      input.text,
      input.sourceLang,
      input.targetLang,
    );
    return platform.commands.favoriteTranslationResult({
      text: input.text,
      sourceLang: input.sourceLang,
      targetLang,
      result: input.result,
    });
  }

  function favoriteTranslationResults(
    input: ResultWindowTranslationFavoritesInput,
  ) {
    return Promise.all(
      input.results.map((result) =>
        favoriteTranslationResult({
          text: input.text,
          sourceLang: input.sourceLang,
          targetLang: input.targetLang,
          result,
        }),
      ),
    );
  }

  function copyText(text: string) {
    return platform.clipboard.copyText(text);
  }

  async function close(presentation: ResultWindowPresentation) {
    invalidateTranslationGeneration();
    state.hideResultWindow();
    needsPlacement = true;
    if (presentation === 'standalone') {
      try {
        await platform.dismiss();
      } catch (err) {
        console.error('Failed to hide result window:', err);
      }
    }
  }

  async function resizeStandaloneWindow({
    presentation,
    visible,
    mode,
    origin,
    panelHeightPx,
  }: ResultWindowResizeInput) {
    if (presentation !== 'standalone' || !visible) return;

    await platform.resizeTo(
      Math.min(
        1000,
        Math.max(
          300,
          mode === 'translation'
            ? getTranslationSettings?.()?.windowWidth ??
                resultWindowStandaloneWidthPx
            : resultWindowStandaloneWidthPx,
        ),
      ),
      resultWindowStandaloneWindowHeight(panelHeightPx),
    );
    const position =
      mode === 'ocr'
        ? getOcrSettings?.()?.windowPosition ?? 'center'
        : origin === 'input'
          ? getTranslationSettings?.()?.inputWindowPosition ?? 'center'
          : getTranslationSettings?.()?.selectionWindowPosition ?? 'below-cursor';
    if (needsPlacement || position !== lastPlacedPosition) {
      if (position === 'last-position') {
        await platform.placeAt(position, positionStore.load());
      } else {
        await platform.placeAt(position);
      }
      needsPlacement = false;
      lastPlacedPosition = position;
    }
  }

  async function beginDrag() {
    const position = await platform.beginDrag();
    try {
      await positionStore.save(position);
    } catch (error) {
      console.error('Failed to save result window position:', error);
    }
  }

  async function speakText(text: string, languageCode?: string) {
    await speakResultWindowText(speech, text, languageCode);
  }

  return {
    updateSourceText: (text: string) => {
      invalidateTranslationGeneration();
      state.setSourceText(text);
    },
    changeSourceLanguage,
    changeTargetLanguage: (language: string) => {
      invalidateTranslationGeneration();
      state.setTargetLang(language);
    },
    swapTranslationLanguages,
    updateOcrText: state.setOcrText,
    clearOcrImage: () => state.setOcrImageBase64(null),
    loadTranslationProviders: () =>
      state.loadActiveTranslationProviderIds().then(() => undefined),
    loadCurrentPayload,
    loadPayload,
    applyPayload,
    subscribeToPayloads,
    startFileOcr,
    favoriteTranslationResult,
    favoriteTranslationResults,
    favoriteOcrResult,
    copyText,
    speakText,
    translate,
    retryTranslationProvider,
    close,
    resizeStandaloneWindow,
    dismiss: platform.dismiss,
    beginDrag,
    setAlwaysOnTop: platform.setAlwaysOnTop,
  };
}

export type ResultWindowRuntime = ReturnType<typeof createResultWindowRuntime>;

function base64ToBytes(base64: string) {
  const payload = base64.includes(',') ? base64.split(',').pop() ?? '' : base64;
  const binary = globalThis.atob(payload);
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function translationFailure(providerId: string, error: unknown): TranslationResult {
  const message = errorMessage(error);
  const failure: TranslationError = {
    code: 'request_failed',
    message,
    retryable: true,
  };
  return {
    provider_id: providerId,
    translated_text: '',
    detected_language: null,
    confidence: null,
    error: failure,
  };
}
