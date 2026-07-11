import { invoke } from '@tauri-apps/api/core';
import type { CaptureWindowPort } from '../../application/capture-workspace/ports';

export const captureWindow: CaptureWindowPort = {
  prepareForReveal() {
    return invoke<void>('prepare_capture_window_for_reveal');
  },
  reveal() {
    return invoke<void>('reveal_capture_window');
  },
  hide() {
    return invoke<void>('hide_capture_window');
  },
};
