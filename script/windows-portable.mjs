import { spawnSync } from 'node:child_process';
import { basename, dirname, join } from 'node:path';
import {
  copyFileSync,
  cpSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
} from 'node:fs';

export function stageWindowsPortable(context, destination) {
  const root = context.repositoryRoot;
  const executable = join(
    context.targetDirectory,
    'release',
    `${context.packageName}.exe`,
  );
  if (readFileSync(executable).subarray(0, 2).toString() !== 'MZ') {
    throw new Error('Windows portable executable is not a PE binary');
  }
  rmSync(destination, { recursive: true, force: true });
  mkdirSync(destination, { recursive: true });
  copyFileSync(executable, join(destination, `${context.packageName}.exe`));
  for (const [source, target] of Object.entries(
    context.config.bundle.resources,
  )) {
    const output = join(destination, target);
    mkdirSync(dirname(output), { recursive: true });
    cpSync(join(root, 'src-tauri', source), output, {
      recursive: true,
      filter: (path) => basename(path) !== '.gitkeep',
    });
  }
  for (const relative of [
    'vcruntime140.dll',
    'vcruntime140_1.dll',
    'msvcp140.dll',
    'msvcp140_1.dll',
    'ocr/medium_det.onnx',
    'ocr/medium_rec.onnx',
    'ocr/keys.txt',
    'ocr/runtime/onnxruntime.dll',
    'ocr/runtime/DirectML.dll',
  ]) {
    const file = join(destination, relative);
    const metadata = statSync(file, { throwIfNoEntry: false });
    if (!metadata?.isFile() || metadata.size === 0) {
      throw new Error(
        `Windows portable dependency is missing or empty: ${relative}`,
      );
    }
  }
  mkdirSync(join(destination, 'licenses'), { recursive: true });
  copyFileSync(join(root, 'LICENSE'), join(destination, 'LICENSE'));
  copyFileSync(join(root, 'src/domain/thirdPartyNotices.json'), join(destination, 'licenses/THIRD_PARTY_NOTICES.json'));
  copyFileSync(
    join(root, 'src-tauri/assets/fonts/LICENSE-NotoSans.txt'),
    join(destination, 'licenses/NotoSans.txt'),
  );
  copyFileSync(
    join(root, 'docs/WINDOWS_PORTABLE.md'),
    join(destination, 'README.md'),
  );
}

export function createWindowsPortableArtifact(context) {
  const output = join(context.bundleDirectory, 'portable');
  const name = `${context.productName}_${context.version}`;
  const directory = join(output, name);
  const archive = join(output, `${name}_${process.arch}-portable.zip`);
  stageWindowsPortable(context, directory);
  rmSync(archive, { force: true });
  // PowerShell and .NET ship with supported Windows versions. Quote literal paths
  // separately from shell syntax, including checkout paths containing apostrophes.
  const quote = (value) => `'${value.replaceAll("'", "''")}'`;
  try {
    const result = spawnSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-NonInteractive',
        '-Command',
        [
          "$ErrorActionPreference = 'Stop'",
          'Add-Type -AssemblyName System.IO.Compression.FileSystem',
          `[IO.Compression.ZipFile]::CreateFromDirectory(${quote(directory)}, ${quote(archive)}, [IO.Compression.CompressionLevel]::Optimal, $true)`,
        ].join('; '),
      ],
      { encoding: 'utf8' },
    );
    if (result.error) throw result.error;
    if (result.status !== 0)
      throw new Error(`Windows portable ZIP creation failed: ${result.stderr}`);
    return archive;
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}
