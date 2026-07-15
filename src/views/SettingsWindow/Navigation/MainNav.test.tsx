// @vitest-environment happy-dom

import { act } from 'react-dom/test-utils';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { MainNav } from './MainNav';

describe('settings main navigation', () => {
  const mountedRoots: Array<ReturnType<typeof createRoot>> = [];

  beforeEach(() => {
    useSettingsConfigStore.setState({
      history: {
        autoCleanupEnabled: true,
        retentionDays: 30,
        maximumRecords: 5000,
        maximumFavorites: 1000,
      },
      updateHistorySettings: vi.fn(async () => {
        throw new Error('not called');
      }),
    });
  });

  afterEach(async () => {
    while (mountedRoots.length > 0) {
      const root = mountedRoots.pop();
      if (root) await act(async () => root.unmount());
    }
  });

  it('groups settings separately from the library without a workspace group', async () => {
    const container = document.createElement('div');
    const root = createRoot(container);
    mountedRoots.push(root);

    await act(async () => {
      root.render(<MainNav activeTab="history" onTabChange={vi.fn()} />);
    });

    expect(container.textContent).toContain('设置');
    expect(container.textContent).toContain('资料库');
    expect(container.textContent).not.toContain('工作区');
    expect(container.textContent).not.toContain('高级');
    expect(container.textContent).toContain('收藏上限');
    expect(container.textContent).toContain('1,000');

    const settingsButton = [...container.querySelectorAll('button')].find(
      (button) => button.textContent === '设置',
    );
    await act(async () => settingsButton?.click());

    expect(container.textContent).toContain('自动清理历史记录');
    expect(container.textContent).toContain('历史记录保留天数');
    expect(container.textContent).toContain('最多保留记录数');
    expect(container.textContent).toContain('收藏夹容量');
  });
});
