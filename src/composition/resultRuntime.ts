import { createResultWindowPlatformRuntime } from '../application/result-window/platformRuntime';
import { createResultWindowRuntime } from '../application/result-window/runtime';
import { resultWindowEvents } from '../platform/tauri/appEvents';
import {
  currentCaptureResultWindowRequestId,
  takeCaptureResultWindowPayload,
} from '../platform/tauri/capture';
import { writeClipboardText } from '../platform/tauri/clipboard';
import { favorites } from '../platform/tauri/favorites';
import { recognizeImageFile, selectImageFile } from '../platform/tauri/ocr';
import { resultWindow } from '../platform/tauri/resultWindow';
import * as durableSettings from '../platform/tauri/settings';
import { systemTts } from '../platform/tauri/tts';
import {
  recordTranslationHistory,
  translateTextWithProvider,
} from '../platform/tauri/translation';
import type { SettingsConfiguration } from '../application/settings/configuration';

export function createWindowResultRuntime(
  configuration: Pick<SettingsConfiguration, 'settings' | 'providers'>,
) {
  const resultWindowPlatformRuntime = createResultWindowPlatformRuntime({
    events: resultWindowEvents,
    window: resultWindow,
    clipboard: { writeText: writeClipboardText },
    commands: {
      currentPayloadRequestId: currentCaptureResultWindowRequestId,
      takePayload: takeCaptureResultWindowPayload,
      selectImageFile,
      recognizeImageFile,
      translateTextWithProvider,
      recordTranslationHistory,
      favoriteTranslationResult: (input) =>
        favorites.addTranslationFavorite({
          sourceText: input.text,
          sourceLang: input.sourceLang,
          targetLang: input.targetLang,
          providerId: input.result.provider_id,
          translatedText: input.result.translated_text,
          detectedLanguage: input.result.detected_language,
          confidence: input.result.confidence,
        }),
      favoriteOcrResult: (input) =>
        favorites.addOcrFavorite({
          imageData: input.imageData,
          recognizedText: input.result.text,
          language: input.language,
          providerUsed: input.providerUsed,
          confidence: input.result.confidence,
        }),
    },
  });

  function getLastResultWindowPosition() {
    const general = configuration.settings.getState().snapshot?.general;
    const x = general?.lastResultWindowX;
    const y = general?.lastResultWindowY;
    return typeof x === 'number' && typeof y === 'number' ? { x, y } : undefined;
  }

  return createResultWindowRuntime({
    platform: resultWindowPlatformRuntime,
    speech: systemTts,
    getTranslationSettings: () =>
      configuration.settings.getState().snapshot?.translation,
    getOcrSettings: () =>
      configuration.settings.getState().snapshot?.ocr,
    positionStore: {
      load: getLastResultWindowPosition,
      save: durableSettings.updateLastResultWindowPosition,
    },
    providers: configuration.providers,
  });
}
