export function FavoritesPage() {
  return (
    <div className="max-w-4xl space-y-6">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">收藏夹</h2>
        <p className="text-gray-600">已收藏的截图</p>
      </div>

      <div className="text-center py-16 text-gray-400">
        暂无收藏的截图
        <p className="text-sm mt-2">截图后可将其添加到收藏夹</p>
      </div>
    </div>
  );
}
