import type {
  CaptureHotkeyHandler,
  CaptureWorkspaceClipboardPort,
  CaptureWorkspaceCommandsPort,
  CaptureWindowPort,
  CaptureWorkspaceEventsPort,
  CaptureWorkspaceRequestHandler,
  CaptureWorkspaceUnsubscribe,
} from './ports';


export interface CaptureWorkspacePlatformRuntime {
  commands: CaptureWorkspaceCommandsPort;
  clipboard: {
    copyText(text: string): Promise<void>;
  };
  onCancelRequested(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  onCopyRequested(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  onSaveRequested(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  onUndoRequested(
    handler: CaptureWorkspaceRequestHandler,
  ): Promise<CaptureWorkspaceUnsubscribe>;
  onRedoRequested(
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
  commands: CaptureWorkspaceCommandsPort;
  clipboard: CaptureWorkspaceClipboardPort;
  events: CaptureWorkspaceEventsPort;
  window: CaptureWindowPort;
}

export function createCaptureWorkspacePlatformRuntime(
  ports: CaptureWorkspacePlatformPorts,
): CaptureWorkspacePlatformRuntime {
  return {
    commands: ports.commands,
    clipboard: {
      copyText: (text) => ports.clipboard.writeText(text),
    },
    onCancelRequested: (handler) =>
      ports.events.subscribeCaptureCancel(handler),
    onCopyRequested: (handler) => ports.events.subscribeCaptureCopy(handler),
    onSaveRequested: (handler) => ports.events.subscribeCaptureSave(handler),
    onUndoRequested: (handler) => ports.events.subscribeCaptureUndo(handler),
    onRedoRequested: (handler) => ports.events.subscribeCaptureRedo(handler),
    onHotkeyTriggered: (handler) =>
      ports.events.subscribeHotkeyTriggered(handler),
    prepareForReveal: () => ports.window.prepareForReveal(),
    reveal: () => ports.window.reveal(),
    dismiss: () => ports.window.hide(),
  };
}
