import { useHistoryStore } from '../../../stores/historyStore';
import { formatRelativeTime } from '../../../utils/formatTime';

const typeLabels: Record<string, string> = {
  screenshot: '截图 OCR',
  silent: '静默 OCR',
  file: '选图 OCR',
};

export function FavoritesPage() {
  const history = useHistoryStore((state) => state.ocrHistory);
  const toggleFavorite = useHistoryStore((state) => state.toggleOcrFavorite);

  const favorites = history.filter((item) => item.favorite);

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">收藏夹</h2>
        <p className="text-gray-600">已收藏的 OCR 识别结果</p>
      </div>

      {favorites.length > 0 ? (
        <div className="space-y-3">
          {favorites.map((item) => (
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
                    className="w-6 h-6 flex items-center justify-center text-yellow-500 rounded transition-colors"
                    title="取消收藏"
                  >
                    ★
                  </button>
                </div>
              </div>
              <div className="text-sm text-gray-800 whitespace-pre-wrap">{item.text}</div>
              {item.note && (
                <div className="text-xs text-gray-500 bg-gray-50 rounded p-2 mt-2">
                  📝 {item.note}
                </div>
              )}
            </div>
          ))}
        </div>
      ) : (
        <div className="text-center py-16 text-gray-400">
          暂无收藏的 OCR 结果
          <p className="text-sm mt-2">在历史记录中点击 ☆ 即可收藏</p>
        </div>
      )}
    </div>
  );
}
