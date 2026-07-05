import {
  getCurrentWindow,
  LogicalSize,
  PhysicalPosition,
} from '@tauri-apps/api/window';
import {
  getCurrentWebviewWindow,
  WebviewWindow,
} from '@tauri-apps/api/webviewWindow';

export function getCurrentAppWindow() {
  return getCurrentWindow();
}

export function getCurrentAppWebviewWindow() {
  return getCurrentWebviewWindow();
}

export function getWebviewWindowByLabel(label: string) {
  return WebviewWindow.getByLabel(label);
}

export function createLogicalSize(width: number, height: number) {
  return new LogicalSize(width, height);
}

export function createPhysicalPosition(x: number, y: number) {
  return new PhysicalPosition(x, y);
}
