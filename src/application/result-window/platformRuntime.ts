import type {
  ResultPayloadReadyHandler,
  ResultWindowClipboardPort,
  ResultWindowCommandsPort,
  ResultWindowEventsPort,
  ResultWindowPort,
  ResultWindowUnsubscribe,
} from './ports';

export interface ResultWindowPlatformRuntime {
  commands: ResultWindowCommandsPort;
  clipboard: {
    copyText(text: string): Promise<void>;
  };
  onPayloadReady(
    handler: ResultPayloadReadyHandler,
  ): Promise<ResultWindowUnsubscribe>;
  resizeTo(width: number, height: number): Promise<void>;
  dismiss(): Promise<void>;
  beginDrag(): Promise<void>;
  setAlwaysOnTop(value: boolean): Promise<void>;
}

interface ResultWindowPlatformPorts {
  commands: ResultWindowCommandsPort;
  clipboard: ResultWindowClipboardPort;
  events: ResultWindowEventsPort;
  window: ResultWindowPort;
}

export function createResultWindowPlatformRuntime(
  ports: ResultWindowPlatformPorts,
): ResultWindowPlatformRuntime {
  return {
    commands: ports.commands,
    clipboard: {
      copyText: (text) => ports.clipboard.writeText(text),
    },
    onPayloadReady: (handler) =>
      ports.events.subscribeResultPayloadReady(handler),
    resizeTo: (width, height) => ports.window.resize(width, height),
    dismiss: () => ports.window.hide(),
    beginDrag: () => ports.window.startDragging(),
    setAlwaysOnTop: (value) => ports.window.setAlwaysOnTop(value),
  };
}
