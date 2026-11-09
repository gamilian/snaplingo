import { isEditorShortcutKey, normalizeEditorShortcutKey } from '../capture-workspace/annotationShortcuts';

const MODIFIERS = ['ctrl', 'meta', 'alt', 'shift'];

export function normalizePinBorderShortcut(value: string): string | null {
  if (!value.trim()) return '';
  const parts = value.toLowerCase().replace(/⌃/g, 'ctrl+').replace(/[⌘⊞]/g, 'meta+').replace(/⌥/g, 'alt+').replace(/⇧/g, 'shift+').replace(/\s/g, '').split('+').map((part) =>
    ({ control: 'ctrl', command: 'meta', super: 'meta', option: 'alt' }[part] ?? part),
  );
  const key = parts.pop()!;
  if (!isEditorShortcutKey(key) || parts.some((part) => !MODIFIERS.includes(part)) || new Set(parts).size !== parts.length) return null;
  return [...MODIFIERS.filter((modifier) => parts.includes(modifier)), key].join('+');
}

export function pinBorderShortcutFromEvent(event: {
  key: string; code?: string; ctrlKey: boolean; metaKey: boolean; altKey: boolean; shiftKey: boolean;
}) {
  return normalizePinBorderShortcut([
    event.ctrlKey && 'ctrl', event.metaKey && 'meta', event.altKey && 'alt', event.shiftKey && 'shift',
    normalizeEditorShortcutKey(event),
  ].filter(Boolean).join('+'));
}

export function isPinBorderShortcut(event: Parameters<typeof pinBorderShortcutFromEvent>[0] & { repeat?: boolean }, shortcut = 'b') {
  return Boolean(shortcut) && !event.repeat && pinBorderShortcutFromEvent(event) === normalizePinBorderShortcut(shortcut);
}
