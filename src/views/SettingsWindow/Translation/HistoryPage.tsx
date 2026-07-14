import { useEffect, useState } from 'react';
import { translationFavoriteKey } from '../../../application/favorites/identity';
import IconActionButton from '../../../components/common/IconActionButton';
import { useFavoritesStore } from '../../../stores/favoritesStore';
import { useHistoryStore } from '../../../stores/historyStore';
import { formatRelativeTime } from '../../../utils/formatTime';
import { HistoryPagination } from '../HistoryPagination';

export function HistoryPage() {
  const history = useHistoryStore((state) => state.translationHistory);
  const total = useHistoryStore((state) => state.translationHistoryTotal);
  const queryHistory = useHistoryStore((state) => state.queryTranslationHistory);
  const revision = useHistoryStore((state) => state.revision);
  const deleteItem = useHistoryStore((state) => state.deleteTranslationHistory);
  const clearHistory = useHistoryStore((state) => state.clearTranslationHistory);
  const addFavorite = useFavoritesStore((state) => state.addTranslation);
  const favoriteKeys = useFavoritesStore((state) => state.keys);
  const hydrateFavoriteKeys = useFavoritesStore((state) => state.hydrateKeys);
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(0);
  const pageSize = 20;

  useEffect(() => {
    void queryHistory({
      search,
      limit: pageSize,
      offset: page * pageSize,
    });
  }, [page, queryHistory, revision, search]);

  useEffect(() => {
    void hydrateFavoriteKeys('translation');
  }, [hydrateFavoriteKeys]);

  const handleClear = () => {
    if (confirm('确定要清空所有翻译历史记录吗？此操作不可恢复。')) {
      void clearHistory();
    }
  };

  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="mb-2 text-3xl font-bold text-gray-900">历史记录</h2>
        <p className="text-gray-600">查看和管理翻译历史</p>
      </div>

      <div className="flex justify-end">
        <input
          type="text"
          value={search}
          onChange={(event) => {
            setSearch(event.target.value);
            setPage(0);
          }}
          placeholder="搜索历史记录..."
          className="max-w-xs flex-1 rounded-lg border border-gray-300 px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
        />
      </div>

      {history.length > 0 ? (
        <div className="space-y-3">
          {history.map((item) => (
            <div
              key={item.id}
              className="rounded-lg border border-gray-200 bg-white p-4 transition-shadow hover:shadow-md"
            >
              <div className="mb-3 flex items-start justify-between gap-4">
                <div className="flex items-center space-x-2">
                  <span className="rounded bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700">
                    翻译
                  </span>
                  <span className="text-xs text-gray-500">
                    {item.sourceLang} → {item.targetLang}
                  </span>
                  <span className="text-xs text-gray-400">
                    {item.results.length} 个服务
                  </span>
                </div>
                <div className="flex items-center space-x-2">
                  <span className="text-xs text-gray-400">
                    {formatRelativeTime(item.timestamp)}
                  </span>
                  <IconActionButton
                    onClick={() => void deleteItem(item.id)}
                    title="删除整次翻译"
                    className="flex h-6 w-6 items-center justify-center rounded text-gray-300 transition-colors hover:text-red-500"
                  >
                    ✕
                  </IconActionButton>
                </div>
              </div>

              <div className="mb-3 whitespace-pre-wrap text-sm text-gray-800">
                {item.sourceText}
              </div>
              <div className="space-y-2">
                {item.results.map((result) => {
                  const favoriteInput = {
                    sourceHistoryId: item.id,
                    sourceText: item.sourceText,
                    sourceLang: item.sourceLang,
                    targetLang: item.targetLang,
                    providerId: result.providerId,
                    translatedText: result.translatedText,
                    detectedLanguage: result.detectedLanguage,
                    confidence: result.confidence,
                  };
                  const isFavorite = favoriteKeys.has(
                    translationFavoriteKey(favoriteInput),
                  );

                  return (
                    <div
                      key={`${item.id}-${result.providerId}`}
                      className="flex items-start gap-3 border-l-2 border-blue-200 pl-3"
                    >
                      <div className="min-w-0 flex-1">
                        <div className="mb-1 text-xs text-gray-400">
                          {result.providerId}
                        </div>
                        <div className="whitespace-pre-wrap text-sm text-gray-600">
                          {result.translatedText}
                        </div>
                      </div>
                      <IconActionButton
                        onClick={() => {
                          if (!isFavorite) void addFavorite(favoriteInput);
                        }}
                        title={isFavorite ? '已收藏' : '收藏此译文'}
                        disabled={isFavorite}
                        className={`flex h-6 w-6 shrink-0 items-center justify-center rounded transition-colors ${
                          isFavorite
                            ? 'text-yellow-500'
                            : 'text-gray-300 hover:text-gray-400'
                        }`}
                      >
                        {isFavorite ? '★' : '☆'}
                      </IconActionButton>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="py-16 text-center text-gray-400">
          {search ? '没有匹配的历史记录' : '暂无翻译历史记录'}
        </div>
      )}

      {history.length > 0 && (
        <div className="border-t border-gray-200 pt-4">
          <button
            onClick={handleClear}
            className="rounded-lg px-4 py-2 text-sm text-red-600 transition-colors hover:bg-red-50"
          >
            清空所有历史记录
          </button>
        </div>
      )}
      <HistoryPagination
        page={page}
        pageSize={pageSize}
        total={total}
        onPageChange={setPage}
      />
    </div>
  );
}
