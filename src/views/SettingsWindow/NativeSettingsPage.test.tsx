// @vitest-environment happy-dom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { NativeSettingsPresentation, NativeSettingsRequest } from '../../application/settings/ports';

const fixture = vi.hoisted(() => ({
  present: vi.fn<(request: NativeSettingsRequest) => Promise<NativeSettingsPresentation>>(),
  section: null as string | null,
  consume: vi.fn(),
  resize: null as (() => void) | null,
}));
vi.mock('./runtimeContext', () => ({ useSettingsRuntime: () => ({ window: { presentNativeSettingsPage: fixture.present } }) }));
vi.mock('../../stores/settingsStore', () => ({ useSettingsStore: (selector: (state: unknown) => unknown) => selector({ requestedSection: fixture.section, consumeRequestedSection: fixture.consume }) }));
import { NativeSettingsPage } from './NativeSettingsPage';
const NativeGeneralPage = () => <NativeSettingsPage page="general" fallback={<div>Retained General page</div>} />;

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let root: ReturnType<typeof createRoot>;
let container: HTMLDivElement;
async function flush() { await act(async () => { await Promise.resolve(); }); }

beforeEach(() => {
  fixture.present.mockReset().mockResolvedValue({ supported: true });
  fixture.section = null;
  fixture.consume.mockReset();
  vi.stubGlobal('ResizeObserver', class {
    constructor(receive: () => void) { fixture.resize = receive; }
    observe() {} disconnect() {}
  });
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({ x: 202, y: 0, width: 698, height: 650 } as DOMRect);
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(558.4);
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('native General page mount', () => {
  it('keeps the configured scale exact when clientWidth rounds fractional CSS bounds', async () => {
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({ x: 303, y: 0, width: 797, height: 650 } as DOMRect);
    vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(531);
    await act(async () => root.render(<NativeSettingsPage page="ocr" interfaceScale={1.5} fallback={<div>OCR fallback</div>} />));
    const token = fixture.present.mock.calls[0][0].token;
    expect(fixture.present.mock.calls[0][0].viewport?.scale).toBe(1.5);
    await act(async () => root.render(<NativeSettingsPage page="ocr" interfaceScale={1.75} fallback={<div>OCR fallback</div>} />));
    expect(fixture.present).toHaveBeenLastCalledWith({ token, page: 'ocr', viewport: { x: 303, y: 0, width: 797, height: 650, scale: 1.75 } });
  });

  it('forwards scaled bounds and section, then updates layout on resize', async () => {
    fixture.section = 'about';
    await act(async () => root.render(<NativeGeneralPage />));
    expect(fixture.present).toHaveBeenCalledWith({ token: expect.any(String), page: 'general', viewport: { x: 202, y: 0, width: 698, height: 650, scale: 1.25 }, section: 'about' });
    expect(fixture.consume).toHaveBeenCalledOnce();
    await act(async () => fixture.resize?.());
    expect(fixture.present).toHaveBeenCalledTimes(2);
  });

  it('removes an in-flight mount after unmount so it cannot cover the next tab', async () => {
    let finish!: (value: NativeSettingsPresentation) => void;
    fixture.present.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
    await act(async () => root.render(<NativeGeneralPage />));
    const token = fixture.present.mock.calls[0][0].token;
    await act(async () => root.render(<div>OCR settings</div>));
    expect(fixture.present).toHaveBeenCalledOnce();
    finish({ supported: true });
    await flush();
    expect(fixture.present).toHaveBeenLastCalledWith({ token, page: 'general', viewport: null });
    expect(container.textContent).toBe('OCR settings');
  });

  it('keeps the retained page on platforms without the native renderer', async () => {
    fixture.present.mockResolvedValueOnce({ supported: false });
    await act(async () => root.render(<NativeGeneralPage />));
    expect(container.textContent).toBe('Retained General page');
    expect(fixture.consume).not.toHaveBeenCalled();
  });

  it('serializes an old mount and disposal before mounting a new General page', async () => {
    let finish!: (value: NativeSettingsPresentation) => void;
    fixture.present.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
    await act(async () => root.render(<NativeGeneralPage key="first" />));
    const oldToken = fixture.present.mock.calls[0][0].token;
    await act(async () => root.render(<NativeGeneralPage key="second" />));
    expect(fixture.present).toHaveBeenCalledOnce();
    finish({ supported: true });
    await flush();
    expect(fixture.present.mock.calls[1][0]).toEqual({ token: oldToken, page: 'general', viewport: null });
    expect(fixture.present.mock.calls[2][0].token).not.toBe(oldToken);
    expect(fixture.present.mock.calls[2][0].viewport).not.toBeNull();
  });

  it('replaces General with native OCR and Translation using page-specific owners', async () => {
    await act(async () => root.render(<NativeSettingsPage page="general" fallback={<div>General fallback</div>} />));
    const generalToken = fixture.present.mock.calls[0][0].token;
    await act(async () => root.render(<NativeSettingsPage page="ocr" fallback={<div>OCR fallback</div>} />));
    expect(fixture.present.mock.calls[1][0]).toEqual({ token: generalToken, page: 'general', viewport: null });
    const ocrRequest = fixture.present.mock.calls[2][0];
    expect(ocrRequest.page).toBe('ocr');
    expect(ocrRequest.viewport).not.toBeNull();
    expect(container.querySelector('[aria-label="OCR 设置"]')).not.toBeNull();
    await act(async () => root.render(<NativeSettingsPage page="translation" fallback={<div>Translation fallback</div>} />));
    expect(fixture.present.mock.calls[3][0]).toEqual({ token: ocrRequest.token, page: 'ocr', viewport: null });
    expect(fixture.present.mock.calls[4][0].page).toBe('translation');
    expect(container.querySelector('[aria-label="翻译设置"]')).not.toBeNull();
    const translationRequest = fixture.present.mock.calls[4][0];
    await act(async () => root.render(<NativeSettingsPage page="screenshot" fallback={<div>Screenshot fallback</div>} />));
    expect(fixture.present.mock.calls[5][0]).toEqual({ token: translationRequest.token, page: 'translation', viewport: null });
    expect(fixture.present.mock.calls[6][0].page).toBe('screenshot');
    expect(container.querySelector('[aria-label="截图设置"]')).not.toBeNull();
  });

  it('removes a previous native view if a later layout update fails', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    await act(async () => root.render(<NativeGeneralPage />));
    const token = fixture.present.mock.calls[0][0].token;
    fixture.present.mockRejectedValueOnce(new Error('Native window unavailable'));
    await act(async () => fixture.resize?.());
    expect(container.textContent).toBe('Retained General page');
    expect(fixture.present).toHaveBeenLastCalledWith({ token, page: 'general', viewport: null });
  });

  it('mounts History and Favorites with independent native owners', async () => {
    await act(async () => root.render(<NativeSettingsPage page="history" fallback={<div>History fallback</div>} />));
    const history = fixture.present.mock.calls[0][0];
    expect(history.page).toBe('history');
    expect(container.querySelector('[aria-label="历史记录"]')).not.toBeNull();
    await act(async () => root.render(<NativeSettingsPage page="favorites" fallback={<div>Favorites fallback</div>} />));
    expect(fixture.present.mock.calls[1][0]).toEqual({ token: history.token, page: 'history', viewport: null });
    expect(fixture.present.mock.calls[2][0].page).toBe('favorites');
    expect(container.querySelector('[aria-label="收藏"]')).not.toBeNull();
  });

  it('hides the native controls under retained dialogs and restores them after closing', async () => {
    await act(async () => root.render(<NativeGeneralPage />));
    const token = fixture.present.mock.calls[0][0].token;
    fixture.present.mockImplementation(async (request) => ({ supported: true, ...(request.hidden ? { preview: 'frozen-controls-png' } : {}) }));
    const modal = document.createElement('section');
    modal.setAttribute('aria-modal', 'true');
    await act(async () => {
      document.body.append(modal);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(fixture.present).toHaveBeenLastCalledWith(expect.objectContaining({ token, hidden: true }));
    const preview = container.querySelector<HTMLImageElement>('[data-native-settings-preview]')!;
    expect(preview.src).toBe('data:image/png;base64,frozen-controls-png');
    expect(preview.getAttribute('aria-hidden')).toBe('true');
    expect(preview.alt).toBe('');
    expect(preview.classList.contains('pointer-events-none')).toBe(true);
    await act(async () => {
      modal.remove();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(fixture.present.mock.calls[fixture.present.mock.calls.length - 1][0].hidden).toBeUndefined();
    expect(fixture.present.mock.calls[fixture.present.mock.calls.length - 1][0].viewport).not.toBeNull();
    expect(container.querySelector('[data-native-settings-preview]')).toBeNull();
  });

  it('discards a late modal preview after switching to another Settings tab', async () => {
    await act(async () => root.render(<NativeGeneralPage />));
    const token = fixture.present.mock.calls[0][0].token;
    let finish!: (value: NativeSettingsPresentation) => void;
    fixture.present.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
    const modal = document.createElement('dialog');
    modal.setAttribute('open', '');
    await act(async () => {
      document.body.append(modal);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    await act(async () => root.render(<div>OCR settings</div>));
    finish({ supported: true, preview: 'stale-controls' });
    await flush();
    expect(container.textContent).toBe('OCR settings');
    expect(container.querySelector('[data-native-settings-preview]')).toBeNull();
    expect(fixture.present).toHaveBeenLastCalledWith({ token, page: 'general', viewport: null });
    modal.remove();
  });
});
