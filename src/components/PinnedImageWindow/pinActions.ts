export type PinInvokeArgs = Record<string, unknown>;
export type PinInvoke = <T>(
  command: string,
  args?: PinInvokeArgs,
) => Promise<T>;

export async function savePinnedImage(invoke: PinInvoke, imageId: string) {
  const path = await invoke<string>('default_capture_save_path');

  await invoke('save_pinned_image', {
    imageId,
    path,
  });
}
