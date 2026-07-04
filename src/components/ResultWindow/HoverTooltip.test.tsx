import { renderToStaticMarkup } from 'react-dom/server';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const reactDomState = vi.hoisted(() => ({
  createPortal: vi.fn(
    (children: React.ReactNode, _container: Element | DocumentFragment) => children,
  ),
}));

vi.mock('react-dom', async () => {
  const actual = await vi.importActual<typeof import('react-dom')>('react-dom');
  return {
    ...actual,
    createPortal: reactDomState.createPortal,
  };
});

import HoverTooltip from './HoverTooltip';

describe('hover tooltip', () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, 'document', {
      value: { body: {} as HTMLElement },
      configurable: true,
    });
    reactDomState.createPortal.mockClear();
  });

  it('renders through a body portal with a light floating style', () => {
    const markup = renderToStaticMarkup(
      <HoverTooltip
        label="固定窗口"
        placement="bottom"
        visible
        anchorRect={
          {
            x: 24,
            y: 16,
            top: 16,
            left: 24,
            right: 56,
            bottom: 48,
            width: 32,
            height: 32,
            toJSON: () => ({}),
          } as DOMRect
        }
      />,
    );

    expect(reactDomState.createPortal).toHaveBeenCalledTimes(1);
    expect(reactDomState.createPortal.mock.calls[0][1]).toBe(document.body);
    expect(markup).toContain('z-[2147483647]');
    expect(markup).toContain('bg-white/95');
    expect(markup).toContain('border-slate-200/90');
    expect(markup).not.toContain('bg-slate-900/90');
    expect(markup).toContain('>固定窗口<');
  });
});
