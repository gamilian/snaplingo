import {
  currentMonitor,
  cursorPosition,
  getCurrentWindow,
  LogicalSize,
  monitorFromPoint,
  PhysicalPosition,
} from '@tauri-apps/api/window';
import type { ResultWindowPort } from '../../application/result-window/ports';

const LAST_RESULT_WINDOW_POSITION_KEY = 'snaplingo.result-window.last-position';

interface StoredWindowPosition {
  x: number;
  y: number;
}

export const resultWindow: ResultWindowPort = {
  resize(width, height) {
    return getCurrentWindow().setSize(new LogicalSize(width, height));
  },
  async place(position) {
    const window = getCurrentWindow();
    const cursor = await cursorPosition();
    const storedPosition =
      position === 'last-position' ? readLastResultWindowPosition() : null;
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
    writeLastResultWindowPosition(await window.outerPosition());
  },
  setAlwaysOnTop(value) {
    return getCurrentWindow().setAlwaysOnTop(value);
  },
};

export function readLastResultWindowPosition(): StoredWindowPosition | null {
  try {
    const value = JSON.parse(localStorage.getItem(LAST_RESULT_WINDOW_POSITION_KEY) ?? 'null');
    return isFinitePosition(value) ? value : null;
  } catch {
    return null;
  }
}

function writeLastResultWindowPosition(position: StoredWindowPosition) {
  if (!isFinitePosition(position)) return;
  localStorage.setItem(LAST_RESULT_WINDOW_POSITION_KEY, JSON.stringify(position));
}

function isFinitePosition(value: unknown): value is StoredWindowPosition {
  if (!value || typeof value !== 'object') return false;
  const position = value as Partial<StoredWindowPosition>;
  return Number.isFinite(position.x) && Number.isFinite(position.y);
}

export function getCurrentWindowLabel() {
  return getCurrentWindow().label;
}
