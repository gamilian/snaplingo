import { describe, expect, it, vi } from 'vitest';

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

  it('opens screenshot OCR results with the capture result command', async () => {
    const { openCaptureOcrResultWindow } = await import('../captureSession');
    invoke.mockResolvedValueOnce(undefined);

    await openCaptureOcrResultWindow('ocr text');

    expect(invoke).toHaveBeenCalledWith('open_capture_ocr_result_window', {
      text: 'ocr text',
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
});
