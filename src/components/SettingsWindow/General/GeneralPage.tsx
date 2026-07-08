import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { CustomSelect } from '../../common/CustomSelect';

export function GeneralPage() {
  const general = useSettingsConfigStore((state) => state.general);
  const updateGeneralSettings = useSettingsConfigStore(
    (state) => state.updateGeneralSettings,
  );

  if (!general) {
    return <div className="text-sm text-gray-500">设置加载中...</div>;
  }

  const updateGeneral = (input: Partial<typeof general>) => {
    void updateGeneralSettings({
      ...general,
      ...input,
    });
  };

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">通用设置</h2>
        <p className="text-gray-600">应用的基础配置选项</p>
      </div>

      {/* 界面设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">界面</h3>

        {/* 语言 */}
        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">界面语言</div>
            <div className="text-sm text-gray-500 mt-1">选择应用显示的语言</div>
          </div>
          <div className="w-48">
            <CustomSelect
              options={[
                { value: 'zh-CN', label: '中文简体' },
                { value: 'zh-TW', label: '中文繁體' },
                { value: 'en', label: 'English' },
                { value: 'ja', label: '日本語' },
              ]}
              value={general.language}
              onChange={(language) => updateGeneral({ language })}
            />
          </div>
        </div>

        {/* 主题 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">主题</div>
            <div className="text-sm text-gray-500 mt-1">选择应用的外观主题</div>
          </div>
          <div className="w-48">
            <CustomSelect
              options={[
                { value: 'system', label: '跟随系统' },
                { value: 'light', label: '浅色' },
                { value: 'dark', label: '深色' },
              ]}
              value={general.theme}
              onChange={(theme) => updateGeneral({ theme })}
            />
          </div>
        </div>

        {/* 开机自启 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">开机自启</div>
            <div className="text-sm text-gray-500 mt-1">系统启动时自动运行 SnapLingo</div>
          </div>
          <button
            onClick={() => updateGeneral({ startOnBoot: !general.startOnBoot })}
            className={`relative w-12 h-6 rounded-full transition-colors ${
              general.startOnBoot ? 'bg-primary-600' : 'bg-gray-300'
            }`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform ${
                general.startOnBoot ? 'translate-x-6' : 'translate-x-0'
              }`}
            />
          </button>
        </div>
      </div>

      {/* 关于 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-4">
        <h3 className="text-lg font-semibold text-gray-800">关于</h3>

        <div className="flex items-center space-x-4">
          <div className="w-16 h-16 bg-gradient-to-br from-blue-500 to-blue-600 rounded-2xl flex items-center justify-center shadow-lg">
            <svg className="w-10 h-10 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"
              />
            </svg>
          </div>
          <div>
            <div className="text-xl font-bold text-gray-800">SnapLingo</div>
            <div className="text-sm text-gray-600">版本 0.1.0</div>
          </div>
        </div>

        <div className="pt-4 space-y-3">
          <div className="flex items-center justify-between text-sm">
            <span className="text-gray-600">开源协议</span>
            <span className="text-gray-800 font-medium">MIT License</span>
          </div>
          <div className="flex items-center justify-between text-sm">
            <span className="text-gray-600">GitHub</span>
            <a
              href="https://github.com/gamilian/snaplingo"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary-600 hover:text-blue-700 font-medium"
            >
              查看仓库 →
            </a>
          </div>
        </div>

        <div className="pt-4 border-t border-gray-100">
          <button className="px-4 py-2 bg-primary-600 hover:bg-blue-700 text-white text-sm rounded-lg transition-colors">
            检查更新
          </button>
        </div>
      </div>
    </div>
  );
}
