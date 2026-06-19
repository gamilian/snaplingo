import { useEffect } from 'react';
import { useAppStore } from '../../stores/appStore';
import { useTranslate } from '../../hooks/useTranslate';
import TranslationCard from './TranslationCard';
import { CustomSelect } from '../common/CustomSelect';

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

export default function ResultWindow() {
  const {
    sourceText,
    sourceLang,
    targetLang,
    translations,
    isTranslating,
    resultWindowVisible,
    pendingAutoTranslate,
    setSourceText,
    setSourceLang,
    setTargetLang,
    consumeAutoTranslateRequest,
    hideResultWindow,
  } = useAppStore();

  const { translate } = useTranslate();

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && resultWindowVisible) {
        hideResultWindow();
      }
    };

    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [resultWindowVisible, hideResultWindow]);

  useEffect(() => {
    if (!resultWindowVisible || !pendingAutoTranslate || !sourceText.trim()) {
      return;
    }

    consumeAutoTranslateRequest();
    void translate(sourceText, sourceLang, targetLang);
  }, [
    consumeAutoTranslateRequest,
    pendingAutoTranslate,
    resultWindowVisible,
    sourceLang,
    sourceText,
    targetLang,
    translate,
  ]);

  if (!resultWindowVisible) return null;

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      hideResultWindow();
    }
  };

  const handleSwapLanguages = () => {
    if (sourceLang !== 'auto') {
      const temp = sourceLang;
      setSourceLang(targetLang);
      setTargetLang(temp);
    }
  };

  const handleTranslate = () => {
    translate();
  };

  return (
    <div
      className="fixed inset-0 bg-black/25 backdrop-blur-sm flex items-center justify-center z-50 p-8"
      onClick={handleOverlayClick}
    >
      <div className="bg-white rounded-2xl shadow-xl w-full max-w-3xl max-h-[90vh] overflow-hidden flex flex-col animate-[slideIn_0.3s_ease-out]">
        {/* Header */}
        <div className="px-6 py-4 bg-gray-50 border-b border-gray-200 flex items-center justify-between">
          <h2 className="text-lg font-bold text-gray-900">翻译</h2>
          <button
            onClick={hideResultWindow}
            className="w-8 h-8 flex items-center justify-center text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-md transition-all duration-150"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-4">
          {/* Source Text */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Source Text
            </label>
            <textarea
              value={sourceText}
              onChange={(e) => setSourceText(e.target.value)}
              placeholder="Enter text to translate..."
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none"
              rows={4}
            />
          </div>

          {/* Language Selection */}
          <div className="flex items-center gap-2">
            <div className="flex-1">
              <label className="block text-sm font-medium text-gray-700 mb-2">
                From
              </label>
              <CustomSelect
                options={LANGUAGES.map(lang => ({ value: lang.code, label: lang.name }))}
                value={sourceLang}
                onChange={setSourceLang}
              />
            </div>

            <button
              onClick={handleSwapLanguages}
              disabled={sourceLang === 'auto'}
              className="mt-7 p-2 rounded-lg hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              title="Swap languages"
            >
              <svg
                className="w-5 h-5 text-gray-600"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"
                />
              </svg>
            </button>

            <div className="flex-1">
              <label className="block text-sm font-medium text-gray-700 mb-2">
                To
              </label>
              <CustomSelect
                options={LANGUAGES.filter(lang => lang.code !== 'auto').map(lang => ({ value: lang.code, label: lang.name }))}
                value={targetLang}
                onChange={setTargetLang}
              />
            </div>
          </div>

          {/* Translate Button */}
          <button
            onClick={handleTranslate}
            disabled={!sourceText.trim() || isTranslating}
            className="w-full py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors"
          >
            {isTranslating ? 'Translating...' : 'Translate'}
          </button>

          {/* Translation Results */}
          {translations.length > 0 && (
            <div className="space-y-3">
              <h3 className="text-sm font-medium text-gray-700">Results</h3>
              {translations.map((result, index) => (
                <TranslationCard
                  key={index}
                  providerId={result.provider_id}
                  text={result.translated_text}
                  detectedLanguage={result.detected_language || undefined}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
