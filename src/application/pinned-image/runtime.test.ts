import { describe, expect, it, vi } from 'vitest';
import type { PinnedImageView } from '../../domain/capture';
import type {
  PinnedImageClipboardPort,
  PinnedImageCommandsPort,
  PinnedImageSettingsPort,
  PinnedWindowPort,
} from './ports';
import { createPinnedImageRuntime } from './runtime';

const initialImage: PinnedImageView = {
  id: 'pin-1',
  image_base64: 'initial',
  width: 300,
  height: 200,
  source_text: 'source text',
};

function createRuntime(options: {
  image?: PinnedImageView;
  replacement?: PinnedImageView;
  closeError?: Error;
  removeError?: Error;
  resizeError?: Error;
} = {}) {
  const calls: string[] = [];
  const image = options.image ?? initialImage;
  const replacement = options.replacement ?? {
    ...initialImage,
    image_base64: 'replacement',
    width: 1200,
    height: 800,
  };
  const commands: PinnedImageCommandsPort = {
    getPinnedImage: vi.fn(async () => image),
    defaultCaptureSavePath: vi.fn(async () => '/pictures/default.png'),
    quickCaptureSavePath: vi.fn(async () => '/pictures/quick.png'),
    copyPinnedImage: vi.fn(async () => undefined),
    replacePinnedImageFromClipboard: vi.fn(async () => {
      calls.push('replace');
      return replacement;
    }),
    savePinnedImage: vi.fn(async () => undefined),
    closePinnedImage: vi.fn(async () => {
      if (options.closeError) throw options.closeError;
    }),
    removePinnedImage: vi.fn(async () => {
      calls.push('remove');
      if (options.removeError) throw options.removeError;
    }),
    movePinnedImageToNextGroup: vi.fn(async () => 2),
    hidePinnedImageGroup: vi.fn(async () => ['pin-1']),
    destroyPinnedImageGroup: vi.fn(async () => ['pin-1']),
  };
  const clipboard: PinnedImageClipboardPort = {
    writeText: vi.fn(async () => undefined),
  };
  const settings: PinnedImageSettingsPort = {
    open: vi.fn(async () => undefined),
  };
  const window: PinnedWindowPort = {
    resize: vi.fn(async () => {
      calls.push('resize');
      if (options.resizeError) throw options.resizeError;
    }),
    moveBy: vi.fn(async () => undefined),
    startDragging: vi.fn(async () => undefined),
    setShadow: vi.fn(async () => undefined),
    close: vi.fn(async () => {
      calls.push('window-close');
    }),
  };
  const runtime = createPinnedImageRuntime({
    imageId: 'pin-1',
    commands,
    clipboard,
    settings,
    window,
  });

  return { runtime, commands, clipboard, settings, window, calls };
}

