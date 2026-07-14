import { invoke } from '@tauri-apps/api/core';
import type { TranslationResult } from '../../types';

export interface TranslateTextInput {
  text: string;
  sourceLang: string;
  targetLang: string;
}

export interface RecordTranslationHistoryInput extends TranslateTextInput {
  results: TranslationResult[];
  durationMs: number;
}

export async function translateText(input: TranslateTextInput) {
  return invoke<TranslationResult[]>('translate_text_v2', {
    request: {
      text: input.text,
      source_lang: input.sourceLang === 'auto' ? null : input.sourceLang,
      target_lang: input.targetLang,
    },
  });
}

export async function translateTextWithProvider(
  providerId: string,
  input: TranslateTextInput,
) {
  return invoke<TranslationResult>('translate_text_with_provider', {
    providerId,
    request: {
      text: input.text,
      source_lang: input.sourceLang === 'auto' ? null : input.sourceLang,
      target_lang: input.targetLang,
    },
  });
}

export function recordTranslationHistory(input: RecordTranslationHistoryInput) {
  return invoke<void>('record_translation_history', {
    request: {
      text: input.text,
      source_lang: input.sourceLang === 'auto' ? null : input.sourceLang,
      target_lang: input.targetLang,
    },
    results: input.results,
    durationMs: input.durationMs,
  });
}
