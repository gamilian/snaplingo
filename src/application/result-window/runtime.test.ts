import { describe, expect, it, vi } from 'vitest';
import {
  createResultWindowRuntime,
  resultWindowStandaloneWindowHeight,
} from './runtime';
import type {
  CaptureResultWindowPayload,
  ResultPayloadReadyHandler,
  ResultWindowUnsubscribe,
} from './ports';

describe('result window application runtime', () => {
  it('hydrates the current translation payload by request ID', async () => {
    const payload: CaptureResultWindowPayload = {
      mode: 'translation',
      text: 'Visit https://example.\ncom',
      autoTranslate: true,
    };
    const { runtime, platform, state } = createRuntime({
      currentPayloadRequestId: '42',
      payloads: { '42': payload },
    });

    await expect(runtime.loadCurrentPayload()).resolves.toBe(true);

    expect(platform.commands.currentPayloadRequestId).toHaveBeenCalledTimes(1);
    expect(platform.commands.takePayload).toHaveBeenCalledWith('42');
    expect(state.clearTranslationResults).toHaveBeenCalledTimes(1);
    expect(state.setSourceText).toHaveBeenCalledWith(
      'Visit https://example.com',
    );
    expect(state.requestAutoTranslate).toHaveBeenCalledTimes(1);
    expect(state.showResultWindow).toHaveBeenCalledTimes(1);
  });

  it('does not take a payload when no current request ID exists', async () => {
    const { runtime, platform, state } = createRuntime({
      currentPayloadRequestId: null,
    });

    await expect(runtime.loadCurrentPayload()).resolves.toBe(false);

    expect(platform.commands.takePayload).not.toHaveBeenCalled();
    expect(state.showResultWindow).not.toHaveBeenCalled();
    expect(state.showOcrWindow).not.toHaveBeenCalled();
  });

  it('subscribes to payload-ready events and takes only the matching payload', async () => {
    const payload: CaptureResultWindowPayload = {
      mode: 'ocr',
      text: 'recognized',
      autoTranslate: false,
      ocrIntent: 'display-text',
      imageBase64: 'image-base64',
    };
    const onLoaded = vi.fn();
    const { runtime, platform, state, emitPayloadReady, unsubscribe } =
      createRuntime({
        payloads: { '7': payload },
      });

    await expect(runtime.subscribeToPayloads(onLoaded)).resolves.toBe(
      unsubscribe,
    );
    await emitPayloadReady('7');
    await Promise.resolve();

    expect(platform.commands.takePayload).toHaveBeenCalledWith('7');
    expect(state.setOcrText).toHaveBeenCalledWith('recognized');
    expect(state.setOcrImageBase64).toHaveBeenCalledWith('image-base64');
    expect(state.showOcrWindow).toHaveBeenCalledTimes(1);
    expect(onLoaded).toHaveBeenCalledTimes(1);
  });

  it('contains payload-ready hydration failures inside the runtime handler', async () => {
    const onLoaded = vi.fn();
    const { runtime, emitPayloadReady } = createRuntime({
      takePayloadError: new Error('payload unavailable'),
    });

    await runtime.subscribeToPayloads(onLoaded);
    await expect(emitPayloadReady('7')).resolves.toBeUndefined();
    await Promise.resolve();

    expect(onLoaded).not.toHaveBeenCalled();
  });

  it('starts OCR file workflow from a file-intent payload', async () => {
    const { runtime, platform, state } = createRuntime({
      selectedImagePath: '/tmp/example.png',
      recognizedFileText: 'file text',
    });

    await runtime.applyPayload({
      mode: 'ocr',
      text: '',
      autoTranslate: false,
      ocrIntent: 'file',
    });

    expect(state.showOcrWindow).toHaveBeenCalledTimes(1);
    expect(state.setOcrText).toHaveBeenCalledWith('');
    expect(state.setOcrImageBase64).toHaveBeenCalledWith(null);
    expect(platform.commands.selectImageFile).toHaveBeenCalledTimes(1);
    expect(platform.commands.recognizeImageFile).toHaveBeenCalledWith(
      '/tmp/example.png',
    );
    expect(state.setOcrText).toHaveBeenLastCalledWith('file text');
    expect(state.setOcrRunning).toHaveBeenNthCalledWith(1, true);
    expect(state.setOcrRunning).toHaveBeenLastCalledWith(false);
  });

  it('recognizes the current OCR image data through the injected commands', async () => {
    const { runtime, platform, state } = createRuntime({
      recognizedImageText: 'image text',
    });

    await runtime.recognizeCurrentOcrImage('aGVsbG8=');

    expect(platform.commands.recognizeImageData).toHaveBeenCalledWith(
      new Uint8Array([104, 101, 108, 108, 111]),
    );
    expect(state.setOcrText).toHaveBeenCalledWith('image text');
    expect(state.setOcrRunning).toHaveBeenNthCalledWith(1, true);
    expect(state.setOcrRunning).toHaveBeenLastCalledWith(false);
  });

  it('closes overlay state locally and standalone state plus native window', async () => {
    const { runtime, platform, state } = createRuntime();

    await runtime.close('overlay');
    await runtime.close('standalone');

    expect(state.hideResultWindow).toHaveBeenCalledTimes(2);
    expect(platform.dismiss).toHaveBeenCalledTimes(1);
  });

  it('keeps standalone close state hidden when native hide fails', async () => {
    const { runtime, state } = createRuntime({
      dismissError: new Error('window unavailable'),
    });

    await expect(runtime.close('standalone')).resolves.toBeUndefined();

    expect(state.hideResultWindow).toHaveBeenCalledTimes(1);
  });

  it('resizes only visible standalone result windows', async () => {
    const { runtime, platform } = createRuntime();

    await runtime.resizeStandaloneWindow({
      presentation: 'overlay',
      visible: true,
      panelHeightPx: 300,
    });
    await runtime.resizeStandaloneWindow({
      presentation: 'standalone',
      visible: false,
      panelHeightPx: 300,
    });
    await runtime.resizeStandaloneWindow({
      presentation: 'standalone',
      visible: true,
      panelHeightPx: 300,
    });

    expect(platform.resizeTo).toHaveBeenCalledOnce();
    expect(platform.resizeTo).toHaveBeenCalledWith(
      660,
      resultWindowStandaloneWindowHeight(300),
    );
  });
});

