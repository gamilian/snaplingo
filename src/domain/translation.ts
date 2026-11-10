export interface TranslationError {
  code: string;
  message: string;
  retryable: boolean;
}

export interface TranslationResult {
  provider_id: string;
  translated_text: string;
  detected_language: string | null;
  confidence: number | null;
  error?: TranslationError | null;
  request_id?: string | null;
  duration_ms?: number | null;
}
