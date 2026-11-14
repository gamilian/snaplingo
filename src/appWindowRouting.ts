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

export function applicationWindow(label: string, search: string) {
  const requested = new URLSearchParams(search).get('window');
  if (requested === 'pin') return 'pin';
  if (requested === 'scrolling' || requested === 'scrolling-preview') return 'scrolling';
  if (requested === 'scrolling-outline') return 'scrolling-outline';
  if (isCaptureResultWindowLaunch(label, search)) return 'result';
  if (isSettingsWindowLaunch(label, search)) return 'settings';
  return null;
}
