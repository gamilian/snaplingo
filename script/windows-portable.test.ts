import { spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import {
  createWindowsPortableArtifact,
  stageWindowsPortable,
} from './windows-portable.mjs';

const directories: string[] = [];
const config = JSON.parse(
  readFileSync(
    new URL('../src-tauri/tauri.conf.json', import.meta.url),
    'utf8',
  ),
);

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "snaplingo portable's "));
  directories.push(root);
  const write = (relative: string, contents = relative) => {
    const file = join(root, relative);
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, contents);
  };
  write('target/release/snaplingo.exe', 'MZ-test-executable');
  for (const file of [
    'small_det.mnn',
    'small_rec.mnn',
    'small_keys.txt',
    'licenses/mnn/LICENSE-MNN.txt',
    '.gitkeep',
  ]) {
    write(`src-tauri/resources/ocr/${file}`);
  }
  for (const file of [
    'vcruntime140.dll',
    'vcruntime140_1.dll',
    'msvcp140.dll',
    'msvcp140_1.dll',
  ]) {
    write(`src-tauri/resources/windows-crt/${file}`);
  }
  write('LICENSE', 'MIT license');
  write('src/domain/thirdPartyNotices.json', '{"components": []}');
  write('src-tauri/assets/fonts/LICENSE-NotoSans.txt', 'SIL Open Font License');
  write('docs/WINDOWS_PORTABLE.md', 'WebView2 required');
  return {
    context: {
      repositoryRoot: root,
      targetDirectory: join(root, 'target'),
      bundleDirectory: join(root, 'target/release/bundle'),
      packageName: 'snaplingo',
      productName: 'SnapLingo',
      version: '0.2.0',
      config,
    },
    destination: join(root, 'staged'),
    write,
  };
}

afterEach(() => {
  for (const directory of directories.splice(0))
    rmSync(directory, { recursive: true, force: true });
});

describe('Windows portable packaging', () => {
  it('keeps mapped OCR/runtime resources beside the executable and preserves licenses', () => {
    const { context, destination } = fixture();
    stageWindowsPortable(context, destination);
    expect(readFileSync(join(destination, 'snaplingo.exe'), 'utf8')).toBe(
      'MZ-test-executable',
    );
    expect(
      readFileSync(join(destination, 'ocr/small_det.mnn'), 'utf8'),
    ).toContain('small_det.mnn');
    expect(existsSync(join(destination, 'vcruntime140.dll'))).toBe(true);
    expect(existsSync(join(destination, 'ocr/licenses/mnn/LICENSE-MNN.txt'))).toBe(
      true,
    );
    expect(existsSync(join(destination, 'ocr/.gitkeep'))).toBe(false);
    expect(
      readFileSync(join(destination, 'licenses/NotoSans.txt'), 'utf8'),
    ).toContain('SIL');
    expect(readFileSync(join(destination, 'LICENSE'), 'utf8')).toContain('MIT');
    expect(readFileSync(join(destination, 'README.md'), 'utf8')).toContain(
      'WebView2',
    );
  });

  it.each([
    'resources/windows-crt/msvcp140.dll',
    'resources/ocr/small_rec.mnn',
  ])('rejects a missing dependency: %s', (relative) => {
    const { context, destination } = fixture();
    rmSync(join(context.repositoryRoot, 'src-tauri', relative));
    expect(() => stageWindowsPortable(context, destination)).toThrow(
      /dependency is missing or empty/,
    );
  });

  it('rejects empty models and invalid executable files', () => {
    const { context, destination, write } = fixture();
    write('src-tauri/resources/ocr/small_det.mnn', '');
    expect(() => stageWindowsPortable(context, destination)).toThrow(
      /small_det.mnn/,
    );
    write('target/release/snaplingo.exe', 'not-an-executable');
    expect(() => stageWindowsPortable(context, destination)).toThrow(
      /not a PE binary/,
    );
  });

  it.skipIf(process.platform !== 'win32')(
    'creates an extractable ZIP on Windows, including paths with spaces and apostrophes',
    () => {
      const { context } = fixture();
      const archive = createWindowsPortableArtifact(context);
      const extracted = join(context.repositoryRoot, 'extracted');
      const quote = (value: string) => `'${value.replaceAll("'", "''")}'`;
      const result = spawnSync(
        'powershell.exe',
        [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          `Expand-Archive -LiteralPath ${quote(archive)} -DestinationPath ${quote(extracted)}`,
        ],
        { encoding: 'utf8' },
      );
      expect(result.status, result.stderr).toBe(0);
      expect(
        readFileSync(
          join(extracted, 'SnapLingo_0.2.0/ocr/small_rec.mnn'),
          'utf8',
        ),
      ).toBe('src-tauri/resources/ocr/small_rec.mnn');
      expect(
        existsSync(join(context.bundleDirectory, 'portable/SnapLingo_0.2.0')),
      ).toBe(false);
    },
    60_000,
  );
});
