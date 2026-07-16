import { describe, expect, it } from 'vitest';

import { uiText } from './uiText';

describe('settings UI text', () => {
  it('uses the selected language and falls back to simplified Chinese', () => {
    expect(uiText('en', 'general')).toBe('General');
    expect(uiText('ja', 'logRetentionDays')).toBe('ログ保持日数');
    expect(uiText('unsupported', 'general')).toBe('通用');
  });
});
