import type { ProviderTranslation } from '../../stores/appStore';

export type ResultWindowPresentation = 'overlay' | 'standalone';
export type ResultWindowTextKind = 'source' | 'ocr' | 'result';

const resultWindowSurfaceRadiusClassName = 'rounded-[14px]';
const resultWindowSurfaceClipClassName = '[clip-path:inset(0_round_14px)]';
const translationResultsMaxHeightPx = 560;
const translationWindowMaxHeightPx = 820;
const translationWindowStaticHeightPx = 204;
const translationHeaderHeightPx = 48;
const translationContentTopPaddingPx = 12;
const translationContentGapPx = 12;
const translationContentBottomPaddingPx = 12;
const translationProviderCardHeaderHeightPx = 36;
const translationProviderCardBorderHeightPx = 2;
const translationProviderCardGapPx = 12;
const translationProviderBodyMinHeightPx = 44;
const translationProviderPendingBodyHeightPx = 62;
const translationProviderBodyVerticalPaddingPx = 16;
const translationProviderBodyLineHeightPx = 18;
const translationResultsBottomPaddingPx = 12;
const resultWindowSourceTextAreaMinRows = 2;
const resultWindowOcrTextAreaMinRows = 4;
const resultWindowSourceTextAreaMinHeightPx = 52;
const resultWindowOcrTextAreaMinHeightPx = 72;
const resultWindowTextAreaSlackRatio = 0.75;
const resultWindowTextAreaMinSlackPx = 10;
const resultWindowTextAreaMaxSlackPx = 16;
const resultWindowStandaloneContainerPaddingPx = 16;
const resultWindowTranslationFixedLineHeight = 1.38;
const resultWindowInitialBlurGraceMs = 300;

export interface ResultWindowAdaptiveTextStyle {
  fontSize: string;
  lineHeight: number;
}

export interface ResultWindowTextAreaAutosizeOptions {
  minHeightPx?: number;
  slackPx?: number;
  textStyle?: ResultWindowAdaptiveTextStyle;
}

