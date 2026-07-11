export type CaptureCancelHandler = () => void | Promise<void>;
export type CaptureCopyHandler = () => void | Promise<void>;
export type CaptureCancelRequestListener = (
  handler: () => void,
) => Promise<() => void>;

export function subscribeCaptureCancelRequests(
  onCancel: CaptureCancelHandler,
  listenForEvent: CaptureCancelRequestListener,
) {
  return listenForEvent(() => {
    void onCancel();
  });
}

export function subscribeCaptureCopyRequests(
  onCopy: CaptureCopyHandler,
  listenForEvent: CaptureCancelRequestListener,
) {
  return listenForEvent(() => {
    void onCopy();
  });
}
