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
