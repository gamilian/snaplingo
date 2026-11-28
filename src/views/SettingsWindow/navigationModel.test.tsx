import { describe, expect, it } from 'vitest';
import { findSettingsSection, settingsSections, type MainTab } from './navigationModel';

describe('settings navigation model', () => {
  it('describes the six primary destinations in sidebar order', () => {
    expect(settingsSections.slice(0, 6).map(({ key, label }) => ({ key, label }))).toEqual([
      { key: 'overview', label: '概览与快捷键' },
      { key: 'screenshot', label: '截图与贴图' },
      { key: 'translation', label: '翻译与识别' },
      { key: 'services', label: '引擎与语音' },
      { key: 'general', label: '通用设置' },
      { key: 'history', label: '资料库' },
    ]);
  });

  it('keeps the existing native OCR and favorites destinations accessible', () => {
    expect(findSettingsSection('ocr').render().type).toBe(findSettingsSection('translation').render().type);
    expect(findSettingsSection('favorites').label).toBe('资料库');
    expect(findSettingsSection('favorites').render().type).not.toBe(findSettingsSection('history').render().type);
  });

  it('falls back to overview when a destination is stale', () => {
    expect(findSettingsSection('removed-tab' as MainTab).key).toBe('overview');
  });
});
