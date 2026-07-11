import type { ResultWindowPresentation } from './presentation';

interface ResultWindowCloseOptions {
  presentation: ResultWindowPresentation;
  hideResultWindow: () => void;
  hideNativeWindow?: () => Promise<void> | void;
}

export function closeResultWindowForPresentation({
  presentation,
  hideResultWindow,
  hideNativeWindow,
}: ResultWindowCloseOptions) {
  hideResultWindow();

  if (presentation !== 'standalone' || !hideNativeWindow) return;

  try {
    const result = hideNativeWindow();
    if (result && typeof result.catch === 'function') {
      void result.catch(() => undefined);
    }
  } catch {
    // Store state is already hidden; native hide failures should not reopen UI.
  }
}
