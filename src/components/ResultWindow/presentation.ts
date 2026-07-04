import type { ProviderTranslation } from '../../stores/appStore';

export type ResultWindowPresentation = 'overlay' | 'standalone';
export type ResultWindowTextKind = 'source' | 'ocr' | 'result';

const resultWindowSurfaceRadiusClassName = 'rounded-[14px]';
const resultWindowSurfaceClipClassName = '[clip-path:inset(0_round_14px)]';
const translationResultsMaxHeightPx = 560;
const translationWindowMaxHeightPx = 820;
const translationWindowMinHeightPx = 320;
const translationWindowStaticHeightPx = 236;
const translationProviderCardHeaderHeightPx = 36;
const translationProviderCardGapPx = 8;
const translationProviderBodyMinHeightPx = 44;
const translationProviderPendingBodyHeightPx = 62;
const translationProviderBodyVerticalPaddingPx = 16;
const translationProviderBodyLineHeightPx = 18;
const translationResultsBottomPaddingPx = 8;
const resultWindowStandaloneContainerPaddingPx = 16;

export interface ResultWindowAdaptiveTextStyle {
  fontSize: string;
  lineHeight: number;
}

export interface ResultWindowTranslationLayoutItem {
  providerId: string;
  text: string;
  status?: ProviderTranslation['status'];
}

export interface ResultWindowTranslationLayout {
  bodyHeightByProviderId: Record<string, number>;
  isConstrained: boolean;
  resultsHeightPx: number;
  windowHeightPx: number;
}

export function resultWindowContainerClassName(
  presentation: ResultWindowPresentation,
) {
  if (presentation === 'standalone') {
    return 'h-screen overflow-hidden bg-transparent flex items-stretch justify-center p-2';
  }

  return 'fixed inset-0 bg-black/25 backdrop-blur-sm flex items-center justify-center z-50 p-8';
}

export function resultWindowPanelClassName(
  presentation: ResultWindowPresentation,
) {
  const baseClassName =
    `min-h-0 bg-white ${resultWindowSurfaceRadiusClassName} shadow-xl w-full max-w-[660px] overflow-hidden ${resultWindowSurfaceClipClassName} flex flex-col`;

  if (presentation === 'standalone') {
    return `${baseClassName} h-full max-h-[calc(100vh-1rem)]`;
  }

  return `${baseClassName} max-h-[90vh] animate-[slideIn_0.3s_ease-out]`;
}

export function resultWindowContentClassName() {
  return 'flex min-h-0 flex-1 flex-col gap-2.5 overflow-y-auto overflow-x-hidden px-3 py-2.5 result-window-scrollbar';
}

export function resultWindowTextBoxClassName() {
  return `overflow-hidden ${resultWindowSurfaceRadiusClassName} border border-slate-300 bg-white`;
}

export function resultWindowTextAreaClassName(kind: 'source' | 'ocr') {
  const baseClassName =
    'w-full resize-none overflow-hidden border-0 outline-none placeholder:text-slate-400';

  if (kind === 'ocr') {
    return `${baseClassName} min-h-[86px] px-3 py-2.5 text-[14px] leading-[1.42] text-slate-800`;
  }

  return `${baseClassName} min-h-[68px] px-3 py-2 text-[13px] leading-[1.38] text-slate-900`;
}

export function resultWindowTextAreaRows(
  text: string,
  kind: 'source' | 'ocr',
) {
  const minRows = kind === 'source' ? 3 : 5;
  return Math.max(minRows, estimateVisualLineCount(text, kind));
}

export function autosizeResultWindowTextArea(
  textArea: Pick<HTMLTextAreaElement, 'scrollHeight' | 'style'> | null,
) {
  if (!textArea) return;

  textArea.style.height = 'auto';
  textArea.style.height = `${textArea.scrollHeight}px`;
}

export function resultWindowAdaptiveTextStyle(
  text: string,
  kind: ResultWindowTextKind,
): ResultWindowAdaptiveTextStyle {
  const charCount = Array.from(text.trim()).length;
  const visualLineCount = estimateVisualLineCount(text, kind);
  const shortLineLimit = kind === 'result' ? 3 : 2;
  const mediumCharLimit = kind === 'result' ? 180 : 140;
  const density = Math.max(
    charCount / mediumCharLimit,
    visualLineCount / shortLineLimit,
  );

  if (density >= 2.2) {
    return { fontSize: '12px', lineHeight: 1.28 };
  }

  if (density >= 1.15) {
    return { fontSize: '13px', lineHeight: kind === 'source' ? 1.34 : 1.36 };
  }

  return { fontSize: '14px', lineHeight: kind === 'source' ? 1.42 : 1.44 };
}

