import type {
  PinnedImageClipboardPort,
  PinnedImageCommandsPort,
  PinnedImageSettingsPort,
  PinnedWindowPort,
} from './ports';

export interface PinnedImagePlatformRuntime {
  commands: PinnedImageCommandsPort;
  clipboard: {
    copyText(text: string): Promise<void>;
  };
  settings: PinnedImageSettingsPort;
  resizeTo(width: number, height: number): Promise<void>;
  moveBy(deltaX: number, deltaY: number): Promise<void>;
  beginDrag(): Promise<void>;
  dismiss(): Promise<void>;
}

interface PinnedImagePlatformPorts {
  commands: PinnedImageCommandsPort;
  clipboard: PinnedImageClipboardPort;
  settings: PinnedImageSettingsPort;
  window: PinnedWindowPort;
}

export function createPinnedImagePlatformRuntime(
  ports: PinnedImagePlatformPorts,
): PinnedImagePlatformRuntime {
  return {
    commands: ports.commands,
    clipboard: {
      copyText: (text) => ports.clipboard.writeText(text),
    },
    settings: ports.settings,
    resizeTo: (width, height) => ports.window.resize(width, height),
    moveBy: (deltaX, deltaY) => ports.window.moveBy(deltaX, deltaY),
    beginDrag: () => ports.window.startDragging(),
    dismiss: () => ports.window.close(),
  };
}
