import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
  type ReactNode,
} from 'react';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { useAppStore } from '../../stores/appStore';
import { useProviderStore } from '../../stores/providerStore';
import { useTranslate } from '../../hooks/useTranslate';
import TranslationCard from './TranslationCard';
import { CustomSelect } from '../common/CustomSelect';
import { recognizeImageFile, selectImageFile } from '../../tauri/ocr';
import { runOcrFileWorkflow } from './ocrFileWorkflow';
import { getTranslationProviderDisplayName } from './translationProviderDisplayName';
import {
  autosizeResultWindowTextArea,
  resultWindowContentClassName,
  resultWindowAdaptiveTextStyle,
  resultWindowContainerClassName,
  resultWindowHeaderDragHandleClassName,
  resultWindowOcrImagePanelClassName,
  resultWindowOcrResultGridClassName,
  resultWindowOcrResultTextAreaClassName,
  resultWindowPanelClassName,
  resultWindowPinButtonClassName,
  resultWindowResultsListClassName,
  resultWindowStandaloneWindowHeight,
  resultWindowTranslationSubtitle,
  resultWindowTextAreaClassName,
  resultWindowTextAreaMinHeightPx,
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
} from './icons';

const LANGUAGES = [
  { code: 'auto', name: 'Auto Detect' },
  { code: 'en', name: 'English' },
  { code: 'zh-CN', name: 'Chinese (Simplified)' },
  { code: 'zh-TW', name: 'Chinese (Traditional)' },
  { code: 'ja', name: 'Japanese' },
  { code: 'ko', name: 'Korean' },
  { code: 'es', name: 'Spanish' },
  { code: 'fr', name: 'French' },
  { code: 'de', name: 'German' },
  { code: 'ru', name: 'Russian' },
  { code: 'ar', name: 'Arabic' },
];

const LANGUAGE_DISPLAY_NAMES: Record<string, string> = {
  auto: '自动检测',
  en: '英语',
  zh: '中文',
  'zh-CN': '中文简体',
  'zh-TW': '中文繁体',
  ja: '日语',
  ko: '韩语',
  es: '西班牙语',
  fr: '法语',
  de: '德语',
  ru: '俄语',
  ar: '阿拉伯语',
};

const RESULT_WINDOW_SCROLLBAR_IDLE_DELAY_MS = 900;

interface ResultWindowProps {
  presentation?: ResultWindowPresentation;
}

function IconButton({
  title,
  children,
  onClick,
  disabled = false,
}: {
  title: string;
  children: ReactNode;
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="grid h-7 w-7 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700 disabled:cursor-not-allowed disabled:opacity-45"
      title={title}
      aria-label={title}
    >
      {children}
    </button>
  );
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
    <div className="flex min-h-12 items-center justify-between gap-2.5 rounded-t-[14px] border-b border-slate-200 bg-slate-50/90 px-2.5">
      <button
        type="button"
        onClick={onTogglePinned}
        className={resultWindowPinButtonClassName(isPinned)}
        title={isPinned ? '取消固定' : '固定窗口'}
        aria-label={isPinned ? '取消固定' : '固定窗口'}
        aria-pressed={isPinned}
      >
        <PinIcon className="h-[18px] w-[18px]" />
      </button>

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
        <div className="flex min-w-0 flex-wrap items-baseline gap-x-2.5">
          <h2 className="text-base font-bold leading-tight text-slate-950">
            {isOcr ? 'OCR' : '翻译'}
          </h2>
          <span className="text-xs font-semibold text-slate-500">{subtitle}</span>
        </div>
      </div>

      <button
        type="button"
        onClick={onClose}
        className="grid h-7 w-7 shrink-0 place-items-center rounded-[7px] text-slate-500 transition-colors duration-150 hover:bg-slate-100 hover:text-slate-800"
        title="关闭"
        aria-label="关闭"
      >
        <CloseIcon className="h-[18px] w-[18px]" />
      </button>
    </div>
  );
}

