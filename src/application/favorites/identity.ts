export interface TranslationFavoriteIdentity {
  sourceText: string;
  sourceLang: string;
  targetLang: string;
  providerId: string;
  translatedText: string;
}

export interface OcrFavoriteIdentity {
  recognizedText: string;
  language: string | null;
  providerUsed: string;
}

export function translationFavoriteKey(input: TranslationFavoriteIdentity) {
  return JSON.stringify([
    'translation',
    input.sourceText,
    input.sourceLang,
    input.targetLang,
    input.providerId,
    input.translatedText,
  ]);
}

export function ocrFavoriteKey(input: OcrFavoriteIdentity) {
  return JSON.stringify([
    'ocr',
    input.recognizedText,
    input.language,
    input.providerUsed,
  ]);
}
