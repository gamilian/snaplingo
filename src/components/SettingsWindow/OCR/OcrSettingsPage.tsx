export function OcrSettingsPage() {
  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">OCR 设置</h2>
        <p className="text-gray-600">配置 OCR 识别行为和选项</p>
      </div>

      {/* 识别设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">识别设置</h3>

        {/* 识别语言 */}
        <div>
          <label className="block font-medium text-gray-700 mb-2">识别语言</label>
          <select className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option value="auto">自动检测</option>
            <option value="chi_sim">中文简体</option>
            <option value="chi_tra">中文繁体</option>
            <option value="eng">English</option>
            <option value="jpn">日本語</option>
            <option value="kor">한국어</option>
            <option value="multi">多语言混合</option>
          </select>
          <p className="text-sm text-gray-500 mt-2">选择 OCR 识别的主要语言</p>
        </div>

        {/* 识别精度 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">识别精度</label>
          <select className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option value="fast">快速（速度优先）</option>
            <option value="balanced">平衡（默认）</option>
            <option value="accurate">精确（质量优先）</option>
          </select>
          <p className="text-sm text-gray-500 mt-2">速度越快精度越低，根据需求选择</p>
        </div>

        {/* 图像预处理 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">图像预处理</label>
          <div className="space-y-3">
            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                defaultChecked
                className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
              />
              <span className="text-sm text-gray-700">自动旋转校正</span>
            </label>
            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                defaultChecked
                className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
              />
              <span className="text-sm text-gray-700">对比度增强</span>
            </label>
            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
              />
              <span className="text-sm text-gray-700">去除噪点</span>
            </label>
          </div>
          <p className="text-sm text-gray-500 mt-2">预处理可以提高识别准确率，但会略微增加处理时间</p>
        </div>
      </div>

      {/* 行为设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">OCR 行为</h3>

        {/* 自动复制 */}
        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">识别后自动复制</div>
            <div className="text-sm text-gray-500 mt-1">OCR 完成后自动复制结果到剪贴板</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-blue-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 保留格式 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">保留文本格式</div>
            <div className="text-sm text-gray-500 mt-1">尝试保留原文的换行和段落结构</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-blue-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 去除空格 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">中文自动去除空格</div>
            <div className="text-sm text-gray-500 mt-1">识别中文时自动去除多余空格</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-blue-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 显示置信度 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">显示识别置信度</div>
            <div className="text-sm text-gray-500 mt-1">在结果中显示每行文字的识别可信度</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-gray-300 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform" />
          </button>
        </div>
      </div>

      {/* 高级选项 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">高级选项</h3>

        {/* 线程数 */}
        <div>
          <label className="block font-medium text-gray-700 mb-2">处理线程数</label>
          <select className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option value="auto">自动（推荐）</option>
            <option value="1">1 线程</option>
            <option value="2">2 线程</option>
            <option value="4">4 线程</option>
            <option value="8">8 线程</option>
          </select>
          <p className="text-sm text-gray-500 mt-2">更多线程可能提升速度，但会占用更多 CPU</p>
        </div>

        {/* DPI */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">图像 DPI 设置</label>
          <input
            type="number"
            defaultValue={300}
            min={72}
            max={600}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <p className="text-sm text-gray-500 mt-2">默认 300，较高的 DPI 可以提高小字识别率</p>
        </div>
      </div>
    </div>
  );
}
