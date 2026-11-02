// @vitest-environment happy-dom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { AnnotationShortcutSettings } from './AnnotationShortcutSettings';
import { useHotkeyConfigStore } from '../../../stores/hotkeyConfigStore';

(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;
const mounts: {
  root: ReturnType<typeof createRoot>;
  container: HTMLDivElement;
}[] = [];
const initialHotkeys = useHotkeyConfigStore.getState();

function mount() {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  const onUpdate = vi.fn(async (_value: Record<string, string>) => undefined);
  act(() =>
    root.render(createElement(AnnotationShortcutSettings, { onUpdate })),
  );
  mounts.push({ root, container });
  const input = (label: string) =>
    container.querySelector<HTMLInputElement>(`[aria-label="${label}快捷键"]`)!;
  return { container, onUpdate, input };
}

afterEach(() => {
  for (const { root, container } of mounts.splice(0)) {
    act(() => root.unmount());
    container.remove();
  }
  useHotkeyConfigStore.setState(initialHotkeys, true);
});

describe('editor shortcut settings', () => {
  it('reports an OS-registered tool key through the native recording channel', async () => {
    let recorded!: (event: { recordingId: string; hotkey: string }) => void;
    const beginRecording = vi.fn(async (_id: string) => {});
    const endRecording = vi.fn(async (_id: string) => {});
    useHotkeyConfigStore.setState({
      hydrated: true,
      beginRecording,
      endRecording,
      subscribeRecordedHotkey: async (handler) => { recorded = handler; return () => {}; },
    });
    const { input, onUpdate, container } = mount();
    await act(async () => input('箭头').focus());
    expect(beginRecording).toHaveBeenCalledOnce();
    await act(async () => recorded({ recordingId: 'stale', hotkey: 'F1' }));
    expect(container.querySelector('[role="alert"]')).toBeNull();
    await act(async () => recorded({ recordingId: beginRecording.mock.calls[0][0], hotkey: 'F1' }));
    expect(container.querySelector('[role="alert"]')?.textContent).toContain('全局快捷键占用');
    expect(onUpdate).not.toHaveBeenCalled();
    await act(async () => input('箭头').blur());
    expect(endRecording).toHaveBeenCalledWith(beginRecording.mock.calls[0][0]);
  });

  it('records a physical letter key when an IME supplies Process as the key value', () => {
    const { input, onUpdate } = mount();
    act(() => {
      input('箭头').focus();
      input('箭头').dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'Process',
          code: 'KeyX',
          isComposing: true,
          bubbles: true,
          cancelable: true,
        }),
      );
    });
    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ arrow: 'x', scrolling: 's' }),
    );
  });

  it('reports duplicate or modified keys before attempting to save', () => {
    const { input, onUpdate, container } = mount();
    act(() => {
      input('箭头').focus();
      input('箭头').dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'o',
          code: 'KeyO',
          bubbles: true,
          cancelable: true,
        }),
      );
    });
    expect(container.querySelector('[role="alert"]')?.textContent).toContain(
      '重复',
    );
    act(() =>
      input('箭头').dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 's',
          code: 'KeyS',
          bubbles: true,
          cancelable: true,
        }),
      ),
    );
    expect(container.querySelector('[role="alert"]')?.textContent).toContain(
      '长截图',
    );
    act(() =>
      input('箭头').dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'x',
          code: 'KeyX',
          ctrlKey: true,
          bubbles: true,
          cancelable: true,
        }),
      ),
    );
    expect(container.querySelector('[role="alert"]')?.textContent).toContain(
      '修饰键',
    );
    expect(onUpdate).not.toHaveBeenCalled();
  });

  it('clears the scrolling shortcut and restores the defaults', () => {
    const { input, onUpdate, container } = mount();
    act(() => {
      input('长截图').focus();
      input('长截图').dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'Delete',
          bubbles: true,
          cancelable: true,
        }),
      );
    });
    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ scrolling: '' }),
    );
    act(() => container.querySelector<HTMLButtonElement>('button')!.click());
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({ arrow: 'a', scrolling: 's' }),
    );
  });
});
