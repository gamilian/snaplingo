import type { PinnedImageView } from '../../domain/capture';
import {
  colorSampleToClipboardText,
  type ColorSample,
  type ColorSampleFormat,
} from '../image-inspection/colorSampler';
import {
  createDefaultPinnedTransform,
  getPinnedDisplaySizeForTransform,
  type PinnedTransform,
} from './model';
import type {
  PinnedImageClipboardPort,
  PinnedImageCommandsPort,
  PinnedImageSettingsPort,
  PinnedWindowPort,
} from './ports';

export interface PinnedImagePresentation {
  zoom: number;
  transform: PinnedTransform;
  thumbnailMode: boolean;
}

interface PinnedImageRuntimePorts {
  imageId: string;
  commands: PinnedImageCommandsPort;
  clipboard: PinnedImageClipboardPort;
  settings: PinnedImageSettingsPort;
  window: PinnedWindowPort;
}

const defaultPresentation: PinnedImagePresentation = {
  zoom: 1,
  transform: createDefaultPinnedTransform(),
  thumbnailMode: false,
};

export function createPinnedImageRuntime({
  imageId,
  commands,
  clipboard,
  settings,
  window,
}: PinnedImageRuntimePorts) {
  let currentImage: PinnedImageView | null = null;
  let imageGeneration = 0;
  let mutationTail = Promise.resolve();

  function enqueueMutation<T>(operation: () => Promise<T>) {
    const result = mutationTail.then(operation, operation);
    mutationTail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  async function load() {
    const generation = ++imageGeneration;
    const image = await commands.getPinnedImage(imageId);
    if (generation !== imageGeneration) return null;

    currentImage = image;
    return image;
  }

  function resizeImage(
    image: PinnedImageView,
    presentation: PinnedImagePresentation,
  ) {
    const size = getPinnedDisplaySizeForTransform(
      image,
      presentation.zoom,
      presentation.transform,
      presentation.thumbnailMode,
    );
    return window.resize(size.width, size.height);
  }

  async function runBestEffortWindowEffect(
    operationName: string,
    operation: () => Promise<void>,
  ) {
    try {
      await operation();
    } catch (error) {
      console.error(`Failed to ${operationName}:`, error);
    }
  }

  return {
    load,

    async resize(presentation: PinnedImagePresentation) {
      const image = currentImage;
      if (!image) return;
      await runBestEffortWindowEffect('resize pinned image window', () =>
        resizeImage(image, presentation),
      );
    },

    async setShadow(enabled: boolean) {
      await runBestEffortWindowEffect('update pinned image shadow', () =>
        window.setShadow(enabled),
      );
    },

    moveBy(deltaX: number, deltaY: number) {
      return window.moveBy(deltaX, deltaY);
    },

    beginDrag() {
      return window.startDragging();
    },

    copyImage() {
      return commands.copyPinnedImage(imageId);
    },

    async copySourceText(sourceText?: string | null) {
      if (!sourceText) return false;
      await clipboard.writeText(sourceText);
      return true;
    },

    copyColor(sample: ColorSample, format: ColorSampleFormat) {
      return clipboard.writeText(colorSampleToClipboardText(sample, format));
    },

    async save() {
      const path = await commands.defaultCaptureSavePath();
      await commands.savePinnedImage(imageId, path);
    },

    async quickSave(directory?: string) {
      const path = await commands.quickCaptureSavePath(directory);
      await commands.savePinnedImage(imageId, path);
    },

    openPreferences() {
      return settings.open();
    },

    replaceFromClipboard() {
      return enqueueMutation(async () => {
        const generation = ++imageGeneration;
        const image = await commands.replacePinnedImageFromClipboard(imageId);
        if (generation === imageGeneration) currentImage = image;
        await runBestEffortWindowEffect('resize replaced pinned image window', () =>
          resizeImage(image, defaultPresentation),
        );
        return image;
      });
    },

    moveToNextGroup() {
      return enqueueMutation(async () => {
        await commands.movePinnedImageToNextGroup(imageId);
      });
    },

    hideGroup() {
      return enqueueMutation(async () => {
        await commands.hidePinnedImageGroup(imageId);
      });
    },

    destroyGroup() {
      return enqueueMutation(async () => {
        await commands.destroyPinnedImageGroup(imageId);
        currentImage = null;
        imageGeneration += 1;
      });
    },

    async close() {
      await runBestEffortWindowEffect('hide pinned image', () =>
        enqueueMutation(() => commands.closePinnedImage(imageId)),
      );
    },

    async destroy() {
      await runBestEffortWindowEffect('destroy pinned image', () =>
        enqueueMutation(async () => {
          await commands.removePinnedImage(imageId);
          currentImage = null;
          imageGeneration += 1;
          await window.close();
        }),
      );
    },
  };
}

export type PinnedImageRuntime = ReturnType<typeof createPinnedImageRuntime>;
