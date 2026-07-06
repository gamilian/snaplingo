import { describe, expect, it } from 'vitest';
import { shouldHydrateCaptureMagnifierPixels } from './captureMagnifierRuntime';

describe('captureMagnifierRuntime', () => {
  it('hydrates snapshot pixels only after the magnifier is explicitly requested', () => {
    expect(
      shouldHydrateCaptureMagnifierPixels({
        hasSession: true,
        hasHydratedPixelSource: false,
        isMagnifierRequested: true,
      }),
    ).toBe(true);

    expect(
      shouldHydrateCaptureMagnifierPixels({
        hasSession: true,
        hasHydratedPixelSource: false,
        isMagnifierRequested: false,
      }),
    ).toBe(false);

    expect(
      shouldHydrateCaptureMagnifierPixels({
        hasSession: true,
        hasHydratedPixelSource: true,
        isMagnifierRequested: true,
      }),
    ).toBe(false);
  });
});
