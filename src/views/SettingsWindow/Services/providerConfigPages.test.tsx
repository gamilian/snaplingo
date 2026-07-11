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
    ocrProviders: [
      {
        id: 'baidu-ocr',
        name: 'Baidu OCR',
        type: 'ocr',
        status: 'unconfigured',
        isBuiltin: true,
        requiresApiKey: true,
      },
    ],
    activeOcrProvider: null,
    loadOcrProviders: vi.fn(),
    activateOcrProvider: vi.fn(),
    configureOcrProvider: vi.fn(),
    ttsProviders: [
      {
        id: 'system-tts',
        name: 'System TTS',
        type: 'tts',
        status: 'unconfigured',
        isBuiltin: true,
        requiresApiKey: true,
      },
    ],
    activeTtsProvider: null,
    activateTtsProvider: vi.fn(),
    updateProviderConfig: vi.fn(),
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

vi.mock('../runtimeContext', () => ({
  useSettingsRuntime: () => ({
    providers: {
      getOcrCredentialSchema: vi.fn(),
    },
  }),
}));

import { OcrProvidersPage } from './OcrProvidersPage';
import { TtsProvidersPage } from './TtsProvidersPage';

describe('provider config pages', () => {
  beforeEach(() => {
    reactState.cursor = 0;
    reactState.values.length = 0;
    reactState.useState.mockClear();
    reactState.useEffect.mockClear();
  });

  it('opens OCR provider configuration inline instead of behind a black modal overlay', () => {
    let view = renderPage(OcrProvidersPage);
    findProviderCard(view).props.onConfigure?.();

    view = renderPage(OcrProvidersPage);

    expect(findProviderConfig(view).props.presentation).toBe('inline');
  });

  it('opens TTS provider configuration inline instead of behind a black modal overlay', () => {
    let view = renderPage(TtsProvidersPage);
    findProviderCard(view).props.onConfigure?.();

    view = renderPage(TtsProvidersPage);

    expect(findProviderConfig(view).props.presentation).toBe('inline');
  });
});

function renderPage(Component: () => ReactNode): PageElement {
  reactState.cursor = 0;
  const view = Component();

  if (!isElement(view)) {
    throw new Error('Page did not render an element');
  }

  return view;
}

function findProviderCard(root: ReactNode): PageElement {
  return findElement(root, (element) => getElementName(element) === 'ProviderCard');
}

function findProviderConfig(root: ReactNode): PageElement {
  return findElement(
    root,
    (element) => getElementName(element) === 'ProviderConfigDialog',
  );
}

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

type PageElement = ReactElement<{
  children?: ReactNode;
  onConfigure?: () => unknown;
  presentation?: string;
}>;
