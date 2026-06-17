import { describe, expect, it } from 'vitest';
import {
  destroyPinnedImage,
  destroyPinnedImageGroup,
  hidePinnedImageGroup,
  hidePinnedImage,
  isClosePinnedImageShortcut,
  isCopyPinnedImageShortcut,
  isDestroyPinnedImageShortcut,
  isReplacePinnedImageShortcut,
  isSavePinnedImageShortcut,
  movePinnedImageToNextGroup,
  replacePinnedImageFromClipboard,
  type PinInvoke,
  type PinInvokeArgs,
  savePinnedImage,
} from './pinActions';

describe('pinned image actions', () => {
  it('saves a pinned image to the default capture path', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const invoke: PinInvoke = async <T>(
      command: string,
      args?: PinInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      if (command === 'default_capture_save_path') {
        return '/tmp/SnapLingo-20260617-023000.png' as T;
      }
      return undefined as T;
    };

    await savePinnedImage(invoke, 'pin-1');

    expect(calls).toEqual([
      { command: 'default_capture_save_path', args: undefined },
      {
        command: 'save_pinned_image',
        args: {
          imageId: 'pin-1',
          path: '/tmp/SnapLingo-20260617-023000.png',
        },
      },
    ]);
  });

  it('uses Cmd/Ctrl+C for copying a pinned image', () => {
    expect(
      isCopyPinnedImageShortcut({
        key: 'c',
        metaKey: true,
        ctrlKey: false,
      }),
    ).toBe(true);
    expect(
      isCopyPinnedImageShortcut({
        key: 'C',
        metaKey: false,
        ctrlKey: true,
      }),
    ).toBe(true);
    expect(
      isCopyPinnedImageShortcut({
        key: 'c',
        metaKey: false,
        ctrlKey: false,
      }),
    ).toBe(false);
    expect(
      isCopyPinnedImageShortcut({
        key: 'c',
        metaKey: true,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isCopyPinnedImageShortcut({
        key: 'c',
        metaKey: false,
        ctrlKey: true,
        altKey: true,
      }),
    ).toBe(false);
  });

  it('uses Cmd/Ctrl+S for saving a pinned image', () => {
    expect(
      isSavePinnedImageShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
      }),
    ).toBe(true);
    expect(
      isSavePinnedImageShortcut({
        key: 'S',
        metaKey: false,
        ctrlKey: true,
      }),
    ).toBe(true);
    expect(
      isSavePinnedImageShortcut({
        key: 's',
        metaKey: false,
        ctrlKey: false,
      }),
    ).toBe(false);
    expect(
      isSavePinnedImageShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isSavePinnedImageShortcut({
        key: 's',
        metaKey: false,
        ctrlKey: true,
        altKey: true,
      }),
    ).toBe(false);
  });

  it('uses Cmd/Ctrl+W for closing a pinned image window', () => {
    expect(
      isClosePinnedImageShortcut({
        key: 'w',
        metaKey: true,
        ctrlKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isClosePinnedImageShortcut({
        key: 'W',
        metaKey: false,
        ctrlKey: true,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isClosePinnedImageShortcut({
        key: 'w',
        metaKey: false,
        ctrlKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isClosePinnedImageShortcut({
        key: 'w',
        metaKey: true,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isClosePinnedImageShortcut({
        key: 'w',
        metaKey: false,
        ctrlKey: true,
        altKey: true,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('uses Cmd/Ctrl+V for replacing a pinned image from the clipboard', () => {
    expect(
      isReplacePinnedImageShortcut({
        key: 'v',
        metaKey: true,
        ctrlKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isReplacePinnedImageShortcut({
        key: 'V',
        metaKey: false,
        ctrlKey: true,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isReplacePinnedImageShortcut({
        key: 'v',
        metaKey: true,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isReplacePinnedImageShortcut({
        key: 'v',
        metaKey: false,
        ctrlKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('replaces a pinned image with clipboard content', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const nextImage = {
      id: 'pin-1',
      image_base64: 'updated',
      width: 4,
      height: 3,
    };
    const invoke: PinInvoke = async <T>(
      command: string,
      args?: PinInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      return nextImage as T;
    };

    await expect(replacePinnedImageFromClipboard(invoke, 'pin-1')).resolves.toEqual(
      nextImage,
    );
    expect(calls).toEqual([
      {
        command: 'replace_pinned_image_from_clipboard',
        args: { imageId: 'pin-1' },
      },
    ]);
  });

  it('moves a pinned image to another group', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const invoke: PinInvoke = async <T>(
      command: string,
      args?: PinInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      return undefined as T;
    };

    await movePinnedImageToNextGroup(invoke, 'pin-1');

    expect(calls).toEqual([
      {
        command: 'move_pinned_image_to_next_group',
        args: { imageId: 'pin-1' },
      },
    ]);
  });

  it('hides a pinned image window without removing the image', async () => {
    const calls: string[] = [];

    await hidePinnedImage({
      hide: async () => {
        calls.push('hide');
      },
    });

    expect(calls).toEqual(['hide']);
  });

  it('destroys a pinned image by removing it before closing the window', async () => {
    const calls: Array<{ command?: string; args?: unknown; window?: string }> = [];
    const invoke: PinInvoke = async <T>(
      command: string,
      args?: PinInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      return undefined as T;
    };

    await destroyPinnedImage(invoke, 'pin-1', {
      close: async () => {
        calls.push({ window: 'close' });
      },
    });

    expect(calls).toEqual([
      {
        command: 'remove_pinned_image',
        args: { imageId: 'pin-1' },
      },
      { window: 'close' },
    ]);
  });

  it('destroys the group containing a pinned image', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const invoke: PinInvoke = async <T>(
      command: string,
      args?: PinInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      return undefined as T;
    };

    await destroyPinnedImageGroup(invoke, 'pin-1');

    expect(calls).toEqual([
      {
        command: 'destroy_pinned_image_group',
        args: { imageId: 'pin-1' },
      },
    ]);
  });

  it('hides the group containing a pinned image', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const invoke: PinInvoke = async <T>(
      command: string,
      args?: PinInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      return undefined as T;
    };

    await hidePinnedImageGroup(invoke, 'pin-1');

    expect(calls).toEqual([
      {
        command: 'hide_pinned_image_group',
        args: { imageId: 'pin-1' },
      },
    ]);
  });

  it('uses Shift+Escape for destroying a pinned image', () => {
    expect(isDestroyPinnedImageShortcut({ key: 'Escape', shiftKey: true })).toBe(
      true,
    );
    expect(isDestroyPinnedImageShortcut({ key: 'Escape', shiftKey: false })).toBe(
      false,
    );
    expect(isDestroyPinnedImageShortcut({ key: 'x', shiftKey: true })).toBe(
      false,
    );
  });
});
