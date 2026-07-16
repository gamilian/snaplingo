export function resolveApplicationTheme({
  configuredTheme,
  isSettingsWindow,
  prefersDark,
}: {
  configuredTheme: string;
  isSettingsWindow: boolean;
  prefersDark: boolean;
}): 'light' | 'dark' {
  if (!isSettingsWindow) return 'light';
  if (configuredTheme === 'dark') return 'dark';
  if (configuredTheme === 'light') return 'light';
  return prefersDark ? 'dark' : 'light';
}
