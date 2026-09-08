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
  ResultWindowProvidersPort,
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
import {
  createInitialResultWindowState,
  type ProviderTranslation,
  type ResultWindowState,
} from './projection';

export type ResultWindowPresentation = 'overlay' | 'standalone';

export interface ResultWindowRuntimePorts {
  platform: ResultWindowPlatformRuntime;
  speech: ResultWindowSpeechPort;
  providers: ResultWindowProvidersPort;
  getTranslationSettings?: () => TranslationSettings | undefined;
  getOcrSettings?: () => OcrSettings | undefined;
  positionStore: ResultWindowPositionStore;
}

interface TranslationInput {
  text: string;
  sourceLang: string;
  targetLang: string;
}

interface TranslationSession {
  request: TranslationInput;
  providerRequests: Map<string, object>;
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
  providers,
  getTranslationSettings,
  getOcrSettings,
  positionStore,
}: ResultWindowRuntimePorts) {
  let needsPlacement = true;
  let lastPlacedPosition: ResultWindowPosition | null = null;
  let state = createInitialResultWindowState();
  const listeners = new Set<(state: ResultWindowState) => void>();
  let translationSession: TranslationSession | null = null;
  let automaticTranslationTimer: ReturnType<typeof setTimeout> | null = null;
  let lastTranslationKey: string | null = null;
  let ocrGeneration = 0;

  function updateState(update: Partial<ResultWindowState>) {
    state = { ...state, ...update };
    listeners.forEach((listener) => listener(state));
  }

  function cancelAutomaticTranslation() {
    if (automaticTranslationTimer !== null) {
      clearTimeout(automaticTranslationTimer);
      automaticTranslationTimer = null;
    }
  }

  function invalidateTranslationSession() {
    cancelAutomaticTranslation();
    translationSession = null;
    if (state.isTranslating) lastTranslationKey = null;
    updateState({ isTranslating: false });
  }

  function invalidateOcrGeneration() {
    ocrGeneration += 1;
    updateState({ isOcrRunning: false });
  }

  function currentTranslationInput(): TranslationInput {
    return {
      text: state.sourceText,
      sourceLang: state.sourceLang,
      targetLang: state.targetLang,
    };
  }

  function translationKey(input: TranslationInput) {
    return `${input.sourceLang}\u0000${input.targetLang}\u0000${input.text}`;
  }

  function scheduleAutomaticTranslation(immediate = false) {
    cancelAutomaticTranslation();
    const settings = getTranslationSettings?.();
    if (
      !state.resultWindowVisible ||
      state.resultWindowMode !== 'translation' ||
      !state.sourceText.trim() ||
      state.isTranslating ||
      (!immediate && !settings?.autoTranslate)
    ) {
      return;
    }

    const input = currentTranslationInput();
    if (!immediate && translationKey(input) === lastTranslationKey) return;

    const run = () => {
      automaticTranslationTimer = null;
      void translate(input).catch((error) => {
        console.error('Failed to start automatic translation:', error);
      });
    };
    if (immediate) {
      run();
    } else {
      automaticTranslationTimer = setTimeout(
        run,
        settings?.incrementalTranslation ? 150 : 500,
      );
    }
  }

  async function loadTranslationProviders() {
    await providers.loadTranslation();
    return providers.getState().activeTranslationProviders;
  }

  function translationRequestText(text: string) {
    return getTranslationSettings?.()?.preserveLineBreaks === false
      ? text.replace(/\s+/g, ' ').trim()
      : text;
  }

  function createTranslationSession(input: TranslationInput): TranslationSession {
    const text = translationRequestText(input.text);
    return {
      request: {
        text,
        ...resolveTranslationRequestLanguages(text, input.sourceLang, input.targetLang),
      },
      providerRequests: new Map(),
    };
  }

  function changeSourceLanguage(language: string) {
    invalidateTranslationSession();
    const { targetLang } = state;
    updateState({
      sourceLang: language,
      targetLang: targetLang === 'auto'
        ? 'auto'
        : defaultTargetLanguageForSource(language),
    });
    scheduleAutomaticTranslation();
  }

  function swapTranslationLanguages() {
    invalidateTranslationSession();
    const next = swapTranslationLanguagePair(
      state.sourceLang,
      state.targetLang,
    );
    updateState(next);
    scheduleAutomaticTranslation();
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

  function updateProviderTranslation(result: ProviderTranslation) {
    const providerTranslations = state.providerTranslations.map((entry) =>
      entry.provider_id === result.provider_id ? result : entry,
    );
    updateState({
      providerTranslations,
      isTranslating: providerTranslations.some((entry) => entry.status === 'pending'),
    });
  }

  async function requestProviderTranslation(
    session: TranslationSession,
    providerId: string,
  ): Promise<TranslationResult | null> {
    const requestToken = {};
    session.providerRequests.set(providerId, requestToken);
    updateProviderTranslation(pendingTranslation(providerId));
    let result: TranslationResult;
    try {
      result = await platform.commands.translateTextWithProvider(
        providerId,
        session.request,
      );
    } catch (error) {
      result = translationFailure(providerId, error);
    }
    if (
      translationSession !== session ||
      session.providerRequests.get(providerId) !== requestToken
    ) {
      return null;
    }

    if (result.error) {
      updateProviderTranslation({
        ...result,
        provider_id: providerId,
        status: 'error',
        translated_text: '',
        detected_language: null,
        confidence: null,
        request_id: result.request_id ?? null,
        duration_ms: result.duration_ms ?? null,
      });
      return null;
    }
    updateProviderTranslation({
      ...result,
      provider_id: providerId,
      translated_text: result.translated_text.trim(),
      status: 'success',
    });
    return result;
  }

  async function translate(input: TranslationInput) {
    cancelAutomaticTranslation();
    if (!input.text.trim()) return;

    lastTranslationKey = translationKey(input);
    const settings = getTranslationSettings?.();
    const session = createTranslationSession(input);
    translationSession = session;
    updateState({
      sourceText: input.text,
      sourceLang: input.sourceLang,
      targetLang: input.targetLang,
      resultWindowMode: 'translation',
      providerTranslations: [],
      isTranslating: true,
    });
    let providerIds: string[];
    try {
      providerIds = await loadTranslationProviders();
    } catch (error) {
      if (translationSession === session) {
        translationSession = null;
        lastTranslationKey = null;
        updateState({ isTranslating: false });
      }
      throw error;
    }
    if (translationSession !== session) return;
    updateState({
      providerTranslations: providerIds.map(pendingTranslation),
      isTranslating: providerIds.length > 0,
    });
    if (providerIds.length === 0) return;

    const startedAt = performance.now();
    const results = await Promise.all(
      providerIds.map((providerId) => requestProviderTranslation(session, providerId)),
    );
    const completedResults = results.filter(
      (result): result is TranslationResult => result !== null,
    );

    if (translationSession !== session || completedResults.length === 0) {
      return;
    }

    await persistTranslationHistory({
      text: input.text,
      sourceLang: session.request.sourceLang,
      targetLang: session.request.targetLang,
      results: completedResults,
      startedAt,
    });
    if (translationSession !== session) return;
    if (settings?.autoCopy) {
      try {
        await platform.clipboard.copyText(completedResults[0].translated_text);
      } catch (error) {
        console.error('Failed to auto-copy translation result:', error);
      }
    }
  }

  async function retryTranslationProvider(providerId: string) {
    const session = translationSession;
    if (
      !session ||
      !state.providerTranslations.some((entry) => entry.provider_id === providerId)
    ) {
      return;
    }
    await requestProviderTranslation(session, providerId);
  }

  async function applyPayload(payload: CaptureResultWindowPayload) {
    invalidateTranslationSession();
    invalidateOcrGeneration();
    updateState({
      resultWindowOrigin: payload.origin ?? (payload.mode === 'ocr' ? 'ocr' : 'input'),
    });
    needsPlacement = true;
    if (payload.mode === 'translation') {
      if (shouldClearTranslationResultsForPayload(payload)) {
        lastTranslationKey = null;
        updateState({ providerTranslations: [] });
      }
      if (shouldApplyTranslationPayloadText(payload)) {
        updateState({
          sourceText: translationPayloadSourceText(payload, getOcrSettings?.()),
          sourceLang: payload.detectedLanguage || state.sourceLang,
        });
      }
      updateState({ resultWindowVisible: true, resultWindowMode: 'translation' });
      if (state.sourceText.trim() && state.providerTranslations.length > 0) {
        translationSession = createTranslationSession(currentTranslationInput());
      }
      scheduleAutomaticTranslation(payload.autoTranslate);
      return;
    }

    if (shouldClearOcrResultsForPayload(payload)) {
      updateState({
        ocrText: '',
        ocrConfidence: null,
        ocrImageBase64: null,
        ocrError: null,
      });
    }
    if (shouldApplyOcrPayloadText(payload)) {
      updateState({
        ocrText: ocrPayloadDisplayText(payload, getOcrSettings?.()),
        ocrConfidence: payload.confidence ?? null,
        ocrImageBase64: payload.imageBase64 ?? null,
      });
    }
    if (shouldStartFileOcrForPayload(payload)) {
      await startFileOcr();
      return;
    }
    updateState({ resultWindowVisible: true, resultWindowMode: 'ocr' });
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
    invalidateTranslationSession();
    const generation = ++ocrGeneration;
    const settings = getOcrSettings?.();
    updateState({
      resultWindowVisible: true,
      resultWindowMode: 'ocr',
      ocrText: '',
      ocrConfidence: null,
      ocrImageBase64: null,
      ocrError: null,
    });
    await runOcrFileWorkflow({
      selectImageFile: platform.commands.selectImageFile,
      recognizeImageFile: platform.commands.recognizeImageFile,
      language:
        settings?.recognitionLanguage === 'auto'
          ? undefined
          : settings?.recognitionLanguage,
      transformText: (text) =>
        settings ? applyOcrTextPreferences(text, settings) : text,
      isCurrent: () => generation === ocrGeneration,
      setText: (ocrText) => updateState({ ocrText }),
      setConfidence: (ocrConfidence) => updateState({ ocrConfidence }),
      setImageDataUrl: (ocrImageBase64) => updateState({ ocrImageBase64 }),
      setRunning: (isOcrRunning) => updateState({ isOcrRunning }),
      setError: (ocrError) => updateState({ ocrError }),
    });
  }

  async function favoriteOcrResult(
    imageBase64: string | null,
    text: string,
    confidence: number | null,
  ) {
    const recognitionLanguage = getOcrSettings?.()?.recognitionLanguage;
    if (!providers.getState().activeOcrProvider) await providers.loadOcr();
    const providerUsed = providers.getState().activeOcrProvider;
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
    invalidateTranslationSession();
    invalidateOcrGeneration();
    updateState({ resultWindowVisible: false });
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
    getState: () => state,
    subscribe: (listener: (state: ResultWindowState) => void) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    applyTranslationDefaults: (defaults: Pick<
      TranslationSettings,
      'defaultSourceLang' | 'defaultTargetLang'
    >) => {
      if (
        state.sourceLang !== defaults.defaultSourceLang ||
        state.targetLang !== defaults.defaultTargetLang
      ) {
        invalidateTranslationSession();
        updateState({
          sourceLang: defaults.defaultSourceLang,
          targetLang: defaults.defaultTargetLang,
        });
      }
      scheduleAutomaticTranslation();
    },
    updateSourceText: (text: string) => {
      if (text === state.sourceText) return;
      invalidateTranslationSession();
      updateState({ sourceText: text });
      scheduleAutomaticTranslation();
    },
    changeSourceLanguage,
    changeTargetLanguage: (language: string) => {
      if (language === state.targetLang) return;
      invalidateTranslationSession();
      updateState({ targetLang: language });
      scheduleAutomaticTranslation();
    },
    swapTranslationLanguages,
    updateOcrText: (text: string) => {
      invalidateOcrGeneration();
      updateState({ ocrText: text });
    },
    clearOcrImage: () => updateState({ ocrImageBase64: null }),
    loadTranslationProviders,
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

function pendingTranslation(providerId: string): ProviderTranslation {
  return {
    provider_id: providerId,
    status: 'pending',
    translated_text: '',
    detected_language: null,
    confidence: null,
  };
}

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
