// @vitest-environment happy-dom

import { act, createElement } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const settingsConfig = vi.hoisted(() => ({
  state: {
    screenshot: {
      annotationColors: [
        [255, 77, 79, 255],
        [24, 144, 255, 255],
      ] as [number, number, number, number][],
    },
    updateAnnotationColors: vi.fn(
      async (_colors: [number, number, number, number][]) => undefined,
    ),
  },
}));

vi.mock('../../../stores/settingsConfigStore', () => ({
  useSettingsConfigStore: (
    selector: (state: typeof settingsConfig.state) => unknown,
  ) => selector(settingsConfig.state),
}));

import { EditorPage } from './EditorPage';

const mountedRoots: Array<{ container: HTMLDivElement; root: Root }> = [];

beforeEach(() => {
  settingsConfig.state.screenshot.annotationColors = [
    [255, 77, 79, 255],
    [24, 144, 255, 255],
  ];
  settingsConfig.state.updateAnnotationColors.mockImplementation(
    async (colors) => {
      settingsConfig.state.screenshot.annotationColors = colors;
    },
  );
});

afterEach(() => {
  for (const { container, root } of mountedRoots.splice(0)) {
    root.unmount();
    container.remove();
  }
  vi.clearAllMocks();
});

describe('EditorPage annotation colors', () => {
  it('opens the system palette and persists a newly added preset color', async () => {
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push({ container, root });

    await act(async () => {
      root.render(createElement(EditorPage));
    });

    const systemPalette = container.querySelector<HTMLInputElement>(
      'input[aria-label="选择系统颜色"]',
    );
    expect(systemPalette).not.toBeNull();
    expect(systemPalette!.className).toContain('absolute');
    expect(systemPalette!.className).toContain('inset-0');
    expect(systemPalette!.className).not.toContain('sr-only');
    const paletteClick = vi.spyOn(systemPalette!, 'click');

    await act(async () => {
      findButton(container, '系统调色板').click();
    });
    expect(paletteClick).toHaveBeenCalledOnce();

    await act(async () => {
      systemPalette!.value = '#663399';
      systemPalette!.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await act(async () => {
      findButton(container, '新增').click();
    });

    expect(settingsConfig.state.updateAnnotationColors).toHaveBeenCalledWith([
      [255, 77, 79, 255],
      [24, 144, 255, 255],
      [102, 51, 153, 255],
    ]);
  });

  it('selects the next preset and synchronizes its draft after deletion', async () => {
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push({ container, root });

    await act(async () => {
      root.render(createElement(EditorPage));
    });
    await act(async () => {
      findButton(container, '删除').click();
    });
    await act(async () => {
      root.render(createElement(EditorPage));
    });

    expect(
      container.querySelector<HTMLInputElement>(
        'input[aria-label="颜色十六进制值"]',
      )?.value,
    ).toBe('#1890FF');
  });
});

function findButton(container: HTMLElement, label: string) {
  const button = [...container.querySelectorAll('button')].find(
    (candidate) => candidate.textContent?.trim() === label,
  );
  if (!button) {
    throw new Error(`Button '${label}' was not found`);
  }
  return button;
}
