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

const reactDomState = vi.hoisted(() => ({
  createPortal: vi.fn(
    (children: ReactNode, _container: Element | DocumentFragment) => children,
  ),
}));

const providerApi = vi.hoisted(() => ({
  listAnthropicModels: vi.fn(),
  listGeminiModels: vi.fn(),
  listOpenAICompatibleModels: vi.fn(),
  listTranslationPromptStrategies: vi.fn(),
  saveTranslationPromptStrategies: vi.fn(),
  testAnthropicProvider: vi.fn(),
  testGeminiProvider: vi.fn(),
  testOpenAICompatibleProvider: vi.fn(),
  testOpenAIResponsesProvider: vi.fn(),
}));

vi.mock('react', async () => {
  const actual = await vi.importActual<typeof import('react')>('react');
  return {
    ...actual,
    useState: reactState.useState,
    useEffect: reactState.useEffect,
  };
});

vi.mock('react-dom', async () => {
  const actual = await vi.importActual<typeof import('react-dom')>('react-dom');
  return {
    ...actual,
    createPortal: reactDomState.createPortal,
  };
});

vi.mock('../../../tauri/providers', () => providerApi);

import { CustomTranslationProviderDialog } from './CustomTranslationProviderDialog';

describe('CustomTranslationProviderDialog', () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, 'document', {
      value: { body: {} as HTMLElement },
      configurable: true,
    });
    reactState.cursor = 0;
    reactState.effectCursor = 0;
    reactState.values.length = 0;
    reactState.effectDeps.length = 0;
    reactState.useState.mockClear();
    reactState.useEffect.mockClear();
    reactDomState.createPortal.mockClear();
    Object.values(providerApi).forEach((fn) => fn.mockReset());
    providerApi.listTranslationPromptStrategies.mockResolvedValue({
      strategies: [
        {
          id: 'general',
          name: '通用翻译',
          description: '适合大多数普通文本。',
          system_prompt: 'Translate to {target_lang}',
          is_builtin: true,
          is_deletable: false,
        },
      ],
    });
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
      prompt_strategy_id: 'smart',
      prompt_fallback_strategy_id: 'general',
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

  it('edits custom providers with the full custom provider form', async () => {
    const onClose = vi.fn();
    const onSave = vi.fn();
    const onUpdate = vi.fn().mockResolvedValue(undefined);
    const props = {
      isOpen: true,
      onClose,
      onSave,
      onUpdate,
      initialProvider: {
        id: 'custom-gpt',
        name: 'gpt-5-mini',
        type: 'translation',
        status: 'active',
        isBuiltin: false,
        requiresApiKey: true,
        protocol: 'openai',
        endpoint: 'https://api.openai.com',
        model: 'gpt-5-mini',
        reasoningLevel: 'minimal',
      },
    } as const;

    const view = renderDialog(props);

    expect(textContent(view)).toContain('配置 gpt-5-mini');
    expect(findInputByPlaceholder(view, '例如：我的 GPT-4').props.value).toBe(
      'gpt-5-mini',
    );
    expect(findInputByPlaceholder(view, 'gpt-4o / claude-3-5-sonnet-latest / gemini-1.5-flash').props.value).toBe(
      'gpt-5-mini',
    );

    const saveButton = findButtonByText(view, '保存配置');
    expect(saveButton.props.disabled).toBe(false);
    await clickButton(saveButton);

    expect(onUpdate).toHaveBeenCalledWith('custom-gpt', {
      name: 'gpt-5-mini',
      protocol: 'openai',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      api_key: undefined,
      reasoning_level: 'minimal',
      prompt_strategy_id: 'smart',
      prompt_fallback_strategy_id: 'general',
    });
    expect(onSave).not.toHaveBeenCalled();
  });

  it('uses styled controls instead of native selects for protocol and reasoning', () => {
    const view = renderDialog({
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn(),
    });

    expect(findElements(view, (element) => element.type === 'select')).toHaveLength(0);
    expect(findButtonByText(view, 'OpenAI')).toBeTruthy();
    expect(findButtonByTitle(view, 'OpenAI Responses')).toBeNull();
    expect(findButtonByText(view, 'Chat Completions')).toBeTruthy();
    expect(findButtonByText(view, 'Responses')).toBeTruthy();
    expect(findButtonByText(view, 'Minimal')).toBeTruthy();
  });

  it('does not show a fallback selector because smart strategy falls back to general translation', () => {
    const view = renderDialog({
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn(),
    });

    expect(textContent(view)).not.toContain('智能选择兜底策略');
  });

  it('places translation strategy controls after reasoning controls', () => {
    const view = renderDialog({
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn(),
    });

    const content = textContent(view);
    expect(content.indexOf('Reasoning 强度')).toBeLessThan(
      content.indexOf('翻译策略'),
    );
  });

  it('keeps OpenAI Responses as an OpenAI mode when editing', async () => {
    const onClose = vi.fn();
    const onSave = vi.fn();
    const onUpdate = vi.fn().mockResolvedValue(undefined);
    const props = {
      isOpen: true,
      onClose,
      onSave,
      onUpdate,
      initialProvider: {
        id: 'custom-gpt',
        name: 'gpt-5-mini',
        type: 'translation',
        status: 'active',
        isBuiltin: false,
        requiresApiKey: true,
        protocol: 'openai-responses',
        endpoint: 'https://api.openai.com',
        model: 'gpt-5-mini',
      },
    } as const;

    const view = renderDialog(props);
    const saveButton = findButtonByText(view, '保存配置');
    await clickButton(saveButton);

    expect(onUpdate).toHaveBeenCalledWith('custom-gpt', {
      name: 'gpt-5-mini',
      protocol: 'openai-responses',
      endpoint: 'https://api.openai.com',
      model: 'gpt-5-mini',
      api_key: undefined,
      reasoning_level: undefined,
      prompt_strategy_id: 'smart',
      prompt_fallback_strategy_id: 'general',
    });
  });

  it('renders through a body portal so the overlay covers the full settings window', () => {
    renderDialog({
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn(),
    });

    expect(reactDomState.createPortal).toHaveBeenCalledTimes(1);
    expect(reactDomState.createPortal.mock.calls[0][1]).toBe(document.body);
  });

  it('does not show explanatory copy under reasoning strength', () => {
    const view = renderDialog({
      isOpen: true,
      onClose: vi.fn(),
      onSave: vi.fn(),
    });

    expect(textContent(view)).not.toContain('仅支持推理模型');
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
  reactState.effectCursor = 0;
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

function findButtonByTitle(root: DialogElement, title: string): DialogElement | null {
  return findElementOrNull(root, (element) => {
    return element.type === 'button' && element.props.title === title;
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

function findElements(
  root: ReactNode,
  predicate: (element: DialogElement) => boolean,
): DialogElement[] {
  const matches: DialogElement[] = [];

  if (isElement(root) && predicate(root)) {
    matches.push(root);
  }

  for (const child of childNodes(root)) {
    matches.push(...findElements(child, predicate));
  }

  return matches;
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
  title?: string;
  value?: string;
}>;
