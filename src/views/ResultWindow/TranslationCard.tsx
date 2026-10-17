import { useEffect, useState } from 'react';
import type { ProviderTranslationStatus } from '../../application/result-window/projection';
import {
  ChevronDownIcon,
  ChevronUpIcon,
  CopyIcon,
  FavoriteIcon,
  RetryIcon,
  VolumeIcon,
} from './icons';
import IconActionButton from './IconActionButton';
import { resultWindowAdaptiveTextStyle } from './presentation';

interface TranslationCardProps {
  providerId: string;
  providerName?: string;
  status: ProviderTranslationStatus;
  text: string;
  detectedLanguage?: string;
  languageCode?: string;
  bodyHeightPx?: number;
  onRetry?: () => void;
  onFavorite?: () => Promise<void>;
  isFavorite?: boolean;
  copyText: (text: string) => Promise<void>;
  speakText: (text: string, language?: string) => Promise<void>;
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
  onFavorite,
  isFavorite = false,
  copyText,
  speakText,
}: TranslationCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [isFavoritePending, setIsFavoritePending] = useState(false);

  const providerColor =
    providerColors[providerId.toLowerCase()] || 'rgb(var(--muted))';
  const isPending = status === 'pending';
  const isError = status === 'error';
  const displayText = text.trim();

  useEffect(() => {
    setIsFavoritePending(false);
  }, [providerId, text]);

  return (
    <div className="overflow-hidden rounded-xl border border-line bg-surface">
      <div className="flex min-h-9 items-center justify-between gap-3 border-b border-line bg-sunken/80 px-3 py-2">
        <button
          type="button"
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex min-w-0 flex-1 items-center gap-3 text-left"
        >
          <div
            className="h-5 w-1 shrink-0 rounded-full"
            style={{ background: providerColor }}
          />
          <span className="truncate text-card font-semibold text-ink">
            {providerName || providerId}
          </span>
          {isPending && (
            <span className="shrink-0 rounded-md bg-blue-50 px-2 py-0.5 text-hint font-semibold text-blue-700">
              翻译中
            </span>
          )}
          {isError && (
            <span className="shrink-0 rounded-md bg-red-50 px-2 py-0.5 text-hint font-semibold text-red-700">
              失败
            </span>
          )}
        </button>

        <div className="flex shrink-0 items-center gap-2">
          {onRetry && !isPending && (
            <IconActionButton
              title="重试"
              className="grid h-6 w-6 place-items-center rounded-md border border-line bg-surface text-muted transition-colors duration-150 hover:border-line-strong hover:bg-sunken hover:text-ink"
              tooltipPlacement="bottom"
              onClick={(event) => {
                event.stopPropagation();
                onRetry();
              }}
            >
              <RetryIcon className="h-[15px] w-[15px]" />
            </IconActionButton>
          )}

          {!isError && (
            <IconActionButton
              title="朗读"
              className="grid h-6 w-6 place-items-center rounded-md border border-line bg-surface text-muted transition-colors duration-150 hover:border-line-strong hover:bg-sunken hover:text-ink"
              tooltipPlacement="bottom"
              onClick={(event) => {
                event.stopPropagation();
                void speakText(displayText, languageCode);
              }}
            >
              <VolumeIcon className="h-[15px] w-[15px]" />
            </IconActionButton>
          )}

          {onFavorite && !isPending && !isError && (
            <IconActionButton
              title={isFavorite ? '已收藏' : '收藏'}
              aria-pressed={isFavorite}
              disabled={isFavorite || isFavoritePending || !displayText}
              className={`grid h-6 w-6 place-items-center rounded-md border transition-colors duration-150 disabled:cursor-not-allowed ${
                isFavorite
                  ? 'border-amber-200 bg-amber-50 text-amber-600'
                  : 'border-line bg-surface text-muted hover:border-amber-200 hover:bg-amber-50 hover:text-amber-600'
              }`}
              tooltipPlacement="bottom"
              onClick={(event) => {
                event.stopPropagation();
                setIsFavoritePending(true);
                void onFavorite()
                  .catch((error) => {
                    console.error('Failed to favorite translation result:', error);
                  })
                  .finally(() => setIsFavoritePending(false));
              }}
            >
              <FavoriteIcon
                className="h-[15px] w-[15px]"
                fill={isFavorite ? 'currentColor' : 'none'}
              />
            </IconActionButton>
          )}

          {!isError && (
            <IconActionButton
              title="复制"
              className="grid h-6 w-6 place-items-center rounded-md border border-line bg-surface text-muted transition-colors duration-150 hover:border-line-strong hover:bg-sunken hover:text-ink"
              tooltipPlacement="bottom"
              onClick={(event) => {
                event.stopPropagation();
                void copyText(displayText);
              }}
            >
              <CopyIcon className="h-[15px] w-[15px]" />
            </IconActionButton>
          )}

          <IconActionButton
            title={isExpanded ? '收起' : '展开'}
            className="grid h-6 w-6 place-items-center rounded-md border border-line bg-surface text-muted transition-colors duration-150 hover:border-line-strong hover:bg-sunken hover:text-ink"
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
        <div className="bg-surface">
          {isPending ? (
            <div
              className="space-y-2 overflow-hidden px-3 py-2"
              style={{
                height: `${bodyHeightPx}px`,
                maxHeight: `${bodyHeightPx}px`,
              }}
              aria-label="翻译中"
            >
              <div className="h-2.5 w-11/12 rounded-full bg-raised animate-pulse" />
              <div className="h-2.5 w-3/4 rounded-full bg-raised animate-pulse" />
              <div className="h-2.5 w-1/2 rounded-full bg-raised animate-pulse" />
            </div>
          ) : (
            <p
              className={`whitespace-pre-wrap break-words px-3 py-2 text-card leading-[1.38] ${
                isError ? 'text-red-700' : 'text-ink'
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
