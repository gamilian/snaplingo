import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { CustomRange } from '../../../components/common/CustomRange';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { useSettingsRuntime } from '../runtimeContext';

export function SaveSettingsPage() {
  const runtime = useSettingsRuntime();
  const screenshot = useSettingsConfigStore((state) => state.screenshot);
  const updateScreenshotSettings = useSettingsConfigStore(
    (state) => state.updateScreenshotSettings,
  );

  if (!screenshot) {
    return <div className="text-sm text-gray-500">设置加载中...</div>;
  }

  const updateScreenshot = (input: Partial<typeof screenshot>) => {
    void updateScreenshotSettings({
      ...screenshot,
      ...input,
    });
  };

  const handleBrowse = async () => {
    const savePath = await runtime.window.selectScreenshotDirectory();
    if (savePath) updateScreenshot({ savePath });
  };

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">保存设置</h2>
        <p className="text-gray-600">配置截图文件的保存选项</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        {/* 保存路径 */}
        <div>
          <label className="block font-medium text-gray-700 mb-2">默认保存路径</label>
          <div className="flex items-center space-x-3">
            <input
              type="text"
              value={screenshot.savePath}
              onChange={(e) => updateScreenshot({ savePath: e.target.value })}
              className="flex-1 px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
              placeholder="~/Pictures/SnapLingo"
            />
            <button
              onClick={handleBrowse}
              className="px-4 py-2 text-sm text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors"
            >
              浏览
            </button>
          </div>
          <p className="text-sm text-gray-500 mt-2">截图文件将保存到此文件夹</p>
        </div>

        {/* 图片格式 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">图片格式</label>
          <div className="flex items-center space-x-4">
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="radio"
                name="format"
                value="png"
                checked={screenshot.format === 'png'}
                onChange={(e) =>
                  updateScreenshot({ format: e.target.value as typeof screenshot.format })
                }
                className="w-4 h-4 text-primary-600 focus:ring-primary-500"
              />
              <span className="text-sm text-gray-700">PNG</span>
            </label>
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="radio"
                name="format"
                value="jpg"
                checked={screenshot.format === 'jpg'}
                onChange={(e) =>
                  updateScreenshot({ format: e.target.value as typeof screenshot.format })
                }
                className="w-4 h-4 text-primary-600 focus:ring-primary-500"
              />
              <span className="text-sm text-gray-700">JPG</span>
            </label>
            <label className="flex items-center space-x-2 cursor-pointer">
              <input
                type="radio"
                name="format"
                value="webp"
                checked={screenshot.format === 'webp'}
                onChange={(e) =>
                  updateScreenshot({ format: e.target.value as typeof screenshot.format })
                }
                className="w-4 h-4 text-primary-600 focus:ring-primary-500"
              />
              <span className="text-sm text-gray-700">WebP</span>
            </label>
          </div>
          <p className="text-sm text-gray-500 mt-2">PNG 无损压缩，JPG/WebP 有损压缩但文件更小</p>
        </div>

        {/* 图片质量 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">
            图片质量：<span className="text-primary-600">{screenshot.quality}%</span>
          </label>
          <CustomRange
            value={screenshot.quality}
            onChange={(quality) => updateScreenshot({ quality })}
            min={50}
            max={100}
            step={1}
          />
          <div className="flex justify-between text-xs text-gray-500 mt-1">
            <span>50%（文件更小）</span>
            <span>100%（质量更高）</span>
          </div>
          <p className="text-sm text-gray-500 mt-2">仅对 JPG 和 WebP 格式生效</p>
        </div>

        {/* 文件命名规则 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">文件命名规则</label>
          <CustomSelect
            options={[
              { value: 'timestamp', label: '时间戳（SnapLingo-20260714-133045）' },
              { value: 'date', label: '日期（SnapLingo-2026-07-14）' },
              { value: 'counter', label: '计数器（Screenshot_001）' },
              { value: 'custom', label: '自定义' },
            ]}
            value={screenshot.namingRule}
            onChange={(namingRule) =>
              updateScreenshot({ namingRule: namingRule as typeof screenshot.namingRule })
            }
          />
          {screenshot.namingRule === 'custom' && (
            <input
              type="text"
              value={screenshot.customFileName}
              onChange={(event) => updateScreenshot({ customFileName: event.target.value })}
              className="mt-3 w-full px-4 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
              placeholder="自定义文件名前缀"
            />
          )}
          <p className="text-sm text-gray-500 mt-2">截图文件的命名方式</p>
        </div>

        {/* 自动复制 */}
        <div className="pt-6 border-t border-gray-100 flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">截图后自动复制到剪贴板</div>
            <div className="text-sm text-gray-500 mt-1">截图完成后自动将图片复制到剪贴板</div>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={screenshot.autoCopy}
            onClick={() => updateScreenshot({ autoCopy: !screenshot.autoCopy })}
            className={`relative w-12 h-6 rounded-full transition-colors ${
              screenshot.autoCopy ? 'bg-primary-600' : 'bg-gray-300'
            }`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform ${
                screenshot.autoCopy ? 'translate-x-6' : ''
              }`}
            />
          </button>
        </div>
      </div>
    </div>
  );
}
