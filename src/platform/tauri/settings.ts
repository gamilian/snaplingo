import { invoke } from '@tauri-apps/api/core';
import type {
  AnnotationColorPreset,
  ScreenshotFormat,
  ScreenshotNamingRule,
} from '../../application/settings/ports';

interface BackendGeneralSettings {
  language: string;
  theme: string;
  start_on_boot: boolean;
}

interface BackendScreenshotSettings {
  save_path: string;
  format: ScreenshotFormat;
  quality: number;
  naming_rule: ScreenshotNamingRule;
  custom_file_name: string;
  auto_copy: boolean;
  default_stroke_width: number;
  default_font_size: number;
  remember_last_tool: boolean;
  show_selection_size: boolean;
  show_magnifier: boolean;
  pin_opacity: number;
  pin_shadow: boolean;
  annotation_colors: AnnotationColorPreset[];
}

interface BackendTranslationSettings {
  default_source_lang: string;
  default_target_lang: string;
  auto_translate: boolean;
  auto_copy: boolean;
  preserve_line_breaks: boolean;
  incremental_translation: boolean;
  window_always_on_top: boolean;
  hide_on_blur: boolean;
}

interface BackendOcrSettings {
  recognition_language: string;
  auto_copy: boolean;
  preserve_formatting: boolean;
  remove_chinese_spaces: boolean;
  show_confidence: boolean;
}

interface BackendHistorySettings {
  auto_cleanup_enabled: boolean;
  retention_days: number;
  maximum_records: number;
  maximum_favorites: number;
}

interface BackendSettingsSnapshot {
  general: BackendGeneralSettings;
  screenshot: BackendScreenshotSettings;
  translation: BackendTranslationSettings;
  ocr: BackendOcrSettings;
  history: BackendHistorySettings;
}

interface GeneralSettingsInput {
  language: string;
  theme: string;
  startOnBoot: boolean;
}

interface ScreenshotSettingsInput {
  savePath: string;
  format: ScreenshotFormat;
  quality: number;
  namingRule: ScreenshotNamingRule;
  customFileName: string;
  autoCopy: boolean;
  defaultStrokeWidth: number;
  defaultFontSize: number;
  rememberLastTool: boolean;
  showSelectionSize: boolean;
  showMagnifier: boolean;
  pinOpacity: number;
  pinShadow: boolean;
  annotationColors: AnnotationColorPreset[];
}

interface TranslationSettingsInput {
  defaultSourceLang: string;
  defaultTargetLang: string;
  autoTranslate: boolean;
  autoCopy: boolean;
  preserveLineBreaks: boolean;
  incrementalTranslation: boolean;
  windowAlwaysOnTop: boolean;
  hideOnBlur: boolean;
}

interface OcrSettingsInput {
  recognitionLanguage: string;
  autoCopy: boolean;
  preserveFormatting: boolean;
  removeChineseSpaces: boolean;
  showConfidence: boolean;
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
      namingRule: snapshot.screenshot.naming_rule,
      customFileName: snapshot.screenshot.custom_file_name,
      autoCopy: snapshot.screenshot.auto_copy,
      defaultStrokeWidth: snapshot.screenshot.default_stroke_width,
      defaultFontSize: snapshot.screenshot.default_font_size,
      rememberLastTool: snapshot.screenshot.remember_last_tool,
      showSelectionSize: snapshot.screenshot.show_selection_size,
      showMagnifier: snapshot.screenshot.show_magnifier,
      pinOpacity: snapshot.screenshot.pin_opacity,
      pinShadow: snapshot.screenshot.pin_shadow,
      annotationColors: snapshot.screenshot.annotation_colors,
    },
    translation: {
      defaultSourceLang: snapshot.translation.default_source_lang,
      defaultTargetLang: snapshot.translation.default_target_lang,
      autoTranslate: snapshot.translation.auto_translate,
      autoCopy: snapshot.translation.auto_copy,
      preserveLineBreaks: snapshot.translation.preserve_line_breaks,
      incrementalTranslation: snapshot.translation.incremental_translation,
      windowAlwaysOnTop: snapshot.translation.window_always_on_top,
      hideOnBlur: snapshot.translation.hide_on_blur,
    },
    ocr: {
      recognitionLanguage: snapshot.ocr.recognition_language,
      autoCopy: snapshot.ocr.auto_copy,
      preserveFormatting: snapshot.ocr.preserve_formatting,
      removeChineseSpaces: snapshot.ocr.remove_chinese_spaces,
      showConfidence: snapshot.ocr.show_confidence,
    },
    history: {
      autoCleanupEnabled: snapshot.history.auto_cleanup_enabled,
      retentionDays: snapshot.history.retention_days,
      maximumRecords: snapshot.history.maximum_records,
      maximumFavorites: snapshot.history.maximum_favorites,
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
        naming_rule: input.namingRule,
        custom_file_name: input.customFileName,
        auto_copy: input.autoCopy,
        default_stroke_width: input.defaultStrokeWidth,
        default_font_size: input.defaultFontSize,
        remember_last_tool: input.rememberLastTool,
        show_selection_size: input.showSelectionSize,
        show_magnifier: input.showMagnifier,
        pin_opacity: input.pinOpacity,
        pin_shadow: input.pinShadow,
        annotation_colors: input.annotationColors,
      },
    }),
  );
}

export async function updateAnnotationColors(colors: AnnotationColorPreset[]) {
  return normalizeSnapshot(
    await invoke<BackendSettingsSnapshot>('update_annotation_colors', {
      colors,
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
        auto_translate: input.autoTranslate,
        auto_copy: input.autoCopy,
        preserve_line_breaks: input.preserveLineBreaks,
        incremental_translation: input.incrementalTranslation,
        window_always_on_top: input.windowAlwaysOnTop,
        hide_on_blur: input.hideOnBlur,
      },
    }),
  );
}

export async function updateOcrSettings(input: OcrSettingsInput) {
  return normalizeSnapshot(
    await invoke<BackendSettingsSnapshot>('update_ocr_settings', {
      input: {
        recognition_language: input.recognitionLanguage,
        auto_copy: input.autoCopy,
        preserve_formatting: input.preserveFormatting,
        remove_chinese_spaces: input.removeChineseSpaces,
        show_confidence: input.showConfidence,
      },
    }),
  );
}

export async function updateHistorySettings(input: {
  autoCleanupEnabled: boolean;
  retentionDays: number;
  maximumRecords: number;
  maximumFavorites: number;
}) {
  return normalizeSnapshot(
    await invoke<BackendSettingsSnapshot>('update_history_settings', {
      input: {
        auto_cleanup_enabled: input.autoCleanupEnabled,
        retention_days: input.retentionDays,
        maximum_records: input.maximumRecords,
        maximum_favorites: input.maximumFavorites,
      },
    }),
  );
}
