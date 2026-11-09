import { spawnSync } from 'node:child_process';
import { chmodSync, copyFileSync, cpSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { prepareOcr, sha256 } from './prepare-ocr.mjs';
import { verifyMacOSApplication } from './macos-release-verification.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
function run(command, args, environment = process.env) {
  const result = spawnSync(command, args, { cwd: root, env: environment, stdio: 'inherit' });
  if (result.error || result.status !== 0) throw new Error(`${command} failed: ${result.error?.message ?? result.status}`);
}
function required(path) {
  if (!statSync(path, { throwIfNoEntry: false })?.size) throw new Error(`Missing native bundle resource: ${path}`);
}
function xml(text) {
  return String(text).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
}
export function nativeInfoPlist(config) {
  const entries = {
    CFBundleDevelopmentRegion: 'en', CFBundleExecutable: 'snaplingo-native',
    CFBundleIdentifier: config.identifier, CFBundleName: config.productName,
    CFBundleDisplayName: config.productName, CFBundlePackageType: 'APPL',
    CFBundleShortVersionString: config.version, CFBundleVersion: config.version,
    CFBundleIconFile: 'icon.icns', LSMinimumSystemVersion: config.bundle.macOS.minimumSystemVersion,
    NSScreenCaptureUsageDescription: 'SnapLingo needs screen recording permission to capture screenshots for OCR and translation.',
    NSScreenCaptureDescription: 'SnapLingo captures your screen to provide screenshot, OCR, and translation features.',
    NSAppleEventsUsageDescription: 'SnapLingo uses browser automation to read selected text when you invoke selection translation.',
  };
  return `<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0"><dict>\n${Object.entries(entries).map(([key, value]) => `<key>${key}</key><string>${xml(value)}</string>`).join('\n')}\n<key>LSUIElement</key><true/>\n<key>NSHighResolutionCapable</key><true/>\n</dict></plist>\n`;
}
export function stageNativeMacBundle(repositoryRoot, binary, destination, config) {
  const appRoot = join(repositoryRoot, 'crates/snaplingo-app');
  const resources = join(appRoot, 'resources/ocr');
  required(binary);
  for (const path of ['medium_det.mnn', 'medium_rec.mnn', 'keys.txt', 'runtime/libonnxruntime.dylib']) required(join(resources, path));
  if (readFileSync(join(resources, '.platform'), 'utf8').trim() !== `darwin-${process.arch}`) throw new Error('OCR resources do not match the native build architecture');
  required(join(appRoot, 'icons/icon.icns'));
  // Only the generated application directory is replaced; build caches and
  // installed applications have separate ownership.
  rmSync(destination, { recursive: true, force: true });
  const contents = join(destination, 'Contents');
  mkdirSync(join(contents, 'MacOS'), { recursive: true });
  mkdirSync(join(contents, 'Resources/licenses'), { recursive: true });
  copyFileSync(binary, join(contents, 'MacOS/snaplingo-native'));
  chmodSync(join(contents, 'MacOS/snaplingo-native'), 0o755);
  cpSync(resources, join(contents, 'Resources/ocr'), { recursive: true, filter: path => !path.endsWith('/.gitkeep') });
  copyFileSync(join(appRoot, 'icons/icon.icns'), join(contents, 'Resources/icon.icns'));
  copyFileSync(join(repositoryRoot, 'LICENSE'), join(contents, 'Resources/licenses/LICENSE'));
  copyFileSync(join(repositoryRoot, 'assets/fonts/LICENSE-NotoSans.txt'), join(contents, 'Resources/licenses/NotoSans.txt'));
  copyFileSync(join(repositoryRoot, 'src/domain/thirdPartyNotices.json'), join(contents, 'Resources/licenses/THIRD_PARTY_NOTICES.json'));
  writeFileSync(join(contents, 'Info.plist'), nativeInfoPlist(config));
  return destination;
}
async function main(args) {
  const command = args[0] ?? 'build';
  if (!['build', 'bundle'].includes(command) || args.slice(1).some(a => a !== '--debug')) throw new Error('Usage: node script/native-build.mjs [build|bundle] [--debug]');
  const config = { ...JSON.parse(readFileSync(join(root, 'crates/snaplingo-app/desktop.json'), 'utf8')), version: JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')).version };
  const cargoVersion = readFileSync(join(root, 'crates/snaplingo-app/Cargo.toml'), 'utf8').match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (cargoVersion !== config.version) throw new Error(`Native application version ${cargoVersion} differs from package version ${config.version}`);
  const profile = args.includes('--debug') ? 'debug' : 'release';
  const targetDirectory = process.env.CARGO_TARGET_DIR ? resolve(root, process.env.CARGO_TARGET_DIR) : join(root, 'target');
  if (process.platform === 'win32') {
    const { buildNativeWindows } = await import('./native-windows-build.mjs');
    return buildNativeWindows({ repositoryRoot: root, command, profile, config, targetDirectory });
  }
  if (process.platform !== 'darwin') throw new Error('Native application bundling supports macOS and Windows');
  const environment = { ...process.env, MACOSX_DEPLOYMENT_TARGET: config.bundle.macOS.minimumSystemVersion };
  if (command === 'build') {
    await prepareOcr(`darwin-${process.arch}`);
    run('node', ['script/third-party-notices.mjs', '--check']);
    run('cargo', ['build', '--locked', '-p', 'snaplingo', '--no-default-features', '--features', 'native-app', '--bin', 'snaplingo-native', ...(profile === 'release' ? ['--release'] : [])], environment);
  }
  const destination = join(targetDirectory, 'native', profile, `${config.productName}.app`);
  stageNativeMacBundle(root, join(targetDirectory, profile, 'snaplingo-native'), destination, config);
  const manifest = join(targetDirectory, 'native', profile, 'desktop.json');
  writeFileSync(manifest, `${JSON.stringify(config, null, 2)}\n`);
  const dmg = join(targetDirectory, 'native', profile, 'bundle/dmg', `snaplingo-v${config.version}-macos-${process.arch === 'arm64' ? 'aarch64' : 'x86_64'}.dmg`);
  mkdirSync(dirname(dmg), { recursive: true });
  run('node', ['script/fix-macos-release-signing.mjs', destination], { ...environment, SNAPLINGO_BUNDLE_CONFIG: manifest, SNAPLINGO_DMG_OUTPUT: dmg });
  const verification = verifyMacOSApplication(destination, config, { allowAdhoc: process.env.SNAPLINGO_SIGNING_MODE === 'adhoc' });
  run('/usr/bin/hdiutil', ['verify', dmg]);
  const checksum = await sha256(dmg);
  writeFileSync(`${dmg}.SHA256SUMS.txt`, `${checksum}  ${dmg.split('/').pop()}\n`);
  writeFileSync(join(targetDirectory, 'native', profile, 'verification.json'), `${JSON.stringify({ version: config.version, platform: process.platform, architecture: process.arch, ...verification, artifacts: [{ path: dmg, size: statSync(dmg).size, sha256: checksum }] }, null, 2)}\n`);
  console.log(`[native-build] Verified ${destination}`);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch(error => { console.error(error.message); process.exitCode = 1; });
}
