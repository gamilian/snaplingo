import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';

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
      let detectedFrom = fromLang;

      if (fromLang === 'auto') {
        detectedFrom = await invoke<string>('detect_language', {
          text: textToTranslate
        });
      }

      const results = await invoke<any[]>('translate_text', {
        text: textToTranslate,
        from: detectedFrom,
        to: toLang,
        providerIds: ['google-translate'],
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
