import { describe, expect, it } from 'vitest';
import {
  copyPinnedImage,
  copyPinnedText,
  closePinnedImage,
  destroyPinnedImage,
  destroyPinnedImageGroup,
  hidePinnedImageGroup,
  getPinnedHoverToolbarActions,
  isClosePinnedImageShortcut,
  isCopyPinnedImageShortcut,
  isCopyPinnedTextShortcut,
  isDestroyPinnedImageShortcut,
  isOpenPinnedPreferencesShortcut,
  isQuickSavePinnedImageShortcut,
  isReplacePinnedImageShortcut,
  isSavePinnedImageShortcut,
  movePinnedImageToNextGroup,
  quickSavePinnedImage,
  replacePinnedImageFromClipboard,
  type PinActionClient,
  savePinnedImage,
} from './pinActions';
import type { PinnedImageView } from '../../domain/capture';

function createPinActionClient(
  calls: Array<{ command: string; args?: unknown }>,
  options: {
    defaultPath?: string;
    quickPath?: string;
    nextImage?: PinnedImageView;
  } = {},
): PinActionClient {
  return {
    defaultCaptureSavePath: async () => {
      calls.push({ command: 'default_capture_save_path', args: undefined });
      return options.defaultPath ?? '';
    },
    quickCaptureSavePath: async (directory) => {
      calls.push({
        command: 'quick_capture_save_path',
        args: { directory },
      });
      return options.quickPath ?? '';
    },
    copyPinnedImage: async (imageId) => {
      calls.push({ command: 'copy_pinned_image', args: { imageId } });
    },
    replacePinnedImageFromClipboard: async (imageId) => {
      calls.push({
        command: 'replace_pinned_image_from_clipboard',
        args: { imageId },
      });
      return (
        options.nextImage ?? {
          id: imageId,
          image_base64: '',
          width: 0,
          height: 0,
          source_text: null,
        }
      );
    },
    savePinnedImage: async (imageId, path) => {
      calls.push({ command: 'save_pinned_image', args: { imageId, path } });
    },
    closePinnedImage: async (imageId) => {
      calls.push({ command: 'close_pinned_image', args: { imageId } });
    },
    removePinnedImage: async (imageId) => {
      calls.push({ command: 'remove_pinned_image', args: { imageId } });
    },
    movePinnedImageToNextGroup: async (imageId) => {
      calls.push({
        command: 'move_pinned_image_to_next_group',
        args: { imageId },
      });
      return 1;
    },
    hidePinnedImageGroup: async (imageId) => {
      calls.push({ command: 'hide_pinned_image_group', args: { imageId } });
      return [imageId];
    },
    destroyPinnedImageGroup: async (imageId) => {
      calls.push({
        command: 'destroy_pinned_image_group',
        args: { imageId },
      });
      return [imageId];
    },
  };
}

