// @vitest-environment happy-dom

import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { SettingsScrollPage } from './SettingsScrollPage';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

describe('SettingsScrollPage navigation requests', () => {
  afterEach(() => {
    delete (HTMLElement.prototype as { scrollIntoView?: unknown }).scrollIntoView;
  });

  it('scrolls to and consumes a requested section', async () => {
    const scrollIntoView = vi.fn();
    const onHandled = vi.fn();
    Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
      configurable: true,
      value: scrollIntoView,
    });
    const container = document.createElement('div');
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <SettingsScrollPage
          title="通用"
          description="设置"
          requestedSectionId="about"
          onRequestedSectionHandled={onHandled}
          sections={[
            {
              id: 'interface',
              label: '界面',
              description: '界面设置',
              content: <div />,
            },
            {
              id: 'about',
              label: '关于',
              description: '版本信息',
              content: <div />,
            },
          ]}
        />,
      );
    });

    expect(scrollIntoView).toHaveBeenCalledWith({
      behavior: 'smooth',
      block: 'start',
    });
    expect(onHandled).toHaveBeenCalledOnce();
    act(() => root.unmount());
  });
});
