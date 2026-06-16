import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { PinnedImageView } from '../ScreenshotSession/types';
import {
  getPinnedDisplaySize,
  getPinnedOpacityFromWheel,
  getPinnedZoomFromWheel,
} from './pinControls';

const appWindow = getCurrentWindow();
const webviewWindow = getCurrentWebviewWindow();

function readPinnedImageId(search: string) {
  const params = new URLSearchParams(search);
  if (params.get('window') !== 'pin') return null;

  return params.get('imageId');
}

interface PinnedImageWindowProps {
  imageId: string;
}

export function PinnedImageWindow({ imageId }: PinnedImageWindowProps) {
  const [image, setImage] = useState<PinnedImageView | null>(null);
  const [, setZoom] = useState(1);
  const [opacity, setOpacity] = useState(1);
  const [error, setError] = useState<string | null>(null);

  const closePinnedImage = useCallback(async () => {
    try {
      await invoke('remove_pinned_image', { imageId });
    } catch (err) {
      console.error('Failed to remove pinned image:', err);
    }

    await webviewWindow.close();
  }, [imageId]);

  useEffect(() => {
    let disposed = false;

    invoke<PinnedImageView>('get_pinned_image', { imageId })
      .then((nextImage) => {
        if (!disposed) {
          setImage(nextImage);
        }
      })
      .catch((err) => {
        if (!disposed) {
          setError(err instanceof Error ? err.message : String(err));
        }
      });

    return () => {
      disposed = true;
    };
  }, [imageId]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;

      event.preventDefault();
      void closePinnedImage();
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [closePinnedImage]);

  const resizePinnedWindow = useCallback(
    async (nextZoom: number, nextImage = image) => {
      if (!nextImage) return;

      const size = getPinnedDisplaySize(nextImage, nextZoom);
      await appWindow.setSize(new LogicalSize(size.width, size.height));
    },
    [image],
  );

  const handleWheel = (event: React.WheelEvent<HTMLDivElement>) => {
    if (!image) return;

    event.preventDefault();
    const wheelDirection = Math.sign(event.deltaY) || 1;

    if (event.shiftKey) {
      setOpacity((currentOpacity) =>
        getPinnedOpacityFromWheel(currentOpacity, wheelDirection),
      );
      return;
    }

    setZoom((currentZoom) => {
      const nextZoom = getPinnedZoomFromWheel(currentZoom, wheelDirection);
      void resizePinnedWindow(nextZoom);
      return nextZoom;
    });
  };

  if (error) {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-neutral-950 p-3 text-xs text-red-100">
        {error}
      </div>
    );
  }

  return (
    <div
      className="group relative h-screen w-screen overflow-hidden bg-transparent"
      onWheel={handleWheel}
    >
      {image && (
        <img
          src={`data:image/png;base64,${image.image_base64}`}
          className="h-full w-full object-fill"
          style={{ opacity }}
          draggable={false}
          onPointerDown={() => {
            void appWindow.startDragging();
          }}
        />
      )}
      <button
        type="button"
        className="absolute right-1 top-1 h-6 w-6 rounded bg-black/70 text-xs leading-6 text-white opacity-0 shadow hover:bg-black/90 focus:opacity-100 group-hover:opacity-100"
        aria-label="Close pinned image"
        title="Close"
        onClick={closePinnedImage}
      >
        X
      </button>
    </div>
  );
}

export function readPinnedImageLaunch(search: string) {
  return readPinnedImageId(search);
}
