import { useLayoutEffect } from 'react';
import type { GeneralSettings } from '../application/settings/ports';
import { resolveApplicationTheme } from '../application/settings/theme';

export function useApplicationTheme(
  general: GeneralSettings | null,
  scope: 'settings' | 'application' = 'application',
) {
  useLayoutEffect(() => {
    const root = document.documentElement;
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    root.dataset.themeScope = scope;
    root.dataset.interfaceTheme = 'native';
    root.lang = general?.language ?? 'zh-CN';
    root.dataset.experimentalGpu = String(general?.experimentalGpuAcceleration ?? false);

    const applyAppearance = () => {
      const appearance = resolveApplicationTheme({ configuredTheme: 'system', prefersDark: media.matches });
      root.dataset.theme = appearance;
      root.style.colorScheme = appearance;
    };
    applyAppearance();
    media.addEventListener('change', applyAppearance);
    return () => media.removeEventListener('change', applyAppearance);
  }, [general, scope]);
}
