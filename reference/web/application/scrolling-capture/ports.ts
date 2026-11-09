import type { ScrollingCaptureView } from '../../domain/scrollingCapture';
import type { CaptureOutputAction } from '../capture-workspace/ports';

export type ScrollingOutputAction = Exclude<
  CaptureOutputAction,
  { type: 'favorite' }
> | { type: 'complete' };

export interface ScrollingCapturePorts {
  commands: {
    get(captureId: string): Promise<ScrollingCaptureView>;
    capture(captureId: string): Promise<ScrollingCaptureView>;
    undo(captureId: string): Promise<ScrollingCaptureView>;
    finish(captureId: string, action: ScrollingOutputAction): Promise<void>;
    cancel(captureId: string): Promise<void>;
    saveAction(): Promise<ScrollingOutputAction | null>;
  };
  events: {
    subscribeUpdates(
      handler: (view: ScrollingCaptureView) => void,
    ): Promise<() => void>;
    subscribeErrors(handler: (error: string) => void): Promise<() => void>;
  };
  window: {
    resizePreview(expanded: boolean): Promise<boolean | void>;
    startDragging(): Promise<void>;
  };
}
