import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { PinnedImageView } from '../ScreenshotSession/types';
import {
  getPinnedContextMenuPosition,
  getPinnedDisplaySize,
  getPinnedOpacityFromWheel,
  getPinnedOpacityPreset,
  getPinnedZoomFromWheel,
} from './pinControls';

const appWindow = getCurrentWindow();
const webviewWindow = getCurrentWebviewWindow();
const PIN_CONTEXT_MENU_SIZE = { width: 132, height: 164 };

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
  const [zoom, setZoom] = useState(1);
  const [opacity, setOpacity] = useState(1);
  const [contextMenuPosition, setContextMenuPosition] = useState<{
    x: number;
    y: number;
  } | null>(null);
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

  const resetPinnedSize = useCallback(() => {
    setZoom(1);
    setContextMenuPosition(null);
    void resizePinnedWindow(1);
  }, [resizePinnedWindow]);

  const setPinnedOpacityPreset = useCallback((nextOpacity: number) => {
    setOpacity(getPinnedOpacityPreset(nextOpacity));
    setContextMenuPosition(null);
  }, []);

  const handleWheel = (event: React.WheelEvent<HTMLDivElement>) => {
    if (!image) return;

    event.preventDefault();
    setContextMenuPosition(null);
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

  const handleContextMenu = (event: React.MouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    setContextMenuPosition(
      getPinnedContextMenuPosition(
        { x: event.clientX, y: event.clientY },
        PIN_CONTEXT_MENU_SIZE,
        { width: window.innerWidth, height: window.innerHeight },
      ),
    );
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
      onContextMenu={handleContextMenu}
      onPointerDown={() => setContextMenuPosition(null)}
    >
      {image && (
        <img
          src={`data:image/png;base64,${image.image_base64}`}
          className="h-full w-full object-fill"
          style={{ opacity }}
          draggable={false}
          onPointerDown={(event) => {
            if (event.button !== 0) return;
            void appWindow.startDragging();
          }}
        />
      )}
      {contextMenuPosition && (
        <div
          className="absolute z-10 w-[132px] rounded bg-neutral-950/95 p-1 text-xs text-white shadow-xl ring-1 ring-white/15"
          style={{
            left: `${contextMenuPosition.x}px`,
            top: `${contextMenuPosition.y}px`,
          }}
          onPointerDown={(event) => event.stopPropagation()}
        >
          <button
            type="button"
            className="flex h-7 w-full items-center justify-between rounded px-2 text-left hover:bg-white/15"
            onClick={resetPinnedSize}
          >
            <span>Reset size</span>
            <span className="font-mono text-[10px] text-white/55">
              {Math.round(zoom * 100)}%
            </span>
          </button>
          {[1, 0.75, 0.5].map((preset) => (
            <button
              key={preset}
              type="button"
              className="flex h-7 w-full items-center justify-between rounded px-2 text-left hover:bg-white/15"
              onClick={() => setPinnedOpacityPreset(preset)}
            >
              <span>Opacity</span>
              <span className="font-mono text-[10px] text-white/55">
                {Math.round(preset * 100)}%
              </span>
            </button>
          ))}
          <div className="my-1 h-px bg-white/10" />
          <button
            type="button"
            className="h-7 w-full rounded px-2 text-left text-red-100 hover:bg-red-500/20"
            onClick={closePinnedImage}
          >
            Close
          </button>
        </div>
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
