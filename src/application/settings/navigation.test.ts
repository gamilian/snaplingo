import { describe, expect, it } from 'vitest';
import {
  parseSettingsNavigationRequest,
  readSettingsNavigationLaunch,
} from './navigation';

describe('settings navigation requests', () => {
  it('reads an initial section route from the settings window URL', () => {
    expect(
      readSettingsNavigationLaunch('?window=settings&tab=general&section=about'),
    ).toEqual({ tab: 'general', section: 'about' });
  });

  it.each([
    null,
    {},
    { tab: 'advanced' },
    { tab: 'general', section: 42 },
  ])('rejects malformed navigation payload %j', (payload) => {
    expect(parseSettingsNavigationRequest(payload)).toBeNull();
  });
});
