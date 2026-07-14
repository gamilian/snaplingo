import { useEffect, useState } from 'react';
import { useScreenshotFavoritesStore } from '../../../stores/screenshotFavoritesStore';
import { FavoriteMetadataEditor } from '../FavoriteMetadataEditor';
import { HistoryPagination } from '../HistoryPagination';

const PAGE_SIZE = 12;

export function FavoritesPage() {
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(0);
  const { items, total, loading, error, query, updateMetadata } =
    useScreenshotFavoritesStore();
  const revision = useScreenshotFavoritesStore((state) => state.revision);
  const deleteFavorite = useScreenshotFavoritesStore((state) => state.delete);
  const copyFavorite = useScreenshotFavoritesStore((state) => state.copy);
  const revealFavorite = useScreenshotFavoritesStore((state) => state.reveal);

  useEffect(() => {
    void query(search, PAGE_SIZE, page * PAGE_SIZE);
  }, [page, query, revision, search]);

  return (
    <div className="max-w-5xl space-y-6">
      <div>
        <h2 className="mb-2 text-3xl font-bold text-gray-900">收藏夹</h2>
        <p className="text-gray-600">已收藏的截图</p>
      </div>

      <input
        value={search}
        onChange={(event) => {
          setSearch(event.target.value);
          setPage(0);
        }}
        placeholder="搜索笔记或标签"
        className="w-full rounded-xl border border-gray-200 bg-white px-4 py-2.5 text-sm outline-none focus:border-primary-400"
      />

      {error && <p className="rounded-lg bg-red-50 p-3 text-sm text-red-600">{error}</p>}
      {loading && items.length === 0 ? (
        <div className="py-16 text-center text-gray-400">正在加载收藏…</div>
      ) : items.length === 0 ? (
        <div className="py-16 text-center text-gray-400">
          暂无收藏的截图
          <p className="mt-2 text-sm">截图后点击收藏按钮即可添加</p>
        </div>
      ) : (
        <div className="grid gap-5 sm:grid-cols-2 xl:grid-cols-3">
          {items.map((item) => (
            <article
              key={item.id}
              className="overflow-hidden rounded-2xl border border-gray-200 bg-white shadow-sm"
            >
              <button
                type="button"
                onClick={() => void revealFavorite(item.id)}
                className="block aspect-video w-full overflow-hidden bg-gray-100"
                title="在文件管理器中显示"
              >
                <img
                  src={item.thumbnailDataUrl}
                  alt={`收藏截图 ${item.id}`}
                  className="h-full w-full object-contain"
                />
              </button>
              <div className="p-4">
                <div className="flex items-center justify-between text-xs text-gray-500">
                  <span>{item.width} × {item.height}</span>
                  <span>{new Date(item.createdAt).toLocaleString()}</span>
                </div>
                <FavoriteMetadataEditor
                  note={item.note ?? undefined}
                  tags={item.tags}
                  onSaveNote={(note) =>
                    updateMetadata(item.id, note || null, item.tags)
                  }
                  onSaveTags={(tags) =>
                    updateMetadata(item.id, item.note, tags)
                  }
                />
                <div className="mt-3 flex justify-end gap-2">
                  <button
                    type="button"
                    onClick={() => void copyFavorite(item.id)}
                    className="rounded-lg border border-gray-200 px-3 py-1.5 text-sm hover:bg-gray-50"
                  >
                    复制
                  </button>
                  <button
                    type="button"
                    onClick={() => void revealFavorite(item.id)}
                    className="rounded-lg border border-gray-200 px-3 py-1.5 text-sm hover:bg-gray-50"
                  >
                    显示文件
                  </button>
                  <button
                    type="button"
                    onClick={() => void deleteFavorite(item.id)}
                    className="rounded-lg px-3 py-1.5 text-sm text-red-600 hover:bg-red-50"
                  >
                    删除
                  </button>
                </div>
              </div>
            </article>
          ))}
        </div>
      )}

      <HistoryPagination
        page={page}
        pageSize={PAGE_SIZE}
        total={total}
        onPageChange={setPage}
      />
    </div>
  );
}
