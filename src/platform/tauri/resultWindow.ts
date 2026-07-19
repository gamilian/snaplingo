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
  startDragging() {
    const window = getCurrentWindow();
    return new Promise((resolve, reject) => {
      let latestPosition: { x: number; y: number } | undefined;
      let unlisten: (() => void) | undefined;
      let settleTimer: ReturnType<typeof setTimeout> | undefined;
      let idleTimer: ReturnType<typeof setTimeout> | undefined;
      let finished = false;

      const clearTimers = () => {
        if (settleTimer !== undefined) clearTimeout(settleTimer);
        if (idleTimer !== undefined) clearTimeout(idleTimer);
      };

      const finish = async () => {
        if (finished) return;
        finished = true;
        clearTimers();
        unlisten?.();
        try {
          const position = latestPosition ?? (await window.outerPosition());
          resolve({ x: position.x, y: position.y });
        } catch (error) {
          reject(error);
        }
      };

      const scheduleFinish = (position: { x: number; y: number }) => {
        latestPosition = position;
        if (idleTimer !== undefined) clearTimeout(idleTimer);
        if (settleTimer !== undefined) clearTimeout(settleTimer);
        settleTimer = setTimeout(() => void finish(), 250);
      };

      void (async () => {
        try {
          unlisten = await window.onMoved(({ payload }) => {
            scheduleFinish({ x: payload.x, y: payload.y });
          });
          await window.startDragging();
          idleTimer = setTimeout(() => void finish(), 1_500);
        } catch (error) {
          finished = true;
          clearTimers();
          unlisten?.();
          reject(error);
        }
      })();
    });
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
