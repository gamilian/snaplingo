import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  copyFileSync,
  writeFileSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { verifyMacOSApplication } from "./macos-release-verification.mjs";
import { verifyNativeWindowsBundle } from "./native-windows-build.mjs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repositoryRoot = resolve(dirname(scriptPath), "..");

export function assertUnifiedReleaseVersion(versions) {
  const entries = Object.entries(versions);
  const missing = entries.filter(([, version]) => !version);
  if (missing.length > 0) {
    throw new Error(
      `Release version is missing from: ${missing.map(([source]) => source).join(", ")}`,
    );
  }

  const expected = entries[0][1];
  const mismatches = entries.filter(([, version]) => version !== expected);
  if (mismatches.length > 0) {
    throw new Error(
      `Release versions must match: ${entries
        .map(([source, version]) => `${source}=${version}`)
        .join(", ")}`,
    );
  }

  return expected;
}

export function assertMatchingReleaseTag(version, environment = process.env) {
  if (
    environment.GITHUB_REF_TYPE === "tag" &&
    environment.GITHUB_REF_NAME !== `v${version}`
  ) {
    throw new Error(
      `Release tag ${environment.GITHUB_REF_NAME} must match version v${version}`,
    );
  }
}

export function verifyReleaseArtifacts({
  platform,
  bundleDirectory,
  productName,
  version,
}) {
  const contract = releaseArtifactContract(platform, productName, version);
  return contract.map((artifact) => verifyArtifact(bundleDirectory, artifact));
}

export function releaseAssetName({ projectName, version, target, kind }) {
  const extension = {
    "macOS disk image": ".dmg",
    "Windows MSI installer": ".msi",
    "Windows NSIS installer": "-setup.exe",
    "Windows portable archive": "-portable.zip",
  }[kind];
  if (!extension) throw new Error(`Unsupported release artifact kind: ${kind}`);
  return `${projectName}-v${version}-${target}${extension}`;
}

export function releaseArtifactContract(platform, productName, version) {
  switch (platform) {
    case "darwin":
      return [
        {
          kind: "macOS application",
          directory: ".",
          matches: (name) => name === `${productName}.app`,
          validate: validateMacOsApplication,
        },
        {
          kind: "macOS disk image",
          directory: "bundle/dmg",
          matches: (name) => ["aarch64", "x86_64"].some(architecture =>
            name === `${productName.toLowerCase()}-v${version}-macos-${architecture}.dmg`),
          validate: validateNonEmptyFile,
        },
      ];
    case "win32":
      return [
        { kind: "Windows MSI installer", directory: "bundle/msi", suffix: ".msi" },
        { kind: "Windows NSIS installer", directory: "bundle/nsis", suffix: "-setup.exe" },
        { kind: "Windows portable archive", directory: "bundle/portable", suffix: "-portable.zip" },
      ].map(artifact => ({
        ...artifact,
        matches: (name) => name === `${productName}_${version}_x64${artifact.suffix}`,
        validate: validateNonEmptyFile,
      }));
    default:
      throw new Error(`Unsupported release platform: ${platform}`);
  }
}

function verifyArtifact(bundleDirectory, artifact) {
  const artifactDirectory = join(bundleDirectory, artifact.directory);
  if (!existsSync(artifactDirectory)) {
    throw new Error(
      `${artifact.kind} directory was not produced: ${artifactDirectory}`,
    );
  }

  const artifactName = readdirSync(artifactDirectory)
    .sort()
    .find(artifact.matches);
  if (!artifactName) {
    throw new Error(`${artifact.kind} was not found in ${artifactDirectory}`);
  }

  const artifactPath = join(artifactDirectory, artifactName);
  return {
    kind: artifact.kind,
    path: artifactPath,
    size: artifact.validate(artifactPath),
  };
}

function validateMacOsApplication(applicationPath) {
  if (!statSync(applicationPath).isDirectory()) {
    throw new Error(`macOS application is not a directory: ${applicationPath}`);
  }

  const executableDirectory = join(applicationPath, "Contents", "MacOS");
  if (!existsSync(executableDirectory)) {
    throw new Error(
      `macOS executable directory is missing: ${executableDirectory}`,
    );
  }

  const executableName = readdirSync(executableDirectory)
    .sort()
    .find((name) => statSync(join(executableDirectory, name)).isFile());
  if (!executableName) {
    throw new Error(`macOS application has no executable: ${applicationPath}`);
  }

  return validateExecutableFile(join(executableDirectory, executableName));
}

