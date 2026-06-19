import { describe, expect, it } from 'vitest';
import { getCaptureRootClassName, shouldShowCaptureLoadingMask } from './capturePresentation';

describe('capture presentation', () => {
  it('keeps the capture surface transparent while the snapshot session loads', () => {
    expect(getCaptureRootClassName('loading')).not.toContain('bg-black');
    expect(shouldShowCaptureLoadingMask('loading')).toBe(false);
  });

  it('uses the same transparent capture surface once selection is active', () => {
    expect(getCaptureRootClassName('selecting')).toContain('bg-transparent');
    expect(getCaptureRootClassName('preview')).toContain('bg-transparent');
  });
});
