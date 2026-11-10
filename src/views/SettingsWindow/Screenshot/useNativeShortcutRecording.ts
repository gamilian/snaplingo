import { useEffect, useRef } from 'react';
import { useHotkeyConfigStore } from '../../../stores/hotkeyConfigStore';

// Registered OS shortcuts do not reach the focused input's DOM key events.
export function useNativeShortcutRecording(recording: string | null, onRecorded: (hotkey: string) => void, onError: (message: string) => void) {
  const hydrated = useHotkeyConfigStore((state) => state.hydrated);
  const begin = useHotkeyConfigStore((state) => state.beginRecording);
  const end = useHotkeyConfigStore((state) => state.endRecording);
  const subscribe = useHotkeyConfigStore((state) => state.subscribeRecordedHotkey);
  const handler = useRef(onRecorded), errorHandler = useRef(onError);
  handler.current = onRecorded; errorHandler.current = onError;
  useEffect(() => {
    if (!recording || !hydrated) return;
    const id = `local-shortcut-${Array.from(crypto.getRandomValues(new Uint32Array(4)), (value) => value.toString(16)).join('-')}`;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    const starting = (async () => {
      const unlisten = await subscribe((event) => { if (!disposed && event.recordingId === id) handler.current(event.hotkey); });
      if (disposed) { unlisten(); return; }
      cleanup = unlisten;
      await begin(id);
    })();
    void starting.catch(() => { if (!disposed) errorHandler.current('快捷键录制未能启动，请重试。'); });
    return () => {
      disposed = true; cleanup?.();
      void starting.then(() => end(id), () => undefined).catch(() => {});
    };
  }, [recording, hydrated, begin, end, subscribe]);
}
