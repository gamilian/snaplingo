import { useEffect, useMemo, useState } from 'react';
import { ocrFavoriteKey, translationFavoriteKey } from '../../../application/favorites/identity';
import type {
  HistoryLibraryItem,
  LibraryHistoryFilter,
} from '../../../application/settings/library';
import { useFavoritesStore } from '../../../stores/favoritesStore';
import { useHistoryStore } from '../../../stores/historyStore';
import { formatRelativeTime } from '../../../utils/formatTime';
import { useSettingsRuntime } from '../runtimeContext';
import {
  DetailActionButton,
  DetailCard,
  DetailHeader,
  LibraryEmptyState,
  LibraryLayout,
  LibraryListItem,
  SmallActionButton,
  type LibraryFilter,
} from './LibraryLayout';

const PAGE_SIZE = 20;

const filters: LibraryFilter<LibraryHistoryFilter>[] = [
  { key: 'all', label: '全部' },
  { key: 'translation', label: '翻译' },
  { key: 'ocr', label: 'OCR' },
];

export function HistoryLibraryPage() {
  const runtime = useSettingsRuntime();
  const revision = useHistoryStore((state) => state.revision);
  const favoriteKeys = useFavoritesStore((state) => state.keys);
  const hydrateFavoriteKeys = useFavoritesStore((state) => state.hydrateKeys);
  const addTranslationFavorite = useFavoritesStore((state) => state.addTranslation);
  const addOcrFavorite = useFavoritesStore((state) => state.addOcr);
  const [filter, setFilter] = useState<LibraryHistoryFilter>('all');
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(0);
  const [items, setItems] = useState<HistoryLibraryItem[]>([]);
  const [total, setTotal] = useState(0);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [refresh, setRefresh] = useState(0);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    void Promise.all([
      hydrateFavoriteKeys('translation'),
      hydrateFavoriteKeys('ocr'),
    ]);
  }, [hydrateFavoriteKeys]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void runtime.library
      .queryHistory({ filter, search, page, pageSize: PAGE_SIZE })
      .then((result) => {
        if (cancelled) return;
        setItems(result.items);
        setTotal(result.total);
        setSelectedKey((current) =>
          result.items.some((item) => item.key === current)
            ? current
            : result.items[0]?.key ?? null,
        );
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [filter, page, refresh, revision, runtime, search]);

  const selected = useMemo(
    () => items.find((item) => item.key === selectedKey) ?? null,
    [items, selectedKey],
  );

  const deleteSelected = async () => {
    if (!selected) return;
    await runtime.library.deleteHistory(selected);
    setRefresh((value) => value + 1);
  };

  const clearVisibleKind = async () => {
    const label =
      filter === 'all' ? '全部' : filter === 'translation' ? '翻译' : 'OCR';
    if (!confirm(`确定要清空${label}历史记录吗？独立收藏不会被删除。`)) return;
    await runtime.library.clearHistory(filter);
    setRefresh((value) => value + 1);
  };

  return (
    <LibraryLayout
      title="历史记录"
      total={total}
      search={search}
      searchPlaceholder="搜索历史记录"
      onSearchChange={(value) => {
        setSearch(value);
        setPage(0);
      }}
      filters={filters}
      activeFilter={filter}
      onFilterChange={(value) => {
        setFilter(value);
        setPage(0);
      }}
      page={page}
      pageSize={PAGE_SIZE}
      onPageChange={setPage}
      footerAction={
        total > 0 ? (
          <button
            type="button"
            onClick={() => void clearVisibleKind()}
            className="text-gray-400 hover:text-red-600"
          >
            清空记录
          </button>
        ) : null
      }
      list={
        loading && items.length === 0 ? (
          <LibraryEmptyState>正在加载历史记录…</LibraryEmptyState>
        ) : items.length === 0 ? (
          <LibraryEmptyState>
            {search ? '没有匹配的历史记录' : '暂无历史记录'}
          </LibraryEmptyState>
        ) : (
          items.map((item) => (
            <LibraryListItem
              key={item.key}
              active={item.key === selectedKey}
              kind={item.kind === 'translation' ? '翻译' : 'OCR'}
              kindTone={item.kind === 'translation' ? 'blue' : 'purple'}
              time={formatRelativeTime(item.timestamp)}
              title={historyTitle(item)}
              preview={historyPreview(item)}
              onClick={() => setSelectedKey(item.key)}
            />
          ))
        )
      }
      detail={
        selected ? (
          selected.kind === 'translation' ? (
            <TranslationHistoryDetail
              item={selected}
              favoriteKeys={favoriteKeys}
              onCopy={(text) => void runtime.clipboard.copyText(text)}
              onDelete={() => void deleteSelected()}
              onFavorite={(input) =>
                void addTranslationFavorite(input).catch(showFavoriteError)
              }
            />
          ) : (
            <OcrHistoryDetail
              item={selected}
              isFavorite={favoriteKeys.has(
                ocrFavoriteKey({
                  recognizedText: selected.entry.recognizedText,
                  language: selected.entry.language,
                  providerUsed: selected.entry.providerUsed,
                }),
              )}
              onCopy={() =>
                void runtime.clipboard.copyText(selected.entry.recognizedText)
              }
              onRerun={() =>
                void runtime.library.rerunHistoryOcrAndCopy(selected.entry.id)
              }
              onDelete={() => void deleteSelected()}
              onFavorite={() =>
                void addOcrFavorite({
                  sourceHistoryId: selected.entry.id,
                  recognizedText: selected.entry.recognizedText,
                  language: selected.entry.language,
                  providerUsed: selected.entry.providerUsed,
                  confidence: selected.entry.confidence,
                }).catch(showFavoriteError)
              }
            />
          )
        ) : (
          <LibraryEmptyState>选择一条记录查看详情</LibraryEmptyState>
        )
      }
    />
  );
}

