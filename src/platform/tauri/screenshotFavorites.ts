import { invoke } from '@tauri-apps/api/core';
import type {
  ScreenshotFavoritePage,
  ScreenshotFavoriteQuery,
  SettingsScreenshotFavoritesPort,
} from '../../application/settings/ports';

export function queryScreenshotFavorites(
  query: ScreenshotFavoriteQuery,
): Promise<ScreenshotFavoritePage> {
  return invoke('query_screenshot_favorites', { query });
}

export function updateScreenshotFavoriteMetadata(
  id: number,
  note: string | null,
  tags: string[],
) {
  return invoke<void>('update_screenshot_favorite_metadata', { id, note, tags });
}

export function deleteScreenshotFavorite(id: number) {
  return invoke<void>('delete_screenshot_favorite', { id });
}

export function copyScreenshotFavorite(id: number) {
  return invoke<void>('copy_screenshot_favorite', { id });
}

export function revealScreenshotFavorite(id: number) {
  return invoke<void>('reveal_screenshot_favorite', { id });
}

export const screenshotFavorites: SettingsScreenshotFavoritesPort = {
  queryScreenshotFavorites,
  updateScreenshotFavoriteMetadata,
  deleteScreenshotFavorite,
  copyScreenshotFavorite,
  revealScreenshotFavorite,
};
