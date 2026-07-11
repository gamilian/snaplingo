import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import {
  getCaptureEditorCommandButtonClassName,
  getCaptureEditorIconButtonClassName,
  getCaptureEditorSelectionClassName,
  getCaptureSelectionOverlayCanvasClassName,
  getCaptureEditorToolbarClassName,
  getCaptureRootCursorStyle,
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

  it('keeps the capture result html shell transparent before React renders', () => {
    const html = readFileSync(new URL('../../../index.html', import.meta.url), 'utf8');

    expect(html).toContain('data-window="capture-result"');
    expect(html).toMatch(
      /html\[data-window="capture-result"\][\s\S]*background:\s*transparent/s,
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

  it('hides the system cursor while the canvas crosshair is active', () => {
    expect(getCaptureRootCursorStyle('selecting')).toBe('none');
    expect(getCaptureRootCursorStyle('preview')).toBe('crosshair');
  });

  it('keeps the selection overlay canvas visible on the first revealed frame', () => {
    const className = getCaptureSelectionOverlayCanvasClassName();

    expect(className).not.toContain('opacity-0');
    expect(className).not.toContain('transition-opacity');
  });

  it('uses a light floating toolbar for the editing surface', () => {
    const className = getCaptureEditorToolbarClassName();

    expect(className).toContain('h-[42px]');
    expect(className).toContain('gap-1');
    expect(className).toContain('rounded-[12px]');
    expect(className).toContain('bg-white/95');
    expect(className).toContain('text-slate-600');
  });

  it('uses compact editor controls for small capture selections', () => {
    expect(getCaptureEditorIconButtonClassName()).toContain('h-7 w-7');
    expect(getCaptureEditorCommandButtonClassName()).toContain('h-7');
    expect(getCaptureEditorCommandButtonClassName()).toContain('text-[11px]');
    expect(getCaptureEditorCommandButtonClassName('primary')).toContain(
      'w-7',
    );
  });

  it('uses short text for escape and OCR while keeping output actions as SVG icons', () => {
    const toolbar = readFileSync(
      new URL('./captureEditorToolbar.tsx', import.meta.url),
      'utf8',
    );

    expect(toolbar).toMatch(/>\s*ESC\s*</);
    expect(toolbar).toMatch(/>\s*OCR\s*</);
    expect(toolbar).not.toContain('取消 (Esc)');
    expect(toolbar).not.toContain('完成 (Enter)');
    expect(toolbar).toContain('<CopyIcon />');
    expect(toolbar).toContain('<SaveIcon />');
    expect(toolbar).toContain('<CheckIcon />');
    expect(toolbar).toMatch(
      /<button(?:(?!<\/button>)[\s\S])*className=\{getCaptureEditorCommandButtonClassName\(\)\}(?:(?!<\/button>)[\s\S])*title="OCR"/,
    );
    expect(toolbar).toMatch(
      /<button(?:(?!<\/button>)[\s\S])*className=\{getCaptureEditorCommandButtonClassName\('icon'\)\}(?:(?!<\/button>)[\s\S])*title="Copy/,
    );
  });

  it('keeps toolbar placement dimensions synchronized with the compact surface', () => {
    const runtimeView = readFileSync(
      new URL('./useCaptureWorkspaceRuntimeView.ts', import.meta.url),
      'utf8',
    );

    expect(runtimeView).toContain(
      'const TOOLBAR_SIZE = { width: 640, height: 42 };',
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
      expect.stringContaining('cursor-crosshair'),
    );
    expect(getCaptureEditorSelectionClassName('preview')).not.toContain(
      'cursor-move',
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
