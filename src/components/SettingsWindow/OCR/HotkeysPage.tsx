import { useState, useEffect } from 'react';
import { HotkeyRow } from '../Hotkey/HotkeyRow';
import { DEFAULT_HOTKEYS, useSettingsStore } from '../../../stores/settingsStore';
import { configureHotkey } from '../../../tauri/hotkeys';
import { saveHotkeyWithRegistration } from '../Hotkey/hotkeyRegistration';

export function HotkeysPage() {
  const hotkeys = useSettingsStore((state) => state.hotkeys.ocr);
  const setHotkey = useSettingsStore((state) => state.setHotkey);
  const clearHotkey = useSettingsStore((state) => state.clearHotkey);
  const resetHotkeys = useSettingsStore((state) => state.resetHotkeys);

  const [recordingKey, setRecordingKey] = useState<string | null>(null);

  const defaultHotkeys: Record<string, string> = DEFAULT_HOTKEYS.ocr;

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
          category: 'ocr',
          action: recordingKey,
          hotkey: hotkeyString,
          configureHotkey,
          setHotkey,
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
  }, [recordingKey, setHotkey]);

  const handleRecord = (key: string) => {
    // 如果已经在录制这个键，点击取消录制
    if (recordingKey === key) {
      setRecordingKey(null);
    } else {
      setRecordingKey(key);
    }
  };

  const handleClear = (key: string) => {
    clearHotkey('ocr', key);
  };

  const handleReset = (key: string) => {
    const defaultValue = defaultHotkeys[key];
    if (defaultValue) {
      setHotkey('ocr', key, defaultValue);
    }
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
        <p className="text-gray-600">配置 OCR 相关的全局快捷键。点击快捷键框直接录制，按 ESC 取消。</p>
      </div>

      <div className="bg-white rounded-lg border border-gray-200 divide-y divide-gray-100">
        <HotkeyRow
          label="截图 OCR"
          value={hotkeys['screenshot-ocr']}
          description="截图区域 → 自动 OCR → 显示识别结果"
          onRecord={() => handleRecord('screenshot-ocr')}
          onClear={() => handleClear('screenshot-ocr')}
          onReset={() => handleReset('screenshot-ocr')}
          isRecording={recordingKey === 'screenshot-ocr'}
          defaultValue={defaultHotkeys['screenshot-ocr']}
        />
        <HotkeyRow
          label="静默截图 OCR"
          value={hotkeys['silent-screenshot-ocr']}
          description="后台识别，自动将结果拷贝到剪切板"
          onRecord={() => handleRecord('silent-screenshot-ocr')}
          onClear={() => handleClear('silent-screenshot-ocr')}
          onReset={() => handleReset('silent-screenshot-ocr')}
          isRecording={recordingKey === 'silent-screenshot-ocr'}
          defaultValue={defaultHotkeys['silent-screenshot-ocr']}
        />
        <HotkeyRow
          label="访问选图 OCR"
          value={hotkeys['file-ocr']}
          description="通过文件选择器选择图片进行 OCR"
          onRecord={() => handleRecord('file-ocr')}
          onClear={() => handleClear('file-ocr')}
          onReset={() => handleReset('file-ocr')}
          isRecording={recordingKey === 'file-ocr'}
          defaultValue={defaultHotkeys['file-ocr']}
        />
        <HotkeyRow
          label="显示 OCR 窗口"
          value={hotkeys['show-window']}
          description="直接显示 OCR 窗口"
          onRecord={() => handleRecord('show-window')}
          onClear={() => handleClear('show-window')}
          onReset={() => handleReset('show-window')}
          isRecording={recordingKey === 'show-window'}
          defaultValue={defaultHotkeys['show-window']}
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
