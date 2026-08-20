import {
  chmodSync,
  copyFileSync,
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
import { basename, dirname, join, resolve } from "node:path";
import { homedir, tmpdir } from "node:os";
import { randomBytes } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  findAttachedDiskImageDevices,
  retryDiskImageAttach,
} from "./macos-disk-image.mjs";

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

function listMachODependencies(path) {
  return run("/usr/bin/otool", ["-L", path])
    .split("\n")
    .slice(1)
    .map((line) => line.trim().match(/^(.+?) \(compatibility version/)?.[1])
    .filter(Boolean);
}

function isExternalMachODependency(path) {
  return (
    path.startsWith("/") &&
    !path.startsWith("/System/") &&
    !path.startsWith("/usr/lib/")
  );
}

function listMachORpaths(path) {
  const lines = run("/usr/bin/otool", ["-l", path]).split("\n");
  const rpaths = [];

  for (let index = 0; index < lines.length; index += 1) {
    if (lines[index].trim() !== "cmd LC_RPATH") {
      continue;
    }

    const pathLine = lines.slice(index + 1, index + 5)
      .find((line) => line.trim().startsWith("path "));
    const rpath = pathLine?.trim().match(/^path (.+) \(offset/)?.[1];
    if (rpath) {
      rpaths.push(rpath);
    }
  }

  return rpaths;
}

function resolveMachOPath(sourcePath, path) {
  if (path.startsWith("@loader_path/")) {
    return resolve(dirname(sourcePath), path.slice("@loader_path/".length));
  }

  return path.startsWith("/") ? path : null;
}

function resolveExternalDependency(sourcePath, dependencyPath) {
  if (isExternalMachODependency(dependencyPath)) {
    return dependencyPath;
  }

  if (dependencyPath.startsWith("@loader_path/")) {
    const resolvedPath = resolveMachOPath(sourcePath, dependencyPath);
    return resolvedPath && isExternalMachODependency(resolvedPath) && existsSync(resolvedPath)
      ? resolvedPath
      : null;
  }

  if (!dependencyPath.startsWith("@rpath/")) {
    return null;
  }

  const relativePath = dependencyPath.slice("@rpath/".length);
  const candidates = [
    resolve(dirname(sourcePath), relativePath),
    ...listMachORpaths(sourcePath)
      .map((rpath) => resolveMachOPath(sourcePath, rpath))
      .filter(Boolean)
      .map((rpath) => resolve(rpath, relativePath)),
  ];

  return candidates.find((path) => isExternalMachODependency(path) && existsSync(path)) ?? null;
}

function bundleExternalMachODependencies(executablePath) {
  const frameworksPath = resolve(appPath, "Contents/Frameworks");
  const dependenciesBySource = new Map();
  const externalByPath = new Map();
  const sourceByBasename = new Map();
  const queue = [executablePath];

  while (queue.length > 0) {
    const sourcePath = queue.shift();
    if (dependenciesBySource.has(sourcePath)) {
      continue;
    }

    const dependencies = listMachODependencies(sourcePath);
    dependenciesBySource.set(sourcePath, dependencies);

    for (const dependencyPath of dependencies) {
      const dependencySourcePath = resolveExternalDependency(sourcePath, dependencyPath);
      if (!dependencySourcePath) {
        continue;
      }

      if (!existsSync(dependencySourcePath)) {
        throw new Error(`[macos-sign] External dependency not found: ${dependencySourcePath}`);
      }

      const fileName = basename(dependencySourcePath);
      const existingSource = sourceByBasename.get(fileName);
      if (existingSource && existingSource !== dependencySourcePath) {
        throw new Error(
          `[macos-sign] Cannot bundle two dependencies named ${fileName}: ${existingSource}, ${dependencySourcePath}`,
        );
      }

      sourceByBasename.set(fileName, dependencySourcePath);
      externalByPath.set(dependencySourcePath, join(frameworksPath, fileName));
      queue.push(dependencySourcePath);
    }
  }

  if (externalByPath.size === 0) {
    return [];
  }

  mkdirSync(frameworksPath, { recursive: true });
  for (const [sourcePath, destinationPath] of externalByPath) {
    copyFileSync(sourcePath, destinationPath);
    chmodSync(destinationPath, 0o755);
  }

  for (const [sourcePath, dependencies] of dependenciesBySource) {
    const destinationPath = externalByPath.get(sourcePath) ?? sourcePath;
    const dependencyPrefix = sourcePath === executablePath
      ? "@executable_path/../Frameworks"
      : "@loader_path";

    for (const dependencyPath of dependencies) {
      const dependencySourcePath = resolveExternalDependency(sourcePath, dependencyPath);
      const bundledPath = externalByPath.get(dependencySourcePath);
      if (!bundledPath) {
        continue;
      }

      run("/usr/bin/install_name_tool", [
        "-change",
        dependencyPath,
        `${dependencyPrefix}/${basename(bundledPath)}`,
        destinationPath,
      ]);
    }

    if (sourcePath !== executablePath) {
      run("/usr/bin/install_name_tool", [
        "-id",
        `@rpath/${basename(destinationPath)}`,
        destinationPath,
      ]);
    }
  }

  const bundledPaths = [...externalByPath.values()];
  for (const path of [executablePath, ...bundledPaths]) {
    const dependencies = listMachODependencies(path);
    const remainingExternal = dependencies.filter(isExternalMachODependency);
    if (remainingExternal.length > 0) {
      throw new Error(
        `[macos-sign] ${path} still references external dependencies: ${remainingExternal.join(", ")}`,
      );
    }

    const missingRelative = dependencies.filter((dependencyPath) => {
      if (!dependencyPath.startsWith("@loader_path/") && !dependencyPath.startsWith("@rpath/")) {
        return false;
      }

      const relativePath = dependencyPath.slice(dependencyPath.indexOf("/") + 1);
      return !existsSync(resolve(dirname(path), relativePath)) &&
        !existsSync(resolve(frameworksPath, relativePath));
    });
    if (missingRelative.length > 0) {
      throw new Error(
        `[macos-sign] ${path} has unresolved bundled dependencies: ${missingRelative.join(", ")}`,
      );
    }
  }

  return bundledPaths;
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

  return ensureLocalCodesignIdentity();
}

function prepareSigningEntitlements(signing) {
  if (!existsSync(entitlementsPath) || signing.identity !== localIdentityName) {
    return { path: existsSync(entitlementsPath) ? entitlementsPath : null, cleanup: () => {} };
  }

  const temporaryDir = mkdtempSync(join(tmpdir(), "snaplingo-entitlements-"));
  const temporaryPath = join(temporaryDir, "entitlements.plist");
  copyFileSync(entitlementsPath, temporaryPath);
  run("/usr/libexec/PlistBuddy", [
    "-c",
    "Add :com.apple.security.cs.disable-library-validation bool true",
    temporaryPath,
  ]);

  return {
    path: temporaryPath,
    cleanup: () => rmSync(temporaryDir, { recursive: true, force: true }),
  };
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
  assertIncludes(
    signatureDetails,
    "flags=0x10000(runtime)",
    "[macos-sign] Hardened runtime is required for release signing.",
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

function notarizationArgs(path) {
  const profile = process.env.SNAPLINGO_NOTARY_PROFILE;
  if (!profile) {
    throw new Error(
      "[macos-sign] SNAPLINGO_NOTARIZE=1 requires SNAPLINGO_NOTARY_PROFILE. Create it with xcrun notarytool store-credentials so secrets never enter build process arguments.",
    );
  }

  return ["notarytool", "submit", path, "--keychain-profile", profile, "--wait"];
}

function notarizeAndStaple(path) {
  run("/usr/bin/xcrun", notarizationArgs(path));
  run("/usr/bin/xcrun", ["stapler", "staple", path]);
  run("/usr/bin/xcrun", ["stapler", "validate", path]);
}

function verifyDmgSignature(path) {
  run("/usr/bin/codesign", ["--verify", "--strict", "--verbose=4", path]);
  const signatureDetails = run("/usr/bin/codesign", ["-dv", "--verbose=4", path]);
  assertDoesNotInclude(
    signatureDetails,
    "Signature=adhoc",
    "[macos-sign] The disk image must not remain ad-hoc signed.",
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

function detachExistingDmgAttachments(dmgPath) {
  const info = run("/usr/bin/hdiutil", ["info"]);
  for (const device of findAttachedDiskImageDevices(info, dmgPath)) {
    run("/usr/bin/hdiutil", ["detach", device]);
  }
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

    detachExistingDmgAttachments(dmgPath);
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

    const attachOutput = retryDiskImageAttach(
      () => run("/usr/bin/hdiutil", ["attach", dmgPath, "-nobrowse", "-readonly"]),
      (attempt) => {
        console.warn(
          `[macos-sign] Could not mount ${dmgPath} for verification (attempt ${attempt}/3); retrying.`,
        );
        detachExistingDmgAttachments(dmgPath);
        run("/bin/sleep", [String(attempt)]);
      },
    );
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

const executableName = run("/usr/bin/plutil", [
  "-extract",
  "CFBundleExecutable",
  "raw",
  "-o",
  "-",
  infoPlistPath,
]).trim();
const executablePath = resolve(appPath, "Contents/MacOS", executableName);
const bundledLibraries = bundleExternalMachODependencies(executablePath);

const signing = resolveCodesignIdentity();
const signingEntitlements = prepareSigningEntitlements(signing);
const signArgs = [
  "--force",
  "--deep",
  "--sign",
  signing.identity,
  "--options",
  "runtime",
  "--identifier",
  identifier,
];

if (signing.keychain) {
  signArgs.push("--keychain", signing.keychain);
}

if (signingEntitlements.path) {
  signArgs.push("--entitlements", signingEntitlements.path);
}

signArgs.push(appPath);

try {
  run("/usr/bin/codesign", signArgs);
} finally {
  signingEntitlements.cleanup();
}
verifyAppSignature(appPath);

for (const dmgPath of findDmgPaths()) {
  recreateDmg(dmgPath);
  const dmgSignArgs = ["--force", "--sign", signing.identity];
  if (signing.keychain) {
    dmgSignArgs.push("--keychain", signing.keychain);
  }
  dmgSignArgs.push(dmgPath);
  run("/usr/bin/codesign", dmgSignArgs);
  verifyDmgSignature(dmgPath);
  if (process.env.SNAPLINGO_NOTARIZE === "1") {
    notarizeAndStaple(dmgPath);
  }
  console.log(`[macos-sign] Recreated and verified ${dmgPath}`);
}

console.log(`[macos-sign] Signed and verified ${appPath}`);
console.log(`[macos-sign] Bundled ${bundledLibraries.length} external libraries`);
console.log(`[macos-sign] Identifier: ${identifier}`);
console.log(`[macos-sign] Signing identity: ${signing.identity}`);
if (signing.identity === localIdentityName) {
  console.warn(
    "[macos-sign] Distribution: small-test beta; testers must approve the app in macOS Privacy & Security on first launch.",
  );
}
