import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from 'react';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { useAppStore } from '../../stores/appStore';
import { useProviderStore } from '../../stores/providerStore';
import {
  defaultTargetLanguageForSource,
  getTranslationLanguageDisplayName,
  getTranslationLanguageSelectLabel,
  resolveTranslationRequestLanguages,
  swapTranslationLanguagePair,
  TRANSLATION_LANGUAGES,
} from '../../hooks/translationLanguages';
import { useTranslate } from '../../hooks/useTranslate';
import TranslationCard from './TranslationCard';
import { CustomSelect } from '../common/CustomSelect';
import { recognizeImageFile, selectImageFile } from '../../tauri/ocr';
import { runOcrFileWorkflow } from './ocrFileWorkflow';
import { getTranslationProviderDisplayName } from './translationProviderDisplayName';
import IconActionButton from './IconActionButton';
import ResultWindowScrollArea from './ResultWindowScrollArea';
import {
  resultWindowContentClassName,
  resultWindowAdaptiveTextStyle,
  resultWindowContainerClassName,
  resultWindowHeaderDragHandleClassName,
  autosizeResultWindowTextArea,
  measureResultWindowTextMirrorHeight,
  resultWindowOcrImagePanelClassName,
  resultWindowOcrResultGridClassName,
  resultWindowOcrResultTextAreaClassName,
  resultWindowPanelClassName,
  resultWindowPinButtonClassName,
  resultWindowResultsSectionClassName,
  resultWindowResultsListClassName,
  resultWindowStandaloneWindowHeight,
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
  type ResultWindowPresentation,
} from './presentation';
import {
  ClearTextIcon,
  CloseIcon,
  CopyIcon,
  LanguageIcon,
  PinIcon,
  RetryIcon,
  ScanIcon,
  SwapIcon,
  UploadIcon,
  VolumeIcon,
} from './icons';
import { speakResultWindowText } from './speech';

interface ResultWindowProps {
  presentation?: ResultWindowPresentation;
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

export default function ResultWindow({
  presentation = 'overlay',
}: ResultWindowProps) {
  const {
    sourceText,
    sourceLang,
    targetLang,
    providerTranslations,
    isTranslating,
    ocrText,
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
    setOcrRunning,
    setOcrError,
    hideResultWindow,
  } = useAppStore();
  const translationProviders = useProviderStore((state) => state.translationProviders);
  const loadTranslationProviders = useProviderStore(
    (state) => state.loadTranslationProviders,
  );

  const { translate, retryProvider } = useTranslate();
  const lastAutoTranslateRequestId = useRef(0);
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
  const sourceTextStyle = useMemo(
    () => resultWindowAdaptiveTextStyle(sourceText, 'source'),
    [sourceText],
  );
  const ocrTextStyle = useMemo(
    () => resultWindowAdaptiveTextStyle(ocrText, 'ocr'),
    [ocrText],
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

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (shouldCloseFromEscapeKey(e.key) && resultWindowVisible) {
        hideResultWindow();
      }
    };

    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [resultWindowVisible, hideResultWindow]);

  useEffect(() => {
    if (!resultWindowVisible || !shouldCloseFromWindowBlur(presentation, isResultWindowPinned)) return;

    const handleBlur = () => {
      hideResultWindow();
    };

    window.addEventListener('blur', handleBlur);
    return () => window.removeEventListener('blur', handleBlur);
  }, [hideResultWindow, isResultWindowPinned, presentation, resultWindowVisible]);

  useEffect(() => {
    if (resultWindowVisible) return;

    setResultWindowPinned(false);
    setMeasuredTranslationPanelHeightPx(undefined);
  }, [resultWindowVisible]);

  useEffect(() => {
    if (!resultWindowVisible || presentation !== 'standalone') return;

    const panelHeight =
      resultWindowMode === 'translation' ? translationPanelHeightPx : 660;

    void getCurrentWindow().setSize(
      new LogicalSize(660, resultWindowStandaloneWindowHeight(panelHeight)),
    );
  }, [
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
    void translate(sourceText, sourceLang, targetLang);
  }, [
    autoTranslateRequestId,
    resultWindowVisible,
    sourceLang,
    sourceText,
    targetLang,
    translate,
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
      hideResultWindow();
    }
  };

  const handleStartDrag = () => {
    void getCurrentWindow().startDragging();
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
    void translate();
  };

