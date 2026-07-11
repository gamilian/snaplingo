export interface PinnedWindowPort {
  resize(width: number, height: number): Promise<void>;
  moveBy(deltaX: number, deltaY: number): Promise<void>;
  startDragging(): Promise<void>;
  close(): Promise<void>;
}
