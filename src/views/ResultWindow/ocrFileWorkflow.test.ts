import { describe, expect, it, vi } from 'vitest';
import { runOcrFileWorkflow } from './ocrFileWorkflow';

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
});

function createDeps(options: {
  selectedPath: string | null;
  recognizedText?: string;
  error?: unknown;
}) {
  return {
    selectImageFile: vi.fn(async () => options.selectedPath),
    recognizeImageFile: vi.fn(async () => {
      if (options.error) throw options.error;
      return { text: options.recognizedText ?? '', confidence: null };
    }),
    setText: vi.fn(),
    setRunning: vi.fn(),
    setError: vi.fn(),
  };
}