export interface ResultWindowTextMirrorMeasurement {
  offsetHeight: number;
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

export interface ResultWindowTranslationMeasuredPanelHeightOptions {
  sourceBoxHeightPx: number;
  languageSwitcherHeightPx: number;
  resultsListHeightPx?: number;
  hasResults: boolean;
}

export interface ResultWindowShellOptions {
  fitContent?: boolean;
}

export function resultWindowContainerClassName(
  presentation: ResultWindowPresentation,
  { fitContent = false }: ResultWindowShellOptions = {},
) {
  if (presentation === 'standalone') {
    const alignmentClassName = fitContent ? 'items-start' : 'items-stretch';

    return `h-screen overflow-hidden bg-transparent flex ${alignmentClassName} justify-center p-2`;
  }

  return 'fixed inset-0 bg-black/25 backdrop-blur-sm flex items-center justify-center z-50 p-8';
}

export function resultWindowPanelClassName(
  presentation: ResultWindowPresentation,
  { fitContent = false }: ResultWindowShellOptions = {},
) {
  const baseClassName =
    `relative min-h-0 bg-white ${resultWindowSurfaceRadiusClassName} shadow-xl w-full max-w-[660px] overflow-hidden ${resultWindowSurfaceClipClassName} flex flex-col`;

  if (presentation === 'standalone') {
    if (fitContent) {
      return `${baseClassName} max-h-[calc(100vh-1rem)]`;
    }

    return `${baseClassName} h-full max-h-[calc(100vh-1rem)]`;
  }

  return `${baseClassName} max-h-[90vh] animate-[slideIn_0.3s_ease-out]`;
}

export function resultWindowContentClassName({
  reserveBottom = true,
  stretch = true,
}: { reserveBottom?: boolean; stretch?: boolean } = {}) {
  const bottomPaddingClassName = reserveBottom ? 'pb-3' : 'pb-0';
  const heightClassName = stretch ? 'flex-1' : 'flex-none';

  return `flex min-h-0 ${heightClassName} flex-col gap-3 overflow-y-auto overflow-x-hidden pl-3 pr-3 ${bottomPaddingClassName} pt-3 result-window-scrollbar`;
}

export function resultWindowOcrContentClassName({
  hasSourceImage,
}: {
  hasSourceImage: boolean;
}) {
  return resultWindowContentClassName({ stretch: hasSourceImage });
}

export function resultWindowHeaderDragHandleClassName(isDraggable: boolean) {
  const dragClassName = isDraggable ? 'cursor-move' : 'cursor-default';

  return `flex min-w-0 flex-1 select-none items-center gap-3 ${dragClassName}`;
}

export function resultWindowPinButtonClassName(isPinned: boolean) {
  const pinnedClassName = isPinned
    ? 'bg-blue-50 text-blue-600 hover:bg-blue-100 hover:text-blue-700'
    : 'text-slate-500 hover:bg-slate-100 hover:text-slate-800';

  return `grid h-7 w-7 shrink-0 place-items-center rounded-[7px] transition-colors duration-150 ${pinnedClassName}`;
}

export function resultWindowTextBoxClassName() {
  return `overflow-hidden ${resultWindowSurfaceRadiusClassName} border border-slate-300 bg-white`;
}

export function resultWindowTextAreaClassName(kind: 'source' | 'ocr') {
  const baseClassName =
    'w-full resize-none overflow-hidden border-0 outline-none placeholder:text-slate-400';

  if (kind === 'ocr') {
    return `${baseClassName} min-h-[72px] px-3 py-3 text-[14px] leading-[1.42] text-slate-800`;
  }

  return `${baseClassName} min-h-[52px] px-3 py-2.5 text-[13px] leading-[1.38] text-slate-900`;
}

export function resultWindowTextAreaRows(
  text: string,
  kind: 'source' | 'ocr',
) {
  void text;

  return kind === 'source'
    ? resultWindowSourceTextAreaMinRows
    : resultWindowOcrTextAreaMinRows;
}

export function resultWindowTextAreaMinHeightPx(kind: 'source' | 'ocr') {
  return kind === 'source'
    ? resultWindowSourceTextAreaMinHeightPx
    : resultWindowOcrTextAreaMinHeightPx;
}

export function autosizeResultWindowTextArea(
  textArea: Pick<HTMLTextAreaElement, 'scrollHeight' | 'style'> | null,
  options: ResultWindowTextAreaAutosizeOptions = {},
) {
  if (!textArea) return null;

  textArea.style.height = 'auto';
  const minHeightPx = options.minHeightPx ?? 0;
  const baseHeight = Math.max(textArea.scrollHeight, minHeightPx);
  const shouldAddSlack = textArea.scrollHeight > minHeightPx;
  const measuredHeight =
    baseHeight +
    (shouldAddSlack
      ? options.slackPx ?? resultWindowTextAreaAutosizeSlackPx(options.textStyle)
      : 0);

  textArea.style.height = `${measuredHeight}px`;

  return measuredHeight;
}

export function resultWindowTextMirrorContent(text: string) {
  return `${text}\u200b`;
}

export function measureResultWindowTextMirrorHeight(
  mirror: ResultWindowTextMirrorMeasurement | null,
  minHeightPx = 0,
) {
  if (!mirror) return null;

  return Math.max(mirror.offsetHeight, minHeightPx);
}

export function resultWindowAdaptiveTextStyle(
  text: string,
  kind: ResultWindowTextKind,
): ResultWindowAdaptiveTextStyle {
  const measuredText = normalizeResultMeasurementText(text, kind);
  const charCount = Array.from(measuredText).length;
  const visualLineCount = estimateVisualLineCount(measuredText, kind);
  const shortLineLimit = kind === 'result' ? 3 : 2;
  const mediumCharLimit = kind === 'result' ? 180 : 140;
  const density = Math.max(
    charCount / mediumCharLimit,
    visualLineCount / shortLineLimit,
  );
  const lineHeight = resultWindowAdaptiveLineHeight(kind, density);

  if (density >= 2.2) {
    return { fontSize: '12px', lineHeight };
  }

  if (density >= 1.15) {
    return { fontSize: '13px', lineHeight };
  }

  return { fontSize: '14px', lineHeight };
}

export function resultWindowTranslationLayout(
  providers: ResultWindowTranslationLayoutItem[],
  sourceTextAreaHeightPx = resultWindowSourceTextAreaMinHeightPx,
): ResultWindowTranslationLayout {
  const sourceTextAreaExtraHeightPx = Math.max(
    0,
    sourceTextAreaHeightPx - resultWindowSourceTextAreaMinHeightPx,
  );

  if (providers.length === 0) {
    return {
      bodyHeightByProviderId: {},
      isConstrained: false,
      resultsHeightPx: 0,
      windowHeightPx: clampTranslationWindowHeight(
        0,
        sourceTextAreaExtraHeightPx,
      ),
    };
  }

  const bodyHeights = providers.map((provider) =>
    estimateTranslationBodyHeight(provider.text, provider.status),
  );
  const naturalResultsHeightPx =
    bodyHeights.reduce(
      (height, bodyHeight) =>
        height +
        translationProviderCardHeaderHeightPx +
        bodyHeight +
        translationProviderCardBorderHeightPx,
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
    windowHeightPx: clampTranslationWindowHeight(
      naturalResultsHeightPx,
      sourceTextAreaExtraHeightPx,
    ),
  };
}

function normalizeResultMeasurementText(
  text: string,
  kind: ResultWindowTextKind,
) {
  return kind === 'result' ? text.trim() : text;
}

function resultWindowAdaptiveLineHeight(
  kind: ResultWindowTextKind,
  density: number,
) {
  if (kind === 'source' || kind === 'result') {
    return resultWindowTranslationFixedLineHeight;
  }

  if (density >= 2.2) {
    return 1.28;
  }

  if (density >= 1.15) {
    return 1.36;
  }

  return 1.44;
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

  const visualLineCount = estimateVisualLineCount(
    normalizeResultMeasurementText(text, 'result'),
    'result',
  );

  return Math.max(
    translationProviderBodyMinHeightPx,
    visualLineCount * translationProviderBodyLineHeightPx +
      translationProviderBodyVerticalPaddingPx,
  );
}

function clampTranslationWindowHeight(
  resultsHeightPx: number,
  sourceTextAreaExtraHeightPx = 0,
) {
  return Math.min(
    translationWindowMaxHeightPx,
    translationWindowStaticHeightPx +
      sourceTextAreaExtraHeightPx +
      resultsHeightPx,
  );
}

export function resultWindowTranslationMeasuredPanelHeight({
  sourceBoxHeightPx,
  languageSwitcherHeightPx,
  resultsListHeightPx = 0,
  hasResults,
}: ResultWindowTranslationMeasuredPanelHeightOptions) {
  const measuredHeight =
    translationHeaderHeightPx +
    translationContentTopPaddingPx +
    sourceBoxHeightPx +
    translationContentGapPx +
    languageSwitcherHeightPx +
    (hasResults
      ? translationContentGapPx + resultsListHeightPx
      : translationContentBottomPaddingPx);

  return Math.min(translationWindowMaxHeightPx, Math.ceil(measuredHeight));
}

export function resultWindowResultsListClassName() {
  return 'min-h-0 flex-none space-y-3 pb-3';
}

export function resultWindowResultsSectionClassName() {
  return 'flex min-h-0 flex-none flex-col';
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

export function resultWindowOcrResultStackClassName() {
  return 'flex min-h-0 flex-col gap-3';
}

export function resultWindowOcrImagePanelClassName() {
  return `overflow-hidden ${resultWindowSurfaceRadiusClassName} border border-slate-200 bg-slate-50`;
}

export function resultWindowOcrImageActionButtonClassName() {
  return 'inline-flex h-8 w-full items-center justify-center gap-2 rounded-[10px] border border-emerald-200 bg-emerald-50 text-[13px] font-bold text-emerald-700 transition-colors duration-150 hover:border-emerald-300 hover:bg-emerald-100 disabled:cursor-not-allowed disabled:opacity-60';
}

export function resultWindowOcrFullTextBoxClassName() {
  return `flex min-h-0 flex-col overflow-hidden ${resultWindowSurfaceRadiusClassName} border border-slate-300 bg-white`;
}

export function resultWindowOcrResultTextAreaClassName() {
  return 'min-h-[72px] w-full resize-none overflow-hidden border-0 px-3 py-3 text-[14px] leading-[1.42] text-slate-800 outline-none placeholder:text-slate-400';
}

export function resultWindowOcrTokenButtonClassName() {
  return 'inline-flex max-w-full items-center gap-1 break-all rounded-full border border-slate-200 bg-white px-2.5 py-1 text-left text-[11px] font-semibold leading-tight text-slate-700 shadow-sm transition-colors duration-150 hover:border-sky-300 hover:bg-sky-50 hover:text-sky-700 focus-visible:outline focus-visible:outline-2 focus-visible:outline-sky-400';
}

function resultWindowTextAreaAutosizeSlackPx(
  textStyle?: ResultWindowAdaptiveTextStyle,
) {
  const fontSizePx = Number.parseFloat(textStyle?.fontSize ?? '14px');
  const lineHeightPx = fontSizePx * (textStyle?.lineHeight ?? 1.4);

  return Math.ceil(
    Math.min(
      resultWindowTextAreaMaxSlackPx,
      Math.max(
        resultWindowTextAreaMinSlackPx,
        lineHeightPx * resultWindowTextAreaSlackRatio,
      ),
    ),
  );
}

export function shouldCloseFromContainerClick(
  presentation: ResultWindowPresentation,
  target: EventTarget,
  currentTarget: EventTarget,
  isPinned = false,
) {
  return (
    !isPinned &&
    (presentation === 'overlay' || presentation === 'standalone') &&
    target === currentTarget
  );
}

export function shouldCloseFromWindowBlur(
  presentation: ResultWindowPresentation,
  isPinned = false,
  visibleDurationMs = Number.POSITIVE_INFINITY,
) {
  return (
    presentation === 'standalone' &&
    !isPinned &&
    visibleDurationMs >= resultWindowInitialBlurGraceMs
  );
}

export function shouldCloseFromEscapeKey(key: string) {
  return key === 'Escape';
}
