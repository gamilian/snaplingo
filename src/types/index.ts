export interface TranslationResult {
  provider_id: string;
  translated_text: string;
  detected_language: string | null;
  confidence: number | null;
}
