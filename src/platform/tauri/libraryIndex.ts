import { invoke } from '@tauri-apps/api/core';
import type {
  LibraryIndexPage,
  LibraryIndexQuery,
  SettingsLibraryIndexPort,
} from '../../application/settings/ports';

export function queryHistoryIndex(
  query: LibraryIndexQuery,
): Promise<LibraryIndexPage> {
  return invoke('query_library_history_index', { query });
}

export function queryFavoriteIndex(
  query: LibraryIndexQuery,
): Promise<LibraryIndexPage> {
  return invoke('query_library_favorite_index', { query });
}

export const libraryIndex: SettingsLibraryIndexPort = {
  queryHistoryIndex,
  queryFavoriteIndex,
};
