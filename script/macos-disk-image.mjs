export function findAttachedDiskImageDevices(infoOutput, imagePath) {
  return infoOutput
    .split(/^={8,}$/m)
    .filter((section) => {
      const attachedImagePath = section.match(/^image-path\s*:\s*(.+)$/m)?.[1];
      return attachedImagePath?.trim() === imagePath;
    })
    .flatMap((section) => {
      const device = section.match(/^\/dev\/disk\d+(?=\s)/m)?.[0];
      return device ? [device] : [];
    });
}

export function createTauriBuildEnvironment(platform, environment) {
  const buildEnvironment = { ...environment };

  if (platform !== "darwin") {
    return buildEnvironment;
  }

  // The signed post-build step recreates the DMG, so Finder layout automation
  // only adds a fragile macOS Automation permission dependency.
  buildEnvironment.CI = "true";
  buildEnvironment.TAURI_BUNDLER_DMG_IGNORE_CI = "false";

  if (buildEnvironment.LANG === "C.UTF-8") {
    buildEnvironment.LANG = "en_US.UTF-8";
  }
  if (buildEnvironment.LC_ALL === "C.UTF-8") {
    buildEnvironment.LC_ALL = "en_US.UTF-8";
  }

  return buildEnvironment;
}
