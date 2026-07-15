import type { ReactElement, ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

const settingsConfig = vi.hoisted(() => ({
  state: {
    screenshot: {
      savePath: '/captures',
      format: 'png' as const,
      quality: 90,
      namingRule: 'timestamp' as const,
      customFileName: 'SnapLingo',
      autoCopy: false,
      defaultStrokeWidth: 2,
      defaultFontSize: 24,
      rememberLastTool: true,
      showSelectionSize: true,
      showMagnifier: false,
      pinOpacity: 100,
      pinShadow: true,
      annotationColors: [[255, 77, 79, 255]] as [number, number, number, number][],
      selectionBorderWidth: 2,
      selectionMaskColor: [0, 0, 0, 46] as [number, number, number, number],
    },
    translation: {
      defaultSourceLang: 'auto',
      defaultTargetLang: 'auto',
      autoTranslate: true,
      autoCopy: false,
      preserveLineBreaks: true,
      incrementalTranslation: false,
      windowAlwaysOnTop: true,
      hideOnBlur: true,
      selectionWindowPosition: 'below-cursor' as const,
      inputWindowPosition: 'center' as const,
      selectionInputState: 'last' as const,
      screenshotInputState: 'last' as const,
      maxWindowHeightRatio: 70,
      windowWidth: 660,
      selectionTextMode: 'smart' as const,
    },
    ocr: {
      recognitionLanguage: 'auto',
      preserveFormatting: true,
      removeChineseSpaces: true,
      showConfidence: false,
      windowPosition: 'cursor' as const,
      hideSilentStatus: false,
    },
    updateScreenshotSettings: vi.fn(),
    updateTranslationSettings: vi.fn(),
    updateOcrSettings: vi.fn(),
  },
}));

vi.mock('../../stores/settingsConfigStore', () => ({
  useSettingsConfigStore: (
    selector: (state: typeof settingsConfig.state) => unknown,
  ) => selector(settingsConfig.state),
}));

vi.mock('./runtimeContext', () => ({
  useSettingsRuntime: () => ({
    window: { selectScreenshotDirectory: vi.fn(async () => null) },
  }),
}));

import { OcrSettingsScrollPage } from './OCR/OcrSettingsScrollPage';
import { ScreenshotSettingsPage } from './Screenshot/ScreenshotSettingsPage';
import { TranslationSettingsScrollPage } from './Translation/TranslationSettingsScrollPage';

describe('single-scroll settings pages', () => {
  it('groups screenshot settings without editor-only defaults', () => {
    const sections = scrollSections(ScreenshotSettingsPage());

    expect(sections.map((section) => section.label)).toEqual([
      '快捷键',
      '保存与输出',
      '截图界面',
    ]);
    const labels = settingRowLabels(sections);
    expect(labels).toContain('边框宽度');
    expect(labels).toContain('遮罩颜色');
    expect(labels).toContain('遮罩透明度');
    expect(labels).not.toContain('标注颜色');
    expect(labels).not.toContain('默认线条粗细');
    expect(labels).not.toContain('默认字体大小');
  });

  it('groups translation settings and exposes automatic target language', () => {
    const sections = scrollSections(TranslationSettingsScrollPage());

    expect(sections.map((section) => section.label)).toEqual([
      '快捷键',
      '语言与取词',
      '窗口与输入框',
      '翻译行为',
    ]);
    const labels = settingRowLabels(sections);
    expect(labels).toContain('取词模式');
    expect(labels).toContain('最高屏幕占比');
    expect(labels).toContain('窗口宽度');

    const hotkeys = findElements(
      sections[0].content,
      (element) => elementName(element) === 'FeatureHotkeysSection',
    )[0];
    expect(hotkeys.props.actions?.map((action) => action.key)).toEqual([
      'selection-translate',
      'screenshot-translate',
      'show-window',
    ]);

    const targetLanguageRow = findElements(
      sections[1].content,
      (element) => element.props.label === '目标语言',
    )[0];
    const targetLanguageSelect = findElements(
      targetLanguageRow.props.children,
      (element) => elementName(element) === 'SelectField',
    )[0];
    expect(targetLanguageSelect.props.options?.[0]).toEqual({
      value: 'auto',
      label: '自动',
    });
  });

  it('groups OCR window and silent-status settings without auto-copy UI', () => {
    const sections = scrollSections(OcrSettingsScrollPage());

    expect(sections.map((section) => section.label)).toEqual([
      '快捷键',
      '识别与文本',
      '窗口与提示',
    ]);
    const labels = settingRowLabels(sections);
    expect(labels).toContain('OCR 窗口位置');
    expect(labels).toContain('隐藏识别状态提示');
    expect(labels).not.toContain('识别后自动复制');
  });
});

interface ScrollSection {
  label: string;
  content: ReactNode;
}

function scrollSections(view: ReactElement): ScrollSection[] {
  return (view.props as { sections: ScrollSection[] }).sections;
}

function settingRowLabels(sections: ScrollSection[]) {
  return sections.flatMap((section) =>
    findElements(
      section.content,
      (element) =>
        elementName(element) === 'SettingRow' ||
        elementName(element) === 'ToggleRow',
    ).map((element) => element.props.label),
  );
}

function findElements(
  root: ReactNode,
  predicate: (element: TestElement) => boolean,
): TestElement[] {
  const matches: TestElement[] = [];
  walk(root, (element) => {
    if (predicate(element)) matches.push(element);
  });
  return matches;
}

function walk(root: ReactNode, visit: (element: TestElement) => void) {
  if (isElement(root)) visit(root);
  for (const child of childNodes(root)) walk(child, visit);
}

function childNodes(node: ReactNode): ReactNode[] {
  if (Array.isArray(node)) return node.flatMap(childNodes);
  if (!isElement(node)) return [];
  const children = node.props.children;
  if (children === null || children === undefined) return [];
  return Array.isArray(children) ? children : [children];
}

function isElement(node: ReactNode): node is TestElement {
  return Boolean(node && typeof node === 'object' && 'props' in node);
}

function elementName(element: TestElement) {
  return typeof element.type === 'string'
    ? element.type
    : element.type.name || 'Anonymous';
}

type TestElement = ReactElement<{
  actions?: { key: string }[];
  children?: ReactNode;
  label?: string;
  options?: { value: string; label: string }[];
}>;
