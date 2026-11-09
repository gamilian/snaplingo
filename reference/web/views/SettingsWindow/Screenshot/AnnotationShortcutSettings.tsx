import { useState } from 'react';
import { normalizePinBorderShortcut } from '../../../application/pinned-image/borderShortcut';
import { useNativeShortcutRecording } from './useNativeShortcutRecording';
import {
  annotationShortcutConflict,
  DEFAULT_ANNOTATION_SHORTCUTS,
  normalizeEditorShortcutKey,
} from '../../../application/capture-workspace/annotationShortcuts';

const LABELS: Record<string, string> = {
  rectangle: '矩形',
  ellipse: '椭圆',
  arrow: '箭头',
  line: '直线',
  pen: '画笔',
  highlight: '荧光笔',
  'region-highlight': '区域高亮',
  blur: '模糊',
  spotlight: '聚光灯',
  mosaic: '马赛克',
  text: '文字',
  eraser: '橡皮擦',
  scrolling: '长截图',
};

export function AnnotationShortcutSettings({
  value = {},
  onUpdate,
}: {
  value?: Record<string, string>;
  onUpdate(value: Record<string, string>): Promise<unknown>;
}) {
  const [recording, setRecording] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const shortcuts: Record<string, string> = {
    ...DEFAULT_ANNOTATION_SHORTCUTS,
    ...value,
  };
  const save = (next: Record<string, string>, editedTool?: string) => {
    const conflict = annotationShortcutConflict(next);
    if (conflict) {
      setError(
        conflict.kind === 'duplicate'
          ? `与“${LABELS[conflict.conflict === editedTool ? conflict.tool : conflict.conflict]}”快捷键重复。`
          : conflict.kind === 'reserved'
            ? '该键用于固定编辑操作，请换一个按键。'
            : '请使用字母、数字或 F1–F24。',
      );
      return;
    }
    setError(null);
    setRecording(null);
    void onUpdate(next).catch(() => setError('快捷键保存失败，请重试。'));
  };
  useNativeShortcutRecording(recording, (hotkey) => {
    if (!recording) return;
    const key = normalizePinBorderShortcut(hotkey);
    if (key === null || key.includes('+')) { setError('工具切换仅使用主键，请松开修饰键。'); return; }
    setError('该键已被全局快捷键占用，请先修改对应的全局绑定。');
  }, setError);
  return (
    <div className="px-6 py-4">
      <p className="mb-4 text-xs text-muted">
        标注快捷键仅在截图编辑器中生效。可设置字母、数字或功能键，Delete
        可清除。F3 用于贴图；绑定已用作全局快捷键的按键前，请先修改全局绑定。
      </p>
      <div className="grid grid-cols-1 gap-x-10 gap-y-3 md:grid-cols-2">
        {Object.entries(LABELS).map(([tool, label]) => (
          <label
            key={tool}
            className="flex items-center justify-between gap-3 text-xs text-ink"
          >
            {label}
            <input
              className={`h-8 w-[112px] rounded border bg-surface px-3 text-center font-mono text-accent outline-none ${recording === tool ? 'border-accent' : 'border-line'}`}
              value={
                recording === tool ? '' : (shortcuts[tool] ?? '').toUpperCase()
              }
                  placeholder={recording === tool ? '按下按键' : '未设置'}
              aria-label={`${label}快捷键`}
              readOnly
              onFocus={() => setRecording(tool)}
              onBlur={() => setRecording(null)}
              onKeyDown={(event) => {
                if (event.key === 'Tab') return;
                event.preventDefault();
                if (event.key === 'Escape') {
                  setRecording(null);
                  return;
                }
                if (event.key === 'Delete' || event.key === 'Backspace') {
                  save({ ...shortcuts, [tool]: '' }, tool);
                  return;
                }
                if (event.ctrlKey || event.metaKey || event.altKey) {
                  setError('工具切换仅使用主键，请松开修饰键。');
                  return;
                }
                const key = normalizeEditorShortcutKey(event);
                save({ ...shortcuts, [tool]: key }, tool);
              }}
            />
          </label>
        ))}
      </div>
      {error && (
        <p role="alert" className="mt-3 text-xs text-bad">
          {error}
        </p>
      )}
      <button
        type="button"
        className="mt-4 h-8 rounded border border-line bg-surface px-3 text-xs text-muted hover:bg-raised"
        onClick={() => save({ ...DEFAULT_ANNOTATION_SHORTCUTS })}
      >
        恢复标注快捷键默认值
      </button>
    </div>
  );
}
