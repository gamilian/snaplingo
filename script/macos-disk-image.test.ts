import { describe, expect, it } from 'vitest';

import {
  createTauriBuildEnvironment,
  findAttachedDiskImageDevices,
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
});

describe('Tauri build environment', () => {
  it('skips redundant Finder automation and repairs the unsupported macOS locale', () => {
    expect(createTauriBuildEnvironment('darwin', {
      CI: 'false',
      LANG: 'C.UTF-8',
      LC_ALL: 'C.UTF-8',
      TAURI_BUNDLER_DMG_IGNORE_CI: 'true',
    })).toEqual({
      CI: 'true',
      LANG: 'en_US.UTF-8',
      LC_ALL: 'en_US.UTF-8',
      TAURI_BUNDLER_DMG_IGNORE_CI: 'false',
    });
  });

  it('does not change the build environment on other platforms', () => {
    expect(createTauriBuildEnvironment('linux', { CI: 'false' })).toEqual({
      CI: 'false',
    });
  });
});
