import { describe, expect, it } from 'vitest';
import {
  annotationShortcutBindings,
  annotationShortcutConflict,
  isScrollingCaptureShortcut,
} from './annotationShortcuts';
import { captureInterfaceScale } from './interfaceScale';

describe('capture editor preferences', () => {
  it('uses default keys for old configurations and allows a tool to be cleared or reassigned', () => {
    expect(annotationShortcutBindings().a).toBe('arrow');
    const shortcuts = annotationShortcutBindings({ arrow: 'x', text: '' });
    expect(shortcuts.x).toBe('arrow');
    expect(shortcuts.a).toBeUndefined();
    expect(shortcuts.t).toBeUndefined();
  });
  it('rejects duplicate tool keys and keys used by fixed editor actions', () => {
    expect(annotationShortcutConflict({ arrow: 'o' })).toMatchObject({
      kind: 'duplicate',
    });
    expect(annotationShortcutConflict({ arrow: 's' })).toMatchObject({
      kind: 'duplicate',
    });
    expect(annotationShortcutConflict({ arrow: 'c' })).toMatchObject({
      kind: 'reserved',
    });
    expect(annotationShortcutConflict({ arrow: 'Ctrl+A' })).toMatchObject({
      kind: 'invalid',
    });
  });
  it('lets the scrolling shortcut be reassigned or disabled without becoming an annotation tool', () => {
    const event = {
      key: 'x',
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      repeat: false,
    };
    expect(isScrollingCaptureShortcut(event, { scrolling: 'x' })).toBe(true);
    expect(
      isScrollingCaptureShortcut({ ...event, key: 's' }, { scrolling: 'x' }),
    ).toBe(false);
    expect(isScrollingCaptureShortcut(event, { scrolling: '' })).toBe(false);
    expect(
      isScrollingCaptureShortcut(
        { ...event, ctrlKey: true },
        { scrolling: 'x' },
      ),
    ).toBe(false);
    expect(annotationShortcutBindings({ scrolling: 'x', arrow: 's' }).s).toBe(
      'arrow',
    );
    expect(annotationShortcutBindings({ scrolling: 'x' }).x).toBeUndefined();
  });
  it('scales controls without applying the OS DPI multiplier a second time', () => {
    expect(
      captureInterfaceScale('auto', { x: 0, y: 0, width: 1920, height: 1080 }),
    ).toBe(1);
    expect(
      captureInterfaceScale('auto', { x: 0, y: 0, width: 3840, height: 2160 }),
    ).toBeGreaterThan(1);
    expect(captureInterfaceScale('150')).toBe(1.5);
  });
  it('accepts digits and function keys and matches physical keys under an input method', () => {
    expect(annotationShortcutBindings({ arrow: '2', blur: 'F2' })['2']).toBe('arrow');
    expect(annotationShortcutBindings({ arrow: '2', blur: 'F2' }).f2).toBe('blur');
    expect(annotationShortcutConflict({ arrow: 'F25' })).toMatchObject({ kind: 'invalid' });
    expect(isScrollingCaptureShortcut({ key: 'Process', code: 'Digit9', metaKey: false, ctrlKey: false, altKey: false, repeat: false }, { scrolling: '9' })).toBe(true);
  });
});
