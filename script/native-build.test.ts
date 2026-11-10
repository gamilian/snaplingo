import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { afterEach, describe, expect, it } from 'vitest';
import { nativeInfoPlist, stageNativeMacBundle } from './native-build.mjs';

const directories: string[] = [];
afterEach(() => { for (const directory of directories.splice(0)) rmSync(directory, { recursive: true, force: true }); });
const config = { identifier: 'com.snaplingo.app', productName: 'SnapLingo', version: '0.1.23', bundle: { macOS: { minimumSystemVersion: '12.0' } } };
const assets = JSON.parse(readFileSync(new URL('./ocr-assets.json', import.meta.url), 'utf8'));
function fixture(architecture = process.arch) {
  const root = mkdtempSync(join(tmpdir(), 'snaplingo-native-bundle-'));
  directories.push(root);
  const platform = `darwin-${architecture}`;
  const ocrFiles = [
    ...assets.models.filter(asset => asset.platforms.includes(platform)).map(asset => asset.file),
    ...assets.archives.filter(asset => asset.platform === platform || asset.platforms?.includes(platform)).flatMap(asset => Object.values(asset.files)),
  ];
  const files = [
    'target/debug/snaplingo-native', 'app/icons/icon.icns',
    ...ocrFiles.map(file => `app/resources/ocr/${file}`),
    'LICENSE', 'assets/fonts/LICENSE-NotoSans.txt', 'assets/licenses/thirdPartyNotices.json',
  ];
  for (const file of files) { mkdirSync(dirname(join(root, file)), { recursive: true }); writeFileSync(join(root, file), file); }
  writeFileSync(join(root, 'app/resources/ocr/.platform'), platform);
  return { root, binary: join(root, 'target/debug/snaplingo-native'), app: join(root, 'target/native/debug/SnapLingo.app') };
}
describe('native macOS bundle', () => {
  it('packages the native executable, OCR, font notices, and stable application identity without frontend resources', () => {
    const f = fixture('arm64');
    stageNativeMacBundle(f.root, f.binary, f.app, config, { architecture: 'arm64' });
    const contents = join(f.app, 'Contents');
    expect(readFileSync(join(contents, 'MacOS/snaplingo-native'), 'utf8')).toBe('target/debug/snaplingo-native');
    expect(readFileSync(join(contents, 'Info.plist'), 'utf8')).toContain('<key>CFBundleIdentifier</key><string>com.snaplingo.app</string>');
    for (const file of ['ocr/medium_det.mnn', 'ocr/medium_rec.mnn', 'ocr/medium_det.onnx', 'ocr/medium_rec.onnx', 'ocr/runtime/libonnxruntime.dylib', 'licenses/THIRD_PARTY_NOTICES.json', 'licenses/NotoSans.txt']) expect(existsSync(join(contents, 'Resources', file))).toBe(true);
    expect(existsSync(join(contents, 'Resources/index.html'))).toBe(false);
  });
  it('packages the Intel MNN resources prepared by the asset manifest without an ARM-only ONNX runtime', () => {
    const f = fixture('x64');
    stageNativeMacBundle(f.root, f.binary, f.app, config, { architecture: 'x64' });
    const resources = join(f.app, 'Contents/Resources/ocr');
    expect(readFileSync(join(resources, 'medium_rec.mnn'), 'utf8')).toContain('medium_rec.mnn');
    expect(existsSync(join(resources, 'licenses/mnn/LICENSE-MNN.txt'))).toBe(true);
    expect(existsSync(join(resources, 'runtime/libonnxruntime.dylib'))).toBe(false);
    expect(existsSync(join(resources, 'medium_det.onnx'))).toBe(false);
  });
  it('requires the ARM ONNX models, runtime and licenses before replacing an existing bundle', () => {
    for (const file of ['medium_det.onnx', 'medium_rec.onnx', 'runtime/libonnxruntime.dylib', 'licenses/onnxruntime.txt']) {
      const f = fixture('arm64');
      mkdirSync(f.app, { recursive: true }); writeFileSync(join(f.app, 'previous'), 'keep');
      rmSync(join(f.root, 'app/resources/ocr', file));
      expect(() => stageNativeMacBundle(f.root, f.binary, f.app, config, { architecture: 'arm64' })).toThrow(file);
      expect(readFileSync(join(f.app, 'previous'), 'utf8')).toBe('keep');
    }
  });
  it('rejects missing models and wrong-architecture OCR before replacing a previous bundle', () => {
    const f = fixture();
    mkdirSync(f.app, { recursive: true }); writeFileSync(join(f.app, 'previous'), 'keep');
    writeFileSync(join(f.root, 'app/resources/ocr/.platform'), 'windows-x64');
    expect(() => stageNativeMacBundle(f.root, f.binary, f.app, config)).toThrow('architecture');
    rmSync(join(f.root, 'app/resources/ocr/medium_det.mnn'));
    expect(() => stageNativeMacBundle(f.root, f.binary, f.app, config)).toThrow('Missing native bundle resource');
    expect(readFileSync(join(f.app, 'previous'), 'utf8')).toBe('keep');
  });
  it('preserves permission explanations and XML-escapes product metadata', () => {
    const plist = nativeInfoPlist({ ...config, productName: 'Snap & <Lingo>' });
    expect(plist).toContain('Snap &amp; &lt;Lingo&gt;');
    for (const key of ['LSUIElement', 'NSHighResolutionCapable', 'NSScreenCaptureUsageDescription', 'NSAppleEventsUsageDescription']) expect(plist).toContain(`<key>${key}</key>`);
    expect(plist).toContain('<key>LSMinimumSystemVersion</key><string>12.0</string>');
  });
});
