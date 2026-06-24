import {
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { homedir, tmpdir } from "node:os";
import { randomBytes } from "node:crypto";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const config = JSON.parse(readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"));
const identifier = config.identifier;
const productName = config.productName ?? "SnapLingo";
const appPath = resolve(
  root,
  process.argv[2] ?? `target/release/bundle/macos/${productName}.app`,
);
const entitlementsPath = resolve(root, "src-tauri/entitlements.plist");
const localCodesignDir = join(homedir(), ".snaplingo", "codesign");
const localKeychainPath = join(localCodesignDir, "SnapLingoLocalCodesign.keychain-db");
const localKeychainPasswordPath = join(localCodesignDir, "keychain-password.txt");
const localIdentityName = "SnapLingo Local Code Signing";

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
  });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed:\n${output}`);
  }

  return output;
}

function assertIncludes(text, expected, message) {
  if (!text.includes(expected)) {
    throw new Error(`${message}\nExpected: ${expected}\nActual:\n${text}`);
  }
}

function assertDoesNotInclude(text, unexpected, message) {
  if (text.includes(unexpected)) {
    throw new Error(`${message}\nUnexpected: ${unexpected}\nActual:\n${text}`);
  }
}

function parseFirstCodesignIdentity(output, preferredName, excludedNames = []) {
  const lines = output.split("\n");
  const identityLine = preferredName
    ? lines.find((line) => line.includes(`"${preferredName}"`))
    : lines.find((line) => {
        const name = line.match(/"(.+)"/)?.[1];
        return /^\s*\d+\)\s+[0-9A-F]+\s+".+"/.test(line) && !excludedNames.includes(name);
      });

  return identityLine?.match(/"(.+)"/)?.[1] ?? null;
}

function getOrCreateLocalKeychainPassword() {
  mkdirSync(localCodesignDir, { recursive: true });

  if (existsSync(localKeychainPasswordPath)) {
    return readFileSync(localKeychainPasswordPath, "utf8").trim();
  }

  const password = randomBytes(24).toString("hex");
  writeFileSync(localKeychainPasswordPath, `${password}\n`, { mode: 0o600 });
  chmodSync(localKeychainPasswordPath, 0o600);
  return password;
}

function ensureKeychainInSearchList(keychainPath) {
  const output = run("/usr/bin/security", ["list-keychains", "-d", "user"]);
  const keychains = output
    .split("\n")
    .map((line) => line.trim().replace(/^"|"$/g, ""))
    .filter(Boolean);

  if (keychains.includes(keychainPath)) {
    return;
  }

  run("/usr/bin/security", ["list-keychains", "-d", "user", "-s", keychainPath, ...keychains]);
}

function ensureLocalCodesignIdentity() {
  const password = getOrCreateLocalKeychainPassword();
  const opensslConfigPath = join(localCodesignDir, "SnapLingoLocalCodeSigning.openssl.cnf");
  const keyPath = join(localCodesignDir, "SnapLingoLocalCodeSigning.key.pem");
  const certPath = join(localCodesignDir, "SnapLingoLocalCodeSigning.cert.pem");
  const p12Path = join(localCodesignDir, "SnapLingoLocalCodeSigning.p12");

  if (!existsSync(localKeychainPath)) {
    run("/usr/bin/security", ["create-keychain", "-p", password, localKeychainPath]);
  }

  run("/usr/bin/security", ["unlock-keychain", "-p", password, localKeychainPath]);
  run("/usr/bin/security", ["set-keychain-settings", "-lut", "21600", localKeychainPath]);
  ensureKeychainInSearchList(localKeychainPath);

  let identity = parseFirstCodesignIdentity(
    run("/usr/bin/security", ["find-identity", "-v", "-p", "codesigning", localKeychainPath]),
    localIdentityName,
  );

  if (identity) {
    return { identity, keychain: localKeychainPath };
  }

  writeFileSync(
    opensslConfigPath,
    `[ req ]
default_bits = 2048
prompt = no
default_md = sha256
distinguished_name = dn
x509_extensions = v3_req

[ dn ]
CN = ${localIdentityName}

