import type { ReactElement } from 'react';
import type { SettingsNavigationTab } from '../../application/settings/navigation';

import { GeneralPage } from './General/GeneralPage';
import { OcrSettingsScrollPage } from './OCR/OcrSettingsScrollPage';
import { ScreenshotSettingsPage } from './Screenshot/ScreenshotSettingsPage';
import { OcrProvidersPage } from './Services/OcrProvidersPage';
import { TranslationProvidersPage } from './Services/TranslationProvidersPage';
import { TtsProvidersPage } from './Services/TtsProvidersPage';
import { TranslationSettingsScrollPage } from './Translation/TranslationSettingsScrollPage';
import { FavoritesLibraryPage } from './Library/FavoritesLibraryPage';
import { HistoryLibraryPage } from './Library/HistoryLibraryPage';

export type MainTab = SettingsNavigationTab;

export type ServicesSubTab = 'ocr' | 'translation' | 'tts';

export interface SecondaryNavItem<Key extends string = string> {
  key: Key;
  label: string;
  render: () => ReactElement;
}

interface SecondarySettingsSection<Key extends MainTab, SecondaryKey extends string> {
  key: Key;
  label: string;
  secondary: SecondaryNavItem<SecondaryKey>[];
}

interface SimpleSettingsSection<Key extends MainTab> {
  key: Key;
  label: string;
  render: () => ReactElement;
}

export type SettingsSection =
  | SecondarySettingsSection<'services', ServicesSubTab>
  | SimpleSettingsSection<'screenshot'>
  | SimpleSettingsSection<'translation'>
  | SimpleSettingsSection<'ocr'>
  | SimpleSettingsSection<'favorites'>
  | SimpleSettingsSection<'history'>
  | SimpleSettingsSection<'general'>;

export const settingsSections: SettingsSection[] = [
  {
    key: 'general',
    label: '通用',
    render: () => <GeneralPage />,
  },
  {
    key: 'screenshot',
    label: '截图',
    render: () => <ScreenshotSettingsPage />,
  },
  {
    key: 'translation',
    label: '翻译',
    render: () => <TranslationSettingsScrollPage />,
  },
  {
    key: 'ocr',
    label: 'OCR',
    render: () => <OcrSettingsScrollPage />,
  },
  {
    key: 'services',
    label: '服务',
    secondary: [
      { key: 'ocr', label: 'OCR 服务', render: () => <OcrProvidersPage /> },
      { key: 'translation', label: '翻译服务', render: () => <TranslationProvidersPage /> },
      { key: 'tts', label: '语音合成', render: () => <TtsProvidersPage /> },
    ],
  },
  {
    key: 'favorites',
    label: '收藏夹',
    render: () => <FavoritesLibraryPage />,
  },
  {
    key: 'history',
    label: '历史记录',
    render: () => <HistoryLibraryPage />,
  },
];

export function findSettingsSection(tab: MainTab): SettingsSection {
  return settingsSections.find((section) => section.key === tab) ?? settingsSections[0];
}

export function findSecondaryNavItem(
  section: SettingsSection,
  key: string,
): SecondaryNavItem | null {
  if (!('secondary' in section)) {
    return null;
  }

  return section.secondary.find((item) => item.key === key) ?? section.secondary[0] ?? null;
}

export function isServicesSubTab(key: string): key is ServicesSubTab {
  return hasSecondaryKey('services', key);
}

function hasSecondaryKey(tab: MainTab, key: string): boolean {
  const section = findSettingsSection(tab);
  return 'secondary' in section && section.secondary.some((item) => item.key === key);
}
