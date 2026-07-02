import { useState } from 'react';
import type { ProviderTranslationStatus } from '../../stores/appStore';

interface TranslationCardProps {
  providerId: string;
  providerName?: string;
  status: ProviderTranslationStatus;
  text: string;
  detectedLanguage?: string;
  onRetry?: () => void;
}

// Provider品牌色映射
const providerColors: Record<string, string> = {
  'google': '#4285f4',
  'deeplx': '#0f2b46',
  'baidu': '#2932e1',
  'openai': '#10a37f',
};

export default function TranslationCard({
  providerId,
  providerName,
  status,
  text,
  detectedLanguage,
  onRetry,
}: TranslationCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);

  const providerColor = providerColors[providerId.toLowerCase()] || '#6b7280';
  const isPending = status === 'pending';
  const isError = status === 'error';

  return (
    <div className="border border-gray-200 rounded-lg overflow-hidden transition-all duration-150 hover:shadow-sm">
      <div className="w-full px-4 py-3 bg-gray-50 flex items-center justify-between gap-3">
        <button
          type="button"
          onClick={() => setIsExpanded(!isExpanded)}
          className="min-w-0 flex-1 flex items-center gap-2 text-left"
        >
          {/* Provider品牌色条 */}
          <div
            className="w-1 h-4 rounded shrink-0"
            style={{ background: providerColor }}
          />
          <span className="font-semibold text-gray-900 truncate">
            {providerName || providerId}
          </span>
          {isPending && (
            <span className="text-xs text-blue-600 bg-blue-50 px-2 py-1 rounded shrink-0">
              翻译中
            </span>
          )}
          {isError && (
            <span className="text-xs text-red-600 bg-red-50 px-2 py-1 rounded shrink-0">
              失败
            </span>
          )}
          {detectedLanguage && (
            <span className="text-xs text-gray-500 bg-gray-200 px-2 py-1 rounded shrink-0">
              Detected: {detectedLanguage}
            </span>
          )}
        </button>

        <div className="flex items-center gap-2 shrink-0">
          {onRetry && !isPending && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onRetry();
              }}
              className="p-1 px-2 bg-white border border-gray-200 rounded hover:bg-gray-50 transition-colors"
              title="重试"
              aria-label="重试"
            >
              <svg className="w-4 h-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 16 16">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 6a5 5 0 1 0 1 3M13 6V2m0 4H9"/>
              </svg>
            </button>
          )}

          {/* 朗读按钮 */}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              console.log('朗读:', text);
            }}
            className="p-1 px-2 bg-white border border-gray-200 rounded hover:bg-gray-50 transition-colors"
            title="朗读"
          >
            <svg className="w-4 h-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 16 16">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 4L4 7H2v2h2l3 3V4zM13 8c0-1.5-1-3-2-3M15 8c0-2.5-1.5-5-3-5"/>
            </svg>
          </button>

          {/* 复制按钮 */}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              navigator.clipboard.writeText(text);
            }}
            className="p-1 px-2 bg-white border border-gray-200 rounded hover:bg-gray-50 transition-colors"
            title="复制"
          >
            <svg className="w-4 h-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 16 16">
              <rect x="4" y="4" width="8" height="8" rx="1"/>
              <path d="M8 4V2a1 1 0 0 1 1-1h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-2"/>
            </svg>
          </button>

          {/* 展开/收起图标 */}
          <button
            type="button"
            onClick={() => setIsExpanded(!isExpanded)}
            className="p-1 rounded hover:bg-gray-100 transition-colors"
            title={isExpanded ? '收起' : '展开'}
            aria-label={isExpanded ? '收起' : '展开'}
          >
            <svg
              className={`w-5 h-5 text-gray-500 transition-transform duration-150 ${
                isExpanded ? 'rotate-180' : ''
              }`}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M19 9l-7 7-7-7"
              />
            </svg>
          </button>
        </div>
      </div>

      {isExpanded && (
        <div className="p-4 bg-white animate-[expandIn_0.2s_ease-out]">
          {isPending ? (
            <div className="space-y-2" aria-label="翻译中">
              <div className="h-4 w-4/5 rounded bg-gray-100 animate-pulse" />
              <div className="h-4 w-2/3 rounded bg-gray-100 animate-pulse" />
            </div>
          ) : (
            <p
              className={`whitespace-pre-wrap leading-relaxed ${
                isError ? 'text-red-700' : 'text-gray-800'
              }`}
            >
              {text}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
