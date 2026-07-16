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
    useEffect: vi.fn((effect: () => void) => {
      effect();
    }),
  };

  return harness;
});

const providerStore = vi.hoisted(() => ({
  state: {
    translationProviders: [
      {
        id: 'google-translate',
        name: 'Google Translate',
        type: 'translation',
        status: 'active',
        isBuiltin: true,
        requiresApiKey: false,
      },
      {
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
    ],
    activeTranslationProviders: ['google-translate', 'custom-gpt'],
    loadTranslationProviders: vi.fn(),
    activateTranslationProvider: vi.fn(),
    deactivateTranslationProvider: vi.fn(),
    updateProviderConfig: vi.fn(),
    addCustomTranslationProvider: vi.fn(),
    updateCustomTranslationProvider: vi.fn(),
    removeTranslationProvider: vi.fn(),
    testCustomTranslationProvider: vi.fn(),
    reorderTranslationProviders: vi.fn(),
  },
}));

vi.mock('react', async () => {
  const actual = await vi.importActual<typeof import('react')>('react');
  return {
    ...actual,
    useState: reactState.useState,
    useEffect: reactState.useEffect,
  };
});

vi.mock('../../../stores/providerStore', () => ({
  useProviderStore: (selector: (state: typeof providerStore.state) => unknown) =>
    selector(providerStore.state),
}));

import { TranslationProvidersPage } from './TranslationProvidersPage';

describe('TranslationProvidersPage', () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, 'alert', {
      value: vi.fn(),
      configurable: true,
    });
    reactState.cursor = 0;
    reactState.values.length = 0;
    reactState.useState.mockClear();
    reactState.useEffect.mockClear();
    providerStore.state.loadTranslationProviders.mockClear();
    providerStore.state.testCustomTranslationProvider.mockClear();
    providerStore.state.reorderTranslationProviders.mockClear();
  });

  it('writes active provider id to drag data when dragging starts', () => {
    const view = renderPage();
    const draggableCards = findElements(
      view,
      (element) => element.props.draggable === true,
    );
    const dataTransfer = {
      effectAllowed: '',
      setData: vi.fn(),
    };

    draggableCards[0].props.onDragStart({ dataTransfer });

    expect(dataTransfer.effectAllowed).toBe('move');
    expect(dataTransfer.setData).toHaveBeenCalledWith(
      'text/plain',
      'google-translate',
    );
  });

  it('renders a hover-visible drag handle for active provider rows', () => {
    const view = renderPage();
    const providerCards = findElements(
      view,
      (element) => getElementName(element) === 'ProviderCard',
    );

    expect(providerCards).toHaveLength(2);
    expect(providerCards[0].props.leadingSlot?.props.className).toContain('opacity-0');
    expect(providerCards[0].props.leadingSlot?.props.className).toContain('group-hover:opacity-100');
  });

  it('places the add custom provider action in the toolbar with the primary color', () => {
    const view = renderPage();
    const addButton = findElement(
      view,
      (element) =>
        getElementName(element) === 'button' &&
        element.props['aria-label'] === '添加自定义服务',
    );

    expect(addButton.props.className).toContain('inline-flex');
    expect(addButton.props.className).toContain('absolute');
    expect(addButton.props.className).toContain('right-0');
    expect(addButton.props.className).toContain('top-1');
    expect(addButton.props.className).toContain('bg-primary-600');
    expect(addButton.props.children).toContainEqual(
      expect.objectContaining({ props: expect.objectContaining({ children: '添加自定义服务' }) }),
    );
  });

  it('reorders active providers when a dragged provider is dropped on another active provider', async () => {
    let view = renderPage();
    let draggableCards = findElements(
      view,
      (element) => element.props.draggable === true,
    );
    const dataTransfer = {
      effectAllowed: '',
      setData: vi.fn(),
    };

    draggableCards[0].props.onDragStart({ dataTransfer });

    view = renderPage();
    draggableCards = findElements(
      view,
      (element) => element.props.draggable === true,
    );
    await draggableCards[1].props.onDrop({ preventDefault: vi.fn() });

    expect(providerStore.state.reorderTranslationProviders).toHaveBeenCalledWith([
      'custom-gpt',
      'google-translate',
    ]);
  });

  it('opens the full custom provider form when configuring a custom provider', () => {
    let view = renderPage();
    const customProviderCard = findElement(
      view,
      (element) =>
        getElementName(element) === 'ProviderCard' &&
        element.props.provider?.id === 'custom-gpt',
    );

    customProviderCard.props.onConfigure();
    view = renderPage();

    const customDialog = findElement(
      view,
      (element) => getElementName(element) === 'CustomTranslationProviderDialog',
    );
    const genericDialogs = findElements(
      view,
      (element) => getElementName(element) === 'ProviderConfigDialog',
    );

    expect(customDialog.props.isOpen).toBe(true);
    expect(customDialog.props.presentation).toBe('inline');
    expect(customDialog.props.initialProvider?.id).toBe('custom-gpt');
    expect(genericDialogs).toHaveLength(0);
    expect(
      findElements(view, (element) => getElementName(element) === 'ProviderCard'),
    ).toHaveLength(0);
  });

  it('opens custom provider creation as an inline page instead of a modal over the list', () => {
    let view = renderPage();
    const addButton = findElement(
      view,
      (element) =>
        getElementName(element) === 'button' &&
        element.props['aria-label'] === '添加自定义服务',
    );

    addButton.props.onClick();
    view = renderPage();

    const customForm = findElement(
      view,
      (element) => getElementName(element) === 'CustomTranslationProviderDialog',
    );

    expect(customForm.props.isOpen).toBe(true);
    expect(customForm.props.presentation).toBe('inline');
    expect(
      findElements(view, (element) => getElementName(element) === 'ProviderCard'),
    ).toHaveLength(0);
  });

  it('tests custom provider connectivity from the hover action', async () => {
    const view = renderPage();
    const customProviderCard = findElement(
      view,
      (element) =>
        getElementName(element) === 'ProviderCard' &&
        element.props.provider?.id === 'custom-gpt',
    );

    if (!customProviderCard.props.onTest) {
      throw new Error('Expected custom provider to expose a test action');
    }

    await customProviderCard.props.onTest();

    expect(providerStore.state.testCustomTranslationProvider).toHaveBeenCalledWith('custom-gpt');
  });
});

