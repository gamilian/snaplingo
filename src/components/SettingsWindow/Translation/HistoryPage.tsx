import { useState, useEffect } from 'react';
import { useHistoryStore } from '../../../stores/historyStore';
import { formatRelativeTime } from '../../../utils/formatTime';
import IconActionButton from '../../common/IconActionButton';

type FilterType = 'all' | 'selection' | 'screenshot' | 'input';

const typeLabels: Record<string, string> = {
  selection: '划词',
  screenshot: '截图',
  input: '输入',
};

export function HistoryPage() {
  const history = useHistoryStore((state) => state.translationHistory);
  const loadHistory = useHistoryStore((state) => state.loadTranslationHistory);
  const deleteItem = useHistoryStore((state) => state.deleteTranslationHistory);
  const toggleFavorite = useHistoryStore((state) => state.toggleTranslationFavorite);
  const clearHistory = useHistoryStore((state) => state.clearTranslationHistory);

  const [filter, setFilter] = useState<FilterType>('all');
  const [search, setSearch] = useState('');

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  const filteredHistory = history.filter((item) => {
    const matchesFilter = filter === 'all' || item.type === filter;
    const matchesSearch =
      search === '' ||
      item.sourceText.toLowerCase().includes(search.toLowerCase()) ||
      item.targetText.toLowerCase().includes(search.toLowerCase());
    return matchesFilter && matchesSearch;
  });

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
      <div className="flex items-center justify-between gap-4">
        {/* 过滤器 */}
        <div className="flex items-center space-x-1 bg-gray-100 rounded-lg p-1">
          {(['all', 'selection', 'screenshot', 'input'] as FilterType[]).map((type) => (
            <button
              key={type}
              onClick={() => setFilter(type)}
              className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
                filter === type
                  ? 'bg-white text-primary-600 shadow-sm font-medium'
                  : 'text-gray-600 hover:text-gray-800'
              }`}
            >
              {type === 'all' ? '全部' : typeLabels[type]}
            </button>
          ))}
        </div>

        {/* 搜索框 */}
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="搜索历史记录..."
          className="flex-1 max-w-xs px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
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
                  <span className="px-2 py-0.5 text-xs bg-blue-100 text-blue-700 rounded font-medium">
                    {typeLabels[item.type]}
                  </span>
                  <span className="text-xs text-gray-500">
                    {item.sourceLang} → {item.targetLang}
                  </span>
                  <span className="text-xs text-gray-400">{item.provider}</span>
                </div>
                <div className="flex items-center space-x-2">
                  <span className="text-xs text-gray-400">{formatRelativeTime(item.timestamp)}</span>
                  <IconActionButton
                    onClick={() => toggleFavorite(item.id)}
                    title={item.favorite ? '取消收藏' : '收藏'}
                    className={`w-6 h-6 flex items-center justify-center rounded transition-colors ${
                      item.favorite ? 'text-yellow-500' : 'text-gray-300 hover:text-gray-400'
                    }`}
                  >
                    {item.favorite ? '★' : '☆'}
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
          {search || filter !== 'all' ? '没有匹配的历史记录' : '暂无翻译历史记录'}
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
