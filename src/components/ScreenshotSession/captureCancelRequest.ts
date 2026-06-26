import { listen } from '@tauri-apps/api/event';

export const CAPTURE_CANCEL_REQUESTED_EVENT = 'capture-cancel-requested';

export type CaptureCancelHandler = () => void | Promise<void>;
export type CaptureCancelRequestListener = (
  eventName: typeof CAPTURE_CANCEL_REQUESTED_EVENT,
  handler: () => void,
) => Promise<() => void>;

export function subscribeCaptureCancelRequests(
  onCancel: CaptureCancelHandler,
  listenForEvent: CaptureCancelRequestListener = (eventName, handler) =>
    listen(eventName, handler),
) {
  return listenForEvent(CAPTURE_CANCEL_REQUESTED_EVENT, () => {
    void onCancel();
  });
}
