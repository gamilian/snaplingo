import { spawnSync } from 'node:child_process';
import { copyFile, mkdir, readdir, stat } from 'node:fs/promises';
import { join } from 'node:path';

const REQUIRED = ['vcruntime140.dll', 'vcruntime140_1.dll', 'msvcp140.dll', 'msvcp140_1.dll'];

async function isDirectory(path) {
  try { return (await stat(path)).isDirectory(); } catch { return false; }
}

export async function findWindowsCrt() {
  let redistRoot = process.env.VCToolsRedistDir;
  if (!redistRoot) {
    const vswhere = join(process.env['ProgramFiles(x86)'] ?? 'C:/Program Files (x86)', 'Microsoft Visual Studio/Installer/vswhere.exe');
    const result = spawnSync(vswhere, ['-latest', '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath'], { encoding: 'utf8' });
    if (result.error || result.status !== 0 || !result.stdout.trim()) throw new Error('Windows OCR packaging requires Visual Studio C++ Build Tools and its redistributable files.');
    const parent = join(result.stdout.trim(), 'VC/Redist/MSVC');
    const versions = (await readdir(parent)).sort((a,b) => b.localeCompare(a, undefined, { numeric: true }));
    for (const version of versions) {
      if (await isDirectory(join(parent,version,'x64'))) { redistRoot = join(parent,version); break; }
    }
  }
  if (!redistRoot) throw new Error('Visual C++ x64 redistributable directory was not found.');
  const x64 = join(redistRoot,'x64');
  const folder = (await readdir(x64)).find(name => /^Microsoft\.VC\d+\.CRT$/.test(name));
  if (!folder) throw new Error('Visual C++ x64 CRT files were not found.');
  return join(x64,folder);
}

export async function copyWindowsCrt(source, destination) {
  const names = await readdir(source);
  for (const name of REQUIRED) {
    if (!names.some(file => file.toLowerCase() === name)) throw new Error(`Visual C++ redistributable is missing ${name}.`);
  }
  await mkdir(destination, { recursive: true });
  for (const file of names.filter(file => file.toLowerCase().endsWith('.dll'))) {
    await copyFile(join(source,file), join(destination,file));
  }
}
