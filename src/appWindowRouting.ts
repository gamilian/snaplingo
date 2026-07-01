export const CAPTURE_RESULT_WINDOW_LABEL = 'capture-result';

export function isCaptureResultWindowLaunch(label: string, search: string) {
  return (
    label === CAPTURE_RESULT_WINDOW_LABEL ||
    new URLSearchParams(search).get('window') === CAPTURE_RESULT_WINDOW_LABEL
  );
}

export function isSettingsWindowLaunch(label: string, search: string): boolean {
  const params = new URLSearchParams(search);
  return label === 'settings' || params.get('window') === 'settings';
}