function LanguageSelect({
  options,
  value,
  onChange,
  buttonClassName = 'h-9 rounded-[14px] border-slate-200 py-0 text-[14px] font-semibold shadow-none hover:shadow-none',
}: {
  options: { value: string; label: string }[];
  value: string;
  onChange: (value: string) => void;
  buttonClassName?: string;
}) {
  return (
    <CustomSelect
      options={options}
      value={value}
      onChange={onChange}
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
    'h-9 rounded-none !border-0 !bg-transparent py-0 text-[14px] font-semibold !shadow-none !ring-0 hover:!border-transparent hover:!bg-transparent hover:!shadow-none';

  return (
    <div className="grid h-10 flex-none grid-cols-[minmax(0,1fr)_40px_minmax(0,1fr)] items-center overflow-visible rounded-[14px] border border-slate-200 bg-slate-50/90">
      <LanguageSelect
        options={LANGUAGES.map((lang) => ({
          value: lang.code,
          label: lang.name,
        }))}
        value={sourceLang}
        onChange={onSourceChange}
        buttonClassName={mergedSelectButtonClassName}
      />
      <button
        type="button"
        onClick={onSwap}
        disabled={sourceLang === 'auto'}
        className="grid h-full w-full place-items-center border-x border-slate-200 text-slate-600 transition-colors duration-150 hover:bg-white disabled:cursor-not-allowed disabled:opacity-45"
        title="Swap languages"
        aria-label="Swap languages"
      >
        <SwapIcon className="h-4 w-4" />
      </button>
      <LanguageSelect
        options={LANGUAGES.filter((lang) => lang.code !== 'auto').map((lang) => ({
          value: lang.code,
          label: lang.name,
        }))}
        value={targetLang}
        onChange={onTargetChange}
        buttonClassName={mergedSelectButtonClassName}
      />
    </div>
  );
}

function getLanguageDisplayName(languageCode: string) {
  return LANGUAGE_DISPLAY_NAMES[languageCode] ?? languageCode;
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
  const ocrImageTextAreaRef = useRef<HTMLTextAreaElement>(null);
  const ocrTextAreaRef = useRef<HTMLTextAreaElement>(null);
  const resultWindowScrollIdleTimeoutRef = useRef<number | null>(null);
  const [sourceTextAreaHeightPx, setSourceTextAreaHeightPx] = useState<
    number | undefined
  >(undefined);
  const [isResultWindowPinned, setResultWindowPinned] = useState(false);
  const [isResultWindowScrolling, setResultWindowScrolling] = useState(false);
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
    setResultWindowScrolling(false);
    if (resultWindowScrollIdleTimeoutRef.current !== null) {
      window.clearTimeout(resultWindowScrollIdleTimeoutRef.current);
      resultWindowScrollIdleTimeoutRef.current = null;
    }
  }, [resultWindowVisible]);

  useEffect(() => {
    return () => {
      if (resultWindowScrollIdleTimeoutRef.current !== null) {
        window.clearTimeout(resultWindowScrollIdleTimeoutRef.current);
      }
    };
  }, []);

  useEffect(() => {
    if (!resultWindowVisible || presentation !== 'standalone') return;

    const panelHeight =
      resultWindowMode === 'translation' ? translationLayout.windowHeightPx : 660;

    void getCurrentWindow().setSize(
      new LogicalSize(660, resultWindowStandaloneWindowHeight(panelHeight)),
    );
  }, [
    presentation,
    resultWindowMode,
    resultWindowVisible,
    translationLayout.windowHeightPx,
  ]);

  useEffect(() => {
    if (!resultWindowVisible) return;

    void loadTranslationProviders();
  }, [loadTranslationProviders, resultWindowVisible]);

  useLayoutEffect(() => {
    if (!resultWindowVisible) return;

    const measuredHeight = autosizeResultWindowTextArea(sourceTextAreaRef.current, {
      minHeightPx: resultWindowTextAreaMinHeightPx('source'),
      textStyle: sourceTextStyle,
    });

    if (measuredHeight !== null) {
      setSourceTextAreaHeightPx((currentHeight) =>
        currentHeight === measuredHeight ? currentHeight : measuredHeight,
      );
    }
  }, [resultWindowVisible, sourceText, sourceTextStyle]);

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

  const handleResultWindowScroll = () => {
    setResultWindowScrolling(true);

    if (resultWindowScrollIdleTimeoutRef.current !== null) {
      window.clearTimeout(resultWindowScrollIdleTimeoutRef.current);
    }

    resultWindowScrollIdleTimeoutRef.current = window.setTimeout(() => {
      setResultWindowScrolling(false);
      resultWindowScrollIdleTimeoutRef.current = null;
    }, RESULT_WINDOW_SCROLLBAR_IDLE_DELAY_MS);
  };

  const handleSwapLanguages = () => {
    if (sourceLang !== 'auto') {
      const temp = sourceLang;
      setSourceLang(targetLang);
      setTargetLang(temp);
    }
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
      ? `min(${translationLayout.windowHeightPx}px, 90vh)`
      : `${translationLayout.windowHeightPx}px`;

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
          <div
            className={resultWindowContentClassName({
              isScrolling: isResultWindowScrolling,
            })}
            onScroll={handleResultWindowScroll}
          >
            {ocrImageBase64 ? (
              <div className={resultWindowOcrResultGridClassName()}>
                <div className="flex min-h-0 flex-col gap-2">
                  <div className="flex min-h-[18px] items-center justify-between gap-2.5">
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

                <div className="flex min-h-0 flex-col gap-2">
                  <div className="flex min-h-[18px] items-center justify-between gap-2.5">
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
                    <div className="flex min-h-8 items-center justify-between gap-2.5 border-t border-slate-100 bg-slate-50/80 px-2.5 py-1">
                      <span className="text-[11px] text-slate-400">
                        {ocrText.length} chars
                      </span>
                      <div className="flex items-center gap-2">
                        <IconButton
                          title="复制"
                          disabled={!ocrText.trim()}
                          onClick={() => {
                            void navigator.clipboard.writeText(ocrText);
                          }}
                        >
                          <CopyIcon className="h-4 w-4" />
                        </IconButton>
                        <IconButton
                          title="清空"
                          disabled={!ocrText}
                          onClick={() => {
                            setOcrText('');
                            setOcrImageBase64(null);
                          }}
                        >
                          <ClearTextIcon className="h-4 w-4" />
                        </IconButton>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            ) : (
              <>
                <div className="space-y-2.5">
                  <div className="flex min-h-[18px] items-center justify-between gap-2.5">
                    <label className="text-[13px] font-bold text-slate-600">图片</label>
                    <span className="text-[11px] text-slate-400">PNG/JPG/WebP</span>
                  </div>
                  <button
                    type="button"
                    onClick={handleUploadImage}
                    disabled={isOcrRunning}
                    className="grid min-h-[92px] w-full place-items-center rounded-[14px] border border-dashed border-slate-300 bg-gradient-to-b from-slate-50 to-white px-2.5 py-2.5 text-center transition-colors duration-150 hover:border-emerald-300 hover:bg-emerald-50/30 disabled:cursor-not-allowed disabled:opacity-70"
                  >
                    <span className="grid justify-items-center gap-2">
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
                  <div className="rounded-[14px] border border-red-200 bg-red-50 px-2.5 py-2.5 text-sm font-medium text-red-700">
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

                <div className="min-h-0 space-y-2.5">
                  <div className="flex min-h-[18px] items-center justify-between gap-2.5">
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
                    <div className="flex min-h-8 items-center justify-between gap-2.5 border-t border-slate-100 bg-slate-50/80 px-2.5 py-1">
                      <span className="text-[11px] text-slate-400">
                        {ocrText.length} chars
                      </span>
                      <div className="flex items-center gap-2">
                        <IconButton
                          title="复制"
                          disabled={!ocrText.trim()}
                          onClick={() => {
                            void navigator.clipboard.writeText(ocrText);
                          }}
                        >
                          <CopyIcon className="h-4 w-4" />
                        </IconButton>
                        <IconButton
                          title="清空"
                          disabled={!ocrText}
                          onClick={() => setOcrText('')}
                        >
                          <ClearTextIcon className="h-4 w-4" />
                        </IconButton>
                      </div>
                    </div>
                  </div>
                </div>
              </>
            )}
          </div>
        ) : (
          <div
            className={resultWindowContentClassName({
              isScrolling: isResultWindowScrolling,
              reserveBottom: providerTranslations.length === 0,
            })}
            onScroll={handleResultWindowScroll}
          >
            <div className="flex-none">
              <div className={resultWindowTextBoxClassName()}>
                <textarea
                  ref={sourceTextAreaRef}
                  value={sourceText}
                  onChange={(e) => setSourceText(e.target.value)}
                  rows={resultWindowTextAreaRows(sourceText, 'source')}
                  placeholder="输入需要翻译的文本..."
                  className={resultWindowTextAreaClassName('source')}
                  style={sourceTextStyle}
                />
                <div className="flex min-h-8 items-center justify-between gap-2.5 border-t border-slate-100 bg-slate-50/80 px-2.5 py-1">
                  <span className="min-w-0 truncate text-[11px] text-slate-400">
                    {sourceText.length} chars · 识别为{' '}
                    <span className="font-semibold text-blue-600">
                      {getLanguageDisplayName(detectedSourceLanguage)}
                    </span>
                  </span>
                  <div className="flex items-center gap-2">
                    <IconButton
                      title="复制"
                      disabled={!sourceText.trim()}
                      onClick={() => {
                        void navigator.clipboard.writeText(sourceText);
                      }}
                    >
                      <CopyIcon className="h-4 w-4" />
                    </IconButton>
                    <IconButton
                      title="重试"
                      disabled={!sourceText.trim() || isTranslating}
                      onClick={handleTranslate}
                    >
                      <RetryIcon className="h-4 w-4" />
                    </IconButton>
                    <IconButton
                      title="清空"
                      disabled={!sourceText}
                      onClick={() => setSourceText('')}
                    >
                      <ClearTextIcon className="h-4 w-4" />
                    </IconButton>
                  </div>
                </div>
              </div>
            </div>

            <LanguageSwitcher
              sourceLang={sourceLang}
              targetLang={targetLang}
              onSourceChange={setSourceLang}
              onTargetChange={setTargetLang}
              onSwap={handleSwapLanguages}
            />

            {providerTranslations.length > 0 && (
              <div className="flex min-h-0 flex-1 flex-col">
                <div
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
          </div>
        )}
      </div>
    </div>
  );
}
