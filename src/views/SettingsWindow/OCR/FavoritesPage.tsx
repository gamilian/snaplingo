import { useEffect, useState } from 'react';
import { useFavoritesStore } from '../../../stores/favoritesStore';
import { useSettingsRuntime } from '../runtimeContext';
import { formatRelativeTime } from '../../../utils/formatTime';
import IconActionButton from '../../../components/common/IconActionButton';
import { FavoriteMetadataEditor } from '../FavoriteMetadataEditor';
import { HistoryPagination } from '../HistoryPagination';
import type { OcrFavoriteItem } from '../../../application/settings/ports';

export function FavoritesPage() {
  const runtime = useSettingsRuntime();
  const allItems = useFavoritesStore((state) => state.items);
  const favorites = allItems.filter(
    (item): item is OcrFavoriteItem => item.content.contentKind === 'ocr',
  );
  const total = useFavoritesStore((state) => state.total);
  const loadFavorites = useFavoritesStore((state) => state.query);
  const revision = useFavoritesStore((state) => state.revision);
  const updateMetadata = useFavoritesStore((state) => state.updateMetadata);
  const deleteFavorite = useFavoritesStore((state) => state.delete);
  const [search, setSearch] = useState('');
  const [tag, setTag] = useState('');
  const [tags, setTags] = useState<string[]>([]);
  const [page, setPage] = useState(0);
  const pageSize = 20;

  useEffect(() => {
    void loadFavorites('ocr', search, tag, pageSize, page * pageSize);
  }, [loadFavorites, page, revision, search, tag]);

  useEffect(() => {
    void runtime.favorites.listTags('ocr').then(setTags);
  }, [revision, runtime]);

  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="mb-2 text-3xl font-bold text-gray-900">收藏夹</h2>
        <p className="text-gray-600">独立保存的 OCR 识别结果</p>
      </div>
      <div className="flex gap-3">
        <input
          value={search}
          onChange={(event) => {
            setSearch(event.target.value);
            setPage(0);
          }}
          placeholder="搜索收藏的 OCR 文本"
          className="flex-1 rounded-lg border border-gray-300 px-4 py-2 text-sm outline-none focus:ring-2 focus:ring-primary-500"
        />
        <select value={tag} onChange={(event) => { setTag(event.target.value); setPage(0); }} className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm">
          <option value="">全部标签</option>
          {tags.map((value) => <option key={value} value={value}>{value}</option>)}
        </select>
      </div>
      {favorites.length > 0 ? (
        <div className="space-y-3">
          {favorites.map((item) => {
            const snapshot = item.content.snapshot;
            return (
              <div key={item.id} className="rounded-lg border border-gray-200 bg-white p-4 transition-shadow hover:shadow-md">
                <div className="mb-2 flex items-start justify-between">
                  <div className="flex items-center space-x-2">
                    <span className="rounded bg-purple-100 px-2 py-0.5 text-xs font-medium text-purple-700">OCR</span>
                    <span className="text-xs text-gray-500">{snapshot.language ?? '自动'}</span>
                    <span className="text-xs text-gray-400">{snapshot.providerUsed}</span>
                  </div>
                  <div className="flex items-center space-x-2">
                    <span className="text-xs text-gray-400">{formatRelativeTime(new Date(item.createdAt).getTime())}</span>
                    {item.content.snapshot.sourceAssetPath && (
                      <IconActionButton
                        onClick={() => {
                          void runtime.favorites
                            .rerunOcr(item.id)
                            .then((text) => runtime.clipboard.copyText(text));
                        }}
                        title="重新识别并复制"
                        className="flex h-6 w-6 items-center justify-center rounded text-gray-400 transition-colors hover:text-blue-500"
                      >
                        ↻
                      </IconActionButton>
                    )}
                    <IconActionButton onClick={() => void runtime.clipboard.copyText(snapshot.recognizedText)} title="复制文本" className="flex h-6 w-6 items-center justify-center rounded text-gray-400 transition-colors hover:text-blue-500">📋</IconActionButton>
                    <IconActionButton onClick={() => void deleteFavorite(item.id)} title="删除收藏" className="flex h-6 w-6 items-center justify-center rounded text-yellow-500 transition-colors hover:text-red-500">★</IconActionButton>
                  </div>
                </div>
                {item.thumbnailDataUrl && <img src={item.thumbnailDataUrl} alt="OCR source" className="mb-3 max-h-40 w-full rounded-lg bg-gray-50 object-contain" />}
                <div className="whitespace-pre-wrap text-sm text-gray-800">{snapshot.recognizedText}</div>
                <FavoriteMetadataEditor
                  note={item.note ?? undefined}
                  tags={item.tags}
                  onSaveNote={(note) => updateMetadata(item.id, note || null, item.tags)}
                  onSaveTags={(nextTags) => updateMetadata(item.id, item.note, nextTags)}
                />
              </div>
            );
          })}
        </div>
      ) : (
        <div className="py-16 text-center text-gray-400">暂无收藏的 OCR 结果</div>
      )}
      <HistoryPagination page={page} pageSize={pageSize} total={total} onPageChange={setPage} />
    </div>
  );
}
