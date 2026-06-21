import { useAppStore } from '../stores/appStore';
import { translateText } from '../tauri/translation';

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
      const results = await translateText({
        text: textToTranslate,
        sourceLang: fromLang,
        targetLang: toLang,
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
