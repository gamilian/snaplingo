import type { TranslationResult } from '../../types';
import { resolveTranslationRequestLanguages } from '../translation/languages';
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
  loadActiveTranslationProviderIds(): Promise<string[]>;
  getTranslationSession(): {
    sessionId: string | null;
    sourceText: string;
    sourceLang: string;
    targetLang: string;
  };
  startTranslationSession(text: string, providerIds: string[]): string;
  beginProviderTranslation(sessionId: string, providerId: string): void;
  completeProviderTranslation(sessionId: string, result: TranslationResult): void;
  failProviderTranslation(
    sessionId: string,
    providerId: string,
    message: string,
  ): void;
  setTranslating(value: boolean): void;
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
  async function persistTranslationHistory(input: {
    text: string;
    sourceLang: string;
    targetLang: string;
    results: TranslationResult[];
    startedAt: number;
  }) {
    try {
      await platform.commands.recordTranslationHistory({
        text: input.text,
        sourceLang: input.sourceLang,
        targetLang: input.targetLang,
        results: input.results,
        durationMs: Math.max(0, Math.round(performance.now() - input.startedAt)),
      });
    } catch (error) {
      console.error('Failed to record translation history:', error);
    }
  }

  async function translate(input: {
    text: string;
    sourceLang: string;
    targetLang: string;
  }) {
    if (!input.text.trim()) return;

    const request = resolveTranslationRequestLanguages(
      input.text,
      input.sourceLang,
      input.targetLang,
    );
    const providerIds = await state.loadActiveTranslationProviderIds();
    const sessionId = state.startTranslationSession(input.text, providerIds);

    if (providerIds.length === 0) {
      state.setTranslating(false);
      return;
    }

    const startedAt = performance.now();
    const results = await Promise.all(
      providerIds.map(async (providerId) => {
        state.beginProviderTranslation(sessionId, providerId);
        try {
          const result = await platform.commands.translateTextWithProvider(
            providerId,
            {
              text: input.text,
              sourceLang: request.sourceLang,
              targetLang: request.targetLang,
            },
          );
          state.completeProviderTranslation(sessionId, result);
          return result;
        } catch (error) {
          state.failProviderTranslation(
            sessionId,
            providerId,
            errorMessage(error),
          );
          return null;
        }
      }),
    );
    const completedResults = results.filter(
      (result): result is TranslationResult => result !== null,
    );

    if (completedResults.length > 0) {
      await persistTranslationHistory({
        text: input.text,
        sourceLang: request.sourceLang,
        targetLang: request.targetLang,
        results: completedResults,
        startedAt,
      });
    }
  }

  async function retryTranslationProvider(providerId: string) {
    const session = state.getTranslationSession();
    if (!session.sessionId || !session.sourceText.trim()) return;

    const request = resolveTranslationRequestLanguages(
      session.sourceText,
      session.sourceLang,
      session.targetLang,
    );
    const startedAt = performance.now();
    state.beginProviderTranslation(session.sessionId, providerId);

    try {
      const result = await platform.commands.translateTextWithProvider(
        providerId,
        {
          text: session.sourceText,
          sourceLang: request.sourceLang,
          targetLang: request.targetLang,
        },
      );
      state.completeProviderTranslation(session.sessionId, result);
      await persistTranslationHistory({
        text: session.sourceText,
        sourceLang: request.sourceLang,
        targetLang: request.targetLang,
        results: [result],
        startedAt,
      });
    } catch (error) {
      state.failProviderTranslation(
        session.sessionId,
        providerId,
        errorMessage(error),
      );
    }
  }

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

  async function favoriteOcrResult(imageBase64: string | null, text: string) {
    return platform.commands.favoriteOcrResult({
      imageData: imageBase64 ? base64ToBytes(imageBase64) : [],
      result: { text, confidence: null },
    });
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
    favoriteOcrResult,
    translate,
    retryTranslationProvider,
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

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
