import { listen, type EventCallback } from '@tauri-apps/api/event';

export function listenTauriEvent<T>(
  eventName: string,
  handler: EventCallback<T>,
) {
  return listen<T>(eventName, handler);
}
