import { useState } from 'react';
import { HotkeyRow } from '../Hotkey/HotkeyRow';
import { HotkeyRecorderDialog } from '../Hotkey/HotkeyRecorderDialog';
import { useSettingsStore } from '../../../stores/settingsStore';

export function HotkeysPage() {
  const hotkeys = useSettingsStore((state) => state.hotkeys.ocr);
  const setHotkey = useSettingsStore((state) => state.setHotkey);
  const clearHotkey = useSettingsStore((state) => state.clearHotkey);
  const resetHotkeys = useSettingsStore((state) => state.resetHotkeys);

  const [recordingKey, setRecordingKey] = useState<string | null>(null);
  const [recordingLabel, setRecordingLabel] = useState<string>('');

  const hotkeyLabels: Record<string, string> = {
    'screenshot-ocr': '截图 OCR',
    'silent-screenshot-ocr': '静默截图 OCR',
    'file-ocr': '访问选图 OCR',
    'show-window': '显示 OCR 窗口',
  };

  const handleRecord = (key: string) => {
    setRecordingKey(key);
    setRecordingLabel(hotkeyLabels[key] || key);
  };

  const handleSaveHotkey = (newHotkey: string) => {
    if (recordingKey) {
      setHotkey('ocr', recordingKey, newHotkey);
    }
    setRecordingKey(null);
  };

  const handleClear = (key: string) => {
    clearHotkey('ocr', key);
  };

  const handleResetAll = () => {
    if (confirm('确定要恢复所有快捷键到默认值吗？')) {
      resetHotkeys('ocr');
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
        <p className="text-gray-600">配置 OCR 相关的全局快捷键。点击快捷键框进行录制。</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow
          label="截图 OCR"
          value={hotkeys['screenshot-ocr']}
          description="截图区域 → 自动 OCR → 显示识别结果"
          onRecord={() => handleRecord('screenshot-ocr')}
          onClear={() => handleClear('screenshot-ocr')}
        />
        <HotkeyRow
          label="静默截图 OCR"
          value={hotkeys['silent-screenshot-ocr']}
          description="后台识别，自动将结果拷贝到剪切板"
          onRecord={() => handleRecord('silent-screenshot-ocr')}
          onClear={() => handleClear('silent-screenshot-ocr')}
        />
        <HotkeyRow
          label="访问选图 OCR"
          value={hotkeys['file-ocr']}
          description="通过文件选择器选择图片进行 OCR"
          onRecord={() => handleRecord('file-ocr')}
          onClear={() => handleClear('file-ocr')}
        />
        <HotkeyRow
          label="显示 OCR 窗口"
          value={hotkeys['show-window']}
          description="直接显示 OCR 窗口"
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
