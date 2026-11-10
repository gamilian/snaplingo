import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { afterEach, describe, expect, it } from 'vitest';
import { Window } from 'happy-dom';
import { createNativeWindowsPortable, verifyNativeWindowsBundle, nativeNsisSource, nativeWindowsUpgradeCode, nativeWixSource, stageNativeWindowsBundle, verifyWindowsExecutable } from './native-windows-build.mjs';
const directories: string[] = [];
afterEach(() => { for (const path of directories.splice(0)) rmSync(path, { recursive: true, force: true }); });
const config = { identifier: 'com.snaplingo.app', productName: 'SnapLingo', version: '0.1.23' };
function fixture() {
  const root = mkdtempSync(join(tmpdir(), "snaplingo-native-win-$'")); directories.push(root);
  const ocrRoot = join(root, 'target/native/resources/win32-x64/ocr');
  const crtRoot = join(dirname(ocrRoot), 'windows-crt');
  const resources = ['medium_det.onnx', 'medium_rec.onnx', 'keys.txt', 'runtime/onnxruntime.dll', 'runtime/DirectML.dll'];
  for (const path of resources) { mkdirSync(dirname(join(ocrRoot,path)), { recursive: true }); writeFileSync(join(ocrRoot,path), path); }
  writeFileSync(join(ocrRoot,'.platform'), 'win32-x64'); mkdirSync(crtRoot, { recursive: true });
  for (const path of ['vcruntime140.dll','vcruntime140_1.dll','msvcp140.dll','msvcp140_1.dll']) writeFileSync(join(crtRoot,path), path);
  for (const path of ['LICENSE','assets/fonts/LICENSE-NotoSans.txt','assets/licenses/thirdPartyNotices.json','app/icons/icon.ico']) { mkdirSync(dirname(join(root,path)), { recursive: true }); writeFileSync(join(root,path), path); }
  const binary = join(root,'target/release/snaplingo-native.exe'); mkdirSync(dirname(binary), { recursive: true });
  const pe = Buffer.alloc(128); pe.write('MZ'); pe.writeUInt32LE(64,60); pe.write('PE\0\0',64); pe.writeUInt16LE(0x8664,68); writeFileSync(binary, pe);
  return { root, binary, ocrRoot, crtRoot, stage: join(root,'target/native/release/app') };
}
describe('native Windows packages', () => {
  it('stages the native PE and complete offline OCR/CRT tree using isolated resources', () => {
    const f = fixture(); stageNativeWindowsBundle(f.root,f.binary,f.stage,config,f);
    expect(readFileSync(join(f.stage,'snaplingo.exe'))).toEqual(readFileSync(f.binary));
    for (const path of ['ocr/medium_det.onnx','ocr/medium_rec.onnx','ocr/keys.txt','ocr/runtime/onnxruntime.dll','ocr/runtime/DirectML.dll','vcruntime140.dll','msvcp140_1.dll','licenses/THIRD_PARTY_NOTICES.json','licenses/NotoSans.txt','icon.ico']) expect(existsSync(join(f.stage,path))).toBe(true);
    expect(existsSync(join(f.stage,'index.html'))).toBe(false);
    expect(existsSync(join(f.root,'app/resources/ocr'))).toBe(false);
  });
  it('rejects wrong architecture or missing resources before replacing a previous stage', () => {
    const f = fixture(); mkdirSync(f.stage, { recursive: true }); writeFileSync(join(f.stage,'previous'), 'keep');
    const pe = readFileSync(f.binary); pe.writeUInt16LE(0xaa64,68); writeFileSync(f.binary,pe);
    expect(() => stageNativeWindowsBundle(f.root,f.binary,f.stage,config,f)).toThrow('target x64');
    pe.writeUInt16LE(0x8664,68); writeFileSync(f.binary,pe); writeFileSync(join(f.ocrRoot,'.platform'),'darwin-arm64');
    expect(() => stageNativeWindowsBundle(f.root,f.binary,f.stage,config,f)).toThrow('architecture');
    writeFileSync(join(f.ocrRoot,'.platform'),'win32-x64'); rmSync(join(f.crtRoot,'vcruntime140.dll'));
    expect(() => stageNativeWindowsBundle(f.root,f.binary,f.stage,config,f)).toThrow('Missing native Windows resource');
    expect(readFileSync(join(f.stage,'previous'),'utf8')).toBe('keep');
  });
  it('validates PE headers instead of accepting an MZ-prefixed file', () => {
    const f = fixture(); writeFileSync(f.binary,'MZ invalid executable');
    expect(() => verifyWindowsExecutable(f.binary)).toThrow('PE binary');
  });
  it('generates NSIS upgrades and uninstalls that only remove packaged files', () => {
    const f = fixture(); stageNativeWindowsBundle(f.root,f.binary,f.stage,config,f);
    const source = nativeNsisSource(f.stage,join(f.root,'setup.exe'),config);
    expect(source).toContain('RequestExecutionLevel user');
    expect(source).toContain('InstallDirRegKey HKCU "Software\\snaplingo\\SnapLingo" ""');
    expect(source).toContain('Delete "$INSTDIR\\ocr\\runtime\\DirectML.dll"');
    expect(source).toContain('Delete "$INSTDIR\\snaplingo.exe"');
    expect(source).not.toContain('RMDir /r');
    expect(source).not.toContain('Delete "$APPDATA');
    expect(source).toContain('native-win-$');
    expect(source).not.toContain('native-win-$$');
    expect(source).not.toContain('WebView');
    expect(source).toContain('WriteUninstaller "$INSTDIR\\uninstall.exe"');
  });
  it('generates a complete per-machine x64 MSI with stable upgrade identity and valid component references', () => {
    const f = fixture(); stageNativeWindowsBundle(f.root,f.binary,f.stage,config,f);
    const source = nativeWixSource(f.stage,config);
    const upgraded = nativeWixSource(f.stage,{ ...config, version:'0.1.24' });
    expect(source.match(/UpgradeCode="([^"]+)"/)?.[1]).toBe(upgraded.match(/UpgradeCode="([^"]+)"/)?.[1]);
    expect(source).toContain('InstallScope="perMachine" Platform="x64"');
    expect(source).toContain('Name="onnxruntime.dll"');
    const components = [...source.matchAll(/<Component Id="([^"]+)"/g)].map(match => match[1]);
    const references = [...source.matchAll(/<ComponentRef Id="([^"]+)"/g)].map(match => match[1]);
    expect(references.sort()).toEqual(components.sort());
    expect(source).toContain('Root="HKLM"');
    expect(source).toContain('Id="ProgramFiles64Folder"');
    expect(nativeWindowsUpgradeCode(config)).toBe('A6E0F6D1-4F1F-5967-AEB3-62E77A923554');
    expect(source).not.toContain('RemoveLocalPrograms');
    expect(source).toContain('AllowSameVersionUpgrades="yes" DowngradeErrorMessage=');
    expect(source).not.toContain('Id="AppDataFolder"');
    expect(() => nativeWixSource(f.stage,{ ...config,version:'0.1.24-beta' })).toThrow('three numeric');
  });
  it('owns the advertised Start menu shortcut and machine registration with the executable', () => {
    const f = fixture(); stageNativeWindowsBundle(f.root, f.binary, f.stage, config, f);
    const window = new Window();
    const document = new window.DOMParser().parseFromString(nativeWixSource(f.stage, config), 'application/xml');
    const executable = document.querySelector('File[Name="snaplingo.exe"]')!;
    const component = executable.parentElement!;
    const shortcut = document.querySelector('Shortcut[Id="ApplicationShortcut"]')!;
    expect(executable.getAttribute('KeyPath')).toBe('yes');
    expect(shortcut.parentElement).toBe(executable);
    expect(shortcut.getAttribute('Advertise')).toBe('yes');
    expect(shortcut.getAttribute('Directory')).toBe('ApplicationProgramsFolder');
    expect(shortcut.hasAttribute('Target')).toBe(false);
    expect(component.parentElement?.getAttribute('Id')).toBe('INSTALLFOLDER');
    expect(component.querySelector('RegistryValue[Root="HKLM"][Name="InstallDir"]')?.getAttribute('Value')).toBe('[INSTALLFOLDER]');
    expect(component.querySelector('RemoveFolder[On="uninstall"]')?.getAttribute('Directory')).toBe('ApplicationProgramsFolder');
    const installLocation = document.querySelector('SetProperty[Id="ARPINSTALLLOCATION"]')!;
    expect(installLocation.getAttribute('Value')).toBe('[INSTALLFOLDER]');
    expect(installLocation.getAttribute('After')).toBe('CostFinalize');
    expect(installLocation.getAttribute('Sequence')).toBe('execute');
    expect(document.querySelector('Property[Id="ARPINSTALLLOCATION"]')).toBeNull();
  });
});


