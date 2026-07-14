export type ResultWindowUnsubscribe = () => void;
export type ResultWindowRequestId = string;
export type ResultPayloadReadyHandler = (
  requestId: ResultWindowRequestId,
) => void | Promise<void>;

export interface ResultWindowEventsPort {
  subscribeResultPayloadReady(
    handler: ResultPayloadReadyHandler,
  ): Promise<ResultWindowUnsubscribe>;
}

export interface ResultWindowPort {
  resize(width: number, height: number): Promise<void>;
  hide(): Promise<void>;
  startDragging(): Promise<void>;
}

export interface CaptureResultWindowPayload {
  mode: 'translation' | 'ocr';
  text: string;
  autoTranslate: boolean;
  ocrIntent?: 'display-text' | 'file';
  imageBase64?: string;
}

export interface ResultWindowCommandsPort {
  currentPayloadRequestId(): Promise<ResultWindowRequestId | null>;
  takePayload(
    requestId: ResultWindowRequestId,
  ): Promise<CaptureResultWindowPayload | null>;
  selectImageFile(): Promise<string | null>;
  recognizeImageFile(path: string): Promise<OcrResult>;
  translateTextWithProvider(
    providerId: string,
    input: { text: string; sourceLang: string; targetLang: string },
  ): Promise<TranslationResult>;
  recordTranslationHistory(input: {
    text: string;
    sourceLang: string;
    targetLang: string;
    results: TranslationResult[];
    durationMs: number;
  }): Promise<void>;
  favoriteTranslationResult(input: {
    text: string;
    sourceLang: string;
    targetLang: string;
    result: TranslationResult;
  }): Promise<number>;
  favoriteOcrResult(input: {
    imageData: Uint8Array | number[];
    result: OcrResult;
  }): Promise<number>;
}

export interface ResultWindowClipboardPort {
  writeText(text: string): Promise<void>;
}
import type { OcrResult } from '../../domain/capture';
import type { TranslationResult } from '../../types';
