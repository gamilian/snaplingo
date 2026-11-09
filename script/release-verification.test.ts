import {
  chmodSync,
  readFileSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';

import {
  collectArtifacts,
  assertUnifiedReleaseVersion,
  assertMatchingReleaseTag,
  releaseAssetName,
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
  it('requires package and Cargo versions to match', () => {
    expect(
      assertUnifiedReleaseVersion({
        'package.json': '0.2.0',
        'Cargo.toml': '0.2.0',
      }),
    ).toBe('0.2.0');

    expect(() =>
      assertUnifiedReleaseVersion({
        'package.json': '0.2.0',
        'Cargo.toml': '0.2.1',
      }),
    ).toThrow(/Release versions must match/);
  });

  it('requires a tag to match the unified release version', () => {
    expect(() => assertMatchingReleaseTag('0.2.0', {
      GITHUB_REF_TYPE: 'tag',
      GITHUB_REF_NAME: 'v0.2.0',
    })).not.toThrow();
    expect(() => assertMatchingReleaseTag('0.2.0', {
      GITHUB_REF_TYPE: 'tag',
      GITHUB_REF_NAME: 'v0.2.1',
    })).toThrow(/must match version v0.2.0/);
    expect(() => assertMatchingReleaseTag('0.2.0', {
      GITHUB_REF_TYPE: 'branch',
      GITHUB_REF_NAME: 'master',
    })).not.toThrow();
  });
});

describe('release artifact contract', () => {
  it('uses consistent, release-friendly names for distributed assets', () => {
    expect(releaseAssetName({
      projectName: 'snaplingo',
      version: '0.2.0',
      target: 'macos-aarch64',
      kind: 'macOS disk image',
    })).toBe('snaplingo-v0.2.0-macos-aarch64.dmg');
    expect(releaseAssetName({
      projectName: 'snaplingo',
      version: '0.2.0',
      target: 'windows-x86_64',
      kind: 'Windows NSIS installer',
    })).toBe('snaplingo-v0.2.0-windows-x86_64-setup.exe');
    expect(releaseAssetName({
      projectName: 'snaplingo',
      version: '0.2.0',
      target: 'windows-x86_64',
      kind: 'Windows MSI installer',
    })).toBe('snaplingo-v0.2.0-windows-x86_64.msi');
    expect(releaseAssetName({
      projectName: 'snaplingo', version: '0.2.0', target: 'windows-x86_64',
      kind: 'Windows portable archive',
    })).toBe('snaplingo-v0.2.0-windows-x86_64-portable.zip');
  });

  it('verifies macOS application and disk image output', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(
      join(
        bundleDirectory,
        'SnapLingo.app',
        'Contents',
        'MacOS',
        'snaplingo',
      ),
      true,
    );
    writeArtifact(join(bundleDirectory, 'bundle/dmg', 'rw.temporary.dmg'));
    writeArtifact(join(bundleDirectory, 'bundle/dmg', 'snaplingo-v0.2.0-macos-aarch64.dmg'));

    expect(
      verifyReleaseArtifacts({
        platform: 'darwin',
        bundleDirectory,
        productName: 'SnapLingo',
        version: '0.2.0',
      }).map(({ kind }) => kind),
    ).toEqual(['macOS application', 'macOS disk image']);
  });

  it('does not accept retired Linux bundles as a supported native release', () => {
    expect(() => releaseArtifactContract('linux', 'SnapLingo', '0.2.0')).toThrow(/Unsupported release platform/);
  });

  it('verifies Windows MSI, NSIS, and portable output', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(join(bundleDirectory, 'bundle/msi', 'SnapLingo_0.2.0_x64.msi'));
    writeArtifact(join(bundleDirectory, 'bundle/nsis', 'SnapLingo_0.2.0_x64-setup.exe'));
    writeArtifact(join(bundleDirectory, 'bundle/portable', 'SnapLingo_0.2.0_x64-portable.zip'));

    expect(
      verifyReleaseArtifacts({
        platform: 'win32',
        bundleDirectory,
        productName: 'SnapLingo',
        version: '0.2.0',
      }),
    ).toHaveLength(3);
  });

  it('requires the portable archive from the current version', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(join(bundleDirectory, 'bundle/msi', 'SnapLingo_0.2.0_x64.msi'));
    writeArtifact(join(bundleDirectory, 'bundle/nsis', 'SnapLingo_0.2.0_x64-setup.exe'));
    writeArtifact(join(bundleDirectory, 'bundle/portable', 'SnapLingo_0.1.0_x64-portable.zip'));
    expect(() => verifyReleaseArtifacts({
      platform: 'win32', bundleDirectory, productName: 'SnapLingo', version: '0.2.0',
    })).toThrow(/Windows portable archive was not found/);
  });

  it('fails when a required platform artifact is missing', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(join(bundleDirectory, 'bundle/msi', 'SnapLingo_0.2.0_x64.msi'));

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
    writeArtifact(join(bundleDirectory, 'bundle/msi', 'SnapLingo_0.1.0_x64.msi'));
    writeArtifact(join(bundleDirectory, 'bundle/nsis', 'SnapLingo_0.2.0_x64-setup.exe'));

    expect(() =>
      verifyReleaseArtifacts({
        platform: 'win32',
        bundleDirectory,
        productName: 'SnapLingo',
        version: '0.2.0',
      }),
    ).toThrow(/Windows MSI installer was not found/);
  });

  it('does not mistake version 10.2.0 for 0.2.0', () => {
    const bundleDirectory = temporaryBundleDirectory();
    writeArtifact(join(bundleDirectory, 'bundle/msi', 'SnapLingo_10.2.0_x64.msi'));
    writeArtifact(join(bundleDirectory, 'bundle/nsis', 'SnapLingo_10.2.0_x64-setup.exe'));
    expect(() => verifyReleaseArtifacts({
      platform: 'win32', bundleDirectory, productName: 'SnapLingo', version: '0.2.0',
    })).toThrow(/Windows MSI installer was not found/);
  });

  it('rejects unsupported release platforms', () => {
    expect(() => releaseArtifactContract('freebsd', 'SnapLingo')).toThrow(
      /Unsupported release platform/,
    );
  });
});


