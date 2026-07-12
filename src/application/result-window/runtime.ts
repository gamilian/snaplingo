import type { ResultWindowPlatformRuntime } from './platformRuntime';
import type {
  CaptureResultWindowPayload,
  ResultWindowUnsubscribe,
} from './ports';
import { runOcrFileWorkflow } from './fileOcrWorkflow';
import {
  ocrPayloadDisplayText,
  shouldApplyOcrPayloadText,
  shouldApplyTranslationPayloadText,
  shouldClearOcrResultsForPayload,
  shouldClearTranslationResultsForPayload,
  shouldStartFileOcrForPayload,
  translationPayloadSourceText,
} from './payload';
import { normalizeOcrText } from '../../utils/ocrTextProcessing';

export type ResultWindowPresentation = 'overlay' | 'standalone';

export interface ResultWindowStatePort {
  setSourceText(text: string): void;
  clearTranslationResults(): void;
  setOcrText(text: string): void;
  setOcrImageBase64(imageBase64: string | null): void;
  setOcrRunning(value: boolean): void;
  setOcrError(message: string | null): void;
  requestAutoTranslate(): void;
  showResultWindow(): void;
  showOcrWindow(): void;
  hideResultWindow(): void;
}

export interface ResultWindowRuntimePorts {
  platform: ResultWindowPlatformRuntime;
  state: ResultWindowStatePort;
}

export interface ResultWindowResizeInput {
  presentation: ResultWindowPresentation;
  visible: boolean;
  panelHeightPx: number;
}

const resultWindowStandaloneContainerPaddingPx = 16;
const resultWindowStandaloneWidthPx = 660;

export function resultWindowStandaloneWindowHeight(panelHeightPx: number) {
  return panelHeightPx + resultWindowStandaloneContainerPaddingPx;
}

export function createResultWindowRuntime({
  platform,
  state,
}: ResultWindowRuntimePorts) {
  async function applyPayload(payload: CaptureResultWindowPayload) {
    if (payload.mode === 'translation') {
      if (shouldClearTranslationResultsForPayload(payload)) {
        state.clearTranslationResults();
      }
      if (shouldApplyTranslationPayloadText(payload)) {
        state.setSourceText(translationPayloadSourceText(payload));
      }
      if (payload.autoTranslate) {
        state.requestAutoTranslate();
      }
      state.showResultWindow();
      return;
    }

    if (shouldClearOcrResultsForPayload(payload)) {
      state.setOcrText('');
      state.setOcrImageBase64(null);
      state.setOcrError(null);
    }
    if (shouldApplyOcrPayloadText(payload)) {
      state.setOcrText(ocrPayloadDisplayText(payload));
      state.setOcrImageBase64(payload.imageBase64 ?? null);
    }
    if (shouldStartFileOcrForPayload(payload)) {
      await startFileOcr();
      return;
    }
    state.showOcrWindow();
  }

  async function loadPayload(requestId: string) {
    const payload = await platform.commands.takePayload(requestId);
    if (!payload) return false;

    await applyPayload(payload);
    return true;
  }

  async function loadCurrentPayload() {
    const requestId = await platform.commands.currentPayloadRequestId();
    if (!requestId) return false;

    return loadPayload(requestId);
  }

  async function subscribeToPayloads(
    onLoaded?: () => void,
  ): Promise<ResultWindowUnsubscribe> {
    return platform.onPayloadReady((requestId) => {
      void loadPayload(requestId)
        .then((loaded) => {
          if (loaded) {
            onLoaded?.();
          }
        })
        .catch((err) => {
          console.error('Failed to reload result window payload:', err);
        });
    });
  }

  async function startFileOcr() {
    state.showOcrWindow();
    state.setOcrText('');
    state.setOcrImageBase64(null);
    state.setOcrError(null);
    await runOcrFileWorkflow({
      selectImageFile: platform.commands.selectImageFile,
      recognizeImageFile: platform.commands.recognizeImageFile,
      setText: state.setOcrText,
      setRunning: state.setOcrRunning,
      setError: state.setOcrError,
    });
  }

  async function recognizeCurrentOcrImage(imageBase64: string | null) {
    if (!imageBase64) return;

    state.setOcrError(null);
    state.setOcrRunning(true);
    try {
      const result = await platform.commands.recognizeImageData(
        base64ToBytes(imageBase64),
      );
      state.setOcrText(normalizeOcrText(result.text));
    } catch (err) {
      state.setOcrError(errorMessage(err));
    } finally {
      state.setOcrRunning(false);
    }
  }

  async function close(presentation: ResultWindowPresentation) {
    state.hideResultWindow();
    if (presentation === 'standalone') {
      try {
        await platform.dismiss();
      } catch (err) {
        console.error('Failed to hide result window:', err);
      }
    }
  }

  async function resizeStandaloneWindow({
    presentation,
    visible,
    panelHeightPx,
  }: ResultWindowResizeInput) {
    if (presentation !== 'standalone' || !visible) return;

    await platform.resizeTo(
      resultWindowStandaloneWidthPx,
      resultWindowStandaloneWindowHeight(panelHeightPx),
    );
  }

  return {
    commands: platform.commands,
    clipboard: platform.clipboard,
    loadCurrentPayload,
    loadPayload,
    applyPayload,
    subscribeToPayloads,
    startFileOcr,
    recognizeCurrentOcrImage,
    close,
    resizeStandaloneWindow,
    dismiss: platform.dismiss,
    beginDrag: platform.beginDrag,
  };
}

export type ResultWindowRuntime = ReturnType<typeof createResultWindowRuntime>;

function base64ToBytes(base64: string) {
  const payload = base64.includes(',') ? base64.split(',').pop() ?? '' : base64;
  const binary = globalThis.atob(payload);
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

function errorMessage(err: unknown) {
  if (err instanceof Error) return err.message;
  return String(err);
}
