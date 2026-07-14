import { useEffect, useState } from 'react';
import { useHistoryStore } from '../../../stores/historyStore';
import { useSettingsRuntime } from '../runtimeContext';
import { formatRelativeTime } from '../../../utils/formatTime';
import IconActionButton from '../../../components/common/IconActionButton';
import { HistoryPagination } from '../HistoryPagination';
import { useFavoritesStore } from '../../../stores/favoritesStore';

export function HistoryPage() {
  const runtime = useSettingsRuntime();
  const history = useHistoryStore((state) => state.ocrHistory);
  const total = useHistoryStore((state) => state.ocrHistoryTotal);
  const queryHistory = useHistoryStore((state) => state.queryOcrHistory);
  const revision = useHistoryStore((state) => state.revision);
  const deleteItem = useHistoryStore((state) => state.deleteOcrHistory);
  const addFavorite = useFavoritesStore((state) => state.addOcr);
  const favoriteRevision = useFavoritesStore((state) => state.revision);
  const [favoritedIds, setFavoritedIds] = useState<Set<string>>(new Set());
  const clearHistory = useHistoryStore((state) => state.clearOcrHistory);

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
    void runtime.favorites.query({ kind: 'ocr', limit: 1000, offset: 0 }).then((result) => {
      setFavoritedIds(
        new Set(
          result.items
            .filter((item) => item.content.contentKind === 'ocr')
            .map((item) => item.sourceHistoryId)
            .filter((id): id is number => id !== null)
            .map(String),
        ),
      );
    });
  }, [favoriteRevision, runtime]);

  const handleCopy = (text: string) => {
    void runtime.clipboard.copyText(text);
  };

  const handleRerun = async (id: string) => {
    const text = await runtime.history.rerunOcr(Number(id));
    await runtime.clipboard.copyText(text);
  };

  const handleClear = () => {
    if (confirm('确定要清空所有 OCR 历史记录吗？此操作不可恢复。')) {
      clearHistory();
    }
  };

  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">历史记录</h2>
        <p className="text-gray-600">查看和管理 OCR 识别历史</p>
      </div>

      {/* 工具栏 */}
      <div className="flex justify-end">
        <input
          type="text"
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            setPage(0);
          }}
          placeholder="搜索识别文本..."
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
                  <span className="px-2 py-0.5 text-xs bg-purple-100 text-purple-700 rounded font-medium">
                    OCR
                  </span>
                  <span className="text-xs text-gray-500">{item.language}</span>
                </div>
                <div className="flex items-center space-x-2">
                  <span className="text-xs text-gray-400">{formatRelativeTime(item.timestamp)}</span>
                  <IconActionButton
                    onClick={() => void handleRerun(item.id)}
                    title="重新识别并复制"
                    className="w-6 h-6 flex items-center justify-center rounded text-gray-400 transition-colors hover:text-blue-500"
                  >
                    ↻
                  </IconActionButton>
                  <IconActionButton
                    onClick={() => handleCopy(item.text)}
                    title="复制文本"
                    className="w-6 h-6 flex items-center justify-center rounded text-gray-400 transition-colors hover:text-blue-500"
                  >
                    📋
                  </IconActionButton>
                  <IconActionButton
                    onClick={() => {
                      if (favoritedIds.has(item.id)) return;
                      void addFavorite({
                        sourceHistoryId: Number(item.id),
                        recognizedText: item.text,
                        language: item.language === 'Unknown' ? null : item.language,
                        providerUsed: item.providerUsed,
                        confidence: item.confidence,
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
              {item.imageThumbnail && (
                <img
                  src={item.imageThumbnail}
                  alt="OCR source"
                  className="mb-3 max-h-40 w-full rounded-lg bg-gray-50 object-contain"
                />
              )}
              <div className="text-sm text-gray-800 whitespace-pre-wrap">{item.text}</div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-center py-16 text-gray-400">
          {search ? '没有匹配的历史记录' : '暂无 OCR 历史记录'}
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
