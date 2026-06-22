import { create } from 'zustand';
import { TranslationResult } from '../types';

export type ResultWindowMode = 'translation' | 'ocr';

interface AppState {
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  translations: TranslationResult[];
  isTranslating: boolean;
  ocrText: string;
  isOcrRunning: boolean;
  ocrError: string | null;
  resultWindowVisible: boolean;
  resultWindowMode: ResultWindowMode;
  pendingAutoTranslate: boolean;

  setSourceText: (text: string) => void;
  setSourceLang: (lang: string) => void;
  setTargetLang: (lang: string) => void;
  setTranslations: (results: TranslationResult[]) => void;
  setTranslating: (value: boolean) => void;
  setOcrText: (text: string) => void;
  setOcrRunning: (value: boolean) => void;
  setOcrError: (message: string | null) => void;
  requestAutoTranslate: () => void;
  consumeAutoTranslateRequest: () => void;
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
  isTranslating: false,
  ocrText: '',
  isOcrRunning: false,
  ocrError: null,
  resultWindowVisible: false,
  resultWindowMode: 'translation',
  pendingAutoTranslate: false,

  setSourceText: (text) => set({ sourceText: text }),
  setSourceLang: (lang) => set({ sourceLang: lang }),
  setTargetLang: (lang) => set({ targetLang: lang }),
  setTranslations: (results) => set({ translations: results }),
  setTranslating: (value) => set({ isTranslating: value }),
  setOcrText: (text) => set({ ocrText: text }),
  setOcrRunning: (value) => set({ isOcrRunning: value }),
  setOcrError: (message) => set({ ocrError: message }),
  requestAutoTranslate: () => set({ pendingAutoTranslate: true }),
  consumeAutoTranslateRequest: () => set({ pendingAutoTranslate: false }),
  showResultWindow: () => set({ resultWindowVisible: true, resultWindowMode: 'translation' }),
  showTranslationWindow: () => set({ resultWindowVisible: true, resultWindowMode: 'translation' }),
  showOcrWindow: () => set({ resultWindowVisible: true, resultWindowMode: 'ocr' }),
  hideResultWindow: () => set({ resultWindowVisible: false }),
  reset: () => set({
    sourceText: '',
    sourceLang: 'auto',
    targetLang: 'zh-CN',
    translations: [],
    isTranslating: false,
    ocrText: '',
    isOcrRunning: false,
    ocrError: null,
    pendingAutoTranslate: false,
  }),
}));
