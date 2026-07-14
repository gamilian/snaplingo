import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from 'react';
import { useAppStore } from '../../stores/appStore';
import { useProviderStore } from '../../stores/providerStore';
import { useSettingsConfigStore } from '../../stores/settingsConfigStore';
import { translationFavoriteKey } from '../../application/favorites/identity';
import {
  defaultTargetLanguageForSource,
  getTranslationLanguageDisplayName,
  getTranslationLanguageSelectLabel,
  resolveTranslationRequestLanguages,
  swapTranslationLanguagePair,
  TRANSLATION_LANGUAGES,
} from '../../application/translation/languages';
import TranslationCard from './TranslationCard';
import { CustomSelect } from '../../components/common/CustomSelect';
import { getTranslationProviderDisplayName } from './translationProviderDisplayName';
import IconActionButton from './IconActionButton';
import ResultWindowScrollArea from './ResultWindowScrollArea';
import { ocrCopyTokens } from '../../utils/ocrTextProcessing';
import {
  resultWindowContentClassName,
  resultWindowAdaptiveTextStyle,
  resultWindowContainerClassName,
  resultWindowHeaderDragHandleClassName,
  autosizeResultWindowTextArea,
  measureResultWindowTextMirrorHeight,
  resultWindowOcrContentClassName,
  resultWindowOcrFullTextBoxClassName,
  resultWindowOcrImageActionButtonClassName,
  resultWindowOcrImagePanelClassName,
  resultWindowOcrPanelHeight,
  resultWindowOcrResultStackClassName,
  resultWindowOcrResultTextAreaClassName,
  resultWindowOcrTokenButtonClassName,
  resultWindowPanelClassName,
  resultWindowPinButtonClassName,
  resultWindowResultsSectionClassName,
  resultWindowResultsListClassName,
  resultWindowTranslationMeasuredPanelHeight,
  resultWindowTranslationSubtitle,
  resultWindowTextAreaClassName,
  resultWindowTextAreaMinHeightPx,
  resultWindowTextMirrorContent,
  resultWindowTextAreaRows,
  resultWindowTextBoxClassName,
  resultWindowTranslationLayout,
  shouldCloseFromContainerClick,
  shouldCloseFromEscapeKey,
  shouldCloseFromWindowBlur,
} from './presentation';
import {
  ClearTextIcon,
  CloseIcon,
  CopyIcon,
  FavoriteIcon,
  LanguageIcon,
  PinIcon,
  RetryIcon,
  ScanIcon,
  SwapIcon,
  UploadIcon,
  VolumeIcon,
} from './icons';
import { speakResultWindowText } from './speech';
import {
  ResultWindowRuntimeProvider,
  useResultWindowRuntime,
} from './runtimeContext';
import type {
  ResultWindowPresentation,
  ResultWindowRuntime,
} from '../../application/result-window/runtime';

interface ResultWindowProps {
  presentation?: ResultWindowPresentation;
  runtime: ResultWindowRuntime;
}

function Header({
  mode,
  subtitle,
  isPinned,
  onTogglePinned,
  onStartDrag,
  onClose,
}: {
  mode: 'translation' | 'ocr';
  subtitle: string;
  isPinned: boolean;
  onTogglePinned: () => void;
  onStartDrag?: () => void;
  onClose: () => void;
}) {
  const isOcr = mode === 'ocr';
  const isDraggable = Boolean(onStartDrag);

  return (
    <div className="flex min-h-12 items-center justify-between gap-3 rounded-t-[14px] border-b border-slate-200 bg-slate-50/90 px-3">
      <IconActionButton
        onClick={onTogglePinned}
        className={resultWindowPinButtonClassName(isPinned)}
        title={isPinned ? '取消固定' : '固定窗口'}
        aria-pressed={isPinned}
        tooltipPlacement="bottom"
      >
        <PinIcon className="h-[18px] w-[18px]" />
      </IconActionButton>

      <div
        className={resultWindowHeaderDragHandleClassName(isDraggable)}
        onMouseDown={(event) => {
          if (!onStartDrag || event.button !== 0) return;

          event.preventDefault();
          onStartDrag();
        }}
      >
        <div
          className={`grid h-7 w-7 shrink-0 place-items-center rounded-[7px] text-white ${
            isOcr
              ? 'bg-gradient-to-br from-emerald-600 to-blue-600'
              : 'bg-gradient-to-br from-blue-600 to-emerald-500'
          }`}
        >
          {isOcr ? (
            <ScanIcon className="h-4 w-4" />
          ) : (
            <LanguageIcon className="h-4 w-4" />
          )}
        </div>
        <div className="flex min-w-0 flex-wrap items-baseline gap-x-3">
          <h2 className="text-base font-bold leading-tight text-slate-950">
            {isOcr ? 'OCR' : '翻译'}
          </h2>
          <span className="text-xs font-semibold text-slate-500">{subtitle}</span>
        </div>
      </div>

      <IconActionButton
        onClick={onClose}
        className="grid h-7 w-7 shrink-0 place-items-center rounded-[7px] text-slate-500 transition-colors duration-150 hover:bg-slate-100 hover:text-slate-800"
        title="关闭"
        tooltipPlacement="bottom"
      >
        <CloseIcon className="h-[18px] w-[18px]" />
      </IconActionButton>
    </div>
  );
}

