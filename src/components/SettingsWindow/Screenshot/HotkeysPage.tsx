import { useState } from 'react';
import { HotkeyRow } from '../Hotkey/HotkeyRow';
import { HotkeyRecorderDialog } from '../Hotkey/HotkeyRecorderDialog';
import { useSettingsStore } from '../../../stores/settingsStore';

export function HotkeysPage() {
  const hotkeys = useSettingsStore((state) => state.hotkeys.screenshot);
  const setHotkey = useSettingsStore((state) => state.setHotkey);
  const clearHotkey = useSettingsStore((state) => state.clearHotkey);
  const resetHotkeys = useSettingsStore((state) => state.resetHotkeys);

  const [recordingKey, setRecordingKey] = useState<string | null>(null);
  const [recordingLabel, setRecordingLabel] = useState<string>('');

  const hotkeyLabels: Record<string, string> = {
    screenshot: '截屏',
    'screenshot-copy': '截屏并自动复制',
    'screenshot-custom': '自定义截屏',
    pin: '贴图',
    'pin-toggle-all': '隐藏/显示所有贴图',
    'pin-switch-group': '切换到另一贴图组',
  };

  const handleRecord = (key: string) => {
    setRecordingKey(key);
    setRecordingLabel(hotkeyLabels[key] || key);
  };

  const handleSaveHotkey = (newHotkey: string) => {
    if (recordingKey) {
      setHotkey('screenshot', recordingKey, newHotkey);
    }
    setRecordingKey(null);
  };

  const handleClear = (key: string) => {
    clearHotkey('screenshot', key);
  };

  const handleResetAll = () => {
    if (confirm('确定要恢复所有快捷键到默认值吗？')) {
      resetHotkeys('screenshot');
    }
  };

  const handleDetectConflicts = () => {
    // 检测冲突逻辑
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
        <p className="text-gray-600">配置截图相关的全局快捷键。点击快捷键框进行录制。</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow
          label="截屏"
          value={hotkeys['screenshot']}
          onRecord={() => handleRecord('screenshot')}
          onClear={() => handleClear('screenshot')}
        />
        <HotkeyRow
          label="截屏并自动复制"
          value={hotkeys['screenshot-copy']}
          onRecord={() => handleRecord('screenshot-copy')}
          onClear={() => handleClear('screenshot-copy')}
        />
        <HotkeyRow
          label="自定义截屏"
          value={hotkeys['screenshot-custom']}
          onRecord={() => handleRecord('screenshot-custom')}
          onClear={() => handleClear('screenshot-custom')}
        />
        <HotkeyRow
          label="贴图"
          value={hotkeys['pin']}
          onRecord={() => handleRecord('pin')}
          onClear={() => handleClear('pin')}
        />
        <HotkeyRow
          label="隐藏/显示所有贴图"
          value={hotkeys['pin-toggle-all']}
          onRecord={() => handleRecord('pin-toggle-all')}
          onClear={() => handleClear('pin-toggle-all')}
        />
        <HotkeyRow
          label="切换到另一贴图组"
          value={hotkeys['pin-switch-group']}
          onRecord={() => handleRecord('pin-switch-group')}
          onClear={() => handleClear('pin-switch-group')}
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

      {/* 快捷键录制对话框 */}
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
