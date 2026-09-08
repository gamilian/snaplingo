import { create } from 'zustand';
import type { ResultWindowRuntime } from '../application/result-window/runtime';
import {
  createInitialResultWindowState,
  type ResultWindowState,
  type ResultWindowProjection,
} from '../application/result-window/projection';
import { useProviderStore } from './providerStore';
import { useSettingsConfigStore } from './settingsConfigStore';

export const useResultWindowStore = create<ResultWindowState>(
  createInitialResultWindowState,
);

let unsubscribe: (() => void) | null = null;

export function initializeResultWindowStore(
  runtime: Pick<ResultWindowRuntime, 'getState' | 'subscribe'>,
) {
  unsubscribe?.();
  useResultWindowStore.setState(runtime.getState(), true);
  unsubscribe = runtime.subscribe((state) =>
    useResultWindowStore.setState(state, true),
  );
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
    translationProviders,
    translationSettings,
    ocrSettings,
  };
}
