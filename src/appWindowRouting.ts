export const CAPTURE_RESULT_WINDOW_LABEL = 'capture-result';
export const SETTINGS_WINDOW_LABEL = 'settings';

export function isCaptureResultWindowLaunch(label: string, search: string) {
  return (
    label === CAPTURE_RESULT_WINDOW_LABEL ||
    new URLSearchParams(search).get('window') === CAPTURE_RESULT_WINDOW_LABEL
  );
}

export function isSettingsWindowLaunch(label: string, _search: string): boolean {
  return label === SETTINGS_WINDOW_LABEL;
}
