import { create } from 'zustand';
import type { TranslationResult } from '../types';
import type { ResultWindowOrigin } from '../application/result-window/ports';
import type { SettingsConfiguration } from '../application/settings/configuration';
import type { ResultWindowStatePort } from '../application/result-window/runtime';
import type {
  ProviderTranslation,
  ResultWindowMode,
  ResultWindowProjection,
} from '../application/result-window/projection';
import { useProviderStore } from './providerStore';
import { useSettingsConfigStore } from './settingsConfigStore';

interface TranslationDefaults {
  defaultSourceLang: string;
  defaultTargetLang: string;
}

let translationSessionSequence = 0;

function nextTranslationSessionId() {
  translationSessionSequence += 1;
  return `translation-${translationSessionSequence}`;
}

function normalizeTranslatedText(text: string) {
  return text.trim();
}

function normalizeTranslationResult<T extends TranslationResult>(result: T): T {
  return {
    ...result,
    translated_text: normalizeTranslatedText(result.translated_text),
  };
}

function updateProviderTranslation(
  translations: ProviderTranslation[],
  providerId: string,
  update: Partial<ProviderTranslation>,
) {
  const next = translations.map((translation) =>
    translation.provider_id === providerId
      ? { ...translation, ...update }
      : translation,
  );

  if (next.some((translation) => translation.provider_id === providerId)) {
    return next;
  }

  return [
    ...next,
    {
      provider_id: providerId,
      status: update.status ?? 'pending',
      translated_text: update.translated_text ?? '',
      detected_language: update.detected_language ?? null,
      confidence: update.confidence ?? null,
    },
  ];
}

function hasPendingProviderTranslation(translations: ProviderTranslation[]) {
  return translations.some((translation) => translation.status === 'pending');
}

interface ResultWindowStoreState {
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  translations: TranslationResult[];
  providerTranslations: ProviderTranslation[];
  translationSessionId: string | null;
  isTranslating: boolean;
  ocrText: string;
  ocrConfidence: number | null;
  ocrImageBase64: string | null;
  isOcrRunning: boolean;
  ocrError: string | null;
  resultWindowVisible: boolean;
  resultWindowMode: ResultWindowMode;
  resultWindowOrigin: ResultWindowOrigin;
  autoTranslateRequestId: number;

  setSourceText: (text: string) => void;
  setSourceLang: (lang: string) => void;
  setTargetLang: (lang: string) => void;
  applyTranslationDefaults: (defaults: TranslationDefaults) => void;
  setTranslations: (results: TranslationResult[]) => void;
  clearTranslationResults: () => void;
  startTranslationSession: (text: string, providerIds: string[]) => string;
  beginProviderTranslation: (sessionId: string, providerId: string) => void;
  completeProviderTranslation: (
    sessionId: string,
    result: TranslationResult,
  ) => void;
  failProviderTranslation: (
    sessionId: string,
    providerId: string,
    message: string,
  ) => void;
  setTranslating: (value: boolean) => void;
  setOcrText: (text: string) => void;
  setOcrConfidence: (confidence: number | null) => void;
  setOcrImageBase64: (imageBase64: string | null) => void;
  setOcrRunning: (value: boolean) => void;
  setOcrError: (message: string | null) => void;
  requestAutoTranslate: () => void;
  setResultWindowOrigin: (origin: ResultWindowOrigin) => void;
  showResultWindow: () => void;
  showTranslationWindow: () => void;
  showOcrWindow: () => void;
  hideResultWindow: () => void;
  reset: () => void;
}

