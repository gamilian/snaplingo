import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';

import {
  assertUnifiedReleaseVersion,
  releaseArtifactContract,
  verifyReleaseArtifacts,
} from './release-verification.mjs';

const temporaryDirectories: string[] = [];

function temporaryBundleDirectory() {
  const directory = mkdtempSync(join(tmpdir(), 'snaplingo-release-'));
  temporaryDirectories.push(directory);
  return directory;
}

function writeArtifact(path: string, executable = false) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, 'artifact');
  if (executable) chmodSync(path, 0o755);
}

afterEach(() => {
  temporaryDirectories.splice(0).forEach((directory) => {
    rmSync(directory, { recursive: true, force: true });
  });
});

describe('release version contract', () => {
  it('requires package, Cargo, and Tauri versions to match', () => {
    expect(
      assertUnifiedReleaseVersion({
        'package.json': '0.2.0',
        'Cargo.toml': '0.2.0',
        'tauri.conf.json': '0.2.0',
      }),
    ).toBe('0.2.0');

    expect(() =>
      assertUnifiedReleaseVersion({
        'package.json': '0.2.0',
        'Cargo.toml': '0.2.1',
        'tauri.conf.json': '0.2.0',
      }),
    ).toThrow(/Release versions must match/);
  });
});

describe('release artifact contract', () => {
  it('verifies macOS application and disk image output', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(
      join(
        bundleDirectory,
        'macos',
        'SnapLingo.app',
        'Contents',
        'MacOS',
        'snaplingo',
      ),
      true,
    );
    writeArtifact(join(bundleDirectory, 'dmg', 'rw.temporary.dmg'));
    writeArtifact(join(bundleDirectory, 'dmg', 'SnapLingo_0.2.0_aarch64.dmg'));

    expect(
      verifyReleaseArtifacts({
        platform: 'darwin',
        bundleDirectory,
        productName: 'SnapLingo',
        version: '0.2.0',
      }).map(({ kind }) => kind),
    ).toEqual(['macOS application', 'macOS disk image']);
  });

  it('verifies Linux AppImage and Debian output', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(
      join(bundleDirectory, 'appimage', 'SnapLingo_0.2.0_amd64.AppImage'),
      true,
    );
    writeArtifact(join(bundleDirectory, 'deb', 'snaplingo_0.2.0_amd64.deb'));

    expect(
      verifyReleaseArtifacts({
        platform: 'linux',
        bundleDirectory,
        productName: 'SnapLingo',
        version: '0.2.0',
      }),
    ).toHaveLength(2);
  });

  it('verifies Windows MSI and NSIS output', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(join(bundleDirectory, 'msi', 'SnapLingo_0.2.0_x64_en-US.msi'));
    writeArtifact(join(bundleDirectory, 'nsis', 'SnapLingo_0.2.0_x64-setup.exe'));

    expect(
      verifyReleaseArtifacts({
        platform: 'win32',
        bundleDirectory,
        productName: 'SnapLingo',
        version: '0.2.0',
      }),
    ).toHaveLength(2);
  });

  it('fails when a required platform artifact is missing', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(join(bundleDirectory, 'msi', 'SnapLingo_0.2.0_x64_en-US.msi'));

    expect(() =>
      verifyReleaseArtifacts({
        platform: 'win32',
        bundleDirectory,
        productName: 'SnapLingo',
        version: '0.2.0',
      }),
    ).toThrow(/Windows NSIS installer directory was not produced/);
  });

  it('rejects an installer left behind by an older version', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(join(bundleDirectory, 'msi', 'SnapLingo_0.1.0_x64_en-US.msi'));
    writeArtifact(join(bundleDirectory, 'nsis', 'SnapLingo_0.2.0_x64-setup.exe'));

    expect(() =>
      verifyReleaseArtifacts({
        platform: 'win32',
        bundleDirectory,
        productName: 'SnapLingo',
        version: '0.2.0',
      }),
    ).toThrow(/Windows MSI installer was not found/);
  });

  it('rejects unsupported release platforms', () => {
    expect(() => releaseArtifactContract('freebsd', 'SnapLingo')).toThrow(
      /Unsupported release platform/,
    );
  });
});
