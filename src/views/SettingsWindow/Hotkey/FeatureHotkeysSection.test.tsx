// @vitest-environment happy-dom

import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useHotkeyConfigStore } from '../../../stores/hotkeyConfigStore';
import { FeatureHotkeysSection } from './FeatureHotkeysSection';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const initialState = useHotkeyConfigStore.getState();
const mounted: Array<{ root: Root; container: HTMLDivElement }> = [];

afterEach(async () => {
  for (const { root, container } of mounted.splice(0)) {
    await act(async () => root.unmount());
    container.remove();
  }
  useHotkeyConfigStore.setState(initialState, true);
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

async function createRecorder(value = 'F1') {
  const snapshot = {
    screenshot: { screenshot: value },
    translation: {},
    ocr: {},
  };
  let nativeSession: string | null = null;
  let nativeListener: ((event: { recordingId: string; hotkey: string }) => void) | null = null;
  const nativeAction = vi.fn();
  const updateHotkey = vi.fn(async (_category: string, _action: string, hotkey: string) => {
    const updated = { ...snapshot, screenshot: { screenshot: hotkey } };
    useHotkeyConfigStore.setState({ snapshot: updated });
    return updated;
  });
  const beginRecording = vi.fn(async (id: string) => { nativeSession = id; });
  const endRecording = vi.fn(async (id: string) => {
    if (nativeSession === id) nativeSession = null;
  });
  const unsubscribe = vi.fn(() => { nativeListener = null; });
  const subscribeRecordedHotkey = vi.fn(async (handler: NonNullable<typeof nativeListener>) => {
    nativeListener = handler;
    return unsubscribe;
  });
  const state = {
    snapshot,
    defaultSnapshot: snapshot,
    updateHotkey,
    beginRecording,
    endRecording,
    subscribeRecordedHotkey,
  };
  useHotkeyConfigStore.setState(state);
  vi.stubGlobal('alert', vi.fn());

  const container = document.createElement('div');
  document.body.append(container);
  const root = createRoot(container);
  mounted.push({ root, container });
  await act(async () => {
    root.render(
      <FeatureHotkeysSection
        category="screenshot"
        actions={[{ key: 'screenshot', label: '截图' }]}
      />,
    );
  });

  return {
    container, root, updateHotkey, beginRecording, endRecording,
    subscribeRecordedHotkey, unsubscribe, nativeAction,
    start: () => act(async () => container.querySelector('button')!.click()),
    releaseRegisteredHotkey(hotkey: string) {
      // Registered global shortcuts are consumed before a DOM keydown exists.
      if (nativeSession && nativeListener) {
        nativeListener({ recordingId: nativeSession, hotkey });
      } else {
        nativeAction(hotkey);
      }
    },
  };
}

async function pressKey(key: string, code = key, modifiers: KeyboardEventInit = {}) {
  await act(async () => {
    window.dispatchEvent(new KeyboardEvent('keydown', {
      key, code, ...modifiers, bubbles: true, cancelable: true,
    }));
  });
}

async function releaseKey(key: string, code = key, modifiers: KeyboardEventInit = {}) {
  await act(async () => {
    window.dispatchEvent(new KeyboardEvent('keyup', {
      key, code, ...modifiers, bubbles: true, cancelable: true,
    }));
  });
}

describe('FeatureHotkeysSection keyboard recording', () => {
  it('records on older WebViews without crypto.randomUUID', async () => {
    const getRandomValues = crypto.getRandomValues.bind(crypto);
    vi.stubGlobal('crypto', { getRandomValues });
    const recorder = await createRecorder();
    await recorder.start();
    await act(async () => recorder.releaseRegisteredHotkey('F1'));
    expect(recorder.updateHotkey).toHaveBeenCalledExactlyOnceWith('screenshot', 'screenshot', 'F1');
  });

  it.each(['F1', 'F3', '⌘F1'])(
    'records an already registered %s from the native key channel without triggering its action',
    async (hotkey) => {
      const recorder = await createRecorder();
      await recorder.start();
      await act(async () => recorder.releaseRegisteredHotkey(hotkey));

      expect(recorder.updateHotkey).toHaveBeenCalledExactlyOnceWith(
        'screenshot', 'screenshot', hotkey,
      );
      expect(recorder.nativeAction).not.toHaveBeenCalled();
      expect(recorder.endRecording).toHaveBeenCalled();
      expect(recorder.container.querySelector('select')).toBeNull();
    },
  );

  it.each(['F1', 'F2', 'F3', 'F12', 'F20'])(
    'records a normal %s key only after release',
    async (key) => {
      const recorder = await createRecorder('⇧⌘R');
      await recorder.start();
      await pressKey(key);
      expect(recorder.updateHotkey).not.toHaveBeenCalled();
      await releaseKey(key);
      expect(recorder.updateHotkey).toHaveBeenCalledExactlyOnceWith(
        'screenshot', 'screenshot', key,
      );
    },
  );

  it('keeps recording Cmd+Shift+R', async () => {
    const recorder = await createRecorder();
    await recorder.start();
    await pressKey('R', 'KeyR', { metaKey: true, shiftKey: true });
    await releaseKey('R', 'KeyR', { metaKey: true, shiftKey: true });
    expect(recorder.updateHotkey).toHaveBeenCalledExactlyOnceWith(
      'screenshot', 'screenshot', '⇧⌘R',
    );
  });

  it.each(['', 'Unidentified'])('recognizes a function key when its code is %j', async (code) => {
    const recorder = await createRecorder();
    await recorder.start();
    await pressKey('F1', code);
    await releaseKey('F1', code);
    expect(recorder.updateHotkey).toHaveBeenCalledExactlyOnceWith('screenshot', 'screenshot', 'F1');
  });

  it('ignores auto-repeat and keeps native actions suppressed until saving finishes', async () => {
    const recorder = await createRecorder();
    await recorder.start();
    let finishSaving!: () => void;
    const savePending = new Promise<void>((resolve) => { finishSaving = resolve; });
    const save = recorder.updateHotkey.getMockImplementation()!;
    recorder.updateHotkey.mockImplementationOnce(async (...args) => {
      await savePending;
      return save(...args);
    });
    await pressKey('F2');
    await pressKey('F2', 'F2', { repeat: true });
    await releaseKey('F2');
    await act(async () => recorder.releaseRegisteredHotkey('F1'));
    expect(recorder.nativeAction).not.toHaveBeenCalled();
    expect(recorder.updateHotkey).toHaveBeenCalledOnce();
    expect(recorder.endRecording).not.toHaveBeenCalled();
    await act(async () => finishSaving());
    expect(recorder.endRecording).toHaveBeenCalled();
  });

  it('ignores an old native event after a new recording starts', async () => {
    const recorder = await createRecorder();
    await recorder.start();
    const oldListener = recorder.subscribeRecordedHotkey.mock.calls[0][0];
    const oldId = recorder.beginRecording.mock.calls[0][0];
    await pressKey('Escape');
    await recorder.start();
    await act(async () => oldListener({ recordingId: oldId, hotkey: 'F1' }));
    expect(recorder.updateHotkey).not.toHaveBeenCalled();
    await act(async () => recorder.releaseRegisteredHotkey('F1'));
    expect(recorder.updateHotkey).toHaveBeenCalledExactlyOnceWith('screenshot', 'screenshot', 'F1');
  });

  it.each(['Escape', 'outside-click', 'blur'])(
    'ends recording on %s and lets subsequent Cmd+C pass through',
    async (reason) => {
      const recorder = await createRecorder();
      await recorder.start();
      await act(async () => {
        if (reason === 'outside-click') {
          document.body.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
        } else if (reason === 'blur') {
          window.dispatchEvent(new Event('blur'));
        } else {
          window.dispatchEvent(new KeyboardEvent('keydown', {
            key: 'Escape', code: 'Escape', bubbles: true, cancelable: true,
          }));
        }
      });
      expect(recorder.endRecording).toHaveBeenCalled();
      expect(recorder.unsubscribe).toHaveBeenCalled();
      const copy = new KeyboardEvent('keydown', {
        key: 'c', code: 'KeyC', metaKey: true, bubbles: true, cancelable: true,
      });
      window.dispatchEvent(copy);
      expect(copy.defaultPrevented).toBe(false);
      expect(recorder.updateHotkey).not.toHaveBeenCalled();
      recorder.releaseRegisteredHotkey('F1');
      expect(recorder.nativeAction).toHaveBeenCalledWith('F1');
    },
  );

  it('cleans up a native recording that starts after the view unmounts', async () => {
    const recorder = await createRecorder();
    let finishStarting!: () => void;
    recorder.beginRecording.mockImplementationOnce(() => new Promise<void>((resolve) => {
      finishStarting = resolve;
    }));
    await recorder.start();
    expect(recorder.beginRecording).toHaveBeenCalledOnce();
    await act(async () => recorder.root.unmount());
    await act(async () => finishStarting());
    expect(recorder.endRecording).toHaveBeenCalledWith(recorder.beginRecording.mock.calls[0][0]);
    expect(recorder.unsubscribe).toHaveBeenCalled();
    expect(recorder.updateHotkey).not.toHaveBeenCalled();
  });

  it('reports a native startup failure and releases local key listeners', async () => {
    const recorder = await createRecorder();
    recorder.beginRecording.mockRejectedValueOnce(new Error('window is not focused'));
    await recorder.start();
    expect(alert).toHaveBeenCalledWith(expect.stringContaining('window is not focused'));
    expect(recorder.unsubscribe).toHaveBeenCalled();
    const copy = new KeyboardEvent('keydown', {
      key: 'c', code: 'KeyC', metaKey: true, cancelable: true,
    });
    window.dispatchEvent(copy);
    expect(copy.defaultPrevented).toBe(false);
  });

  it('reports registration failures and preserves the original hotkey', async () => {
    const recorder = await createRecorder('⇧⌘R');
    await recorder.start();
    recorder.updateHotkey.mockRejectedValueOnce(new Error('already registered'));
    await pressKey('F1');
    await releaseKey('F1');
    expect(alert).toHaveBeenCalledWith('快捷键 F1 注册失败：already registered');
    expect(useHotkeyConfigStore.getState().snapshot!.screenshot.screenshot).toBe('⇧⌘R');
    expect(recorder.endRecording).toHaveBeenCalled();
  });
});
