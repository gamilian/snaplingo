export type PinInvokeArgs = Record<string, unknown>;
export type PinInvoke = <T>(
  command: string,
  args?: PinInvokeArgs,
) => Promise<T>;

interface PinShortcutEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
}

export function isCopyPinnedImageShortcut(event: PinShortcutEvent) {
  return event.key.toLowerCase() === 'c' && (event.metaKey || event.ctrlKey);
}

export function isSavePinnedImageShortcut(event: PinShortcutEvent) {
  return event.key.toLowerCase() === 's' && (event.metaKey || event.ctrlKey);
}

export async function savePinnedImage(invoke: PinInvoke, imageId: string) {
  const path = await invoke<string>('default_capture_save_path');

  await invoke('save_pinned_image', {
    imageId,
    path,
  });
}

export async function movePinnedImageToNextGroup(
  invoke: PinInvoke,
  imageId: string,
) {
  await invoke('move_pinned_image_to_next_group', {
    imageId,
  });
}
