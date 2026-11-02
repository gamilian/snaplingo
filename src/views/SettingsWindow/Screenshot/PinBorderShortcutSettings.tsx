import { useState } from 'react';
import { normalizePinBorderShortcut, pinBorderShortcutFromEvent } from '../../../application/pinned-image/borderShortcut';
import { useHotkeyConfigStore } from '../../../stores/hotkeyConfigStore';
import { SettingRow } from '../SettingsControls';
import { useNativeShortcutRecording } from './useNativeShortcutRecording';

export function PinBorderShortcutSettings({ value = 'b', onUpdate }: { value?: string; onUpdate(value: string): Promise<unknown> }) {
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const hotkeys = useHotkeyConfigStore((state) => state.snapshot);
  const globalKey = (binding: string) => normalizePinBorderShortcut(binding.replace(/⌘/g, /win/i.test(navigator.platform) ? 'ctrl+' : 'meta+'));
  const save = (key: string | null) => {
    if (key === null) { setError('请使用字母、数字或功能键，可组合修饰键。'); return; }
    const conflicts = hotkeys && Object.values(hotkeys).flatMap((category) => Object.values(category));
    if (key && conflicts?.some((binding) => globalKey(binding) === key)) { setError('与全局快捷键重复，请先修改对应的全局快捷键。'); return; }
    setError(null); setRecording(false);
    void onUpdate(key).catch(() => setError('快捷键保存失败，请重试。'));
  };
  useNativeShortcutRecording(recording ? 'pin-border' : null, (hotkey) => save(globalKey(hotkey)), setError);
  return (
    <SettingRow label="贴图边框快捷键" description="在贴图窗口内切换所有贴图的边框。Delete 清除。">
      <div className="flex max-w-[270px] flex-col items-end gap-2">
        <input readOnly aria-label="贴图边框快捷键" value={recording ? '' : value.toUpperCase()} placeholder={recording ? '按下快捷键' : '未设置'}
          className={`h-9 w-[180px] rounded-lg border bg-surface px-3 text-center font-mono text-xs text-accent-text outline-none ${recording ? 'border-accent ring-2 ring-accent-line' : 'border-line'}`}
          onFocus={() => setRecording(true)} onBlur={() => setRecording(false)}
          onKeyDown={(event) => {
            if (event.key === 'Tab') return;
            event.preventDefault();
            event.stopPropagation();
            if (event.key === 'Escape') { setRecording(false); return; }
            if (['Control', 'Meta', 'Alt', 'Shift'].includes(event.key)) return;
            const key = ['Delete', 'Backspace'].includes(event.key) ? '' : pinBorderShortcutFromEvent(event);
            save(key);
          }} />
        {error && <p role="alert" className="text-xs text-bad">{error}</p>}
      </div>
    </SettingRow>
  );
}
