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

  it('drops a late OCR result after the workflow is invalidated', async () => {
    let current = true;
    let resolveRecognition!: (result: {
      text: string;
      confidence: number | null;
      imageDataUrl: string;
    }) => void;
    const deps = createDeps({
      selectedPath: '/tmp/example.png',
      recognizeImageFile: () =>
        new Promise((resolve) => {
          resolveRecognition = resolve;
        }),
      isCurrent: () => current,
    });

    const workflow = runOcrFileWorkflow(deps);
    await Promise.resolve();
    current = false;
    resolveRecognition({
      text: 'stale result',
      confidence: 0.9,
      imageDataUrl: 'stale-image',
    });
    await workflow;

    expect(deps.setText).not.toHaveBeenCalled();
    expect(deps.setImageDataUrl).not.toHaveBeenCalled();
    expect(deps.setError).toHaveBeenCalledTimes(1);
    expect(deps.setRunning).toHaveBeenCalledTimes(1);
    expect(deps.setRunning).toHaveBeenCalledWith(true);
  });

  it('does not start a stale workflow after file selection resolves', async () => {
    let current = true;
    let resolveSelection!: (path: string | null) => void;
    const deps = createDeps({
      selectedPath: '/tmp/example.png',
      selectImageFile: () =>
        new Promise((resolve) => {
          resolveSelection = resolve;
        }),
      isCurrent: () => current,
    });

    const selection = runOcrFileWorkflow(deps);
    current = false;
    resolveSelection('/tmp/example.png');
    await selection;

    expect(deps.setRunning).not.toHaveBeenCalled();
    expect(deps.recognizeImageFile).not.toHaveBeenCalled();
  });
});

function createDeps(options: {
  selectedPath: string | null;
  selectImageFile?: () => Promise<string | null>;
  recognizedText?: string;
  error?: unknown;
  copyError?: unknown;
  recognizeImageFile?: () => Promise<{
    text: string;
    confidence: number | null;
    imageDataUrl: string;
  }>;
  isCurrent?: () => boolean;
}) {
  return {
    selectImageFile: vi.fn(
      options.selectImageFile ?? (async () => options.selectedPath),
    ),
    recognizeImageFile: vi.fn(async () => {
      if (options.recognizeImageFile) return options.recognizeImageFile();
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
    isCurrent: options.isCurrent,
  };
}
