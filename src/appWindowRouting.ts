export const CAPTURE_RESULT_WINDOW_LABEL = 'capture-result';

export function isCaptureResultWindowLaunch(label: string, search: string) {
  return (
    label === CAPTURE_RESULT_WINDOW_LABEL ||
    new URLSearchParams(search).get('window') === CAPTURE_RESULT_WINDOW_LABEL
  );
}
