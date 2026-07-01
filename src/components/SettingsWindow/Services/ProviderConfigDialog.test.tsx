import type { ReactElement, ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const reactState = vi.hoisted(() => {
  const harness = {
    cursor: 0,
    effectCursor: 0,
    values: [] as unknown[],
    effectDeps: [] as unknown[][],
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
    useEffect: vi.fn((effect: () => void, deps?: unknown[]) => {
      const index = harness.effectCursor;
      harness.effectCursor += 1;
      const previousDeps = harness.effectDeps[index];
      const shouldRun =
        !deps ||
        !previousDeps ||
        deps.some((dep, depIndex) => !Object.is(dep, previousDeps[depIndex]));

      if (shouldRun) {
        harness.effectDeps[index] = deps || [];
        effect();
      }
    }),
  };

  return harness;
});

const providerApi = vi.hoisted(() => ({
  getProviderCredentialSchema: vi.fn(),
}));

vi.mock('react', async () => {
  const actual = await vi.importActual<typeof import('react')>('react');
  return {
    ...actual,
    useState: reactState.useState,
    useEffect: reactState.useEffect,
  };
});

vi.mock('../../../tauri/providers', () => providerApi);

import { ProviderConfigDialog } from './ProviderConfigDialog';

describe('ProviderConfigDialog', () => {
  beforeEach(() => {
    reactState.cursor = 0;
    reactState.effectCursor = 0;
    reactState.values.length = 0;
    reactState.effectDeps.length = 0;
    reactState.useState.mockClear();
    reactState.useEffect.mockClear();
    providerApi.getProviderCredentialSchema.mockReset();
  });

  it('saves DeepLX endpoint credentials by default', async () => {
    const onClose = vi.fn();
    const onSave = vi.fn();
    const props = createDeepLXDialogProps({ onClose, onSave });

    let view = renderDialog(props);
    await settleCredentialLoading();
    view = renderDialog(props);

    changeInput(
      findInputByPlaceholder(view, '例如：https://deeplx.example.com'),
      ' https://deeplx.example.test ',
    );

    view = renderDialog(props);
    clickButton(findButtonByText(view, '保存配置'));

    expect(onSave).toHaveBeenCalledWith({
      mode: 'deeplx',
      endpoint: 'https://deeplx.example.test',
    });
    expect(onClose).toHaveBeenCalled();
  });

  it('saves standard DeepL credentials when the switch is enabled', async () => {
    const onClose = vi.fn();
    const onSave = vi.fn();
    const props = createDeepLXDialogProps({ onClose, onSave });

    let view = renderDialog(props);
    await settleCredentialLoading();
    view = renderDialog(props);

    clickButton(findSwitch(view));
    view = renderDialog(props);
    changeInput(findInputByPlaceholder(view, '请输入 DeepL API Key'), ' deepl-key ');

    view = renderDialog(props);
    clickButton(findButtonByText(view, '保存配置'));

    expect(onSave).toHaveBeenCalledWith({
      mode: 'deepl',
      api_key: 'deepl-key',
    });
    expect(onClose).toHaveBeenCalled();
  });
});

function createDeepLXDialogProps({
  onClose,
  onSave,
}: {
  onClose: () => void;
  onSave: (credentials: Record<string, string>) => void;
}): Parameters<typeof ProviderConfigDialog>[0] {
  return {
    isOpen: true,
    onClose,
    onSave,
    provider: {
      id: 'deeplx',
      name: 'DeepLX',
      type: 'translation',
      status: 'unconfigured',
      isBuiltin: true,
      description: 'DeepLX / DeepL translation',
      requiresApiKey: true,
    },
    loadCredentialSchema: vi.fn().mockResolvedValue([
      { name: 'mode', label: '模式', secret: false },
      { name: 'endpoint', label: 'DeepLX API 地址', secret: false },
      { name: 'api_key', label: 'DeepL API Key', secret: true },
    ]),
  };
}

function renderDialog(
  props: Parameters<typeof ProviderConfigDialog>[0],
): DialogElement {
  reactState.cursor = 0;
  reactState.effectCursor = 0;
  const view = ProviderConfigDialog(props);

  if (!isElement(view)) {
    throw new Error('Dialog did not render an element');
  }

  return view;
}

async function settleCredentialLoading() {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

function changeInput(element: DialogElement, value: string) {
  if (!element.props.onChange) {
    throw new Error('Input does not have an onChange handler');
  }

  element.props.onChange({ target: { value } });
}

function clickButton(element: DialogElement) {
  if (!element.props.onClick) {
    throw new Error('Button does not have an onClick handler');
  }

  element.props.onClick();
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

function findSwitch(root: DialogElement): DialogElement {
  return findElement(root, (element) => element.props.role === 'switch');
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

  throw new Error(`Element not found in: ${textContent(root)}`);
}

function findElementOrNull(
  root: ReactNode,
  predicate: (element: DialogElement) => boolean,
): DialogElement | null {
  if (isElement(root) && predicate(root)) {
    return root;
  }

  if (isFunctionComponent(root)) {
    const match = findElementOrNull(root.type(root.props), predicate);
    if (match) return match;
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
    if (isFunctionComponent(node)) {
      return textContent(node.type(node.props));
    }

    return textContent(node.props.children);
  }

  return '';
}

function isElement(node: ReactNode): node is DialogElement {
  return Boolean(node && typeof node === 'object' && 'props' in node);
}

function isFunctionComponent(
  node: ReactNode,
): node is DialogElement & { type: (props: DialogElement['props']) => ReactNode } {
  return isElement(node) && typeof node.type === 'function';
}

type DialogElement = ReactElement<{
  children?: ReactNode;
  onChange?: (event: { target: { value: string } }) => void;
  onClick?: () => void;
  placeholder?: string;
  role?: string;
}>;
