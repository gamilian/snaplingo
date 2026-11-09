import { useLayoutEffect } from 'react';
import type { GeneralSettings } from '../application/settings/ports';
import { resolveApplicationTheme, resolveInterfaceTheme } from '../application/settings/theme';

export function useApplicationTheme(
  general: GeneralSettings | null,
  scope: 'settings' | 'application' = 'application',
) {
  useLayoutEffect(() => {
    const root = document.documentElement;
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const configuredTheme = general?.theme ?? 'system';
    root.dataset.themeScope = scope;
    root.dataset.interfaceTheme = resolveInterfaceTheme(general?.interfaceTheme);
    root.lang = general?.language ?? 'zh-CN';
    root.dataset.experimentalGpu = String(general?.experimentalGpuAcceleration ?? false);

    const applyAppearance = () => {
      const appearance = resolveApplicationTheme({ configuredTheme, prefersDark: media.matches });
      root.dataset.theme = appearance;
      root.style.colorScheme = appearance;
    };
    applyAppearance();
    if (configuredTheme !== 'system') return;
    media.addEventListener('change', applyAppearance);
    return () => media.removeEventListener('change', applyAppearance);
  }, [general, scope]);
}
