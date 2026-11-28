import { useState } from 'react';
import { AnnotationToolIcon } from '../../../components/common/AnnotationToolIcon';
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
  const [replacement, setReplacement] = useState<{ next: Record<string, string>; tool: string; conflict: string } | null>(null);
  const shortcuts: Record<string, string> = {
    ...DEFAULT_ANNOTATION_SHORTCUTS,
    ...value,
  };
  const save = (next: Record<string, string>, editedTool?: string) => {
    setReplacement(null);
    const conflict = annotationShortcutConflict(next);
    if (conflict) {
      if (conflict.kind === 'duplicate' && editedTool) {
        setReplacement({ next, tool: editedTool, conflict: conflict.tool === editedTool ? conflict.conflict : conflict.tool });
      }
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
    <div className="editor-shortcut-settings">
      <div className="flex items-center justify-between gap-3 px-4 pt-3">
        <p className="text-hint text-muted">字母、数字或 F1–F24 · Delete 清除 · Esc 取消</p>
        <button type="button" className="settings-action" onClick={() => save({ ...DEFAULT_ANNOTATION_SHORTCUTS })}>恢复标注快捷键默认值</button>
      </div>
      <div className="keytile-grid">
        {Object.entries(LABELS).map(([tool, label]) => (
          <div key={tool} className={`keytile ${recording === tool ? 'recording' : ''} ${replacement && (replacement.tool === tool || replacement.conflict === tool) ? 'conflict' : ''}`}>
            <label className="keytile-label">
              <span className="keytile-icon"><AnnotationToolIcon tool={tool} /></span>
              <span className="keytile-name">{label}</span>
              <input
                className="keytile-cap"
                value={recording === tool ? '' : (shortcuts[tool] ?? '').toUpperCase()}
                placeholder={recording === tool ? '…' : '—'}
                aria-label={`${label}快捷键`}
                readOnly
                onFocus={() => { setError(null); setRecording(tool); }}
                onBlur={() => setRecording(null)}
                onKeyDown={(event) => {
                  if (event.key === 'Tab') return;
                  event.preventDefault();
                  if (event.key === 'Escape') { setError(null); setReplacement(null); setRecording(null); return; }
                  if (event.key === 'Delete' || event.key === 'Backspace') { save({ ...shortcuts, [tool]: '' }, tool); return; }
                  if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) { setError('工具切换仅使用主键，请松开修饰键。'); return; }
                  save({ ...shortcuts, [tool]: normalizeEditorShortcutKey(event) }, tool);
                }}
              />
            </label>
            {shortcuts[tool] !== DEFAULT_ANNOTATION_SHORTCUTS[tool as keyof typeof DEFAULT_ANNOTATION_SHORTCUTS] && <button type="button" className="keytile-reset" aria-label={`恢复${label}默认快捷键`} onClick={() => save({ ...shortcuts, [tool]: DEFAULT_ANNOTATION_SHORTCUTS[tool as keyof typeof DEFAULT_ANNOTATION_SHORTCUTS] }, tool)}>↺</button>}
          </div>
        ))}
      </div>
      {error && <div className="keytile-hint flex items-center justify-between gap-3" role="alert"><span>{error}</span>{replacement && <button type="button" className="settings-action" onClick={() => save({ ...replacement.next, [replacement.conflict]: '' }, replacement.tool)}>替换「{LABELS[replacement.conflict]}」绑定</button>}</div>}
      <p className="keytile-hint">F3 用于贴图。已被全局快捷键占用的按键需先修改对应绑定；重复键位会提示冲突。</p>
    </div>
  );
}
