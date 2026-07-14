import {
  getCurrentWindow,
  LogicalSize,
  PhysicalPosition,
} from '@tauri-apps/api/window';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { PinnedWindowPort } from '../../application/pinned-image/ports';

export const pinnedWindow: PinnedWindowPort = {
  resize(width, height) {
    return getCurrentWindow().setSize(new LogicalSize(width, height));
  },
  async moveBy(deltaX, deltaY) {
    const window = getCurrentWindow();
    const position = await window.outerPosition();

    await window.setPosition(
      new PhysicalPosition(position.x + deltaX, position.y + deltaY),
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
