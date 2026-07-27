import { spawnSync } from "node:child_process";
import {
  existsSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
} from "node:fs";
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

export function verifyReleaseArtifacts({
  platform,
  bundleDirectory,
  productName,
  version,
}) {
  const contract = releaseArtifactContract(platform, productName, version);
  return contract.map((artifact) => verifyArtifact(bundleDirectory, artifact));
}

export function releaseArtifactContract(platform, productName, version) {
  const isCurrentVersion = (name) => !version || name.includes(version);
  switch (platform) {
    case "darwin":
      return [
        {
          kind: "macOS application",
          directory: "macos",
          matches: (name) => name === `${productName}.app`,
          validate: validateMacOsApplication,
        },
        {
          kind: "macOS disk image",
          directory: "dmg",
          matches: (name) =>
            name.endsWith(".dmg") &&
            !name.startsWith("rw.") &&
            isCurrentVersion(name),
          validate: validateNonEmptyFile,
        },
      ];
    case "linux":
      return [
        {
          kind: "Linux AppImage",
          directory: "appimage",
          matches: (name) =>
            name.endsWith(".AppImage") && isCurrentVersion(name),
          validate: validateExecutableFile,
        },
        {
          kind: "Debian package",
          directory: "deb",
          matches: (name) => name.endsWith(".deb") && isCurrentVersion(name),
          validate: validateNonEmptyFile,
        },
      ];
    case "win32":
      return [
        {
          kind: "Windows MSI installer",
          directory: "msi",
          matches: (name) =>
            name.toLowerCase().endsWith(".msi") && isCurrentVersion(name),
          validate: validateNonEmptyFile,
        },
        {
          kind: "Windows NSIS installer",
          directory: "nsis",
          matches: (name) =>
            name.toLowerCase().endsWith(".exe") && isCurrentVersion(name),
          validate: validateNonEmptyFile,
        },
      ];
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
  if ((statSync(path).mode & 0o111) === 0) {
    throw new Error(`Release executable is not executable: ${path}`);
  }
  return size;
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
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
  const tauriConfig = readJson(
    join(repositoryRoot, "src-tauri", "tauri.conf.json"),
  );
  const cargoMetadata = JSON.parse(
    run("cargo", [
      "metadata",
      "--no-deps",
      "--format-version",
      "1",
      "--manifest-path",
      "src-tauri/Cargo.toml",
    ]),
  );
  const cargoPackage = cargoMetadata.packages.find(
    (candidate) => candidate.name === packageManifest.name,
  );
  if (!cargoPackage) {
    throw new Error(`Cargo package ${packageManifest.name} was not found`);
  }

  const version = assertUnifiedReleaseVersion({
    "package.json": packageManifest.version,
    "Cargo.toml": cargoPackage.version,
    "tauri.conf.json": tauriConfig.version,
  });

  return {
    version,
    productName: tauriConfig.productName ?? packageManifest.name,
    targetDirectory: cargoMetadata.target_directory,
    bundleDirectory: join(cargoMetadata.target_directory, "release", "bundle"),
  };
}

function cleanReleaseOutputs(targetDirectory) {
  rmSync(join(repositoryRoot, "dist"), { recursive: true, force: true });
  rmSync(join(targetDirectory, "release"), { recursive: true, force: true });
}

function buildRelease() {
  run(
    process.execPath,
    [join(repositoryRoot, "script", "run-tauri-build.mjs")],
    { stdio: "inherit" },
  );
  run(
    process.execPath,
    [join(repositoryRoot, "script", "fix-macos-release-signing.mjs")],
    { stdio: "inherit" },
  );
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
  if (command !== "build" && command !== "verify") {
    throw new Error(`Unknown release command: ${command}`);
  }

  const context = loadReleaseContext();
  console.log(`[release] Version ${context.version}`);
  console.log(`[release] Target directory: ${context.targetDirectory}`);

  if (command === "build") {
    if (args.includes("--clean")) {
      cleanReleaseOutputs(context.targetDirectory);
      console.log("[release] Removed previous frontend and release outputs");
    }
    buildRelease();
  }

  const artifacts = verifyReleaseArtifacts({
    platform: process.platform,
    bundleDirectory: context.bundleDirectory,
    productName: context.productName,
    version: context.version,
  });
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
