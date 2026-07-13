import { describe, expect, it } from 'vitest';

import { findAttachedDiskImageDevices } from './macos-disk-image.mjs';

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
});
