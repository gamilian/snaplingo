import type { OcrSettings, ResultWindowPosition } from '../../../application/settings/ports';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { FeatureHotkeysSection, HotkeyToolbar } from '../Hotkey/FeatureHotkeysSection';
import { SettingRow, SettingsGroup, SettingsToggle } from '../SettingsControls';
import { SettingsScrollPage } from '../SettingsScrollPage';

const OCR_HOTKEYS = [
  { key: 'screenshot-ocr', label: '截图 OCR' },
  { key: 'silent-screenshot-ocr', label: '静默截图 OCR' },
  { key: 'file-ocr', label: '上传图片 OCR' },
];

export function OcrSettingsScrollPage() {
  const ocr = useSettingsConfigStore((state) => state.ocr);
  const updateOcrSettings = useSettingsConfigStore(
    (state) => state.updateOcrSettings,
  );

  if (!ocr) {
    return <div className="p-12 text-sm text-gray-500">设置加载中...</div>;
  }

  const updateOcr = (input: Partial<OcrSettings>) => {
    void updateOcrSettings(input);
  };

  return (
    <SettingsScrollPage
      title="OCR"
      description="快捷键、识别文本与结果窗口"
      sections={[
        {
          id: 'hotkeys',
          label: '快捷键',
          description: '常用 OCR 入口的全局快捷键',
          action: <HotkeyToolbar category="ocr" actions={OCR_HOTKEYS} />,
          content: (
            <FeatureHotkeysSection category="ocr" actions={OCR_HOTKEYS} />
          ),
        },
        {
          id: 'recognition',
          label: '识别与文本',
          description: '识别语言与结果文本的处理方式',
          content: (
            <SettingsGroup title="识别">
              <SettingRow label="识别语言">
                <div className="w-[230px]">
                  <CustomSelect
                    value={ocr.recognitionLanguage}
                    options={[
                      { value: 'auto', label: '自动检测' },
                      { value: 'zh-CN', label: '中文简体' },
                      { value: 'zh-TW', label: '中文繁體' },
                      { value: 'en', label: 'English' },
                      { value: 'ja', label: '日本語' },
                      { value: 'ko', label: '한국어' },
                    ]}
                    onChange={(recognitionLanguage) =>
                      updateOcr({ recognitionLanguage })
                    }
                  />
                </div>
              </SettingRow>
              <ToggleRow
                label="保留文本格式"
                description="保留换行和段落结构"
                checked={ocr.preserveFormatting}
                onChange={(preserveFormatting) =>
                  updateOcr({ preserveFormatting })
                }
              />
              <ToggleRow
                label="中文自动去除空格"
                checked={ocr.removeChineseSpaces}
                onChange={(removeChineseSpaces) =>
                  updateOcr({ removeChineseSpaces })
                }
              />
              <ToggleRow
                label="显示识别置信度"
                checked={ocr.showConfidence}
                onChange={(showConfidence) => updateOcr({ showConfidence })}
              />
            </SettingsGroup>
          ),
        },
        {
          id: 'window',
          label: '窗口与提示',
          description: 'OCR 结果窗口位置与静默识别提示',
          content: (
            <>
              <SettingsGroup title="结果窗口">
                <SettingRow label="OCR 窗口位置">
                  <div className="w-[230px]">
                    <CustomSelect
                      value={ocr.windowPosition ?? 'cursor'}
                      options={[
                        { value: 'center', label: '居中' },
                        { value: 'below-cursor', label: '鼠标下方' },
                        { value: 'cursor', label: '鼠标位置' },
                        { value: 'last-position', label: '上次停留位置' },
                      ]}
                      onChange={(windowPosition) =>
                        updateOcr({
                          windowPosition: windowPosition as ResultWindowPosition,
                        })
                      }
                    />
                  </div>
                </SettingRow>
              </SettingsGroup>
              <SettingsGroup title="静默 OCR">
                <ToggleRow
                  label="隐藏识别状态提示"
                  description="开启后，后台识别时不再在鼠标位置显示加载和成功提示"
                  checked={ocr.hideSilentStatus ?? false}
                  onChange={(hideSilentStatus) =>
                    updateOcr({ hideSilentStatus })
                  }
                />
              </SettingsGroup>
            </>
          ),
        },
      ]}
    />
  );
}

function ToggleRow({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <SettingRow label={label} description={description}>
      <SettingsToggle label={label} checked={checked} onChange={onChange} />
    </SettingRow>
  );
}
