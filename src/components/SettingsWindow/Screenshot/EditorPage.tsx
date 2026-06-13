export function EditorPage() {
  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">编辑器</h2>
        <p className="text-gray-600">配置截图编辑工具的默认值</p>
      </div>

      {/* 默认工具 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">默认工具</h3>

        {/* 默认颜色 */}
        <div>
          <label className="block font-medium text-gray-700 mb-2">默认标注颜色</label>
          <div className="flex items-center space-x-3">
            {['#EF4444', '#F59E0B', '#10B981', '#3B82F6', '#8B5CF6', '#1F2937'].map((color) => (
              <button
                key={color}
                className="w-8 h-8 rounded-full border-2 border-white shadow-md hover:scale-110 transition-transform"
                style={{ backgroundColor: color }}
              />
            ))}
          </div>
          <p className="text-sm text-gray-500 mt-2">新建标注时默认使用的颜色</p>
        </div>

        {/* 默认线条粗细 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">
            默认线条粗细：<span className="text-blue-600">3px</span>
          </label>
          <input
            type="range"
            min="1"
            max="10"
            defaultValue="3"
            className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-600"
          />
        </div>

        {/* 默认字体大小 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">默认字体大小</label>
          <select className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option value="12">12px（小）</option>
            <option value="16">16px（中）</option>
            <option value="20">20px（大）</option>
            <option value="24">24px（特大）</option>
          </select>
          <p className="text-sm text-gray-500 mt-2">文字标注的默认字体大小</p>
        </div>
      </div>

      {/* 行为设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">编辑行为</h3>

        {/* 自动选中工具 */}
        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">记住上次使用的工具</div>
            <div className="text-sm text-gray-500 mt-1">下次截图时自动选中上次使用的标注工具</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-blue-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 显示尺寸 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">显示选区尺寸</div>
            <div className="text-sm text-gray-500 mt-1">截图时显示选区的像素尺寸</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-blue-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 放大镜 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">显示放大镜</div>
            <div className="text-sm text-gray-500 mt-1">截图时显示像素级放大镜，便于精确选取</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-gray-300 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform" />
          </button>
        </div>
      </div>

      {/* 贴图设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">贴图设置</h3>

        {/* 默认透明度 */}
        <div>
          <label className="block font-medium text-gray-700 mb-2">
            贴图默认透明度：<span className="text-blue-600">100%</span>
          </label>
          <input
            type="range"
            min="20"
            max="100"
            defaultValue="100"
            className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-600"
          />
        </div>

        {/* 阴影 */}
        <div className="pt-6 border-t border-gray-100 flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">贴图显示阴影</div>
            <div className="text-sm text-gray-500 mt-1">为贴图窗口添加投影效果</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-blue-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>
      </div>
    </div>
  );
}
