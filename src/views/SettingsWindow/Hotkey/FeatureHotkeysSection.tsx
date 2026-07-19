import { useEffect, useState } from 'react';
import type { HotkeyCategory } from '../../../application/settings/ports';
import { useHotkeyConfigStore } from '../../../stores/hotkeyConfigStore';
import { HotkeyRow } from './HotkeyRow';
import { saveHotkeyWithRegistration } from './hotkeyRegistration';

export interface FeatureHotkeyAction {
  key: string;
  label: string;
}

function reportMutationError(action: string, error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  alert(`${action}失败：${message}`);
}

export function FeatureHotkeysSection({
  category,
  actions,
}: {
  category: HotkeyCategory;
  actions: FeatureHotkeyAction[];
}) {
  const snapshot = useHotkeyConfigStore((state) => state.snapshot);
  const defaultSnapshot = useHotkeyConfigStore((state) => state.defaultSnapshot);
  const updateHotkey = useHotkeyConfigStore((state) => state.updateHotkey);
  const resetHotkey = useHotkeyConfigStore((state) => state.resetHotkey);
  const [recordingKey, setRecordingKey] = useState<string | null>(null);

  useEffect(() => {
    if (!recordingKey) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === 'Escape') {
        setRecordingKey(null);
        return;
      }

      const modifiers = [];
      if (event.shiftKey) modifiers.push('⇧');
      if (event.altKey) modifiers.push('⌥');
      if (event.metaKey) modifiers.push('⌘');
      if (event.ctrlKey) modifiers.push('⌃');

      let mainKey = '';
      if (event.code.startsWith('Key')) mainKey = event.code.slice(3);
      else if (event.code.startsWith('Digit')) mainKey = event.code.slice(5);
      else if (/^F\d+$/.test(event.code)) mainKey = event.code;
      if (!mainKey) return;

      void saveHotkeyWithRegistration({
        category,
        action: recordingKey,
        hotkey: modifiers.join('') + mainKey,
        updateHotkey,
        reportError: alert,
      }).then(() => setRecordingKey(null));
    };

    const cancelOnOutsideClick = (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      if (!target.closest('button') && !target.closest('.hotkey-display')) {
        setRecordingKey(null);
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    document.addEventListener('mousedown', cancelOnOutsideClick, true);
    return () => {
      window.removeEventListener('keydown', handleKeyDown, true);
      document.removeEventListener('mousedown', cancelOnOutsideClick, true);
    };
  }, [category, recordingKey, updateHotkey]);

  const hotkeys = snapshot?.[category];
  const defaults = defaultSnapshot?.[category] ?? hotkeys;
  if (!hotkeys || !defaults) {
    return <div className="px-6 py-8 text-sm text-gray-500">正在加载快捷键配置...</div>;
  }

  return (
    <div className="divide-y divide-gray-100 px-[22px] pb-2">
      {actions.map((action) => (
        <HotkeyRow
          key={action.key}
          label={action.label}
          value={hotkeys[action.key] ?? '未设置'}
          defaultValue={defaults[action.key] ?? '未设置'}
          isRecording={recordingKey === action.key}
          onRecord={() =>
            setRecordingKey((current) =>
              current === action.key ? null : action.key,
            )
          }
          onClear={() => {
            void updateHotkey(category, action.key, '未设置').catch((error) =>
              reportMutationError('清除快捷键', error),
            );
          }}
          onReset={() => {
            void resetHotkey(category, action.key).catch((error) =>
              reportMutationError('恢复快捷键', error),
            );
          }}
        />
      ))}
    </div>
  );
}

export function HotkeyToolbar({
  category,
  actions,
}: {
  category: HotkeyCategory;
  actions: FeatureHotkeyAction[];
}) {
  const snapshot = useHotkeyConfigStore((state) => state.snapshot);
  const resetCategory = useHotkeyConfigStore((state) => state.resetCategory);
  const hotkeys = snapshot?.[category];

  const detectConflicts = () => {
    if (!hotkeys) return;
    const configured = actions
      .map((action) => hotkeys[action.key])
      .filter((hotkey) => hotkey && hotkey !== '未设置');
    const conflicts = configured.filter(
      (hotkey, index) => configured.indexOf(hotkey) !== index,
    );
    alert(
      conflicts.length > 0
        ? `发现冲突的快捷键：${[...new Set(conflicts)].join(', ')}`
        : '未发现冲突的快捷键',
    );
  };

  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={() => {
          if (confirm('确定要恢复这一组快捷键到默认值吗？')) {
            void resetCategory(category).catch((error) =>
              reportMutationError('恢复快捷键组', error),
            );
          }
        }}
        className="h-8 rounded-lg px-3 text-[11px] font-medium text-gray-500 hover:bg-white hover:text-gray-800"
      >
        恢复默认值
      </button>
      <button
        type="button"
        onClick={detectConflicts}
        className="h-8 rounded-lg border border-gray-200 bg-white px-3 text-[11px] font-semibold text-gray-600 hover:border-primary-200 hover:text-primary-600"
      >
        检测冲突
      </button>
    </div>
  );
}