  const handleUploadImage = () => {
    setOcrImageBase64(null);
    void runOcrFileWorkflow({
      selectImageFile,
      recognizeImageFile,
      setText: setOcrText,
      setRunning: setOcrRunning,
      setError: setOcrError,
    });
  };

  const isOcrMode = resultWindowMode === 'ocr';
  const translationSubtitle = resultWindowTranslationSubtitle(
    providerTranslations,
    isTranslating,
  );
  const ocrSubtitle = isOcrRunning
    ? '识别中'
    : ocrText
      ? `已识别 ${ocrText.length} 个字符`
      : '等待上传';
  const detectedSourceLanguage =
    providerTranslations.find((translation) => translation.detected_language)
      ?.detected_language || (sourceLang !== 'auto' ? sourceLang : 'auto');
  const translationPanelHeight =
    presentation === 'overlay'
      ? `min(${translationPanelHeightPx}px, 90vh)`
      : `${translationPanelHeightPx}px`;

  return (
    <div
      className={resultWindowContainerClassName(presentation)}
      onClick={handleOverlayClick}
    >
      <div
        className={resultWindowPanelClassName(presentation)}
        style={isOcrMode ? undefined : { height: translationPanelHeight }}
      >
        <Header
          mode={isOcrMode ? 'ocr' : 'translation'}
          subtitle={isOcrMode ? ocrSubtitle : translationSubtitle}
          isPinned={isResultWindowPinned}
          onTogglePinned={() => setResultWindowPinned((isPinned) => !isPinned)}
          onStartDrag={presentation === 'standalone' ? handleStartDrag : undefined}
          onClose={hideResultWindow}
        />

        {isOcrMode ? (
          <ResultWindowScrollArea className={resultWindowContentClassName()}>
            {ocrImageBase64 ? (
              <div className={resultWindowOcrResultGridClassName()}>
                <div className="flex min-h-0 flex-col gap-3">
                  <div className="flex min-h-[18px] items-center justify-between gap-3">
                    <h3 className="text-[13px] font-bold text-slate-600">截图区域</h3>
                    <span className="text-[11px] text-slate-400">source</span>
                  </div>
                  <div className={resultWindowOcrImagePanelClassName()}>
                    <img
                      src={`data:image/png;base64,${ocrImageBase64}`}
                      alt=""
                      className="h-full w-full object-contain"
                      draggable={false}
                    />
                  </div>
                </div>

                <div className="flex min-h-0 flex-col gap-3">
                  <div className="flex min-h-[18px] items-center justify-between gap-3">
                    <h3 className="text-[13px] font-bold text-slate-600">识别文本</h3>
                    <span className="text-[11px] text-slate-400">
                      {ocrText ? `${ocrText.length} chars` : 'No text'}
                    </span>
                  </div>
                  <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-[14px] border border-slate-300 bg-white">
                    <textarea
                      ref={ocrImageTextAreaRef}
                      value={ocrText}
                      readOnly
                      placeholder="OCR 结果..."
                      rows={resultWindowTextAreaRows(ocrText, 'ocr')}
                      className={resultWindowOcrResultTextAreaClassName()}
                      style={ocrTextStyle}
                    />
                    <div className="flex min-h-8 items-center justify-between gap-3 border-t border-slate-100 bg-slate-50/80 px-3 py-1.5">
                      <span className="text-[11px] text-slate-400">
                        {ocrText.length} chars
                      </span>
                      <div className="flex items-center gap-2">
                        <IconActionButton
                          title="复制"
                          disabled={!ocrText.trim()}
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                          onClick={() => {
                            void navigator.clipboard.writeText(ocrText);
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

                <button
                  type="button"
                  onClick={handleUploadImage}
                  disabled={isOcrRunning}
                  className="grid h-10 w-full place-items-center rounded-[14px] bg-emerald-600 text-[14px] font-bold text-white shadow-[0_10px_24px_rgba(5,150,105,0.18)] transition-colors duration-150 hover:bg-emerald-700 disabled:bg-slate-300 disabled:shadow-none"
                >
                  {ocrText ? '重新选择图片' : '选择图片'}
                </button>

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
                          title="复制"
                          disabled={!ocrText.trim()}
                          className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                          onClick={() => {
                            void navigator.clipboard.writeText(ocrText);
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
                      title="复制"
                      disabled={!sourceText.trim()}
                      className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
                      onClick={() => {
                        void navigator.clipboard.writeText(sourceText);
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
                        void retryProvider(result.provider_id);
                      }}
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
