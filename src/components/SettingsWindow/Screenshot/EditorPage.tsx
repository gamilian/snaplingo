import { useState } from 'react';
import { CustomSelect } from '../../common/CustomSelect';
import { CustomRange } from '../../common/CustomRange';

export function EditorPage() {
  const [fontSize, setFontSize] = useState('12');
  const [lineWidth, setLineWidth] = useState(3);
  const [pinOpacity, setPinOpacity] = useState(100);
  const [capturedScreenshot, setCapturedScreenshot] = useState<string | null>(null);

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">编辑器</h2>
        <p className="text-gray-600">配置截图编辑工具的默认值</p>
      </div>

      {capturedScreenshot && (
        <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold text-gray-800">最近截图</h3>
            <button
              onClick={() => setCapturedScreenshot(null)}
              className="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
            >
              清除
            </button>
          </div>
          <div className="overflow-auto rounded-lg border border-gray-200 bg-gray-100 max-h-[520px]">
            <img
              src={capturedScreenshot}
              alt="最近截图"
              className="block max-w-full h-auto mx-auto"
            />
          </div>
        </div>
      )}

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
            默认线条粗细：<span className="text-primary-600">{lineWidth}px</span>
          </label>
          <CustomRange
            value={lineWidth}
            onChange={setLineWidth}
            min={1}
            max={10}
            step={1}
          />
        </div>

        {/* 默认字体大小 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">默认字体大小</label>
          <CustomSelect
            options={[
              { value: '12', label: '12px（小）' },
              { value: '16', label: '16px（中）' },
              { value: '20', label: '20px（大）' },
              { value: '24', label: '24px（特大）' },
            ]}
            value={fontSize}
            onChange={setFontSize}
          />
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
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 显示尺寸 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">显示选区尺寸</div>
            <div className="text-sm text-gray-500 mt-1">截图时显示选区的像素尺寸</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
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
            贴图默认透明度：<span className="text-primary-600">{pinOpacity}%</span>
          </label>
          <CustomRange
            value={pinOpacity}
            onChange={setPinOpacity}
            min={20}
            max={100}
            step={1}
          />
        </div>

        {/* 阴影 */}
        <div className="pt-6 border-t border-gray-100 flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">贴图显示阴影</div>
            <div className="text-sm text-gray-500 mt-1">为贴图窗口添加投影效果</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>
      </div>
    </div>
  );
}
