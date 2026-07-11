import { invoke } from '@tauri-apps/api/core';

export interface BackendTranslationEntry {
  id: number;
  timestamp: string;
  source_text: string;
  source_lang: string;
  target_lang: string;
  providers_used: string[];
  results: Array<{
    provider_id: string;
    translated_text: string;
    detected_language: string | null;
    confidence: number | null;
  }>;
  duration_ms: number;
}

export interface BackendOcrEntry {
  id: number;
  timestamp: string;
  image_hash: string;
  language: string | null;
  provider_used: string;
  recognized_text: string;
  confidence: number | null;
  duration_ms: number;
}

export type BackendHistoryEntry =
  | (BackendTranslationEntry & { type: 'Translation' })
  | (BackendOcrEntry & { type: 'Ocr' });

export function getTranslationHistory(limit: number, offset: number) {
  return invoke<BackendTranslationEntry[]>('get_translation_history', { limit, offset });
}

export function getOcrHistory(limit: number, offset: number) {
  return invoke<BackendOcrEntry[]>('get_ocr_history', { limit, offset });
}

export function searchHistory(query: string) {
  return invoke<BackendHistoryEntry[]>('search_history', { query });
}

export function deleteHistory(id: number) {
  return invoke<void>('delete_history', { id });
}

export function clearAllHistory() {
  return invoke<void>('clear_all_history');
}
