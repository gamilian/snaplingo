export type ResultWindowUnsubscribe = () => void;
export type ResultPayloadReadyHandler = () => void | Promise<void>;

export interface ResultWindowEventsPort {
  subscribeResultPayloadReady(
    handler: ResultPayloadReadyHandler,
  ): Promise<ResultWindowUnsubscribe>;
}

export interface ResultWindowPort {
  resize(width: number, height: number): Promise<void>;
  hide(): Promise<void>;
  startDragging(): Promise<void>;
}
