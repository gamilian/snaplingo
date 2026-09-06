import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const repository = process.argv[2];
if (process.platform !== 'darwin' || !/^[\w.-]+\/[\w.-]+$/.test(repository ?? '')) {
  throw new Error('Usage on macOS: node script/configure-macos-release.mjs OWNER/REPOSITORY');
}
function run(command, args, input) {
  const result = spawnSync(command, args, { input, encoding: 'utf8' });
  if (result.error || result.status !== 0) throw new Error(`${command} failed; verify account access and local certificate (exit ${result.status})`);
  return result.stdout;
}
const repo = JSON.parse(run('gh', ['repo', 'view', repository, '--json', 'visibility,nameWithOwner']));
if (repo.visibility !== 'PUBLIC') throw new Error('This zero-cost setup is restricted to public repositories');
run(process.execPath, ['script/fix-macos-release-signing.mjs', '--prepare-identity']);
const directory = join(homedir(), '.snaplingo/codesign');
const cert = join(directory, 'SnapLingoLocalCodeSigning.cert.pem');
const p12 = join(directory, 'SnapLingoLocalCodeSigning.p12');
if (!existsSync(p12) || !existsSync(cert)) throw new Error('Persistent certificate export missing; export the SAME identity from Keychain Access. See docs/RELEASING.md');
const fingerprint = run('/usr/bin/openssl', ['x509', '-in', cert, '-noout', '-fingerprint', '-sha1'])
  .split('=').at(-1).replace(/[^a-f0-9]/gi, '').toUpperCase();
const variables = JSON.parse(run('gh', ['variable', 'list', '--repo', repository, '--json', 'name,value']));
const previous = variables.find(variable => variable.name === 'MACOS_CERTIFICATE_SHA1');
if (previous && previous.value.toUpperCase() !== fingerprint) {
  throw new Error('The repository pins another signing identity. Restore its certificate rather than rotating it.');
}
const password = readFileSync(join(directory, 'keychain-password.txt'), 'utf8').trim();
// Credentials travel only through stdin, never command arguments or output.
run('gh', ['secret', 'set', 'MACOS_CERTIFICATE_P12', '--repo', repository], readFileSync(p12).toString('base64'));
run('gh', ['secret', 'set', 'MACOS_CERTIFICATE_PASSWORD', '--repo', repository], password);
run('gh', ['variable', 'set', 'MACOS_CERTIFICATE_SHA1', '--repo', repository], fingerprint);
console.log(`Configured ${repo.nameWithOwner}; pinned certificate SHA-1: ${fingerprint}`);
console.log(`Back up the encrypted .p12 and its password from ${directory} separately and securely.`);
