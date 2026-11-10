import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { prepareOcr } from './prepare-ocr.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const requiredResources = ['medium_det.onnx', 'medium_rec.onnx', 'keys.txt', 'runtime/onnxruntime.dll', 'runtime/DirectML.dll'];
const requiredCrt = ['vcruntime140.dll', 'vcruntime140_1.dll', 'msvcp140.dll', 'msvcp140_1.dll'];
const xml = value => String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
const nsis = value => String(value).replaceAll('$', () => '$$').replaceAll('"', '$\\"').replaceAll('\r', '').replaceAll('\n', '$\\n');
const nsisFile = value => String(value).replaceAll('\"', '$\\\"').replaceAll('\r', '').replaceAll('\n', '$\\n');
const windowsPath = value => String(value).replaceAll('/', '\\');
const hash = value => createHash('sha256').update(value).digest('hex');
function required(path) {
  if (!statSync(path, { throwIfNoEntry: false })?.isFile() || !statSync(path).size) throw new Error(`Missing native Windows resource: ${path}`);
}
function files(directory) {
  return readdirSync(directory, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name)).flatMap(entry => entry.isDirectory() ? files(join(directory, entry.name)) : [join(directory, entry.name)]);
}
export function verifyWindowsExecutable(path) {
  required(path);
  const bytes = readFileSync(path);
  if (bytes.length < 64 || bytes.subarray(0, 2).toString() !== 'MZ') throw new Error('Native Windows executable is not a PE binary');
  const offset = bytes.readUInt32LE(60);
  if (offset + 6 > bytes.length || bytes.subarray(offset, offset + 4).toString() !== 'PE\0\0' || bytes.readUInt16LE(offset + 4) !== 0x8664) throw new Error('Native Windows executable must target x64');
}
export function verifyNativeWindowsBundle(directory) {
  verifyWindowsExecutable(join(directory, 'snaplingo.exe'));
  for (const path of [...requiredCrt, ...requiredResources.map(path => `ocr/${path}`),
    'LICENSE', 'licenses/NotoSans.txt', 'licenses/THIRD_PARTY_NOTICES.json', 'icon.ico']) {
    required(join(directory, path));
  }
}
export function createNativeWindowsPortable(stage, archive) {
  verifyNativeWindowsBundle(stage);
  mkdirSync(dirname(archive), { recursive: true });
  rmSync(archive, { force: true });
  const quote = value => `'${value.replaceAll("'", "''")}'`;
  run('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `$ErrorActionPreference = 'Stop'; Add-Type -AssemblyName System.IO.Compression.FileSystem; [IO.Compression.ZipFile]::CreateFromDirectory(${quote(stage)}, ${quote(archive)}, [IO.Compression.CompressionLevel]::Optimal, $true)`], root);
  required(archive);
  return archive;
}
export function stageNativeWindowsBundle(repositoryRoot, binary, destination, config, options = {}) {
  const resources = options.ocrRoot ?? join(repositoryRoot, 'app/resources/ocr');
  const crt = options.crtRoot ?? join(repositoryRoot, 'app/resources/windows-crt');
  verifyWindowsExecutable(binary);
  for (const path of requiredResources) required(join(resources, path));
  for (const path of requiredCrt) required(join(crt, path));
  if (readFileSync(join(resources, '.platform'), 'utf8').trim() !== 'win32-x64') throw new Error('OCR resources do not match the native Windows x64 architecture');
  for (const path of ['LICENSE', 'assets/fonts/LICENSE-NotoSans.txt', 'assets/licenses/thirdPartyNotices.json', 'app/icons/icon.ico']) required(join(repositoryRoot, path));
  rmSync(destination, { recursive: true, force: true });
  mkdirSync(join(destination, 'licenses'), { recursive: true });
  copyFileSync(binary, join(destination, 'snaplingo.exe'));
  cpSync(resources, join(destination, 'ocr'), { recursive: true, filter: path => basename(path) !== '.gitkeep' });
  for (const file of readdirSync(crt).filter(file => file.toLowerCase().endsWith('.dll'))) copyFileSync(join(crt, file), join(destination, file));
  for (const [source, target] of [['LICENSE', 'LICENSE'], ['assets/fonts/LICENSE-NotoSans.txt', 'licenses/NotoSans.txt'], ['assets/licenses/thirdPartyNotices.json', 'licenses/THIRD_PARTY_NOTICES.json'], ['app/icons/icon.ico', 'icon.ico']]) copyFileSync(join(repositoryRoot, source), join(destination, target));
  writeFileSync(join(destination, 'README.txt'), `${config.productName} ${config.version}\r\n\r\nLaunch snaplingo.exe to start the native application.\r\nKeep the OCR directory and DLL files beside the executable.\r\nSettings and saved history remain in the Windows application-data directory during upgrades and uninstallation.\r\n`);
  return destination;
}
function version(config) {
  if (!/^\d+\.\d+\.\d+$/.test(config.version)) throw new Error('Native Windows installer version must use three numeric parts');
  const parts = config.version.split('.').map(Number);
  if (parts[0] > 255 || parts[1] > 255 || parts[2] > 65535) throw new Error('Native Windows installer version exceeds MSI limits');
  if (!config.productName || /[\\/:*?"<>|\r\n]/.test(config.productName)) throw new Error('Invalid Windows product name');
  if (!/^[a-zA-Z0-9._-]+$/.test(config.identifier)) throw new Error('Invalid Windows application identifier');
  return config.version;
}
export function nativeWindowsUpgradeCode(config) {
  const configured = config.bundle?.windows?.wix?.upgradeCode;
  if (configured) {
    if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(configured)) throw new Error('Invalid Windows MSI UpgradeCode');
    return configured.toUpperCase();
  }
  // Tauri 2's default is UUIDv5(DNS, `${productName}.exe.app.x64`).
  const namespace = Buffer.from('6ba7b8109dad11d180b400c04fd430c8', 'hex');
  const bytes = createHash('sha1').update(namespace).update(`${config.productName}.exe.app.x64`).digest().subarray(0, 16);
  bytes[6] = (bytes[6] & 15) | 80; bytes[8] = (bytes[8] & 63) | 128;
  const value = bytes.toString('hex');
  return `${value.slice(0,8)}-${value.slice(8,12)}-${value.slice(12,16)}-${value.slice(16,20)}-${value.slice(20)}`.toUpperCase();
}
function publisher(config) { return config.publisher ?? config.identifier.split('.')[1] ?? config.identifier; }
export function nativeNsisSource(stage, output, config) {
  version(config);
  const inventory = files(stage).map(file => ({ file, path: windowsPath(relative(stage, file)) }));
  const directories = [...new Set(inventory.map(file => dirname(file.path.replaceAll('\\', '/'))).filter(path => path !== '.'))].sort((a, b) => b.length - a.length);
  return `Unicode true
!include "MUI2.nsh"
!include "WordFunc.nsh"
!include "LogicLib.nsh"
Name "${nsis(config.productName)}"
Icon "${nsisFile(windowsPath(join(stage, 'icon.ico')))}"
UninstallIcon "${nsisFile(windowsPath(join(stage, 'icon.ico')))}"
OutFile "${nsisFile(windowsPath(output))}"
InstallDir "$LOCALAPPDATA\\${nsis(config.productName)}"
InstallDirRegKey HKCU "Software\\${nsis(publisher(config))}\\${nsis(config.productName)}" ""
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "${config.version}.0"
VIAddVersionKey "ProductName" "${nsis(config.productName)}"
VIAddVersionKey "FileVersion" "${config.version}"
VIAddVersionKey "LegalCopyright" "SnapLingo contributors"
VIAddVersionKey "FileDescription" "${nsis(config.productName)} native installer"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"
Function .onInit
  ReadRegStr $0 HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}" "DisplayVersion"
  \${VersionCompare} $0 "${config.version}" $1
  \${If} $1 == 1
    MessageBox MB_ICONSTOP "A newer version of ${nsis(config.productName)} is already installed." /SD IDOK
    Abort
  \${EndIf}
FunctionEnd
Section "Install"
  SetShellVarContext current
${inventory.map(({ file, path }) => `  SetOutPath "$INSTDIR${dirname(path.replaceAll('\\','/')) === '.' ? '' : `\\${nsis(windowsPath(dirname(path.replaceAll('\\','/'))))}`}"\n  File "${nsisFile(windowsPath(file))}"`).join('\n')}
  WriteUninstaller "$INSTDIR\\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\\${nsis(config.productName)}"
  CreateShortcut "$SMPROGRAMS\\${nsis(config.productName)}\\${nsis(config.productName)}.lnk" "$INSTDIR\\snaplingo.exe"
  WriteRegStr HKCU "Software\\${nsis(publisher(config))}\\${nsis(config.productName)}" "" "$INSTDIR"
  WriteRegStr HKCU "Software\\${nsis(publisher(config))}\\${nsis(config.productName)}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}" "DisplayName" "${nsis(config.productName)}"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}" "DisplayVersion" "${config.version}"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}" "Publisher" "${nsis(publisher(config))}"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}" "MainBinaryName" "snaplingo.exe"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}" "DisplayIcon" "$INSTDIR\\snaplingo.exe"
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}" "UninstallString" '$\\"$INSTDIR\\uninstall.exe$\\"'
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}" "QuietUninstallString" '$\\"$INSTDIR\\uninstall.exe$\\" /S'
  WriteRegStr HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}" "InstallLocation" "$INSTDIR"
SectionEnd
Section "Uninstall"
  SetShellVarContext current
${inventory.map(({ path }) => `  Delete "$INSTDIR\\${nsis(path)}"`).join('\n')}
  Delete "$INSTDIR\\uninstall.exe"
${directories.map(path => `  RMDir "$INSTDIR\\${nsis(windowsPath(path))}"`).join('\n')}
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\\${nsis(config.productName)}\\${nsis(config.productName)}.lnk"
  RMDir "$SMPROGRAMS\\${nsis(config.productName)}"
  DeleteRegKey HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${nsis(config.productName)}"
  DeleteRegKey HKCU "Software\\${nsis(publisher(config))}\\${nsis(config.productName)}"
SectionEnd
`;
}
export function nativeWixSource(stage, config) {
  version(config);
  const inventory = files(stage).map(file => ({ file, path: relative(stage, file).replaceAll('\\', '/') }));
  const directories = new Map([['', []]]);
  for (const entry of inventory) {
    let directory = dirname(entry.path); if (directory === '.') directory = '';
    if (!directories.has(directory)) directories.set(directory, []);
    directories.get(directory).push(entry);
    let parent = directory;
    while (parent.includes('/')) { parent = dirname(parent); if (!directories.has(parent)) directories.set(parent, []); }
  }
  const directoryId = path => path ? `D${hash(path).slice(0,20)}` : 'INSTALLFOLDER';
  const componentId = path => `C${hash(path).slice(0,20)}`;
  const fileId = path => `F${hash(path).slice(0,20)}`;
  // Advertised shortcuts need a PE icon stream whose Icon table name matches
  // the key file's extension (ICE50). Reuse the application's embedded icon.
  const iconId = 'AppIcon.exe';
  const component = entry => {
    const application = entry.path === 'snaplingo.exe';
    const shortcut = application ? `<Shortcut Id="ApplicationShortcut" Name="${xml(config.productName)}" Directory="ApplicationProgramsFolder" Advertise="yes" WorkingDirectory="INSTALLFOLDER" Icon="${iconId}"/>` : '';
    const registration = application ? `<RemoveFolder Id="RemoveProgramMenu" Directory="ApplicationProgramsFolder" On="uninstall"/><RegistryValue Root="HKLM" Key="Software\\${xml(publisher(config))}\\${xml(config.productName)}" Name="InstallDir" Type="string" Value="[INSTALLFOLDER]"/>` : '';
    return `<Component Id="${componentId(entry.path)}" Guid="*" Win64="yes"><File Id="${fileId(entry.path)}" Name="${xml(basename(entry.path))}" Source="${xml(windowsPath(entry.file).replaceAll('$', () => '$$'))}" KeyPath="yes">${shortcut}</File>${registration}</Component>`;
  };
  const directory = path => `<Directory Id="${directoryId(path)}" Name="${xml(path ? basename(path) : config.productName)}">${directories.get(path).map(component).join('')}${[...directories.keys()].filter(child => child && (dirname(child) === '.' ? '' : dirname(child)) === path).sort().map(directory).join('')}</Directory>`;
  return `<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi"><Product Id="*" Name="${xml(config.productName)}" Language="1033" Version="${config.version}" Manufacturer="${xml(publisher(config))}" UpgradeCode="${nativeWindowsUpgradeCode(config)}">
<Package InstallerVersion="500" Compressed="yes" InstallScope="perMachine" Platform="x64"/>
<MajorUpgrade Schedule="afterInstallInitialize" AllowSameVersionUpgrades="yes" DowngradeErrorMessage="A newer version of ${xml(config.productName)} is already installed."/>
<MediaTemplate EmbedCab="yes" CompressionLevel="high"/>
<Property Id="INSTALLFOLDER"><RegistrySearch Id="PreviousTauriInstallDir" Root="HKCU" Key="Software\\${xml(publisher(config))}\\${xml(config.productName)}" Name="InstallDir" Type="raw"/></Property>
<Icon Id="${iconId}" SourceFile="${xml(windowsPath(join(stage,'snaplingo.exe')).replaceAll('$', () => '$$'))}"/><Property Id="ARPPRODUCTICON" Value="${iconId}"/>
<SetProperty Id="ARPINSTALLLOCATION" Value="[INSTALLFOLDER]" After="CostFinalize" Sequence="execute"/>
<Directory Id="TARGETDIR" Name="SourceDir"><Directory Id="ProgramFiles64Folder">${directory('')}</Directory>
<Directory Id="ProgramMenuFolder"><Directory Id="ApplicationProgramsFolder" Name="${xml(config.productName)}"/></Directory></Directory>
<Feature Id="MainFeature" Title="${xml(config.productName)}" Level="1">${inventory.map(entry=>`<ComponentRef Id="${componentId(entry.path)}"/>`).join('')}</Feature>
</Product></Wix>\n`;
}
function run(command, args, repositoryRoot) {
  const result = spawnSync(command, args, { cwd: repositoryRoot, stdio: 'inherit' });
  if (result.error || result.status !== 0) throw new Error(`${command} failed: ${result.error?.message ?? result.status}`);
}
function tool(name, environment, candidates) {
  if (process.env[environment]) return process.env[environment];
  return candidates.find(path => existsSync(path)) ?? name;
}
export async function buildNativeWindows({ repositoryRoot = root, command = 'build', profile = 'release', config, targetDirectory = join(repositoryRoot, 'target'), formats = ['nsis', 'msi', 'portable'] }) {
  if (process.platform !== 'win32' || process.arch !== 'x64') throw new Error('Native Windows packaging requires an x64 Windows build host');
  const compiler = spawnSync('rustc', ['--version', '--verbose'], { encoding: 'utf8' });
  if (compiler.error || compiler.status !== 0 || !/^host: x86_64-pc-windows-msvc$/m.test(compiler.stdout ?? '')) throw new Error('Native Windows release packages require the x64 MSVC Rust toolchain');
  version(config);
  if (!['build','bundle'].includes(command) || !['debug','release'].includes(profile) || formats.some(format => !['nsis','msi','portable'].includes(format))) throw new Error('Invalid native Windows build options');
  const resources = join(targetDirectory, 'native/resources/win32-x64/ocr');
  if (command === 'build') {
    await prepareOcr('win32-x64', resources);
    run('node', ['script/third-party-notices.mjs', '--check'], repositoryRoot);
    run('cargo', ['build', '--locked', '-p', 'snaplingo', '--no-default-features', '--features', 'native-app', '--bin', 'snaplingo-native', ...(profile === 'release' ? ['--release'] : [])], repositoryRoot);
  }
  const output = join(targetDirectory, 'native', profile);
  const stage = join(output, 'app');
  stageNativeWindowsBundle(repositoryRoot, join(targetDirectory, profile, 'snaplingo-native.exe'), stage, config, { ocrRoot: resources, crtRoot: join(dirname(resources), 'windows-crt') });
  const artifactBase = `${config.productName}_${config.version}_x64`;
  const generated = join(output, 'installer'); mkdirSync(generated, { recursive: true });
  const nsisOutput = join(output, 'bundle/nsis', `${artifactBase}-setup.exe`);
  const wixOutput = join(output, 'bundle/msi', `${artifactBase}.msi`);
  writeFileSync(join(generated, 'installer.nsi'), nativeNsisSource(stage, nsisOutput, config));
  writeFileSync(join(generated, 'installer.wxs'), nativeWixSource(stage, config));
  const artifacts = [];
  if (formats.includes('nsis')) {
    mkdirSync(dirname(nsisOutput), { recursive: true });
    const makensis = tool('makensis.exe', 'SNAPLINGO_MAKENSIS', [join(process.env['ProgramFiles(x86)'] ?? 'C:/Program Files (x86)', 'NSIS/makensis.exe')]);
    run(makensis, ['/V2', join(generated, 'installer.nsi')], repositoryRoot); required(nsisOutput); artifacts.push(nsisOutput);
  }
  if (formats.includes('msi')) {
    mkdirSync(dirname(wixOutput), { recursive: true });
    const wixBin = process.env.SNAPLINGO_WIX_BIN ?? (process.env.WIX ? join(process.env.WIX, 'bin') : undefined);
    const object = join(generated, 'installer.wixobj');
    run(wixBin ? join(wixBin, 'candle.exe') : 'candle.exe', ['-nologo', '-arch', 'x64', '-out', object, join(generated, 'installer.wxs')], repositoryRoot);
    run(wixBin ? join(wixBin, 'light.exe') : 'light.exe', ['-nologo', '-out', wixOutput, object], repositoryRoot); required(wixOutput); artifacts.push(wixOutput);
  }
  if (formats.includes('portable')) {
    artifacts.push(createNativeWindowsPortable(stage, join(output, 'bundle/portable', `${artifactBase}-portable.zip`)));
  }
  const verification = { version: config.version, platform: 'win32', architecture: 'x64', signing: 'unsigned', files: files(stage).map(file => ({ path: relative(stage, file), sha256: hash(readFileSync(file)) })), artifacts: artifacts.map(file => ({ path: relative(output, file), sha256: hash(readFileSync(file)) })) };
  writeFileSync(join(output, 'verification.json'), `${JSON.stringify(verification, null, 2)}\n`);
  console.log(`[native-build] Staged ${stage}; created ${artifacts.length} Windows artifacts`);
  return artifacts;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2); const command = args[0] ?? 'build'; const formatsIndex = args.indexOf('--formats');
  const config = { ...JSON.parse(readFileSync(join(root,'app/desktop.json'),'utf8')), version: JSON.parse(readFileSync(join(root,'package.json'),'utf8')).version };
  buildNativeWindows({ command, config, profile: args.includes('--debug') ? 'debug' : 'release', targetDirectory: process.env.CARGO_TARGET_DIR ? resolve(root, process.env.CARGO_TARGET_DIR) : join(root,'target'), ...(formatsIndex >= 0 ? { formats: args[formatsIndex+1]?.split(',') ?? [] } : {}) }).catch(error => { console.error(error.message); process.exitCode = 1; });
}