export const useResultWindowStore = create<ResultWindowStoreState>((set) => ({
  sourceText: '',
  sourceLang: 'auto',
  targetLang: 'zh-CN',
  translations: [],
  providerTranslations: [],
  translationSessionId: null,
  isTranslating: false,
  ocrText: '',
  ocrConfidence: null,
  ocrImageBase64: null,
  isOcrRunning: false,
  ocrError: null,
  resultWindowVisible: false,
  resultWindowMode: 'translation',
  resultWindowOrigin: 'input',
  autoTranslateRequestId: 0,

  setSourceText: (text) => set({ sourceText: text }),
  setSourceLang: (lang) => set({ sourceLang: lang }),
  setTargetLang: (lang) => set({ targetLang: lang }),
  applyTranslationDefaults: (defaults) =>
    set({
      sourceLang: defaults.defaultSourceLang,
      targetLang: defaults.defaultTargetLang,
    }),
  setTranslations: (results) =>
    set({
      translations: results.map(normalizeTranslationResult),
      providerTranslations: results.map((result) => ({
        ...normalizeTranslationResult(result),
        status: 'success',
      })),
    }),
  clearTranslationResults: () =>
    set({
      translations: [],
      providerTranslations: [],
      translationSessionId: null,
      isTranslating: false,
    }),
  startTranslationSession: (text, providerIds) => {
    const sessionId = nextTranslationSessionId();
    set({
      sourceText: text,
      translations: [],
      providerTranslations: providerIds.map((providerId) => ({
        provider_id: providerId,
        status: 'pending',
        translated_text: '',
        detected_language: null,
        confidence: null,
      })),
      translationSessionId: sessionId,
      isTranslating: providerIds.length > 0,
      resultWindowMode: 'translation',
    });
    return sessionId;
  },
  beginProviderTranslation: (sessionId, providerId) =>
    set((state) => {
      if (state.translationSessionId !== sessionId) return state;
      const providerTranslations = updateProviderTranslation(
        state.providerTranslations,
        providerId,
        {
          status: 'pending',
          translated_text: '',
          detected_language: null,
          confidence: null,
        },
      );
      return {
        providerTranslations,
        isTranslating: true,
      };
    }),
  completeProviderTranslation: (sessionId, result) =>
    set((state) => {
      if (state.translationSessionId !== sessionId) return state;
      const normalizedResult = normalizeTranslationResult(result);
      const providerTranslations = updateProviderTranslation(
        state.providerTranslations,
        normalizedResult.provider_id,
        {
          ...normalizedResult,
          status: 'success',
        },
      );
      return {
        providerTranslations,
        translations: providerTranslations
          .filter((translation) => translation.status === 'success')
          .map(({ status: _status, ...translation }) => translation),
        isTranslating: hasPendingProviderTranslation(providerTranslations),
      };
    }),
  failProviderTranslation: (sessionId, providerId, message) =>
    set((state) => {
      if (state.translationSessionId !== sessionId) return state;
      const providerTranslations = updateProviderTranslation(
        state.providerTranslations,
        providerId,
        {
          provider_id: providerId,
          status: 'error',
          translated_text: `Translation failed: ${message}`,
          detected_language: null,
          confidence: null,
        },
      );
      return {
        providerTranslations,
        isTranslating: hasPendingProviderTranslation(providerTranslations),
      };
    }),
  setTranslating: (value) => set({ isTranslating: value }),
  setOcrText: (text) => set({ ocrText: text }),
  setOcrConfidence: (confidence) => set({ ocrConfidence: confidence }),
  setOcrImageBase64: (imageBase64) => set({ ocrImageBase64: imageBase64 }),
  setOcrRunning: (value) => set({ isOcrRunning: value }),
  setOcrError: (message) => set({ ocrError: message }),
  requestAutoTranslate: () =>
    set((state) => ({ autoTranslateRequestId: state.autoTranslateRequestId + 1 })),
  setResultWindowOrigin: (origin) => set({ resultWindowOrigin: origin }),
  showResultWindow: () => set({ resultWindowVisible: true, resultWindowMode: 'translation' }),
  showTranslationWindow: () => set({ resultWindowVisible: true, resultWindowMode: 'translation' }),
  showOcrWindow: () => set({ resultWindowVisible: true, resultWindowMode: 'ocr' }),
  hideResultWindow: () => set({ resultWindowVisible: false }),
  reset: () => set({
    sourceText: '',
    sourceLang: 'auto',
    targetLang: 'zh-CN',
    translations: [],
    providerTranslations: [],
    translationSessionId: null,
    isTranslating: false,
    ocrText: '',
    ocrConfidence: null,
    ocrImageBase64: null,
    isOcrRunning: false,
    ocrError: null,
    autoTranslateRequestId: 0,
  }),
}));

