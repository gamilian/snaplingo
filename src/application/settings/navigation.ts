export const SETTINGS_NAVIGATION_TABS = [
  'general',
  'screenshot',
  'translation',
  'ocr',
  'services',
  'favorites',
  'history',
] as const;

export type SettingsNavigationTab = (typeof SETTINGS_NAVIGATION_TABS)[number];

export interface SettingsNavigationRequest {
  tab: SettingsNavigationTab;
  section?: string;
}

export function parseSettingsNavigationRequest(
  payload: unknown,
): SettingsNavigationRequest | null {
  if (!payload || typeof payload !== 'object') return null;

  const candidate = payload as { tab?: unknown; section?: unknown };
  if (
    typeof candidate.tab !== 'string' ||
    !SETTINGS_NAVIGATION_TABS.includes(candidate.tab as SettingsNavigationTab)
  ) {
    return null;
  }
  if (
    candidate.section !== undefined &&
    typeof candidate.section !== 'string'
  ) {
    return null;
  }

  return {
    tab: candidate.tab as SettingsNavigationTab,
    ...(candidate.section ? { section: candidate.section } : {}),
  };
}

export function readSettingsNavigationLaunch(
  search: string,
): SettingsNavigationRequest | null {
  const params = new URLSearchParams(search);
  return parseSettingsNavigationRequest({
    tab: params.get('tab'),
    section: params.get('section') ?? undefined,
  });
}
