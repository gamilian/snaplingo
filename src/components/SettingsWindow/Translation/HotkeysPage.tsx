import { useState } from 'react';
import { HotkeyRow } from '../Hotkey/HotkeyRow';
import { HotkeyRecorderDialog } from '../Hotkey/HotkeyRecorderDialog';
import { useSettingsStore } from '../../../stores/settingsStore';

export function HotkeysPage() {
  const hotkeys = useSettingsStore((state) => state.hotkeys.translation);
  const setHotkey = useSettingsStore((state) => state.setHotkey);
  const clearHotkey = useSettingsStore((state) => state.clearHotkey);
  const resetHotkeys = useSettingsStore((state) => state.resetHotkeys);

  const [recordingKey, setRecordingKey] = useState<string | null>(null);
  const [recordingLabel, setRecordingLabel] = useState<string>('');

  const hotkeyLabels: Record<string, string> = {
    'selection-translate': '划词翻译',
    'screenshot-translate': '截图翻译',
    'input-translate': '输入翻译',
    'show-window': '显示翻译窗口',
  };

  const handleRecord = (key: string) => {
    setRecordingKey(key);
    setRecordingLabel(hotkeyLabels[key] || key);
  };

  const handleSaveHotkey = (newHotkey: string) => {
    if (recordingKey) {
      setHotkey('translation', recordingKey, newHotkey);
    }
    setRecordingKey(null);
  };

  const handleClear = (key: string) => {
    clearHotkey('translation', key);
  };

  const handleResetAll = () => {
    if (confirm('确定要恢复所有快捷键到默认值吗？')) {
      resetHotkeys('translation');
    }
  };

  const handleDetectConflicts = () => {
    const allHotkeys = Object.values(hotkeys);
    const duplicates = allHotkeys.filter((key, index) =>
      key !== '未设置' && allHotkeys.indexOf(key) !== index
    );

    if (duplicates.length > 0) {
      alert(`发现冲突的快捷键：${duplicates.join(', ')}`);
    } else {
      alert('未发现冲突的快捷键');
    }
  };

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">快捷键</h2>
        <p className="text-gray-600">配置翻译相关的全局快捷键。点击快捷键框进行录制。</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow
          label="划词翻译"
          value={hotkeys['selection-translate']}
          description="选中文字后触发翻译"
          onRecord={() => handleRecord('selection-translate')}
          onClear={() => handleClear('selection-translate')}
        />
        <HotkeyRow
          label="截图翻译"
          value={hotkeys['screenshot-translate']}
          description="截图区域 → OCR → 自动翻译"
          onRecord={() => handleRecord('screenshot-translate')}
          onClear={() => handleClear('screenshot-translate')}
        />
        <HotkeyRow
          label="输入翻译"
          value={hotkeys['input-translate']}
          description="清空翻译窗口并显示，用于手动输入"
          onRecord={() => handleRecord('input-translate')}
          onClear={() => handleClear('input-translate')}
        />
        <HotkeyRow
          label="显示翻译窗口"
          value={hotkeys['show-window']}
          description="直接显示翻译窗口，查看之前的结果"
          onRecord={() => handleRecord('show-window')}
          onClear={() => handleClear('show-window')}
        />
      </div>

      <div className="flex items-center justify-between pt-4">
        <button
          onClick={handleResetAll}
          className="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-lg transition-colors"
        >
          恢复所有默认值
        </button>
        <button
          onClick={handleDetectConflicts}
          className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
        >
          检测冲突
        </button>
      </div>

      <HotkeyRecorderDialog
        isOpen={recordingKey !== null}
        onClose={() => setRecordingKey(null)}
        onSave={handleSaveHotkey}
        currentHotkey={recordingKey ? hotkeys[recordingKey] : ''}
        label={recordingLabel}
      />
    </div>
  );
}
