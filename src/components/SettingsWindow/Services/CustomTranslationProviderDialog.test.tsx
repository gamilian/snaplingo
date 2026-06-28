import type { ReactElement, ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const reactState = vi.hoisted(() => {
  const harness = {
    cursor: 0,
    values: [] as unknown[],
    useState: vi.fn((initialValue: unknown) => {
      const index = harness.cursor;
      harness.cursor += 1;

      if (harness.values.length <= index) {
        harness.values.push(initialValue);
      }

      const setValue = (nextValue: unknown) => {
        harness.values[index] =
          typeof nextValue === 'function'
            ? (nextValue as (previous: unknown) => unknown)(harness.values[index])
            : nextValue;
      };

      return [harness.values[index], setValue] as const;
    }),
  };

  return harness;
});

vi.mock('react', async () => {
  const actual = await vi.importActual<typeof import('react')>('react');
  return {
    ...actual,
    useState: reactState.useState,
  };
});

import { CustomTranslationProviderDialog } from './CustomTranslationProviderDialog';

describe('CustomTranslationProviderDialog', () => {
  beforeEach(() => {
    reactState.cursor = 0;
    reactState.values.length = 0;
    reactState.useState.mockClear();
  });

  it('does not close while custom provider creation is still pending', () => {
    const onClose = vi.fn();
    const onSave = vi.fn(() => new Promise<void>(() => {}));
    const props = { isOpen: true, onClose, onSave };

    const view = fillValidOpenAIProvider(props);
    const addButton = findButtonByText(view, '添加');

    expect(addButton.props.disabled).toBe(false);

    clickButton(addButton);

    expect(onSave).toHaveBeenCalledWith({
      name: 'My OpenAI',
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-4o',
      api_key: 'sk-test',
      reasoning_level: undefined,
    });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('keeps the dialog open and shows the backend error when creation fails', async () => {
    const onClose = vi.fn();
    const failure = Promise.reject(new Error('keychain failed'));
    failure.catch(() => undefined);
    const onSave = vi.fn(() => failure);
    const props = { isOpen: true, onClose, onSave };

    let view = fillValidOpenAIProvider(props);
    const addButton = findButtonByText(view, '添加');
    const clickResult = clickButton(addButton);

    if (isPromiseLike(clickResult)) {
      await clickResult;
    }

    view = renderDialog(props);

    expect(onClose).not.toHaveBeenCalled();
    expect(textContent(view)).toContain('添加失败: keychain failed');
  });
});

function fillValidOpenAIProvider(
  props: Parameters<typeof CustomTranslationProviderDialog>[0],
): DialogElement {
  let view = renderDialog(props);
  changeInput(findInputByPlaceholder(view, '例如：我的 GPT-4'), 'My OpenAI');
  changeInput(findInputByPlaceholder(view, 'sk-...'), 'sk-test');

  view = renderDialog(props);
  return view;
}

function renderDialog(
  props: Parameters<typeof CustomTranslationProviderDialog>[0],
): DialogElement {
  reactState.cursor = 0;
  const view = CustomTranslationProviderDialog(props);

  if (!isElement(view)) {
    throw new Error('Dialog did not render an element');
  }

  return view;
}

function changeInput(element: DialogElement, value: string) {
  if (!element.props.onChange) {
    throw new Error('Input does not have an onChange handler');
  }

  element.props.onChange({ target: { value } });
}

function clickButton(element: DialogElement): unknown {
  if (!element.props.onClick) {
    throw new Error('Button does not have an onClick handler');
  }

  return element.props.onClick();
}

function findInputByPlaceholder(root: DialogElement, placeholder: string): DialogElement {
  return findElement(root, (element) => {
    return element.type === 'input' && element.props.placeholder === placeholder;
  });
}

function findButtonByText(root: DialogElement, text: string): DialogElement {
  return findElement(root, (element) => {
    return element.type === 'button' && textContent(element.props.children).trim() === text;
  });
}

function findElement(
  root: ReactNode,
  predicate: (element: DialogElement) => boolean,
): DialogElement {
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
  predicate: (element: DialogElement) => boolean,
): DialogElement | null {
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

function textContent(node: ReactNode): string {
  if (typeof node === 'string' || typeof node === 'number') {
    return String(node);
  }

  if (Array.isArray(node)) {
    return node.map(textContent).join('');
  }

  if (isElement(node)) {
    return textContent(node.props.children);
  }

  return '';
}

function isElement(node: ReactNode): node is DialogElement {
  return Boolean(node && typeof node === 'object' && 'props' in node);
}

function isPromiseLike(value: unknown): value is Promise<unknown> {
  return Boolean(
    value &&
      typeof value === 'object' &&
      'then' in value &&
      typeof value.then === 'function',
  );
}

type DialogElement = ReactElement<{
  children?: ReactNode;
  disabled?: boolean;
  onChange?: (event: { target: { value: string } }) => void;
  onClick?: () => unknown;
  placeholder?: string;
}>;
