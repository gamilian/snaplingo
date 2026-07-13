import { useState, useEffect } from 'react';
import { HotkeyRow } from '../Hotkey/HotkeyRow';
import { useHotkeyConfigStore } from '../../../stores/hotkeyConfigStore';
import { saveHotkeyWithRegistration } from '../Hotkey/hotkeyRegistration';

export function HotkeysPage() {
  const snapshot = useHotkeyConfigStore((state) => state.snapshot);
  const defaultSnapshot = useHotkeyConfigStore((state) => state.defaultSnapshot);
  const updateHotkey = useHotkeyConfigStore((state) => state.updateHotkey);
  const resetHotkey = useHotkeyConfigStore((state) => state.resetHotkey);
  const resetCategory = useHotkeyConfigStore((state) => state.resetCategory);

  const [recordingKey, setRecordingKey] = useState<string | null>(null);

  const hotkeys = snapshot?.screenshot;
  const defaultHotkeys = defaultSnapshot?.screenshot ?? hotkeys;

  // 监听键盘事件进行录制
  useEffect(() => {
    if (!recordingKey) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // ESC取消录制
      if (e.key === 'Escape') {
        setRecordingKey(null);
        return;
      }

      // 构建快捷键字符串
      const modifiers = [];
      if (e.shiftKey) modifiers.push('⇧');
      if (e.altKey) modifiers.push('⌥');
      if (e.metaKey) modifiers.push('⌘');
      if (e.ctrlKey) modifiers.push('⌃');

      // 使用 e.code 来获取物理按键，避免 Option 键导致的字符转换问题
      let mainKey = '';

      // 处理字母键 (KeyA-KeyZ)
      if (e.code.startsWith('Key')) {
        mainKey = e.code.replace('Key', '');
      }
      // 处理数字键 (Digit0-Digit9)
      else if (e.code.startsWith('Digit')) {
        mainKey = e.code.replace('Digit', '');
      }
      // 处理功能键 (F1-F12)
      else if (e.code.startsWith('F') && /^F\d+$/.test(e.code)) {
        mainKey = e.code;
      }

      if (mainKey) {
        const hotkeyString = modifiers.join('') + mainKey;
        void saveHotkeyWithRegistration({
          category: 'screenshot',
          action: recordingKey,
          hotkey: hotkeyString,
          updateHotkey,
          reportError: alert,
        }).then(() => setRecordingKey(null));
      }
    };

    // 点击空白处取消录制
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.closest('button') && !target.closest('.hotkey-display')) {
        setRecordingKey(null);
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    document.addEventListener('mousedown', handleClickOutside, true);
    return () => {
      window.removeEventListener('keydown', handleKeyDown, true);
      document.removeEventListener('mousedown', handleClickOutside, true);
    };
  }, [recordingKey, updateHotkey]);

  const handleRecord = (key: string) => {
    // 如果已经在录制这个键，点击取消录制
    if (recordingKey === key) {
      setRecordingKey(null);
    } else {
      setRecordingKey(key);
    }
  };

  const handleClear = (key: string) => {
    void updateHotkey('screenshot', key, '未设置').catch((err) => {
      alert(`快捷键 ${key} 清除失败：${err}`);
    });
  };

  const handleReset = (key: string) => {
    void resetHotkey('screenshot', key).catch((err) => {
      alert(`快捷键 ${key} 重置失败：${err}`);
    });
  };

  const handleResetAll = () => {
    if (confirm('确定要恢复所有快捷键到默认值吗？')) {
      void resetCategory('screenshot').catch((err) => {
        alert(`快捷键重置失败：${err}`);
      });
    }
  };

  if (!hotkeys || !defaultHotkeys) {
    return <div className="text-sm text-gray-500">正在加载快捷键配置...</div>;
  }

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
        <p className="text-gray-600">
          配置截图相关的全局快捷键。点击快捷键框直接录制，按 ESC 取消。
        </p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow
          label="截屏"
          value={hotkeys['screenshot']}
          onRecord={() => handleRecord('screenshot')}
          onClear={() => handleClear('screenshot')}
          onReset={() => handleReset('screenshot')}
          isRecording={recordingKey === 'screenshot'}
          defaultValue={defaultHotkeys['screenshot'] ?? '未设置'}
        />
        <HotkeyRow
          label="截屏并自动复制"
          value={hotkeys['screenshot-copy']}
          onRecord={() => handleRecord('screenshot-copy')}
          onClear={() => handleClear('screenshot-copy')}
          onReset={() => handleReset('screenshot-copy')}
          isRecording={recordingKey === 'screenshot-copy'}
          defaultValue={defaultHotkeys['screenshot-copy'] ?? '未设置'}
        />
        <HotkeyRow
          label="贴图"
          value={hotkeys['pin']}
          onRecord={() => handleRecord('pin')}
          onClear={() => handleClear('pin')}
          onReset={() => handleReset('pin')}
          isRecording={recordingKey === 'pin'}
          defaultValue={defaultHotkeys['pin'] ?? '未设置'}
        />
        <HotkeyRow
          label="隐藏/显示所有贴图"
          value={hotkeys['pin-toggle-all']}
          onRecord={() => handleRecord('pin-toggle-all')}
          onClear={() => handleClear('pin-toggle-all')}
          onReset={() => handleReset('pin-toggle-all')}
          isRecording={recordingKey === 'pin-toggle-all'}
          defaultValue={defaultHotkeys['pin-toggle-all'] ?? '未设置'}
        />
        <HotkeyRow
          label="切换到另一贴图组"
          value={hotkeys['pin-switch-group']}
          onRecord={() => handleRecord('pin-switch-group')}
          onClear={() => handleClear('pin-switch-group')}
          onReset={() => handleReset('pin-switch-group')}
          isRecording={recordingKey === 'pin-switch-group'}
          defaultValue={defaultHotkeys['pin-switch-group'] ?? '未设置'}
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
          className="px-4 py-2 text-sm bg-primary-600 hover:bg-primary-700 text-white rounded-lg transition-colors"
        >
          检测冲突
        </button>
      </div>
    </div>
  );
}
