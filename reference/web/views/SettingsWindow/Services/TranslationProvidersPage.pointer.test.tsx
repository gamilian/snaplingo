// @vitest-environment happy-dom

import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useProviderStore } from '../../../stores/providerStore';
import { TranslationProvidersPage } from './TranslationProvidersPage';

vi.mock('./ProviderConfigDialog', () => ({ ProviderConfigDialog: () => null }));
vi.mock('./CustomTranslationProviderDialog', () => ({
  CustomTranslationProviderDialog: () => null,
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe('translation provider pointer sorting', () => {
  const reorder = vi.fn(async (ids: string[]) => {
    useProviderStore.setState({ activeTranslationProviders: ids });
  });
  let container: HTMLDivElement;
  let root: ReturnType<typeof createRoot>;

  beforeEach(async () => {
    reorder.mockClear();
    useProviderStore.setState({
      translationProviders: ['first', 'second', 'inactive'].map((id) => ({
        id,
        name: id,
        type: 'translation',
        status: id === 'inactive' ? 'inactive' : 'active',
        isBuiltin: true,
        requiresApiKey: false,
      })),
      activeTranslationProviders: ['first', 'second'],
      loadTranslationProviders: vi.fn(async () => undefined),
      reorderTranslationProviders: reorder,
      reorderInactiveTranslationProviders: vi.fn(async () => undefined),
    });
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(<TranslationProvidersPage />));
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    container.remove();
    vi.restoreAllMocks();
  });

  it('saves and displays the new order using pointer events without native HTML drag events', async () => {
    const handle = container.querySelector<HTMLElement>('[aria-label="拖动排序"]')!;
    handle.setPointerCapture = vi.fn();
    const target = container.querySelectorAll('h3')[1];
    vi.spyOn(document, 'elementFromPoint').mockReturnValue(target);

    await pointer(handle, 'pointerdown');
    await pointer(handle, 'pointermove');
    expect(reorder).not.toHaveBeenCalled();
    await pointer(handle, 'pointerup');

    expect(reorder).toHaveBeenCalledExactlyOnceWith(['second', 'first']);
    expect(handle.setPointerCapture).toHaveBeenCalledWith(1);
    expect(Array.from(container.querySelectorAll('h3'), (node) => node.textContent))
      .toEqual(['second', 'first', 'inactive']);
  });

  it.each(['pointercancel', 'lostpointercapture'])(
    'cancels sorting on %s', async (type) => {
      const handle = container.querySelector<HTMLElement>('[aria-label="拖动排序"]')!;
      handle.setPointerCapture = vi.fn();
      vi.spyOn(document, 'elementFromPoint')
        .mockReturnValue(container.querySelectorAll('h3')[1]);

      await pointer(handle, 'pointerdown');
      await pointer(handle, 'pointermove');
      await pointer(handle, type);
      await pointer(handle, 'pointerup');

      expect(reorder).not.toHaveBeenCalled();
      expect(container.querySelector('.opacity-50')).toBeNull();
      expect(container.querySelector('.ring-2')).toBeNull();
    },
  );

  it.each(['same', 'inactive', 'outside'])(
    'does not save when released over %s', async (target) => {
      const handle = container.querySelector<HTMLElement>('[aria-label="拖动排序"]')!;
      handle.setPointerCapture = vi.fn();
      const headings = container.querySelectorAll('h3');
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(
        target === 'outside' ? null : headings[target === 'same' ? 0 : 2],
      );

      await pointer(handle, 'pointerdown');
      await pointer(handle, 'pointermove');
      await pointer(handle, 'pointerup');

      expect(reorder).not.toHaveBeenCalled();
      expect(container.querySelectorAll('[aria-label="拖动排序"]')).toHaveLength(3);
    },
  );

  it('ignores the right button and pointer events from another pointer', async () => {
    const handle = container.querySelector<HTMLElement>('[aria-label="拖动排序"]')!;
    handle.setPointerCapture = vi.fn();
    vi.spyOn(document, 'elementFromPoint')
      .mockReturnValue(container.querySelectorAll('h3')[1]);

    await pointer(handle, 'pointerdown', { button: 2 });
    await pointer(handle, 'pointerup');
    expect(handle.setPointerCapture).not.toHaveBeenCalled();
    expect(reorder).not.toHaveBeenCalled();

    await pointer(handle, 'pointerdown');
    await pointer(handle, 'pointerup', { pointerId: 2 });
    expect(reorder).not.toHaveBeenCalled();
    await pointer(handle, 'pointerup');
    expect(reorder).toHaveBeenCalledExactlyOnceWith(['second', 'first']);
  });
});

async function pointer(target: HTMLElement, type: string, init: PointerEventInit = {}) {
  await act(async () => {
    target.dispatchEvent(new PointerEvent(type, {
      bubbles: true,
      cancelable: true,
      pointerId: 1,
      pointerType: 'mouse',
      button: 0,
      buttons: type === 'pointerup' ? 0 : 1,
      clientX: 30,
      clientY: 120,
      ...init,
    }));
  });
}
