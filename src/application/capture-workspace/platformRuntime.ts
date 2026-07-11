import type {
  CaptureHotkeyHandler,
  CaptureWindowPort,
  CaptureWorkspaceEventsPort,
  CaptureWorkspaceRequestHandler,
  CaptureWorkspaceUnsubscribe,
} from './ports';

export interface CaptureWorkspacePlatformRuntime {
  onCancelRequested(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  onCopyRequested(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  onHotkeyTriggered(
    handler: CaptureHotkeyHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  prepareForReveal(): Promise<void>;
  reveal(): Promise<void>;
  dismiss(): Promise<void>;
}

interface CaptureWorkspacePlatformPorts {
  events: CaptureWorkspaceEventsPort;
  window: CaptureWindowPort;
}

export function createCaptureWorkspacePlatformRuntime(
  ports: CaptureWorkspacePlatformPorts,
): CaptureWorkspacePlatformRuntime {
  return {
    onCancelRequested: (handler) =>
      ports.events.subscribeCaptureCancel(handler),
    onCopyRequested: (handler) => ports.events.subscribeCaptureCopy(handler),
    onHotkeyTriggered: (handler) =>
      ports.events.subscribeHotkeyTriggered(handler),
    prepareForReveal: () => ports.window.prepareForReveal(),
    reveal: () => ports.window.reveal(),
    dismiss: () => ports.window.hide(),
  };
}
