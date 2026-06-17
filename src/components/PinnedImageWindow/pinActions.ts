export type PinInvokeArgs = Record<string, unknown>;
export type PinInvoke = <T>(
  command: string,
  args?: PinInvokeArgs,
) => Promise<T>;

interface PinShortcutEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
}

interface PinDestroyShortcutEvent {
  key: string;
  shiftKey: boolean;
}

interface PinWindow {
  hide?: () => Promise<void>;
  close?: () => Promise<void>;
}

export function isCopyPinnedImageShortcut(event: PinShortcutEvent) {
  return (
    event.key.toLowerCase() === 'c' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isSavePinnedImageShortcut(event: PinShortcutEvent) {
  return (
    event.key.toLowerCase() === 's' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isClosePinnedImageShortcut(event: PinShortcutEvent) {
  return (
    event.key.toLowerCase() === 'w' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isReplacePinnedImageShortcut(event: PinShortcutEvent) {
  return (
    event.key.toLowerCase() === 'v' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isDestroyPinnedImageShortcut(event: PinDestroyShortcutEvent) {
  return event.key === 'Escape' && event.shiftKey;
}

export async function savePinnedImage(invoke: PinInvoke, imageId: string) {
  const path = await invoke<string>('default_capture_save_path');

  await invoke('save_pinned_image', {
    imageId,
    path,
  });
}

export async function movePinnedImageToNextGroup(
  invoke: PinInvoke,
  imageId: string,
) {
  await invoke('move_pinned_image_to_next_group', {
    imageId,
  });
}

export async function replacePinnedImageFromClipboard<T>(
  invoke: PinInvoke,
  imageId: string,
) {
  return invoke<T>('replace_pinned_image_from_clipboard', { imageId });
}

export async function hidePinnedImage(window: PinWindow) {
  await window.hide?.();
}

export async function hidePinnedImageGroup(invoke: PinInvoke, imageId: string) {
  await invoke('hide_pinned_image_group', { imageId });
}

export async function destroyPinnedImage(
  invoke: PinInvoke,
  imageId: string,
  window: PinWindow,
) {
  await invoke('remove_pinned_image', { imageId });
  await window.close?.();
}

export async function destroyPinnedImageGroup(
  invoke: PinInvoke,
  imageId: string,
) {
  await invoke('destroy_pinned_image_group', { imageId });
}
