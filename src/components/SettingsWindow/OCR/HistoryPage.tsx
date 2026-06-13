import { useState } from 'react';
import { useHistoryStore } from '../../../stores/historyStore';
import { formatRelativeTime } from '../../../utils/formatTime';

type FilterType = 'all' | 'screenshot' | 'silent' | 'file';

const typeLabels: Record<string, string> = {
  screenshot: '截图 OCR',
  silent: '静默 OCR',
  file: '选图 OCR',
};

export function HistoryPage() {
  const history = useHistoryStore((state) => state.ocrHistory);
  const deleteItem = useHistoryStore((state) => state.deleteOcrHistory);
  const toggleFavorite = useHistoryStore((state) => state.toggleOcrFavorite);
  const clearHistory = useHistoryStore((state) => state.clearOcrHistory);

  const [filter, setFilter] = useState<FilterType>('all');
  const [search, setSearch] = useState('');

  const filteredHistory = history.filter((item) => {
    const matchesFilter = filter === 'all' || item.type === filter;
    const matchesSearch =
      search === '' || item.text.toLowerCase().includes(search.toLowerCase());
    return matchesFilter && matchesSearch;
  });

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
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
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center space-x-1 bg-gray-100 rounded-lg p-1">
          {(['all', 'screenshot', 'silent', 'file'] as FilterType[]).map((type) => (
            <button
              key={type}
              onClick={() => setFilter(type)}
              className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
                filter === type
                  ? 'bg-white text-blue-600 shadow-sm font-medium'
                  : 'text-gray-600 hover:text-gray-800'
              }`}
            >
              {type === 'all' ? '全部' : typeLabels[type]}
            </button>
          ))}
        </div>

        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="搜索识别文本..."
          className="flex-1 max-w-xs px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* 历史列表 */}
      {filteredHistory.length > 0 ? (
        <div className="space-y-3">
          {filteredHistory.map((item) => (
            <div
              key={item.id}
              className="bg-white border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow"
            >
              <div className="flex items-start justify-between mb-2">
                <div className="flex items-center space-x-2">
                  <span className="px-2 py-0.5 text-xs bg-purple-100 text-purple-700 rounded font-medium">
                    {typeLabels[item.type]}
                  </span>
                  <span className="text-xs text-gray-500">{item.language}</span>
                </div>
                <div className="flex items-center space-x-2">
                  <span className="text-xs text-gray-400">{formatRelativeTime(item.timestamp)}</span>
                  <button
                    onClick={() => handleCopy(item.text)}
                    className="w-6 h-6 flex items-center justify-center text-gray-400 hover:text-blue-500 rounded transition-colors"
                    title="复制文本"
                  >
                    📋
                  </button>
                  <button
                    onClick={() => toggleFavorite(item.id)}
                    className={`w-6 h-6 flex items-center justify-center rounded transition-colors ${
                      item.favorite ? 'text-yellow-500' : 'text-gray-300 hover:text-gray-400'
                    }`}
                    title={item.favorite ? '取消收藏' : '收藏'}
                  >
                    {item.favorite ? '★' : '☆'}
                  </button>
                  <button
                    onClick={() => deleteItem(item.id)}
                    className="w-6 h-6 flex items-center justify-center text-gray-300 hover:text-red-500 rounded transition-colors"
                    title="删除"
                  >
                    ✕
                  </button>
                </div>
              </div>
              <div className="text-sm text-gray-800 whitespace-pre-wrap">{item.text}</div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-center py-16 text-gray-400">
          {search || filter !== 'all' ? '没有匹配的历史记录' : '暂无 OCR 历史记录'}
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
    </div>
  );
}
