import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { afterEach, describe, expect, it } from 'vitest';
import { nativeInfoPlist, stageNativeMacBundle } from './native-build.mjs';

const directories: string[] = [];
afterEach(() => { for (const directory of directories.splice(0)) rmSync(directory, { recursive: true, force: true }); });
const config = { identifier: 'com.snaplingo.app', productName: 'SnapLingo', version: '0.1.23', bundle: { macOS: { minimumSystemVersion: '12.0' } } };
function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'snaplingo-native-bundle-'));
  directories.push(root);
  const files = [
    'target/debug/snaplingo-native', 'crates/snaplingo-app/icons/icon.icns',
    'crates/snaplingo-app/resources/ocr/medium_det.mnn', 'crates/snaplingo-app/resources/ocr/medium_rec.mnn',
    'crates/snaplingo-app/resources/ocr/keys.txt', 'crates/snaplingo-app/resources/ocr/runtime/libonnxruntime.dylib',
    'LICENSE', 'assets/fonts/LICENSE-NotoSans.txt', 'src/domain/thirdPartyNotices.json',
  ];
  for (const file of files) { mkdirSync(dirname(join(root, file)), { recursive: true }); writeFileSync(join(root, file), file); }
  writeFileSync(join(root, 'crates/snaplingo-app/resources/ocr/.platform'), `darwin-${process.arch}`);
  return { root, binary: join(root, 'target/debug/snaplingo-native'), app: join(root, 'target/native/debug/SnapLingo.app') };
}
describe('native macOS bundle', () => {
  it('packages the native executable, OCR, font notices, and stable application identity without frontend resources', () => {
    const f = fixture();
    stageNativeMacBundle(f.root, f.binary, f.app, config);
    const contents = join(f.app, 'Contents');
    expect(readFileSync(join(contents, 'MacOS/snaplingo-native'), 'utf8')).toBe('target/debug/snaplingo-native');
    expect(readFileSync(join(contents, 'Info.plist'), 'utf8')).toContain('<key>CFBundleIdentifier</key><string>com.snaplingo.app</string>');
    for (const file of ['ocr/medium_det.mnn', 'ocr/medium_rec.mnn', 'ocr/runtime/libonnxruntime.dylib', 'licenses/THIRD_PARTY_NOTICES.json', 'licenses/NotoSans.txt']) expect(existsSync(join(contents, 'Resources', file))).toBe(true);
    expect(existsSync(join(contents, 'Resources/index.html'))).toBe(false);
  });
  it('rejects missing models and wrong-architecture OCR before replacing a previous bundle', () => {
    const f = fixture();
    mkdirSync(f.app, { recursive: true }); writeFileSync(join(f.app, 'previous'), 'keep');
    writeFileSync(join(f.root, 'crates/snaplingo-app/resources/ocr/.platform'), 'windows-x64');
    expect(() => stageNativeMacBundle(f.root, f.binary, f.app, config)).toThrow('architecture');
    rmSync(join(f.root, 'crates/snaplingo-app/resources/ocr/medium_det.mnn'));
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
