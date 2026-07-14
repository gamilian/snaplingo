import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import type { ResultWindowPort } from '../../application/result-window/ports';

export const resultWindow: ResultWindowPort = {
  resize(width, height) {
    return getCurrentWindow().setSize(new LogicalSize(width, height));
  },
  hide() {
    return getCurrentWindow().hide();
  },
  startDragging() {
    return getCurrentWindow().startDragging();
  },
  setAlwaysOnTop(value) {
    return getCurrentWindow().setAlwaysOnTop(value);
  },
};

export function getCurrentWindowLabel() {
  return getCurrentWindow().label;
}
