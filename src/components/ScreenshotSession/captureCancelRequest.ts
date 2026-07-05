import { listenTauriEvent } from '../../tauri/events';

export const CAPTURE_CANCEL_REQUESTED_EVENT = 'capture-cancel-requested';
export const CAPTURE_COPY_REQUESTED_EVENT = 'capture-copy-requested';

export type CaptureCancelHandler = () => void | Promise<void>;
export type CaptureCopyHandler = () => void | Promise<void>;
export type CaptureCancelRequestListener = (
  eventName:
    | typeof CAPTURE_CANCEL_REQUESTED_EVENT
    | typeof CAPTURE_COPY_REQUESTED_EVENT,
  handler: () => void,
) => Promise<() => void>;

export function subscribeCaptureCancelRequests(
  onCancel: CaptureCancelHandler,
  listenForEvent: CaptureCancelRequestListener = (eventName, handler) =>
    listenTauriEvent(eventName, handler),
) {
  return listenForEvent(CAPTURE_CANCEL_REQUESTED_EVENT, () => {
    void onCancel();
  });
}

export function subscribeCaptureCopyRequests(
  onCopy: CaptureCopyHandler,
  listenForEvent: CaptureCancelRequestListener = (eventName, handler) =>
    listenTauriEvent(eventName, handler),
) {
  return listenForEvent(CAPTURE_COPY_REQUESTED_EVENT, () => {
    void onCopy();
  });
}
