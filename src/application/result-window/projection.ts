import type { TranslationResult } from '../../types';
import type { Provider } from '../settings/configuration';
import type { OcrSettings, TranslationSettings } from '../settings/ports';
import type { ResultWindowOrigin } from './ports';

export type ResultWindowMode = 'translation' | 'ocr';
export type ProviderTranslationStatus = 'pending' | 'success' | 'error';

export interface ProviderTranslation extends TranslationResult {
  status: ProviderTranslationStatus;
}

export interface ResultWindowProjection {
  readonly sourceText: string;
  readonly sourceLang: string;
  readonly targetLang: string;
  readonly providerTranslations: ProviderTranslation[];
  readonly isTranslating: boolean;
  readonly ocrText: string;
  readonly ocrConfidence: number | null;
  readonly ocrImageBase64: string | null;
  readonly isOcrRunning: boolean;
  readonly ocrError: string | null;
  readonly resultWindowVisible: boolean;
  readonly resultWindowMode: ResultWindowMode;
  readonly resultWindowOrigin: ResultWindowOrigin;
  readonly autoTranslateRequestId: number;
  readonly translationProviders: Provider[];
  readonly translationSettings: TranslationSettings | null;
  readonly ocrSettings: OcrSettings | null;
}