describe('native release collection', () => {
  it('keeps updater filenames and hashes the exact copied native installer bytes', () => {
    const root = temporaryBundleDirectory();
    const bundleDirectory = join(root, 'target/native/release');
    writeArtifact(join(bundleDirectory, 'bundle/msi', 'SnapLingo_0.2.0_x64.msi'));
    writeArtifact(join(bundleDirectory, 'bundle/nsis', 'SnapLingo_0.2.0_x64-setup.exe'));
    writeArtifact(join(bundleDirectory, 'bundle/portable', 'SnapLingo_0.2.0_x64-portable.zip'));
    const artifacts = verifyReleaseArtifacts({ platform: 'win32', bundleDirectory, productName: 'SnapLingo', version: '0.2.0' });
    collectArtifacts({ repositoryRoot: root, packageName: 'snaplingo', version: '0.2.0' }, artifacts, {
      SNAPLINGO_RELEASE_TARGET: 'windows-x86_64', GITHUB_SHA: 'fixture-commit',
    });
    const prefix = 'snaplingo-v0.2.0-windows-x86_64';
    const checksum = readFileSync(join(root, 'release-assets', `${prefix}.SHA256SUMS.txt`), 'utf8');
    for (const suffix of ['.msi', '-setup.exe', '-portable.zip']) {
      const name = `${prefix}${suffix}`;
      const bytes = readFileSync(join(root, 'release-assets', name));
      expect(checksum).toContain(`${createHash('sha256').update(bytes).digest('hex')}  ${name}`);
    }
    expect(JSON.parse(readFileSync(join(root, 'release-assets', `${prefix}.build.json`), 'utf8'))).toMatchObject({ version: '0.2.0', commit: 'fixture-commit' });
    expect(() => collectArtifacts({ repositoryRoot: root, packageName: 'snaplingo', version: '0.2.0' }, artifacts, {
      SNAPLINGO_RELEASE_TARGET: 'windows-x86_64-offline', GITHUB_SHA: 'fixture-commit',
    })).toThrow('Invalid release target');
  });
});
