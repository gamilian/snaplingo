import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const screenshotSettings = readFileSync(
  new URL(
    '../views/SettingsWindow/Screenshot/ScreenshotSettingsPage.tsx',
    import.meta.url,
  ),
  'utf8',
);
const captureRuntime = readFileSync(
  new URL('./capture-workspace/runtime.ts', import.meta.url),
  'utf8',
);
const appShell = readFileSync(
  new URL('../../src-tauri/src/app_shell.rs', import.meta.url),
  'utf8',
);

describe('frontend architecture boundaries', () => {
  it('keeps screenshot settings and application preferences independent from capture views', () => {
    expect(screenshotSettings).not.toContain('../../CaptureWorkspace/');
    expect(captureRuntime).toContain("from '../../domain/annotationColor'");
  });

  it('keeps retired input translation out of the menu-bar shell', () => {
    expect(appShell).not.toContain('input-translation');
    expect(appShell).not.toContain('Input Translation');
  });

  it('keeps ordinary OCR workflows free of hidden auto-copy preferences', () => {
    expect(captureRuntime).not.toContain('ocrSettings?.autoCopy');
  });
});
