import { spawnSync } from 'node:child_process';
import { appendFileSync, chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';

if (process.platform !== 'darwin' || process.env.GITHUB_ACTIONS !== 'true') {
  throw new Error('This certificate importer is only for ephemeral macOS GitHub runners');
}
for (const name of ['MACOS_CERTIFICATE_P12', 'MACOS_CERTIFICATE_PASSWORD', 'SNAPLINGO_CERTIFICATE_SHA1', 'GITHUB_ENV', 'RUNNER_TEMP']) {
  if (!process.env[name]) throw new Error(`Missing ${name}; see docs/RELEASING.md`);
}
const fingerprint = process.env.SNAPLINGO_CERTIFICATE_SHA1.toUpperCase();
if (!/^[A-F0-9]{40}$/.test(fingerprint)) throw new Error('Expected a 40-character SHA-1 certificate fingerprint');
const directory = mkdtempSync(join(process.env.RUNNER_TEMP, 'snaplingo-signing-'));
chmodSync(directory, 0o700);
const p12 = join(directory, 'certificate.p12');
const cert = join(directory, 'certificate.pem');
const keychain = join(directory, 'signing.keychain-db');
const password = randomBytes(32).toString('hex');
function run(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', timeout: 60_000 });
  // Never include arguments, stdout or stderr: credential tooling may echo secrets.
  if (result.error || result.status !== 0) {
    throw new Error(`${command} failed (exit ${result.status ?? 'timeout'})`);
  }
  return result.stdout;
}
function runStage(label, command, args) {
  console.log(`[macos-sign] ${label}`);
  return run(command, args);
}
try {
  writeFileSync(p12, Buffer.from(process.env.MACOS_CERTIFICATE_P12, 'base64'), { mode: 0o600 });
  runStage('Validate certificate bundle', '/usr/bin/openssl', ['pkcs12', '-in', p12, '-clcerts', '-nokeys', '-out', cert, '-passin', 'env:MACOS_CERTIFICATE_PASSWORD']);
  const actual = runStage('Check certificate fingerprint', '/usr/bin/openssl', ['x509', '-in', cert, '-noout', '-fingerprint', '-sha1']).split('=').at(-1).replace(/[^a-f0-9]/gi, '').toUpperCase();
  if (actual !== fingerprint) throw new Error('Certificate does not match the pinned release identity');
  runStage('Create temporary keychain', '/usr/bin/security', ['create-keychain', '-p', password, keychain]);
  runStage('Configure temporary keychain', '/usr/bin/security', ['set-keychain-settings', '-lut', '21600', keychain]);
  runStage('Unlock temporary keychain', '/usr/bin/security', ['unlock-keychain', '-p', password, keychain]);
  runStage('Import signing identity', '/usr/bin/security', ['import', p12, '-k', keychain, '-P', process.env.MACOS_CERTIFICATE_PASSWORD, '-T', '/usr/bin/codesign']);
  runStage('Authorize codesign access', '/usr/bin/security', ['set-key-partition-list', '-S', 'apple-tool:,apple:', '-s', '-k', password, keychain]);
  const keychains = runStage('Read keychain search list', '/usr/bin/security', ['list-keychains', '-d', 'user'])
    .match(/"([^"]+)"/g)?.map(value => value.slice(1, -1)) ?? [];
  runStage('Add temporary keychain to search list', '/usr/bin/security', ['list-keychains', '-d', 'user', '-s', keychain, ...keychains]);
  const identities = runStage('Verify imported signing identity', '/usr/bin/security', ['find-identity', '-v', '-p', 'codesigning', keychain]);
  if (!identities.includes(fingerprint)) throw new Error('Imported keychain does not contain the pinned signing identity');
  appendFileSync(process.env.GITHUB_ENV, `SNAPLINGO_CODESIGN_IDENTITY=${fingerprint}\nSNAPLINGO_CODESIGN_KEYCHAIN=${keychain}\nSNAPLINGO_SIGNING_DIRECTORY=${directory}\nSNAPLINGO_CERTIFICATE_SHA1=${fingerprint}\n`);
} catch (error) {
  spawnSync('/usr/bin/security', ['delete-keychain', keychain]);
  rmSync(directory, { recursive: true, force: true });
  throw error;
} finally {
  rmSync(p12, { force: true });
  rmSync(cert, { force: true });
}
