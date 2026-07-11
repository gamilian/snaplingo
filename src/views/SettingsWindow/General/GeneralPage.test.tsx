import type { ReactElement, ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

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

vi.mock('../../../stores/settingsConfigStore', () => ({
  useSettingsConfigStore: (selector: (state: typeof settingsConfig.state) => unknown) =>
    selector(settingsConfig.state),
}));

import { GeneralPage } from './GeneralPage';

describe('GeneralPage durable settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('reads and saves general settings through settingsConfigStore', () => {
    const view = GeneralPage();
    const selects = findElements(
      view,
      (element) => getElementName(element) === 'CustomSelect',
    );

    expect(selects[0].props.value).toBe('en');
    expect(selects[1].props.value).toBe('dark');

    selects[0].props.onChange?.('ja');
    selects[1].props.onChange?.('system');
    findElement(
      view,
      (element) => element.type === 'button' && typeof element.props.onClick === 'function',
    ).props.onClick?.();

    expect(settingsConfig.state.updateGeneralSettings).toHaveBeenNthCalledWith(1, {
      language: 'ja',
      theme: 'dark',
      startOnBoot: true,
    });
    expect(settingsConfig.state.updateGeneralSettings).toHaveBeenNthCalledWith(2, {
      language: 'en',
      theme: 'system',
      startOnBoot: true,
    });
    expect(settingsConfig.state.updateGeneralSettings).toHaveBeenNthCalledWith(3, {
      language: 'en',
      theme: 'dark',
      startOnBoot: false,
    });
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
  onChange?: (value: string) => unknown;
  onClick?: () => unknown;
  value?: string;
}>;
