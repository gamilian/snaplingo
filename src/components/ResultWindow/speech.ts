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
  if (!languageCode) return '';
  return RESULT_WINDOW_SPEECH_LANGUAGE_MAP[languageCode] ?? languageCode;
}

export function speakResultWindowText(text: string, languageCode?: string) {
  const normalizedText = text.trim();
  const speechSynthesisApi = globalThis.speechSynthesis;
  const SpeechSynthesisUtteranceCtor = globalThis.SpeechSynthesisUtterance;

  if (!normalizedText || !speechSynthesisApi || !SpeechSynthesisUtteranceCtor) {
    return false;
  }

  speechSynthesisApi.cancel();

  const utterance = new SpeechSynthesisUtteranceCtor(normalizedText);
  const speechLanguage = resultWindowSpeechLanguage(languageCode);

  if (speechLanguage) {
    utterance.lang = speechLanguage;
  }

  speechSynthesisApi.speak(utterance);
  return true;
}
