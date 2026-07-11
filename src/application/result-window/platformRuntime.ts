import type {
  ResultPayloadReadyHandler,
  ResultWindowEventsPort,
  ResultWindowPort,
  ResultWindowUnsubscribe,
} from './ports';

export interface ResultWindowPlatformRuntime {
  onPayloadReady(
    handler: ResultPayloadReadyHandler,
  ): Promise<ResultWindowUnsubscribe>;
  resizeTo(width: number, height: number): Promise<void>;
  dismiss(): Promise<void>;
  beginDrag(): Promise<void>;
}

interface ResultWindowPlatformPorts {
  events: ResultWindowEventsPort;
  window: ResultWindowPort;
}

export function createResultWindowPlatformRuntime(
  ports: ResultWindowPlatformPorts,
): ResultWindowPlatformRuntime {
  return {
    onPayloadReady: (handler) =>
      ports.events.subscribeResultPayloadReady(handler),
    resizeTo: (width, height) => ports.window.resize(width, height),
    dismiss: () => ports.window.hide(),
    beginDrag: () => ports.window.startDragging(),
  };
}
