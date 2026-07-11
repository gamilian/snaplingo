import { invoke } from '@tauri-apps/api/core';
import type {
  HistoryEntry,
  OcrHistoryEntry,
  TranslationHistoryEntry,
} from '../../application/settings/ports';

interface BackendTranslationEntry {
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

interface BackendOcrEntry {
  id: number;
  timestamp: string;
  image_hash: string;
  language: string | null;
  provider_used: string;
  recognized_text: string;
  confidence: number | null;
  duration_ms: number;
}

type BackendHistoryEntry =
  | (BackendTranslationEntry & { type: 'Translation' })
  | (BackendOcrEntry & { type: 'Ocr' });

function toTranslationHistoryEntry(
  entry: BackendTranslationEntry,
): TranslationHistoryEntry {
  return {
    id: entry.id,
    timestamp: entry.timestamp,
    sourceText: entry.source_text,
    sourceLang: entry.source_lang,
    targetLang: entry.target_lang,
    providersUsed: entry.providers_used,
    results: entry.results.map((result) => ({
      providerId: result.provider_id,
      translatedText: result.translated_text,
      detectedLanguage: result.detected_language,
      confidence: result.confidence,
    })),
    durationMs: entry.duration_ms,
  };
}

function toOcrHistoryEntry(entry: BackendOcrEntry): OcrHistoryEntry {
  return {
    id: entry.id,
    timestamp: entry.timestamp,
    imageHash: entry.image_hash,
    language: entry.language,
    providerUsed: entry.provider_used,
    recognizedText: entry.recognized_text,
    confidence: entry.confidence,
    durationMs: entry.duration_ms,
  };
}

function toHistoryEntry(entry: BackendHistoryEntry): HistoryEntry {
  if (entry.type === 'Translation') {
    return { ...toTranslationHistoryEntry(entry), type: 'translation' };
  }

  return { ...toOcrHistoryEntry(entry), type: 'ocr' };
}

export async function getTranslationHistory(limit: number, offset: number) {
  const entries = await invoke<BackendTranslationEntry[]>(
    'get_translation_history',
    { limit, offset },
  );
  return entries.map(toTranslationHistoryEntry);
}

export async function getOcrHistory(limit: number, offset: number) {
  const entries = await invoke<BackendOcrEntry[]>('get_ocr_history', {
    limit,
    offset,
  });
  return entries.map(toOcrHistoryEntry);
}

export async function searchHistory(query: string) {
  const entries = await invoke<BackendHistoryEntry[]>('search_history', {
    query,
  });
  return entries.map(toHistoryEntry);
}

export function deleteHistory(id: number) {
  return invoke<void>('delete_history', { id });
}

export function clearAllHistory() {
  return invoke<void>('clear_all_history');
}
