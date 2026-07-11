import type { CaptureLaunch } from '../../domain/capture';

export type CaptureWorkspaceUnsubscribe = () => void;

export type CaptureWorkspaceRequestHandler = () => void | Promise<void>;
export type CaptureHotkeyHandler = (
  launch: CaptureLaunch,
) => void | Promise<void>;

export interface CaptureWorkspaceEventsPort {
  subscribeCaptureCancel(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  subscribeCaptureCopy(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  subscribeHotkeyTriggered(
    handler: CaptureHotkeyHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
}

export interface CaptureWindowPort {
  prepareForReveal(): Promise<void>;
  reveal(): Promise<void>;
  hide(): Promise<void>;
}
