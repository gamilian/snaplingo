import {
  currentMonitor,
  cursorPosition,
  getCurrentWindow,
  LogicalSize,
  monitorFromPoint,
  PhysicalPosition,
} from '@tauri-apps/api/window';
import type { ResultWindowPort } from '../../application/result-window/ports';

export const resultWindow: ResultWindowPort = {
  resize(width, height) {
    return getCurrentWindow().setSize(new LogicalSize(width, height));
  },
  async place(position) {
    const window = getCurrentWindow();
    const cursor = await cursorPosition();
    const monitor =
      (await monitorFromPoint(cursor.x, cursor.y)) ?? (await currentMonitor());
    if (!monitor) return;

    const size = await window.innerSize();
    const { position: workPosition, size: workSize } = monitor.workArea;
    let x = cursor.x;
    let y = cursor.y;

    if (position === 'center') {
      x = workPosition.x + (workSize.width - size.width) / 2;
      y = workPosition.y + (workSize.height - size.height) / 2;
    } else if (position === 'below-cursor') {
      y += Math.round(12 * monitor.scaleFactor);
    }

    x = Math.min(
      Math.max(x, workPosition.x),
      workPosition.x + Math.max(0, workSize.width - size.width),
    );
    y = Math.min(
      Math.max(y, workPosition.y),
      workPosition.y + Math.max(0, workSize.height - size.height),
    );

    await window.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));
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
