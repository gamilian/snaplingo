import { useHistoryStore } from '../../../stores/historyStore';
import { formatRelativeTime } from '../../../utils/formatTime';
import IconActionButton from '../../../components/common/IconActionButton';

const typeLabels: Record<string, string> = {
  selection: '划词',
  screenshot: '截图',
  input: '输入',
};

export function FavoritesPage() {
  const history = useHistoryStore((state) => state.translationHistory);
  const toggleFavorite = useHistoryStore((state) => state.toggleTranslationFavorite);

  const favorites = history.filter((item) => item.favorite);

  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">收藏夹</h2>
        <p className="text-gray-600">已收藏的翻译记录</p>
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
                    title="取消收藏"
                    className="w-6 h-6 flex items-center justify-center rounded text-yellow-500 transition-colors"
                  >
                    ★
                  </IconActionButton>
                </div>
              </div>
              <div className="space-y-2">
                <div className="text-sm text-gray-800">{item.sourceText}</div>
                <div className="text-sm text-gray-600 pl-3 border-l-2 border-blue-200">
                  {item.targetText}
                </div>
                {item.note && (
                  <div className="text-xs text-gray-500 bg-gray-50 rounded p-2 mt-2">
                    📝 {item.note}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-center py-16 text-gray-400">
          暂无收藏的翻译记录
          <p className="text-sm mt-2">在历史记录中点击 ☆ 即可收藏</p>
        </div>
      )}
    </div>
  );
}
