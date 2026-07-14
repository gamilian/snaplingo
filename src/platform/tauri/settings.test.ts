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
    pin_opacity: 80,
    pin_shadow: false,
    annotation_colors: [[255, 77, 79, 255]],
  },
  translation: {
    default_source_lang: 'ja',
    default_target_lang: 'fr',
  },
  history: {
    auto_cleanup_enabled: true,
    retention_days: 45,
    maximum_records: 2000,
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
        pinOpacity: 80,
        pinShadow: false,
        annotationColors: [[255, 77, 79, 255]],
      },
      translation: {
        defaultSourceLang: 'ja',
        defaultTargetLang: 'fr',
      },
      history: {
        autoCleanupEnabled: true,
        retentionDays: 45,
        maximumRecords: 2000,
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
      },
    });
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
      pinOpacity: 75,
      pinShadow: false,
      annotationColors: [[12, 34, 56, 255]],
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
        pin_opacity: 75,
        pin_shadow: false,
        annotation_colors: [[12, 34, 56, 255]],
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
    });

    expect(invoke).toHaveBeenCalledWith('update_translation_settings', {
      input: {
        default_source_lang: 'auto',
        default_target_lang: 'en',
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
    });

    expect(invoke).toHaveBeenCalledWith('update_history_settings', {
      input: {
        auto_cleanup_enabled: true,
        retention_days: 45,
        maximum_records: 2000,
      },
    });
  });
});
