import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const screenshotEditorSettings = readFileSync(
  new URL('../views/SettingsWindow/Screenshot/EditorPage.tsx', import.meta.url),
  'utf8',
);
const captureRuntime = readFileSync(
  new URL('./capture-workspace/runtime.ts', import.meta.url),
  'utf8',
);

describe('frontend architecture boundaries', () => {
  it('shares annotation color vocabulary without coupling application or settings to capture views', () => {
    expect(screenshotEditorSettings).not.toContain('../../CaptureWorkspace/');
    expect(captureRuntime).toContain("from '../../domain/annotationColor'");
  });
});
