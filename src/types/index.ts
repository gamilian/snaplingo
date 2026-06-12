// Shared types between frontend and backend

export enum CaptureMode {
  Screenshot = "Screenshot",
  Ocr = "Ocr",
  OcrTranslate = "OcrTranslate",
  SelectionTranslate = "SelectionTranslate",
  InputTranslate = "InputTranslate",
}

export interface ImageData {
  data: number[];
  width: number;
  height: number;
}

export interface Config {
  version: string;
  general: GeneralConfig;
  screenshot: ScreenshotConfig;
  ocr: OcrConfig;
  translation: TranslationConfig;
  hotkeys: HotkeysConfig;
  history: HistoryConfig;
}

export interface GeneralConfig {
  language: string;
  theme: string;
  start_on_boot: boolean;
}

export interface ScreenshotConfig {
  default_save_path: string;
  format: string;
  quality: number;
}

export interface OcrConfig {
  active_provider: string | null;
}

export interface TranslationConfig {
  active_providers: string[];
  default_target_language: string;
}

export interface HotkeysConfig {
  screenshot: string;
  ocr: string;
  ocr_translate: string;
  selection_translate: string;
  input_translate: string;
}

export interface HistoryConfig {
  record_screenshot: boolean;
  record_ocr: boolean;
  record_translation: boolean;
  auto_cleanup_enabled: boolean;
  max_age_days: number;
  max_entries: number;
}

export interface ProviderInfo {
  id: string;
  name: string;
  provider_type: "ocr" | "translation";
  is_active: boolean;
  requires_api_key: boolean;
}

export interface TranslationResult {
  provider_id: string;
  provider_name: string;
  text: string;
  error?: string;
}

export interface HistoryEntry {
  id: number;
  timestamp: number;
  capture_mode: string;
  thumbnail?: number[];
  source_text?: string;
  translations: Translation[];
}

export interface Translation {
  provider_id: string;
  text: string;
}