type ProviderConfiguration = SettingsConfiguration['providers'];

export function createResultWindowStatePort(
  providers: ProviderConfiguration,
): ResultWindowStatePort {
  return {
    setSourceText: (text) => useResultWindowStore.getState().setSourceText(text),
    setSourceLang: (language) =>
      useResultWindowStore.getState().setSourceLang(language),
    setTargetLang: (language) =>
      useResultWindowStore.getState().setTargetLang(language),
    setResultWindowOrigin: (origin) =>
      useResultWindowStore.getState().setResultWindowOrigin(origin),
    clearTranslationResults: () =>
      useResultWindowStore.getState().clearTranslationResults(),
    setOcrText: (text) => useResultWindowStore.getState().setOcrText(text),
    setOcrConfidence: (confidence) =>
      useResultWindowStore.getState().setOcrConfidence(confidence),
    setOcrImageBase64: (imageBase64) =>
      useResultWindowStore.getState().setOcrImageBase64(imageBase64),
    setOcrRunning: (value) =>
      useResultWindowStore.getState().setOcrRunning(value),
    setOcrError: (message) =>
      useResultWindowStore.getState().setOcrError(message),
    requestAutoTranslate: () =>
      useResultWindowStore.getState().requestAutoTranslate(),
    showResultWindow: () => useResultWindowStore.getState().showResultWindow(),
    showOcrWindow: () => useResultWindowStore.getState().showOcrWindow(),
    hideResultWindow: () => useResultWindowStore.getState().hideResultWindow(),
    loadActiveTranslationProviderIds: async () => {
      await providers.loadTranslation();
      return providers.getState().activeTranslationProviders;
    },
    loadActiveOcrProviderId: async () => {
      if (!providers.getState().activeOcrProvider) {
        await providers.loadOcr();
      }
      return providers.getState().activeOcrProvider;
    },
    getTranslationSession: () => {
      const state = useResultWindowStore.getState();
      return {
        sessionId: state.translationSessionId,
        sourceText: state.sourceText,
        sourceLang: state.sourceLang,
        targetLang: state.targetLang,
      };
    },
    startTranslationSession: (text, providerIds) =>
      useResultWindowStore.getState().startTranslationSession(text, providerIds),
    beginProviderTranslation: (sessionId, providerId) =>
      useResultWindowStore
        .getState()
        .beginProviderTranslation(sessionId, providerId),
    completeProviderTranslation: (sessionId, result) =>
      useResultWindowStore
        .getState()
        .completeProviderTranslation(sessionId, result),
    failProviderTranslation: (sessionId, providerId, message) =>
      useResultWindowStore
        .getState()
        .failProviderTranslation(sessionId, providerId, message),
    setTranslating: (value) =>
      useResultWindowStore.getState().setTranslating(value),
  };
}

export function useResultWindowProjection(): ResultWindowProjection {
  const resultWindow = useResultWindowStore();
  const translationProviders = useProviderStore(
    (state) => state.translationProviders,
  );
  const translationSettings = useSettingsConfigStore(
    (state) => state.translation,
  );
  const ocrSettings = useSettingsConfigStore((state) => state.ocr);

  return {
    sourceText: resultWindow.sourceText,
    sourceLang: resultWindow.sourceLang,
    targetLang: resultWindow.targetLang,
    providerTranslations: resultWindow.providerTranslations,
    isTranslating: resultWindow.isTranslating,
    ocrText: resultWindow.ocrText,
    ocrConfidence: resultWindow.ocrConfidence,
    ocrImageBase64: resultWindow.ocrImageBase64,
    isOcrRunning: resultWindow.isOcrRunning,
    ocrError: resultWindow.ocrError,
    resultWindowVisible: resultWindow.resultWindowVisible,
    resultWindowMode: resultWindow.resultWindowMode,
    resultWindowOrigin: resultWindow.resultWindowOrigin,
    autoTranslateRequestId: resultWindow.autoTranslateRequestId,
    translationProviders,
    translationSettings,
    ocrSettings,
  };
}
