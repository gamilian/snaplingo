import { describe, expect, it } from 'vitest';
import {
  resultWindowContentClassName,
  resultWindowAdaptiveTextStyle,
  resultWindowContainerClassName,
  resultWindowPanelClassName,
  resultWindowOcrImagePanelClassName,
  resultWindowOcrResultGridClassName,
  resultWindowOcrResultTextAreaClassName,
  resultWindowResultsListClassName,
  resultWindowTextAreaClassName,
  resultWindowTextBoxClassName,
  resultWindowTranslationLayout,
  shouldCloseFromContainerClick,
  shouldCloseFromEscapeKey,
  shouldCloseFromWindowBlur,
} from './presentation';

describe('result window presentation', () => {
  it('does not render a dark modal overlay in standalone result windows', () => {
    const className = resultWindowContainerClassName('standalone');

    expect(className).not.toContain('bg-black/25');
    expect(className).not.toContain('backdrop-blur');
  });

  it('keeps the standalone result window shell transparent and clipped', () => {
    const className = resultWindowContainerClassName('standalone');

    expect(className).toContain('bg-transparent');
    expect(className).toContain('overflow-hidden');
    expect(className).toContain('h-screen');
  });

  it('keeps the dark overlay only for the main-window embedded presentation', () => {
    expect(resultWindowContainerClassName('overlay')).toContain('bg-black/25');
  });

  it('uses the compact pro rounded shell for both result window presentations', () => {
    expect(resultWindowPanelClassName('overlay')).toContain('rounded-[14px]');
    expect(resultWindowPanelClassName('standalone')).toContain('rounded-[14px]');
    expect(resultWindowPanelClassName('overlay')).toContain('max-w-[660px]');
    expect(resultWindowPanelClassName('standalone')).toContain('max-w-[660px]');
    expect(resultWindowPanelClassName('overlay')).toContain(
      '[clip-path:inset(0_round_14px)]',
    );
    expect(resultWindowPanelClassName('standalone')).toContain(
      '[clip-path:inset(0_round_14px)]',
    );
    expect(resultWindowPanelClassName('overlay')).toContain('overflow-hidden');
    expect(resultWindowPanelClassName('standalone')).toContain('overflow-hidden');
  });

  it('keeps result window body scrolling out of the outer shell', () => {
    const className = resultWindowContentClassName();

    expect(className).toContain('overflow-hidden');
    expect(className).not.toContain('overflow-y-auto');
  });

  it('uses small rounded text boxes with internal text scrolling', () => {
    expect(resultWindowTextBoxClassName()).toContain('rounded-[14px]');
    expect(resultWindowTextBoxClassName()).toContain('overflow-hidden');

    expect(resultWindowTextAreaClassName('source')).toContain('max-h-[82px]');
    expect(resultWindowTextAreaClassName('source')).toContain('text-[13px]');
    expect(resultWindowTextAreaClassName('source')).toContain('leading-[1.38]');
    expect(resultWindowTextAreaClassName('ocr')).toContain('max-h-[132px]');
    expect(resultWindowTextAreaClassName('source')).toContain('overflow-y-auto');
    expect(resultWindowTextAreaClassName('ocr')).toContain('overflow-y-auto');
    expect(resultWindowTextAreaClassName('source')).toContain(
      'result-window-scrollbar',
    );
  });

  it('adapts text size and line height to content density', () => {
    expect(resultWindowAdaptiveTextStyle('Short text', 'source')).toEqual({
      fontSize: '14px',
      lineHeight: 1.42,
    });
    expect(
      resultWindowAdaptiveTextStyle('A moderately long source text '.repeat(4), 'source'),
    ).toEqual({
      fontSize: '13px',
      lineHeight: 1.34,
    });
    expect(
      resultWindowAdaptiveTextStyle('A very long translation result '.repeat(24), 'result'),
    ).toEqual({
      fontSize: '12px',
      lineHeight: 1.28,
    });
  });

  it('keeps every provider card visible by preventing result-list scrolling', () => {
    const className = resultWindowResultsListClassName();

    expect(className).toContain('min-h-0');
    expect(className).toContain('overflow-hidden');
    expect(className).not.toContain('overflow-y-auto');
  });

  it('sizes short provider results naturally without forcing inner scrolling', () => {
    const layout = resultWindowTranslationLayout([
      { providerId: 'google', text: '短结果' },
      { providerId: 'openai', text: 'Short result' },
    ]);

    expect(layout.isConstrained).toBe(false);
    expect(layout.bodyMaxHeightByProviderId.google).toBeLessThan(80);
    expect(layout.bodyMaxHeightByProviderId.openai).toBeLessThan(80);
    expect(layout.resultsHeightPx).toBeLessThan(180);
    expect(layout.windowHeightPx).toBeLessThan(480);
  });

  it('splits the maximum result area across many long provider results', () => {
    const layout = resultWindowTranslationLayout([
      { providerId: 'google', text: 'Long result '.repeat(120) },
      { providerId: 'openai', text: 'Long result '.repeat(120) },
      { providerId: 'deeplx', text: 'Long result '.repeat(120) },
      { providerId: 'baidu', text: 'Long result '.repeat(120) },
    ]);

    expect(layout.isConstrained).toBe(true);
    expect(layout.resultsHeightPx).toBeLessThanOrEqual(560);
    expect(layout.windowHeightPx).toBeLessThanOrEqual(820);
    expect(layout.bodyMaxHeightByProviderId.google).toBe(
      layout.bodyMaxHeightByProviderId.openai,
    );
    expect(layout.bodyMaxHeightByProviderId.google).toBeGreaterThanOrEqual(90);
  });

  it('uses a two-panel OCR result layout when a source image is available', () => {
    expect(resultWindowOcrResultGridClassName()).toContain(
      'grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]',
    );
    expect(resultWindowOcrResultGridClassName()).toContain('min-h-0');

    expect(resultWindowOcrImagePanelClassName()).toContain('rounded-[14px]');
    expect(resultWindowOcrImagePanelClassName()).toContain('overflow-hidden');

    expect(resultWindowOcrResultTextAreaClassName()).toContain('overflow-y-auto');
    expect(resultWindowOcrResultTextAreaClassName()).toContain('text-[14px]');
    expect(resultWindowOcrResultTextAreaClassName()).toContain('leading-[1.42]');
    expect(resultWindowOcrResultTextAreaClassName()).toContain(
      'result-window-scrollbar',
    );
  });

  it('closes from background clicks in overlay and standalone presentations', () => {
    const target = new EventTarget();

    expect(shouldCloseFromContainerClick('overlay', target, target)).toBe(true);
    expect(shouldCloseFromContainerClick('standalone', target, target)).toBe(
      true,
    );
  });

  it('closes standalone result windows when focus leaves the window', () => {
    expect(shouldCloseFromWindowBlur('standalone')).toBe(true);
    expect(shouldCloseFromWindowBlur('overlay')).toBe(false);
  });

  it('closes result windows from Escape but not other keys', () => {
    expect(shouldCloseFromEscapeKey('Escape')).toBe(true);
    expect(shouldCloseFromEscapeKey('Enter')).toBe(false);
  });
});
