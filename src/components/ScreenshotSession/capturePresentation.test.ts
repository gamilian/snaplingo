import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import {
  getCaptureEditorCommandButtonClassName,
  getCaptureEditorIconButtonClassName,
  getCaptureEditorSelectionClassName,
  getCaptureEditorToolbarClassName,
  getCaptureRootClassName,
  shouldShowCaptureLoadingMask,
} from './capturePresentation';

describe('capture presentation', () => {
  it('keeps the document canvas transparent before the capture app mounts', () => {
    const css = readFileSync(new URL('../../index.css', import.meta.url), 'utf8');

    expect(css).toMatch(/html,\s*body,\s*#root\s*{[^}]*background:\s*transparent/s);
  });

  it('keeps the capture surface transparent while the snapshot session loads', () => {
    expect(getCaptureRootClassName('loading')).not.toContain('bg-black');
    expect(shouldShowCaptureLoadingMask('loading')).toBe(false);
  });

  it('uses the same transparent capture surface once selection is active', () => {
    expect(getCaptureRootClassName('selecting')).toContain('bg-transparent');
    expect(getCaptureRootClassName('preview')).toContain('bg-transparent');
  });

  it('uses a short linear fade for the first visible capture frame', () => {
    expect(
      getCaptureRootClassName('selecting', { isSurfaceVisible: false }),
    ).toContain('opacity-0');
    expect(
      getCaptureRootClassName('selecting', { isSurfaceVisible: true }),
    ).toEqual(expect.stringContaining('opacity-100'));
    expect(
      getCaptureRootClassName('selecting', { isSurfaceVisible: true }),
    ).toEqual(expect.stringContaining('transition-opacity'));
    expect(
      getCaptureRootClassName('selecting', { isSurfaceVisible: true }),
    ).toEqual(expect.stringContaining('duration-[120ms]'));
    expect(
      getCaptureRootClassName('selecting', { isSurfaceVisible: true }),
    ).toEqual(expect.stringContaining('ease-linear'));
    expect(
      getCaptureRootClassName('selecting', { isSurfaceVisible: true }),
    ).toEqual(expect.stringContaining('will-change-[opacity]'));
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

  it('uses blue selection and primary editor affordances', () => {
    expect(getCaptureEditorSelectionClassName('preview')).toEqual(
      expect.stringContaining('border-[#5b7fff]'),
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