function validateNonEmptyFile(path) {
  const metadata = statSync(path);
  if (!metadata.isFile() || metadata.size === 0) {
    throw new Error(`Release artifact is empty or not a file: ${path}`);
  }
  return metadata.size;
}

function validateExecutableFile(path) {
  const size = validateNonEmptyFile(path);
  if (process.platform !== "win32" && (statSync(path).mode & 0o111) === 0) {
    throw new Error(`Release executable is not executable: ${path}`);
  }
  return size;
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function readCargoLockPackageVersion(path, packageName) {
  let currentPackage = null;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    if (line === "[[package]]") {
      currentPackage = {};
      continue;
    }
    const name = line.match(/^name = "([^"]+)"$/)?.[1];
    if (name) {
      currentPackage = { ...currentPackage, name };
      continue;
    }
    const version = line.match(/^version = "([^"]+)"$/)?.[1];
    if (version && currentPackage?.name === packageName) {
      return version;
    }
  }

  throw new Error(`Cargo.lock package ${packageName} was not found`);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    encoding: "utf8",
    stdio: options.stdio ?? "pipe",
    shell: options.shell ?? false,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const output = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
    throw new Error(
      `${command} ${args.join(" ")} failed${output ? `:\n${output}` : ""}`,
    );
  }
  return result.stdout ?? "";
}

function loadReleaseContext() {
  const packageManifest = readJson(join(repositoryRoot, "package.json"));
  const desktopConfig = readJson(
    join(repositoryRoot, "crates/snaplingo-app", "desktop.json"),
  );
  const cargoMetadata = JSON.parse(
    run("cargo", [
      "metadata",
      "--no-deps",
      "--format-version",
      "1",
      "--locked",
      "--manifest-path",
      "crates/snaplingo-app/Cargo.toml",
    ]),
  );
  const cargoPackage = cargoMetadata.packages.find(
    (candidate) => candidate.name === packageManifest.name,
  );
  if (!cargoPackage) {
    throw new Error(`Cargo package ${packageManifest.name} was not found`);
  }

  const packageLock = readJson(join(repositoryRoot, "package-lock.json"));
  const packageLockRoot = packageLock.packages?.[""];
  const cargoLockVersion = readCargoLockPackageVersion(
    join(repositoryRoot, "Cargo.lock"),
    packageManifest.name,
  );

  const version = assertUnifiedReleaseVersion({
    "package.json": packageManifest.version,
    "package-lock.json": packageLock.version,
    "package-lock.json packages['']": packageLockRoot?.version,
    "Cargo.toml": cargoPackage.version,
    "Cargo.lock": cargoLockVersion,
  });
  assertMatchingReleaseTag(version);

  return {
    repositoryRoot,
    version,
    packageName: packageManifest.name,
    config: { ...desktopConfig, version },
    productName: desktopConfig.productName ?? packageManifest.name,
    targetDirectory: cargoMetadata.target_directory,
    bundleDirectory: join(cargoMetadata.target_directory, "native", "release"),
  };
}

function cleanReleaseOutputs(targetDirectory) {
  rmSync(join(targetDirectory, "release"), { recursive: true, force: true });
  rmSync(join(targetDirectory, "native", "release"), { recursive: true, force: true });
}

function buildRelease(context, args) {
  if (args.length) throw new Error(`Unsupported release build options: ${args.join(" ")}`);
  // Native packaging owns its staging directory; keep Cargo compilation caches.
  rmSync(context.bundleDirectory, { recursive: true, force: true });
  run(process.execPath, [join(repositoryRoot, "script", "native-build.mjs"), "build"],
    { stdio: "inherit" });
}

function verifyNativeMacOSArtifacts(context, artifacts) {
  const options = { allowAdhoc: process.env.SNAPLINGO_SIGNING_MODE === "adhoc" };
  const app = artifacts.find(artifact => artifact.kind === "macOS application");
  const signature = verifyMacOSApplication(app.path, context.config, options);
  const dmg = artifacts.find(artifact => artifact.kind === "macOS disk image");
  run("/usr/bin/codesign", ["--verify", "--strict", dmg.path]);
  run("/usr/bin/hdiutil", ["verify", dmg.path]);
  const mountOutput = run("/usr/bin/hdiutil", ["attach", dmg.path, "-nobrowse", "-readonly"]);
  const mount = mountOutput.split("\n").map(line => line.split(/\t+/).at(-1)?.trim())
    .find(part => part?.startsWith("/Volumes/"));
  if (!mount) throw new Error("Could not locate mounted release DMG");
  try {
    const mounted = verifyMacOSApplication(join(mount, `${context.productName}.app`), context.config, options);
    if (!options.allowAdhoc && mounted.requirement !== signature.requirement) {
      throw new Error("DMG signing identity differs from app");
    }
    if (!existsSync(join(mount, "Applications"))) throw new Error("DMG has no Applications link");
  } finally {
    run("/usr/bin/hdiutil", ["detach", mount]);
  }
  const assessment = spawnSync("/usr/sbin/spctl", ["--assess", "--type", "execute", "--verbose=4", app.path], { encoding: "utf8" });
  console.log(`[release] Gatekeeper (unnotarized beta rejection is expected): ${assessment.stderr?.trim()}`);
}