describe('pinned image application runtime', () => {
  it('loads the bound image and resizes from portable presentation state', async () => {
    const { runtime, commands, window } = createRuntime();

    await expect(runtime.load()).resolves.toEqual(initialImage);
    await runtime.resize({
      zoom: 1,
      transform: { rotation: 90, flipX: false, flipY: false },
      thumbnailMode: false,
    });

    expect(commands.getPinnedImage).toHaveBeenCalledWith('pin-1');
    expect(window.resize).toHaveBeenCalledWith(200, 300);
  });

  it('owns default and configured-directory save workflows', async () => {
    const { runtime, commands } = createRuntime();

    await runtime.save();
    await runtime.quickSave('/captures');

    expect(commands.defaultCaptureSavePath).toHaveBeenCalledTimes(1);
    expect(commands.quickCaptureSavePath).toHaveBeenCalledWith('/captures');
    expect(commands.savePinnedImage).toHaveBeenNthCalledWith(
      1,
      'pin-1',
      '/pictures/default.png',
    );
    expect(commands.savePinnedImage).toHaveBeenNthCalledWith(
      2,
      'pin-1',
      '/pictures/quick.png',
    );
  });

  it('copies source text and sampled colors through one clipboard seam', async () => {
    const { runtime, clipboard } = createRuntime();

    await expect(runtime.copySourceText(null)).resolves.toBe(false);
    await expect(runtime.copySourceText('source text')).resolves.toBe(true);
    await runtime.copyColor(
      { hex: '#0A141E', red: 10, green: 20, blue: 30 },
      'rgb',
    );

    expect(clipboard.writeText).toHaveBeenNthCalledWith(1, 'source text');
    expect(clipboard.writeText).toHaveBeenNthCalledWith(2, 'rgb(10, 20, 30)');
  });

  it('replaces the image before resetting the native window size', async () => {
    const { runtime, window, calls } = createRuntime();

    const image = await runtime.replaceFromClipboard();

    expect(image.image_base64).toBe('replacement');
    expect(window.resize).toHaveBeenCalledWith(900, 600);
    expect(calls).toEqual(['replace', 'resize']);
  });

  it('keeps a clipboard replacement when native window resize fails', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    const { runtime, window } = createRuntime({
      resizeError: new Error('resize failed'),
    });

    await expect(runtime.replaceFromClipboard()).resolves.toMatchObject({
      image_base64: 'replacement',
    });
    await runtime.resize({
      zoom: 1,
      transform: { rotation: 90, flipX: false, flipY: false },
      thumbnailMode: false,
    });

    expect(window.resize).toHaveBeenLastCalledWith(600, 900);
    expect(consoleError).toHaveBeenCalledTimes(2);
    consoleError.mockRestore();
  });

  it('serializes state-changing workflows', async () => {
    let finishReplacement!: (image: PinnedImageView) => void;
    const replacement = new Promise<PinnedImageView>((resolve) => {
      finishReplacement = resolve;
    });
    const { runtime, commands } = createRuntime();
    vi.mocked(commands.replacePinnedImageFromClipboard).mockReturnValue(
      replacement,
    );

    const replacing = runtime.replaceFromClipboard();
    const moving = runtime.moveToNextGroup();
    await Promise.resolve();
    expect(commands.movePinnedImageToNextGroup).not.toHaveBeenCalled();

    finishReplacement(initialImage);
    await Promise.all([replacing, moving]);
    expect(commands.movePinnedImageToNextGroup).toHaveBeenCalledWith('pin-1');
  });

  it('does not let a stale initial load overwrite a clipboard replacement', async () => {
    let finishLoad!: (image: PinnedImageView) => void;
    const loading = new Promise<PinnedImageView>((resolve) => {
      finishLoad = resolve;
    });
    const { runtime, commands, window } = createRuntime();
    vi.mocked(commands.getPinnedImage).mockReturnValue(loading);

    const initialLoad = runtime.load();
    await runtime.replaceFromClipboard();
    finishLoad(initialImage);

    await expect(initialLoad).resolves.toBeNull();
    await runtime.resize({
      zoom: 1,
      transform: { rotation: 90, flipX: false, flipY: false },
      thumbnailMode: false,
    });
    expect(window.resize).toHaveBeenLastCalledWith(600, 900);
  });

  it('removes state before closing the native window', async () => {
    const { runtime, calls } = createRuntime();

    await runtime.destroy();

    expect(calls).toEqual(['remove', 'window-close']);
  });

  it('contains close and destroy failures as non-fatal window cleanup', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    const closeRuntime = createRuntime({ closeError: new Error('hide failed') });
    const destroyRuntime = createRuntime({ removeError: new Error('remove failed') });

    await expect(closeRuntime.runtime.close()).resolves.toBeUndefined();
    await expect(destroyRuntime.runtime.destroy()).resolves.toBeUndefined();

    expect(closeRuntime.window.close).not.toHaveBeenCalled();
    expect(destroyRuntime.window.close).not.toHaveBeenCalled();
    expect(consoleError).toHaveBeenCalledTimes(2);
    consoleError.mockRestore();
  });

  it('exposes user intents without leaking individual platform ports', async () => {
    const { runtime, commands, settings, window } = createRuntime();

    await runtime.copyImage();
    await runtime.openPreferences();
    await runtime.moveBy(2, -3);
    await runtime.beginDrag();
    await runtime.setShadow(false);
    await runtime.hideGroup();
    await runtime.destroyGroup();

    expect(commands.copyPinnedImage).toHaveBeenCalledWith('pin-1');
    expect(settings.open).toHaveBeenCalledTimes(1);
    expect(window.moveBy).toHaveBeenCalledWith(2, -3);
    expect(window.startDragging).toHaveBeenCalledTimes(1);
    expect(window.setShadow).toHaveBeenCalledWith(false);
    expect(commands.hidePinnedImageGroup).toHaveBeenCalledWith('pin-1');
    expect(commands.destroyPinnedImageGroup).toHaveBeenCalledWith('pin-1');
  });
});
