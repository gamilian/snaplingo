export interface PinnedWindowPort {
  resize(width: number, height: number): Promise<void>;
  moveBy(deltaX: number, deltaY: number): Promise<void>;
  startDragging(): Promise<void>;
  close(): Promise<void>;
}

export interface PinnedImageCommandsPort {
  getPinnedImage(imageId: string): Promise<PinnedImageView>;
  defaultCaptureSavePath(): Promise<string>;
  quickCaptureSavePath(directory?: string): Promise<string>;
  copyPinnedImage(imageId: string): Promise<void>;
  replacePinnedImageFromClipboard(imageId: string): Promise<PinnedImageView>;
  savePinnedImage(imageId: string, path: string): Promise<void>;
  closePinnedImage(imageId: string): Promise<void>;
  removePinnedImage(imageId: string): Promise<void>;
  movePinnedImageToNextGroup(imageId: string): Promise<number>;
  hidePinnedImageGroup(imageId: string): Promise<string[]>;
  destroyPinnedImageGroup(imageId: string): Promise<string[]>;
}

export interface PinnedImageClipboardPort {
  writeText(text: string): Promise<void>;
}

export interface PinnedImageSettingsPort {
  open(): Promise<void>;
}
import type { PinnedImageView } from '../../domain/capture';
