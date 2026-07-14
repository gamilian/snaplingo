import { useState, useEffect } from 'react';
import { useHistoryStore } from '../../../stores/historyStore';
import { formatRelativeTime } from '../../../utils/formatTime';
import IconActionButton from '../../../components/common/IconActionButton';
import { HistoryPagination } from '../HistoryPagination';
import { useFavoritesStore } from '../../../stores/favoritesStore';
import { useSettingsRuntime } from '../runtimeContext';

export function HistoryPage() {
  const runtime = useSettingsRuntime();
  const history = useHistoryStore((state) => state.translationHistory);
  const total = useHistoryStore((state) => state.translationHistoryTotal);
  const queryHistory = useHistoryStore((state) => state.queryTranslationHistory);
  const revision = useHistoryStore((state) => state.revision);
  const deleteItem = useHistoryStore((state) => state.deleteTranslationHistory);
  const addFavorite = useFavoritesStore((state) => state.addTranslation);
  const favoriteRevision = useFavoritesStore((state) => state.revision);
  const [favoritedIds, setFavoritedIds] = useState<Set<string>>(new Set());
  const clearHistory = useHistoryStore((state) => state.clearTranslationHistory);

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
    void runtime.favorites
      .query({ kind: 'translation', limit: 1000, offset: 0 })
      .then((result) => {
        const ids = new Set<string>();
        for (const favorite of result.items) {
          if (favorite.content.contentKind !== 'translation') continue;
          for (const item of history) {
            const snapshot = favorite.content.snapshot;
            if (
              favorite.sourceHistoryId === item.entryId &&
              snapshot.result.provider_id === item.provider &&
              snapshot.result.translated_text === item.targetText
            ) {
              ids.add(item.id);
            }
          }
        }
        setFavoritedIds(ids);
      });
  }, [favoriteRevision, history, runtime]);

  const handleClear = () => {
    if (confirm('确定要清空所有翻译历史记录吗？此操作不可恢复。')) {
      clearHistory();
    }
  };

  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">历史记录</h2>
        <p className="text-gray-600">查看和管理翻译历史</p>
      </div>

      {/* 工具栏 */}
      <div className="flex justify-end">
        {/* 搜索框 */}
        <input
          type="text"
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            setPage(0);
          }}
          placeholder="搜索历史记录..."
          className="flex-1 max-w-xs px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
        />
      </div>

      {/* 历史列表 */}
      {history.length > 0 ? (
        <div className="space-y-3">
          {history.map((item) => (
            <div
              key={item.id}
              className="bg-white border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow"
            >
              <div className="flex items-start justify-between mb-2">
                <div className="flex items-center space-x-2">
                  <span className="px-2 py-0.5 text-xs bg-blue-100 text-blue-700 rounded font-medium">
                    翻译
                  </span>
                  <span className="text-xs text-gray-500">
                    {item.sourceLang} → {item.targetLang}
                  </span>
                  <span className="text-xs text-gray-400">{item.provider}</span>
                </div>
                <div className="flex items-center space-x-2">
                  <span className="text-xs text-gray-400">{formatRelativeTime(item.timestamp)}</span>
                  <IconActionButton
                    onClick={() => {
                      if (favoritedIds.has(item.id)) return;
                      void addFavorite({
                        sourceHistoryId: item.entryId,
                        sourceText: item.sourceText,
                        sourceLang: item.sourceLang,
                        targetLang: item.targetLang,
                        providerId: item.provider,
                        translatedText: item.targetText,
                      }).then(() => {
                        setFavoritedIds((current) => new Set(current).add(item.id));
                      });
                    }}
                    title={favoritedIds.has(item.id) ? '已收藏' : '收藏'}
                    disabled={favoritedIds.has(item.id)}
                    className={`w-6 h-6 flex items-center justify-center rounded transition-colors ${
                      favoritedIds.has(item.id) ? 'text-yellow-500' : 'text-gray-300 hover:text-gray-400'
                    }`}
                  >
                    {favoritedIds.has(item.id) ? '★' : '☆'}
                  </IconActionButton>
                  <IconActionButton
                    onClick={() => deleteItem(item.id)}
                    title="删除"
                    className="w-6 h-6 flex items-center justify-center rounded text-gray-300 transition-colors hover:text-red-500"
                  >
                    ✕
                  </IconActionButton>
                </div>
              </div>
              <div className="space-y-2">
                <div className="text-sm text-gray-800">{item.sourceText}</div>
                <div className="text-sm text-gray-600 pl-3 border-l-2 border-blue-200">
                  {item.targetText}
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-center py-16 text-gray-400">
          {search ? '没有匹配的历史记录' : '暂无翻译历史记录'}
        </div>
      )}

      {/* 底部操作 */}
      {history.length > 0 && (
        <div className="pt-4 border-t border-gray-200">
          <button
            onClick={handleClear}
            className="px-4 py-2 text-sm text-red-600 hover:bg-red-50 rounded-lg transition-colors"
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
