import {
  defaultCaptureSavePath,
  quickCaptureSavePath,
} from '../../tauri/captureSession';
import {
  closePinnedImage as closePinnedImageCommand,
  copyPinnedImage as copyPinnedImageCommand,
  destroyPinnedImageGroup as destroyPinnedImageGroupCommand,
  hidePinnedImageGroup as hidePinnedImageGroupCommand,
  movePinnedImageToNextGroup as movePinnedImageToNextGroupCommand,
  removePinnedImage,
  replacePinnedImageFromClipboard as replacePinnedImageFromClipboardCommand,
  savePinnedImage as savePinnedImageCommand,
} from '../../tauri/pinnedImage';
import type { PinnedImageView } from '../ScreenshotSession/types';

export type PinWriteText = (text: string) => Promise<void>;

export interface PinActionClient {
  defaultCaptureSavePath: () => Promise<string>;
  quickCaptureSavePath: (directory?: string) => Promise<string>;
  copyPinnedImage: (imageId: string) => Promise<void>;
  replacePinnedImageFromClipboard: (
    imageId: string,
  ) => Promise<PinnedImageView>;
  savePinnedImage: (imageId: string, path: string) => Promise<void>;
  closePinnedImage: (imageId: string) => Promise<void>;
  removePinnedImage: (imageId: string) => Promise<void>;
  movePinnedImageToNextGroup: (imageId: string) => Promise<number>;
  hidePinnedImageGroup: (imageId: string) => Promise<string[]>;
  destroyPinnedImageGroup: (imageId: string) => Promise<string[]>;
}

const tauriPinActionClient: PinActionClient = {
  defaultCaptureSavePath,
  quickCaptureSavePath,
  copyPinnedImage: copyPinnedImageCommand,
  replacePinnedImageFromClipboard: replacePinnedImageFromClipboardCommand,
  savePinnedImage: savePinnedImageCommand,
  closePinnedImage: closePinnedImageCommand,
  removePinnedImage,
  movePinnedImageToNextGroup: movePinnedImageToNextGroupCommand,
  hidePinnedImageGroup: hidePinnedImageGroupCommand,
  destroyPinnedImageGroup: destroyPinnedImageGroupCommand,
};

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

interface PinSettingsWindow {
  show?: () => Promise<void>;
  setFocus?: () => Promise<void>;
}

type PinSettingsWindowResolver = () => Promise<PinSettingsWindow | null>;

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

export async function copyPinnedImage(
  imageId: string,
  client: PinActionClient = tauriPinActionClient,
) {
  await client.copyPinnedImage(imageId);
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

export async function savePinnedImage(
  imageId: string,
  client: PinActionClient = tauriPinActionClient,
) {
  const path = await client.defaultCaptureSavePath();

  await client.savePinnedImage(imageId, path);
}

export async function quickSavePinnedImage(
  imageId: string,
  directory?: string,
  client: PinActionClient = tauriPinActionClient,
) {
  const path = await client.quickCaptureSavePath(directory);

  await client.savePinnedImage(imageId, path);
}

export async function movePinnedImageToNextGroup(
  imageId: string,
  client: PinActionClient = tauriPinActionClient,
) {
  await client.movePinnedImageToNextGroup(imageId);
}

export async function replacePinnedImageFromClipboard<T>(
  imageId: string,
  client: PinActionClient = tauriPinActionClient,
) {
  return client.replacePinnedImageFromClipboard(imageId) as Promise<T>;
}

export async function openPinnedPreferences(
  getSettingsWindow: PinSettingsWindowResolver,
) {
  const settingsWindow = await getSettingsWindow();
  await settingsWindow?.show?.();
  await settingsWindow?.setFocus?.();
}

export async function closePinnedImage(
  imageId: string,
  client: PinActionClient = tauriPinActionClient,
) {
  await client.closePinnedImage(imageId);
}

export async function hidePinnedImageGroup(
  imageId: string,
  client: PinActionClient = tauriPinActionClient,
) {
  await client.hidePinnedImageGroup(imageId);
}

export async function destroyPinnedImage(
  imageId: string,
  window: PinWindow,
  client: PinActionClient = tauriPinActionClient,
) {
  await client.removePinnedImage(imageId);
  await window.close?.();
}

export async function destroyPinnedImageGroup(
  imageId: string,
  client: PinActionClient = tauriPinActionClient,
) {
  await client.destroyPinnedImageGroup(imageId);
}

function hasPinnedSourceText(sourceText?: string | null): sourceText is string {
  return typeof sourceText === 'string' && sourceText.length > 0;
}
