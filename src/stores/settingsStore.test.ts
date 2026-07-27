import { beforeEach, describe, expect, it, vi } from 'vitest';

function createMemoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => Array.from(values.keys())[index] ?? null,
    removeItem: (key) => values.delete(key),
    setItem: (key, value) => values.set(key, value),
  };
}

function legacyPersistedState(state: Record<string, unknown>) {
  localStorage.setItem(
    'snaplingo-settings',
    JSON.stringify({ state, version: 0 }),
  );
}

describe('settingsStore navigation persistence', () => {
  beforeEach(() => {
    vi.resetModules();
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      value: createMemoryStorage(),
    });
  });

  it('keeps durable configuration out of local navigation state', async () => {
    legacyPersistedState({
      activeMainTab: 'translation',
      servicesSubTab: 'translation',
      hotkeys: { screenshot: { screenshot: 'F12' } },
      language: 'en',
      theme: 'dark',
      screenshotSavePath: '~/legacy-captures',
    });

    const { useSettingsStore } = await import('./settingsStore');
    const state = useSettingsStore.getState() as unknown as Record<string, unknown>;

    expect(state.activeMainTab).toBe('translation');
    expect(state.servicesSubTab).toBe('translation');
    expect('hotkeys' in state).toBe(false);
    expect('language' in state).toBe(false);
    expect('theme' in state).toBe(false);
    expect('screenshotSavePath' in state).toBe(false);
  });

  it('falls back when persisted navigation points at the removed advanced page', async () => {
    legacyPersistedState({
      activeMainTab: 'advanced',
      servicesSubTab: 'translation',
    });
    const { useSettingsStore } = await import('./settingsStore');

    expect(useSettingsStore.getState().activeMainTab).toBe('screenshot');
    expect(useSettingsStore.getState().servicesSubTab).toBe('translation');
  });

  it('keeps one-shot section requests out of persisted navigation state', async () => {
    const { useSettingsStore } = await import('./settingsStore');

    useSettingsStore.getState().navigate({ tab: 'general', section: 'about' });

    expect(useSettingsStore.getState()).toMatchObject({
      activeMainTab: 'general',
      requestedSection: 'about',
    });
    expect(
      JSON.parse(localStorage.getItem('snaplingo-settings') ?? '{}').state,
    ).not.toHaveProperty('requestedSection');
  });
});
