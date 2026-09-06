import { describe, expect, it } from 'vitest';
import { assertMacOSDeploymentTarget } from './macos-release-verification.mjs';

describe('macOS bundled deployment targets', () => {
  it('rejects a dependency newer than the declared minimum OS', () => {
    expect(() => assertMacOSDeploymentTarget('cmd LC_BUILD_VERSION\n minos 14.0\n sdk 15.0', '11.0', 'libpng'))
      .toThrow(/libpng requires macOS 14.0/);
  });
  it('checks every architecture and the legacy load command without confusing SDK with minimum OS', () => {
    expect(() => assertMacOSDeploymentTarget('cmd LC_BUILD_VERSION\n minos 11.0\n sdk 15.0', '14.0', 'app')).not.toThrow();
    expect(() => assertMacOSDeploymentTarget('Load command 1\ncmd LC_VERSION_MIN_MACOSX\n version 15.0\n sdk 15.0', '14.0', 'app')).toThrow();
    expect(() => assertMacOSDeploymentTarget('minos 11.0\nminos 15.0', '14.0', 'app')).toThrow();
    expect(() => assertMacOSDeploymentTarget('minos 14.1', '14.0', 'app')).toThrow();
    expect(() => assertMacOSDeploymentTarget('sdk 14.0', '14.0', 'app')).toThrow(/missing/);
  });
});