describe('native Windows portable output', () => {
  it.each(['ocr/medium_rec.onnx', 'ocr/runtime/DirectML.dll', 'msvcp140.dll', 'licenses/NotoSans.txt'])(
    'rejects a missing staged dependency before archive creation: %s', path => {
      const f = fixture(); stageNativeWindowsBundle(f.root, f.binary, f.stage, config, f);
      rmSync(join(f.stage, path));
      expect(() => verifyNativeWindowsBundle(f.stage)).toThrow('Missing native Windows resource');
      expect(() => createNativeWindowsPortable(f.stage, join(f.root, 'portable.zip'))).toThrow('Missing native Windows resource');
    },
  );
  it.skipIf(process.platform !== 'win32')('creates an extractable native ZIP with quoted checkout paths', () => {
    const f = fixture(); stageNativeWindowsBundle(f.root, f.binary, f.stage, config, f);
    const archive = createNativeWindowsPortable(f.stage, join(f.root, 'portable.zip'));
    const extracted = join(f.root, 'extracted');
    const quote = (value: string) => `'${value.replaceAll("'", "''")}'`;
    const result = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
      `Expand-Archive -LiteralPath ${quote(archive)} -DestinationPath ${quote(extracted)}`], { encoding: 'utf8' });
    expect(result.status, result.stderr).toBe(0);
    verifyNativeWindowsBundle(join(extracted, 'app'));
    expect(readFileSync(join(extracted, 'app/snaplingo.exe'))).toEqual(readFileSync(f.binary));
  }, 60_000);
});
