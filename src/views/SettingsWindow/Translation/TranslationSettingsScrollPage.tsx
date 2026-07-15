import type { ReactNode } from 'react';
import type {
  ResultWindowPosition,
  SelectionTextMode,
  TranslationInputState,
  TranslationSettings,
} from '../../../application/settings/ports';
import { TRANSLATION_LANGUAGES } from '../../../application/translation/languages';
import { CustomRange } from '../../../components/common/CustomRange';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { FeatureHotkeysSection } from '../Hotkey/FeatureHotkeysSection';
import { SettingRow, SettingsGroup, SettingsToggle } from '../SettingsControls';
import { SettingsScrollPage } from '../SettingsScrollPage';

const TRANSLATION_HOTKEYS = [
  { key: 'selection-translate', label: '划词翻译' },
  { key: 'screenshot-translate', label: '截图翻译' },
  { key: 'show-window', label: '显示翻译窗口' },
];

const POSITION_OPTIONS = [
  { value: 'center', label: '居中' },
  { value: 'below-cursor', label: '鼠标下方' },
  { value: 'cursor', label: '鼠标位置' },
];

const INPUT_STATE_OPTIONS = [
  { value: 'last', label: '上次状态' },
  { value: 'collapsed', label: '总是折叠' },
  { value: 'expanded', label: '总是展开' },
];