function LanguageSelect({
  options,
  value,
  onChange,
  selectedLabel,
  buttonClassName = 'h-9 rounded-[14px] border-slate-200 py-0 text-[14px] font-semibold shadow-none hover:shadow-none',
}: {
  options: { value: string; label: string }[];
  value: string;
  onChange: (value: string) => void;
  selectedLabel?: string;
  buttonClassName?: string;
}) {
  return (
    <CustomSelect
      options={options}
      value={value}
      onChange={onChange}
      selectedLabel={selectedLabel}
      align="center"
      buttonClassName={buttonClassName}
      menuClassName="rounded-[14px]"
    />
  );
}

function LanguageSwitcher({
  sourceLang,
  targetLang,
  onSourceChange,
  onTargetChange,
  onSwap,
}: {
  sourceLang: string;
  targetLang: string;
  onSourceChange: (value: string) => void;
  onTargetChange: (value: string) => void;
  onSwap: () => void;
}) {
  const mergedSelectButtonClassName =
    'h-9 rounded-none !border-0 !bg-transparent py-0 text-[13px] font-semibold !shadow-none !ring-0 hover:!border-transparent hover:!bg-transparent hover:!shadow-none';

  return (
    <div className="grid h-10 flex-none grid-cols-[minmax(0,1fr)_40px_minmax(0,1fr)] items-center overflow-visible rounded-[14px] border border-slate-200 bg-slate-50/90">
      <LanguageSelect
        options={TRANSLATION_LANGUAGES.map((lang) => ({
          value: lang.code,
          label: getTranslationLanguageSelectLabel(lang.code),
        }))}
        value={sourceLang}
        onChange={onSourceChange}
        selectedLabel={getTranslationLanguageDisplayName(sourceLang)}
        buttonClassName={mergedSelectButtonClassName}
      />
      <IconActionButton
        onClick={onSwap}
        className="grid h-full w-full place-items-center text-slate-600 transition-colors duration-150 hover:bg-white"
        title="切换语言"
        tooltipPlacement="bottom"
      >
        <SwapIcon className="h-4 w-4" />
      </IconActionButton>
      <LanguageSelect
        options={TRANSLATION_LANGUAGES.map((lang) => ({
          value: lang.code,
          label: getTranslationLanguageSelectLabel(lang.code),
        }))}
        value={targetLang}
        onChange={onTargetChange}
        selectedLabel={getTranslationLanguageDisplayName(targetLang)}
        buttonClassName={mergedSelectButtonClassName}
      />
    </div>
  );
}

export default function ResultWindow({ runtime, ...props }: ResultWindowProps) {
  return (
    <ResultWindowRuntimeProvider runtime={runtime}>
      <ResultWindowContent {...props} />
    </ResultWindowRuntimeProvider>
  );
}

