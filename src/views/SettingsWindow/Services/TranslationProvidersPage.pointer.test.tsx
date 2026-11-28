// @vitest-environment happy-dom

import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useProviderStore } from '../../../stores/providerStore';
import { TranslationProvidersPage } from './TranslationProvidersPage';
import { OcrProvidersPage } from './OcrProvidersPage';

vi.mock('./ProviderConfigDialog', () => ({ ProviderConfigDialog: () => null }));
vi.mock('./CustomTranslationProviderDialog', () => ({ CustomTranslationProviderDialog: () => null }));
vi.mock('../runtimeContext', () => ({ useSettingsRuntime: () => ({ providers: {} }) }));
vi.mock('../../../stores/settingsConfigStore', () => ({
  useSettingsConfigStore: (select: (state: unknown) => unknown) => select({ ocr: {} }),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

for (const kind of ['translation', 'ocr'] as const) {
  describe(`${kind} provider sorting`, () => {
    const ids = ['first', 'second', 'inactive', 'inactive-second'];
    const reorder = vi.fn(async (order: string[]) => {
      if (kind === 'translation') useProviderStore.setState({ activeTranslationProviders: order });
      else useProviderStore.setState((state) => ({ ocrProviders: order.map((id) => state.ocrProviders.find((provider) => provider.id === id)!) }));
    });
    const reorderInactive = vi.fn(async (order: string[]) => {
      useProviderStore.setState((state) => ({ translationProviders: [
        ...state.translationProviders.filter((provider) => state.activeTranslationProviders.includes(provider.id)),
        ...order.map((id) => state.translationProviders.find((provider) => provider.id === id)!),
      ] }));
    });
    let container: HTMLDivElement;
    let root: ReturnType<typeof createRoot>;

    beforeEach(async () => {
      vi.useFakeTimers();
      reorder.mockClear();
      reorderInactive.mockClear();
      vi.stubGlobal('alert', vi.fn());
      const providers = ids.map((id, index) => ({
        id, name: id, type: kind,
        status: (index < (kind === 'translation' ? 2 : 1) ? 'active' : 'inactive') as 'active' | 'inactive',
        isBuiltin: true, requiresApiKey: false,
      }));
      useProviderStore.setState({
        translationProviders: providers,
        ocrProviders: providers,
        activeTranslationProviders: ids.slice(0, 2),
        activeOcrProvider: 'first',
        loadTranslationProviders: vi.fn(async () => undefined),
        loadOcrProviders: vi.fn(async () => undefined),
        reorderTranslationProviders: reorder,
        reorderInactiveTranslationProviders: reorderInactive,
        reorderOcrProviders: reorder,
      });
      vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
        const index = this.dataset.providerSortId ? ids.indexOf(this.dataset.providerSortId) : 0;
        return new DOMRect(0, 100 + index * 78, 600, this.classList.contains('provider-sort-list') ? 304 : 70);
      });
      container = document.createElement('div');
      document.body.append(container);
      root = createRoot(container);
      await act(async () => root.render(kind === 'translation' ? <TranslationProvidersPage /> : <OcrProvidersPage />));
    });

    afterEach(async () => {
      await act(async () => root.unmount());
      container.remove();
      vi.useRealTimers();
      vi.restoreAllMocks();
      vi.unstubAllGlobals();
    });

    const handle = (index = 0) => {
      const element = container.querySelectorAll<HTMLElement>('[aria-label="拖动排序"]')[index];
      element.setPointerCapture = vi.fn();
      return element;
    };
    const headings = () => Array.from(container.querySelectorAll('h3'), (node) => node.textContent);
    const finishAnimation = async () => { await act(async () => vi.advanceTimersByTimeAsync(280)); };

    it('lifts the dragged card, moves its neighbour and saves only after drop', async () => {
      const grip = handle();
      await pointer(grip, 'pointerdown', 135);
      await pointer(grip, 'pointermove', 215);
      expect(reorder).not.toHaveBeenCalled();
      const dragged = container.querySelector<HTMLElement>('[data-sort-dragging]')!;
      expect(dragged.style.transform).toContain(kind === 'translation' ? '78px' : '80px');
      expect(container.querySelector<HTMLElement>('[data-provider-sort-id="second"]')!.style.transform).toContain('-78px');
      expect(container.querySelector('.provider-sort-placeholder')).toBeNull();
      await pointer(grip, 'pointerup', 215);
      expect(reorder).toHaveBeenCalledExactlyOnceWith(kind === 'translation' ? ['second', 'first'] : ['second', 'first', 'inactive', 'inactive-second']);
      await finishAnimation();
      expect(headings()).toEqual(['second', 'first', 'inactive', 'inactive-second']);
      expect(container.querySelector('[data-sort-dragging]')).toBeNull();
      expect(container.querySelector('.provider-sort-placeholder')).toBeNull();
      expect(useProviderStore.getState().activeOcrProvider).toBe('first');
      expect(grip.setPointerCapture).toHaveBeenCalledWith(1);
    });

    it.each(['pointercancel', 'lostpointercapture', 'Escape'])(
      'returns cards to their original positions on %s', async (event) => {
        const grip = handle();
        await pointer(grip, 'pointerdown', 135);
        await pointer(grip, 'pointermove', 215);
        if (event === 'Escape') await act(async () => { window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })); });
        else await pointer(grip, event, 215);
        await pointer(grip, 'pointerup', 215);
        await finishAnimation();
        expect(reorder).not.toHaveBeenCalled();
        expect(headings()).toEqual(ids);
        expect(container.querySelector('.provider-sort-placeholder')).toBeNull();
      },
    );

    it('ignores clicks, the right button and unrelated pointers', async () => {
      const grip = handle();
      await pointer(grip, 'pointerdown', 135, { button: 2 });
      expect(grip.setPointerCapture).not.toHaveBeenCalled();
      await pointer(grip, 'pointerdown', 135);
      await pointer(grip, 'pointermove', 215, { pointerId: 2 });
      await pointer(grip, 'pointerup', 215, { pointerId: 2 });
      await pointer(grip, 'pointerup', 136);
      await finishAnimation();
      expect(reorder).not.toHaveBeenCalled();
    });

    it('cancels a drop outside the list', async () => {
      const grip = handle();
      await pointer(grip, 'pointerdown', 135);
      await pointer(grip, 'pointermove', 215, { clientX: 700 });
      await pointer(grip, 'pointerup', 215, { clientX: 700 });
      await finishAnimation();
      expect(reorder).not.toHaveBeenCalled();
      expect(headings()).toEqual(ids);
    });

    it('supports the arrow keys without changing enabled providers', async () => {
      await act(async () => handle().dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'ArrowDown' })));
      expect(reorder).toHaveBeenCalledWith(kind === 'translation' ? ['second', 'first'] : ['second', 'first', 'inactive', 'inactive-second']);
      expect(useProviderStore.getState().activeOcrProvider).toBe('first');
    });

    it('restores the original order when saving fails', async () => {
      reorder.mockRejectedValueOnce(new Error('disk unavailable'));
      const grip = handle();
      await pointer(grip, 'pointerdown', 135);
      await pointer(grip, 'pointermove', 215);
      await pointer(grip, 'pointerup', 215);
      await act(async () => vi.advanceTimersByTimeAsync(520));
      expect(headings()).toEqual(ids);
      expect(container.querySelector('.provider-sort-placeholder')).toBeNull();
      expect(alert).toHaveBeenCalledWith('重新排序失败: Error: disk unavailable');
    });

    if (kind === 'translation') {
      it('keeps active and inactive translation providers in their own groups', async () => {
        const grip = handle();
        await pointer(grip, 'pointerdown', 135);
        await pointer(grip, 'pointermove', 295);
        await pointer(grip, 'pointerup', 295);
        await finishAnimation();
        expect(reorder).not.toHaveBeenCalled();
        expect(reorderInactive).not.toHaveBeenCalled();
        expect(headings()).toEqual(ids);
      });

      it('persists the inactive group order separately', async () => {
        const grip = handle(2);
        await pointer(grip, 'pointerdown', 291);
        await pointer(grip, 'pointermove', 371);
        await pointer(grip, 'pointerup', 371);
        await finishAnimation();
        expect(reorderInactive).toHaveBeenCalledExactlyOnceWith(['inactive-second', 'inactive']);
        expect(reorder).not.toHaveBeenCalled();
        expect(headings()).toEqual(['first', 'second', 'inactive-second', 'inactive']);
      });
    }
  });
}

async function pointer(target: HTMLElement, type: string, y: number, init: PointerEventInit = {}) {
  await act(async () => {
    target.dispatchEvent(new PointerEvent(type, {
      bubbles: true, cancelable: true, pointerId: 1, pointerType: 'mouse',
      button: 0, buttons: type === 'pointerup' ? 0 : 1, clientX: 8, clientY: y, ...init,
    }));
  });
}
