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

function isWindowsPlatform() {
  return typeof navigator !== 'undefined' && navigator.platform.startsWith('Win');
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
  const beginRecording = useHotkeyConfigStore((state) => state.beginRecording);
  const endRecording = useHotkeyConfigStore((state) => state.endRecording);
  const subscribeRecordedHotkey = useHotkeyConfigStore((state) => state.subscribeRecordedHotkey);
  const [recordingKey, setRecordingKey] = useState<string | null>(null);

  useEffect(() => {
    if (!recordingKey) return;
    const recordingId = Array.from(
      crypto.getRandomValues(new Uint32Array(4)),
      (value) => value.toString(16),
    ).join('-');
    let disposed = false;
    let saving = false;
    let unsubscribe: (() => void) | undefined;
    let pendingKey: { code: string; hotkey: string } | null = null;
    const cancel = () => setRecordingKey((current) => current === recordingKey ? null : current);

    const finish = (hotkey: string) => {
      if (disposed || saving) return;
      saving = true;
      void (async () => {
        try {
          await starting;
          if (disposed) return;
          await saveHotkeyWithRegistration({
            category,
            action: recordingKey,
            hotkey,
            updateHotkey,
            reportError: alert,
          });
          await endRecording(recordingId);
        } catch (error) {
          if (!disposed) reportMutationError('录制快捷键', error);
        } finally {
          if (!disposed) cancel();
        }
      })();
    };

    const starting = (async () => {
      const unlisten = await subscribeRecordedHotkey((event) => {
        if (event.recordingId === recordingId) finish(event.hotkey);
      });
      if (disposed) {
        unlisten();
        return;
      }
      unsubscribe = unlisten;
      await beginRecording(recordingId);
    })();
    void starting.catch((error) => {
      if (!disposed) {
        reportMutationError('开始录制快捷键', error);
        cancel();
      }
    });

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === 'Escape') {
        cancel();
        return;
      }
      if (event.repeat || saving) return;

      const modifiers: string[] = [];
      if (event.shiftKey) modifiers.push('⇧');
      if (event.altKey) modifiers.push('⌥');
      if (isWindowsPlatform()) {
        if (event.ctrlKey) modifiers.push('⌘');
      } else {
        if (event.metaKey) modifiers.push('⌘');
        if (event.ctrlKey) modifiers.push('⌃');
      }

      let mainKey = '';
      if (event.code.startsWith('Key')) mainKey = event.code.slice(3);
      else if (event.code.startsWith('Digit')) mainKey = event.code.slice(5);
      else if (/^F\d+$/.test(event.code)) mainKey = event.code;
      else if (/^F\d+$/.test(event.key)) mainKey = event.key;
      if (!mainKey) return;

      pendingKey = { code: event.code || event.key, hotkey: modifiers.join('') + mainKey };
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      if (!pendingKey || (event.code || event.key) !== pendingKey.code) return;
      event.preventDefault();
      event.stopPropagation();
      finish(pendingKey.hotkey);
      pendingKey = null;
    };

    const cancelOnOutsideClick = (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      if (!target.closest('[data-hotkey-recorder]')) {
        cancel();
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    window.addEventListener('keyup', handleKeyUp, true);
    window.addEventListener('blur', cancel);
    document.addEventListener('mousedown', cancelOnOutsideClick, true);
    return () => {
      disposed = true;
      window.removeEventListener('keydown', handleKeyDown, true);
      window.removeEventListener('keyup', handleKeyUp, true);
      window.removeEventListener('blur', cancel);
      document.removeEventListener('mousedown', cancelOnOutsideClick, true);
      unsubscribe?.();
      void starting.then(() => endRecording(recordingId), () => undefined).catch((error) => {
        console.warn('Failed to finish hotkey recording:', error);
      });
    };
  }, [category, recordingKey, updateHotkey, beginRecording, endRecording, subscribeRecordedHotkey]);

  const hotkeys = snapshot?.[category];
  const defaults = defaultSnapshot?.[category] ?? hotkeys;
  if (!hotkeys || !defaults) {
    return <div className="px-6 py-8 text-sm text-gray-500">正在加载快捷键配置...</div>;
  }

  return (
    <div className="divide-y divide-gray-100 px-[22px] pb-2">
      {actions.map((action) => (
        <div
          key={action.key}
          data-hotkey-recorder={recordingKey === action.key ? '' : undefined}
        >
          <HotkeyRow
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
        </div>
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
