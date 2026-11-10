import { describe, expect, it } from 'vitest';
import { isPinBorderShortcut, normalizePinBorderShortcut, pinBorderShortcutFromEvent } from './borderShortcut';

describe('local pin border shortcut', () => {
  it('normalizes modifier order and aliases and requires the exact modifiers', () => {
    expect(normalizePinBorderShortcut(' Shift + Control + F3 ')).toBe('ctrl+shift+f3');
    expect(normalizePinBorderShortcut('⌃⇧F3')).toBe('ctrl+shift+f3');
    const event = { key: 'F3', ctrlKey: true, metaKey: false, altKey: false, shiftKey: false };
    expect(isPinBorderShortcut(event, 'ctrl+f3')).toBe(true);
    expect(isPinBorderShortcut({ ...event, shiftKey: true }, 'ctrl+f3')).toBe(false);
    expect(isPinBorderShortcut({ ...event, repeat: true }, 'ctrl+f3')).toBe(false);
    expect(isPinBorderShortcut(event, '')).toBe(false);
  });
  it('records physical keys during input-method processing and rejects malformed bindings', () => {
    expect(pinBorderShortcutFromEvent({ key: 'Process', code: 'KeyB', ctrlKey: false, metaKey: true, altKey: false, shiftKey: false })).toBe('meta+b');
    expect(normalizePinBorderShortcut('ctrl+ctrl+b')).toBeNull();
    expect(normalizePinBorderShortcut('f25')).toBeNull();
    expect(normalizePinBorderShortcut('')).toBe('');
  });
});
