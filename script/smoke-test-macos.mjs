import { spawn, spawnSync } from 'node:child_process';
import { cpSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { setTimeout } from 'node:timers/promises';

if (process.platform !== 'darwin' || process.env.GITHUB_ACTIONS !== 'true') {
  throw new Error('Run this installation smoke test on an ephemeral macOS GitHub runner');
}
function run(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8' });
  if (result.error || result.status !== 0) throw new Error(`${command} failed: ${result.stderr}`);
  return result.stdout;
}
const metadata = JSON.parse(run('cargo', ['metadata', '--no-deps', '--format-version', '1']));
const dmgDir = join(metadata.target_directory, 'release/bundle/dmg');
const images = readdirSync(dmgDir).filter(name => name.endsWith('.dmg') && !name.startsWith('rw.'));
if (images.length !== 1) throw new Error('Expected exactly one DMG');
const output = run('/usr/bin/hdiutil', ['attach', join(dmgDir, images[0]), '-nobrowse', '-readonly']);
const mount = output.split('\n').map(line => line.split(/\t+/).at(-1)?.trim()).find(part => part?.startsWith('/Volumes/'));
if (!mount) throw new Error('DMG mount point missing');
const directory = mkdtempSync(join(tmpdir(), 'snaplingo-installed-'));
let processHandle;
try {
  const installed = join(directory, 'SnapLingo.app');
  for (let attempt = 0; attempt < 2; attempt++) {
    rmSync(installed, { recursive: true, force: true });
    cpSync(join(mount, 'SnapLingo.app'), installed, { recursive: true, verbatimSymlinks: true });
    run('/usr/bin/codesign', ['--verify', '--deep', '--strict', installed]);
    const executable = run('/usr/bin/plutil', ['-extract', 'CFBundleExecutable', 'raw', '-o', '-', join(installed, 'Contents/Info.plist')]).trim();
    processHandle = spawn(join(installed, 'Contents/MacOS', executable), [], { stdio: 'inherit' });
    let launchError;
    processHandle.on('error', error => { launchError = error; });
    await setTimeout(8000);
    if (launchError) throw launchError;
    if (processHandle.exitCode !== null || processHandle.signalCode !== null) {
      throw new Error(`Installed app exited: ${processHandle.exitCode ?? processHandle.signalCode}`);
    }
    const exited = new Promise(resolve => processHandle.once('exit', resolve));
    processHandle.kill('SIGTERM');
    await exited;
    processHandle = undefined;
  }
} finally {
  processHandle?.kill('SIGKILL');
  run('/usr/bin/hdiutil', ['detach', mount]);
  rmSync(directory, { recursive: true, force: true });
}