function renderPage(): PageElement {
  reactState.cursor = 0;
  const view = TranslationProvidersPage();

  if (!isElement(view)) {
    throw new Error('TranslationProvidersPage did not render an element');
  }

  return view;
}

function findElements(
  root: ReactNode,
  predicate: (element: PageElement) => boolean,
): PageElement[] {
  const matches: PageElement[] = [];

  if (Array.isArray(root)) {
    for (const child of root) {
      matches.push(...findElements(child, predicate));
    }
    return matches;
  }

  if (!isElement(root)) {
    return matches;
  }

  if (predicate(root)) {
    matches.push(root);
  }

  for (const child of childNodes(root.props.children)) {
    matches.push(...findElements(child, predicate));
  }

  return matches;
}

function findElement(
  root: ReactNode,
  predicate: (element: PageElement) => boolean,
): PageElement {
  const match = findElements(root, predicate)[0];
  if (!match) {
    throw new Error('Element not found');
  }
  return match;
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

function isElement(node: ReactNode): node is PageElement {
  return (
    typeof node === 'object' &&
    node !== null &&
    'props' in node &&
    'type' in node
  );
}

function getElementName(element: PageElement): string {
  return typeof element.type === 'string'
    ? element.type
    : element.type.name || 'Anonymous';
}

type PageElement = ReactElement & {
  props: {
    children?: ReactNode;
    className?: string;
    draggable?: boolean;
    'aria-label'?: string;
    onDragStart?: (event: {
      dataTransfer: {
        effectAllowed: string;
        setData: (type: string, value: string) => void;
      };
    }) => void;
    onDrop?: (event: { preventDefault: () => void }) => Promise<void>;
    onClick?: () => unknown;
    onConfigure?: () => unknown;
    onTest?: () => Promise<void>;
    leadingSlot?: {
      props?: {
        className?: string;
      };
    };
    provider?: {
      id: string;
    };
    isOpen?: boolean;
    initialProvider?: {
      id: string;
    } | null;
  };
};
