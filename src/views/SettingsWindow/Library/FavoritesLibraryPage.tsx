import { useEffect, useMemo, useState } from 'react';
import type {
  FavoriteLibraryItem,
  LibraryFavoritesFilter,
} from '../../../application/settings/library';
import { useFavoritesStore } from '../../../stores/favoritesStore';
import { useScreenshotFavoritesStore } from '../../../stores/screenshotFavoritesStore';
import { formatRelativeTime } from '../../../utils/formatTime';
import { FavoriteMetadataEditor } from '../FavoriteMetadataEditor';
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

const filters: LibraryFilter<LibraryFavoritesFilter>[] = [
  { key: 'all', label: '全部' },
  { key: 'translation', label: '翻译' },
  { key: 'ocr', label: 'OCR' },
  { key: 'screenshot', label: '截图' },
];

export function FavoritesLibraryPage() {
  const runtime = useSettingsRuntime();
  const favoriteRevision = useFavoritesStore((state) => state.revision);
  const screenshotRevision = useScreenshotFavoritesStore(
    (state) => state.revision,
  );
  const [filter, setFilter] = useState<LibraryFavoritesFilter>('all');
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(0);
  const [items, setItems] = useState<FavoriteLibraryItem[]>([]);
  const [total, setTotal] = useState(0);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [refresh, setRefresh] = useState(0);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void runtime.library
      .queryFavorites({ filter, search, page, pageSize: PAGE_SIZE })
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
  }, [favoriteRevision, filter, page, refresh, runtime, screenshotRevision, search]);

  const selected = useMemo(
    () => items.find((item) => item.key === selectedKey) ?? null,
    [items, selectedKey],
  );

  const deleteSelected = async () => {
    if (!selected) return;
    await runtime.library.deleteFavorite(selected);
    setRefresh((value) => value + 1);
  };

  return (
    <LibraryLayout
      title="收藏夹"
      total={total}
      search={search}
      searchPlaceholder="搜索收藏的内容、备注或标签"
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
      list={
        loading && items.length === 0 ? (
          <LibraryEmptyState>正在加载收藏夹…</LibraryEmptyState>
        ) : items.length === 0 ? (
          <LibraryEmptyState>
            {search ? '没有匹配的收藏' : '暂无收藏内容'}
          </LibraryEmptyState>
        ) : (
          items.map((item) => (
            <LibraryListItem
              key={item.key}
              active={item.key === selectedKey}
              kind={favoriteKindLabel(item.kind)}
              kindTone={favoriteKindTone(item.kind)}
              time={formatRelativeTime(item.timestamp)}
              title={favoriteTitle(item)}
              preview={favoritePreview(item)}
              onClick={() => setSelectedKey(item.key)}
            />
          ))
        )
      }
      detail={
        selected ? (
          <FavoriteDetail
            item={selected}
            onDelete={() => void deleteSelected()}
            onCopy={(text) => void runtime.clipboard.copyText(text)}
            onRerunOcr={(id) =>
              void runtime.library.rerunFavoriteOcrAndCopy(id)
            }
            onCopyScreenshot={(id) =>
              void runtime.screenshotFavorites.copy(id)
            }
            onRevealScreenshot={(id) =>
              void runtime.screenshotFavorites.reveal(id)
            }
            onUpdateMetadata={async (note, tags) => {
              await runtime.library.updateFavoriteMetadata(selected, note, tags);
              setRefresh((value) => value + 1);
            }}
          />
        ) : (
          <LibraryEmptyState>选择一条收藏查看详情</LibraryEmptyState>
        )
      }
    />
  );
}

