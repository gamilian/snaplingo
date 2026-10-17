import { useState, useEffect } from 'react';

interface HotkeyRecorderDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (hotkey: string) => void;
  currentHotkey: string;
  label: string;
}

export function HotkeyRecorderDialog({ isOpen, onClose, onSave, currentHotkey, label }: HotkeyRecorderDialogProps) {
  const [recording, setRecording] = useState(false);
  const [recordedKeys, setRecordedKeys] = useState<string[]>([]);

  useEffect(() => {
    if (!isOpen || !recording) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const keys: string[] = [];

      // 修饰键
      if (e.metaKey || e.key === 'Meta') keys.push('⌘');
      if (e.ctrlKey || e.key === 'Control') keys.push('⌃');
      if (e.altKey || e.key === 'Alt') keys.push('⌥');
      if (e.shiftKey || e.key === 'Shift') keys.push('⇧');

      // 主键
      if (
        e.key !== 'Meta' &&
        e.key !== 'Control' &&
        e.key !== 'Alt' &&
        e.key !== 'Shift'
      ) {
        // 功能键
        if (e.key.startsWith('F') && e.key.length <= 3) {
          keys.push(e.key);
        }
        // 字母数字
        else if (e.key.length === 1) {
          keys.push(e.key.toUpperCase());
        }
        // 特殊键
        else if (['Escape', 'Enter', 'Space', 'Tab', 'Backspace', 'Delete'].includes(e.key)) {
          keys.push(e.key);
        }
      }

      if (keys.length > 0) {
        setRecordedKeys(keys);
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    return () => window.removeEventListener('keydown', handleKeyDown, true);
  }, [isOpen, recording]);

  const handleStartRecording = () => {
    setRecording(true);
    setRecordedKeys([]);
  };

  const handleStopRecording = () => {
    setRecording(false);
  };

  const handleSave = () => {
    if (recordedKeys.length > 0) {
      onSave(recordedKeys.join(''));
    }
    handleClose();
  };

  const handleClose = () => {
    setRecording(false);
    setRecordedKeys([]);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={handleClose}>
      <div
        className="bg-surface rounded-xl shadow-2xl w-96 p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-xl font-bold text-ink mb-2">录制快捷键</h3>
        <p className="text-sm text-muted mb-6">{label}</p>

        <div className="mb-6">
          <div className="text-sm text-muted mb-2">当前快捷键：</div>
          <div className="px-4 py-3 bg-sunken rounded-lg text-center">
            <span className="text-lg font-mono text-ink">{currentHotkey}</span>
          </div>
        </div>

        <div className="mb-6">
          <div className="text-sm text-muted mb-2">新快捷键：</div>
          <div
            className={`px-4 py-8 rounded-lg text-center border-2 transition-colors ${
              recording
                ? 'border-blue-500 bg-blue-50'
                : 'border-line bg-sunken'
            }`}
          >
            {recording ? (
              <div>
                <div className="text-sm text-accent-text mb-2">按下快捷键...</div>
                <div className="text-2xl font-mono text-ink min-h-[32px]">
                  {recordedKeys.length > 0 ? recordedKeys.join('') : '等待输入'}
                </div>
              </div>
            ) : (
              <button
                onClick={handleStartRecording}
                className="text-accent-text hover:text-blue-700 font-medium"
              >
                点击开始录制
              </button>
            )}
          </div>
        </div>

        <div className="flex items-center justify-between">
          <button
            onClick={handleClose}
            className="px-4 py-2 text-sm text-muted hover:text-ink hover:bg-raised rounded-lg transition-colors"
          >
            取消
          </button>
          <div className="flex items-center space-x-2">
            {recording && (
              <button
                onClick={handleStopRecording}
                className="px-4 py-2 text-sm text-muted hover:text-ink hover:bg-raised rounded-lg transition-colors"
              >
                停止录制
              </button>
            )}
            <button
              onClick={handleSave}
              disabled={recordedKeys.length === 0}
              className="px-4 py-2 text-sm bg-accent hover:bg-accent-hover text-accent-on rounded-lg transition-colors disabled:bg-line-strong disabled:cursor-not-allowed"
            >
              保存
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
