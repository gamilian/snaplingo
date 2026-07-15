import { describe, expect, it } from 'vitest';
import {
  autosizeResultWindowTextArea,
  measureResultWindowTextMirrorHeight,
  resultWindowContentClassName,
  resultWindowAdaptiveTextStyle,
  resultWindowContainerClassName,
  resultWindowPanelClassName,
  resultWindowOcrContentClassName,
  resultWindowOcrFullTextBoxClassName,
  resultWindowOcrImageActionButtonClassName,
  resultWindowOcrImagePanelClassName,
  resultWindowOcrPanelHeight,
  resultWindowOcrResultStackClassName,
  resultWindowOcrResultTextAreaClassName,
  resultWindowOcrTokenButtonClassName,
  resultWindowHeaderDragHandleClassName,
  resultWindowPinButtonClassName,
  resultWindowResultsSectionClassName,
  resultWindowResultsListClassName,
  resultWindowStandaloneWindowHeight,
  resultWindowTranslationMeasuredPanelHeight,
  resultWindowTranslationSubtitle,
  resultWindowTextAreaClassName,
  resultWindowTextAreaMinHeightPx,
  resultWindowTextAreaRows,
  resultWindowTextBoxClassName,
  resultWindowTextMirrorContent,
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

  it('can align standalone content to its natural height for compact OCR windows', () => {
    expect(resultWindowContainerClassName('standalone', { fitContent: true })).toContain(
      'items-start',
    );
    expect(resultWindowContainerClassName('standalone', { fitContent: true })).not.toContain(
      'items-stretch',
    );
    expect(resultWindowPanelClassName('standalone', { fitContent: true })).not.toContain(
      'h-full',
    );
    expect(resultWindowPanelClassName('standalone', { fitContent: true })).toContain(
      'max-h-[calc(100vh-1rem)]',
    );
  });

  it('keeps the dark overlay only for the main-window embedded presentation', () => {
    expect(resultWindowContainerClassName('overlay')).toContain('bg-black/25');
  });

  it('marks the header blank area as draggable when native dragging is available', () => {
    expect(resultWindowHeaderDragHandleClassName(true)).toContain('cursor-move');
    expect(resultWindowHeaderDragHandleClassName(true)).toContain('select-none');
    expect(resultWindowHeaderDragHandleClassName(false)).toContain('cursor-default');
  });

  it('styles the Bob-style pin control by pinned state', () => {
    expect(resultWindowPinButtonClassName(false)).toContain('text-slate-500');
    expect(resultWindowPinButtonClassName(true)).toContain('bg-blue-50');
    expect(resultWindowPinButtonClassName(true)).toContain('text-blue-600');
  });

  it('uses the compact pro rounded shell for both result window presentations', () => {
    expect(resultWindowPanelClassName('overlay')).toContain('rounded-[14px]');
    expect(resultWindowPanelClassName('standalone')).toContain('rounded-[14px]');
    expect(resultWindowPanelClassName('overlay')).toContain('max-w-[660px]');
    expect(resultWindowPanelClassName('standalone')).not.toContain('max-w-[660px]');
    expect(resultWindowPanelClassName('overlay')).toContain(
      '[clip-path:inset(0_round_14px)]',
    );
    expect(resultWindowPanelClassName('standalone')).toContain(
      '[clip-path:inset(0_round_14px)]',
    );
    expect(resultWindowPanelClassName('overlay')).toContain('overflow-hidden');
    expect(resultWindowPanelClassName('standalone')).toContain('overflow-hidden');
  });

  it('uses one overall content scrollbar inside the result window with the larger 12px spacing rhythm', () => {
    const className = resultWindowContentClassName();

    expect(className).toContain('overflow-y-auto');
    expect(className).toContain('result-window-scrollbar');
    expect(className).toContain('gap-3');
    expect(className).toContain('pl-3');
    expect(className).toContain('pr-3');
    expect(className).toContain('pb-3');
    expect(className).toContain('pt-3');
    expect(className).not.toContain('gap-2.5');
    expect(className).not.toContain('pl-2.5');
    expect(className).not.toContain('pr-2.5');
    expect(className).not.toContain('px-3');
  });

  it('can delegate bottom spacing to the final content block', () => {
    const className = resultWindowContentClassName({ reserveBottom: false });

    expect(className).toContain('pb-0');
    expect(className).not.toContain('pb-3');
  });

  it('can avoid stretching compact OCR content to the full standalone window height', () => {
    const className = resultWindowContentClassName({ stretch: false });

    expect(className).toContain('flex-none');
    expect(className).not.toContain('flex-1');
  });

  it('lets OCR content with a source image scroll inside a constrained result window', () => {
    const className = resultWindowOcrContentClassName();

    expect(className).toContain('flex-1');
    expect(className).toContain('overflow-y-auto');
    expect(className).toContain('result-window-scrollbar');
    expect(className).not.toContain('flex-none');
  });

  it('lets OCR upload-only content use the same overall scrollbar', () => {
    const className = resultWindowOcrContentClassName();

    expect(className).toContain('flex-1');
    expect(className).toContain('overflow-y-auto');
    expect(className).toContain('result-window-scrollbar');
    expect(className).not.toContain('flex-none');
  });

  it('caps the OCR panel below the available screen height so its overall scrollbar remains usable', () => {
    expect(resultWindowOcrPanelHeight(1400, 1000)).toBe(968);
    expect(resultWindowOcrPanelHeight(600, 1000)).toBe(600);
  });

  it('uses expanding text boxes without individual scrollbars', () => {
    expect(resultWindowTextBoxClassName()).toContain('rounded-[14px]');
    expect(resultWindowTextBoxClassName()).toContain('overflow-hidden');

    expect(resultWindowTextAreaClassName('source')).toContain('text-[13px]');
    expect(resultWindowTextAreaClassName('source')).toContain('leading-[1.38]');
    expect(resultWindowTextAreaClassName('source')).not.toContain('max-h-');
    expect(resultWindowTextAreaClassName('ocr')).not.toContain('max-h-');
    expect(resultWindowTextAreaClassName('source')).not.toContain('overflow-y-auto');
    expect(resultWindowTextAreaClassName('ocr')).not.toContain('overflow-y-auto');
    expect(resultWindowTextAreaClassName('source')).not.toContain(
      'result-window-scrollbar',
    );
  });

  it('keeps textarea rows minimal so autosize can measure without blank space', () => {
    expect(resultWindowTextAreaRows('Short text', 'source')).toBe(2);
    expect(resultWindowTextAreaRows('A long source sentence '.repeat(12), 'source')).toBe(2);
    expect(resultWindowTextAreaRows('OCR text', 'ocr')).toBe(4);
    expect(resultWindowTextAreaRows('OCR text '.repeat(60), 'ocr')).toBe(4);
    expect(resultWindowTextAreaMinHeightPx('source')).toBe(52);
    expect(resultWindowTextAreaMinHeightPx('ocr')).toBe(72);
  });

  it('keeps minimum-height textareas compact when autosizing', () => {
    const textArea = {
      scrollHeight: 52,
      style: { height: '320px' },
    } as HTMLTextAreaElement;

    const measuredHeight = autosizeResultWindowTextArea(textArea, {
      minHeightPx: 52,
      textStyle: { fontSize: '14px', lineHeight: 1.42 },
    });

    expect(textArea.style.height).toBe('52px');
    expect(measuredHeight).toBe(52);
  });

  it('adds line-height slack for overflowing textareas to avoid clipping', () => {
    const textArea = {
      scrollHeight: 184,
      style: { height: '320px' },
    } as HTMLTextAreaElement;

    const measuredHeight = autosizeResultWindowTextArea(textArea, {
      minHeightPx: 52,
      textStyle: { fontSize: '12px', lineHeight: 1.28 },
    });

    expect(textArea.style.height).toBe('196px');
    expect(measuredHeight).toBe(196);
  });

  it('can use a tighter autosize slack for wrapped source text', () => {
    const textArea = {
      scrollHeight: 184,
      style: { height: '320px' },
    } as HTMLTextAreaElement;

    const measuredHeight = autosizeResultWindowTextArea(textArea, {
      minHeightPx: 52,
      textStyle: { fontSize: '12px', lineHeight: 1.28 },
      slackPx: 4,
    });

    expect(textArea.style.height).toBe('188px');
    expect(measuredHeight).toBe(188);
  });

  it('measures mirror-based source text height without adding extra slack', () => {
    const mirror = {
      offsetHeight: 184,
    } as HTMLDivElement;

    const measuredHeight = measureResultWindowTextMirrorHeight(mirror, 52);

    expect(measuredHeight).toBe(184);
  });

  it('adds a sentinel to mirror content so trailing newlines still measure correctly', () => {
    expect(resultWindowTextMirrorContent('line 1\nline 2')).toBe('line 1\nline 2\u200b');
    expect(resultWindowTextMirrorContent('line 1\n')).toBe('line 1\n\u200b');
    expect(resultWindowTextMirrorContent('')).toBe('\u200b');
  });

  it('keeps source and result line height fixed while adapting font size to content density', () => {
    expect(resultWindowAdaptiveTextStyle('Short text', 'source')).toEqual({
      fontSize: '14px',
      lineHeight: 1.38,
    });
    expect(
      resultWindowAdaptiveTextStyle('A moderately long source text '.repeat(4), 'source'),
    ).toEqual({
      fontSize: '13px',
      lineHeight: 1.38,
    });
    expect(
      resultWindowAdaptiveTextStyle('A very long translation result '.repeat(24), 'result'),
    ).toEqual({
      fontSize: '12px',
      lineHeight: 1.38,
    });
  });

  it('keeps provider result lists out of individual scrolling', () => {
    expect(resultWindowResultsSectionClassName()).toContain('flex-none');
    expect(resultWindowResultsSectionClassName()).not.toContain('flex-1');

    const className = resultWindowResultsListClassName();

    expect(className).toContain('min-h-0');
    expect(className).toContain('space-y-3');
    expect(className).toContain('pb-3');
    expect(className).not.toContain('space-y-2.5');
    expect(className).not.toContain('overflow-hidden');
    expect(className).not.toContain('overflow-y-auto');
  });

  it('reports returned providers against the active provider total', () => {
    expect(
      resultWindowTranslationSubtitle(
        [
          {
            provider_id: 'google',
            status: 'success',
            translated_text: '你好',
            detected_language: 'en',
            confidence: 1,
          },
          {
            provider_id: 'openai',
            status: 'pending',
            translated_text: '',
            detected_language: null,
            confidence: null,
          },
          {
            provider_id: 'deeplx',
            status: 'pending',
            translated_text: '',
            detected_language: null,
            confidence: null,
          },
        ],
        true,
      ),
    ).toBe('1/3 个服务已返回');
  });

  it('sizes short provider results naturally without forcing inner scrolling', () => {
    const layout = resultWindowTranslationLayout([
      { providerId: 'google', text: '短结果' },
      { providerId: 'openai', text: 'Short result' },
    ]);

    expect(layout.isConstrained).toBe(false);
    expect(layout.bodyHeightByProviderId.google).toBeLessThan(80);
    expect(layout.bodyHeightByProviderId.openai).toBeLessThan(80);
    expect(layout.resultsHeightPx).toBe(188);
    expect(layout.windowHeightPx).toBe(392);
  });

  it('does not add minimum-window filler below a single short provider result', () => {
    const layout = resultWindowTranslationLayout([
      { providerId: 'google', text: '短结果' },
    ]);

    expect(layout.resultsHeightPx).toBe(94);
    expect(layout.windowHeightPx).toBe(298);
  });

  it('can size the translation panel from measured DOM heights', () => {
    expect(
      resultWindowTranslationMeasuredPanelHeight({
        sourceBoxHeightPx: 86,
        languageSwitcherHeightPx: 40,
        resultsListHeightPx: 318,
        hasResults: true,
      }),
    ).toBe(528);
  });

  it('raises the translation window height from the measured source text box height', () => {
    const providers = [
      { providerId: 'google', text: '短结果' },
      { providerId: 'openai', text: 'Short result' },
    ];
    const shortLayout = resultWindowTranslationLayout(providers, 52);
    const longLayout = resultWindowTranslationLayout(
      providers,
      220,
    );

    expect(longLayout.windowHeightPx).toBeGreaterThan(shortLayout.windowHeightPx);
  });

  it('keeps long provider results at natural height and caps only the window', () => {
    const layout = resultWindowTranslationLayout([
      { providerId: 'google', text: 'Long result '.repeat(120) },
      { providerId: 'openai', text: 'Long result '.repeat(120) },
      { providerId: 'deeplx', text: 'Long result '.repeat(120) },
      { providerId: 'baidu', text: 'Long result '.repeat(120) },
    ]);

    expect(layout.isConstrained).toBe(true);
    expect(layout.resultsHeightPx).toBeGreaterThan(560);
    expect(layout.windowHeightPx).toBeLessThanOrEqual(820);
    expect(layout.bodyHeightByProviderId.google).toBeGreaterThan(300);
  });

  it('does not add extra result height for trailing blank lines', () => {
    const cleanLayout = resultWindowTranslationLayout([
      { providerId: 'google', text: 'Short result' },
    ]);
    const trailingNewlineLayout = resultWindowTranslationLayout([
      { providerId: 'google', text: 'Short result\n' },
    ]);

    expect(trailingNewlineLayout.bodyHeightByProviderId.google).toBe(
      cleanLayout.bodyHeightByProviderId.google,
    );
    expect(trailingNewlineLayout.resultsHeightPx).toBe(cleanLayout.resultsHeightPx);
    expect(trailingNewlineLayout.windowHeightPx).toBe(cleanLayout.windowHeightPx);
  });

  it('adds standalone container padding to the native window height', () => {
    expect(resultWindowStandaloneWindowHeight(794)).toBe(810);
  });

  it('allocates pending provider body height and bottom breathing room', () => {
    const layout = resultWindowTranslationLayout([
      { providerId: 'google', text: '短结果', status: 'success' },
      { providerId: 'openai', text: '', status: 'pending' },
      { providerId: 'deeplx', text: '', status: 'pending' },
    ]);

    expect(layout.bodyHeightByProviderId.openai).toBeGreaterThanOrEqual(62);
    expect(layout.bodyHeightByProviderId.deeplx).toBeGreaterThanOrEqual(62);
    expect(layout.resultsHeightPx).toBeGreaterThan(
      3 * 36 + 2 * 8 + 62 + 62,
    );
  });

  it('uses a compact vertical OCR result layout when a source image is available', () => {
    expect(resultWindowOcrResultStackClassName()).toContain('flex-col');
    expect(resultWindowOcrResultStackClassName()).toContain('gap-3');
    expect(resultWindowOcrResultStackClassName()).not.toContain('grid-cols-');
    expect(resultWindowOcrResultStackClassName()).not.toContain('gap-2.5');

    expect(resultWindowOcrImagePanelClassName()).toContain('rounded-[14px]');
    expect(resultWindowOcrImagePanelClassName()).toContain('overflow-hidden');
    expect(resultWindowOcrImagePanelClassName()).not.toContain('min-h-0');

    expect(resultWindowOcrImageActionButtonClassName()).toContain('h-8');
    expect(resultWindowOcrImageActionButtonClassName()).toContain('rounded-[10px]');

    expect(resultWindowOcrFullTextBoxClassName()).not.toContain('flex-1');
    expect(resultWindowOcrFullTextBoxClassName()).toContain('rounded-[14px]');

    expect(resultWindowOcrResultTextAreaClassName()).not.toContain('overflow-y-auto');
    expect(resultWindowOcrResultTextAreaClassName()).not.toContain('flex-1');
    expect(resultWindowOcrResultTextAreaClassName()).toContain('text-[14px]');
    expect(resultWindowOcrResultTextAreaClassName()).toContain('leading-[1.42]');
    expect(resultWindowOcrResultTextAreaClassName()).not.toContain(
      'result-window-scrollbar',
    );

    expect(resultWindowOcrTokenButtonClassName()).toContain('rounded-full');
    expect(resultWindowOcrTokenButtonClassName()).toContain('break-all');
  });

  it('closes from background clicks in overlay and standalone presentations', () => {
    const target = new EventTarget();

    expect(shouldCloseFromContainerClick('overlay', target, target)).toBe(true);
    expect(shouldCloseFromContainerClick('standalone', target, target)).toBe(
      true,
    );
  });

  it('does not close from background clicks when pinned', () => {
    const target = new EventTarget();

    expect(shouldCloseFromContainerClick('overlay', target, target, true)).toBe(false);
    expect(shouldCloseFromContainerClick('standalone', target, target, true)).toBe(false);
  });

  it('closes unpinned standalone result windows after the initial blur grace period', () => {
    expect(shouldCloseFromWindowBlur('standalone', false, 100)).toBe(false);
    expect(shouldCloseFromWindowBlur('standalone', false, 400)).toBe(true);
    expect(shouldCloseFromWindowBlur('overlay')).toBe(false);
    expect(shouldCloseFromWindowBlur('standalone', true)).toBe(false);
  });

  it('closes result windows from Escape but not other keys', () => {
    expect(shouldCloseFromEscapeKey('Escape')).toBe(true);
    expect(shouldCloseFromEscapeKey('Esc')).toBe(true);
    expect(shouldCloseFromEscapeKey('Enter')).toBe(false);
  });
});
