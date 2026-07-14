// @vitest-environment happy-dom

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act } from 'react-dom/test-utils';
import { createRoot } from 'react-dom/client';
import type { ResultWindowRuntime } from '../../application/result-window/runtime';
import { useAppStore } from '../../stores/appStore';
import { useProviderStore } from '../../stores/providerStore';
import ResultWindow from './ResultWindow';

const runtime = {
  commands: {
    favoriteTranslationResult: vi.fn(),
    favoriteOcrResult: vi.fn(),
  },
  clipboard: { copyText: vi.fn() },
  resizeStandaloneWindow: vi.fn(),
  close: vi.fn(),
  startFileOcr: vi.fn(),
  translate: vi.fn(),
  retryTranslationProvider: vi.fn(),
} as unknown as ResultWindowRuntime;

async function renderResultWindow() {
  const container = document.createElement('div');
  const root = createRoot(container);
  await act(async () => {
    root.render(<ResultWindow runtime={runtime} presentation="standalone" />);
  });
  return {
    container,
    unmount: async () => {
      await act(async () => root.unmount());
    },
  };
}

describe('result window text actions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.getState().reset();
    useProviderStore.setState({
      translationProviders: [],
      activeTranslationProviders: [],
      loadTranslationProviders: vi.fn(async () => undefined),
    });
  });

  it('offers a session favorite action beside the translation source text actions', async () => {
    useAppStore.setState({
      resultWindowVisible: true,
      resultWindowMode: 'translation',
      sourceText: 'hello',
      sourceLang: 'en',
      targetLang: 'zh-CN',
      providerTranslations: [
        {
          provider_id: 'google',
          translated_text: '你好',
          detected_language: 'en',
          confidence: null,
          status: 'success',
        },
        {
          provider_id: 'deeplx',
          translated_text: '您好',
          detected_language: 'en',
          confidence: null,
          status: 'success',
        },
      ],
    });

    const view = await renderResultWindow();
    const favoriteButtons = view.container.querySelectorAll<HTMLButtonElement>(
      '[aria-label="收藏"]',
    );

    expect(favoriteButtons).toHaveLength(3);
    await act(async () => favoriteButtons[0].click());
    expect(runtime.commands.favoriteTranslationResult).toHaveBeenCalledTimes(2);
    await view.unmount();
  });

  it('offers speak, favorite, and copy actions for OCR text', async () => {
    useAppStore.setState({
      resultWindowVisible: true,
      resultWindowMode: 'ocr',
      ocrText: 'recognized text',
    });

    const view = await renderResultWindow();

    expect(view.container.querySelector('[aria-label="朗读"]')).not.toBeNull();
    expect(view.container.querySelector('[aria-label="收藏"]')).not.toBeNull();
    expect(view.container.querySelector('[aria-label="复制"]')).not.toBeNull();
    await view.unmount();
  });

  it('delegates translation workflow to the application runtime', async () => {
    useAppStore.setState({
      resultWindowVisible: true,
      resultWindowMode: 'translation',
      sourceText: 'hello',
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });
    const view = await renderResultWindow();
    const retryButton = view.container.querySelector<HTMLButtonElement>(
      '[aria-label="重试"]',
    );
    expect(retryButton).not.toBeNull();

    await act(async () => retryButton?.click());

    expect(runtime.translate).toHaveBeenCalledWith({
      text: 'hello',
      sourceLang: 'en',
      targetLang: 'zh-CN',
    });
    await view.unmount();
  });
});
