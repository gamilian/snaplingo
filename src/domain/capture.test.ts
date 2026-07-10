import { describe, expect, it } from 'vitest';
import { CAPTURE_MODES } from './capture';

describe('capture domain types', () => {
  it('uses backend IPC capture mode strings', () => {
    expect(CAPTURE_MODES).toEqual([
      'screenshot',
      'screenshot-copy',
      'screenshot-ocr',
      'silent-screenshot-ocr',
      'screenshot-translate',
    ]);
    expect(CAPTURE_MODES).not.toContain('Screenshot');
    expect(CAPTURE_MODES).not.toContain('OcrTranslate');
  });
});
