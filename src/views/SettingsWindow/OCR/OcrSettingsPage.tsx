import type { ReactNode } from 'react';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';

export function OcrSettingsPage() {
  const ocr = useSettingsConfigStore((state) => state.ocr);
  const updateOcrSettings = useSettingsConfigStore(
    (state) => state.updateOcrSettings,
  );

  if (!ocr) {
    return <div className="text-sm text-gray-500">设置加载中...</div>;
  }

  const updateOcr = (input: Partial<typeof ocr>) => {
    void updateOcrSettings(input);
  };

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">OCR 设置</h2>
        <p className="text-gray-600">配置 OCR 识别行为和结果展示</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">识别设置</h3>
        <div>
          <label className="block font-medium text-gray-700 mb-2">识别语言</label>
          <CustomSelect
            options={[
              { value: 'auto', label: '自动检测' },
              { value: 'zh-CN', label: '中文简体' },
              { value: 'zh-TW', label: '中文繁體' },
              { value: 'en', label: 'English' },
              { value: 'ja', label: '日本語' },
              { value: 'ko', label: '한국어' },
            ]}
            value={ocr.recognitionLanguage}
            onChange={(recognitionLanguage) =>
              updateOcr({ recognitionLanguage })
            }
          />
          <p className="text-sm text-gray-500 mt-2">用于截图 OCR 和上传图片 OCR</p>
        </div>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">OCR 行为</h3>
        <SettingRow
          label="识别后自动复制"
          description="OCR 完成后自动复制结果到剪贴板"
        >
          <SettingsToggle
            checked={ocr.autoCopy}
            onChange={(autoCopy) => updateOcr({ autoCopy })}
          />
        </SettingRow>
        <SettingRow
          label="保留文本格式"
          description="保留识别结果中的换行和段落结构"
          divided
        >
          <SettingsToggle
            checked={ocr.preserveFormatting}
            onChange={(preserveFormatting) =>
              updateOcr({ preserveFormatting })
            }
          />
        </SettingRow>
        <SettingRow
          label="中文自动去除空格"
          description="去除中文字符之间由识别产生的多余空格"
          divided
        >
          <SettingsToggle
            checked={ocr.removeChineseSpaces}
            onChange={(removeChineseSpaces) =>
              updateOcr({ removeChineseSpaces })
            }
          />
        </SettingRow>
        <SettingRow
          label="显示识别置信度"
          description="在 OCR 结果中显示整体识别置信度"
          divided
        >
          <SettingsToggle
            checked={ocr.showConfidence}
            onChange={(showConfidence) => updateOcr({ showConfidence })}
          />
        </SettingRow>
      </div>
    </div>
  );
}

function SettingRow({
  label,
  description,
  divided = false,
  children,
}: {
  label: string;
  description: string;
  divided?: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className={`flex items-center justify-between ${
        divided ? 'pt-6 border-t border-gray-100' : ''
      }`}
    >
      <div>
        <div className="font-medium text-gray-700">{label}</div>
        <div className="text-sm text-gray-500 mt-1">{description}</div>
      </div>
      {children}
    </div>
  );
}

function SettingsToggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative w-12 h-6 rounded-full transition-colors ${
        checked ? 'bg-primary-600' : 'bg-gray-300'
      }`}
    >
      <span
        className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform ${
          checked ? 'translate-x-6' : ''
        }`}
      />
    </button>
  );
}