export function TranslationSettingsScrollPage() {
  const translation = useSettingsConfigStore((state) => state.translation);
  const updateTranslationSettings = useSettingsConfigStore(
    (state) => state.updateTranslationSettings,
  );

  if (!translation) {
    return <div className="p-12 text-sm text-gray-500">设置加载中...</div>;
  }

  const updateTranslation = (input: Partial<TranslationSettings>) => {
    void updateTranslationSettings(input);
  };

  const languageOptions = TRANSLATION_LANGUAGES.map((language) => ({
    value: language.code,
    label: language.chineseName,
  }));

  return (
    <SettingsScrollPage
      title="翻译"
      description="快捷键、语言规则、窗口与翻译行为"
      sections={[
        {
          id: 'hotkeys',
          label: '快捷键',
          description: '常用翻译入口的全局快捷键',
          content: (
            <FeatureHotkeysSection
              category="translation"
              actions={TRANSLATION_HOTKEYS}
            />
          ),
        },
        {
          id: 'language',
          label: '语言与取词',
          description: '默认语言与划词翻译的取词策略',
          content: (
            <>
              <SettingsGroup title="默认语言">
                <SettingRow label="源语言">
                  <SelectField
                    value={translation.defaultSourceLang}
                    options={languageOptions}
                    onChange={(defaultSourceLang) =>
                      updateTranslation({ defaultSourceLang })
                    }
                  />
                </SettingRow>
                <SettingRow
                  label="目标语言"
                  description="自动：源语言为中文时翻译成英语，其他语言翻译成中文"
                >
                  <SelectField
                    value={translation.defaultTargetLang}
                    options={[
                      { value: 'auto', label: '自动' },
                      ...languageOptions.filter((option) => option.value !== 'auto'),
                    ]}
                    onChange={(defaultTargetLang) =>
                      updateTranslation({ defaultTargetLang })
                    }
                  />
                </SettingRow>
              </SettingsGroup>
              <SettingsGroup title="划词翻译">
                <SettingRow
                  label="取词模式"
                  description="智能模式会根据应用和文本类型选择取词方式"
                >
                  <SelectField
                    value={translation.selectionTextMode ?? 'smart'}
                    options={[
                      { value: 'smart', label: '智能模式' },
                      { value: 'quality', label: '效果优先' },
                      { value: 'speed', label: '速度优先' },
                    ]}
                    onChange={(selectionTextMode) =>
                      updateTranslation({
                        selectionTextMode: selectionTextMode as SelectionTextMode,
                      })
                    }
                  />
                </SettingRow>
              </SettingsGroup>
            </>
          ),
        },
        {
          id: 'window',
          label: '窗口与输入框',
          description: '翻译窗口的位置、尺寸与输入框初始状态',
          content: (
            <>
              <SettingsGroup title="窗口位置">
                <SettingRow label="划词翻译与截图翻译">
                  <PositionSelect
                    value={translation.selectionWindowPosition ?? 'below-cursor'}
                    onChange={(selectionWindowPosition) =>
                      updateTranslation({ selectionWindowPosition })
                    }
                  />
                </SettingRow>
                <SettingRow label="手动翻译窗口">
                  <PositionSelect
                    value={translation.inputWindowPosition ?? 'center'}
                    onChange={(inputWindowPosition) =>
                      updateTranslation({ inputWindowPosition })
                    }
                  />
                </SettingRow>
              </SettingsGroup>
              <SettingsGroup title="窗口尺寸">
                <SettingRow label="最高屏幕占比">
                  <RangeValue value={`${translation.maxWindowHeightRatio ?? 70}%`}>
                    <CustomRange
                      value={translation.maxWindowHeightRatio ?? 70}
                      min={30}
                      max={90}
                      step={1}
                      onChange={(maxWindowHeightRatio) =>
                        updateTranslation({ maxWindowHeightRatio })
                      }
                    />
                  </RangeValue>
                </SettingRow>
                <SettingRow label="窗口宽度">
                  <RangeValue value={`${translation.windowWidth ?? 660}pt`}>
                    <CustomRange
                      value={translation.windowWidth ?? 660}
                      min={300}
                      max={1000}
                      step={10}
                      onChange={(windowWidth) => updateTranslation({ windowWidth })}
                    />
                  </RangeValue>
                </SettingRow>
              </SettingsGroup>
              <SettingsGroup title="输入框初始状态">
                <SettingRow label="划词翻译">
                  <InputStateSelect
                    value={translation.selectionInputState ?? 'last'}
                    onChange={(selectionInputState) =>
                      updateTranslation({ selectionInputState })
                    }
                  />
                </SettingRow>
                <SettingRow label="截图翻译">
                  <InputStateSelect
                    value={translation.screenshotInputState ?? 'last'}
                    onChange={(screenshotInputState) =>
                      updateTranslation({ screenshotInputState })
                    }
                  />
                </SettingRow>
              </SettingsGroup>
            </>
          ),
        },
        {
          id: 'behavior',
          label: '翻译行为',
          description: '输入、结果与窗口的通用行为',
          content: (
            <>
              <SettingsGroup title="输入与结果">
                <ToggleRow
                  label="自动翻译"
                  description="输入文本后自动开始翻译"
                  checked={translation.autoTranslate}
                  onChange={(autoTranslate) => updateTranslation({ autoTranslate })}
                />
                <ToggleRow
                  label="增量翻译"
                  description="输入时实时更新翻译结果"
                  checked={translation.incrementalTranslation}
                  onChange={(incrementalTranslation) =>
                    updateTranslation({ incrementalTranslation })
                  }
                />
                <ToggleRow
                  label="翻译后自动复制"
                  checked={translation.autoCopy}
                  onChange={(autoCopy) => updateTranslation({ autoCopy })}
                />
                <ToggleRow
                  label="保留原文换行"
                  checked={translation.preserveLineBreaks}
                  onChange={(preserveLineBreaks) =>
                    updateTranslation({ preserveLineBreaks })
                  }
                />
              </SettingsGroup>
              <SettingsGroup title="窗口行为">
                <ToggleRow
                  label="结果窗口置顶"
                  checked={translation.windowAlwaysOnTop}
                  onChange={(windowAlwaysOnTop) =>
                    updateTranslation({ windowAlwaysOnTop })
                  }
                />
                <ToggleRow
                  label="失去焦点时隐藏"
                  checked={translation.hideOnBlur}
                  onChange={(hideOnBlur) => updateTranslation({ hideOnBlur })}
                />
              </SettingsGroup>
            </>
          ),
        },
      ]}
    />
  );
}

function SelectField({
  value,
  options,
  onChange,
}: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
}) {
  return (
    <div className="w-[230px]">
      <CustomSelect value={value} options={options} onChange={onChange} />
    </div>
  );
}

function PositionSelect({
  value,
  onChange,
}: {
  value: ResultWindowPosition;
  onChange: (value: ResultWindowPosition) => void;
}) {
  return (
    <SelectField
      value={value}
      options={POSITION_OPTIONS}
      onChange={(next) => onChange(next as ResultWindowPosition)}
    />
  );
}

function InputStateSelect({
  value,
  onChange,
}: {
  value: TranslationInputState;
  onChange: (value: TranslationInputState) => void;
}) {
  return (
    <SelectField
      value={value}
      options={INPUT_STATE_OPTIONS}
      onChange={(next) => onChange(next as TranslationInputState)}
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

function RangeValue({ children, value }: { children: ReactNode; value: string }) {
  return (
    <div className="flex w-[270px] items-center gap-3">
      <div className="min-w-0 flex-1">{children}</div>
      <span className="min-w-[48px] text-right text-[11px] font-semibold text-primary-600">
        {value}
      </span>
    </div>
  );
}
