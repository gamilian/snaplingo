import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

const backendSnapshot = {
  general: {
    language: 'en',
    theme: 'dark',
    start_on_boot: true,
  },
  screenshot: {
    save_path: '/captures',
    format: 'webp',
    quality: 77,
    naming_rule: 'custom',
    custom_file_name: 'Work',
    auto_copy: true,
    default_stroke_width: 4,
    default_font_size: 20,
    remember_last_tool: false,
    show_selection_size: false,
    show_magnifier: true,
    magnifier_zoom: 16,
    pin_opacity: 80,
    pin_shadow: false,
    annotation_colors: [[255, 77, 79, 255]],
  },
  translation: {
    default_source_lang: 'ja',
    default_target_lang: 'fr',
    auto_translate: true,
    auto_copy: false,
    preserve_line_breaks: true,
    incremental_translation: false,
    window_always_on_top: true,
    hide_on_blur: false,
  },
  ocr: {
    recognition_language: 'auto',
    auto_copy: true,
    preserve_formatting: true,
    remove_chinese_spaces: true,
    show_confidence: false,
  },
  history: {
    auto_cleanup_enabled: true,
    retention_days: 45,
    maximum_records: 2000,
    maximum_favorites: 800,
  },
};

describe('Tauri settings command adapter', () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it('normalizes the backend settings snapshot', async () => {
    const { getSettingsSnapshot } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await expect(getSettingsSnapshot()).resolves.toEqual({
      general: {
        language: 'en',
        theme: 'dark',
        startOnBoot: true,
        proxyMode: 'system',
        proxyUrl: '',
        requestTimeoutMs: 10000,
        retryCount: 1,
        logLevel: 'info',
        logRetentionDays: 7,
        performanceMonitoring: false,
        experimentalGpuAcceleration: false,
        systemTtsVoice: '',
        systemTtsRate: 180,
      },
      screenshot: {
        savePath: '/captures',
        format: 'webp',
        quality: 77,
        namingRule: 'custom',
        customFileName: 'Work',
        autoCopy: true,
        defaultStrokeWidth: 4,
        defaultFontSize: 20,
        rememberLastTool: false,
        showSelectionSize: false,
        showMagnifier: true,
        magnifierZoom: 16,
        pinOpacity: 80,
        pinShadow: false,
        annotationColors: [[255, 77, 79, 255]],
        selectionBorderWidth: 2,
        selectionBorderColor: [91, 127, 255, 242],
        selectionMaskColor: [0, 0, 0, 46],
      },
      translation: {
        defaultSourceLang: 'ja',
        defaultTargetLang: 'fr',
        autoTranslate: true,
        autoCopy: false,
        preserveLineBreaks: true,
        incrementalTranslation: false,
        windowAlwaysOnTop: true,
        hideOnBlur: false,
        selectionWindowPosition: 'below-cursor',
        inputWindowPosition: 'center',
        selectionInputState: 'last',
        screenshotInputState: 'last',
        maxWindowHeightRatio: 70,
        windowWidth: 660,
        selectionTextMode: 'smart',
      },
      ocr: {
        recognitionLanguage: 'auto',
        preserveFormatting: true,
        removeChineseSpaces: true,
        showConfidence: false,
        windowPosition: 'cursor',
        hideSilentStatus: false,
      },
      history: {
        autoCleanupEnabled: true,
        retentionDays: 45,
        maximumRecords: 2000,
        maximumFavorites: 800,
      },
    });
    expect(invoke).toHaveBeenCalledWith('get_settings_snapshot');
  });

  it('maps general settings updates to the backend payload shape', async () => {
    const { updateGeneralSettings } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await updateGeneralSettings({
      language: 'en',
      theme: 'dark',
      startOnBoot: true,
    });

    expect(invoke).toHaveBeenCalledWith('update_general_settings', {
      input: {
        language: 'en',
        theme: 'dark',
        start_on_boot: true,
        proxy_mode: 'system',
        proxy_url: '',
        request_timeout_ms: 10000,
        retry_count: 1,
        log_level: 'info',
        log_retention_days: 7,
        performance_monitoring: false,
        experimental_gpu_acceleration: false,
        system_tts_voice: '',
        system_tts_rate: 180,
      },
    });
  });

  it('loads and clears SQLite application logs', async () => {
    const { clearAppLogs, listAppLogs } = await import('./settings');
    const logs = [
      {
        id: 1,
        timestamp: '2026-07-16T00:00:00Z',
        level: 'INFO' as const,
        target: 'capture',
        message: 'ready',
      },
    ];
    invoke.mockResolvedValueOnce(logs).mockResolvedValueOnce(undefined);

    await expect(listAppLogs(20)).resolves.toEqual(logs);
    await clearAppLogs();

    expect(invoke).toHaveBeenNthCalledWith(1, 'list_app_logs', { limit: 20 });
    expect(invoke).toHaveBeenNthCalledWith(2, 'clear_app_logs');
  });

  it('maps screenshot settings updates to the backend payload shape', async () => {
    const { updateScreenshotSettings } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await updateScreenshotSettings({
      savePath: '/captures',
      format: 'jpg',
      quality: 81,
      namingRule: 'date',
      customFileName: 'Capture',
      autoCopy: true,
      defaultStrokeWidth: 6,
      defaultFontSize: 16,
      rememberLastTool: false,
      showSelectionSize: false,
      showMagnifier: true,
      magnifierZoom: 14,
      pinOpacity: 75,
      pinShadow: false,
      annotationColors: [[12, 34, 56, 255]],
      selectionBorderWidth: 4,
      selectionBorderColor: [255, 77, 79, 242],
      selectionMaskColor: [32, 36, 44, 72],
    });

    expect(invoke).toHaveBeenCalledWith('update_screenshot_settings', {
      input: {
        save_path: '/captures',
        format: 'jpg',
        quality: 81,
        naming_rule: 'date',
        custom_file_name: 'Capture',
        auto_copy: true,
        default_stroke_width: 6,
        default_font_size: 16,
        remember_last_tool: false,
        show_selection_size: false,
        show_magnifier: true,
        magnifier_zoom: 14,
        pin_opacity: 75,
        pin_shadow: false,
        annotation_colors: [[12, 34, 56, 255]],
        selection_border_width: 4,
        selection_border_color: [255, 77, 79, 242],
        selection_mask_color: [32, 36, 44, 72],
      },
    });
  });

  it('updates only annotation colors through the narrow backend command', async () => {
    const { updateAnnotationColors } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);
    const colors: [number, number, number, number][] = [
      [12, 34, 56, 255],
      [200, 150, 100, 255],
    ];

    await updateAnnotationColors(colors);

    expect(invoke).toHaveBeenCalledWith('update_annotation_colors', {
      colors,
    });
  });

  it('maps translation settings updates to the backend payload shape', async () => {
    const { updateTranslationSettings } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await updateTranslationSettings({
      defaultSourceLang: 'auto',
      defaultTargetLang: 'en',
      autoTranslate: true,
      autoCopy: false,
      preserveLineBreaks: true,
      incrementalTranslation: false,
      windowAlwaysOnTop: true,
      hideOnBlur: false,
      selectionWindowPosition: 'cursor',
      inputWindowPosition: 'below-cursor',
      selectionInputState: 'collapsed',
      screenshotInputState: 'expanded',
      maxWindowHeightRatio: 80,
      windowWidth: 720,
      selectionTextMode: 'quality',
    });

    expect(invoke).toHaveBeenCalledWith('update_translation_settings', {
      input: {
        default_source_lang: 'auto',
        default_target_lang: 'en',
        auto_translate: true,
        auto_copy: false,
        preserve_line_breaks: true,
        incremental_translation: false,
        window_always_on_top: true,
        hide_on_blur: false,
        selection_window_position: 'cursor',
        input_window_position: 'below-cursor',
        selection_input_state: 'collapsed',
        screenshot_input_state: 'expanded',
        max_window_height_ratio: 80,
        window_width: 720,
        selection_text_mode: 'quality',
      },
    });
  });

  it('maps OCR settings updates to the backend payload shape', async () => {
    const { updateOcrSettings } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await updateOcrSettings({
      recognitionLanguage: 'ja',
      preserveFormatting: false,
      removeChineseSpaces: true,
      showConfidence: true,
      windowPosition: 'center',
      hideSilentStatus: true,
    });

    expect(invoke).toHaveBeenCalledWith('update_ocr_settings', {
      input: {
        recognition_language: 'ja',
        preserve_formatting: false,
        remove_chinese_spaces: true,
        show_confidence: true,
        window_position: 'center',
        hide_silent_status: true,
      },
    });
  });

  it('maps history cleanup settings to the backend payload shape', async () => {
    const { updateHistorySettings } = await import('./settings');
    invoke.mockResolvedValueOnce(backendSnapshot);

    await updateHistorySettings({
      autoCleanupEnabled: true,
      retentionDays: 45,
      maximumRecords: 2000,
      maximumFavorites: 800,
    });

    expect(invoke).toHaveBeenCalledWith('update_history_settings', {
      input: {
        auto_cleanup_enabled: true,
        retention_days: 45,
        maximum_records: 2000,
        maximum_favorites: 800,
      },
    });
  });
});
