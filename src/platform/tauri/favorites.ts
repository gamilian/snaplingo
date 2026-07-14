import { invoke } from '@tauri-apps/api/core';
import type {
  FavoriteKind,
  FavoritePage,
  FavoriteQuery,
  OcrFavoriteInput,
  TranslationFavoriteInput,
} from '../../application/settings/ports';

export function addTranslationFavorite(input: TranslationFavoriteInput) {
  return invoke<number>('favorite_translation_result', {
    sourceHistoryId: input.sourceHistoryId ?? null,
    request: {
      text: input.sourceText,
      source_lang: input.sourceLang,
      target_lang: input.targetLang,
    },
    result: {
      provider_id: input.providerId,
      translated_text: input.translatedText,
      detected_language: input.detectedLanguage ?? null,
      confidence: input.confidence ?? null,
    },
  });
}

export function addOcrFavorite(input: OcrFavoriteInput) {
  return invoke<number>('favorite_ocr_result', {
    sourceHistoryId: input.sourceHistoryId ?? null,
    request: {
      image_data: Array.from(input.imageData ?? []),
      language: input.language ?? null,
    },
    result: {
      text: input.recognizedText,
      confidence: input.confidence ?? null,
    },
    providerUsed: input.providerUsed,
  });
}

export function queryFavorites(query: FavoriteQuery) {
  return invoke<FavoritePage>('query_favorites', { query });
}

export function updateFavoriteMetadata(
  id: number,
  note: string | null,
  tags: string[],
) {
  return invoke<void>('update_favorite_metadata', { id, note, tags });
}

export function deleteFavorite(id: number) {
  return invoke<void>('delete_favorite', { id });
}

export async function rerunOcrFavorite(id: number) {
  const result = await invoke<{ text: string }>('rerun_ocr_favorite', { id });
  return result.text;
}

export function listFavoriteTags(kind: FavoriteKind) {
  return invoke<string[]>('list_favorite_tags', { kind });
}

export const favorites = {
  addTranslationFavorite,
  addOcrFavorite,
  queryFavorites,
  updateFavoriteMetadata,
  deleteFavorite,
  rerunOcrFavorite,
  listFavoriteTags,
};
