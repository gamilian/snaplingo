import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';
import { TranslationResult } from '../types';

export function useTranslate() {
  const {
    sourceText,
    sourceLang,
    targetLang,
    setTranslations,
    setTranslating
  } = useAppStore();

  const translate = async (text?: string, from?: string, to?: string) => {
    const textToTranslate = text || sourceText;
    const fromLang = from || sourceLang;
    const toLang = to || targetLang;

    if (!textToTranslate.trim()) return;

    setTranslating(true);

    try {
      const results = await invoke<TranslationResult[]>('translate_text_v2', {
        request: {
          text: textToTranslate,
          source_lang: fromLang === 'auto' ? null : fromLang,
          target_lang: toLang,
        }
      });

      setTranslations(results);
    } catch (error) {
      console.error('Translation failed:', error);
      setTranslations([]);
    } finally {
      setTranslating(false);
    }
  };

  return { translate };
}
