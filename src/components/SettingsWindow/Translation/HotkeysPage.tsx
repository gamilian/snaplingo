import { useState, useEffect } from 'react';
import { HotkeyRow } from '../Hotkey/HotkeyRow';
import { useSettingsStore } from '../../../stores/settingsStore';

export function HotkeysPage() {
  const hotkeys = useSettingsStore((state) => state.hotkeys.translation);
  const setHotkey = useSettingsStore((state) => state.setHotkey);
  const clearHotkey = useSettingsStore((state) => state.clearHotkey);
  const resetHotkeys = useSettingsStore((state) => state.resetHotkeys);

  const [recordingKey, setRecordingKey] = useState<string | null>(null);
  const [debugLogs, setDebugLogs] = useState<string[]>([]);

  const addDebugLog = (message: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setDebugLogs(prev => [...prev.slice(-10), `${timestamp}: ${message}`]);
  };

  // 默认快捷键配置
  const defaultHotkeys: Record<string, string> = {
    'selection-translate': '⌘⇧D',
    'screenshot-translate': '⌘⇧S',
    'input-translate': '⌘⇧T',
    'show-window': '⌘⇧W',
  };

  // 监听键盘事件进行录制
  useEffect(() => {
    if (!recordingKey) {
      addDebugLog('录制已停止');
      return;
    }

    addDebugLog(`开始录制: ${recordingKey}`);

    const handleKeyDown = (e: KeyboardEvent) => {
      addDebugLog(`按键: ${e.key}, code: ${e.code}, meta:${e.metaKey}, shift:${e.shiftKey}, alt:${e.altKey}, ctrl:${e.ctrlKey}`);

      e.preventDefault();
      e.stopPropagation();

      if (e.key === 'Escape') {
        addDebugLog('ESC取消录制');
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

      addDebugLog(`主键: ${mainKey}, 修饰键: ${modifiers.join('')}`);

      if (mainKey) {
        const hotkeyString = modifiers.join('') + mainKey;
        addDebugLog(`✅ 保存: ${hotkeyString}`);
        setHotkey('translation', recordingKey, hotkeyString);
        setRecordingKey(null);
      } else {
        addDebugLog(`❌ 未识别: ${e.key} (code: ${e.code})`);
      }
    };

    // 点击空白处取消录制
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      // 如果点击的不是快捷键相关元素，取消录制
      if (!target.closest('button') && !target.closest('.hotkey-display')) {
        addDebugLog('点击空白处，取消录制');
        setRecordingKey(null);
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    document.addEventListener('mousedown', handleClickOutside, true);
    addDebugLog('✓ 已添加键盘和鼠标监听器');

    return () => {
      window.removeEventListener('keydown', handleKeyDown, true);
      document.removeEventListener('mousedown', handleClickOutside, true);
      addDebugLog('✗ 已移除监听器');
    };
  }, [recordingKey, setHotkey]);

  const handleRecord = (key: string) => {
    addDebugLog(`点击录制: ${key}, 当前状态: ${recordingKey}`);
    // 如果已经在录制这个键，点击取消录制
    if (recordingKey === key) {
      addDebugLog('取消录制');
      setRecordingKey(null);
    } else {
      addDebugLog('开始录制');
      setRecordingKey(key);
    }
  };

  const handleClear = (key: string) => {
    clearHotkey('translation', key);
  };

  const handleReset = (key: string) => {
    const defaultValue = defaultHotkeys[key];
    if (defaultValue) {
      setHotkey('translation', key, defaultValue);
    }
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
      {/* 调试日志 */}
      <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4">
        <div className="flex items-center justify-between mb-2">
          <h3 className="font-semibold text-yellow-900">🐛 调试日志</h3>
          <button
            onClick={() => setDebugLogs([])}
            className="text-xs px-2 py-1 bg-yellow-200 hover:bg-yellow-300 rounded"
          >
            清除
          </button>
        </div>
        <div className="bg-gray-900 text-green-400 font-mono text-xs p-2 rounded max-h-32 overflow-y-auto">
          {debugLogs.length === 0 ? (
            <div className="text-gray-500">暂无日志...</div>
          ) : (
            debugLogs.map((log, i) => <div key={i}>{log}</div>)
          )}
        </div>
        <div className="mt-2 text-xs text-yellow-700">
          录制状态: <span className="font-bold">{recordingKey || '未录制'}</span>
        </div>
      </div>

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
          defaultValue={defaultHotkeys['selection-translate']}
        />
        <HotkeyRow
          label="截图翻译"
          value={hotkeys['screenshot-translate']}
          description="截图区域 → OCR → 自动翻译"
          onRecord={() => handleRecord('screenshot-translate')}
          onClear={() => handleClear('screenshot-translate')}
          onReset={() => handleReset('screenshot-translate')}
          isRecording={recordingKey === 'screenshot-translate'}
          defaultValue={defaultHotkeys['screenshot-translate']}
        />
        <HotkeyRow
          label="输入翻译"
          value={hotkeys['input-translate']}
          description="清空翻译窗口并显示，用于手动输入"
          onRecord={() => handleRecord('input-translate')}
          onClear={() => handleClear('input-translate')}
          onReset={() => handleReset('input-translate')}
          isRecording={recordingKey === 'input-translate'}
          defaultValue={defaultHotkeys['input-translate']}
        />
        <HotkeyRow
          label="显示翻译窗口"
          value={hotkeys['show-window']}
          description="直接显示翻译窗口，查看之前的结果"
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
