import { useEffect, useState } from 'react';
import { CustomRange } from '../../../components/common/CustomRange';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { SettingRow, SettingsGroup } from '../SettingsControls';
import { useSettingsRuntime } from '../runtimeContext';

export function TtsProvidersPage() {
  const runtime = useSettingsRuntime();
  const general = useSettingsConfigStore((state) => state.general);
  const updateGeneralSettings = useSettingsConfigStore(
    (state) => state.updateGeneralSettings,
  );
  const [voices, setVoices] = useState<Array<{ name: string; locale: string }>>([]);
  const [testStatus, setTestStatus] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    runtime.tts.listVoices().then(
      (items) => {
        if (!disposed) setVoices(items);
      },
      () => {
        if (!disposed) setVoices([]);
      },
    );
    return () => {
      disposed = true;
    };
  }, [runtime]);

  if (!general) {
    return <div className="p-12 text-sm text-gray-500">设置加载中...</div>;
  }

  const voice = general.systemTtsVoice ?? '';
  const rate = general.systemTtsRate ?? 180;
  const updateTts = (input: { systemTtsVoice?: string; systemTtsRate?: number }) => {
    void updateGeneralSettings(input);
  };

  return (
    <div className="max-w-3xl space-y-4">
      <SettingsGroup title="macOS 系统语音">
        <SettingRow label="语音" description="使用 macOS 已安装的系统声音">
          <div className="w-[260px]">
            <CustomSelect
              value={voice}
              options={[
                { value: '', label: '系统默认' },
                ...voices.map((item) => ({
                  value: item.name,
                  label: `${item.name} · ${item.locale}`,
                })),
              ]}
              onChange={(systemTtsVoice) => updateTts({ systemTtsVoice })}
            />
          </div>
        </SettingRow>
        <SettingRow label="语速" description="每分钟朗读字数">
          <div className="flex w-[260px] items-center gap-3">
            <CustomRange
              value={rate}
              min={80}
              max={400}
              step={10}
              onChange={(systemTtsRate) => updateTts({ systemTtsRate })}
            />
            <span className="w-12 text-right text-xs font-medium text-gray-600">
              {rate}
            </span>
          </div>
        </SettingRow>
        <SettingRow label="试听">
          <div className="flex items-center gap-3">
            <button
              type="button"
              className="h-9 rounded-lg bg-emerald-500 px-4 text-sm font-medium text-white hover:bg-emerald-600"
              onClick={() => {
                setTestStatus('正在朗读');
                void runtime.tts.speak('SnapLingo 系统语音测试', 'zh-CN').then(
                  () => setTestStatus('已开始朗读'),
                  (error) =>
                    setTestStatus(error instanceof Error ? error.message : String(error)),
                );
              }}
            >
              测试朗读
            </button>
            {testStatus && <span className="text-xs text-gray-500">{testStatus}</span>}
          </div>
        </SettingRow>
      </SettingsGroup>
    </div>
  );
}
