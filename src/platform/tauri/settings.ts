import { invoke } from '@tauri-apps/api/core';

interface BackendGeneralSettings {
  language: string;
  theme: string;
  start_on_boot: boolean;
}

interface BackendScreenshotSettings {
  save_path: string;
  format: string;
  quality: number;
}

interface BackendTranslationSettings {
  default_source_lang: string;
  default_target_lang: string;
}

interface BackendSettingsSnapshot {
  general: BackendGeneralSettings;
  screenshot: BackendScreenshotSettings;
  translation: BackendTranslationSettings;
}

interface GeneralSettingsInput {
  language: string;
  theme: string;
  startOnBoot: boolean;
}

interface ScreenshotSettingsInput {
  savePath: string;
  format: string;
  quality: number;
}

interface TranslationSettingsInput {
  defaultSourceLang: string;
  defaultTargetLang: string;
}

function normalizeSnapshot(snapshot: BackendSettingsSnapshot) {
  return {
    general: {
      language: snapshot.general.language,
      theme: snapshot.general.theme,
      startOnBoot: snapshot.general.start_on_boot,
    },
    screenshot: {
      savePath: snapshot.screenshot.save_path,
      format: snapshot.screenshot.format,
      quality: snapshot.screenshot.quality,
    },
    translation: {
      defaultSourceLang: snapshot.translation.default_source_lang,
      defaultTargetLang: snapshot.translation.default_target_lang,
    },
  };
}

export async function getSettingsSnapshot() {
  return normalizeSnapshot(
    await invoke<BackendSettingsSnapshot>('get_settings_snapshot'),
  );
}

export async function updateGeneralSettings(input: GeneralSettingsInput) {
  return normalizeSnapshot(
    await invoke<BackendSettingsSnapshot>('update_general_settings', {
      input: {
        language: input.language,
        theme: input.theme,
        start_on_boot: input.startOnBoot,
      },
    }),
  );
}

export async function updateScreenshotSettings(input: ScreenshotSettingsInput) {
  return normalizeSnapshot(
    await invoke<BackendSettingsSnapshot>('update_screenshot_settings', {
      input: {
        save_path: input.savePath,
        format: input.format,
        quality: input.quality,
      },
    }),
  );
}

export async function updateTranslationSettings(
  input: TranslationSettingsInput,
) {
  return normalizeSnapshot(
    await invoke<BackendSettingsSnapshot>('update_translation_settings', {
      input: {
        default_source_lang: input.defaultSourceLang,
        default_target_lang: input.defaultTargetLang,
      },
    }),
  );
}
