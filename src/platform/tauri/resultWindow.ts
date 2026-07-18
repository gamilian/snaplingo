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
  async place(position, lastPosition) {
    const window = getCurrentWindow();
    const cursor = await cursorPosition();
    const storedPosition =
      position === 'last-position' && isFinitePosition(lastPosition)
        ? lastPosition
        : null;
    const monitor =
      (storedPosition
        ? await monitorFromPoint(storedPosition.x, storedPosition.y)
        : await monitorFromPoint(cursor.x, cursor.y)) ?? (await currentMonitor());
    if (!monitor) return;

    const size = await window.innerSize();
    const { position: workPosition, size: workSize } = monitor.workArea;
    let x = storedPosition?.x ?? cursor.x;
    let y = storedPosition?.y ?? cursor.y;

    if (position === 'center' || (position === 'last-position' && !storedPosition)) {
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
  async startDragging() {
    const window = getCurrentWindow();
    await window.startDragging();
    const position = await window.outerPosition();
    return { x: position.x, y: position.y };
  },
  setAlwaysOnTop(value) {
    return getCurrentWindow().setAlwaysOnTop(value);
  },
};

function isFinitePosition(
  value: unknown,
): value is { x: number; y: number } {
  if (!value || typeof value !== 'object') return false;
  const position = value as Partial<{ x: number; y: number }>;
  return Number.isFinite(position.x) && Number.isFinite(position.y);
}

export function getCurrentWindowLabel() {
  return getCurrentWindow().label;
}
