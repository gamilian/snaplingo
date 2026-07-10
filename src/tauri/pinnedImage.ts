import { invoke } from '@tauri-apps/api/core';
import type { PinnedImageView } from '../domain/capture';

export async function getPinnedImage(imageId: string) {
  return invoke<PinnedImageView>('get_pinned_image', { imageId });
}

export async function copyPinnedImage(imageId: string) {
  return invoke<void>('copy_pinned_image', { imageId });
}

export async function replacePinnedImageFromClipboard(imageId: string) {
  return invoke<PinnedImageView>('replace_pinned_image_from_clipboard', {
    imageId,
  });
}

export async function savePinnedImage(imageId: string, path: string) {
  return invoke<void>('save_pinned_image', { imageId, path });
}

export async function closePinnedImage(imageId: string) {
  return invoke<void>('close_pinned_image', { imageId });
}

export async function removePinnedImage(imageId: string) {
  return invoke<void>('remove_pinned_image', { imageId });
}

export async function togglePinnedImagesVisibility() {
  return invoke<boolean | null>('toggle_pinned_images_visibility');
}

export async function switchPinnedImageGroup() {
  return invoke<number | null>('switch_pinned_image_group');
}

export async function movePinnedImageToNextGroup(imageId: string) {
  return invoke<number>('move_pinned_image_to_next_group', { imageId });
}

export async function hidePinnedImageGroup(imageId: string) {
  return invoke<string[]>('hide_pinned_image_group', { imageId });
}

export async function destroyPinnedImageGroup(imageId: string) {
  return invoke<string[]>('destroy_pinned_image_group', { imageId });
}
