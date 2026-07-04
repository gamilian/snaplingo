import { useState } from 'react';
import type { ProviderTranslationStatus } from '../../stores/appStore';
import {
  ChevronDownIcon,
  ChevronUpIcon,
  CopyIcon,
  RetryIcon,
  VolumeIcon,
} from './icons';
import IconActionButton from './IconActionButton';
import { resultWindowAdaptiveTextStyle } from './presentation';
import { speakResultWindowText } from './speech';

interface TranslationCardProps {
  providerId: string;
  providerName?: string;
  status: ProviderTranslationStatus;
  text: string;
  detectedLanguage?: string;
  languageCode?: string;
  bodyHeightPx?: number;
  onRetry?: () => void;
}

const providerColors: Record<string, string> = {
  google: '#4285f4',
  deeplx: '#0f2b46',
  baidu: '#2932e1',
  openai: '#10a37f',
};

export default function TranslationCard({
  providerId,
  providerName,
  status,
  text,
  languageCode,
  bodyHeightPx = 62,
  onRetry,
}: TranslationCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);

  const providerColor = providerColors[providerId.toLowerCase()] || '#6b7280';
  const isPending = status === 'pending';
  const isError = status === 'error';
  const displayText = text.trim();

  return (
    <div className="overflow-hidden rounded-[14px] border border-slate-200 bg-white">
      <div className="flex min-h-9 items-center justify-between gap-3 border-b border-slate-100 bg-slate-50/80 px-3 py-2">
        <button
          type="button"
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex min-w-0 flex-1 items-center gap-3 text-left"
        >
          <div
            className="h-5 w-1 shrink-0 rounded-full"
            style={{ background: providerColor }}
          />
          <span className="truncate text-[13px] font-semibold text-slate-900">
            {providerName || providerId}
          </span>
          {isPending && (
            <span className="shrink-0 rounded-[7px] bg-blue-50 px-2 py-0.5 text-[11px] font-semibold text-blue-700">
              翻译中
            </span>
          )}
          {isError && (
            <span className="shrink-0 rounded-[7px] bg-red-50 px-2 py-0.5 text-[11px] font-semibold text-red-700">
              失败
            </span>
          )}
        </button>

        <div className="flex shrink-0 items-center gap-2">
          {onRetry && !isPending && (
            <IconActionButton
              title="重试"
              className="grid h-6 w-6 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700"
              tooltipPlacement="bottom"
              onClick={(event) => {
                event.stopPropagation();
                onRetry();
              }}
            >
              <RetryIcon className="h-[15px] w-[15px]" />
            </IconActionButton>
          )}

          <IconActionButton
            title="朗读"
            className="grid h-6 w-6 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700"
            tooltipPlacement="bottom"
            onClick={(event) => {
              event.stopPropagation();
              void speakResultWindowText(displayText, languageCode);
            }}
          >
            <VolumeIcon className="h-[15px] w-[15px]" />
          </IconActionButton>

          <IconActionButton
            title="复制"
            className="grid h-6 w-6 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700"
            tooltipPlacement="bottom"
            onClick={(event) => {
              event.stopPropagation();
              void navigator.clipboard.writeText(displayText);
            }}
          >
            <CopyIcon className="h-[15px] w-[15px]" />
          </IconActionButton>

          <IconActionButton
            title={isExpanded ? '收起' : '展开'}
            className="grid h-6 w-6 place-items-center rounded-[7px] border border-slate-200 bg-white text-slate-500 transition-colors duration-150 hover:border-slate-300 hover:bg-slate-50 hover:text-slate-700"
            tooltipPlacement="bottom"
            onClick={(event) => {
              event.stopPropagation();
              setIsExpanded(!isExpanded);
            }}
          >
            {isExpanded ? (
              <ChevronUpIcon className="h-[15px] w-[15px]" />
            ) : (
              <ChevronDownIcon className="h-[15px] w-[15px]" />
            )}
          </IconActionButton>
        </div>
      </div>

      {isExpanded && (
        <div className="bg-white">
          {isPending ? (
            <div
              className="space-y-2 overflow-hidden px-3 py-2"
              style={{
                height: `${bodyHeightPx}px`,
                maxHeight: `${bodyHeightPx}px`,
              }}
              aria-label="翻译中"
            >
              <div className="h-2.5 w-11/12 rounded-full bg-slate-100 animate-pulse" />
              <div className="h-2.5 w-3/4 rounded-full bg-slate-100 animate-pulse" />
              <div className="h-2.5 w-1/2 rounded-full bg-slate-100 animate-pulse" />
            </div>
          ) : (
            <p
              className={`whitespace-pre-wrap break-words px-3 py-2 text-[13px] leading-[1.38] ${
                isError ? 'text-red-700' : 'text-slate-800'
              }`}
              style={resultWindowAdaptiveTextStyle(displayText, 'result')}
            >
              {displayText}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
