import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface InputTranslationEvent {
  text: string;
  autoTranslate: boolean;
}

type EventCallback<T> = (payload: T) => void;

export interface MainWindowEventHandlers {
  onScreenshotCaptured?: EventCallback<string>;
  onScreenshotError?: EventCallback<string>;
  onInputTranslation?: EventCallback<InputTranslationEvent>;
  onInputOcr?: EventCallback<string>;
  onShowOcrWindow?: () => void;
  onStartFileOcr?: () => void;
  onShowTranslationWindow?: () => void;
}

function parseString(payload: unknown): string | null {
  return typeof payload === 'string' ? payload : null;
}

export function parseInputTranslationEvent(
  payload: unknown,
): InputTranslationEvent | null {
  if (typeof payload === 'string') {
    return { text: payload, autoTranslate: true };
  }

  if (!payload || typeof payload !== 'object') return null;
  const input = payload as { text?: unknown; autoTranslate?: unknown };
  if (typeof input.text !== 'string') return null;

  return {
    text: input.text,
    autoTranslate: input.autoTranslate !== false,
  };
}

async function onEvent<T>(
  eventName: string,
  callback: EventCallback<T> | undefined,
  parse: (payload: unknown) => T | null,
): Promise<UnlistenFn> {
  return listen<unknown>(eventName, (event) => {
    const payload = parse(event.payload);
    if (payload !== null) callback?.(payload);
  });
}

export async function subscribeMainWindowEvents(
  handlers: MainWindowEventHandlers,
): Promise<UnlistenFn> {
  const unlisteners = await Promise.all([
    onEvent('screenshot-captured', handlers.onScreenshotCaptured, parseString),
    onEvent('screenshot-error', handlers.onScreenshotError, parseString),
    onEvent(
      'input-translation',
      handlers.onInputTranslation,
      parseInputTranslationEvent,
    ),
    onEvent('input-ocr', handlers.onInputOcr, parseString),
    listen('show-ocr-window', () => handlers.onShowOcrWindow?.()),
    listen('start-file-ocr', () => handlers.onStartFileOcr?.()),
    listen('show-translation-window', () =>
      handlers.onShowTranslationWindow?.(),
    ),
  ]);

  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}
