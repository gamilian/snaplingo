import { useEffect, useState } from 'react';
import { normalizePinBorderShortcut, pinBorderShortcutFromEvent } from '../../../application/pinned-image/borderShortcut';
import { useHotkeyConfigStore } from '../../../stores/hotkeyConfigStore';
import { HotkeyRow } from '../Hotkey/HotkeyRow';
import { useNativeShortcutRecording } from './useNativeShortcutRecording';

export function PinBorderShortcutSettings({ value = 'b', onUpdate }: { value?: string; onUpdate(value: string): Promise<unknown> }) {
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const hotkeys = useHotkeyConfigStore((state) => state.snapshot);
  const isWindows = /win/i.test(navigator.platform);
  const globalKey = (binding: string) => normalizePinBorderShortcut(binding.replace(/⌘/g, isWindows ? 'ctrl+' : 'meta+'));
  const save = (key: string | null) => {
    if (key === null) { setError('请使用字母、数字或功能键，可组合修饰键。'); return; }
    const conflicts = hotkeys && Object.values(hotkeys).flatMap((category) => Object.values(category));
    if (key && conflicts?.some((binding) => globalKey(binding) === key)) { setError('与全局快捷键重复，请先修改对应的全局快捷键。'); return; }
    setError(null);
    setRecording(false);
    void onUpdate(key).catch(() => setError('快捷键保存失败，请重试。'));
  };
  useNativeShortcutRecording(recording ? 'pin-border' : null, (hotkey) => save(globalKey(hotkey)), setError);
  useEffect(() => {
    if (!recording) return;
    const cancel = () => setRecording(false);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Tab') { cancel(); return; }
      event.preventDefault();
      event.stopPropagation();
      if (event.key === 'Escape') { cancel(); return; }
      if (['Control', 'Meta', 'Alt', 'Shift'].includes(event.key)) return;
      save(['Delete', 'Backspace'].includes(event.key) ? '' : pinBorderShortcutFromEvent(event));
    };
    const onOutsideClick = (event: MouseEvent) => {
      if (!(event.target as HTMLElement).closest('[data-pin-border-recorder]')) cancel();
    };
    window.addEventListener('keydown', onKeyDown, true);
    window.addEventListener('blur', cancel);
    document.addEventListener('mousedown', onOutsideClick);
    return () => {
      window.removeEventListener('keydown', onKeyDown, true);
      window.removeEventListener('blur', cancel);
      document.removeEventListener('mousedown', onOutsideClick);
    };
  }, [recording, value, hotkeys, onUpdate]);

  const display = value.split('+').map((key) => ({ shift: '⇧', alt: '⌥', ctrl: isWindows ? '⌘' : '⌃', meta: isWindows ? '⊞' : '⌘' })[key as 'shift' | 'alt' | 'ctrl' | 'meta'] ?? key.toUpperCase()).join('');
  return <div data-pin-border-recorder>
    <HotkeyRow label="贴图边框切换" description="仅在贴图窗口聚焦时生效" value={display || '未设置'} defaultValue="B" isRecording={recording} onRecord={() => { setError(null); setRecording(!recording); }} onClear={() => save('')} onReset={() => save('b')} />
    {error && <p role="alert" className="pb-3 text-xs text-danger">{error}</p>}
  </div>;
}
