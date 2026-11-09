import { describe, expect, it } from 'vitest';
import { join } from 'node:path';
import { nativeDebugApplication } from './native-run.mjs';

describe('native development executable', () => {
  it('resolves Cargo debug outputs for supported platforms', () => {
    expect(nativeDebugApplication('/build target', 'darwin')).toBe(join('/build target', 'debug/snaplingo-native'));
    expect(nativeDebugApplication('/build target', 'win32')).toBe(join('/build target', 'debug/snaplingo-native.exe'));
    expect(() => nativeDebugApplication('/build target', 'linux')).toThrow('supports macOS and Windows');
  });
});
