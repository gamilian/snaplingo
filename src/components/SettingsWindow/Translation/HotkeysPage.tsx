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

  const hotkeys = snapshot?.translation;
  const defaultHotkeys = defaultSnapshot?.translation ?? hotkeys;

  // 监听键盘事件进行录制
  useEffect(() => {
    if (!recordingKey) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === 'Escape') {
        setRecordingKey(null);
        return;
      }

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
          category: 'translation',
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
      // 如果点击的不是快捷键相关元素，取消录制
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
    void updateHotkey('translation', key, '未设置').catch((err) => {
      alert(`快捷键 ${key} 清除失败：${err}`);
    });
  };

  const handleReset = (key: string) => {
    void resetHotkey('translation', key).catch((err) => {
      alert(`快捷键 ${key} 重置失败：${err}`);
    });
  };

  const handleResetAll = () => {
    if (confirm('确定要恢复所有快捷键到默认值吗？')) {
      void resetCategory('translation').catch((err) => {
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
        <p className="text-gray-600">配置翻译相关的全局快捷键。点击快捷键框直接录制，按 ESC 取消。</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow
          label="划词翻译"
          value={hotkeys['selection-translate']}
          description="选中文字后触发翻译"
          onRecord={() => handleRecord('selection-translate')}
          onClear={() => handleClear('selection-translate')}
          onReset={() => handleReset('selection-translate')}
          isRecording={recordingKey === 'selection-translate'}
          defaultValue={defaultHotkeys['selection-translate'] ?? '未设置'}
        />
        <HotkeyRow
          label="截图翻译"
          value={hotkeys['screenshot-translate']}
          description="截图区域 → OCR → 自动翻译"
          onRecord={() => handleRecord('screenshot-translate')}
          onClear={() => handleClear('screenshot-translate')}
          onReset={() => handleReset('screenshot-translate')}
          isRecording={recordingKey === 'screenshot-translate'}
          defaultValue={defaultHotkeys['screenshot-translate'] ?? '未设置'}
        />
        <HotkeyRow
          label="输入翻译"
          value={hotkeys['input-translate']}
          description="清空翻译窗口并显示，用于手动输入"
          onRecord={() => handleRecord('input-translate')}
          onClear={() => handleClear('input-translate')}
          onReset={() => handleReset('input-translate')}
          isRecording={recordingKey === 'input-translate'}
          defaultValue={defaultHotkeys['input-translate'] ?? '未设置'}
        />
        <HotkeyRow
          label="显示翻译窗口"
          value={hotkeys['show-window']}
          description="直接显示翻译窗口，查看之前的结果"
          onRecord={() => handleRecord('show-window')}
          onClear={() => handleClear('show-window')}
          onReset={() => handleReset('show-window')}
          isRecording={recordingKey === 'show-window'}
          defaultValue={defaultHotkeys['show-window'] ?? '未设置'}
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
