import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

function run(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8' });
  if (result.error || result.status !== 0) {
    throw new Error(`${command} failed: ${result.error?.message ?? result.stderr ?? result.stdout}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

export function assertMacOSDeploymentTarget(loadCommands, minimum, label) {
  const versions = [...loadCommands.matchAll(/\bminos\s+(\d+(?:\.\d+)*)/g)].map(m => m[1]);
  for (const command of loadCommands.split(/Load command \d+/)) {
    if (command.includes('LC_VERSION_MIN_MACOSX')) {
      const version = command.match(/\bversion\s+(\d+(?:\.\d+)*)/)?.[1];
      if (version) versions.push(version);
    }
  }
  if (versions.length === 0) throw new Error(`${label}: missing macOS deployment target`);
  const value = version => version.split('.').reduce((sum, part, i) => sum + Number(part) / (1000 ** i), 0);
  for (const version of versions) {
    if (value(version) > value(minimum)) {
      throw new Error(`${label} requires macOS ${version}, but the app declares ${minimum}`);
    }
  }
}

function filesIn(directory) {
  if (!existsSync(directory)) return [];
  return readdirSync(directory).flatMap(name => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? filesIn(path) : [path];
  });
}

export function verifyMacOSApplication(app, config, { allowAdhoc = false } = {}) {
  run('/usr/bin/codesign', ['--verify', '--deep', '--strict', app]);
  const details = run('/usr/bin/codesign', ['-dv', '--verbose=4', app]);
  const requirement = run('/usr/bin/codesign', ['-d', '-r-', app]);
  const plist = JSON.parse(run('/usr/bin/plutil', ['-convert', 'json', '-o', '-', join(app, 'Contents/Info.plist')]));
  for (const [key, value] of Object.entries({
    CFBundleIdentifier: config.identifier,
    CFBundleShortVersionString: config.version,
    CFBundleVersion: config.version,
    LSMinimumSystemVersion: config.bundle.macOS.minimumSystemVersion,
  })) {
    if (plist[key] !== value) throw new Error(`${key}: expected ${value}, got ${plist[key]}`);
  }
  if (!details.includes('runtime)') || !details.includes(`Identifier=${config.identifier}`)) {
    throw new Error('Missing hardened runtime or incorrect signing identifier');
  }
  if (!allowAdhoc && (details.includes('Signature=adhoc') || requirement.includes('cdhash H'))) {
    throw new Error('Distribution requires a stable certificate, not ad-hoc signing');
  }
  const fingerprint = process.env.SNAPLINGO_CERTIFICATE_SHA1?.toUpperCase();
  if (fingerprint && !requirement.toUpperCase().includes(`CERTIFICATE LEAF = H"${fingerprint}"`)) {
    throw new Error('Signing certificate differs from the pinned release fingerprint');
  }
  const executable = join(app, 'Contents/MacOS', plist.CFBundleExecutable);
  const architectures = run('/usr/bin/lipo', ['-archs', executable]).trim().split(/\s+/);
  const hostArch = process.arch === 'arm64' ? 'arm64' : 'x86_64';
  if (!architectures.includes(hostArch)) throw new Error(`Application cannot run on ${hostArch}`);
  for (const file of [...filesIn(join(app, 'Contents/MacOS')), ...filesIn(join(app, 'Contents/Frameworks'))]) {
    if (!run('/usr/bin/file', ['-b', file]).includes('Mach-O')) continue;
    assertMacOSDeploymentTarget(run('/usr/bin/otool', ['-l', file]), plist.LSMinimumSystemVersion, file);
    const libraryArchs = run('/usr/bin/lipo', ['-archs', file]).trim().split(/\s+/);
    if (architectures.some(arch => !libraryArchs.includes(arch))) throw new Error(`Architecture mismatch: ${file}`);
    const dependencies = run('/usr/bin/otool', ['-L', file]).split('\n').slice(1)
      .map(line => line.trim().match(/^(.+?) \(compatibility version/)?.[1]).filter(Boolean);
    for (const dependency of dependencies) {
      if (dependency.startsWith('/System/') || dependency.startsWith('/usr/lib/')) continue;
      let resolved;
      if (dependency.startsWith('@executable_path/')) resolved = resolve(dirname(executable), dependency.slice(17));
      else if (dependency.startsWith('@loader_path/')) resolved = resolve(dirname(file), dependency.slice(13));
      else if (dependency.startsWith('@rpath/')) resolved = join(app, 'Contents/Frameworks', dependency.slice(7));
      if (!resolved || !resolved.startsWith(`${resolve(app)}/`) || !existsSync(resolved)) {
        throw new Error(`Unbundled dependency in ${file}: ${dependency}`);
      }
    }
  }
  return { executable, requirement: requirement.trim(), architectures };
}
