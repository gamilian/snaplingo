import defaults from '../../../../assets/defaults/annotationShortcuts.json';
import type { AnnotationTool } from './annotationStyle';

export const DEFAULT_ANNOTATION_SHORTCUTS: Readonly<
  Record<AnnotationTool | 'scrolling', string>
> = defaults;
const RESERVED_KEYS = new Set(['c', 'f', 'r', 'f3']);

export function normalizeEditorShortcutKey(event: { key: string; code?: string }) {
  const physical = event.code?.match(/^(?:Key([A-Z])|Digit([0-9])|(F(?:[1-9]|1[0-9]|2[0-4])))$/);
  return (physical ? physical[1] ?? physical[2] ?? physical[3] : event.key).toLowerCase();
}

export function isEditorShortcutKey(key: string) {
  return /^[a-z0-9]$|^f(?:[1-9]|1[0-9]|2[0-4])$/.test(key.toLowerCase());
}

export function annotationShortcutConflict(
  overrides: Readonly<Record<string, string>>,
) {
  const keys = new Map<string, string>();
  for (const [tool, value] of Object.entries({ ...defaults, ...overrides })) {
    if (!(tool in defaults)) return { kind: 'invalid' as const, tool };
    const key = value.toLowerCase();
    if (!key) continue;
    if (!isEditorShortcutKey(key)) return { kind: 'invalid' as const, tool };
    if (RESERVED_KEYS.has(key)) return { kind: 'reserved' as const, tool };
    const conflict = keys.get(key);
    if (conflict) return { kind: 'duplicate' as const, tool, conflict };
    keys.set(key, tool);
  }
  return null;
}

export function annotationShortcutBindings(
  overrides: Readonly<Record<string, string>> = {},
): Record<string, AnnotationTool> {
  const shortcuts = annotationShortcutConflict(overrides)
    ? defaults
    : { ...defaults, ...overrides };
  return Object.fromEntries(
    Object.entries(shortcuts)
      .filter(([tool, key]) => tool !== 'scrolling' && key)
      .map(([tool, key]) => [key.toLowerCase(), tool]),
  ) as Record<string, AnnotationTool>;
}

export function isScrollingCaptureShortcut(
  event: {
    key: string;
    code?: string;
    metaKey: boolean;
    ctrlKey: boolean;
    altKey: boolean;
    repeat: boolean;
  },
  overrides: Readonly<Record<string, string>> = {},
) {
  const shortcuts = annotationShortcutConflict(overrides)
    ? defaults
    : { ...defaults, ...overrides };
  return (
    Boolean(shortcuts.scrolling) &&
    normalizeEditorShortcutKey(event) === shortcuts.scrolling.toLowerCase() &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.repeat
  );
}