function showFavoriteError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  window.alert(`收藏失败：${message}`);
}

function TranslationHistoryDetail({
  item,
  favoriteKeys,
  onCopy,
  onDelete,
  onFavorite,
}: {
  item: Extract<HistoryLibraryItem, { kind: 'translation' }>;
  favoriteKeys: Set<string>;
  onCopy: (text: string) => void;
  onDelete: () => void;
  onFavorite: (input: {
    sourceHistoryId: number;
    sourceText: string;
    sourceLang: string;
    targetLang: string;
    providerId: string;
    translatedText: string;
    detectedLanguage: string | null;
    confidence: number | null;
  }) => void;
}) {
  const entry = item.entry;
  return (
    <div className="mx-auto max-w-[820px]">
      <DetailHeader
        title={formatDetailDate(item.timestamp)}
        subtitle={`翻译 · ${entry.sourceLang} → ${entry.targetLang}`}
        actions={
          <DetailActionButton title="删除" tone="danger" onClick={onDelete}>
            ⌫
          </DetailActionButton>
        }
      />
      <DetailCard
        label="原文"
        meta={`${entry.sourceText.length} chars`}
        actions={
          <SmallActionButton onClick={() => onCopy(entry.sourceText)}>
            复制
          </SmallActionButton>
        }
      >
        <p className="whitespace-pre-wrap text-sm leading-7 text-gray-800">
          {entry.sourceText}
        </p>
      </DetailCard>
      <div className="mb-3 flex items-center gap-2 text-xs text-gray-500">
        <span className="rounded-lg bg-white px-3 py-2 shadow-sm">
          {entry.sourceLang}
        </span>
        <span>→</span>
        <span className="rounded-lg bg-white px-3 py-2 shadow-sm">
          {entry.targetLang}
        </span>
      </div>
      {entry.results.map((result) => {
        const input = {
          sourceHistoryId: entry.id,
          sourceText: entry.sourceText,
          sourceLang: entry.sourceLang,
          targetLang: entry.targetLang,
          providerId: result.providerId,
          translatedText: result.translatedText,
          detectedLanguage: result.detectedLanguage,
          confidence: result.confidence,
        };
        const favorite = favoriteKeys.has(translationFavoriteKey(input));
        return (
          <DetailCard
            key={result.providerId}
            label="译文"
            meta={result.providerId}
            actions={
              <>
                <SmallActionButton onClick={() => onCopy(result.translatedText)}>
                  复制
                </SmallActionButton>
                <SmallActionButton
                  onClick={() => {
                    if (!favorite) onFavorite(input);
                  }}
                >
                  {favorite ? '已收藏' : '收藏'}
                </SmallActionButton>
              </>
            }
          >
            <p className="whitespace-pre-wrap text-sm leading-7 text-gray-800">
              {result.translatedText}
            </p>
          </DetailCard>
        );
      })}
    </div>
  );
}

function OcrHistoryDetail({
  item,
  isFavorite,
  onCopy,
  onRerun,
  onDelete,
  onFavorite,
}: {
  item: Extract<HistoryLibraryItem, { kind: 'ocr' }>;
  isFavorite: boolean;
  onCopy: () => void;
  onRerun: () => void;
  onDelete: () => void;
  onFavorite: () => void;
}) {
  const entry = item.entry;
  return (
    <div className="mx-auto max-w-[820px]">
      <DetailHeader
        title={formatDetailDate(item.timestamp)}
        subtitle={`OCR · ${entry.language ?? '自动识别'} · ${entry.providerUsed}`}
        actions={
          <>
            <DetailActionButton
              title={isFavorite ? '已收藏' : '收藏'}
              tone={isFavorite ? 'favorite' : 'default'}
              disabled={isFavorite}
              onClick={onFavorite}
            >
              {isFavorite ? '★' : '☆'}
            </DetailActionButton>
            <DetailActionButton title="删除" tone="danger" onClick={onDelete}>
              ⌫
            </DetailActionButton>
          </>
        }
      />
      {entry.thumbnailDataUrl && (
        <DetailCard label="原图">
          <img
            src={entry.thumbnailDataUrl}
            alt="OCR 原图"
            className="max-h-64 w-full rounded-lg bg-gray-50 object-contain"
          />
        </DetailCard>
      )}
      <DetailCard
        label="识别文本"
        meta={
          entry.confidence === null
            ? `${entry.recognizedText.length} chars`
            : `置信度 ${Math.round(entry.confidence * 100)}%`
        }
        actions={
          <>
            <SmallActionButton onClick={onCopy}>复制</SmallActionButton>
            <SmallActionButton onClick={onRerun}>重新识别并复制</SmallActionButton>
          </>
        }
      >
        <p className="whitespace-pre-wrap text-sm leading-7 text-gray-800">
          {entry.recognizedText}
        </p>
      </DetailCard>
    </div>
  );
}

function historyTitle(item: HistoryLibraryItem) {
  return firstMeaningfulLine(
    item.kind === 'translation'
      ? item.entry.sourceText
      : item.entry.recognizedText,
  );
}

function historyPreview(item: HistoryLibraryItem) {
  if (item.kind === 'translation') {
    return `${item.entry.results.length} 个服务 · ${item.entry.sourceLang} → ${item.entry.targetLang}`;
  }
  return `${item.entry.providerUsed} · ${item.entry.language ?? '自动识别'}`;
}

function firstMeaningfulLine(value: string) {
  return value.split(/\r?\n/).find((line) => line.trim())?.trim() || '空白记录';
}

function formatDetailDate(timestamp: number) {
  return new Date(timestamp).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });
}
