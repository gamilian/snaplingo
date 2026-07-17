import { describe, expect, it, vi } from 'vitest';
import {
  areRequiredPermissionsGranted,
  createRequiredPermissionsRuntime,
} from './runtime';

describe('required permissions runtime', () => {
  it('delegates the native permission request and requires every permission', async () => {
    const request = vi.fn(async () => ({
      screenRecording: true,
      accessibility: false,
    }));
    const runtime = createRequiredPermissionsRuntime({
      status: vi.fn(),
      request,
    });

    const status = await runtime.request();

    expect(request).toHaveBeenCalledOnce();
    expect(areRequiredPermissionsGranted(status)).toBe(false);
  });
});
