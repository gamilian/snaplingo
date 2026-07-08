import type { ReactElement, ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const settingsConfig = vi.hoisted(() => ({
  state: {
    translation: {
      defaultSourceLang: 'auto',
      defaultTargetLang: 'zh-CN',
    },
    updateTranslationSettings: vi.fn(),
  },
}));

vi.mock('../../../stores/settingsConfigStore', () => ({
  useSettingsConfigStore: (selector: (state: typeof settingsConfig.state) => unknown) =>
    selector(settingsConfig.state),
}));

import { TranslationSettingsPage } from './TranslationSettingsPage';

describe('TranslationSettingsPage durable settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('reads and saves translation defaults through settingsConfigStore', () => {
    const view = TranslationSettingsPage();
    const selects = findElements(
      view,
      (element) => getElementName(element) === 'CustomSelect',
    );

    expect(selects[0].props.value).toBe('auto');
    expect(selects[1].props.value).toBe('zh-CN');

    selects[0].props.onChange?.('ja');
    selects[1].props.onChange?.('en');

    expect(settingsConfig.state.updateTranslationSettings).toHaveBeenNthCalledWith(1, {
      defaultSourceLang: 'ja',
      defaultTargetLang: 'zh-CN',
    });
    expect(settingsConfig.state.updateTranslationSettings).toHaveBeenNthCalledWith(2, {
      defaultSourceLang: 'auto',
      defaultTargetLang: 'en',
    });
  });
});

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
  value?: string;
}>;
