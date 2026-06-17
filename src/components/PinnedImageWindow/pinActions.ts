export type PinInvokeArgs = Record<string, unknown>;
export type PinInvoke = <T>(
  command: string,
  args?: PinInvokeArgs,
) => Promise<T>;
export type PinWriteText = (text: string) => Promise<void>;

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
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
}

interface PinWindow {
  hide?: () => Promise<void>;
  close?: () => Promise<void>;
}

interface PinMainWindow {
  show?: () => Promise<void>;
  setFocus?: () => Promise<void>;
}

type PinMainWindowResolver = () => Promise<PinMainWindow | null>;

export type PinnedHoverToolbarActionId = 'copy' | 'save' | 'close';

export interface PinnedHoverToolbarAction {
  id: PinnedHoverToolbarActionId;
  label: string;
  title: string;
  ariaLabel: string;
}

export function getPinnedHoverToolbarActions() {
  return [
    {
      id: 'copy',
      label: 'Copy',
      title: 'Copy',
      ariaLabel: 'Copy pinned image',
    },
    {
      id: 'save',
      label: 'Save',
      title: 'Save',
      ariaLabel: 'Save pinned image',
    },
    {
      id: 'close',
      label: 'X',
      title: 'Close',
      ariaLabel: 'Close pinned image',
    },
  ] satisfies PinnedHoverToolbarAction[];
}

export function isCopyPinnedImageShortcut(event: PinShortcutEvent) {
  return (
    event.key.toLowerCase() === 'c' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isCopyPinnedTextShortcut(
  event: PinShortcutEvent,
  sourceText?: string | null,
) {
  return (
    hasPinnedSourceText(sourceText) &&
    event.key.toLowerCase() === 'c' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !!event.shiftKey
  );
}

export async function copyPinnedText(
  writeText: PinWriteText,
  sourceText?: string | null,
) {
  if (!hasPinnedSourceText(sourceText)) return false;

  await writeText(sourceText);
  return true;
}

export function isSavePinnedImageShortcut(event: PinShortcutEvent) {
  return (
    event.key.toLowerCase() === 's' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  );
}

export function isQuickSavePinnedImageShortcut(event: PinShortcutEvent) {
  return (
    event.key.toLowerCase() === 's' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !!event.shiftKey
  );
}

export function isOpenPinnedPreferencesShortcut(event: PinShortcutEvent) {
  return (
    event.key.toLowerCase() === 'p' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !!event.shiftKey
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
  return (
    event.key === 'Escape' &&
    event.shiftKey &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey
  );
}

export async function savePinnedImage(invoke: PinInvoke, imageId: string) {
  const path = await invoke<string>('default_capture_save_path');

  await invoke('save_pinned_image', {
    imageId,
    path,
  });
}

export async function quickSavePinnedImage(
  invoke: PinInvoke,
  imageId: string,
  directory?: string,
) {
  const path = await invoke<string>('quick_capture_save_path', { directory });

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

export async function openPinnedPreferences(
  getMainWindow: PinMainWindowResolver,
) {
  const mainWindow = await getMainWindow();
  await mainWindow?.show?.();
  await mainWindow?.setFocus?.();
}

export async function closePinnedImage(
  invoke: PinInvoke,
  imageId: string,
) {
  await invoke('close_pinned_image', { imageId });
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

function hasPinnedSourceText(sourceText?: string | null): sourceText is string {
  return typeof sourceText === 'string' && sourceText.length > 0;
}
