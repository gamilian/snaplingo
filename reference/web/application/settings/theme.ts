export type InterfaceTheme = 'native' | 'studio' | 'paper';

export function resolveInterfaceTheme(value?: string): InterfaceTheme {
  return value === 'studio' || value === 'paper' ? value : 'native';
}

export function resolveApplicationTheme({
  configuredTheme,
  prefersDark,
}: {
  configuredTheme: string;
  prefersDark: boolean;
}): 'light' | 'dark' {
  if (configuredTheme === 'dark') return 'dark';
  if (configuredTheme === 'light') return 'light';
  return prefersDark ? 'dark' : 'light';
}
