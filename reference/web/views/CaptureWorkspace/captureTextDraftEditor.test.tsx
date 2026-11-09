// @vitest-environment happy-dom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { CaptureTextDraftEditor } from './capturePreviewPresentation';
import { DEFAULT_ANNOTATION_STYLE } from '../../application/capture-workspace/annotationStyle';

it('commits Enter, inserts modifier Enter at the caret, and protects IME composition', async () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  const container = document.createElement('div'); document.body.append(container);
  const root = createRoot(container);
  const onCommit = vi.fn(), onDiscard = vi.fn(), onTextChange = vi.fn();
  try {
    await act(async () => root.render(createElement(CaptureTextDraftEditor, {
      inputRef: { current: null }, textDraft: { position: { x: 10, y: 30 }, fontSize: 24, text: 'abc' },
      selectionViewportRect: { x: 0, y: 0, width: 200, height: 100 }, annotationStyle: DEFAULT_ANNOTATION_STYLE,
      onCommit, onDiscard, onTextChange,
    })));
    const input = container.querySelector('textarea')!;
    input.focus(); input.setSelectionRange(1, 2);
    await act(async () => input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true, bubbles: true, cancelable: true })));
    expect(onTextChange).toHaveBeenCalledWith('a\nc');
    expect(onCommit).not.toHaveBeenCalled();
    await act(async () => input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', isComposing: true, bubbles: true, cancelable: true })));
    expect(onCommit).not.toHaveBeenCalled();
    await act(async () => input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })));
    expect(onCommit).toHaveBeenCalledOnce();
    input.focus();
    await act(async () => input.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, bubbles: true, cancelable: true })));
    expect(onDiscard).toHaveBeenCalledOnce();
  } finally { await act(async () => root.unmount()); container.remove(); }
});

it('retains a text draft and its selected font when focus moves to native style controls', async () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(false);
  const container = document.createElement('div'); document.body.append(container);
  const root = createRoot(container), onCommit = vi.fn();
  try {
    await act(async () => root.render(createElement(CaptureTextDraftEditor, {
      preserveNativeFocus: true, inputRef: { current: null },
      textDraft: { position: { x: 10, y: 30 }, fontSize: 24, text: '原生样式' },
      selectionViewportRect: { x: 0, y: 0, width: 200, height: 100 },
      annotationStyle: { ...DEFAULT_ANNOTATION_STYLE, fontFamily: '.Al Bayan PUA' },
      onCommit, onDiscard: vi.fn(), onTextChange: vi.fn(),
    })));
    const input = container.querySelector('textarea')!;
    expect(input.style.fontFamily).toBe('".Al Bayan PUA"');
    await act(async () => input.dispatchEvent(new FocusEvent('focusout', { bubbles: true })));
    expect(onCommit).not.toHaveBeenCalled();
    focus.mockReturnValue(true);
    await act(async () => input.dispatchEvent(new FocusEvent('focusout', { bubbles: true })));
    expect(onCommit).toHaveBeenCalledOnce();
  } finally { await act(async () => root.unmount()); container.remove(); focus.mockRestore(); }
});