export function collectArtifacts(context, artifacts, environment = process.env) {
  const output = join(context.repositoryRoot, "release-assets");
  mkdirSync(output, { recursive: true });
  const checksums = [];
  const target = environment.SNAPLINGO_RELEASE_TARGET ?? releaseTarget();
  if (!/^(macos-(aarch64|x86_64)|windows-x86_64)$/.test(target)) throw new Error("Invalid release target");
  for (const artifact of artifacts) {
    if (statSync(artifact.path).isDirectory()) continue;
    const name = releaseAssetName({
      projectName: context.packageName,
      version: context.version,
      target,
      kind: artifact.kind,
    });
    copyFileSync(artifact.path, join(output, name));
    const hash = createHash("sha256").update(readFileSync(artifact.path)).digest("hex");
    checksums.push(`${hash}  ${name}`);
  }
  const assetPrefix = `${context.packageName}-v${context.version}-${target}`;
  writeFileSync(join(output, `${assetPrefix}.SHA256SUMS.txt`), `${checksums.join("\n")}\n`);
  writeFileSync(join(output, `${assetPrefix}.build.json`), JSON.stringify({
    version: context.version, commit: environment.GITHUB_SHA ?? run("git", ["rev-parse", "HEAD"]).trim(),
    platform: process.platform, architecture: process.arch,
    signing: process.platform === "darwin" ? (environment.SNAPLINGO_SIGNING_MODE ?? "self-signed") : "unsigned",
    certificateSha1: environment.SNAPLINGO_CERTIFICATE_SHA1 ?? null,
  }, null, 2) + "\n");
}

function releaseTarget() {
  const platform = process.platform === "darwin" ? "macos" : process.platform === "win32" ? "windows" : process.platform;
  const architecture = process.arch === "arm64" ? "aarch64" : process.arch === "x64" ? "x86_64" : process.arch;
  return `${platform}-${architecture}`;
}

function formatBytes(bytes) {
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = units[0];
  for (let index = 1; value >= 1024 && index < units.length; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 || unit === "B" ? 0 : 1)} ${unit}`;
}

async function main(args) {
  const command = args[0] ?? "build";
  if (
    command !== "build" &&
    command !== "verify" &&
    command !== "collect" &&
    command !== "preflight"
  ) {
    throw new Error(`Unknown release command: ${command}`);
  }

  const context = loadReleaseContext();
  console.log(`[release] Version ${context.version}`);
  console.log(`[release] Target directory: ${context.targetDirectory}`);

  if (command === "preflight") {
    console.log("[release] Version and release tag checks passed");
    return;
  }

  if (command === "build") {
    if (args.includes("--clean")) {
      cleanReleaseOutputs(context.targetDirectory);
      console.log("[release] Removed previous native release outputs");
    }
    buildRelease(context, args.slice(1).filter(arg => arg !== "--clean" && arg !== "--"));
  }

  if (process.platform === "win32") verifyNativeWindowsBundle(join(context.bundleDirectory, "app"));
  const artifacts = verifyReleaseArtifacts({
    platform: process.platform,
    bundleDirectory: context.bundleDirectory,
    productName: context.productName,
    version: context.version,
  });
  if (process.platform === "darwin") {
    verifyNativeMacOSArtifacts(context, artifacts);
  }
  if (command === "collect") collectArtifacts(context, artifacts);
  for (const artifact of artifacts) {
    console.log(
      `[release] ${artifact.kind}: ${artifact.path} (${formatBytes(artifact.size)})`,
    );
  }
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : null;
if (invokedPath === scriptPath) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(`[release] ${error instanceof Error ? error.message : error}`);
    process.exitCode = 1;
  });
}
