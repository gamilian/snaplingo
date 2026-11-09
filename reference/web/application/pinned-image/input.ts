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
    !!sourceText &&
    event.key.toLowerCase() === 'c' &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !!event.shiftKey
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