export function resultWindowTranslationLayout(
  providers: ResultWindowTranslationLayoutItem[],
): ResultWindowTranslationLayout {
  if (providers.length === 0) {
    return {
      bodyHeightByProviderId: {},
      isConstrained: false,
      resultsHeightPx: 0,
      windowHeightPx: translationWindowMinHeightPx,
    };
  }

  const bodyHeights = providers.map((provider) =>
    estimateTranslationBodyHeight(provider.text, provider.status),
  );
  const naturalResultsHeightPx =
    bodyHeights.reduce(
      (height, bodyHeight) =>
        height + translationProviderCardHeaderHeightPx + bodyHeight,
      0,
    ) +
    translationProviderCardGapPx * Math.max(0, providers.length - 1) +
    translationResultsBottomPaddingPx;

  return {
    bodyHeightByProviderId: Object.fromEntries(
      providers.map((provider, index) => [provider.providerId, bodyHeights[index]]),
    ),
    isConstrained: naturalResultsHeightPx > translationResultsMaxHeightPx,
    resultsHeightPx: naturalResultsHeightPx,
    windowHeightPx: clampTranslationWindowHeight(naturalResultsHeightPx),
  };
}

function estimateVisualLineCount(text: string, kind: ResultWindowTextKind) {
  const charsPerLine = kind === 'result' ? 72 : 54;
  const lines = text.length > 0 ? text.split(/\r\n|\r|\n/) : [''];

  return lines.reduce((lineCount, line) => {
    const charCount = Array.from(line).length;
    return lineCount + Math.max(1, Math.ceil(charCount / charsPerLine));
  }, 0);
}

function estimateTranslationBodyHeight(
  text: string,
  status?: ProviderTranslation['status'],
) {
  if (status === 'pending') {
    return translationProviderPendingBodyHeightPx;
  }

  const visualLineCount = estimateVisualLineCount(text, 'result');

  return Math.max(
    translationProviderBodyMinHeightPx,
    visualLineCount * translationProviderBodyLineHeightPx +
      translationProviderBodyVerticalPaddingPx,
  );
}

function clampTranslationWindowHeight(resultsHeightPx: number) {
  return Math.min(
    translationWindowMaxHeightPx,
    Math.max(
      translationWindowMinHeightPx,
      translationWindowStaticHeightPx + resultsHeightPx,
    ),
  );
}

export function resultWindowResultsListClassName() {
  return 'min-h-0 flex-none space-y-2 pb-2';
}

export function resultWindowStandaloneWindowHeight(panelHeightPx: number) {
  return panelHeightPx + resultWindowStandaloneContainerPaddingPx;
}

export function resultWindowTranslationSubtitle(
  providerTranslations: ProviderTranslation[],
  isTranslating: boolean,
) {
  if (providerTranslations.length > 0) {
    const successCount = providerTranslations.filter(
      (translation) => translation.status === 'success',
    ).length;
    return `${successCount}/${providerTranslations.length} 个服务已返回`;
  }

  return isTranslating ? '正在请求服务' : '准备翻译';
}

export function resultWindowOcrResultGridClassName() {
  return 'grid min-h-0 flex-1 grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)] gap-3';
}

export function resultWindowOcrImagePanelClassName() {
  return `min-h-0 overflow-hidden ${resultWindowSurfaceRadiusClassName} border border-slate-200 bg-slate-50`;
}

export function resultWindowOcrResultTextAreaClassName() {
  return 'min-h-[86px] w-full resize-none overflow-hidden border-0 px-3 py-2.5 text-[14px] leading-[1.42] text-slate-800 outline-none placeholder:text-slate-400';
}

export function shouldCloseFromContainerClick(
  presentation: ResultWindowPresentation,
  target: EventTarget,
  currentTarget: EventTarget,
) {
  return (presentation === 'overlay' || presentation === 'standalone') && target === currentTarget;
}

export function shouldCloseFromWindowBlur(presentation: ResultWindowPresentation) {
  return presentation === 'standalone';
}

export function shouldCloseFromEscapeKey(key: string) {
  return key === 'Escape';
}
