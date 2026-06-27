import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import {
  getCaptureEditorCommandButtonClassName,
  getCaptureEditorIconButtonClassName,
  getCaptureEditorSelectionClassName,
  getCaptureSelectionOverlayCanvasClassName,
  getCaptureEditorToolbarClassName,
  getCaptureRootClassName,
  shouldShowCaptureLoadingMask,
} from './capturePresentation';

describe('capture presentation', () => {
  it('keeps the default document canvas transparent before the app mounts', () => {
    const css = readFileSync(new URL('../../index.css', import.meta.url), 'utf8');

    expect(css).toMatch(/html,\s*body,\s*#root\s*{[^}]*background:\s*transparent/s);
  });

  it('keeps the capture html shell transparent before React renders', () => {
    const html = readFileSync(new URL('../../../index.html', import.meta.url), 'utf8');

    expect(html).toContain('data-window="capture"');
    expect(html).toMatch(
      /html\[data-window="capture"\][\s\S]*background:\s*transparent/s,
    );
  });

  it('keeps the capture surface transparent while the snapshot session loads', () => {
    expect(getCaptureRootClassName('loading')).not.toContain('bg-black');
    expect(shouldShowCaptureLoadingMask('loading')).toBe(false);
  });

  it('uses the same transparent capture surface once selection is active', () => {
    expect(getCaptureRootClassName('selecting')).toContain('bg-transparent');
    expect(getCaptureRootClassName('preview')).toContain('bg-transparent');
  });

  it('does not fade the native capture root over underlying windows', () => {
    const className = getCaptureRootClassName('selecting');

    expect(className).not.toContain('opacity-0');
    expect(className).not.toContain('transition-opacity');
  });

  it('keeps the selection overlay canvas visible on the first revealed frame', () => {
    const className = getCaptureSelectionOverlayCanvasClassName();

    expect(className).not.toContain('opacity-0');
    expect(className).not.toContain('transition-opacity');
  });

  it('uses a light floating toolbar for the editing surface', () => {
    expect(getCaptureEditorToolbarClassName()).toEqual(
      expect.stringContaining('bg-white/95'),
    );
    expect(getCaptureEditorToolbarClassName()).toEqual(
      expect.stringContaining('rounded-[16px]'),
    );
    expect(getCaptureEditorToolbarClassName()).toEqual(
      expect.stringContaining('text-slate-600'),
    );
  });

  it('keeps the selection hit layer transparent and primary editor affordances', () => {
    expect(getCaptureEditorSelectionClassName('preview')).toEqual(
      expect.stringContaining('bg-transparent'),
    );
    expect(getCaptureEditorSelectionClassName('preview')).not.toContain(
      'border-[#5b7fff]',
    );
    expect(getCaptureEditorSelectionClassName('preview')).toEqual(
      expect.stringContaining('cursor-move'),
    );
    expect(getCaptureEditorSelectionClassName('preview')).toEqual(
      expect.stringContaining('rounded-[8px]'),
    );
    expect(getCaptureEditorIconButtonClassName(true)).toEqual(
      expect.stringContaining('border-[#5b7fff]'),
    );
    expect(getCaptureEditorCommandButtonClassName('primary')).toEqual(
      expect.stringContaining('bg-[#5b7fff]'),
    );
  });
});