function createRuntime(options: {
  currentPayloadRequestId?: string | null;
  payloads?: Record<string, CaptureResultWindowPayload | null>;
  selectedImagePath?: string | null;
  recognizedFileText?: string;
  recognizedImageText?: string;
  takePayloadError?: unknown;
  dismissError?: unknown;
} = {}) {
  let payloadReadyHandler: ResultPayloadReadyHandler | null = null;
  const unsubscribe: ResultWindowUnsubscribe = vi.fn();
  const platform = {
    commands: {
      currentPayloadRequestId: vi.fn(
        async () => options.currentPayloadRequestId ?? null,
      ),
      takePayload: vi.fn(async (requestId: string) => {
        if (options.takePayloadError) throw options.takePayloadError;
        return options.payloads?.[requestId] ?? null;
      }),
      selectImageFile: vi.fn(async () => options.selectedImagePath ?? null),
      recognizeImageFile: vi.fn(async () => ({
        text: options.recognizedFileText ?? '',
        confidence: null,
      })),
      recognizeImageData: vi.fn(async () => ({
        text: options.recognizedImageText ?? '',
        confidence: null,
      })),
      translateTextWithProvider: vi.fn(),
    },
    clipboard: { copyText: vi.fn() },
    onPayloadReady: vi.fn(async (handler: ResultPayloadReadyHandler) => {
      payloadReadyHandler = handler;
      return unsubscribe;
    }),
    resizeTo: vi.fn(async () => undefined),
    dismiss: vi.fn(async () => {
      if (options.dismissError) throw options.dismissError;
    }),
    beginDrag: vi.fn(async () => undefined),
  };
  const state = {
    setSourceText: vi.fn(),
    clearTranslationResults: vi.fn(),
    setOcrText: vi.fn(),
    setOcrImageBase64: vi.fn(),
    setOcrRunning: vi.fn(),
    setOcrError: vi.fn(),
    requestAutoTranslate: vi.fn(),
    showResultWindow: vi.fn(),
    showOcrWindow: vi.fn(),
    hideResultWindow: vi.fn(),
  };
  const runtime = createResultWindowRuntime({ platform, state });

  return {
    runtime,
    platform,
    state,
    unsubscribe,
    emitPayloadReady: async (requestId: string) => {
      await payloadReadyHandler?.(requestId);
    },
  };
}
