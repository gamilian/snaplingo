import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { prepareOcr } from './prepare-ocr.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export function nativeDebugApplication(targetDirectory, platform) {
  if (platform === 'darwin') return join(targetDirectory, 'debug/snaplingo-native');
  if (platform === 'win32') return join(targetDirectory, 'debug/snaplingo-native.exe');
  throw new Error('Native development supports macOS and Windows');
}
async function main(args) {
  if (args.some(arg => arg !== '--build')) throw new Error('Usage: node script/native-run.mjs [--build]');
  const target = process.env.CARGO_TARGET_DIR ? resolve(root, process.env.CARGO_TARGET_DIR) : join(root, 'target');
  const application = nativeDebugApplication(target, process.platform);
  if (args.includes('--build')) {
    await prepareOcr(`${process.platform}-${process.arch}`);
    const build = spawnSync('cargo', ['build', '--locked', '-p', 'snaplingo', '--bin', 'snaplingo-native'], {
      cwd: root, stdio: 'inherit',
    });
    if (build.error || build.status !== 0) throw new Error(`Native debug build failed: ${build.error?.message ?? build.status}`);
  }
  if (!existsSync(application)) throw new Error('Native debug application is missing; run npm run native:dev first');
  const child = spawnSync(application, [], { cwd: root, stdio: 'inherit' });
  if (child.error) throw child.error;
  process.exitCode = child.status ?? 1;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch(error => { console.error(error.message); process.exitCode = 1; });
}
