import { describe, expect, it, vi } from 'vitest';
import type { CaptureSessionView } from '../../domain/capture';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('capture session tauri adapter', () => {
  it('omits includeCursor when false', async () => {
    const { renderCaptureOutput } = await import('../captureSession');
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
    const { outputCapture } = await import('../captureSession');
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

  it('hydrates capture session snapshots through the native command', async () => {
    const { hydrateCaptureSessionSnapshots } = await import('../captureSession');
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

    const result: CaptureSessionView = await hydrateCaptureSessionSnapshots('capture-1');

    expect(invoke).toHaveBeenCalledWith('hydrate_capture_session_snapshots', {
      sessionId: 'capture-1',
    });
    expect(result.monitors[0].image_base64).toBe('pixels');
  });

  it('opens screenshot OCR results with the capture result command', async () => {
    const { openCaptureOcrResultWindow } = await import('../captureSession');
    invoke.mockResolvedValueOnce(undefined);

    await openCaptureOcrResultWindow('ocr text', 'rendered-image-base64');

    expect(invoke).toHaveBeenCalledWith('open_capture_ocr_result_window', {
      text: 'ocr text',
      imageBase64: 'rendered-image-base64',
    });
  });

  it('opens screenshot translation results with the capture result command', async () => {
    const { openCaptureTranslationResultWindow } = await import('../captureSession');
    invoke.mockResolvedValueOnce(undefined);

    await openCaptureTranslationResultWindow('text to translate');

    expect(invoke).toHaveBeenCalledWith(
      'open_capture_translation_result_window',
      { text: 'text to translate' },
    );
  });

  it('hides the capture overlay through the native command', async () => {
    const { hideCaptureWindow } = await import('../captureSession');
    invoke.mockResolvedValueOnce(undefined);

    await hideCaptureWindow();

    expect(invoke).toHaveBeenCalledWith('hide_capture_window');
  });

  it('keeps the Advanced Settings screenshot entrypoint on Capture Session', async () => {
    const { triggerScreenshot } = await import('../captureSession');
    invoke.mockResolvedValueOnce(undefined);

    await triggerScreenshot();

    expect(invoke).toHaveBeenCalledWith('trigger_screenshot');
  });
});
