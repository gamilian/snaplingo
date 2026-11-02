import {
  getCurrentWindow,
  LogicalSize,
  PhysicalPosition,
} from '@tauri-apps/api/window';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { PinnedWindowPort } from '../../application/pinned-image/ports';
let resizeTail = Promise.resolve();

export const pinnedWindow: PinnedWindowPort = {
  resize(width, height) {
    const operation = resizeTail.then(async () => {
      const window = getCurrentWindow();
      const [position, size, scale] = await Promise.all([window.outerPosition(), window.outerSize(), window.scaleFactor()]);
      await window.setSize(new LogicalSize(width, height));
      await window.setPosition(new PhysicalPosition(
        Math.round(position.x + (size.width - width * scale) / 2),
        Math.round(position.y + (size.height - height * scale) / 2),
      ));
    });
    resizeTail = operation.catch(() => {});
    return operation;
  },
  async moveBy(deltaX, deltaY) {
    const window = getCurrentWindow();
    const position = await window.outerPosition();
    const scale = await window.scaleFactor();

    await window.setPosition(
      new PhysicalPosition(position.x + Math.round(deltaX * scale), position.y + Math.round(deltaY * scale)),
    );
  },
  startDragging() {
    return getCurrentWindow().startDragging();
  },
  setShadow(enabled) {
    return getCurrentWindow().setShadow(enabled);
  },
  close() {
    return getCurrentWebviewWindow().close();
  },
};
