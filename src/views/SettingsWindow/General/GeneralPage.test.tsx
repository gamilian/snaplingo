// @vitest-environment happy-dom

import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import type { ReactElement, ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const settingsConfig = vi.hoisted(() => ({
  state: {
    general: {
      language: 'en',
      theme: 'dark',
      startOnBoot: true,
    },
    updateGeneralSettings: vi.fn(),
  },
}));
const version = vi.hoisted(() => vi.fn(async () => '1.2.3'));

vi.mock('../../../stores/settingsConfigStore', () => ({
  useSettingsConfigStore: (selector: (state: typeof settingsConfig.state) => unknown) =>
    selector(settingsConfig.state),
}));

vi.mock('../../../stores/settingsStore', () => ({
  useSettingsStore: (selector: (state: {
    requestedSection: null;
    consumeRequestedSection: () => void;
  }) => unknown) =>
    selector({ requestedSection: null, consumeRequestedSection: vi.fn() }),
}));

vi.mock('../runtimeContext', () => ({
  useSettingsRuntime: () => ({
    window: { version },
  }),
}));

import { AppVersionValue, GeneralPage } from './GeneralPage';

describe('GeneralPage durable settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('reads and saves general settings through settingsConfigStore', () => {
    const view = GeneralPage();
    const sections = (view.props as {
      sections: { label: string; content: ReactNode }[];
    }).sections;
    expect(sections.map((section) => section.label)).toEqual([
      '界面与启动',
      '网络',
      '日志与维护',
      '实验性功能',
      '关于',
    ]);
    const interfaceContent = sections[0].content;
    const selects = findElements(
      interfaceContent,
      (element) => getElementName(element) === 'SelectField',
    );

    expect(selects[0].props.value).toBe('en');
    expect(selects[1].props.value).toBe('dark');

    selects[0].props.onChange?.('ja');
    selects[1].props.onChange?.('system');
    findElement(
      interfaceContent,
      (element) => getElementName(element) === 'SettingsToggle',
    ).props.onChange?.(false);

    expect(settingsConfig.state.updateGeneralSettings).toHaveBeenNthCalledWith(1, {
      language: 'ja',
    });
    expect(settingsConfig.state.updateGeneralSettings).toHaveBeenNthCalledWith(2, {
      theme: 'system',
    });
    expect(settingsConfig.state.updateGeneralSettings).toHaveBeenNthCalledWith(3, {
      startOnBoot: false,
    });
  });

  it('displays the packaged application version', async () => {
    const container = document.createElement('div');
    const root = createRoot(container);

    await act(async () => {
      root.render(createElement(AppVersionValue));
    });

    expect(container.textContent).toBe('1.2.3');
    expect(version).toHaveBeenCalled();
    act(() => root.unmount());
  });
});

function findElement(
  root: ReactNode,
  predicate: (element: PageElement) => boolean,
): PageElement {
  const match = findElementOrNull(root, predicate);
  if (!match) {
    throw new Error('Element not found');
  }

  return match;
}

function findElements(
  root: ReactNode,
  predicate: (element: PageElement) => boolean,
): PageElement[] {
  const matches: PageElement[] = [];

  walk(root, (element) => {
    if (predicate(element)) {
      matches.push(element);
    }
  });

  return matches;
}

function walk(root: ReactNode, visit: (element: PageElement) => void) {
  if (isElement(root)) {
    visit(root);
  }

  for (const child of childNodes(root)) {
    walk(child, visit);
  }
}

function findElementOrNull(
  root: ReactNode,
  predicate: (element: PageElement) => boolean,
): PageElement | null {
  if (isElement(root) && predicate(root)) {
    return root;
  }

  for (const child of childNodes(root)) {
    const match = findElementOrNull(child, predicate);
    if (match) return match;
  }

  return null;
}

function childNodes(node: ReactNode): ReactNode[] {
  if (Array.isArray(node)) {
    return node.flatMap(childNodes);
  }

  if (!isElement(node)) {
    return [];
  }

  const children = node.props.children;
  if (children === null || children === undefined) {
    return [];
  }

  return Array.isArray(children) ? children : [children];
}

function isElement(node: ReactNode): node is PageElement {
  return Boolean(node && typeof node === 'object' && 'props' in node);
}

function getElementName(element: PageElement): string {
  return typeof element.type === 'string'
    ? element.type
    : element.type.name || 'Anonymous';
}

type PageElement = ReactElement<{
  children?: ReactNode;
  onChange?: (value: string | boolean) => unknown;
  onClick?: () => unknown;
  value?: string;
}>;