function ResultWindowContent({
  presentation = 'overlay',
}: Omit<ResultWindowProps, 'runtime'>) {
  const runtime = useResultWindowRuntime();
  const {
    sourceText,
    sourceLang,
    targetLang,
    providerTranslations,
    isTranslating,
    ocrText,
    ocrConfidence,
    ocrImageBase64,
    isOcrRunning,
    ocrError,
    resultWindowVisible,
    resultWindowMode,
    autoTranslateRequestId,
    setSourceText,
    setSourceLang,
    setTargetLang,
    setOcrText,
    setOcrImageBase64,
  } = useAppStore();
  const translationProviders = useProviderStore((state) => state.translationProviders);
  const loadTranslationProviders = useProviderStore(
    (state) => state.loadTranslationProviders,
  );
  const translationSettings = useSettingsConfigStore(
    (state) => state.translation,
  );
  const ocrSettings = useSettingsConfigStore((state) => state.ocr);

  const lastAutoTranslateRequestId = useRef(0);
  const lastAutomaticTranslationKey = useRef('');
  const resultWindowPanelRef = useRef<HTMLDivElement>(null);
  const sourceTextAreaRef = useRef<HTMLTextAreaElement>(null);
  const sourceTextMirrorRef = useRef<HTMLDivElement>(null);
  const ocrImageTextAreaRef = useRef<HTMLTextAreaElement>(null);
  const ocrTextAreaRef = useRef<HTMLTextAreaElement>(null);
  const translationSourceBoxRef = useRef<HTMLDivElement>(null);
  const translationLanguageSwitcherRef = useRef<HTMLDivElement>(null);
  const translationResultsListRef = useRef<HTMLDivElement>(null);
  const [sourceTextAreaHeightPx, setSourceTextAreaHeightPx] = useState<
    number | undefined
  >(undefined);
  const [measuredTranslationPanelHeightPx, setMeasuredTranslationPanelHeightPx] =
    useState<number | undefined>(undefined);
  const [isResultWindowPinned, setResultWindowPinned] = useState(false);
  const [favoritedOcrSignature, setFavoritedOcrSignature] = useState<string | null>(
    null,
  );
  const [isOcrFavoritePending, setOcrFavoritePending] = useState(false);
  const [favoritedTranslationKeys, setFavoritedTranslationKeys] = useState(
    () => new Set<string>(),
  );
  const [isTranslationFavoritePending, setTranslationFavoritePending] =
    useState(false);
  const sourceTextStyle = useMemo(
    () => resultWindowAdaptiveTextStyle(sourceText, 'source'),
    [sourceText],
  );
  const ocrTextStyle = useMemo(
    () => resultWindowAdaptiveTextStyle(ocrText, 'ocr'),
    [ocrText],
  );
  const ocrTokens = useMemo(() => ocrCopyTokens(ocrText), [ocrText]);
  const ocrFavoriteSignature = `${ocrImageBase64 ?? ''}\u0000${ocrText}`;
  const completedTranslationResults = useMemo(
    () =>
      providerTranslations.filter(
        (translation) =>
          translation.status === 'success' &&
          translation.translated_text.trim().length > 0,
      ),
    [providerTranslations],
  );
  const translationLayout = useMemo(
    () =>
      resultWindowTranslationLayout(
        providerTranslations.map((translation) => ({
          providerId: translation.provider_id,
          text: translation.translated_text,
          status: translation.status,
        })),
        sourceTextAreaHeightPx,
      ),
    [providerTranslations, sourceTextAreaHeightPx],
  );
  const resolvedTargetLanguage = useMemo(
    () =>
      resolveTranslationRequestLanguages(sourceText, sourceLang, targetLang)
        .targetLang,
    [sourceLang, sourceText, targetLang],
  );
  const translationResultFavoriteKey = useCallback(
    (result: (typeof completedTranslationResults)[number]) =>
      translationFavoriteKey({
        sourceText,
        sourceLang,
        targetLang: resolvedTargetLanguage,
        providerId: result.provider_id,
        translatedText: result.translated_text,
      }),
    [resolvedTargetLanguage, sourceLang, sourceText],
  );
  const areAllTranslationResultsFavorited =
    completedTranslationResults.length > 0 &&
    completedTranslationResults.every((result) =>
      favoritedTranslationKeys.has(translationResultFavoriteKey(result)),
    );
  const translationPanelHeightPx =
    measuredTranslationPanelHeightPx ?? translationLayout.windowHeightPx;
  const sourceTextAreaStyle =
    sourceTextAreaHeightPx === undefined
      ? sourceTextStyle
      : {
          ...sourceTextStyle,
          height: `${sourceTextAreaHeightPx}px`,
        };

  const updateSourceTextAreaHeight = useCallback(() => {
    const measuredHeight = measureResultWindowTextMirrorHeight(
      sourceTextMirrorRef.current,
      resultWindowTextAreaMinHeightPx('source'),
    );

    if (measuredHeight !== null) {
      setSourceTextAreaHeightPx((currentHeight) =>
        currentHeight === measuredHeight ? currentHeight : measuredHeight,
      );
    }
  }, []);

  const updateMeasuredTranslationPanelHeight = useCallback(() => {
    if (!resultWindowVisible || resultWindowMode !== 'translation') return;

    const sourceBox = translationSourceBoxRef.current;
    const languageSwitcher = translationLanguageSwitcherRef.current;
    if (!sourceBox || !languageSwitcher) return;

    const measuredHeight = resultWindowTranslationMeasuredPanelHeight({
      sourceBoxHeightPx: sourceBox.offsetHeight,
      languageSwitcherHeightPx: languageSwitcher.offsetHeight,
      resultsListHeightPx: translationResultsListRef.current?.offsetHeight,
      hasResults: providerTranslations.length > 0,
    });

    setMeasuredTranslationPanelHeightPx((currentHeight) =>
      currentHeight === measuredHeight ? currentHeight : measuredHeight,
    );
  }, [providerTranslations.length, resultWindowMode, resultWindowVisible]);

  const closeResultWindow = useCallback(() => {
    void runtime.close(presentation);
  }, [presentation, runtime]);

  const handleFavoriteOcr = useCallback(async () => {
    if (!ocrText.trim() || isOcrFavoritePending) return;

    setOcrFavoritePending(true);
    try {
      await runtime.favoriteOcrResult(ocrImageBase64, ocrText, ocrConfidence);
      setFavoritedOcrSignature(ocrFavoriteSignature);
    } catch (error) {
      console.error('Failed to favorite OCR result:', error);
    } finally {
      setOcrFavoritePending(false);
    }
  }, [
    isOcrFavoritePending,
    ocrFavoriteSignature,
    ocrImageBase64,
    ocrConfidence,
    ocrText,
    runtime,
  ]);

  const handleFavoriteTranslation = useCallback(async () => {
    if (
      completedTranslationResults.length === 0 ||
      isTranslationFavoritePending
    ) {
      return;
    }

    setTranslationFavoritePending(true);
    try {
      await Promise.all(
        completedTranslationResults.map((result) =>
          runtime.commands.favoriteTranslationResult({
            text: sourceText,
            sourceLang,
            targetLang: resolvedTargetLanguage,
            result: {
              provider_id: result.provider_id,
              translated_text: result.translated_text,
              detected_language: result.detected_language,
              confidence: result.confidence,
            },
          }),
        ),
      );
      setFavoritedTranslationKeys((current) => {
        const next = new Set(current);
        completedTranslationResults.forEach((result) =>
          next.add(translationResultFavoriteKey(result)),
        );
        return next;
      });
    } catch (error) {
      console.error('Failed to favorite translation results:', error);
    } finally {
      setTranslationFavoritePending(false);
    }
  }, [
    completedTranslationResults,
    isTranslationFavoritePending,
    resolvedTargetLanguage,
    runtime,
    sourceLang,
    sourceText,
    translationResultFavoriteKey,
  ]);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (shouldCloseFromEscapeKey(e.key) && resultWindowVisible) {
        closeResultWindow();
      }
    };

    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [closeResultWindow, resultWindowVisible]);

  useEffect(() => {
    if (
      !resultWindowVisible ||
      !translationSettings?.hideOnBlur ||
      !shouldCloseFromWindowBlur(presentation, isResultWindowPinned)
    ) return;

    const visibleStartedAtMs = performance.now();
    const handleBlur = () => {
      if (
        shouldCloseFromWindowBlur(
          presentation,
          isResultWindowPinned,
          performance.now() - visibleStartedAtMs,
        )
      ) {
        closeResultWindow();
      }
    };

    window.addEventListener('blur', handleBlur);
    return () => window.removeEventListener('blur', handleBlur);
  }, [
    closeResultWindow,
    isResultWindowPinned,
    presentation,
    resultWindowVisible,
    translationSettings?.hideOnBlur,
  ]);

  useEffect(() => {
    if (presentation !== 'standalone' || !translationSettings) return;
    void runtime.setAlwaysOnTop(translationSettings.windowAlwaysOnTop);
  }, [presentation, runtime, translationSettings?.windowAlwaysOnTop]);

  useEffect(() => {
    if (resultWindowVisible) return;

    setResultWindowPinned(false);
    setMeasuredTranslationPanelHeightPx(undefined);
  }, [resultWindowVisible]);

  useEffect(() => {
    if (!resultWindowVisible || presentation !== 'standalone') return;

    if (resultWindowMode === 'ocr') {
      const panel = resultWindowPanelRef.current;
      if (!panel) return;

      const updateOcrWindowHeight = () => {
        const availableHeight = window.screen?.availHeight || window.innerHeight;
        const panelHeight = resultWindowOcrPanelHeight(
          panel.scrollHeight,
          availableHeight,
        );

        void runtime.resizeStandaloneWindow({
          presentation,
          visible: resultWindowVisible,
          panelHeightPx: panelHeight,
        });
      };

      updateOcrWindowHeight();

      if (typeof ResizeObserver === 'undefined') return;

      const resizeObserver = new ResizeObserver(updateOcrWindowHeight);
      resizeObserver.observe(panel);

      return () => resizeObserver.disconnect();
    }

    void runtime.resizeStandaloneWindow({
      presentation,
      visible: resultWindowVisible,
      panelHeightPx: translationPanelHeightPx,
    });
  }, [
    ocrTokens.length,
    ocrError,
    ocrImageBase64,
    ocrText,
    presentation,
    resultWindowMode,
    resultWindowVisible,
    translationPanelHeightPx,
  ]);

  useEffect(() => {
    if (!resultWindowVisible) return;

    void loadTranslationProviders();
  }, [loadTranslationProviders, resultWindowVisible]);

  useLayoutEffect(() => {
    if (!resultWindowVisible) return;

    updateSourceTextAreaHeight();
  }, [resultWindowVisible, sourceText, sourceTextStyle, updateSourceTextAreaHeight]);

  useLayoutEffect(() => {
    if (!resultWindowVisible) return;

    const sourceBox = translationSourceBoxRef.current;
    if (!sourceBox || typeof ResizeObserver === 'undefined') return;

    const resizeObserver = new ResizeObserver(() => {
      updateSourceTextAreaHeight();
    });
    resizeObserver.observe(sourceBox);

    return () => resizeObserver.disconnect();
  }, [resultWindowVisible, updateSourceTextAreaHeight]);

  useLayoutEffect(() => {
    if (!resultWindowVisible) return;

    autosizeResultWindowTextArea(ocrImageTextAreaRef.current, {
      minHeightPx: resultWindowTextAreaMinHeightPx('ocr'),
      textStyle: ocrTextStyle,
    });
    autosizeResultWindowTextArea(ocrTextAreaRef.current, {
      minHeightPx: resultWindowTextAreaMinHeightPx('ocr'),
      textStyle: ocrTextStyle,
    });
  }, [ocrText, ocrTextStyle, resultWindowVisible]);

  useLayoutEffect(() => {
    if (!resultWindowVisible) return;

    updateMeasuredTranslationPanelHeight();
  }, [
    measuredTranslationPanelHeightPx,
    ocrImageBase64,
    ocrText,
    providerTranslations,
    resultWindowMode,
    resultWindowVisible,
    sourceTextAreaHeightPx,
    updateMeasuredTranslationPanelHeight,
  ]);

  useEffect(() => {
    if (
      !resultWindowVisible ||
      autoTranslateRequestId === 0 ||
      autoTranslateRequestId === lastAutoTranslateRequestId.current ||
      !sourceText.trim()
    ) {
      return;
    }

    lastAutoTranslateRequestId.current = autoTranslateRequestId;
    lastAutomaticTranslationKey.current = `${sourceLang}\u0000${targetLang}\u0000${sourceText}`;
    void runtime.translate({ text: sourceText, sourceLang, targetLang });
  }, [
    autoTranslateRequestId,
    resultWindowVisible,
    sourceLang,
    sourceText,
    targetLang,
    runtime,
  ]);

  useEffect(() => {
    if (
      !resultWindowVisible ||
      resultWindowMode !== 'translation' ||
      !translationSettings?.autoTranslate ||
      !sourceText.trim() ||
      isTranslating
    ) {
      return;
    }

    const key = `${sourceLang}\u0000${targetLang}\u0000${sourceText}`;
    if (key === lastAutomaticTranslationKey.current) return;

    const timeout = window.setTimeout(() => {
      lastAutomaticTranslationKey.current = key;
      void runtime.translate({ text: sourceText, sourceLang, targetLang });
    }, translationSettings.incrementalTranslation ? 150 : 500);

    return () => window.clearTimeout(timeout);
  }, [
    isTranslating,
    resultWindowMode,
    resultWindowVisible,
    runtime,
    sourceLang,
    sourceText,
    targetLang,
    translationSettings?.autoTranslate,
    translationSettings?.incrementalTranslation,
  ]);

  if (!resultWindowVisible) return null;

  const handleOverlayClick = (e: MouseEvent<HTMLDivElement>) => {
    if (
      shouldCloseFromContainerClick(
        presentation,
        e.target,
        e.currentTarget,
        isResultWindowPinned,
      )
    ) {
      closeResultWindow();
    }
  };

  const handleStartDrag = () => {
    void runtime.beginDrag();
  };

  const handleSourceLanguageChange = (nextSourceLang: string) => {
    setSourceLang(nextSourceLang);
    setTargetLang(
      targetLang === 'auto'
        ? 'auto'
        : defaultTargetLanguageForSource(nextSourceLang),
    );
  };

  const handleSwapLanguages = () => {
    const nextLanguagePair = swapTranslationLanguagePair(sourceLang, targetLang);
    setSourceLang(nextLanguagePair.sourceLang);
    setTargetLang(nextLanguagePair.targetLang);
  };

  const handleTranslate = () => {
    lastAutomaticTranslationKey.current = `${sourceLang}\u0000${targetLang}\u0000${sourceText}`;
    void runtime.translate({ text: sourceText, sourceLang, targetLang });
  };

  const handleUploadImage = () => {
    void runtime.startFileOcr();
  };

  const isOcrMode = resultWindowMode === 'ocr';
  const translationSubtitle = resultWindowTranslationSubtitle(
    providerTranslations,
    isTranslating,
  );
  const ocrSubtitle = isOcrRunning
    ? '识别中'
    : ocrText
      ? `已识别 ${ocrText.length} 个字符${
          ocrSettings?.showConfidence && ocrConfidence !== null
            ? ` · 置信度 ${Math.round(ocrConfidence * 100)}%`
            : ''
        }`
      : '等待上传';
  const detectedSourceLanguage =
    providerTranslations.find((translation) => translation.detected_language)
      ?.detected_language || (sourceLang !== 'auto' ? sourceLang : 'auto');
  const translationPanelHeight =
    presentation === 'overlay'
      ? `min(${translationPanelHeightPx}px, 90vh)`
      : `${translationPanelHeightPx}px`;
  const panelStyle = isOcrMode
    ? { height: 'auto' }
    : { height: translationPanelHeight };

  return (
    <div
      className={resultWindowContainerClassName(presentation, {
        fitContent: isOcrMode,
      })}
      onClick={handleOverlayClick}
    >
      <div
        ref={resultWindowPanelRef}
        className={resultWindowPanelClassName(presentation, {
          fitContent: isOcrMode,
        })}
        style={panelStyle}
      >
        <Header
          mode={isOcrMode ? 'ocr' : 'translation'}
          subtitle={isOcrMode ? ocrSubtitle : translationSubtitle}
          isPinned={isResultWindowPinned}
          onTogglePinned={() => setResultWindowPinned((isPinned) => !isPinned)}
          onStartDrag={presentation === 'standalone' ? handleStartDrag : undefined}
          onClose={closeResultWindow}
        />

        {isOcrMode ? (
          <ResultWindowScrollArea
            className={resultWindowOcrContentClassName()}
          >
            {ocrImageBase64 ? (
              <div className={resultWindowOcrResultStackClassName()}>
                <div className="flex min-h-0 flex-col gap-2">
                  <div className="flex min-h-[18px] items-center justify-between gap-3">
                    <h3 className="text-[13px] font-bold text-slate-600">原图</h3>
                    <span className="text-[11px] text-slate-400">source</span>
                  </div>
                  <div className={resultWindowOcrImagePanelClassName()}>
                    <img
                      src={
                        ocrImageBase64.startsWith('data:')
                          ? ocrImageBase64
                          : `data:image/png;base64,${ocrImageBase64}`
                      }
                      alt=""
                      className="block max-h-[220px] w-full object-contain"
                      draggable={false}
                    />
                  </div>
                  <button
                    type="button"
                    className={resultWindowOcrImageActionButtonClassName()}
                    onClick={handleUploadImage}
                    disabled={isOcrRunning}
                  >
                    <UploadIcon className="h-4 w-4" />
                    {isOcrRunning ? '识别中...' : '上传图片 OCR'}
                  </button>
                </div>

                <div className="flex min-h-0 flex-col gap-3">
                  <div className="flex min-h-[18px] items-center justify-between gap-3">
                    <h3 className="text-[13px] font-bold text-slate-600">OCR 结果</h3>
                    <span className="text-[11px] text-slate-400">
                      {ocrText ? `${ocrText.length} chars` : 'No text'}
                    </span>
                  </div>
                  <div className={resultWindowOcrFullTextBoxClassName()}>
                    <textarea
                      ref={ocrImageTextAreaRef}
                      value={ocrText}
                      readOnly
                      placeholder="OCR 结果..."
                      rows={resultWindowTextAreaRows(ocrText, 'ocr')}
                      className={resultWindowOcrResultTextAreaClassName()}
                      style={ocrTextStyle}
                    />
                    {ocrTokens.length > 0 && (
                      <div className="border-t border-slate-100 bg-slate-50/60 px-3 py-2">
                        <div className="mb-2 flex min-h-[16px] items-center justify-between gap-3">
                          <span className="text-[11px] font-bold text-slate-500">
                            推荐复制片段
                          </span>
                          <span className="text-[10px] text-slate-400">
                            {ocrTokens.length} 个
                          </span>
                        </div>
                        <div className="flex flex-wrap gap-1.5">
                          {ocrTokens.map((token) => (
                            <button
                              key={token.id}
                              type="button"
                              title={`复制${token.label}: ${token.value}`}
                              aria-label={`复制${token.label}: ${token.value}`}
                              className={resultWindowOcrTokenButtonClassName()}
                              onClick={() => {
                                void runtime.clipboard.copyText(token.value);
                              }}
                            >
                              <span className="shrink-0 text-slate-400">
                                {token.label}
                              </span>
                              <span className="min-w-0 break-all">{token.value}</span>
                            </button>
                          ))}
                        </div>
                      </div>
                    )}
                    <div className="flex min-h-8 items-center justify-between gap-3 border-t border-slate-100 bg-slate-50/80 px-3 py-1.5">
                      <span className="text-[11px] text-slate-400">
                        {ocrText.length} chars
                      </span>
                      <div className="flex items-center gap-2">
                        <IconActionButton
                          title="朗读"
                          disabled={!ocrText.trim()}
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                          onClick={() => {
                            void speakResultWindowText(ocrText);
                          }}
                        >
                          <VolumeIcon className="h-4 w-4" />
                        </IconActionButton>
                        <IconActionButton
                          title={
                            favoritedOcrSignature === ocrFavoriteSignature
                              ? '已收藏'
                              : '收藏'
                          }
                          aria-pressed={
                            favoritedOcrSignature === ocrFavoriteSignature
                          }
                          disabled={
                            !ocrText.trim() ||
                            isOcrFavoritePending ||
                            favoritedOcrSignature === ocrFavoriteSignature
                          }
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-amber-200 hover:bg-amber-50 hover:text-amber-600 disabled:cursor-not-allowed disabled:opacity-60"
                          onClick={() => void handleFavoriteOcr()}
                        >
                          <FavoriteIcon
                            className="h-4 w-4"
                            fill={
                              favoritedOcrSignature === ocrFavoriteSignature
                                ? 'currentColor'
                                : 'none'
                            }
                          />
                        </IconActionButton>
                        <IconActionButton
                          title="复制"
                          disabled={!ocrText.trim()}
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                          onClick={() => {
                            void runtime.clipboard.copyText(ocrText);
                          }}
                        >
                          <CopyIcon className="h-4 w-4" />
                        </IconActionButton>
                        <IconActionButton
                          title="清空"
                          disabled={!ocrText}
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                          onClick={() => {
                            setOcrText('');
                            setOcrImageBase64(null);
                          }}
                        >
                          <ClearTextIcon className="h-4 w-4" />
                        </IconActionButton>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            ) : (
              <>
                <div className="space-y-3">
                  <div className="flex min-h-[18px] items-center justify-between gap-3">
                    <label className="text-[13px] font-bold text-slate-600">图片</label>
                    <span className="text-[11px] text-slate-400">PNG/JPG/WebP</span>
                  </div>
                  <button
                    type="button"
                    onClick={handleUploadImage}
                    disabled={isOcrRunning}
                    className="grid min-h-[92px] w-full place-items-center rounded-[14px] border border-dashed border-slate-300 bg-gradient-to-b from-slate-50 to-white px-3 py-3 text-center transition-colors duration-150 hover:border-emerald-300 hover:bg-emerald-50/30 disabled:cursor-not-allowed disabled:opacity-70"
                  >
                    <span className="grid justify-items-center gap-3">
                      <span className="grid h-9 w-9 place-items-center rounded-[7px] bg-emerald-50 text-emerald-700">
                        <UploadIcon className="h-5 w-5" />
                      </span>
                      <span className="text-[13px] font-semibold text-slate-900">
                        {isOcrRunning ? '识别中...' : '上传图片 OCR'}
                      </span>
                      <span className="text-[11px] text-slate-500">
                        选择一张图片后自动开始识别
                      </span>
                    </span>
                  </button>
                </div>

                {ocrError && (
                  <div className="rounded-[14px] border border-red-200 bg-red-50 px-3 py-3 text-sm font-medium text-red-700">
                    {ocrError}
                  </div>
                )}

                <div className="min-h-0 space-y-3">
                  <div className="flex min-h-[18px] items-center justify-between gap-3">
                    <h3 className="text-[13px] font-bold text-slate-600">识别文本</h3>
                    <span className="text-[11px] text-slate-400">
                      {ocrText ? `${ocrText.length} chars` : 'No text'}
                    </span>
                  </div>
                  <div className={resultWindowTextBoxClassName()}>
                    <textarea
                      ref={ocrTextAreaRef}
                      value={ocrText}
                      readOnly
                      rows={resultWindowTextAreaRows(ocrText, 'ocr')}
                      placeholder="OCR 结果..."
                      className={resultWindowTextAreaClassName('ocr')}
                      style={ocrTextStyle}
                    />
                    <div className="flex min-h-8 items-center justify-between gap-3 border-t border-slate-100 bg-slate-50/80 px-3 py-1.5">
                      <span className="text-[11px] text-slate-400">
                        {ocrText.length} chars
                      </span>
                      <div className="flex items-center gap-2">
                        <IconActionButton
                          title="朗读"
                          disabled={!ocrText.trim()}
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                          onClick={() => {
                            void speakResultWindowText(ocrText);
                          }}
                        >
                          <VolumeIcon className="h-4 w-4" />
                        </IconActionButton>
                        <IconActionButton
                          title={
                            favoritedOcrSignature === ocrFavoriteSignature
                              ? '已收藏'
                              : '收藏'
                          }
                          aria-pressed={
                            favoritedOcrSignature === ocrFavoriteSignature
                          }
                          disabled={
                            !ocrText.trim() ||
                            isOcrFavoritePending ||
                            favoritedOcrSignature === ocrFavoriteSignature
                          }
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-amber-200 hover:bg-amber-50 hover:text-amber-600 disabled:cursor-not-allowed disabled:opacity-60"
                          onClick={() => void handleFavoriteOcr()}
                        >
                          <FavoriteIcon
                            className="h-4 w-4"
                            fill={
                              favoritedOcrSignature === ocrFavoriteSignature
                                ? 'currentColor'
                                : 'none'
                            }
                          />
                        </IconActionButton>
                        <IconActionButton
                          title="复制"
                          disabled={!ocrText.trim()}
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                          onClick={() => {
                            void runtime.clipboard.copyText(ocrText);
                          }}
                        >
                          <CopyIcon className="h-4 w-4" />
                        </IconActionButton>
                        <IconActionButton
                          title="清空"
                          disabled={!ocrText}
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                          onClick={() => setOcrText('')}
                        >
                          <ClearTextIcon className="h-4 w-4" />
                        </IconActionButton>
                      </div>
                    </div>
                  </div>
                </div>
              </>
            )}
          </ResultWindowScrollArea>
        ) : (
          <ResultWindowScrollArea
            className={resultWindowContentClassName({
              reserveBottom: providerTranslations.length === 0,
            })}
          >
            <div ref={translationSourceBoxRef} className="flex-none">
              <div className={`${resultWindowTextBoxClassName()} relative`}>
                <div
                  ref={sourceTextMirrorRef}
                  aria-hidden="true"
                  className="invisible pointer-events-none absolute inset-x-0 top-0 whitespace-pre-wrap break-words px-3 py-2.5 text-[13px] text-slate-900"
                  style={sourceTextStyle}
                >
                  {resultWindowTextMirrorContent(sourceText)}
                </div>
                <textarea
                  ref={sourceTextAreaRef}
                  value={sourceText}
                  onChange={(e) => setSourceText(e.target.value)}
                  rows={resultWindowTextAreaRows(sourceText, 'source')}
                  placeholder="输入需要翻译的文本..."
                  className={resultWindowTextAreaClassName('source')}
                  style={sourceTextAreaStyle}
                />
                <div className="flex min-h-8 items-center justify-between gap-3 border-t border-slate-100 bg-slate-50/80 px-3 py-1.5">
                  <span className="min-w-0 truncate text-[11px] text-slate-400">
                    {sourceText.length} chars · 识别为{' '}
                    <span className="font-semibold text-blue-600">
                      {getTranslationLanguageDisplayName(detectedSourceLanguage)}
                    </span>
                  </span>
                  <div className="flex items-center gap-2">
                    <IconActionButton
                      title={
                        areAllTranslationResultsFavorited
                          ? '已收藏'
                          : '收藏'
                      }
                      aria-pressed={
                        areAllTranslationResultsFavorited
                      }
                      disabled={
                        completedTranslationResults.length === 0 ||
                        isTranslationFavoritePending ||
                        areAllTranslationResultsFavorited
                      }
                      className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-amber-200 hover:bg-amber-50 hover:text-amber-600 disabled:cursor-not-allowed disabled:opacity-60"
                      onClick={() => void handleFavoriteTranslation()}
                    >
                      <FavoriteIcon
                        className="h-4 w-4"
                        fill={
                          areAllTranslationResultsFavorited
                            ? 'currentColor'
                            : 'none'
                        }
                      />
                    </IconActionButton>
                    <IconActionButton
                      title="复制"
                      disabled={!sourceText.trim()}
                      className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                      onClick={() => {
                        void runtime.clipboard.copyText(sourceText);
                      }}
                    >
                      <CopyIcon className="h-4 w-4" />
                    </IconActionButton>
                    <IconActionButton
                      title="朗读"
                      disabled={!sourceText.trim()}
                      className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                      onClick={() => {
                        void speakResultWindowText(
                          sourceText,
                          detectedSourceLanguage === 'auto'
                            ? undefined
                            : detectedSourceLanguage,
                        );
                      }}
                    >
                      <VolumeIcon className="h-4 w-4" />
                    </IconActionButton>
                    <IconActionButton
                      title="重试"
                      disabled={!sourceText.trim() || isTranslating}
                      className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                      onClick={handleTranslate}
                    >
                      <RetryIcon className="h-4 w-4" />
                    </IconActionButton>
                    <IconActionButton
                      title="清空"
                      disabled={!sourceText}
                      className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                      onClick={() => setSourceText('')}
                    >
                      <ClearTextIcon className="h-4 w-4" />
                    </IconActionButton>
                  </div>
                </div>
              </div>
            </div>

            <div ref={translationLanguageSwitcherRef} className="flex-none">
              <LanguageSwitcher
                sourceLang={sourceLang}
                targetLang={targetLang}
                onSourceChange={handleSourceLanguageChange}
                onTargetChange={setTargetLang}
                onSwap={handleSwapLanguages}
              />
            </div>

            {providerTranslations.length > 0 && (
              <div className={resultWindowResultsSectionClassName()}>
                <div
                  ref={translationResultsListRef}
                  className={resultWindowResultsListClassName()}
                >
                  {providerTranslations.map((result) => (
                    <TranslationCard
                      key={result.provider_id}
                      providerId={result.provider_id}
                      providerName={getTranslationProviderDisplayName(
                        result.provider_id,
                        translationProviders,
                      )}
                      status={result.status}
                      text={result.translated_text}
                      languageCode={resolvedTargetLanguage}
                      bodyHeightPx={
                        translationLayout.bodyHeightByProviderId[
                          result.provider_id
                        ] ?? 44
                      }
                      onRetry={() => {
                        void runtime.retryTranslationProvider(result.provider_id);
                      }}
                      onFavorite={() =>
                        runtime.commands.favoriteTranslationResult({
                          text: sourceText,
                          sourceLang,
                          targetLang: resolvedTargetLanguage,
                          result: {
                            provider_id: result.provider_id,
                            translated_text: result.translated_text,
                            detected_language: result.detected_language,
                            confidence: result.confidence,
                          },
                        }).then(() => {
                          setFavoritedTranslationKeys((current) =>
                            new Set(current).add(
                              translationResultFavoriteKey(result),
                            ),
                          );
                        })
                      }
                      isFavorite={favoritedTranslationKeys.has(
                        translationResultFavoriteKey(result),
                      )}
                      copyText={runtime.clipboard.copyText}
                    />
                  ))}
                </div>
              </div>
            )}
          </ResultWindowScrollArea>
        )}
      </div>
    </div>
  );
}
