import { describe, expect, it, vi } from 'vitest';

import { findSettingsSection } from './navigationModel';
import { createSettingsNavigationState, type SettingsSecondaryKeys } from './settingsNavigationState';

describe('settings navigation state', () => {
  it('keeps a valid persisted secondary key active', () => {
    const state = createSettingsNavigationState(
      findSettingsSection('services'),
      secondaryKeys({ services: 'translation' }),
      noopSetters(),
    );

    expect(state.activeKey).toBe('translation');
    expect(state.activeItem?.key).toBe('translation');
  });

  it('falls back to the first secondary key when persisted state is stale', () => {
    const state = createSettingsNavigationState(
      findSettingsSection('services'),
      secondaryKeys({ services: 'removed-tab' }),
      noopSetters(),
    );

    expect(state.activeKey).toBe('ocr');
    expect(state.activeItem?.key).toBe('ocr');
  });

  it('returns no secondary key for a simple section', () => {
    const state = createSettingsNavigationState(
      findSettingsSection('translation'),
      secondaryKeys(),
      noopSetters(),
    );

    expect(state.activeKey).toBe('');
    expect(state.activeItem).toBeNull();
  });

  it('ignores invalid secondary click keys', () => {
    const setters = noopSetters();
    const state = createSettingsNavigationState(
      findSettingsSection('services'),
      secondaryKeys({ services: 'ocr' }),
      setters,
    );

    state.setActiveKey('removed-tab');

    expect(setters.services).not.toHaveBeenCalled();
  });
});

function secondaryKeys(overrides: Partial<SettingsSecondaryKeys> = {}): SettingsSecondaryKeys {
  return {
    services: 'ocr',
    ...overrides,
  };
}

function noopSetters() {
  return {
    services: vi.fn(),
  };
}
