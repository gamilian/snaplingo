import { beforeEach, describe, expect, it, vi } from 'vitest';

const effectHarness = vi.hoisted(() => {
  let previousDeps: unknown[] | undefined;

  return {
    reset() {
      previousDeps = undefined;
    },
    useEffect(effect: () => void, deps: unknown[]) {
      const changed =
        !previousDeps ||
        deps.some((dependency, index) =>
          !Object.is(dependency, previousDeps?.[index]),
        );
      previousDeps = deps;
      if (changed) effect();
    },
  };
});

const runtime = vi.hoisted(() => ({
  prepareForReveal: vi.fn(),
  reveal: vi.fn(async () => undefined),
}));

vi.mock('react', async () => {
  const actual = await vi.importActual<typeof import('react')>('react');
  return {
    ...actual,
    useEffect: effectHarness.useEffect,
  };
});

vi.mock('./runtimeContext', () => ({
  useCaptureWorkspaceRuntime: () => runtime,
}));

import { useCaptureHostWindowReveal } from './captureHostRuntimeHooks';

describe('useCaptureHostWindowReveal', () => {
  beforeEach(() => {
    effectHarness.reset();
    vi.clearAllMocks();
  });

  it('does not start an overlapping reveal when the root rerenders during preparation', async () => {
    let finishPreparation: (() => void) | undefined;
    runtime.prepareForReveal.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          finishPreparation = resolve;
        }),
    );
    const options = {
      status: 'selecting' as const,
      sessionId: 'session-1',
      hasCaptureImagesReady: true,
      hasRevealedRef: { current: false },
      prepareSurface: vi.fn(async () => undefined),
      onRevealedSession: vi.fn(),
      onError: vi.fn(),
    };

    useCaptureHostWindowReveal(options);
    await Promise.resolve();
    useCaptureHostWindowReveal(options);
    await Promise.resolve();

    expect(runtime.prepareForReveal).toHaveBeenCalledTimes(1);

    finishPreparation?.();
    await Promise.resolve();
    await Promise.resolve();
  });
});
