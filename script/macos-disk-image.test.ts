import { describe, expect, it } from 'vitest';

import {
  findAttachedDiskImageDevices,
  retryDiskImageAttach,
} from './macos-disk-image.mjs';

describe('macOS disk image attachments', () => {
  it('finds only the whole-disk device attached from the requested image path', () => {
    const info = `
================================================
image-path      : /tmp/other.dmg
/dev/disk4\tGUID_partition_scheme
/dev/disk4s1\tApple_APFS
================================================
image-path      : /tmp/SnapLingo.dmg
/dev/disk5\tGUID_partition_scheme
/dev/disk5s1\tApple_APFS
/dev/disk8\tApple_APFS
`;

    expect(findAttachedDiskImageDevices(info, '/tmp/SnapLingo.dmg')).toEqual([
      '/dev/disk5',
    ]);
  });

  it('retries a transient attach failure before returning the mount output', () => {
    let calls = 0;
    const failures: number[] = [];

    const output = retryDiskImageAttach(
      () => {
        calls += 1;
        if (calls < 3) throw new Error('resource temporarily unavailable');
        return '/Volumes/SnapLingo';
      },
      (attempt) => failures.push(attempt),
    );

    expect(output).toBe('/Volumes/SnapLingo');
    expect(failures).toEqual([1, 2]);
  });

  it('keeps the last attach failure', () => {
    expect(() =>
      retryDiskImageAttach(
        () => {
          throw new Error('still unavailable');
        },
        () => {},
        2,
      ),
    ).toThrow('still unavailable');
  });
});
