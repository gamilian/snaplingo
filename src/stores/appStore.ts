import { create } from 'zustand';
import { TranslationResult } from '../types';

export type ResultWindowMode = 'translation' | 'ocr';
export type ProviderTranslationStatus = 'pending' | 'success' | 'error';

export interface ProviderTranslation extends TranslationResult {
  status: ProviderTranslationStatus;
}

let translationSessionSequence = 0;

function nextTranslationSessionId() {
  translationSessionSequence += 1;
  return `translation-${translationSessionSequence}`;
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

interface AppState {
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  translations: TranslationResult[];
  providerTranslations: ProviderTranslation[];
  translationSessionId: string | null;
  isTranslating: boolean;
  ocrText: string;
  ocrImageBase64: string | null;
  isOcrRunning: boolean;
  ocrError: string | null;
  resultWindowVisible: boolean;
  resultWindowMode: ResultWindowMode;
  autoTranslateRequestId: number;

  setSourceText: (text: string) => void;
  setSourceLang: (lang: string) => void;
  setTargetLang: (lang: string) => void;
  setTranslations: (results: TranslationResult[]) => void;
  clearTranslationResults: () => void;
  startTranslationSession: (text: string, providerIds: string[]) => string;
  beginProviderTranslation: (sessionId: string, providerId: string) => void;
  completeProviderTranslation: (sessionId: string, result: TranslationResult) => void;
  failProviderTranslation: (
    sessionId: string,
    providerId: string,
    message: string,
  ) => void;
  setTranslating: (value: boolean) => void;
  setOcrText: (text: string) => void;
  setOcrImageBase64: (imageBase64: string | null) => void;
  setOcrRunning: (value: boolean) => void;
  setOcrError: (message: string | null) => void;
  requestAutoTranslate: () => void;
  showResultWindow: () => void;
  showTranslationWindow: () => void;
  showOcrWindow: () => void;
  hideResultWindow: () => void;
  reset: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  sourceText: '',
  sourceLang: 'auto',
  targetLang: 'zh-CN',
  translations: [],
  providerTranslations: [],
  translationSessionId: null,
  isTranslating: false,
  ocrText: '',
  ocrImageBase64: null,
  isOcrRunning: false,
  ocrError: null,
  resultWindowVisible: false,
  resultWindowMode: 'translation',
  autoTranslateRequestId: 0,

  setSourceText: (text) => set({ sourceText: text }),
  setSourceLang: (lang) => set({ sourceLang: lang }),
  setTargetLang: (lang) => set({ targetLang: lang }),
  setTranslations: (results) =>
    set({
      translations: results,
      providerTranslations: results.map((result) => ({
        ...result,
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
      const providerTranslations = updateProviderTranslation(
        state.providerTranslations,
        result.provider_id,
        {
          ...result,
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
  setOcrImageBase64: (imageBase64) => set({ ocrImageBase64: imageBase64 }),
  setOcrRunning: (value) => set({ isOcrRunning: value }),
  setOcrError: (message) => set({ ocrError: message }),
  requestAutoTranslate: () =>
    set((state) => ({ autoTranslateRequestId: state.autoTranslateRequestId + 1 })),
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
    ocrImageBase64: null,
    isOcrRunning: false,
    ocrError: null,
    autoTranslateRequestId: 0,
  }),
}));
