import type { ReactElement, ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { ProviderCard } from './ProviderCard';

describe('ProviderCard', () => {
  it('renders icon-only actions through the shared tooltip button component', () => {
    const view = ProviderCard({
      provider: {
        id: 'custom-gpt',
        name: 'gpt-5-mini',
        type: 'translation',
        status: 'active',
        isBuiltin: false,
        requiresApiKey: true,
      },
      onConfigure: vi.fn(),
      onTest: vi.fn(),
      onRemove: vi.fn(),
    });

    const tooltipButtons = findElements(
      view,
      (element) => getElementName(element) === 'IconActionButton',
    );

    expect(tooltipButtons).toHaveLength(3);
    expect(tooltipButtons.map((button) => button.props.title)).toEqual([
      '编辑',
      '测试联通',
      '删除',
    ]);
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
  title?: string;
}>;
