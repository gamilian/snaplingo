import { spawnSync } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { designatedRequirement } from './macos-release-verification.mjs';

describe.skipIf(process.platform !== 'darwin')('persistent macOS release identity', () => {
  let directory: string;
  let originalKeychains: string[];
  let environment: NodeJS.ProcessEnv;

  function run(command: string, args: string[], env = environment) {
    const result = spawnSync(command, args, { encoding: 'utf8', env, timeout: 20_000 });
    if (result.error || result.status !== 0) {
      throw new Error(`${command} failed: ${result.stderr}`);
    }
    return `${result.stdout}${result.stderr}`;
  }

  beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'snaplingo-signing-test-'));
    environment = {
      ...process.env,
      GITHUB_ACTIONS: 'true',
      GITHUB_ENV: join(directory, 'github-env'),
      RUNNER_TEMP: directory,
      MACOS_CERTIFICATE_PASSWORD: randomBytes(24).toString('hex'),
      SNAPLINGO_SIGNING_MODE: 'self-signed',
    };
    originalKeychains = run('/usr/bin/security', ['list-keychains', '-d', 'user'])
      .match(/"([^"]+)"/g)?.map(value => value.slice(1, -1)) ?? [];
    const configuration = join(directory, 'openssl.cnf');
    writeFileSync(configuration, '[req]\nprompt=no\ndistinguished_name=dn\nx509_extensions=extensions\n[dn]\nCN=SnapLingo Signing Test\n[extensions]\nbasicConstraints=critical,CA:TRUE\nkeyUsage=critical,digitalSignature,keyCertSign\nextendedKeyUsage=codeSigning\n');
    run('/usr/bin/openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '2',
      '-keyout', join(directory, 'key.pem'), '-out', join(directory, 'cert.pem'), '-config', configuration]);
    run('/usr/bin/openssl', ['pkcs12', '-export', '-inkey', join(directory, 'key.pem'),
      '-in', join(directory, 'cert.pem'), '-out', join(directory, 'identity.p12'),
      '-passout', 'env:MACOS_CERTIFICATE_PASSWORD']);
    environment.MACOS_CERTIFICATE_P12 = readFileSync(join(directory, 'identity.p12')).toString('base64');
    environment.SNAPLINGO_CERTIFICATE_SHA1 = run('/usr/bin/openssl', ['x509', '-in', join(directory, 'cert.pem'),
      '-noout', '-fingerprint', '-sha1']).split('=').at(-1)!.replace(/[^a-f0-9]/gi, '').toUpperCase();
  });

  afterEach(() => {
    const exportsPath = join(directory, 'github-env');
    if (existsSync(exportsPath)) {
      const keychain = readFileSync(exportsPath, 'utf8').match(/^SNAPLINGO_CODESIGN_KEYCHAIN=(.+)$/m)?.[1];
      if (keychain) spawnSync('/usr/bin/security', ['delete-keychain', keychain]);
    }
    spawnSync('/usr/bin/security', ['list-keychains', '-d', 'user', '-s', ...originalKeychains]);
    rmSync(directory, { recursive: true, force: true });
  });

  it('signs different versions with one identity without installing a trusted root', () => {
    run(process.execPath, ['script/import-macos-certificate.mjs']);
    for (const line of readFileSync(environment.GITHUB_ENV!, 'utf8').trim().split('\n')) {
      const separator = line.indexOf('=');
      environment[line.slice(0, separator)] = line.slice(separator + 1);
    }
    expect(environment.SNAPLINGO_CODESIGN_IDENTITY).toBe(environment.SNAPLINGO_CERTIFICATE_SHA1);
    expect(environment.SNAPLINGO_SIGNING_MODE).toBe('self-signed');
    expect(run('/usr/bin/security', ['find-identity', '-v', '-p', 'codesigning', environment.SNAPLINGO_CODESIGN_KEYCHAIN!]))
      .not.toContain(environment.SNAPLINGO_CERTIFICATE_SHA1);

    const requirements: string[] = [];
    const applications: string[] = [];
    for (const version of ['1', '2']) {
      const app = join(directory, version, 'SnapLingo.app');
      const binary = join(app, 'Contents/MacOS/probe');
      mkdirSync(join(app, 'Contents/MacOS'), { recursive: true });
      const plist = join(app, 'Contents/Info.plist');
      writeFileSync(plist, JSON.stringify({ CFBundleIdentifier: 'com.snaplingo.app', CFBundleExecutable: 'probe',
        CFBundlePackageType: 'APPL', CFBundleVersion: version, CFBundleShortVersionString: version }));
      run('/usr/bin/plutil', ['-convert', 'xml1', plist]);
      const source = join(directory, 'probe.c');
      writeFileSync(source, `#include <stdio.h>\nint main(void) { puts("version ${version}"); return 0; }\n`);
      run('/usr/bin/clang', [source, '-o', binary]);
      run(process.execPath, ['script/fix-macos-release-signing.mjs', app]);
      expect(run('/usr/bin/codesign', ['-d', '--entitlements', ':-', app]))
        .not.toContain('com.apple.security.cs.disable-library-validation');
      requirements.push(designatedRequirement(run('/usr/bin/codesign', ['-d', '-r-', app])));
      applications.push(app);
    }
    expect(requirements[0].toUpperCase()).toContain(environment.SNAPLINGO_CERTIFICATE_SHA1);
    expect(requirements[0]).not.toContain('cdhash');
    expect(requirements[1]).toBe(requirements[0]);
    run('/usr/bin/codesign', ['--verify', '--strict', `-R=${requirements[0]}`, applications[1]]);

    run('/usr/bin/codesign', ['--force', '--sign', '-', applications[1]]);
    const adHoc = spawnSync('/usr/bin/codesign', ['--verify', '--strict', `-R=${requirements[0]}`, applications[1]]);
    expect(adHoc.status).not.toBe(0);
  }, 60_000);

  it('rejects a different certificate and leaves no usable signing environment or credential copy', () => {
    environment.SNAPLINGO_CERTIFICATE_SHA1 = '0'.repeat(40);
    const result = spawnSync(process.execPath, ['script/import-macos-certificate.mjs'], {
      env: environment, encoding: 'utf8', timeout: 20_000,
    });
    expect(result.status).not.toBe(0);
    expect(result.stderr).toContain('Certificate does not match the pinned release identity');
    expect(result.stderr).not.toContain(environment.MACOS_CERTIFICATE_PASSWORD);
    expect(result.stderr).not.toContain(environment.MACOS_CERTIFICATE_P12);
    expect(existsSync(environment.GITHUB_ENV!)).toBe(false);
    expect(readdirSync(directory).some(name => name.startsWith('snaplingo-signing-'))).toBe(false);
  });

  it.each(['self-signed', 'adhoc'])('loads the separately bundled OCR runtime from the hardened %s application', mode => {
    environment.SNAPLINGO_SIGNING_MODE = mode;
    if (mode === 'self-signed') {
      run(process.execPath, ['script/import-macos-certificate.mjs']);
      for (const line of readFileSync(environment.GITHUB_ENV!, 'utf8').trim().split('\n')) {
        const separator = line.indexOf('=');
        environment[line.slice(0, separator)] = line.slice(separator + 1);
      }
    }

    const app = join(directory, 'SnapLingo.app');
    const binary = join(app, 'Contents/MacOS/probe');
    const runtime = join(app, 'Contents/Resources/ocr/runtime/libonnxruntime.dylib');
    mkdirSync(join(app, 'Contents/MacOS'), { recursive: true });
    mkdirSync(join(app, 'Contents/Resources/ocr/runtime'), { recursive: true });
    const plist = join(app, 'Contents/Info.plist');
    writeFileSync(plist, JSON.stringify({ CFBundleIdentifier: 'com.snaplingo.app', CFBundleExecutable: 'probe',
      CFBundlePackageType: 'APPL', CFBundleVersion: '1', CFBundleShortVersionString: '1' }));
    run('/usr/bin/plutil', ['-convert', 'xml1', plist]);
    const librarySource = join(directory, 'runtime.c');
    writeFileSync(librarySource, 'int runtime_value(void) { return 42; }\n');
    run('/usr/bin/clang', ['-dynamiclib', librarySource, '-o', runtime]);
    const source = join(directory, 'probe.c');
    writeFileSync(source, `#include <dlfcn.h>
#include <stdio.h>
int main(int argc, char **argv) {
  if (argc != 2) return 2;
  void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
  if (!library) { fprintf(stderr, "%s\\n", dlerror()); return 1; }
  int (*value)(void) = (int (*)(void))dlsym(library, "runtime_value");
  if (!value || value() != 42) return 3;
  puts("OCR runtime loaded");
  dlclose(library);
  return 0;
}
`);
    run('/usr/bin/clang', [source, '-o', binary]);
    // dlopen-only resources do not appear in the executable's dependency list.
    expect(run('/usr/bin/otool', ['-L', binary])).not.toContain('libonnxruntime');
    run(process.execPath, ['script/fix-macos-release-signing.mjs', app]);
    run('/usr/bin/codesign', ['--verify', '--deep', '--strict', app]);
    expect(run('/usr/bin/codesign', ['-dv', '--verbose=4', binary])).toContain('runtime)');
    expect(run(binary, [runtime])).toContain('OCR runtime loaded');
  }, 60_000);
});
