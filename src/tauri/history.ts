import { invoke } from '@tauri-apps/api/core';

export interface BackendTranslationEntry {
  id: number;
  timestamp: string;
  source_text: string;
  source_lang: string;
  target_lang: string;
  providers_used: string[];
  results: Array<{
    provider_id?: string;
    translated_text: string;
    detected_language?: string;
    confidence?: number;
  }>;
  duration_ms: number;
}

export interface BackendOcrEntry {
  id: number;
  timestamp: string;
  recognized_text: string;
  language?: string;
}

export interface BackendHistoryEntry {
  id: number;
  timestamp: string;
  entry_type: string;
}

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
