import type { ReactElement, ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { SecondaryNav } from './SecondaryNav';

describe('SecondaryNav', () => {
  it('can render service category choices as horizontal tabs', () => {
    const view = SecondaryNav({
      items: [
        { key: 'ocr', label: 'OCR 服务' },
        { key: 'translation', label: '翻译服务' },
        { key: 'tts', label: '语音合成' },
      ],
      activeItem: 'translation',
      onItemChange: vi.fn(),
      orientation: 'horizontal',
    });

    const root = asElement(view);
    expect(root.props.className).toContain('inline-flex');
    expect(root.props.className).not.toContain('w-[200px]');

    const nav = findElement(root, (element) => element.type === 'nav');
    expect(nav.props.className).toContain('flex');
    expect(nav.props.className).toContain('gap-1.5');

    const activeButton = findElement(
      root,
      (element) => element.type === 'button' && element.props.children === '翻译服务',
    );
    expect(activeButton.props.className).toContain('bg-primary-600');
    expect(activeButton.props.className).toContain('text-white');
  });
});

function findElement(
  root: ReactNode,
  predicate: (element: NavElement) => boolean,
): NavElement {
  if (Array.isArray(root)) {
    for (const child of root) {
      const match = findElementOrNull(child, predicate);
      if (match) return match;
    }
  }

  const match = findElementOrNull(root, predicate);
  if (!match) {
    throw new Error('Element not found');
  }

  return match;
}

function findElementOrNull(
  root: ReactNode,
  predicate: (element: NavElement) => boolean,
): NavElement | null {
  if (!isElement(root)) return null;
  if (predicate(root)) return root;

  for (const child of childNodes(root.props.children)) {
    const match = findElementOrNull(child, predicate);
    if (match) return match;
  }

  return null;
}

function childNodes(node: ReactNode): ReactNode[] {
  if (Array.isArray(node)) {
    return node.flatMap(childNodes);
  }
  if (node === null || node === undefined || typeof node === 'boolean') {
    return [];
  }
  return [node];
}

function asElement(node: ReactNode): NavElement {
  if (!isElement(node)) {
    throw new Error('Expected element');
  }
  return node;
}

function isElement(node: ReactNode): node is NavElement {
  return (
    typeof node === 'object' &&
    node !== null &&
    'props' in node &&
    'type' in node
  );
}

type NavElement = ReactElement & {
  props: {
    children?: ReactNode;
    className?: string;
  };
};
