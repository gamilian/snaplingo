import { randomBytes } from "node:crypto";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`[macos-sign] Missing ${name}.`);
  return value;
}

function run(command, args) {
  const result = spawnSync(command, args, { encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`${command} failed: ${(result.stderr || result.stdout).trim()}`);
  }
  return `${result.stdout || ""}${result.stderr || ""}`;
}

if (process.platform !== "darwin") {
  throw new Error("[macos-sign] A macOS runner is required to import the signing identity.");
}

const environmentPath = required("GITHUB_ENV");
const certificate = Buffer.from(
  required("SNAPLINGO_MACOS_SIGNING_CERTIFICATE_P12_BASE64"),
  "base64",
);
if (certificate.length === 0) {
  throw new Error("[macos-sign] The signing certificate is not valid base64.");
}

const certificatePassword = required("SNAPLINGO_MACOS_SIGNING_CERTIFICATE_PASSWORD");
const temporaryDirectory = mkdtempSync(join(process.env.RUNNER_TEMP || tmpdir(), "snaplingo-signing-"));
const certificatePath = join(temporaryDirectory, "certificate.p12");
const certificatePemPath = join(temporaryDirectory, "certificate.pem");
const keychainPath = join(temporaryDirectory, "SnapLingoRelease.keychain-db");
const keychainPassword = randomBytes(24).toString("hex");

try {
  writeFileSync(certificatePath, certificate, { mode: 0o600 });
  chmodSync(certificatePath, 0o600);
  run("/usr/bin/security", ["create-keychain", "-p", keychainPassword, keychainPath]);
  run("/usr/bin/security", ["unlock-keychain", "-p", keychainPassword, keychainPath]);
  run("/usr/bin/security", ["set-keychain-settings", "-lut", "21600", keychainPath]);
  run("/usr/bin/security", [
    "import",
    certificatePath,
    "-k",
    keychainPath,
    "-P",
    certificatePassword,
    "-A",
    "-t",
    "cert",
    "-f",
    "pkcs12",
  ]);

  writeFileSync(
    certificatePemPath,
    run("/usr/bin/security", ["find-certificate", "-p", keychainPath]),
    { mode: 0o600 },
  );
  run("/usr/bin/security", [
    "add-trusted-cert",
    "-r",
    "trustRoot",
    "-p",
    "codeSign",
    "-k",
    keychainPath,
    certificatePemPath,
  ]);
  const imported = run("/usr/bin/security", [
    "find-identity",
    "-v",
    "-p",
    "codesigning",
    keychainPath,
  ]);
  const identity = imported.match(/^\s*\d+\)\s+[0-9A-F]+\s+"(.+)"/m)?.[1];
  if (!identity) {
    throw new Error("[macos-sign] The imported certificate is not a code-signing identity.");
  }
  run("/usr/bin/security", [
    "set-key-partition-list",
    "-S",
    "apple-tool:,apple:",
    "-s",
    "-k",
    keychainPassword,
    keychainPath,
  ]);

  const fingerprint = run("/usr/bin/security", [
    "find-certificate",
    "-Z",
    "-c",
    identity,
    keychainPath,
  ]).match(/SHA-1 hash:\s*([A-F0-9]{40})/i)?.[1];
  if (!fingerprint) {
    throw new Error("[macos-sign] Could not read the signing certificate fingerprint.");
  }

  writeFileSync(
    environmentPath,
    `SNAPLINGO_CODESIGN_IDENTITY=${identity}\nSNAPLINGO_CODESIGN_KEYCHAIN=${keychainPath}\nSNAPLINGO_CERTIFICATE_SHA1=${fingerprint.toUpperCase()}\n`,
    { flag: "a" },
  );
  console.log(`[macos-sign] Imported persistent signing identity: ${identity}`);
  console.log(`[macos-sign] Certificate SHA-1: ${fingerprint.toUpperCase()}`);
} finally {
  rmSync(certificatePath, { force: true });
  rmSync(certificatePemPath, { force: true });
}
