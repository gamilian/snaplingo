import type { PinnedWindowPort } from './ports';

export interface PinnedImagePlatformRuntime {
  resizeTo(width: number, height: number): Promise<void>;
  moveBy(deltaX: number, deltaY: number): Promise<void>;
  beginDrag(): Promise<void>;
  dismiss(): Promise<void>;
}

interface PinnedImagePlatformPorts {
  window: PinnedWindowPort;
}

export function createPinnedImagePlatformRuntime(
  ports: PinnedImagePlatformPorts,
): PinnedImagePlatformRuntime {
  return {
    resizeTo: (width, height) => ports.window.resize(width, height),
    moveBy: (deltaX, deltaY) => ports.window.moveBy(deltaX, deltaY),
    beginDrag: () => ports.window.startDragging(),
    dismiss: () => ports.window.close(),
  };
}