[ v3_req ]
basicConstraints = critical, CA:TRUE
keyUsage = critical, digitalSignature, keyCertSign
extendedKeyUsage = codeSigning
subjectKeyIdentifier = hash
`,
  );

  run("/usr/bin/openssl", [
    "req",
    "-x509",
    "-newkey",
    "rsa:2048",
    "-nodes",
    "-days",
    "3650",
    "-keyout",
    keyPath,
    "-out",
    certPath,
    "-config",
    opensslConfigPath,
  ]);
  run("/usr/bin/openssl", [
    "pkcs12",
    "-export",
    "-inkey",
    keyPath,
    "-in",
    certPath,
    "-out",
    p12Path,
    "-name",
    localIdentityName,
    "-passout",
    `pass:${password}`,
  ]);
  run("/usr/bin/security", ["import", p12Path, "-k", localKeychainPath, "-P", password, "-A"]);
  run("/usr/bin/security", [
    "add-trusted-cert",
    "-r",
    "trustRoot",
    "-p",
    "codeSign",
    "-k",
    localKeychainPath,
    certPath,
  ]);
  run("/usr/bin/security", [
    "set-key-partition-list",
    "-S",
    "apple-tool:,apple:",
    "-s",
    "-k",
    password,
    localKeychainPath,
  ]);

  identity = parseFirstCodesignIdentity(
    run("/usr/bin/security", ["find-identity", "-v", "-p", "codesigning", localKeychainPath]),
    localIdentityName,
  );

  if (!identity) {
    throw new Error("[macos-sign] Failed to create a local code signing identity.");
  }

  return { identity, keychain: localKeychainPath };
}

function resolveCodesignIdentity() {
  const configuredIdentity =
    process.env.SNAPLINGO_CODESIGN_IDENTITY ?? process.env.MACOS_CODESIGN_IDENTITY;

  if (configuredIdentity) {
    return { identity: configuredIdentity, keychain: process.env.SNAPLINGO_CODESIGN_KEYCHAIN };
  }

  const systemIdentity = parseFirstCodesignIdentity(
    run("/usr/bin/security", ["find-identity", "-v", "-p", "codesigning"]),
    null,
    [localIdentityName],
  );

  if (systemIdentity) {
    return { identity: systemIdentity };
  }

  return ensureLocalCodesignIdentity();
}

function verifyAppSignature(path) {
  run("/usr/bin/codesign", ["--verify", "--deep", "--strict", "--verbose=4", path]);

  const signatureDetails = run("/usr/bin/codesign", ["-dv", "--verbose=4", path]);
  assertIncludes(
    signatureDetails,
    `Identifier=${identifier}`,
    "[macos-sign] The code signing identifier is not stable.",
  );
  assertIncludes(
    signatureDetails,
    "Sealed Resources version=",
    "[macos-sign] The app bundle resources are not sealed.",
  );
  assertDoesNotInclude(
    signatureDetails,
    "Signature=adhoc",
    "[macos-sign] The app must not remain ad-hoc signed; TCC Screen Recording permission is unstable for ad-hoc builds.",
  );

  const requirementDetails = run("/usr/bin/codesign", ["-d", "-r-", path]);
  assertIncludes(
    requirementDetails,
    `identifier "${identifier}"`,
    "[macos-sign] The designated requirement does not include the stable bundle identifier.",
  );
  assertDoesNotInclude(
    requirementDetails,
    "cdhash H",
    "[macos-sign] The designated requirement must not be tied to a per-build cdhash.",
  );
}

function findDmgPaths() {
  const dmgDir = resolve(root, "target/release/bundle/dmg");
  if (!existsSync(dmgDir)) {
    return [];
  }

  return readdirSync(dmgDir)
    .filter((name) => name.startsWith(`${productName}_`) && name.endsWith(".dmg") && !name.startsWith("rw."))
    .map((name) => resolve(dmgDir, name));
}

function recreateDmg(dmgPath) {
  const stagingDir = mkdtempSync(join(tmpdir(), "snaplingo-dmg-"));
  const stagedAppPath = join(stagingDir, `${productName}.app`);

  try {
    cpSync(appPath, stagedAppPath, {
      recursive: true,
      verbatimSymlinks: true,
      preserveTimestamps: true,
    });
    symlinkSync("/Applications", join(stagingDir, "Applications"));

    run("/usr/bin/hdiutil", [
      "create",
      "-volname",
      productName,
      "-srcfolder",
      stagingDir,
      "-ov",
      "-format",
      "UDZO",
      dmgPath,
    ]);

    const attachOutput = run("/usr/bin/hdiutil", ["attach", dmgPath, "-nobrowse", "-readonly"]);
    const mountPoint = attachOutput
      .split("\n")
      .map((line) => line.split(/\t+/).at(-1)?.trim())
      .find((part) => part?.startsWith("/Volumes/"));

    if (!mountPoint) {
      throw new Error(`[macos-sign] Failed to find mounted volume for ${dmgPath}.`);
    }

    try {
      verifyAppSignature(join(mountPoint, `${productName}.app`));
    } finally {
      run("/usr/bin/hdiutil", ["detach", mountPoint]);
    }
  } finally {
    rmSync(stagingDir, { recursive: true, force: true });
  }
}

if (process.platform !== "darwin") {
  console.log("[macos-sign] Skipping macOS signing check on non-macOS host.");
  process.exit(0);
}

if (!identifier) {
  throw new Error("[macos-sign] Missing top-level identifier in src-tauri/tauri.conf.json.");
}

if (!existsSync(appPath)) {
  throw new Error(`[macos-sign] App bundle not found: ${appPath}`);
}

const infoPlistPath = resolve(appPath, "Contents/Info.plist");
const bundleIdentifier = run("/usr/bin/plutil", [
  "-extract",
  "CFBundleIdentifier",
  "raw",
  "-o",
  "-",
  infoPlistPath,
]).trim();

if (bundleIdentifier !== identifier) {
  throw new Error(
    `[macos-sign] CFBundleIdentifier (${bundleIdentifier}) does not match Tauri identifier (${identifier}).`,
  );
}

const signing = resolveCodesignIdentity();
const signArgs = [
  "--force",
  "--deep",
  "--sign",
  signing.identity,
  "--identifier",
  identifier,
];

if (signing.keychain) {
  signArgs.push("--keychain", signing.keychain);
}

if (existsSync(entitlementsPath)) {
  signArgs.push("--entitlements", entitlementsPath);
}

signArgs.push(appPath);

run("/usr/bin/codesign", signArgs);
verifyAppSignature(appPath);

for (const dmgPath of findDmgPaths()) {
  recreateDmg(dmgPath);
  console.log(`[macos-sign] Recreated and verified ${dmgPath}`);
}

console.log(`[macos-sign] Signed and verified ${appPath}`);
console.log(`[macos-sign] Identifier: ${identifier}`);
console.log(`[macos-sign] Signing identity: ${signing.identity}`);
