import type { ReactElement, ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const reactState = vi.hoisted(() => ({
  useState: vi.fn((initialValue: unknown) => [initialValue, vi.fn()] as const),
}));

const settingsConfig = vi.hoisted(() => ({
  state: {
    screenshot: {
      savePath: '/captures',
      format: 'png',
      quality: 90,
    },
    updateScreenshotSettings: vi.fn(),
  },
}));

vi.mock('react', async () => {
  const actual = await vi.importActual<typeof import('react')>('react');
  return {
    ...actual,
    useState: reactState.useState,
  };
});

vi.mock('../../../stores/settingsConfigStore', () => ({
  useSettingsConfigStore: (selector: (state: typeof settingsConfig.state) => unknown) =>
    selector(settingsConfig.state),
}));

import { SaveSettingsPage } from './SaveSettingsPage';

describe('SaveSettingsPage durable settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('reads and saves screenshot settings through settingsConfigStore', () => {
    const view = SaveSettingsPage();
    const savePathInput = findElement(
      view,
      (element) => element.type === 'input' && element.props.type === 'text',
    );
    const jpgInput = findElement(
      view,
      (element) => element.type === 'input' && element.props.value === 'jpg',
    );
    const qualityRange = findElement(
      view,
      (element) => getElementName(element) === 'CustomRange',
    );

    expect(savePathInput.props.value).toBe('/captures');
    expect(jpgInput.props.checked).toBe(false);
    expect(qualityRange.props.value).toBe(90);

    savePathInput.props.onChange?.({ target: { value: '/next' } });
    jpgInput.props.onChange?.({ target: { value: 'jpg' } });
    qualityRange.props.onChange?.(82);

    expect(settingsConfig.state.updateScreenshotSettings).toHaveBeenNthCalledWith(1, {
      savePath: '/next',
      format: 'png',
      quality: 90,
    });
    expect(settingsConfig.state.updateScreenshotSettings).toHaveBeenNthCalledWith(2, {
      savePath: '/captures',
      format: 'jpg',
      quality: 90,
    });
    expect(settingsConfig.state.updateScreenshotSettings).toHaveBeenNthCalledWith(3, {
      savePath: '/captures',
      format: 'png',
      quality: 82,
    });
  });
});

function findElement(
  root: ReactNode,
  predicate: (element: PageElement) => boolean,
): PageElement {
  if (isElement(root) && predicate(root)) {
    return root;
  }

  for (const child of childNodes(root)) {
    const match = findElementOrNull(child, predicate);
    if (match) return match;
  }

  throw new Error('Element not found');
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

type ChangeEventLike = {
  target: {
    value: string;
  };
};

type PageElement = ReactElement<{
  checked?: boolean;
  children?: ReactNode;
  onChange?: (value: ChangeEventLike | number) => unknown;
  type?: string;
  value?: string | number;
}>;
