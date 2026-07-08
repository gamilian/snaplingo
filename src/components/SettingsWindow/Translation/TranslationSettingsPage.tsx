import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { CustomSelect } from '../../common/CustomSelect';

export function TranslationSettingsPage() {
  const translation = useSettingsConfigStore((state) => state.translation);
  const updateTranslationSettings = useSettingsConfigStore(
    (state) => state.updateTranslationSettings,
  );

  if (!translation) {
    return <div className="text-sm text-gray-500">设置加载中...</div>;
  }

  const updateTranslation = (input: Partial<typeof translation>) => {
    void updateTranslationSettings({
      ...translation,
      ...input,
    });
  };

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">翻译设置</h2>
        <p className="text-gray-600">配置翻译行为和默认选项</p>
      </div>

      {/* 语言设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">默认语言</h3>

        {/* 源语言 */}
        <div>
          <label className="block font-medium text-gray-700 mb-2">源语言</label>
          <CustomSelect
            options={[
              { value: 'auto', label: '自动检测' },
              { value: 'zh-CN', label: '中文简体' },
              { value: 'zh-TW', label: '中文繁體' },
              { value: 'en', label: 'English' },
              { value: 'ja', label: '日本語' },
              { value: 'ko', label: '한국어' },
              { value: 'fr', label: 'Français' },
              { value: 'de', label: 'Deutsch' },
              { value: 'es', label: 'Español' },
            ]}
            value={translation.defaultSourceLang}
            onChange={(defaultSourceLang) =>
              updateTranslation({ defaultSourceLang })
            }
          />
          <p className="text-sm text-gray-500 mt-2">翻译窗口默认的源语言</p>
        </div>

        {/* 目标语言 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">目标语言</label>
          <CustomSelect
            options={[
              { value: 'zh-CN', label: '中文简体' },
              { value: 'zh-TW', label: '中文繁體' },
              { value: 'en', label: 'English' },
              { value: 'ja', label: '日本語' },
              { value: 'ko', label: '한국어' },
              { value: 'fr', label: 'Français' },
              { value: 'de', label: 'Deutsch' },
              { value: 'es', label: 'Español' },
            ]}
            value={translation.defaultTargetLang}
            onChange={(defaultTargetLang) =>
              updateTranslation({ defaultTargetLang })
            }
          />
          <p className="text-sm text-gray-500 mt-2">翻译窗口默认的目标语言</p>
        </div>
      </div>

      {/* 行为设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">翻译行为</h3>

        {/* 自动翻译 */}
        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">自动翻译</div>
            <div className="text-sm text-gray-500 mt-1">输入文本后自动开始翻译，无需点击按钮</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 自动复制 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">翻译后自动复制</div>
            <div className="text-sm text-gray-500 mt-1">翻译完成后自动复制结果到剪贴板</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-gray-300 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform" />
          </button>
        </div>

        {/* 保留换行 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">保留原文换行</div>
            <div className="text-sm text-gray-500 mt-1">翻译时保留原文的换行和段落格式</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 增量翻译 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">增量翻译</div>
            <div className="text-sm text-gray-500 mt-1">输入时实时翻译，而不是等待输入完成</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-gray-300 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform" />
          </button>
        </div>
      </div>

      {/* 窗口行为 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">窗口行为</h3>

        {/* 窗口置顶 */}
        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">翻译窗口置顶</div>
            <div className="text-sm text-gray-500 mt-1">翻译窗口始终显示在最前面</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 失焦隐藏 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">失去焦点时隐藏</div>
            <div className="text-sm text-gray-500 mt-1">点击窗口外部区域时自动隐藏翻译窗口</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-gray-300 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform" />
          </button>
        </div>
      </div>
    </div>
  );
}
