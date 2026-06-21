import { invoke } from '@tauri-apps/api/core';
import type { TranslationResult } from '../types';

export interface TranslateTextInput {
  text: string;
  sourceLang: string;
  targetLang: string;
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
