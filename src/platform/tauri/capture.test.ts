import { describe, expect, expectTypeOf, it, vi } from 'vitest';
import type { CaptureMode, CaptureSessionView } from '../../domain/capture';

const { invoke, save } = vi.hoisted(() => ({
  invoke: vi.fn(),
  save: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ save }));

describe('Tauri capture command adapter', () => {
  it('accepts only the domain capture mode vocabulary', async () => {
    const { logCaptureFrontendPerf, openCaptureWindow } = await import('./capture');

    expectTypeOf(openCaptureWindow).parameter(0).toEqualTypeOf<CaptureMode>();
    expectTypeOf<Parameters<typeof logCaptureFrontendPerf>[0]['mode']>()
      .toEqualTypeOf<CaptureMode>();
  });

  it('omits includeCursor when false', async () => {
    const { renderCaptureOutput } = await import('./capture');
    invoke.mockResolvedValueOnce('base64');

    await renderCaptureOutput({
      sessionId: 'capture-1',
      rect: { x: 0, y: 0, width: 10, height: 10 },
      annotations: [],
      includeCursor: false,
    });

    expect(invoke).toHaveBeenCalledWith('render_capture_output', {
      sessionId: 'capture-1',
      rect: { x: 0, y: 0, width: 10, height: 10 },
      annotations: [],
    });
  });

  it('passes pin action to output_capture', async () => {
    const { outputCapture } = await import('./capture');
    invoke.mockResolvedValueOnce(undefined);

    await outputCapture({
      sessionId: 'capture-1',
      rect: { x: 1, y: 2, width: 3, height: 4 },
      annotations: [],
      includeCursor: true,
      action: { type: 'pin' },
    });

    expect(invoke).toHaveBeenCalledWith('output_capture', {
      sessionId: 'capture-1',
      rect: { x: 1, y: 2, width: 3, height: 4 },
      annotations: [],
      includeCursor: true,
      action: { type: 'pin' },
    });
  });

  it('routes favorite output through the screenshot favorite workflow', async () => {
    const { outputCapture } = await import('./capture');
    invoke.mockResolvedValueOnce(undefined);

    await outputCapture({
      sessionId: 'capture-1',
      rect: { x: 1, y: 2, width: 3, height: 4 },
      annotations: [],
      includeCursor: true,
      action: { type: 'favorite' },
    });

    expect(invoke).toHaveBeenCalledWith('favorite_capture_selection', {
      sessionId: 'capture-1',
      rect: { x: 1, y: 2, width: 3, height: 4 },
      annotations: [],
      includeCursor: true,
    });
  });

  it('opens a PNG save dialog from the suggested capture path', async () => {
    const { selectCaptureSavePath } = await import('./capture');
    invoke.mockResolvedValueOnce('/Downloads/SnapLingo-1.png');
    save.mockResolvedValueOnce('/Pictures/SnapLingo-1.png');

    await expect(selectCaptureSavePath()).resolves.toBe(
      '/Pictures/SnapLingo-1.png',
    );
    expect(save).toHaveBeenCalledWith({
      defaultPath: '/Downloads/SnapLingo-1.png',
      filters: [{ name: 'PNG image', extensions: ['png'] }],
    });
  });

  it('hydrates capture session snapshots through the native command', async () => {
    const { hydrateCaptureSessionSnapshots } = await import('./capture');
    const hydratedSession: CaptureSessionView = {
      id: 'capture-1',
      monitors: [
        {
          id: 'monitor-1',
          logical_bounds: { x: 0, y: 0, width: 100, height: 80 },
          physical_bounds: { x: 0, y: 0, width: 200, height: 160 },
          scale_factor: 2,
          image_base64: 'pixels',
        },
      ],
      candidates: [],
      captured_cursor: null,
    };
    invoke.mockResolvedValueOnce(hydratedSession);

    const result: CaptureSessionView =
      await hydrateCaptureSessionSnapshots('capture-1');

    expect(invoke).toHaveBeenCalledWith('hydrate_capture_session_snapshots', {
      sessionId: 'capture-1',
    });
    expect(result.monitors[0].image_base64).toBe('pixels');
  });

  it('hydrates only the monitor needed by the magnifier', async () => {
    const { hydrateCaptureMonitorSnapshot } = await import('./capture');
    const monitor = {
      id: 'monitor-1',
      logical_bounds: { x: 0, y: 0, width: 100, height: 80 },
      physical_bounds: { x: 0, y: 0, width: 200, height: 160 },
      scale_factor: 2,
      image_base64: 'pixels',
    };
    invoke.mockResolvedValueOnce(monitor);

    await expect(
      hydrateCaptureMonitorSnapshot('capture-1', 'monitor-1'),
    ).resolves.toEqual(monitor);
    expect(invoke).toHaveBeenCalledWith('hydrate_capture_monitor_snapshot', {
      sessionId: 'capture-1',
      monitorId: 'monitor-1',
    });
  });

  it('queries control candidates and moves the native cursor through commands', async () => {
    const { currentCaptureControlCandidate, moveCaptureCursor } = await import(
      './capture'
    );
    const candidate = {
      id: 'control-1',
      kind: 'control' as const,
      rect: { x: 10, y: 20, width: 80, height: 30 },
      priority: 10_001,
    };
    invoke.mockResolvedValueOnce(candidate).mockResolvedValueOnce(undefined);

    await expect(
      currentCaptureControlCandidate('capture-1', { x: 30, y: 40 }),
    ).resolves.toEqual(candidate);
    await moveCaptureCursor({ x: -1, y: 0 });

    expect(invoke).toHaveBeenCalledWith('current_capture_control_candidate', {
      sessionId: 'capture-1',
      point: { x: 30, y: 40 },
    });
    expect(invoke).toHaveBeenCalledWith('move_capture_cursor', {
      deltaX: -1,
      deltaY: 0,
    });
  });

  it('opens screenshot OCR results with the capture result command', async () => {
    const { openCaptureOcrResultWindow } = await import('./capture');
    invoke.mockResolvedValueOnce(undefined);

    await openCaptureOcrResultWindow('ocr text', 'rendered-image-base64');

    expect(invoke).toHaveBeenCalledWith('open_capture_ocr_result_window', {
      text: 'ocr text',
      imageBase64: 'rendered-image-base64',
    });
  });

  it('opens screenshot translation results with the capture result command', async () => {
    const { openCaptureTranslationResultWindow } = await import('./capture');
    invoke.mockResolvedValueOnce(undefined);

    await openCaptureTranslationResultWindow('text to translate');

    expect(invoke).toHaveBeenCalledWith(
      'open_capture_translation_result_window',
      { text: 'text to translate' },
    );
  });

  it('loads the current result window request ID for standalone bootstrap', async () => {
    const { currentCaptureResultWindowRequestId } = await import('./capture');
    invoke.mockResolvedValueOnce('42');

    await expect(currentCaptureResultWindowRequestId()).resolves.toBe('42');

    expect(invoke).toHaveBeenCalledWith(
      'current_capture_result_window_request_id',
    );
  });

  it('takes a capture result payload by request ID', async () => {
    const { takeCaptureResultWindowPayload } = await import('./capture');
    const payload = {
      mode: 'translation',
      text: 'hello',
      autoTranslate: false,
    };
    invoke.mockResolvedValueOnce(payload);

    await expect(takeCaptureResultWindowPayload('42')).resolves.toBe(payload);

    expect(invoke).toHaveBeenCalledWith('take_capture_result_window_payload', {
      requestId: '42',
    });
  });

  it('keeps the Advanced Settings screenshot entrypoint on the capture adapter', async () => {
    const { triggerScreenshot } = await import('./capture');
    invoke.mockResolvedValueOnce(undefined);

    await triggerScreenshot();

    expect(invoke).toHaveBeenCalledWith('trigger_screenshot');
  });
});
