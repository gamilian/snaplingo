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
  ocrIntent?: 'show' | 'display-text' | 'file';
  imageBase64?: string;
}

export interface ResultWindowCommandsPort {
  currentPayloadRequestId(): Promise<ResultWindowRequestId | null>;
  takePayload(
    requestId: ResultWindowRequestId,
  ): Promise<CaptureResultWindowPayload | null>;
  selectImageFile(): Promise<string | null>;
  recognizeImageFile(path: string): Promise<OcrResult>;
  recognizeImageData(imageData: Uint8Array | number[]): Promise<OcrResult>;
  translateTextWithProvider(
    providerId: string,
    input: { text: string; sourceLang: string; targetLang: string },
  ): Promise<TranslationResult>;
}

export interface ResultWindowClipboardPort {
  writeText(text: string): Promise<void>;
}
import type { OcrResult } from '../../domain/capture';
import type { TranslationResult } from '../../types';