describe('pinned image actions', () => {
  it('exposes Snipaste-style hover toolbar actions for pinned images', () => {
    expect(getPinnedHoverToolbarActions()).toEqual([
      {
        id: 'copy',
        label: 'Copy',
        title: 'Copy',
        ariaLabel: 'Copy pinned image',
      },
      {
        id: 'save',
        label: 'Save',
        title: 'Save',
        ariaLabel: 'Save pinned image',
      },
      {
        id: 'close',
        label: 'X',
        title: 'Close',
        ariaLabel: 'Close pinned image',
      },
    ]);
  });

  it('saves a pinned image to the default capture path', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const client = createPinActionClient(calls, {
      defaultPath: '/tmp/SnapLingo-20260617-023000.png',
    });

    await savePinnedImage('pin-1', client);

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

  it('quick saves a pinned image to the configured capture path', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const client = createPinActionClient(calls, {
      quickPath: '/tmp/SnapLingo/SnapLingo-20260617-023000.png',
    });

    await quickSavePinnedImage('pin-1', '~/Pictures/SnapLingo', client);

    expect(calls).toEqual([
      {
        command: 'quick_capture_save_path',
        args: { directory: '~/Pictures/SnapLingo' },
      },
      {
        command: 'save_pinned_image',
        args: {
          imageId: 'pin-1',
          path: '/tmp/SnapLingo/SnapLingo-20260617-023000.png',
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

  it('copies a pinned image to the image clipboard', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const client = createPinActionClient(calls);

    await copyPinnedImage('pin-1', client);

    expect(calls).toEqual([
      {
        command: 'copy_pinned_image',
        args: { imageId: 'pin-1' },
      },
    ]);
  });

  it('uses Cmd/Ctrl+Shift+C for copying source text when a pin has text', () => {
    expect(
      isCopyPinnedTextShortcut(
        {
          key: 'c',
          metaKey: true,
          ctrlKey: false,
          shiftKey: true,
        },
        'Hello from clipboard',
      ),
    ).toBe(true);
    expect(
      isCopyPinnedTextShortcut(
        {
          key: 'C',
          metaKey: false,
          ctrlKey: true,
          shiftKey: true,
        },
        'Hello from clipboard',
      ),
    ).toBe(true);
    expect(
      isCopyPinnedTextShortcut(
        {
          key: 'c',
          metaKey: true,
          ctrlKey: false,
          shiftKey: true,
        },
        null,
      ),
    ).toBe(false);
    expect(
      isCopyPinnedTextShortcut(
        {
          key: 'c',
          metaKey: true,
          ctrlKey: false,
          shiftKey: false,
        },
        'Hello from clipboard',
      ),
    ).toBe(false);
    expect(
      isCopyPinnedTextShortcut(
        {
          key: 'c',
          metaKey: true,
          ctrlKey: false,
          altKey: true,
          shiftKey: true,
        },
        'Hello from clipboard',
      ),
    ).toBe(false);
  });

  it('copies pinned source text to the text clipboard', async () => {
    const calls: string[] = [];

    await expect(
      copyPinnedText(async (text) => {
        calls.push(text);
      }, 'Hello from clipboard'),
    ).resolves.toBe(true);

    expect(calls).toEqual(['Hello from clipboard']);
  });

  it('does not copy pinned source text when the pin has no text', async () => {
    const calls: string[] = [];

    await expect(
      copyPinnedText(async (text) => {
        calls.push(text);
      }, null),
    ).resolves.toBe(false);

    expect(calls).toEqual([]);
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

  it('uses Cmd/Ctrl+Shift+S for quick saving a pinned image', () => {
    expect(
      isQuickSavePinnedImageShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isQuickSavePinnedImageShortcut({
        key: 'S',
        metaKey: false,
        ctrlKey: true,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isQuickSavePinnedImageShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isQuickSavePinnedImageShortcut({
        key: 's',
        metaKey: false,
        ctrlKey: true,
        altKey: true,
        shiftKey: true,
      }),
    ).toBe(false);
  });

  it('uses Cmd/Ctrl+Shift+P for opening preferences from a pinned image', () => {
    expect(
      isOpenPinnedPreferencesShortcut({
        key: 'p',
        metaKey: true,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isOpenPinnedPreferencesShortcut({
        key: 'P',
        metaKey: false,
        ctrlKey: true,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isOpenPinnedPreferencesShortcut({
        key: 'p',
        metaKey: true,
        ctrlKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isOpenPinnedPreferencesShortcut({
        key: 'p',
        metaKey: false,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isOpenPinnedPreferencesShortcut({
        key: 'p',
        metaKey: true,
        ctrlKey: false,
        altKey: true,
        shiftKey: true,
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
    const nextImage: PinnedImageView = {
      id: 'pin-1',
      image_base64: 'updated',
      width: 4,
      height: 3,
    };
    const client = createPinActionClient(calls, { nextImage });

    await expect(replacePinnedImageFromClipboard('pin-1', client)).resolves.toEqual(
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
    const client = createPinActionClient(calls);

    await movePinnedImageToNextGroup('pin-1', client);

    expect(calls).toEqual([
      {
        command: 'move_pinned_image_to_next_group',
        args: { imageId: 'pin-1' },
      },
    ]);
  });

  it('closes a pinned image through the backend recovery path', async () => {
    const calls: Array<{ command?: string; args?: unknown }> = [];
    const client = createPinActionClient(
      calls as Array<{ command: string; args?: unknown }>,
    );

    await closePinnedImage('pin-1', client);

    expect(calls).toEqual([
      {
        command: 'close_pinned_image',
        args: { imageId: 'pin-1' },
      },
    ]);
  });

  it('destroys a pinned image by removing it before closing the window', async () => {
    const calls: Array<{ command?: string; args?: unknown; window?: string }> = [];
    const client = createPinActionClient(
      calls as Array<{ command: string; args?: unknown }>,
    );

    await destroyPinnedImage('pin-1', {
      close: async () => {
        calls.push({ window: 'close' });
      },
    }, client);

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
    const client = createPinActionClient(calls);

    await destroyPinnedImageGroup('pin-1', client);

    expect(calls).toEqual([
      {
        command: 'destroy_pinned_image_group',
        args: { imageId: 'pin-1' },
      },
    ]);
  });

  it('hides the group containing a pinned image', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const client = createPinActionClient(calls);

    await hidePinnedImageGroup('pin-1', client);

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
    expect(
      isDestroyPinnedImageShortcut({
        key: 'Escape',
        shiftKey: true,
        ctrlKey: true,
      }),
    ).toBe(false);
    expect(
      isDestroyPinnedImageShortcut({
        key: 'Escape',
        shiftKey: true,
        altKey: true,
      }),
    ).toBe(false);
  });
});
