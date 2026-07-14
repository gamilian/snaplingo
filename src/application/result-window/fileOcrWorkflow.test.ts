import { describe, expect, it, vi } from 'vitest';
import { runOcrFileWorkflow } from './fileOcrWorkflow';

describe('OCR file workflow', () => {
  it('does nothing when the user cancels file selection', async () => {
    const deps = createDeps({ selectedPath: null });

    await runOcrFileWorkflow(deps);

    expect(deps.setRunning).not.toHaveBeenCalled();
    expect(deps.recognizeImageFile).not.toHaveBeenCalled();
    expect(deps.setText).not.toHaveBeenCalled();
    expect(deps.setError).not.toHaveBeenCalled();
  });

  it('recognizes the selected image file and stores the OCR text', async () => {
    const deps = createDeps({
      selectedPath: '/tmp/example.png',
      recognizedText: 'hello from image',
    });

    await runOcrFileWorkflow(deps);

    expect(deps.setError).toHaveBeenCalledWith(null);
    expect(deps.setRunning).toHaveBeenNthCalledWith(1, true);
    expect(deps.recognizeImageFile).toHaveBeenCalledWith('/tmp/example.png');
    expect(deps.setText).toHaveBeenCalledWith('hello from image');
    expect(deps.setImageDataUrl).toHaveBeenCalledWith(
      'data:image/png;base64,aW1hZ2U=',
    );
    expect(deps.setRunning).toHaveBeenLastCalledWith(false);
  });

  it('stores a readable error when OCR fails', async () => {
    const deps = createDeps({
      selectedPath: '/tmp/example.png',
      error: new Error('provider missing'),
    });

    await runOcrFileWorkflow(deps);

    expect(deps.setError).toHaveBeenLastCalledWith('provider missing');
    expect(deps.setRunning).toHaveBeenLastCalledWith(false);
    expect(deps.setText).not.toHaveBeenCalled();
  });

  it('keeps a successful OCR result when automatic copy fails', async () => {
    const deps = createDeps({
      selectedPath: '/tmp/example.png',
      recognizedText: 'recognized',
      copyError: new Error('clipboard unavailable'),
    });

    await runOcrFileWorkflow(deps);

    expect(deps.setText).toHaveBeenCalledWith('recognized');
    expect(deps.setError).toHaveBeenLastCalledWith(null);
    expect(deps.setRunning).toHaveBeenLastCalledWith(false);
  });
});

function createDeps(options: {
  selectedPath: string | null;
  recognizedText?: string;
  error?: unknown;
  copyError?: unknown;
}) {
  return {
    selectImageFile: vi.fn(async () => options.selectedPath),
    recognizeImageFile: vi.fn(async () => {
      if (options.error) throw options.error;
      return {
        text: options.recognizedText ?? '',
        confidence: null,
        imageDataUrl: 'data:image/png;base64,aW1hZ2U=',
      };
    }),
    transformText: (text: string) => text.replace(/\s+/g, ' ').trim(),
    setText: vi.fn(),
    setConfidence: vi.fn(),
    setImageDataUrl: vi.fn(),
    setRunning: vi.fn(),
    setError: vi.fn(),
    copyText: options.copyError
      ? vi.fn(async () => {
          throw options.copyError;
        })
      : undefined,
  };
}
