import { useAppStore } from '../stores/appStore';
import { useProviderStore } from '../stores/providerStore';
import type { ResultWindowCommandsPort } from '../application/result-window/ports';
import { resolveTranslationRequestLanguages } from './translationLanguages';

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export async function activeTranslationProviderIds() {
  const providerState = useProviderStore.getState();
  await providerState.loadTranslationProviders();

  return useProviderStore.getState().activeTranslationProviders;
}

export function useTranslate(
  translateTextWithProvider: ResultWindowCommandsPort['translateTextWithProvider'],
  recordTranslationHistory: ResultWindowCommandsPort['recordTranslationHistory'],
) {
  const {
    sourceText,
    sourceLang,
    targetLang,
    startTranslationSession,
    beginProviderTranslation,
    completeProviderTranslation,
    failProviderTranslation,
    setTranslating,
  } = useAppStore();

  const translate = async (text?: string, from?: string, to?: string) => {
    const textToTranslate = text || sourceText;
    const fromLang = from || sourceLang;
    const toLang = to || targetLang;
    const requestLanguages = resolveTranslationRequestLanguages(
      textToTranslate,
      fromLang,
      toLang,
    );

    if (!textToTranslate.trim()) return;

    const providerIds = await activeTranslationProviderIds();
    const sessionId = startTranslationSession(textToTranslate, providerIds);

    if (providerIds.length === 0) {
      setTranslating(false);
      return;
    }

    const startedAt = performance.now();
    const results = await Promise.all(
      providerIds.map(async (providerId) => {
        beginProviderTranslation(sessionId, providerId);
        try {
          const result = await translateTextWithProvider(providerId, {
            text: textToTranslate,
            sourceLang: requestLanguages.sourceLang,
            targetLang: requestLanguages.targetLang,
          });
          completeProviderTranslation(sessionId, result);
          return result;
        } catch (error) {
          console.error(`Translation failed for provider ${providerId}:`, error);
          failProviderTranslation(sessionId, providerId, errorMessage(error));
          return null;
        }
      }),
    );

    const completedResults = results.filter((result) => result !== null);
    if (completedResults.length > 0) {
      try {
        await recordTranslationHistory({
          text: textToTranslate,
          sourceLang: requestLanguages.sourceLang,
          targetLang: requestLanguages.targetLang,
          results: completedResults,
          durationMs: Math.max(0, Math.round(performance.now() - startedAt)),
        });
      } catch (error) {
        console.error('Failed to record translation history:', error);
      }
    }
  };

  const retryProvider = async (providerId: string) => {
    const {
      translationSessionId,
      sourceText: currentSourceText,
      sourceLang: currentSourceLang,
      targetLang: currentTargetLang,
    } = useAppStore.getState();

    if (!translationSessionId || !currentSourceText.trim()) return;
    const requestLanguages = resolveTranslationRequestLanguages(
      currentSourceText,
      currentSourceLang,
      currentTargetLang,
    );

    beginProviderTranslation(translationSessionId, providerId);
    try {
      const result = await translateTextWithProvider(providerId, {
        text: currentSourceText,
        sourceLang: requestLanguages.sourceLang,
        targetLang: requestLanguages.targetLang,
      });
      completeProviderTranslation(translationSessionId, result);
    } catch (error) {
      console.error(`Translation retry failed for provider ${providerId}:`, error);
      failProviderTranslation(translationSessionId, providerId, errorMessage(error));
    }
  };

  return { translate, retryProvider };
}
