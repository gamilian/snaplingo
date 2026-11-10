import { createHash } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import { copyFile, mkdir, readFile, readdir, rename, rm, stat, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { spawnSync } from 'node:child_process';
import { copyWindowsCrt, findWindowsCrt } from './windows-ocr-runtime.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const manifest = JSON.parse(await readFile(join(root, 'script/ocr-assets.json'), 'utf8'));
const targetNames = {
  'aarch64-apple-darwin': 'darwin-arm64',
  'x86_64-apple-darwin': 'darwin-x64',
  'x86_64-pc-windows-msvc': 'win32-x64',
  'x86_64-unknown-linux-gnu': 'linux-x64',
};

export function ocrResourceFiles(platform) {
  return [
    ...manifest.models.filter(asset => asset.platforms.includes(platform)).map(asset => asset.file),
    ...manifest.archives.filter(asset => asset.platform === platform || asset.platforms?.includes(platform)).flatMap(asset => Object.values(asset.files)),
  ];
}

export async function sha256(path) {
  const digest = createHash('sha256');
  for await (const chunk of createReadStream(path)) digest.update(chunk);
  return digest.digest('hex');
}

async function exists(path) {
  try { return (await stat(path)).isFile(); } catch { return false; }
}

async function cachedDownload(asset, cache) {
  const destination = join(cache, `${asset.sha256}-${new URL(asset.url).pathname.split('/').pop()}`);
  if (await exists(destination) && await sha256(destination) === asset.sha256) return destination;
  const partial = `${destination}.partial-${process.pid}`;
  try {
    console.log(`[ocr] Downloading ${new URL(asset.url).pathname.split('/').pop()}`);
    const response = await fetch(asset.url);
    if (!response.ok || !response.body) throw new Error(`OCR download failed: HTTP ${response.status}`);
    await pipeline(Readable.fromWeb(response.body), createWriteStream(partial));
    if (await sha256(partial) !== asset.sha256) throw new Error(`OCR asset checksum mismatch: ${asset.url}`);
    await rm(destination, { force: true });
    await rename(partial, destination);
    return destination;
  } finally { await rm(partial, { force: true }); }
}

function extract(archive, destination) {
  const result = process.platform === 'win32' && !archive.endsWith('.tgz')
    ? spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', 'Expand-Archive -LiteralPath $env:SNAPLINGO_OCR_ARCHIVE -DestinationPath $env:SNAPLINGO_OCR_EXTRACT -Force'], {
        env: { ...process.env, SNAPLINGO_OCR_ARCHIVE: archive, SNAPLINGO_OCR_EXTRACT: destination }, encoding: 'utf8',
      })
    : spawnSync('tar', ['-xf', archive, '-C', destination], { encoding: 'utf8' });
  if (result.error || result.status !== 0) throw new Error(`Could not unpack OCR runtime: ${result.error ?? result.stderr}`);
}

export async function prepareOcr(platform, destination = join(root, 'app/resources/ocr'), cache = process.env.SNAPLINGO_OCR_ASSET_CACHE ?? join(homedir(), '.cache/snaplingo/ocr')) {
  if (!['darwin-arm64','darwin-x64','win32-x64','linux-x64'].includes(platform)) throw new Error(`Unsupported OCR target: ${platform}`);
  await mkdir(destination, { recursive: true });
  await mkdir(cache, { recursive: true });
  const platformFile = join(destination, '.platform');
  const previous = await readFile(platformFile, 'utf8').catch(() => null);
  if (previous && previous !== platform) {
    await rm(join(destination, 'runtime'), { recursive: true, force: true });
    await rm(join(destination, 'licenses'), { recursive: true, force: true });
    for (const asset of manifest.models.filter(asset => !asset.platforms.includes(platform))) {
      await rm(join(destination, asset.file), { force: true });
    }
  }
  for (const file of await readdir(destination)) {
    if (/^runtime-[a-f0-9]+\.json$/.test(file)) await rm(join(destination, file));
  }
  for (const asset of manifest.models.filter(asset => asset.platforms.includes(platform))) {
    const output = join(destination, asset.file);
    if (await exists(output) && await sha256(output) === asset.sha256) continue;
    await mkdir(dirname(output), { recursive: true });
    await copyFile(await cachedDownload(asset, cache), output);
  }
  for (const asset of manifest.archives.filter(asset => asset.platform === platform || asset.platforms?.includes(platform))) {
    const marker = join(cache, `prepared-${asset.sha256}.json`);
    if (await exists(marker)) {
      const hashes = JSON.parse(await readFile(marker, 'utf8'));
      const valid = await Promise.all(Object.values(asset.files).map(async file => await exists(join(destination,file)) && await sha256(join(destination,file)) === hashes[file]));
      if (valid.every(Boolean)) continue;
    }
    const archive = await cachedDownload(asset, cache);
    const unpacked = join(cache, `unpacked-${asset.sha256}-${process.pid}`);
    await mkdir(unpacked, { recursive: true });
    try {
      // Expand-Archive requires a .zip extension, including for NuGet packages.
      const zip = archive.endsWith('.nupkg') ? `${archive}.zip` : archive;
      if (zip !== archive) await copyFile(archive, zip);
      extract(zip, unpacked);
      const hashes = {};
      for (const [source, file] of Object.entries(asset.files)) {
        const output = join(destination, file);
        await mkdir(dirname(output), { recursive: true });
        await copyFile(join(unpacked, source), output);
        hashes[file] = await sha256(output);
      }
      await writeFile(marker, `${JSON.stringify(hashes, null, 2)}\n`);
    } finally {
      await rm(unpacked, { recursive: true, force: true });
      if (archive.endsWith('.nupkg')) await rm(`${archive}.zip`, { force: true });
    }
  }
  await writeFile(platformFile, platform);
  if (platform === 'win32-x64' && process.platform === 'win32') {
    await copyWindowsCrt(await findWindowsCrt(), join(dirname(destination), 'windows-crt'));
  }
  console.log(`[ocr] PP-OCRv6 medium assets ready (${platform}).`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const index = process.argv.indexOf('--target');
  const target = index >= 0 ? process.argv[index + 1] : process.env.CARGO_BUILD_TARGET;
  await prepareOcr(targetNames[target] ?? target ?? `${process.platform}-${process.arch}`);
}