function FavoriteDetail({
  item,
  onDelete,
  onCopy,
  onRerunOcr,
  onCopyScreenshot,
  onRevealScreenshot,
  onUpdateMetadata,
}: {
  item: FavoriteLibraryItem;
  onDelete: () => void;
  onCopy: (text: string) => void;
  onRerunOcr: (id: number) => void;
  onCopyScreenshot: (id: number) => void;
  onRevealScreenshot: (id: number) => void;
  onUpdateMetadata: (note: string | null, tags: string[]) => Promise<void>;
}) {
  const createdAt = item.entry.createdAt;
  const metadata = (
    <DetailCard label="收藏信息" meta="自动保存">
      <FavoriteMetadataEditor
        note={item.entry.note ?? undefined}
        tags={item.entry.tags}
        onSaveNote={(note) =>
          onUpdateMetadata(note.trim() ? note : null, item.entry.tags)
        }
        onSaveTags={(tags) => onUpdateMetadata(item.entry.note, tags)}
      />
    </DetailCard>
  );

  if (item.kind === 'screenshot') {
    return (
      <div className="mx-auto max-w-[820px]">
        <DetailHeader
          title={formatDetailDate(createdAt)}
          subtitle={`截图 · ${item.entry.width} × ${item.entry.height}`}
          actions={
            <>
              <DetailActionButton
                title="取消收藏"
                tone="favorite"
                onClick={onDelete}
              >
                ★
              </DetailActionButton>
              <DetailActionButton
                title="删除"
                tone="danger"
                onClick={onDelete}
              >
                ⌫
              </DetailActionButton>
            </>
          }
        />
        <DetailCard
          label="截图预览"
          meta={`${item.entry.width} × ${item.entry.height}`}
          actions={
            <>
              <SmallActionButton
                onClick={() => onCopyScreenshot(item.entry.id)}
              >
                复制图片
              </SmallActionButton>
              <SmallActionButton
                onClick={() => onRevealScreenshot(item.entry.id)}
              >
                显示文件
              </SmallActionButton>
            </>
          }
        >
          <img
            src={item.entry.thumbnailDataUrl}
            alt="收藏截图"
            className="max-h-[360px] w-full rounded-lg bg-gray-50 object-contain"
          />
        </DetailCard>
        {metadata}
      </div>
    );
  }

  if (item.entry.content.contentKind === 'translation') {
    const snapshot = item.entry.content.snapshot;
    return (
      <div className="mx-auto max-w-[820px]">
        <DetailHeader
          title={formatDetailDate(createdAt)}
          subtitle={`翻译 · ${snapshot.sourceLang} → ${snapshot.targetLang}`}
          actions={
            <>
              <DetailActionButton
                title="取消收藏"
                tone="favorite"
                onClick={onDelete}
              >
                ★
              </DetailActionButton>
              <DetailActionButton
                title="删除"
                tone="danger"
                onClick={onDelete}
              >
                ⌫
              </DetailActionButton>
            </>
          }
        />
        <DetailCard
          label="原文"
          actions={
            <SmallActionButton onClick={() => onCopy(snapshot.sourceText)}>
              复制
            </SmallActionButton>
          }
        >
          <p className="whitespace-pre-wrap text-sm leading-7 text-gray-800">
            {snapshot.sourceText}
          </p>
        </DetailCard>
        <DetailCard
          label="译文"
          meta={snapshot.result.provider_id}
          actions={
            <SmallActionButton
              onClick={() => onCopy(snapshot.result.translated_text)}
            >
              复制
            </SmallActionButton>
          }
        >
          <p className="whitespace-pre-wrap text-sm leading-7 text-gray-800">
            {snapshot.result.translated_text}
          </p>
        </DetailCard>
        {metadata}
      </div>
    );
  }

  const snapshot = item.entry.content.snapshot;
  return (
    <div className="mx-auto max-w-[820px]">
      <DetailHeader
        title={formatDetailDate(createdAt)}
        subtitle={`OCR · ${snapshot.language ?? '自动识别'} · ${snapshot.providerUsed}`}
        actions={
          <>
            <DetailActionButton
              title="取消收藏"
              tone="favorite"
              onClick={onDelete}
            >
              ★
            </DetailActionButton>
            <DetailActionButton title="删除" tone="danger" onClick={onDelete}>
              ⌫
            </DetailActionButton>
          </>
        }
      />
      {item.entry.thumbnailDataUrl && (
        <DetailCard label="原图">
          <img
            src={item.entry.thumbnailDataUrl}
            alt="OCR 收藏原图"
            className="max-h-64 w-full rounded-lg bg-gray-50 object-contain"
          />
        </DetailCard>
      )}
      <DetailCard
        label="识别文本"
        actions={
          <>
            <SmallActionButton onClick={() => onCopy(snapshot.recognizedText)}>
              复制
            </SmallActionButton>
            {snapshot.sourceAssetPath && (
              <SmallActionButton onClick={() => onRerunOcr(item.entry.id)}>
                重新识别并复制
              </SmallActionButton>
            )}
          </>
        }
      >
        <p className="whitespace-pre-wrap text-sm leading-7 text-gray-800">
          {snapshot.recognizedText}
        </p>
      </DetailCard>
      {metadata}
    </div>
  );
}

function favoriteKindLabel(kind: FavoriteLibraryItem['kind']) {
  if (kind === 'translation') return '翻译';
  if (kind === 'ocr') return 'OCR';
  return '截图';
}

function favoriteKindTone(kind: FavoriteLibraryItem['kind']) {
  if (kind === 'translation') return 'blue' as const;
  if (kind === 'ocr') return 'purple' as const;
  return 'green' as const;
}

function favoriteTitle(item: FavoriteLibraryItem) {
  if (item.kind === 'screenshot') {
    return item.entry.note?.trim() || `截图 ${item.entry.width} × ${item.entry.height}`;
  }
  if (item.entry.content.contentKind === 'translation') {
    return firstMeaningfulLine(item.entry.content.snapshot.sourceText);
  }
  return firstMeaningfulLine(item.entry.content.snapshot.recognizedText);
}

function favoritePreview(item: FavoriteLibraryItem) {
  if (item.kind === 'screenshot') {
    return item.entry.tags.length > 0
      ? item.entry.tags.join(' · ')
      : `${item.entry.width} × ${item.entry.height}`;
  }
  if (item.entry.content.contentKind === 'translation') {
    const snapshot = item.entry.content.snapshot;
    return `${snapshot.result.provider_id} · ${snapshot.sourceLang} → ${snapshot.targetLang}`;
  }
  const snapshot = item.entry.content.snapshot;
  return `${snapshot.providerUsed} · ${snapshot.language ?? '自动识别'}`;
}

function firstMeaningfulLine(value: string) {
  return (
    value
      .split(/\r?\n/)
      .find((line) => line.trim())
      ?.trim() || '空白收藏'
  );
}

function formatDetailDate(value: string) {
  return new Date(value).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });
}
