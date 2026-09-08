import { invoke } from '@tauri-apps/api/core';
import type { CaptureWindowPort } from '../../application/capture-workspace/ports';

export const captureWindow: CaptureWindowPort = {
  prepareForReveal(sessionId) {
    return invoke<void>('prepare_capture_window_for_reveal', { sessionId });
  },
  reveal(sessionId) {
    return invoke<void>('reveal_capture_window', { sessionId });
  },
  hide() {
    return invoke<void>('hide_capture_window');
  },
};
