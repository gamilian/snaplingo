import type { CaptureWorkspacePointerInput } from './captureWorkspaceRuntimeTypes';

interface CapturePointerFrameScheduler {
  requestFrame(callback: FrameRequestCallback): number;
  cancelFrame(frameId: number): void;
  move(input: CaptureWorkspacePointerInput): void;
}

export interface CapturePointerFrameDispatcher {
  schedule(input: CaptureWorkspacePointerInput): void;
  flush(): void;
  cancel(): void;
}

export function createCapturePointerFrameDispatcher({
  requestFrame,
  cancelFrame,
  move,
}: CapturePointerFrameScheduler): CapturePointerFrameDispatcher {
  let frameId: number | null = null;
  let pendingInput: CaptureWorkspacePointerInput | null = null;

  const flush = () => {
    if (frameId !== null) {
      cancelFrame(frameId);
      frameId = null;
    }
    const input = pendingInput;
    pendingInput = null;
    if (input) move(input);
  };

  return {
    schedule(input) {
      pendingInput = input;
      if (frameId !== null) return;

      frameId = requestFrame(() => {
        frameId = null;
        const latestInput = pendingInput;
        pendingInput = null;
        if (latestInput) move(latestInput);
      });
    },
    flush,
    cancel() {
      if (frameId !== null) cancelFrame(frameId);
      frameId = null;
      pendingInput = null;
    },
  };
}
