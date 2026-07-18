import type { ResultWindowSpeechPort } from './ports';

const RESULT_WINDOW_SPEECH_LANGUAGE_MAP: Record<string, string> = {
  zh: 'zh-CN',
  'zh-CN': 'zh-CN',
  'zh-TW': 'zh-TW',
  en: 'en-US',
  ja: 'ja-JP',
  ko: 'ko-KR',
  es: 'es-ES',
  fr: 'fr-FR',
  de: 'de-DE',
  ru: 'ru-RU',
  ar: 'ar-SA',
};

export function resultWindowSpeechLanguage(languageCode?: string) {
  if (!languageCode) return undefined;
  return RESULT_WINDOW_SPEECH_LANGUAGE_MAP[languageCode] ?? languageCode;
}

export async function speakResultWindowText(
  speech: ResultWindowSpeechPort,
  text: string,
  languageCode?: string,
) {
  const normalizedText = text.trim();
  if (!normalizedText) return false;

  try {
    await speech.speak(
      normalizedText,
      resultWindowSpeechLanguage(languageCode),
    );
    return true;
  } catch (error) {
    console.error('Failed to speak result text:', error);
    return false;
  }
}
